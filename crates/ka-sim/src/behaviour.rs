//! How each card behaves as a server: capacity, lanes, service time, queue (ADR 0006).
//! Props come from the card, then its flavour's overrides, then the kind's defaults; the
//! few figures a kind has no prop for live in `KindDefaults` (M1.2 calibrates both).

use ka_core::{ComponentKind, Node, PropValue, Registry};

use crate::route::Role;

/// Values the registry has no prop for.
#[derive(Debug, Clone, Copy)]
pub struct KindDefaults {
    /// Concurrent requests per instance when the kind has no `concurrency` prop.
    pub servers_per_instance: f64,
    /// Median service time when the kind has no `service_ms` prop.
    pub service_ms: f64,
    /// p99 / median of the service time.
    pub p99_ratio: f64,
    /// Queue slots per server.
    pub queue_per_server: f64,
}

pub fn kind_defaults(kind: ComponentKind) -> KindDefaults {
    use ComponentKind as K;
    let d = |servers_per_instance: f64, service_ms: f64, p99_ratio: f64, queue_per_server: f64| {
        KindDefaults {
            servers_per_instance,
            service_ms,
            p99_ratio,
            queue_per_server,
        }
    };
    match kind {
        K::Dns => d(10_000.0, 0.2, 3.0, 4.0),
        K::Cdn => d(100_000.0, 2.0, 4.0, 4.0),
        K::Waf => d(2_000.0, 0.5, 3.0, 8.0),
        K::Lb => d(10_000.0, 0.2, 3.0, 8.0),
        K::Gateway => d(256.0, 2.0, 4.0, 8.0),
        K::Ratelimit => d(10_000.0, 0.3, 3.0, 8.0),
        K::App | K::Micro => d(64.0, 10.0, 4.0, 8.0),
        K::Worker => d(8.0, 100.0, 4.0, 8.0),
        K::Serverless => d(1.0, 20.0, 4.0, 2.0),
        K::Sql => d(100.0, 5.0, 5.0, 4.0),
        K::Nosql => d(64.0, 3.0, 4.0, 4.0),
        // Redis and Memcached answer from one event loop per instance, very fast.
        K::Cache => d(1.0, 0.05, 3.0, 512.0),
        K::Object => d(10_000.0, 50.0, 4.0, 4.0),
        K::Search => d(16.0, 20.0, 4.0, 8.0),
        K::Queue | K::Stream | K::Pubsub | K::Dlq => d(10_000.0, 1.0, 3.0, 4.0),
        K::Client | K::Scheduler | K::Autoscale | K::Metrics | K::Logs | K::Tracing => {
            d(1.0, 0.0, 1.0, 1.0)
        }
    }
}

/// A card's resolved behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSpec {
    pub role: Role,
    pub instances: f64,
    /// Concurrent requests in total (servers), before sampling.
    pub servers: f64,
    /// Independent lanes (partitions or shards): a key always lands in the same lane.
    pub lanes: u32,
    pub service_ms: f64,
    pub p99_ratio: f64,
    /// Queue slots in total, before sampling.
    pub queue_cap: f64,
    /// Compute and workers hold their slot while their calls run.
    pub holds: bool,
    pub cold_start_ms: f64,
    /// Cache memory (GB) and TTL (s; 0 = never expires).
    pub memory_gb: f64,
    pub ttl_s: f64,
    /// Brokers.
    pub max_depth: f64,
    pub max_retries: u32,
    pub visibility_timeout_s: f64,
    pub partitions: f64,
    /// Rate limiters, per API key.
    pub limit_rps: f64,
    pub burst: f64,
    /// Load balancers: "round_robin" | "least_conn" | "ip_hash".
    pub algorithm: String,
    /// Data clusters.
    pub replication_lag_ms: f64,
    pub failover_s: f64,
}

/// The value of `key` for `node`: the card's prop, else its flavour override, else the
/// kind's default; None if the kind has no such prop.
pub fn prop(registry: &Registry, node: &Node, key: &str) -> Option<PropValue> {
    if let Some(v) = node.props.get(key) {
        return Some(v.clone());
    }
    let spec = registry.kind(node.kind);
    if let Some(o) = spec
        .flavour(node.flavour)
        .and_then(|f| f.overrides.iter().find(|o| o.key == key))
    {
        return Some(o.value.clone());
    }
    spec.prop(key).map(|p| p.default.clone())
}

fn num(registry: &Registry, node: &Node, key: &str) -> Option<f64> {
    match prop(registry, node, key)? {
        PropValue::Number(n) => Some(n),
        _ => None,
    }
}

fn text(registry: &Registry, node: &Node, key: &str) -> Option<String> {
    match prop(registry, node, key)? {
        PropValue::Text(s) => Some(s),
        _ => None,
    }
}

pub fn node_spec(registry: &Registry, node: &Node) -> NodeSpec {
    use ComponentKind as K;
    let d = kind_defaults(node.kind);
    let n = |k: &str| num(registry, node, k);
    let instances = n("instances").unwrap_or(1.0).max(1.0);
    let servers = match node.kind {
        K::Serverless => n("concurrency_limit").unwrap_or(1000.0),
        _ => instances * n("concurrency").unwrap_or(d.servers_per_instance),
    }
    .max(1.0);
    let lanes = match node.kind {
        K::Nosql => n("partitions").unwrap_or(1.0),
        K::Sql | K::Search => n("shards").unwrap_or(1.0),
        _ => 1.0,
    }
    .max(1.0) as u32;
    let role = Role::of(node.kind);
    NodeSpec {
        role,
        instances,
        servers: if lanes > 1 {
            servers * f64::from(lanes)
        } else {
            servers
        },
        lanes,
        service_ms: n("service_ms").unwrap_or(d.service_ms),
        p99_ratio: d.p99_ratio,
        queue_cap: (servers * d.queue_per_server).max(16.0),
        holds: role.is_caller(),
        cold_start_ms: n("cold_start_ms").unwrap_or(0.0),
        memory_gb: n("memory_gb").unwrap_or(0.0),
        ttl_s: n("ttl_s").unwrap_or(0.0),
        max_depth: n("max_depth").unwrap_or(100_000.0),
        max_retries: n("max_retries").unwrap_or(3.0) as u32,
        visibility_timeout_s: n("visibility_timeout_s").unwrap_or(30.0),
        partitions: n("partitions").unwrap_or(f64::INFINITY),
        limit_rps: n("limit_rps").unwrap_or(f64::INFINITY),
        burst: n("burst").unwrap_or(1.0),
        algorithm: text(registry, node, "algorithm").unwrap_or_else(|| "round_robin".into()),
        replication_lag_ms: n("replication_lag_ms").unwrap_or(0.0),
        failover_s: n("failover_s").unwrap_or(10.0),
    }
}

//! How each card behaves as a server: capacity, lanes, service time, queue (ADR 0006).
//! Props come from the card, then its flavour's overrides, then the kind's defaults; the
//! figures a kind has no prop for are the flavour's `SimDefaults` (both calibrated in the
//! registry, ADR 0005).

use ka_core::{ComponentKind, Node, PropValue, Registry, SimDefaults};

use crate::route::Role;

/// The flavour's behaviour without a prop; kinds never on a path get inert values.
pub fn sim_defaults(registry: &Registry, node: &Node) -> SimDefaults {
    registry
        .kind(node.kind)
        .flavour(node.flavour)
        .and_then(|f| f.sim)
        .unwrap_or(SimDefaults {
            servers_per_instance: 1.0,
            service_ms: 0.0,
            p99_ratio: 1.0,
            queue_per_server: 1.0,
            failover_s: 0.0,
        })
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
    let d = sim_defaults(registry, node);
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
        failover_s: n("failover_s").unwrap_or(d.failover_s),
    }
}

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
            write_slots: 1.0,
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
    /// Queues: a message whose worker died comes back after this (its lease).
    pub visibility_timeout_s: f64,
    /// Streams: a dead consumer's partitions are reassigned after this.
    pub session_timeout_s: f64,
    pub partitions: f64,
    /// Rate limiters, per API key.
    pub limit_rps: f64,
    pub burst: f64,
    /// Load balancers: "round_robin" | "least_conn" | "key_hash" (sticky by request key).
    pub algorithm: String,
    /// Data clusters.
    pub replication_lag_ms: f64,
    pub failover_s: f64,
    /// Reads go to the primary only (read-your-writes), not spread over replicas.
    pub read_from_primary: bool,
    /// Change data capture: how long committed changes wait in the database log for a
    /// stream that is down before they count as lost.
    pub cdc_buffer_s: f64,
    /// Service slots a write occupies (DynamoDB: 3).
    pub write_slots: f64,
    /// SQL and Search: shards over the kind's default count; the per-instance price scales
    /// with it (cost per shard, ADR 0006). 1 for every other kind.
    pub cost_lanes: f64,
    /// What `Cost::per_lane_month` is multiplied by: a NoSQL card's partitions, a stream's
    /// partitions (shards); 0 for kinds without lanes.
    pub bill_lanes: f64,
    /// The share of requests through the card that are billed (DNS: `query_ratio`).
    pub bill_ratio: f64,
    /// Clients (S8): when a synchronous request gives up, and how it tries again.
    pub timeout_us: u64,
    pub retries: u32,
    pub backoff_ms: f64,
    /// Full jitter: each backoff is drawn uniformly between zero and its full length.
    pub full_jitter: bool,
    /// Retries allowed as a share of first tries over the last 10 s; 0 = no budget.
    pub retry_budget: f64,
    /// Callers (S8): give up on a call after this (0 = wait for it).
    pub call_timeout_us: u64,
    /// A circuit breaker per dependency opens at this share of failed calls (0 = none), for
    /// `breaker_open_us`.
    pub breaker_pct: f64,
    pub breaker_open_us: u64,
    /// Each dependency gets its own share of the card's slots.
    pub bulkhead: bool,
    /// Turn new requests away once this many wait (0 = never; before sampling).
    pub shed_queue: f64,
    /// S8 rollouts: the share of instances each stage reaches, the wait between stages
    /// (deploys, and a Config card's staged pushes), and automatic rollback.
    pub rollout_share: f64,
    pub bake_us: u64,
    pub auto_rollback: bool,
    /// A Config card: pushes go one Region at a time; a kill switch rolls back fast.
    pub staged: bool,
    pub kill_switch: bool,
    /// DNS in failover mode (S8): answers point at the first healthy target.
    pub dns_failover: bool,
    /// Load balancers (S8): eject targets whose queue wait passes this (0 = liveness only),
    /// keeping at least this share in rotation.
    pub health_timeout_us: u64,
    pub min_healthy: f64,
    /// S7 data clusters: a replica cut off from its primary promotes itself
    /// (`split_failover = auto`); writes need a current fencing token (`fencing`).
    pub split_failover: bool,
    pub fencing: bool,
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
    let lane_key = match node.kind {
        K::Nosql => Some("partitions"),
        K::Sql | K::Search => Some("shards"),
        _ => None,
    };
    let lanes = lane_key.and_then(n).unwrap_or(1.0).max(1.0) as u32;
    // The flavour's default lane count (the card's own value left out): a card at its
    // default costs what one instance costs; more shards cost more, fewer cost less.
    let default_lanes = lane_key
        .and_then(|k| {
            let bare = Node {
                props: Default::default(),
                ..node.clone()
            };
            num(registry, &bare, k)
        })
        .unwrap_or(1.0)
        .max(1.0);
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
        session_timeout_s: n("session_timeout_s").unwrap_or(45.0),
        partitions: n("partitions").unwrap_or(f64::INFINITY),
        limit_rps: n("limit_rps").unwrap_or(f64::INFINITY),
        burst: n("burst").unwrap_or(1.0),
        algorithm: match text(registry, node, "algorithm").as_deref() {
            // `ip_hash` is the name files before `.kbarch` v3 used.
            Some("key_hash" | "ip_hash") => "key_hash".into(),
            Some(a) => a.to_owned(),
            None => "round_robin".into(),
        },
        replication_lag_ms: n("replication_lag_ms").unwrap_or(0.0),
        failover_s: n("failover_s").unwrap_or(d.failover_s),
        read_from_primary: text(registry, node, "read_from").as_deref() == Some("primary"),
        cdc_buffer_s: n("cdc_buffer_s").unwrap_or(300.0),
        write_slots: d.write_slots.max(1.0),
        cost_lanes: match node.kind {
            K::Sql | K::Search => f64::from(lanes) / default_lanes,
            _ => 1.0,
        },
        bill_lanes: match node.kind {
            K::Nosql => f64::from(lanes),
            K::Stream => n("partitions").unwrap_or(0.0),
            _ => 0.0,
        },
        bill_ratio: match node.kind {
            K::Dns => n("query_ratio").unwrap_or(1.0),
            _ => 1.0,
        },
        timeout_us: (n("timeout_ms").unwrap_or(5_000.0).max(1.0) * 1000.0) as u64,
        retries: n("retries").unwrap_or(0.0).max(0.0) as u32,
        backoff_ms: n("backoff_ms").unwrap_or(100.0).max(0.0),
        full_jitter: text(registry, node, "jitter").as_deref() == Some("full"),
        retry_budget: n("retry_budget_pct").unwrap_or(0.0).max(0.0) / 100.0,
        call_timeout_us: (n("call_timeout_ms").unwrap_or(0.0).max(0.0) * 1000.0) as u64,
        breaker_pct: n("breaker_errors_pct").unwrap_or(0.0).clamp(0.0, 100.0) / 100.0,
        breaker_open_us: (n("breaker_open_s").unwrap_or(30.0).max(0.0) * 1e6) as u64,
        bulkhead: matches!(
            prop(registry, node, "bulkhead"),
            Some(PropValue::Bool(true))
        ),
        shed_queue: n("shed_queue").unwrap_or(0.0).max(0.0),
        rollout_share: n("rollout_percent").unwrap_or(100.0).clamp(1.0, 100.0) / 100.0,
        bake_us: (n("bake_s").unwrap_or(0.0).max(0.0) * 1e6) as u64,
        auto_rollback: matches!(
            prop(registry, node, "auto_rollback"),
            Some(PropValue::Bool(true))
        ),
        staged: matches!(prop(registry, node, "staged"), Some(PropValue::Bool(true))),
        kill_switch: matches!(
            prop(registry, node, "kill_switch"),
            Some(PropValue::Bool(true))
        ),
        dns_failover: text(registry, node, "routing").as_deref() == Some("failover"),
        health_timeout_us: (n("health_timeout_ms").unwrap_or(0.0).max(0.0) * 1000.0) as u64,
        min_healthy: n("min_healthy_pct").unwrap_or(0.0).clamp(0.0, 100.0) / 100.0,
        split_failover: text(registry, node, "split_failover").as_deref() == Some("auto"),
        fencing: matches!(prop(registry, node, "fencing"), Some(PropValue::Bool(true))),
    }
}

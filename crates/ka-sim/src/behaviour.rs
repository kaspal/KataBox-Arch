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
    /// S1 holders (WebSocket gateways, IoT brokers): connections one instance can hold, new
    /// connections it accepts per second, and server pushes it sends per second.
    pub conn_capacity: f64,
    pub accept_per_s: f64,
    pub msgs_per_s: f64,
    /// S1 clients: how reconnects spread after a drop, and the wait before retrying a
    /// refused connection.
    pub reconnect_jitter_s: f64,
    pub reconnect_backoff_s: f64,
    /// S2 model inference: what it answers (`chat`, `embed`, `classify`), whether a provider
    /// hosts it (no slot limit, per-minute request and token limits), its speeds, and the
    /// batch API (half the token price, results by the batch window).
    pub model_type: String,
    pub hosted: bool,
    pub ttft_ms: f64,
    pub tokens_per_s: f64,
    pub prefill_tps: f64,
    pub rpm: f64,
    pub tpm: f64,
    pub batch_api: bool,
    /// S3: a semantic cache (`mode = semantic`).
    pub semantic: bool,
    /// S4 batch cards: launching a task, re-running slow ones near the end, splitting the
    /// largest tasks, and resuming interrupted tasks from a checkpoint.
    pub task_overhead_ms: f64,
    pub speculation: bool,
    pub salting: u32,
    pub checkpoint: bool,
    /// S5 stream processors: the window (0 none, 1 tumbling, 2 sliding), its length and
    /// slide, the watermark delay, the allowed lateness, the checkpoint interval and the
    /// state each key holds; data cards: idempotent writes.
    pub window: u8,
    pub window_us: u64,
    pub slide_us: u64,
    pub watermark_us: u64,
    pub lateness_us: u64,
    pub checkpoint_us: u64,
    pub state_kb: f64,
    pub dedup: bool,
    /// S6 warehouses: scan rate per query, overhead per query, the result cache and the
    /// auto-suspend tail; lake tables: file size, partitioning (0 none, 1 day, 2 hour) and
    /// compaction.
    pub scan_gbps: f64,
    pub overhead_ms: f64,
    pub result_cache: bool,
    pub auto_suspend_s: f64,
    pub target_file_mb: f64,
    pub partitioning: u8,
    pub compaction: bool,
    /// M8 identity providers: login and token-check times, the login rate limit, and
    /// whether apps check tokens themselves (JWKS) instead of calling.
    pub login_ms: f64,
    pub verify_ms: f64,
    pub logins_per_s: f64,
    pub verify_local: bool,
    /// M8: a database's connection limit (with the scenario's `connections`), and a
    /// connection pooler's pool.
    pub max_connections: f64,
    pub pool_size: f64,
    /// M8: the provider a Third-Party API card stands for.
    pub binding: String,
    /// M8: graph traversal speed; vector count; time-series ingest rate and series limit;
    /// a media server's mode (true: transcode) and segment length.
    pub edges_per_ms: f64,
    pub vectors: f64,
    pub points_per_s: f64,
    pub series_limit: f64,
    pub transcode: bool,
    pub segment_s: f64,
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

fn is_hosted(node: &Node) -> bool {
    matches!(
        node.flavour,
        ka_core::Flavour::ClaudeApi
            | ka_core::Flavour::OpenaiApi
            | ka_core::Flavour::Bedrock
            | ka_core::Flavour::GeminiApi
    )
}

pub fn node_spec(registry: &Registry, node: &Node) -> NodeSpec {
    use ComponentKind as K;
    let d = sim_defaults(registry, node);
    let n = |k: &str| num(registry, node, k);
    let instances = n("instances").unwrap_or(1.0).max(1.0);
    let servers = match node.kind {
        K::Serverless => n("concurrency_limit").unwrap_or(1000.0),
        // S2: a provider has no slot limit (its per-minute limits bind); GPUs have batch slots.
        K::Infer if is_hosted(node) => 10_000.0,
        K::Infer => instances * n("batch_slots").unwrap_or(16.0),
        // S4: executors run one task each.
        K::Batch => instances * n("executors").unwrap_or(4.0),
        _ => instances * n("concurrency").unwrap_or(d.servers_per_instance),
    }
    .max(1.0);
    let lane_key = match node.kind {
        K::Nosql => Some("partitions"),
        K::Sql | K::Search | K::Vector => Some("shards"),
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
        // S5: a stream processor's slot handles `records_per_s` simple records a second.
        service_ms: match node.kind {
            K::Streamproc => 1000.0 / n("records_per_s").unwrap_or(20_000.0).max(1e-6),
            _ => n("service_ms").unwrap_or(d.service_ms),
        },
        p99_ratio: d.p99_ratio,
        // S4: a batch card's tasks wait for an executor however many there are.
        queue_cap: if node.kind == K::Batch {
            f64::MAX / 4.0
        } else {
            (servers * d.queue_per_server).max(16.0)
        },
        holds: role.is_caller(),
        cold_start_ms: n("cold_start_ms").unwrap_or(0.0),
        // S6: a lake table bills its stored data per GB.
        memory_gb: match node.kind {
            K::Lakehouse => n("stored_tb").unwrap_or(0.0) * 1024.0,
            _ => n("memory_gb").unwrap_or(0.0),
        },
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
            K::Sql | K::Search | K::Vector => f64::from(lanes) / default_lanes,
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
        conn_capacity: {
            let max = match node.kind {
                // M8: a media server holds streams (viewers of an SFU, channels of an encoder).
                K::Media => n("streams_per_instance").unwrap_or(500.0),
                _ => n("max_connections").unwrap_or(f64::INFINITY),
            };
            let by_memory = match (n("memory_gb"), n("conn_memory_kb")) {
                (Some(gb), Some(kb)) if kb > 0.0 => gb * 1024.0 * 1024.0 / kb,
                _ => f64::INFINITY,
            };
            max.min(by_memory)
        },
        accept_per_s: n("accept_per_s").unwrap_or(10_000.0).max(0.0),
        msgs_per_s: n("msgs_per_s").unwrap_or(f64::INFINITY).max(0.0),
        reconnect_jitter_s: n("reconnect_jitter_s").unwrap_or(0.0).max(0.0),
        reconnect_backoff_s: n("reconnect_backoff_s").unwrap_or(1.0).max(0.0),
        model_type: text(registry, node, "model_type").unwrap_or_default(),
        hosted: is_hosted(node),
        ttft_ms: n("ttft_ms").unwrap_or(0.0).max(0.0),
        tokens_per_s: n("tokens_per_s").unwrap_or(1.0).max(1e-6),
        prefill_tps: n("prefill_tps").unwrap_or(f64::INFINITY).max(1e-6),
        rpm: n("rpm").unwrap_or(f64::INFINITY),
        tpm: n("tpm").unwrap_or(f64::INFINITY),
        batch_api: text(registry, node, "api_mode").as_deref() == Some("batch"),
        semantic: text(registry, node, "mode").as_deref() == Some("semantic")
            && node.kind == K::Cache,
        task_overhead_ms: n("task_overhead_ms").unwrap_or(0.0).max(0.0),
        speculation: matches!(
            prop(registry, node, "speculation"),
            Some(PropValue::Bool(true))
        ),
        salting: n("salting").unwrap_or(1.0).max(1.0) as u32,
        checkpoint: matches!(
            prop(registry, node, "checkpoint"),
            Some(PropValue::Bool(true))
        ),
        window: match text(registry, node, "window").as_deref() {
            Some("tumbling") => 1,
            Some("sliding") => 2,
            _ => 0,
        },
        window_us: (n("window_s").unwrap_or(60.0).max(1.0) * 1e6) as u64,
        slide_us: (n("slide_s").unwrap_or(10.0).max(1.0) * 1e6) as u64,
        watermark_us: (n("watermark_s").unwrap_or(0.0).max(0.0) * 1e6) as u64,
        lateness_us: (n("allowed_lateness_s").unwrap_or(0.0).max(0.0) * 1e6) as u64,
        checkpoint_us: (n("checkpoint_s").unwrap_or(60.0).max(1.0) * 1e6) as u64,
        state_kb: n("state_kb").unwrap_or(1.0).max(0.0),
        dedup: matches!(prop(registry, node, "dedup"), Some(PropValue::Bool(true))),
        scan_gbps: n("scan_gbps").unwrap_or(0.5).max(1e-6),
        overhead_ms: n("overhead_ms").unwrap_or(0.0).max(0.0),
        result_cache: matches!(
            prop(registry, node, "result_cache"),
            Some(PropValue::Bool(true))
        ),
        auto_suspend_s: n("auto_suspend_s").unwrap_or(0.0).max(0.0),
        target_file_mb: n("target_file_mb").unwrap_or(1024.0).max(1e-3),
        partitioning: match text(registry, node, "partitioning").as_deref() {
            Some("day") => 1,
            Some("hour") => 2,
            _ => 0,
        },
        compaction: text(registry, node, "compaction")
            .as_deref()
            .is_some_and(|c| c != "none"),
        login_ms: n("login_ms").unwrap_or(150.0).max(0.0),
        verify_ms: n("verify_ms").unwrap_or(5.0).max(0.0),
        logins_per_s: n("logins_per_s").unwrap_or(f64::INFINITY).max(0.0),
        verify_local: text(registry, node, "verify").as_deref() == Some("local"),
        max_connections: n("max_connections").unwrap_or(f64::INFINITY).max(1.0),
        pool_size: n("pool_size").unwrap_or(100.0).max(1.0),
        binding: text(registry, node, "binding").unwrap_or_default(),
        edges_per_ms: n("edges_per_ms").unwrap_or(500.0).max(1e-6),
        vectors: n("vectors_m").unwrap_or(1.0).max(0.0) * 1e6,
        points_per_s: n("points_per_s").unwrap_or(f64::INFINITY).max(1e-6),
        series_limit: n("series_limit").unwrap_or(f64::INFINITY),
        transcode: text(registry, node, "mode").as_deref() == Some("transcode"),
        segment_s: n("segment_s").unwrap_or(2.0).max(0.0),
    }
}

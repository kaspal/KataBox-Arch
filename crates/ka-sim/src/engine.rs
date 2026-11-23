//! The discrete-event engine (ADR 0006). One thread, integer µs time, a total event order
//! by (time, sequence), and every random draw from a named seeded stream: the same input
//! and seed always give the same run, bit for bit. `Engine: Clone` is the snapshot used by
//! live-run scrubbing.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::sync::Arc;

use ka_core::rng::Rng;
use ka_core::scenario::{Measure, Source};
use ka_core::{ComponentKind, Protocol, detmath};

use crate::compile::{Compiled, TRACKED_KEYS, Target, edge_protocol, hop_latency_us};
use crate::hist::Hist;
use crate::result::{NodeResult, Percentiles, RunResult, SecondResult, TypeResult};
use crate::route::Role;

const NONE: u32 = u32::MAX;
/// Clients give up on a synchronous request after this long.
pub const TIMEOUT_US: u64 = 5_000_000;
/// After the last arrival, in-flight work may finish for this long.
pub const DRAIN_US: u64 = 30_000_000;
/// A provider that is down fails a call after this connect timeout.
const EXTERNAL_DOWN_US: u64 = 1_000_000;
/// Seconds in a 30.4-day month, for per-request prices.
const MONTH_S: f64 = 2_629_746.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ev {
    Gen(u32),
    Arrive {
        r: u32,
        node: u32,
    },
    SvcDone {
        r: u32,
        node: u32,
        lane: u16,
    },
    /// A call answered or failed; `call` is the call's number on its request, so an answer
    /// that arrives after the caller gave up on that call (S8 call timeout) is ignored.
    CallBack {
        r: u32,
        call: u32,
    },
    CallFail {
        r: u32,
        call: u32,
    },
    /// S8: the caller's `call_timeout_ms` for call `call` ran out.
    CallTimeout {
        r: u32,
        call: u32,
    },
    Timeout {
        r: u32,
    },
    Requeue {
        r: u32,
    },
    /// A message whose consumer died comes back when its lease runs out (a queue's
    /// visibility timeout, a stream's consumer session timeout).
    Redeliver {
        r: u32,
    },
    /// A client tries a failed synchronous request again (S8).
    Retry {
        r: u32,
    },
    /// S8: rollout stage `stage` (1-based) of deploy or config push `chaos` reaches `node`
    /// (a config push stage: every node in stage `stage`'s Region; node = NONE).
    Stage {
        chaos: u16,
        node: u32,
        stage: u16,
    },
    /// S8: a bad change was noticed: roll it back (auto-rollback, kill switch) or halt.
    Noticed {
        chaos: u16,
        node: u32,
    },
    /// S8: a config push's kill switch rolls the change back everywhere.
    Rollback {
        chaos: u16,
    },
    /// S1: the connection model's 100 ms step (only when a type is a connection type).
    ConnTick,
    /// S4: job run `job` starts: its tasks go to its batch cards.
    JobStart {
        job: u16,
    },
    /// S5: a record's window closes (end + watermark): its result goes to the sinks.
    WindowEmit {
        r: u32,
    },
    Second(u32),
    /// Chaos event `i` starts / ends (only scheduled when the scenario has chaos).
    Chaos(u16),
    ChaosEnd(u16),
    /// Health checks every 2 s: routing learns about dead (and revived) cards.
    Health,
    /// A data cluster promotes a replica if its primary is still down.
    Failover(u32),
    /// Autoscaling: evaluate every 5 s; apply a decision after the policy delay.
    ScaleTick,
    ScaleApply {
        group: u32,
        instances: u32,
    },
}

/// Health checks run every 2 s; a change is believed after two checks in a row.
const HEALTH_EVERY_US: u64 = 2_000_000;
const HEALTH_CHECKS: u8 = 2;
/// Autoscaling evaluates the mean utilisation of its cards every 5 s.
const SCALE_EVERY_US: u64 = 5_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Sched {
    t: u64,
    seq: u64,
    ev: Ev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Hop,
    Entry,
    Call,
    BrokerIn,
    Consumer,
}

#[derive(Debug, Clone)]
struct Req {
    ty: u16,
    key: u64,
    start: u64,
    /// Forward network latency of the hop chain: the response pays it again on the way back.
    net: u64,
    /// The node the request last left (for the packet layer's hit and drop marks).
    from: u32,
    phase: Phase,
    seg: u16,
    step: u16,
    caller: u32,
    caller_lane: u16,
    caller_held: bool,
    call_write: bool,
    call_aside: bool,
    /// A cache to fill once the data step after a cache-aside miss answers.
    fill_cache: u32,
    hit_end: bool,
    resp_sent: bool,
    /// A synchronous failure (timeout, drop): later events only clean up.
    dead: bool,
    done: bool,
    pending: u32,
    /// A queue/stream consumer slot reserved at dispatch, taken when the message arrives.
    reserved: bool,
    /// The message holds one of its broker's in-flight slots until it is done or fails.
    in_flight: bool,
    units: f64,
    attempts: u32,
    broker: u32,
    group: u32,
    /// The data card the last write call committed on (the source of a `cdc` step).
    wrote: u32,
    /// When a change-data-capture message was committed (0: not a CDC message).
    cdc_at: u64,
    /// A CDC change on its way to its stream (not yet accepted).
    cdc_pending: bool,
    /// The client source that sent a synchronous request (NONE: none), and which try this
    /// is (0 = the first; S8 client retries).
    src: u32,
    try_n: u32,
    /// S8: the number of the call in progress, whether one is, and its dependency key
    /// (callee card, or EXT_KEY | provider) for circuit breakers and bulkheads.
    call_gen: u32,
    in_call: bool,
    call_key: u32,
    /// S8: served by a faulty instance (a bad deploy or config): fails when service ends.
    bad: bool,
    /// S7: a check-then-write on a hot key (when its first call left: its read), the call in
    /// progress is a lock / an atomic write, a lock is held (from a quorum service), and the
    /// write in progress goes to a self-promoted primary on the far side of a split.
    contended: bool,
    read_at: u64,
    call_lock: bool,
    call_atomic: bool,
    locked: bool,
    lock_quorum: bool,
    shadow: bool,
    /// S2: the request's prompt and answer tokens (drawn at its first model call; -1: not yet).
    tin: f64,
    tout: f64,
    /// S4: a job task (job run `job`, task `task`): its executor time, when it started
    /// running, and whether a spot interruption already freed its executor.
    job: u16,
    task: u32,
    task_us: u64,
    started: u64,
    released: bool,
    /// S4: the batch card a task copy was sent to.
    runner: u32,
}

/// S4: no job.
const NO_JOB: u16 = u16::MAX;

/// S4: one job run: its tasks' sizes (seconds), which are done, the copies running per task,
/// finished task times, and when the last task finished.
#[derive(Debug, Clone)]
struct JobRt {
    ty: u16,
    start: u64,
    sizes: Vec<f64>,
    done: Vec<bool>,
    left: u32,
    running: BTreeMap<u32, Vec<u32>>,
    finished: Vec<f64>,
    completed_at: Option<u64>,
    rr: usize,
}

/// S8: dependency keys of external providers (beside card indices).
const EXT_KEY: u32 = 0x8000_0000;

/// S8: a caller's circuit breaker for one dependency: calls and failures per second over
/// the last 10 s (slot = second % 10, with the second it holds), and when it closes again.
#[derive(Debug, Clone)]
struct BreakerRt {
    window: [(u64, f64, f64); 10],
    open_until: u64,
}

impl Default for BreakerRt {
    fn default() -> Self {
        BreakerRt {
            window: [(u64::MAX, 0.0, 0.0); 10],
            open_until: 0,
        }
    }
}

/// A network split in progress (chaos `split`).
#[derive(Debug, Clone)]
struct SplitRt {
    chaos: u16,
    zone: String,
    since: u64,
    checks: u8,
    /// Health checks have noticed it: routing avoids crossing it.
    known: bool,
}

#[derive(Debug, Clone, Default)]
struct Lane {
    busy: u32,
    queue: VecDeque<u32>,
    busy_since: u64,
    busy_integral: f64,
}

#[derive(Debug, Clone, Default)]
struct NodeRt {
    lanes: Vec<Lane>,
    down: bool,
    /// What health checks have concluded (routing avoids these); lags `down`.
    known_down: bool,
    checks: u8,
    /// Simulated servers per lane and the service-time stretch (sampling), at runtime:
    /// autoscaling changes them.
    lane_servers: u32,
    stretch: f64,
    /// Instances now, and their time integral (cost of autoscaled cards).
    instances: f64,
    inst_integral: f64,
    inst_since: u64,
    /// Busy integral at the last autoscaling tick.
    scale_busy: f64,
    add_us: u64,
    served: f64,
    /// Billed units (ADR 0006): reads and writes, each scaled by the flavour's unit size.
    bill_r: f64,
    bill_w: f64,
    drops: f64,
    max_queue: usize,
    sec_max_queue: usize,
    sec_drops: f64,
    last_integral: f64,
    util_series: Vec<f64>,
    down_seconds: Vec<u32>,
    queue_series: Vec<f64>,
    drop_series: Vec<f64>,
    rr: usize,
    warm: u32,
    /// S8: the share of this card's instances running a bad build or config, what they do
    /// wrong, and whether its LB health checks have ejected it for being overloaded.
    faulty: f64,
    faulty_err: f64,
    faulty_add_us: u64,
    ejected: bool,
    eject_checks: u8,
    /// S2 hosted models: request and token buckets (one minute's limit, refilled
    /// continuously) and when they were last refilled; S2/S1 usage billed by meters.
    rpm_left: f64,
    tpm_left: f64,
    bucket_at: u64,
    bill_tin: f64,
    bill_tout: f64,
    bill_conn_s: f64,
    bill_msgs: f64,
    /// S5: the checkpoint interval the card is in, and records it processed since.
    ckpt_idx: u64,
    since_ckpt: f64,
    /// S6: terabytes scanned (billed by `tb_scanned`).
    bill_tb: f64,
}

#[derive(Debug, Clone, Default)]
struct CacheRt {
    /// Per request type: the last fill time per tracked key rank (i64::MIN: never).
    fills: Vec<Vec<i64>>,
    /// When the cache was last emptied (a flush, or coming back after a kill).
    flushed_at: Option<u64>,
    /// S3: questions a semantic cache has seen since it was last emptied (its fill).
    seen: f64,
}

#[derive(Debug, Clone, Default)]
struct BrokerRt {
    queue: VecDeque<u32>,
    in_flight: u32,
}

#[derive(Debug, Clone, Default)]
struct ExternalRt {
    tokens: f64,
    last: u64,
    down: bool,
    add_ms: f64,
    /// A rate limit cut by chaos (calls per second), replacing the provider's own.
    rate_cut: Option<f64>,
}

#[derive(Debug, Clone)]
struct Group {
    remaining: u32,
    start: u64,
}

#[derive(Debug, Clone, Default)]
struct TypeMetrics {
    requests: f64,
    ok: f64,
    errors: f64,
    shed: f64,
    response: Hist,
    completion: Hist,
    measured: Hist,
    /// S2: time to first token of the requests that called a model.
    ttft: Hist,
}

#[derive(Debug, Clone, Default)]
struct Second {
    requests: f64,
    ok: f64,
    errors: f64,
    measured: Hist,
}

#[derive(Debug, Clone)]
pub struct Engine {
    c: Arc<Compiled>,
    seed: u64,
    now: u64,
    seq: u64,
    heap: BinaryHeap<Reverse<Sched>>,
    rng_arrive: Vec<Rng>,
    rng_key: Vec<Rng>,
    rng_svc: Vec<Rng>,
    rng_misc: Rng,
    nodes: Vec<NodeRt>,
    caches: Vec<Option<CacheRt>>,
    buckets: Vec<BTreeMap<u64, (f64, u64)>>,
    brokers: Vec<BrokerRt>,
    externals: Vec<ExternalRt>,
    cluster_primary: Vec<u32>,
    cluster_rr: Vec<usize>,
    reqs: Vec<Req>,
    free: Vec<u32>,
    groups: Vec<Group>,
    types: Vec<TypeMetrics>,
    seconds: Vec<Second>,
    lost: f64,
    dead_lettered: f64,
    /// Failed attempts that were retried (not errors: the message lives on).
    retried: f64,
    stale_reads: f64,
    reads: f64,
    /// Max traffic multiplier per source (for thinning).
    max_mult: Vec<f64>,
    /// Forward hops recorded for the packet layer (live runs only; never read back).
    trace: Option<Vec<Packet>>,
    /// The cards each chaos event hit (resolved when it starts).
    chaos_targets: Vec<Vec<u32>>,
    /// The last write time per tracked key rank (index = rank; i64::MIN: never). Keys are
    /// shared across request types: rank k is the same item for reads and writes. Empty
    /// until the first write.
    last_write: Vec<i64>,
    /// Autoscaling: a decision waiting for its delay, per group.
    scale_pending: Vec<bool>,
    /// Network splits in progress.
    splits: Vec<SplitRt>,
    /// Per data card: committed changes waiting for their CDC stream, in commit order.
    cdc_backlog: Vec<VecDeque<u32>>,
    /// Keys and commit times of CDC messages in the order streams accepted them (tests).
    cdc_audit: Option<Vec<(u64, u64)>>,
    /// Messages still unfinished at the end have been counted (once).
    censored: bool,
    /// S8 client retries: the backoff jitter stream, and per source the first tries and
    /// retries of the last 10 seconds (slot = second % 10, with the second it holds).
    rng_retry: Rng,
    retry_window: Vec<[(u64, f64, f64); 10]>,
    /// S8: circuit breakers and bulkhead pools per (caller, dependency).
    breakers: BTreeMap<(u32, u32), BreakerRt>,
    bulk_in_flight: BTreeMap<(u32, u32), u32>,
    /// S8: faulty-instance and DNS draws; rollouts halted or rolled back, per (chaos, node);
    /// control-plane outages in progress and the revivals they hold back; a DNS outage's
    /// start; when each failover DNS card saw its primary go.
    rng_s8: Rng,
    rollout_done: BTreeSet<(u16, u32)>,
    control_plane: u32,
    held_revives: Vec<u32>,
    dns_out_since: Option<u64>,
    dns_switch: BTreeMap<u32, u64>,
    /// S7: conflicting writes, and the last commit time of each contended key.
    conflicts: f64,
    hot_commits: BTreeMap<u64, u64>,
    /// S1: per connection type, its fluid state.
    conns: Vec<Option<ConnRt>>,
    conn_drops: f64,
    /// S2: model calls a provider refused with 429 (rate limits), and the token draws.
    rate_limited: f64,
    rng_tokens: Rng,
    /// S4: job runs, task draws, tasks started again, and executors spot chaos took
    /// (chaos, card, executors).
    jobs: Vec<JobRt>,
    rng_jobs: Rng,
    tasks_retried: f64,
    spot_taken: Vec<(u16, u32, u32)>,
    /// S5: windowed records, late ones, duplicate results, and event-delay draws.
    windowed: f64,
    late: f64,
    duplicates: f64,
    rng_s5: Rng,
    /// S9: requests whose data left their residency.
    residency_violations: f64,
}

/// S1: one connection type's state: connections per holder, push backlog per holder,
/// reconnect attempts waiting per 100 ms slot (a ring), the least connected share per
/// second.
#[derive(Debug, Clone)]
struct ConnRt {
    conns: Vec<f64>,
    backlog: Vec<f64>,
    slots: Vec<f64>,
    pending: f64,
    min_share: Vec<f64>,
}

/// S6: opening one file of a table in object storage (a GET and its metadata; model).
const SCAN_FILE_S: f64 = 0.020;

/// S3: what one semantic-cache lookup adds (an embedding and an ANN search; model).
const SEMANTIC_LOOKUP_MS: f64 = 15.0;

/// S1: the connection model's step and its reconnect horizon (2 minutes of slots).
const CONN_TICK_US: u64 = 100_000;
const CONN_SLOTS: usize = 1200;

/// One forward hop of a simulated request, for the live packet layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packet {
    pub from: u32,
    pub to: u32,
    /// The request type's index in the scenario.
    pub ty: u16,
    pub start_us: u64,
    pub travel_us: u64,
    /// Part of the asynchronous life of a message (publish, consume, after the answer).
    pub is_async: bool,
    /// An event at `to` rather than a hop (travel 0): a cache hit, or a request dropped
    /// (full queue, card down).
    pub hit: bool,
    pub drop: bool,
}

/// A card's state at a point in a run, for live frames.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSnap {
    /// Busy server-microseconds since the start (simulated servers).
    pub busy_us: f64,
    /// Simulated servers in total (lanes × servers per lane).
    pub servers: f64,
    /// Requests waiting, weighted back to real traffic.
    pub queue: f64,
    pub down: bool,
}

/// The run's state at the current time, for live frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub t_us: u64,
    pub nodes: Vec<NodeSnap>,
    /// The last completed second, if any.
    pub last_second: Option<SecondResult>,
    /// Monthly cost at the traffic seen so far.
    pub cost_month: f64,
    /// Stale reads over reads so far.
    pub stale_rate: f64,
    /// Failed attempts retried so far (weighted back to real traffic).
    pub retried: f64,
}

impl Engine {
    pub fn new(c: Arc<Compiled>, seed: u64) -> Engine {
        let nsec = (c.duration_us as f64 / 1e6).ceil().max(1.0) as usize;
        let mut nodes: Vec<NodeRt> = c
            .nodes
            .iter()
            .map(|n| NodeRt {
                lanes: vec![Lane::default(); n.spec.lanes as usize],
                ..Default::default()
            })
            .collect();
        for (n, cn) in nodes.iter_mut().zip(&c.nodes) {
            n.rpm_left = cn.spec.rpm * c.sample;
            n.tpm_left = cn.spec.tpm * c.sample;
            n.util_series.reserve(nsec);
            n.lane_servers = cn.lane_servers;
            n.stretch = cn.service_stretch;
            n.instances = cn.spec.instances;
        }
        let chaos_traffic = c
            .scenario
            .chaos
            .iter()
            .filter_map(|e| match e {
                ka_core::scenario::ChaosEvent::Traffic { x, .. } => Some(*x),
                _ => None,
            })
            .fold(1.0, f64::max);
        let max_mult = c
            .sources
            .iter()
            .map(|s| {
                let tm = c
                    .scenario
                    .type_multipliers
                    .get(&c.types[s.ty as usize].def.id)
                    .copied()
                    .unwrap_or(1.0);
                c.scenario.traffic.peak().max(1.0) * chaos_traffic * tm
            })
            .collect();
        let mut e = Engine {
            seed,
            now: 0,
            seq: 0,
            heap: BinaryHeap::new(),
            rng_arrive: c
                .sources
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    Rng::stream(seed, &format!("arrive:{i}:{}", c.nodes[s.node as usize].id))
                })
                .collect(),
            rng_key: c
                .sources
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    Rng::stream(seed, &format!("key:{i}:{}", c.nodes[s.node as usize].id))
                })
                .collect(),
            rng_svc: c
                .nodes
                .iter()
                .map(|n| Rng::stream(seed, &format!("svc:{}", n.id)))
                .collect(),
            rng_misc: Rng::stream(seed, "misc"),
            rng_retry: Rng::stream(seed, "retry"),
            retry_window: vec![[(u64::MAX, 0.0, 0.0); 10]; c.sources.len()],
            breakers: BTreeMap::new(),
            bulk_in_flight: BTreeMap::new(),
            rng_s8: Rng::stream(seed, "s8"),
            rollout_done: BTreeSet::new(),
            control_plane: 0,
            held_revives: Vec::new(),
            dns_out_since: None,
            dns_switch: BTreeMap::new(),
            conflicts: 0.0,
            hot_commits: BTreeMap::new(),
            conns: c
                .types
                .iter()
                .map(|t| {
                    t.conn.as_ref().map(|p| ConnRt {
                        conns: vec![0.0; p.holders.len()],
                        backlog: vec![0.0; p.holders.len()],
                        slots: vec![0.0; CONN_SLOTS],
                        pending: 0.0,
                        min_share: vec![f64::INFINITY; nsec],
                    })
                })
                .collect(),
            conn_drops: 0.0,
            rate_limited: 0.0,
            rng_tokens: Rng::stream(seed, "tokens"),
            jobs: c
                .types
                .iter()
                .enumerate()
                .filter_map(|(i, t)| t.def.job.as_ref().map(|j| (i, j)))
                .flat_map(|(i, j)| {
                    j.starts.iter().map(move |&at| JobRt {
                        ty: i as u16,
                        start: (at.max(0.0) * 1e6) as u64,
                        sizes: Vec::new(),
                        done: Vec::new(),
                        left: 0,
                        running: BTreeMap::new(),
                        finished: Vec::new(),
                        completed_at: None,
                        rr: 0,
                    })
                })
                .collect(),
            rng_jobs: Rng::stream(seed, "jobs"),
            tasks_retried: 0.0,
            spot_taken: Vec::new(),
            windowed: 0.0,
            late: 0.0,
            duplicates: 0.0,
            rng_s5: Rng::stream(seed, "s5"),
            residency_violations: 0.0,
            caches: c
                .nodes
                .iter()
                .map(|n| {
                    matches!(n.kind, ComponentKind::Cache | ComponentKind::Cdn).then(|| CacheRt {
                        fills: vec![Vec::new(); c.types.len()],
                        flushed_at: None,
                        seen: 0.0,
                    })
                })
                .collect(),
            buckets: vec![BTreeMap::new(); c.nodes.len()],
            brokers: vec![BrokerRt::default(); c.nodes.len()],
            externals: c
                .externals
                .iter()
                .map(|x| ExternalRt {
                    tokens: x.rate_limit.unwrap_or(0.0) * c.sample,
                    ..Default::default()
                })
                .collect(),
            cluster_primary: c.clusters.iter().map(|cl| cl.members[0]).collect(),
            cluster_rr: vec![0; c.clusters.len()],
            reqs: Vec::new(),
            free: Vec::new(),
            groups: Vec::new(),
            types: vec![TypeMetrics::default(); c.types.len()],
            seconds: vec![Second::default(); nsec],
            lost: 0.0,
            dead_lettered: 0.0,
            retried: 0.0,
            stale_reads: 0.0,
            reads: 0.0,
            max_mult,
            trace: None,
            chaos_targets: vec![Vec::new(); c.scenario.chaos.len()],
            last_write: Vec::new(),
            scale_pending: vec![false; c.scaling.len()],
            splits: Vec::new(),
            cdc_backlog: vec![VecDeque::new(); c.nodes.len()],
            cdc_audit: None,
            censored: false,
            nodes,
            c,
        };
        if !e.c.scenario.cold_caches {
            e.warm_caches();
        }
        for s in 0..e.c.sources.len() {
            e.schedule_next_arrival(s as u32);
        }
        for sec in 1..=nsec as u32 {
            e.push(u64::from(sec) * 1_000_000, Ev::Second(sec));
        }
        // Chaos, health checks and autoscaling only when the run needs them, so runs without
        // them keep their event order (goldens).
        let chaos: Vec<(u64, u16)> =
            e.c.scenario
                .chaos
                .iter()
                .enumerate()
                .map(|(i, ev)| ((ev.at().max(0.0) * 1e6) as u64, i as u16))
                .collect();
        for (t, i) in chaos {
            e.push(t, Ev::Chaos(i));
        }
        for j in 0..e.jobs.len() {
            if e.jobs[j].start < e.c.duration_us {
                e.push(e.jobs[j].start, Ev::JobStart { job: j as u16 });
            }
        }
        if e.conns.iter().any(Option::is_some) {
            let mut t = 0;
            while t <= e.c.duration_us {
                e.push(t, Ev::ConnTick);
                t += CONN_TICK_US;
            }
        }
        let ejects = e.c.nodes.iter().any(|n| n.spec.health_timeout_us > 0);
        if !e.c.scenario.chaos.is_empty() || !e.c.scaling.is_empty() || ejects {
            let mut t = HEALTH_EVERY_US;
            while t <= e.c.duration_us + DRAIN_US {
                e.push(t, Ev::Health);
                t += HEALTH_EVERY_US;
            }
        }
        if !e.c.scaling.is_empty() {
            let mut t = SCALE_EVERY_US;
            while t <= e.c.duration_us {
                e.push(t, Ev::ScaleTick);
                t += SCALE_EVERY_US;
            }
        }
        e
    }

    pub fn now_us(&self) -> u64 {
        self.now
    }

    pub fn finished(&self) -> bool {
        self.heap.is_empty() || self.now > self.c.duration_us + DRAIN_US
    }

    /// Process every event up to and including time `t` (µs).
    pub fn advance_to(&mut self, t: u64) {
        while let Some(Reverse(s)) = self.heap.peek().copied() {
            if s.t > t || s.t > self.c.duration_us + DRAIN_US {
                break;
            }
            self.heap.pop();
            self.now = s.t;
            self.handle(s.ev);
        }
        self.now = self.now.max(t.min(self.c.duration_us + DRAIN_US));
        if !self.censored && self.now >= self.c.duration_us + DRAIN_US {
            self.censored = true;
            self.censor_unfinished();
        }
    }

    /// Asynchronous work still unfinished when the run ends (a backlog behind a dead or
    /// slow consumer, changes waiting for their stream) counts as completed at the end,
    /// with its age then: a lower bound on its real completion time. Without this, work
    /// that never finishes would simply be missing from the completion percentiles, and a
    /// design whose only consumer dies would look fast. Not lost: it is still in a broker.
    fn censor_unfinished(&mut self) {
        let end = self.now;
        let w = self.c.weight;
        let mut groups_seen = std::collections::BTreeSet::new();
        for i in 0..self.reqs.len() {
            let q = &self.reqs[i];
            if q.pending == u32::MAX / 2 || q.done || q.dead || !q.resp_sent {
                continue;
            }
            let ty = q.ty as usize;
            let mut start = q.start;
            if q.group != NONE {
                if !groups_seen.insert(q.group) {
                    continue;
                }
                start = self.groups[q.group as usize].start;
            }
            if q.seg == 0 && !q.cdc_pending {
                continue;
            }
            let lat = end.saturating_sub(start) as f64;
            let m = &mut self.types[ty];
            m.completion.add(lat, w);
            if self.c.types[ty].def.measure == Measure::Completion {
                m.measured.add(lat, w);
            }
        }
    }

    /// Record forward hops for the packet layer (live runs). Tracing never changes the run.
    pub fn set_trace(&mut self, on: bool) {
        self.trace = on.then(Vec::new);
    }

    /// The hops recorded since the last call.
    pub fn take_packets(&mut self) -> Vec<Packet> {
        self.trace.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// Record the (key, commit time) of every CDC message as its stream accepts it, to
    /// check per-key ordering. Never changes the run.
    pub fn set_cdc_audit(&mut self, on: bool) {
        self.cdc_audit = on.then(Vec::new);
    }

    pub fn cdc_audit(&self) -> &[(u64, u64)] {
        self.cdc_audit.as_deref().unwrap_or_default()
    }

    pub fn duration_us(&self) -> u64 {
        self.c.duration_us
    }

    pub fn compiled(&self) -> &Arc<Compiled> {
        &self.c
    }

    /// The state of every card now, and the last completed second.
    pub fn snapshot(&self) -> Snapshot {
        let c = &self.c;
        let now = self.now;
        // Billed over the test's own length, as the judge does (the drain after it isn't
        // traffic), so a finished run's tile equals its judged cost.
        let elapsed_s = (now.min(self.c.duration_us) as f64 / 1e6).max(1e-6);
        let mut cost_month = 0.0;
        let nodes = c
            .nodes
            .iter()
            .zip(&self.nodes)
            .enumerate()
            .map(|(i, (n, rt))| {
                let mut busy_us = 0.0;
                let mut queue = 0usize;
                for l in &rt.lanes {
                    busy_us += l.busy_integral
                        + f64::from(l.busy) * now.saturating_sub(l.busy_since) as f64;
                    queue += l.queue.len();
                }
                cost_month += self.node_cost(i, elapsed_s);
                NodeSnap {
                    busy_us,
                    servers: f64::from(rt.lane_servers) * f64::from(n.spec.lanes),
                    queue: queue as f64 * c.weight,
                    down: rt.down,
                }
            })
            .collect();
        let done = (now / 1_000_000) as usize;
        let last_second = done
            .checked_sub(1)
            .and_then(|i| self.seconds.get(i).map(|s| (i, s)))
            .map(|(i, s)| SecondResult {
                t: i as u32,
                requests: s.requests,
                ok: s.ok,
                errors: s.errors,
                p95_ms: s.measured.quantile_ms(0.95),
            });
        Snapshot {
            t_us: now,
            nodes,
            last_second,
            cost_month: round2(cost_month),
            retried: self.retried,
            stale_rate: if self.reads > 0.0 {
                self.stale_reads / self.reads
            } else {
                0.0
            },
        }
    }

    pub fn run(&mut self) {
        self.advance_to(self.c.duration_us + DRAIN_US);
    }

    fn push(&mut self, t: u64, ev: Ev) {
        self.seq += 1;
        self.heap.push(Reverse(Sched {
            t,
            seq: self.seq,
            ev,
        }));
    }

    fn push_req(&mut self, t: u64, ev: Ev, r: u32) {
        self.reqs[r as usize].pending += 1;
        self.push(t, ev);
    }

    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::Gen(s) => self.on_gen(s),
            Ev::Arrive { r, node } => {
                self.reqs[r as usize].pending -= 1;
                self.on_arrive(r, node);
            }
            Ev::SvcDone { r, node, lane } => {
                self.reqs[r as usize].pending -= 1;
                self.on_svc_done(r, node, lane);
            }
            Ev::CallBack { r, call } if call != self.reqs[r as usize].call_gen => {
                // An answer to a call the caller already gave up on.
                self.reqs[r as usize].pending -= 1;
                self.maybe_free(r);
            }
            Ev::CallFail { r, call } if call != self.reqs[r as usize].call_gen => {
                self.reqs[r as usize].pending -= 1;
                self.maybe_free(r);
            }
            Ev::CallTimeout { r, call } => {
                self.reqs[r as usize].pending -= 1;
                let q = &self.reqs[r as usize];
                if q.dead || !q.in_call || q.call_gen != call {
                    self.maybe_free(r);
                } else {
                    // The caller stops waiting; the work it asked for runs on, unwanted.
                    self.reqs[r as usize].call_gen += 1;
                    self.call_failed(r);
                }
            }
            Ev::CallBack { r, .. } => {
                self.reqs[r as usize].pending -= 1;
                self.on_callback(r);
            }
            Ev::CallFail { r, .. } => {
                self.reqs[r as usize].pending -= 1;
                self.call_failed(r);
            }
            Ev::Timeout { r } => {
                self.reqs[r as usize].pending -= 1;
                self.on_timeout(r);
            }
            Ev::Chaos(i) => self.chaos_start(i),
            Ev::ChaosEnd(i) => self.chaos_end(i),
            Ev::Health => self.health_check(),
            Ev::Failover(cl) => self.failover(cl),
            Ev::ScaleTick => self.scale_tick(),
            Ev::ScaleApply { group, instances } => self.scale_apply(group, f64::from(instances)),
            Ev::Retry { r } => {
                self.reqs[r as usize].pending -= 1;
                self.on_retry(r);
            }
            Ev::Stage { chaos, node, stage } => self.rollout_stage(chaos, node, stage),
            Ev::Noticed { chaos, node } => self.rollout_noticed(chaos, node),
            Ev::ConnTick => self.conn_tick(),
            Ev::JobStart { job } => self.job_start(job),
            Ev::WindowEmit { r } => {
                self.reqs[r as usize].pending -= 1;
                if self.reqs[r as usize].dead {
                    self.maybe_free(r);
                } else {
                    self.run_calls(r);
                }
            }
            Ev::Rollback { chaos } => {
                for n in self.chaos_targets[chaos as usize].clone() {
                    self.nodes[n as usize].faulty = 0.0;
                }
            }
            Ev::Requeue { r } => {
                self.reqs[r as usize].pending -= 1;
                let b = self.reqs[r as usize].broker;
                self.enqueue_broker(r, b);
            }
            Ev::Redeliver { r } => {
                self.reqs[r as usize].pending -= 1;
                self.redeliver(r);
            }
            Ev::Second(sec) => self.on_second(sec),
        }
    }

    // --- Arrivals ---------------------------------------------------------------------

    fn rate_at(&self, s: u32, t: u64) -> f64 {
        let src = &self.c.sources[s as usize];
        let ty = &self.c.types[src.ty as usize];
        let secs = t as f64 / 1e6;
        let mut x = self.c.scenario.traffic.at(secs, self.c.scenario.duration_s);
        x *= self
            .c
            .scenario
            .type_multipliers
            .get(&ty.def.id)
            .copied()
            .unwrap_or(1.0);
        for e in &self.c.scenario.chaos {
            if let ka_core::scenario::ChaosEvent::Traffic { x: m, at, for_s } = e
                && secs >= *at
                && for_s.is_none_or(|f| secs < at + f)
            {
                x *= m;
            }
        }
        src.rate * self.c.sample * x
    }

    fn schedule_next_arrival(&mut self, s: u32) {
        let max = self.c.sources[s as usize].rate * self.c.sample * self.max_mult[s as usize];
        if max.is_nan() || max <= 0.0 {
            return;
        }
        let gap = self.rng_arrive[s as usize].exponential(1e6 / max).max(1.0) as u64;
        let t = self.now + gap;
        if t < self.c.duration_us {
            self.push(t, Ev::Gen(s));
        }
    }

    fn on_gen(&mut self, s: u32) {
        let max = self.c.sources[s as usize].rate * self.c.sample * self.max_mult[s as usize];
        let accept = self.rate_at(s, self.now) / max;
        if self.rng_arrive[s as usize].uniform() < accept {
            self.start_request(s);
        }
        self.schedule_next_arrival(s);
    }

    fn alloc(&mut self, ty: u16, key: u64) -> u32 {
        let req = Req {
            ty,
            key,
            start: self.now,
            net: 0,
            phase: Phase::Hop,
            seg: 0,
            step: 0,
            caller: NONE,
            caller_lane: 0,
            caller_held: false,
            call_write: false,
            call_aside: false,
            fill_cache: NONE,
            hit_end: false,
            from: NONE,
            resp_sent: false,
            dead: false,
            done: false,
            pending: 0,
            reserved: false,
            in_flight: false,
            units: 1.0,
            attempts: 0,
            broker: NONE,
            group: NONE,
            wrote: NONE,
            cdc_at: 0,
            cdc_pending: false,
            src: NONE,
            try_n: 0,
            call_gen: 0,
            in_call: false,
            call_key: NONE,
            bad: false,
            contended: false,
            read_at: u64::MAX,
            call_lock: false,
            call_atomic: false,
            locked: false,
            lock_quorum: false,
            shadow: false,
            tin: -1.0,
            tout: 0.0,
            job: NO_JOB,
            task: 0,
            task_us: 0,
            started: u64::MAX,
            released: false,
            runner: NONE,
        };
        if let Some(i) = self.free.pop() {
            self.reqs[i as usize] = req;
            i
        } else {
            self.reqs.push(req);
            (self.reqs.len() - 1) as u32
        }
    }

    fn start_request(&mut self, s: u32) {
        let c = Arc::clone(&self.c);
        let src = &c.sources[s as usize];
        let ty = &c.types[src.ty as usize];
        let key = match &ty.zipf {
            Some(z) => z.sample(&mut self.rng_key[s as usize]),
            None => 1 + self.rng_key[s as usize].below(ty.def.key_space.max(1)),
        };
        // S7: a share of a contended type's requests go for one of its few hot keys.
        let mut key = key;
        let mut contended = false;
        if let Some(c) = ty.def.contended
            && self.rng_key[s as usize].uniform() < c.share
        {
            key = 1 + self.rng_key[s as usize].below(c.keys.max(1));
            contended = true;
        }
        let r = self.alloc(src.ty, key);
        self.reqs[r as usize].contended = contended;
        let w = self.c.weight;
        self.types[src.ty as usize].requests += w;
        self.second_mut().requests += w;
        if ty.def.source == Source::Scheduler {
            // The scheduler is the first caller; nothing waits on a response.
            let req = &mut self.reqs[r as usize];
            req.resp_sent = true;
            req.caller = src.node;
            req.phase = Phase::Entry;
            self.run_calls(r);
        } else {
            self.reqs[r as usize].src = s;
            self.count_try(s, false);
            let timeout = self.c.nodes[src.node as usize].spec.timeout_us;
            self.push_req(self.now + timeout, Ev::Timeout { r }, r);
            self.hop_from(r, src.node);
        }
    }

    /// S8: count a first try or a retry in the source's 10-second window.
    fn count_try(&mut self, s: u32, retry: bool) {
        let sec = self.now / 1_000_000;
        let slot = &mut self.retry_window[s as usize][(sec % 10) as usize];
        if slot.0 != sec {
            *slot = (sec, 0.0, 0.0);
        }
        if retry {
            slot.2 += 1.0;
        } else {
            slot.1 += 1.0;
        }
    }

    /// S8: a client's synchronous request failed; try it again if its policy allows.
    /// The new try is a fresh request with the same type, key and start (its latency spans
    /// every try); the failed one only cleans up.
    fn retry_sync(&mut self, r: u32) -> bool {
        let q = &self.reqs[r as usize];
        let s = q.src;
        if s == NONE || q.seg != 0 {
            return false;
        }
        let node = self.c.sources[s as usize].node;
        let spec = &self.c.nodes[node as usize].spec;
        if q.try_n >= spec.retries {
            return false;
        }
        let (budget, backoff_ms, jitter) = (spec.retry_budget, spec.backoff_ms, spec.full_jitter);
        if budget > 0.0 {
            let sec = self.now / 1_000_000;
            let (first, again) = self.retry_window[s as usize]
                .iter()
                .filter(|(at, ..)| *at != u64::MAX && sec.saturating_sub(*at) < 10)
                .fold((0.0, 0.0), |(f, a), (_, x, y)| (f + x, a + y));
            if again + 1.0 > budget * first {
                return false;
            }
        }
        let (ty, key, start, try_n) = (q.ty, q.key, q.start, q.try_n);
        let mut wait = backoff_ms * 1000.0 * f64::from(1u32 << try_n.min(16));
        if jitter {
            wait *= self.rng_retry.uniform();
        }
        // The new try first (so it never reuses the failed one's slot while callers still
        // hold it), then the failed try ends, quietly.
        let r2 = self.alloc(ty, key);
        {
            let q = &mut self.reqs[r as usize];
            q.resp_sent = true;
            q.dead = true;
        }
        self.release_caller(r);
        self.finish(r);
        {
            let q = &mut self.reqs[r2 as usize];
            q.start = start;
            q.src = s;
            q.try_n = try_n + 1;
        }
        self.retried += self.c.weight;
        self.count_try(s, true);
        self.push_req(self.now + wait as u64, Ev::Retry { r: r2 }, r2);
        true
    }

    fn on_retry(&mut self, r: u32) {
        let s = self.reqs[r as usize].src;
        let node = self.c.sources[s as usize].node;
        let timeout = self.c.nodes[node as usize].spec.timeout_us;
        self.push_req(self.now + timeout, Ev::Timeout { r }, r);
        self.hop_from(r, node);
    }

    // --- Hops -------------------------------------------------------------------------

    fn pick(&mut self, from: u32, cands: &[u32], key: u64) -> Option<u32> {
        let live: Vec<u32> = cands
            .iter()
            .copied()
            .filter(|c| {
                let rt = &self.nodes[*c as usize];
                !rt.known_down && !rt.ejected && !self.known_cut(from, *c)
            })
            .collect();
        // S8: every target ejected by health checks for overload: the LB has nowhere to
        // send it (a 503).
        if live.is_empty() && cands.iter().any(|c| self.nodes[*c as usize].ejected) {
            return None;
        }
        let list: &[u32] = if live.is_empty() { cands } else { &live };
        if list.is_empty() {
            return None;
        }
        let n = &self.c.nodes[from as usize];
        let i = if n.kind == ComponentKind::Lb && n.spec.algorithm == "least_conn" {
            let load = |c: u32| -> usize {
                self.nodes[c as usize]
                    .lanes
                    .iter()
                    .map(|l| l.busy as usize + l.queue.len())
                    .sum()
            };
            (0..list.len())
                .min_by_key(|&i| (load(list[i]), i))
                .unwrap_or(0)
        } else if n.kind == ComponentKind::Lb && n.spec.algorithm == "key_hash" {
            (mix(key) % list.len() as u64) as usize
        } else {
            let rt = &mut self.nodes[from as usize];
            rt.rr = rt.rr.wrapping_add(1);
            rt.rr % list.len()
        };
        Some(list[i])
    }

    /// S8 failover DNS: answers point at the first target (in design order) while health
    /// checks find it up, else at the next live one. Clients cache answers for the card's
    /// TTL, so after a switch a shrinking share (uniform cache ages) still goes to the old
    /// target.
    fn dns_pick(&mut self, from: u32, cands: &[u32]) -> Option<u32> {
        let &primary = cands.first()?;
        let up = |e: &Engine, c: u32| !e.nodes[c as usize].known_down && !e.known_cut(from, c);
        if up(self, primary) {
            self.dns_switch.remove(&from);
            return Some(primary);
        }
        let switched = *self.dns_switch.entry(from).or_insert(self.now);
        let ttl = self.c.nodes[from as usize].spec.ttl_s * 1e6;
        let stale = if ttl <= 0.0 {
            0.0
        } else {
            (1.0 - (self.now - switched) as f64 / ttl).max(0.0)
        };
        if self.rng_s8.uniform() < stale {
            return Some(primary);
        }
        cands
            .iter()
            .copied()
            .skip(1)
            .find(|&c| up(self, c))
            .or(Some(primary))
    }

    fn send(&mut self, r: u32, from: u32, to: u32, proto: Protocol, hop: bool) {
        let lat = hop_latency_us(&self.c, from, to, proto);
        self.reqs[r as usize].from = from;
        if let Some(trace) = &mut self.trace {
            let q = &self.reqs[r as usize];
            trace.push(Packet {
                from,
                to,
                ty: q.ty,
                start_us: self.now,
                travel_us: lat,
                is_async: q.resp_sent || matches!(proto, Protocol::Publish | Protocol::Consume),
                hit: false,
                drop: false,
            });
        }
        if hop {
            self.reqs[r as usize].net += lat;
        }
        self.push_req(self.now + lat, Ev::Arrive { r, node: to }, r);
    }

    /// Record a hit or a drop at `at` for the packet layer (no effect on the run).
    fn mark(&mut self, r: u32, from: u32, at: u32, hit: bool) {
        if let Some(trace) = &mut self.trace
            && from != NONE
        {
            let q = &self.reqs[r as usize];
            trace.push(Packet {
                from,
                to: at,
                ty: q.ty,
                start_us: self.now,
                travel_us: 0,
                is_async: q.resp_sent,
                hit,
                drop: !hit,
            });
        }
    }

    fn hop_from(&mut self, r: u32, from: u32) {
        let ty = self.reqs[r as usize].ty as usize;
        let cands = self.c.types[ty].plan.next_hops[from as usize].clone();
        let key = self.reqs[r as usize].key;
        let picked = if self.c.nodes[from as usize].spec.dns_failover {
            self.dns_pick(from, &cands)
        } else {
            self.pick(from, &cands, key)
        };
        let Some(next) = picked else {
            self.fail(r);
            return;
        };
        let entry = self.c.types[ty].plan.entry_ok[next as usize]
            && self.c.nodes[next as usize].spec.role != Role::Edge;
        self.reqs[r as usize].phase = if entry { Phase::Entry } else { Phase::Hop };
        let proto = edge_protocol(&self.c, from, next);
        self.send(r, from, next, proto, true);
    }

    // --- Servers ----------------------------------------------------------------------

    fn lane_of(&self, node: u32, r: u32) -> u16 {
        let lanes = self.c.nodes[node as usize].spec.lanes;
        if lanes <= 1 {
            return 0;
        }
        let q = &self.reqs[r as usize];
        (mix(q.key ^ (u64::from(q.ty) << 48)) % u64::from(lanes)) as u16
    }

    fn set_busy(&mut self, node: u32, lane: u16, delta: i32) {
        let now = self.now;
        let l = &mut self.nodes[node as usize].lanes[lane as usize];
        l.busy_integral += f64::from(l.busy) * (now - l.busy_since) as f64;
        l.busy_since = now;
        l.busy = (l.busy as i64 + i64::from(delta)).max(0) as u32;
    }

    fn on_arrive(&mut self, r: u32, node: u32) {
        if self.reqs[r as usize].dead {
            self.maybe_free(r);
            return;
        }
        let from = self.reqs[r as usize].from;
        let unreachable = self.cut(from, node);
        if self.reqs[r as usize].phase == Phase::BrokerIn {
            if unreachable {
                self.publish_failed(r, node);
            } else {
                self.broker_accept(r, node);
            }
            return;
        }
        // A consumer's reserved slot is used by this arrival only; the calls it makes
        // afterwards queue for their servers like any request.
        let reserved = std::mem::take(&mut self.reqs[r as usize].reserved);
        if self.nodes[node as usize].down || unreachable {
            if reserved {
                self.set_busy(node, 0, -1);
            }
            self.mark(r, from, node, false);
            match self.reqs[r as usize].phase {
                // A message handed to a consumer that died (or was cut off) on the way.
                Phase::Consumer => self.lease_lost(r),
                Phase::Call => self.call_failed(r),
                _ => self.fail(r),
            }
            return;
        }
        let lane = self.lane_of(node, r);
        if reserved {
            self.start_service(r, node, lane);
            return;
        }
        let cap = self.nodes[node as usize].lane_servers;
        // S8 load shedding: past `shed_queue` waiting requests, turn new ones away at once.
        let shed_at = self.c.nodes[node as usize].shed_queue;
        if self.nodes[node as usize].lanes[lane as usize].busy < cap {
            self.set_busy(node, lane, 1);
            self.start_service(r, node, lane);
        } else if self.nodes[node as usize].lanes[lane as usize].queue.len()
            < self.c.nodes[node as usize].lane_queue
            && self.nodes[node as usize].lanes[lane as usize].queue.len() < shed_at
        {
            let rt = &mut self.nodes[node as usize];
            rt.lanes[lane as usize].queue.push_back(r);
            let qlen: usize = rt.lanes.iter().map(|l| l.queue.len()).sum();
            rt.max_queue = rt.max_queue.max(qlen);
            rt.sec_max_queue = rt.sec_max_queue.max(qlen);
            self.reqs[r as usize].pending += 1;
        } else {
            let w = self.c.weight;
            let rt = &mut self.nodes[node as usize];
            rt.drops += w;
            rt.sec_drops += w;
            let from = self.reqs[r as usize].from;
            self.mark(r, from, node, false);
            if self.reqs[r as usize].phase == Phase::Call {
                self.call_failed(r);
            } else {
                self.fail(r);
            }
        }
    }

    fn service_us(&mut self, r: u32, node: u32) -> u64 {
        let q = &self.reqs[r as usize];
        if q.job != NO_JOB && q.phase == Phase::Entry {
            return q.task_us.max(1);
        }
        if self.c.nodes[node as usize].kind == ComponentKind::Infer {
            return self.model_us(r, node);
        }
        if matches!(
            self.c.nodes[node as usize].kind,
            ComponentKind::Warehouse | ComponentKind::Lakehouse
        ) && let Some(scan) = self.c.types[self.reqs[r as usize].ty as usize].def.scan
        {
            return self.scan_us(r, node, scan);
        }
        let n = &self.c.nodes[node as usize];
        let q = &self.reqs[r as usize];
        let ty = &self.c.types[q.ty as usize].def;
        let mut median = n.spec.service_ms;
        if matches!(n.kind, ComponentKind::Sql | ComponentKind::Nosql) {
            median *= 1.0 + ty.payload_kb / 64.0;
        }
        if q.phase == Phase::Call && q.call_write && n.spec.role == Role::Datastore {
            // A write occupies `write_slots` read slots (DynamoDB: 3).
            median *= n.spec.write_slots;
        }
        if q.seg >= 1 {
            median *= q.units;
        }
        if n.kind == ComponentKind::Streamproc && n.spec.window != 0 {
            // S5: a sliding window updates window ÷ slide results per record; state past
            // memory spills to disk (×4, a modelling choice).
            if n.spec.window == 2 {
                median *= (n.spec.window_us as f64 / n.spec.slide_us.max(1) as f64).max(1.0);
            }
            let ct = &self.c.types[q.ty as usize];
            let keys = (ct.def.key_space as f64).min(ct.rate * n.spec.window_us as f64 / 1e6);
            let state_gb = keys * n.spec.state_kb / (1024.0 * 1024.0);
            if state_gb > n.spec.memory_gb * n.spec.instances {
                median *= 4.0;
            }
        }
        if n.spec.semantic {
            // S3: an embedding plus a nearest-neighbour lookup (a modelling choice).
            median += SEMANTIC_LOOKUP_MS;
        }
        let ratio = n.spec.p99_ratio;
        let stretch = self.nodes[node as usize].stretch;
        let cold = n.spec.cold_start_ms;
        let draw = self.rng_svc[node as usize].lognormal(median, ratio);
        let mut us = draw * 1000.0 * stretch;
        if n.kind == ComponentKind::Serverless {
            let busy: u32 = self.nodes[node as usize].lanes.iter().map(|l| l.busy).sum();
            if busy > self.nodes[node as usize].warm {
                self.nodes[node as usize].warm = busy;
                us += cold * 1000.0;
            }
        }
        (us as u64).max(1) + self.nodes[node as usize].add_us
    }

    /// S3: a semantic cache answers a near-repeat once it has seen the question: hit
    /// probability = `semantic_repeat_share` × fill × capacity, where fill = 1 − e^(−seen ÷
    /// distinct questions) (questions seen since the cache was last emptied) and capacity =
    /// min(1, entries that fit ÷ distinct questions). A warm cache starts full.
    fn semantic_hit(&mut self, node: u32, ty: usize) -> bool {
        let def = &self.c.types[ty].def;
        let (share, distinct, value_kb) = (
            def.semantic_repeat_share,
            def.key_space.max(1) as f64,
            def.value_kb,
        );
        let memory = self.c.nodes[node as usize].spec.memory_gb;
        let w = self.c.weight;
        let cold = self.c.scenario.cold_caches;
        let Some(cache) = self.caches[node as usize].as_mut() else {
            return false;
        };
        if cache.flushed_at.is_none() && !cold && cache.seen == 0.0 {
            cache.seen = distinct * 10.0;
        }
        let fill = 1.0 - detmath::exp(-cache.seen / distinct);
        cache.seen += w;
        let capacity = (memory * 1024.0 * 1024.0 / value_kb.max(1e-9) / distinct).min(1.0);
        self.rng_s8.uniform() < share * fill * capacity
    }

    /// S2: the request's tokens, drawn once (lognormal from the mean and 95th percentile).
    fn draw_tokens(&mut self, r: u32) {
        if self.reqs[r as usize].tin >= 0.0 {
            return;
        }
        let def = &self.c.types[self.reqs[r as usize].ty as usize].def;
        let (tin, tout) = (def.tokens_in, def.tokens_out);
        // p99 / median from p95 / mean, for the simulator's lognormal (median, p99 ratio).
        let mut draw = |t: Option<ka_core::scenario::Tokens>| {
            t.map_or(0.0, |t| {
                let ratio = (t.p95 / t.mean.max(1e-9)).max(1.0);
                let r99 = detmath::powf(ratio, 2.326 / 1.645);
                self.rng_tokens
                    .lognormal(t.mean.max(0.0), r99)
                    .round()
                    .max(0.0)
            })
        };
        let (a, b) = (draw(tin), draw(tout));
        let q = &mut self.reqs[r as usize];
        q.tin = a;
        q.tout = b;
    }

    /// S2: a hosted model's per-minute request and token limits (buckets holding one
    /// minute's allowance, refilled continuously; sampling scales them).
    fn model_admit(&mut self, r: u32, node: u32) -> bool {
        self.draw_tokens(r);
        let spec = &self.c.nodes[node as usize].spec;
        if !spec.hosted || spec.batch_api {
            return true;
        }
        let (rpm, tpm, sample) = (
            spec.rpm * self.c.sample,
            spec.tpm * self.c.sample,
            self.c.sample,
        );
        let tokens = self.reqs[r as usize].tin + self.reqs[r as usize].tout;
        let now = self.now;
        let rt = &mut self.nodes[node as usize];
        let dt = now.saturating_sub(rt.bucket_at) as f64 / 60e6;
        rt.bucket_at = now;
        rt.rpm_left = (rt.rpm_left + rpm * dt).min(rpm);
        rt.tpm_left = (rt.tpm_left + tpm * dt).min(tpm);
        if rt.rpm_left < 1.0 || rt.tpm_left < tokens {
            self.rate_limited += 1.0 / sample;
            return false;
        }
        rt.rpm_left -= 1.0;
        rt.tpm_left -= tokens;
        true
    }

    /// S2: time to first token (base + prompt ÷ prefill speed), then the answer at the decode
    /// speed; on GPUs every request in the batch slows the others
    /// (speed × (1 − 0.5 × busy ÷ slots), a modelling choice). Records the request's TTFT.
    fn model_us(&mut self, r: u32, node: u32) -> u64 {
        self.draw_tokens(r);
        let n = &self.c.nodes[node as usize];
        let spec = &n.spec;
        let (tin, mut tout) = (self.reqs[r as usize].tin, self.reqs[r as usize].tout);
        match spec.model_type.as_str() {
            "embed" => tout = 0.0,
            "classify" => tout = tout.min(1.0),
            _ => {}
        }
        let mut speed = spec.tokens_per_s;
        if !spec.hosted {
            let rt = &self.nodes[node as usize];
            let busy: u32 = rt.lanes.iter().map(|l| l.busy).sum();
            let slots = f64::from(rt.lane_servers.max(1)) * rt.lanes.len() as f64;
            speed *= 1.0 - 0.5 * (f64::from(busy) / slots).min(1.0);
        }
        let ttft_s = spec.ttft_ms / 1000.0 + tin / spec.prefill_tps;
        let decode_s = tout / speed.max(1e-6);
        let stretch = self.nodes[node as usize].stretch;
        let add = self.nodes[node as usize].add_us;
        let first = self.now + (ttft_s * 1e6 * stretch) as u64 + add;
        let q = &self.reqs[r as usize];
        let lat = (first - q.start + q.net) as f64;
        let ty = q.ty as usize;
        let w = self.c.weight;
        self.types[ty].ttft.add(lat, w);
        ((ttft_s + decode_s) * 1e6 * stretch) as u64 + add + 1
    }

    fn start_service(&mut self, r: u32, node: u32, lane: u16) {
        if self.reqs[r as usize].job != NO_JOB && self.reqs[r as usize].phase == Phase::Entry {
            self.reqs[r as usize].started = self.now;
        }
        let mut us = self.service_us(r, node);
        let faulty = self.nodes[node as usize].faulty;
        if faulty > 0.0 && self.rng_s8.uniform() < faulty {
            let rt = &self.nodes[node as usize];
            us += rt.faulty_add_us;
            let err = rt.faulty_err;
            self.reqs[r as usize].bad = err > 0.0 && self.rng_s8.uniform() < err;
        }
        self.push_req(self.now + us, Ev::SvcDone { r, node, lane }, r);
    }

    /// Free a slot and start the next queued request on it.
    fn release(&mut self, node: u32, lane: u16) {
        self.set_busy(node, lane, -1);
        let cap = self.nodes[node as usize].lane_servers;
        while self.nodes[node as usize].lanes[lane as usize].busy < cap {
            let Some(next) = self.nodes[node as usize].lanes[lane as usize]
                .queue
                .pop_front()
            else {
                break;
            };
            self.reqs[next as usize].pending -= 1;
            if self.reqs[next as usize].dead {
                self.maybe_free(next);
                continue;
            }
            self.set_busy(node, lane, 1);
            self.start_service(next, node, lane);
        }
        // A consumer slot freed: brokers feeding it may dispatch.
        for b in self.c.feeds[node as usize].clone() {
            self.try_dispatch(b);
        }
    }

    fn release_caller(&mut self, r: u32) {
        let q = &mut self.reqs[r as usize];
        if q.caller_held {
            q.caller_held = false;
            let (c, l) = (q.caller, q.caller_lane);
            self.release(c, l);
        }
    }

    fn on_svc_done(&mut self, r: u32, node: u32, lane: u16) {
        let w = self.c.weight;
        self.nodes[node as usize].served += w;
        let units = w * self.bill_units(node, self.reqs[r as usize].ty as usize);
        if self.c.nodes[node as usize].kind == ComponentKind::Infer {
            let (tin, tout) = (
                self.reqs[r as usize].tin.max(0.0),
                self.reqs[r as usize].tout,
            );
            let rt = &mut self.nodes[node as usize];
            rt.bill_tin += tin * w;
            rt.bill_tout += tout * w;
        }
        let q = &self.reqs[r as usize];
        if q.phase == Phase::Call && q.call_write {
            self.nodes[node as usize].bill_w += units;
        } else {
            self.nodes[node as usize].bill_r += units;
        }
        let phase = self.reqs[r as usize].phase;
        let holds = self.c.nodes[node as usize].spec.holds
            && matches!(phase, Phase::Entry | Phase::Consumer);
        if self.reqs[r as usize].dead {
            // A spot interruption already freed this executor (S4).
            if !std::mem::take(&mut self.reqs[r as usize].released) {
                self.release(node, lane);
            }
            self.maybe_free(r);
            return;
        }
        if std::mem::take(&mut self.reqs[r as usize].bad) && !self.nodes[node as usize].down {
            // S8: a faulty instance (bad deploy or config) got it wrong: an ordinary failure.
            self.release(node, lane);
            if phase == Phase::Call {
                self.call_failed(r);
            } else {
                self.fail(r);
            }
            return;
        }
        if self.nodes[node as usize].down {
            // The card died while working on it. A consumer's message comes back when its
            // lease runs out; anything else fails now.
            self.release(node, lane);
            match phase {
                Phase::Consumer => self.lease_lost(r),
                Phase::Call => self.call_failed(r),
                _ => self.fail(r),
            }
            return;
        }
        if holds {
            let q = &mut self.reqs[r as usize];
            q.caller = node;
            q.caller_lane = lane;
            q.caller_held = true;
            q.step = 0;
        } else {
            self.release(node, lane);
        }
        if phase == Phase::Consumer && self.c.nodes[node as usize].kind == ComponentKind::Streamproc
        {
            self.count_for_checkpoint(node);
            if self.c.nodes[node as usize].spec.window != 0 && self.window_wait(r, node) {
                return;
            }
        }
        match phase {
            Phase::Hop => self.after_hop(r, node),
            Phase::Entry => {
                if self.c.nodes[node as usize].spec.role.is_caller() {
                    self.run_calls(r);
                } else {
                    // Served directly behind the edge (a CDN origin).
                    self.succeed_sync(r);
                }
            }
            Phase::Call => self.after_call(r, node),
            Phase::Consumer => self.run_calls(r),
            Phase::BrokerIn => {}
        }
    }

    fn after_hop(&mut self, r: u32, node: u32) {
        let kind = self.c.nodes[node as usize].kind;
        let ty = self.reqs[r as usize].ty as usize;
        if kind == ComponentKind::Cdn && self.c.types[ty].def.cacheable {
            let key = self.reqs[r as usize].key;
            if self.cache_hit(node, ty, key) {
                self.succeed_sync(r);
                return;
            }
            // The origin's answer fills the edge on its way back.
            self.fill(node, ty, key);
        }
        if kind == ComponentKind::Ratelimit && !self.take_token(r, node) {
            self.shed(r);
            return;
        }
        if kind == ComponentKind::Dns
            && let Some(since) = self.dns_out_since
        {
            // S8 DNS outage: answers cached within the TTL still work (cache ages are
            // uniform), the rest can't resolve.
            let ttl = self.c.nodes[node as usize].spec.ttl_s * 1e6;
            let expired = if ttl <= 0.0 {
                1.0
            } else {
                ((self.now - since) as f64 / ttl).min(1.0)
            };
            if self.rng_s8.uniform() < expired {
                self.fail(r);
                return;
            }
        }
        self.hop_from(r, node);
    }

    // --- Calls ------------------------------------------------------------------------

    fn run_calls(&mut self, r: u32) {
        loop {
            let q = &self.reqs[r as usize];
            if q.dead {
                self.release_caller(r);
                self.maybe_free(r);
                return;
            }
            let (ty, seg, step, caller) =
                (q.ty as usize, q.seg as usize, q.step as usize, q.caller);
            let Some(calls) = self.c.types[ty].plan.calls[seg][caller as usize].as_ref() else {
                self.fail(r);
                return;
            };
            if step >= calls.len() || q.hit_end {
                self.segment_end(r);
                return;
            }
            let targets = calls[step].clone();
            if targets.is_empty() {
                self.reqs[r as usize].step += 1;
                continue;
            }
            let def = &self.c.types[ty].route.segments[seg].calls[step];
            let (writes, aside) = (def.writes(), def.is_cache_aside());
            let (lock, atomic) = (def.lock, def.alts.iter().any(|a| a.atomic));
            // Callers avoid targets health checks have found dead. A cache-aside read whose
            // caches are all known dead is skipped: the app treats it as a miss and reads
            // the store (which then takes the full load).
            let live: Vec<Target> = targets
                .iter()
                .copied()
                .filter(|t| self.target_alive(*t, caller))
                .collect();
            if live.is_empty() && aside {
                self.reqs[r as usize].step += 1;
                continue;
            }
            let targets = if live.is_empty() { targets } else { live };
            let pick = {
                let rt = &mut self.nodes[caller as usize];
                rt.rr = rt.rr.wrapping_add(1);
                targets[rt.rr % targets.len()]
            };
            let node = match pick {
                Target::External(x) => {
                    if !self.begin_call(r, caller, EXT_KEY | x) {
                        return;
                    }
                    self.call_external(r, x);
                    return;
                }
                Target::Node(n) => n,
                Target::Cluster(cl) => {
                    let p = self.cluster_member(cl, writes, caller);
                    if writes && !lock {
                        self.shadow_primary(cl, p, caller).unwrap_or(p)
                    } else {
                        p
                    }
                }
            };
            if lock
                && self.c.nodes[node as usize].kind == ComponentKind::Coord
                && !self.quorum(node, caller)
            {
                // S7: this side of a split can't reach a majority of the members.
                self.reqs[r as usize].call_key = NONE;
                self.call_failed(r);
                return;
            }
            if !self.begin_call(r, caller, node) {
                return;
            }
            if self.c.nodes[node as usize].kind == ComponentKind::Infer
                && !self.model_admit(r, node)
            {
                // S2: the provider answers 429 at once.
                let call = self.reqs[r as usize].call_gen;
                self.push_req(self.now + 50_000, Ev::CallFail { r, call }, r);
                return;
            }
            let shadow = writes
                && self.c.cluster_of[node as usize]
                    .is_some_and(|cl| self.cluster_primary[cl as usize] != node);
            let now = self.now;
            let q = &mut self.reqs[r as usize];
            q.phase = Phase::Call;
            q.call_write = writes;
            q.call_aside = aside;
            q.call_lock = lock;
            q.call_atomic = atomic;
            q.shadow = shadow && !lock;
            if q.contended && q.read_at == u64::MAX {
                q.read_at = now;
            }
            let proto = match self.c.nodes[node as usize].spec.role {
                Role::Cache | Role::Datastore => Protocol::Query,
                _ => edge_protocol(&self.c, caller, node),
            };
            self.send(r, caller, node, proto, false);
            return;
        }
    }

    /// S8: a call is about to leave `caller` for dependency `key`. An open circuit breaker or
    /// a full bulkhead pool fails it at once (returns false, having handled the failure);
    /// otherwise the call is numbered, counted in its pool and given its timeout.
    fn begin_call(&mut self, r: u32, caller: u32, key: u32) -> bool {
        let spec = &self.c.nodes[caller as usize].spec;
        let (pct, bulkhead, timeout) = (spec.breaker_pct, spec.bulkhead, spec.call_timeout_us);
        if pct > 0.0
            && self
                .breakers
                .get(&(caller, key))
                .is_some_and(|b| self.now < b.open_until)
        {
            self.reqs[r as usize].call_key = NONE;
            self.call_failed(r);
            return false;
        }
        if bulkhead {
            let deps = self.c.deps[caller as usize].max(1) as f64;
            let servers = f64::from(self.nodes[caller as usize].lane_servers)
                * self.nodes[caller as usize].lanes.len() as f64;
            let cap = ((servers / deps).floor() as u32).max(1);
            let used = self.bulk_in_flight.entry((caller, key)).or_insert(0);
            if *used >= cap {
                self.reqs[r as usize].call_key = NONE;
                self.call_failed(r);
                return false;
            }
            *used += 1;
        }
        let q = &mut self.reqs[r as usize];
        q.call_gen += 1;
        q.in_call = true;
        q.call_key = key;
        let call = q.call_gen;
        if timeout > 0 {
            self.push_req(self.now + timeout, Ev::CallTimeout { r, call }, r);
        }
        true
    }

    /// S8: the call in progress ended (answered, failed or given up on): free its bulkhead
    /// slot and tell its circuit breaker.
    fn end_call(&mut self, r: u32, ok: bool) {
        let q = &mut self.reqs[r as usize];
        if !q.in_call {
            return;
        }
        q.in_call = false;
        let (caller, key) = (q.caller, q.call_key);
        if caller == NONE || key == NONE {
            return;
        }
        let spec = &self.c.nodes[caller as usize].spec;
        let (pct, open_us, bulkhead) = (spec.breaker_pct, spec.breaker_open_us, spec.bulkhead);
        if bulkhead && let Some(n) = self.bulk_in_flight.get_mut(&(caller, key)) {
            *n = n.saturating_sub(1);
        }
        if pct > 0.0 {
            let sec = self.now / 1_000_000;
            let now = self.now;
            let b = self.breakers.entry((caller, key)).or_default();
            if now >= b.open_until && b.open_until > 0 {
                // Closed again after its open time: start counting afresh.
                *b = BreakerRt::default();
            }
            let slot = &mut b.window[(sec % 10) as usize];
            if slot.0 != sec {
                *slot = (sec, 0.0, 0.0);
            }
            slot.1 += 1.0;
            if !ok {
                slot.2 += 1.0;
                let (calls, fails) = b
                    .window
                    .iter()
                    .filter(|(at, ..)| *at != u64::MAX && sec.saturating_sub(*at) < 10)
                    .fold((0.0, 0.0), |(c, f), (_, x, y)| (c + x, f + y));
                if calls >= 20.0 * self.c.sample.clamp(0.05, 1.0) && fails >= pct * calls {
                    b.open_until = now + open_us;
                }
            }
        }
    }

    /// A call failed (an error, a timeout, an open breaker, a full bulkhead). An optional
    /// `service?` dependency falls back: the step is skipped and the request goes on.
    /// Anything else fails the request, as before M8.
    fn call_failed(&mut self, r: u32) {
        self.end_call(r, false);
        let q = &self.reqs[r as usize];
        if !q.dead {
            let (ty, seg, step) = (q.ty as usize, q.seg as usize, q.step as usize);
            let def = self.c.types[ty].route.segments[seg].calls.get(step);
            if def.is_some_and(|d| d.optional && d.alts.iter().all(|a| a.service)) {
                self.reqs[r as usize].step += 1;
                self.run_calls(r);
                return;
            }
        }
        self.fail(r);
    }

    /// S7: does `caller`'s side reach a majority of the coordination cluster's members
    /// (each card counts its `instances`)?
    fn quorum(&self, node: u32, caller: u32) -> bool {
        let members: Vec<u32> = match self.c.cluster_of[node as usize] {
            Some(cl) => self.c.clusters[cl as usize].members.clone(),
            None => vec![node],
        };
        let size = |m: u32| self.c.nodes[m as usize].spec.instances;
        let total: f64 = members.iter().map(|&m| size(m)).sum();
        let reach: f64 = members
            .iter()
            .filter(|&&m| !self.nodes[m as usize].down && !self.cut(caller, m))
            .map(|&m| size(m))
            .sum();
        reach * 2.0 > total
    }

    /// S7 split-brain: with `split_failover = auto`, a caller cut off from the primary by a
    /// split (once health checks know, plus the primary's `failover_s`) writes to the
    /// member on its own side that promoted itself.
    fn shadow_primary(&self, cl: u32, primary: u32, caller: u32) -> Option<u32> {
        let spec = &self.c.nodes[primary as usize].spec;
        if !spec.split_failover || caller == NONE || !self.known_cut(caller, primary) {
            return None;
        }
        let since = self
            .splits
            .iter()
            .filter(|s| s.known)
            .map(|s| s.since)
            .min()?;
        let after = since
            + HEALTH_EVERY_US * u64::from(HEALTH_CHECKS)
            + (spec.failover_s.max(0.0) * 1e6) as u64;
        if self.now < after {
            return None;
        }
        self.c.clusters[cl as usize]
            .members
            .iter()
            .copied()
            .find(|&m| m != primary && !self.nodes[m as usize].down && !self.cut(caller, m))
    }

    /// S7: a write commits. On a self-promoted primary it is refused with a fencing token
    /// from a quorum lock, else it is a conflict; on a contended key, a write whose check
    /// came before another write's commit raced, unless it was atomic or held a lock.
    /// Returns false if the write was refused (the call failed).
    fn s7_write_ok(&mut self, r: u32, node: u32, key: u64) -> bool {
        let w = self.c.weight;
        let q = &self.reqs[r as usize];
        let (shadow, quorum, contended, read_at, atomic, locked) = (
            q.shadow,
            q.lock_quorum,
            q.contended,
            q.read_at,
            q.call_atomic,
            q.locked,
        );
        if shadow {
            if self.c.nodes[node as usize].spec.fencing && quorum {
                self.call_failed(r);
                return false;
            }
            self.conflicts += w;
        }
        if contended {
            let raced = self.hot_commits.get(&key).is_some_and(|&t| t > read_at);
            if raced && !atomic && !locked {
                self.conflicts += w;
            }
            // An atomic write that lost the race is refused cleanly ("sold out"): an answer.
            self.hot_commits.insert(key, self.now);
        }
        true
    }

    /// Does routing believe this call target is up, and reachable from `caller`?
    fn target_alive(&self, t: Target, caller: u32) -> bool {
        let ok = |m: u32| !self.nodes[m as usize].known_down && !self.known_cut(caller, m);
        match t {
            Target::Node(n) => ok(n),
            Target::Cluster(cl) => self.c.clusters[cl as usize].members.iter().any(|m| ok(*m)),
            Target::External(_) => true,
        }
    }

    fn cluster_member(&mut self, cl: u32, write: bool, caller: u32) -> u32 {
        let primary = self.cluster_primary[cl as usize];
        if write || self.c.nodes[primary as usize].spec.read_from_primary {
            return primary;
        }
        let members = &self.c.clusters[cl as usize].members;
        let mut live: Vec<u32> = members
            .iter()
            .copied()
            .filter(|m| !self.nodes[*m as usize].known_down && !self.known_cut(caller, *m))
            .collect();
        if live.is_empty() {
            return primary;
        }
        // Reads stay in the caller's Region when a live member is there (the nearest replica).
        let region = |n: u32| self.c.nodes[n as usize].region;
        if caller != NONE && region(caller).is_some() {
            let local: Vec<u32> = live
                .iter()
                .copied()
                .filter(|&m| region(m) == region(caller))
                .collect();
            if !local.is_empty() {
                live = local;
            }
        }
        self.cluster_rr[cl as usize] = self.cluster_rr[cl as usize].wrapping_add(1);
        live[self.cluster_rr[cl as usize] % live.len()]
    }

    fn after_call(&mut self, r: u32, node: u32) {
        let (ty, key) = (self.reqs[r as usize].ty as usize, self.reqs[r as usize].key);
        let q = &self.reqs[r as usize];
        let (write, aside, fill_cache) = (q.call_write, q.call_aside, q.fill_cache);
        let w = self.c.weight;
        let role = if q.call_lock {
            // S7: a lock grant, not a write of data.
            let quorum = self.c.nodes[node as usize].kind == ComponentKind::Coord;
            let q = &mut self.reqs[r as usize];
            q.locked = true;
            q.lock_quorum |= quorum;
            Role::None
        } else {
            self.c.nodes[node as usize].spec.role
        };
        if write && role == Role::Datastore && !self.s7_write_ok(r, node, key) {
            return;
        }
        if matches!(role, Role::Datastore | Role::Cache) {
            self.check_residency(r, node);
        }
        match role {
            // A write-through: the cache holds the new value for every reader of the key.
            Role::Cache if write => {
                self.record_write(key);
                for t in 0..self.c.types.len() {
                    if t == ty || self.has_fills(node, t) {
                        self.fill(node, t, key);
                    }
                }
            }
            Role::Cache if aside => {
                let hit = if self.c.nodes[node as usize].spec.semantic {
                    self.semantic_hit(node, ty)
                } else {
                    self.cache_hit(node, ty, key)
                };
                if hit {
                    // One read, answered by the cache: stale if the key was written after
                    // the cache filled it (cache-aside doesn't invalidate).
                    self.reads += w;
                    if self.fill_before_write(node, ty, key) {
                        self.stale_reads += w;
                    }
                    self.reqs[r as usize].hit_end = true;
                    let caller = self.reqs[r as usize].caller;
                    self.mark(r, caller, node, true);
                } else {
                    self.reqs[r as usize].fill_cache = node;
                }
            }
            Role::Cache => {
                // A cache used as the store.
                self.reads += w;
            }
            Role::Datastore => {
                if write {
                    self.record_write(key);
                    self.reqs[r as usize].wrote = node;
                } else {
                    // One read, answered by the store: stale on a replica that hasn't caught
                    // up with the key's last write.
                    self.reads += w;
                    if self.replica_behind(node, key) {
                        self.stale_reads += w;
                    }
                }
                if fill_cache != NONE {
                    self.fill(fill_cache, ty, key);
                    self.reqs[r as usize].fill_cache = NONE;
                }
            }
            _ => {}
        }
        let caller = self.reqs[r as usize].caller;
        if self.cut(node, caller) {
            // A split opened while the call ran: the answer never arrives.
            let call = self.reqs[r as usize].call_gen;
            self.push_req(self.now + EXTERNAL_DOWN_US, Ev::CallFail { r, call }, r);
            return;
        }
        let lat = hop_latency_us(&self.c, node, caller, Protocol::Query);
        let call = self.reqs[r as usize].call_gen;
        self.push_req(self.now + lat, Ev::CallBack { r, call }, r);
    }

    fn on_callback(&mut self, r: u32) {
        if self.reqs[r as usize].dead {
            self.maybe_free(r);
            return;
        }
        self.end_call(r, true);
        let caller = self.reqs[r as usize].caller;
        if caller != NONE && self.nodes[caller as usize].down {
            // The caller died while waiting: a synchronous request is lost with it; a
            // consumer's message comes back when its lease runs out.
            if self.reqs[r as usize].seg >= 1 {
                self.lease_lost(r);
            } else {
                self.fail(r);
            }
            return;
        }
        self.reqs[r as usize].step += 1;
        self.run_calls(r);
    }

    fn call_external(&mut self, r: u32, x: u32) {
        let def = self.c.externals[x as usize].clone();
        let now = self.now;
        let sample = self.c.sample;
        let ext = &mut self.externals[x as usize];
        if ext.down {
            let call = self.reqs[r as usize].call_gen;
            self.push_req(now + EXTERNAL_DOWN_US, Ev::CallFail { r, call }, r);
            return;
        }
        if let Some(limit) = ext.rate_cut.or(def.rate_limit) {
            let rate = limit * sample;
            ext.tokens = (ext.tokens + rate * (now - ext.last) as f64 / 1e6).min(rate.max(1.0));
            ext.last = now;
            if ext.tokens < 1.0 {
                let fast = (def.median_ms * 1000.0) as u64;
                let call = self.reqs[r as usize].call_gen;
                self.push_req(now + fast, Ev::CallFail { r, call }, r);
                return;
            }
            ext.tokens -= 1.0;
        }
        let add = ext.add_ms;
        let ms = self.rng_misc.lognormal(
            def.median_ms,
            (def.p99_ms / def.median_ms.max(1e-9)).max(1.0),
        ) + add;
        let call = self.reqs[r as usize].call_gen;
        self.push_req(now + (ms * 1000.0) as u64, Ev::CallBack { r, call }, r);
    }

    // --- Segment ends, brokers --------------------------------------------------------

    fn segment_end(&mut self, r: u32) {
        let q = &self.reqs[r as usize];
        let (ty, seg, caller) = (q.ty as usize, q.seg as usize, q.caller);
        let nseg = self.c.types[ty].route.segments.len();
        // An optional hand-off with no broker wired: the caller goes on with the next
        // segment's calls itself, still holding its slot (the answer waits for them).
        if !q.hit_end
            && seg + 1 < nseg
            && caller != NONE
            && self.c.types[ty].route.segments[seg + 1]
                .broker
                .as_ref()
                .is_some_and(|b| b.optional)
            && self.c.types[ty].plan.brokers[seg + 1][caller as usize].is_empty()
        {
            let q = &mut self.reqs[r as usize];
            q.seg += 1;
            q.step = 0;
            self.run_calls(r);
            return;
        }
        self.release_caller(r);
        let q = &self.reqs[r as usize];
        if q.hit_end || seg + 1 >= nseg {
            if !self.reqs[r as usize].resp_sent {
                self.succeed_sync(r);
            } else {
                self.complete_async(r);
            }
            return;
        }
        let cdc = self.c.types[ty].route.segments[seg + 1]
            .broker
            .as_ref()
            .is_some_and(|b| b.cdc);
        if cdc {
            // Change data capture: the caller's answer leaves at the commit; the database
            // publishes the change.
            if !self.reqs[r as usize].resp_sent {
                self.record_response(r, self.now);
            }
            let q = &mut self.reqs[r as usize];
            q.cdc_at = self.now.max(1);
            q.cdc_pending = true;
            q.caller = NONE;
            let src = q.wrote;
            self.cdc_publish(r, src);
            return;
        }
        let brokers = self.c.types[ty].plan.brokers[seg + 1][caller as usize].clone();
        let key = self.reqs[r as usize].key;
        let Some(b) = self.pick(caller, &brokers, key) else {
            self.fail(r);
            return;
        };
        self.reqs[r as usize].phase = Phase::BrokerIn;
        self.send(r, caller, b, Protocol::Publish, false);
    }

    /// The CDC stream a change on `src` goes to: chosen by key among the source's streams,
    /// so one key always takes the same one. None if it can't be reached now.
    fn cdc_target(&self, r: u32, src: u32) -> Option<Option<u32>> {
        let q = &self.reqs[r as usize];
        let streams = &self.c.types[q.ty as usize].plan.brokers[q.seg as usize + 1][src as usize];
        if streams.is_empty() {
            // This card has no CDC edge (a promoted replica without one): nothing publishes.
            return None;
        }
        let b = streams[(mix(q.key) % streams.len() as u64) as usize];
        let reachable = !self.nodes[b as usize].down && !self.cut(src, b);
        Some(reachable.then_some(b))
    }

    /// Publish a committed change from data card `src`, keeping per-card commit order: if
    /// changes are already waiting, it waits behind them.
    fn cdc_publish(&mut self, r: u32, src: u32) {
        if src == NONE {
            self.cdc_lost(r);
            return;
        }
        if !self.cdc_backlog[src as usize].is_empty() {
            self.cdc_backlog[src as usize].push_back(r);
            return;
        }
        match self.cdc_target(r, src) {
            None => self.cdc_lost(r),
            Some(None) => self.cdc_backlog[src as usize].push_back(r),
            Some(Some(b)) => {
                self.reqs[r as usize].phase = Phase::BrokerIn;
                self.send(r, src, b, Protocol::Publish, false);
            }
        }
    }

    /// Send waiting changes whose stream is reachable again, in order; changes older than
    /// the card's `cdc_buffer_s` are lost.
    fn cdc_flush(&mut self, src: u32) {
        let buffer = (self.c.nodes[src as usize].spec.cdc_buffer_s.max(0.0) * 1e6) as u64;
        while let Some(&r) = self.cdc_backlog[src as usize].front() {
            if self.now.saturating_sub(self.reqs[r as usize].cdc_at) > buffer {
                self.cdc_backlog[src as usize].pop_front();
                self.cdc_lost(r);
                continue;
            }
            match self.cdc_target(r, src) {
                None => {
                    self.cdc_backlog[src as usize].pop_front();
                    self.cdc_lost(r);
                }
                Some(None) => break,
                Some(Some(b)) => {
                    self.cdc_backlog[src as usize].pop_front();
                    self.reqs[r as usize].phase = Phase::BrokerIn;
                    self.send(r, src, b, Protocol::Publish, false);
                }
            }
        }
    }

    fn cdc_lost(&mut self, r: u32) {
        self.lost += self.c.weight;
        self.finish(r);
    }

    /// A publish reached a broker that is down, full or cut off. A CDC change goes back to
    /// the front of its database's backlog (delayed, not lost); anything else fails.
    fn publish_failed(&mut self, r: u32, b: u32) {
        let from = self.reqs[r as usize].from;
        self.mark(r, from, b, false);
        let q = &self.reqs[r as usize];
        if q.cdc_pending && q.wrote != NONE {
            // Back into the backlog in commit order (several may bounce at once).
            let (src, at) = (q.wrote, q.cdc_at);
            let backlog = &self.cdc_backlog[src as usize];
            let i = backlog
                .iter()
                .position(|&o| self.reqs[o as usize].cdc_at > at)
                .unwrap_or(backlog.len());
            self.cdc_backlog[src as usize].insert(i, r);
        } else {
            self.fail(r);
        }
    }

    /// The consumer holding a queue or stream message died: the message comes back when
    /// its lease runs out (a queue's `visibility_timeout_s`, a stream's
    /// `session_timeout_s`), holding its broker slot until then. Other brokers fail it.
    fn lease_lost(&mut self, r: u32) {
        let q = &self.reqs[r as usize];
        let b = q.broker;
        if !q.resp_sent || b == NONE || q.group != NONE {
            self.fail(r);
            return;
        }
        let spec = &self.c.nodes[b as usize].spec;
        let timeout_s = match self.c.nodes[b as usize].kind {
            ComponentKind::Queue => spec.visibility_timeout_s,
            ComponentKind::Stream => spec.session_timeout_s,
            _ => {
                self.fail(r);
                return;
            }
        };
        self.release_caller(r);
        let q = &mut self.reqs[r as usize];
        q.step = 0;
        q.caller = NONE;
        q.hit_end = false;
        q.fill_cache = NONE;
        q.reserved = false;
        let at = self.now + (timeout_s.max(0.0) * 1e6) as u64;
        self.push_req(at, Ev::Redeliver { r }, r);
    }

    /// A lease ran out: the broker slot frees and the message counts an attempt.
    fn redeliver(&mut self, r: u32) {
        self.message_done(r);
        let w = self.c.weight;
        let q = &mut self.reqs[r as usize];
        let b = q.broker;
        q.attempts += 1;
        if q.attempts <= self.c.nodes[b as usize].spec.max_retries {
            self.retried += w;
            // The retry is one more receive.
            self.nodes[b as usize].bill_r += w;
            self.enqueue_broker(r, b);
        } else if let Some(d) = self.c.dlq_of[b as usize] {
            self.dead_lettered += w;
            self.nodes[d as usize].bill_w += w;
            self.finish(r);
        } else {
            self.lost += w;
            self.finish(r);
        }
    }

    /// Does a split separate cards `a` and `b` now? (Cards outside every zone are on the
    /// outside of each split.)
    fn cut(&self, a: u32, b: u32) -> bool {
        if a == NONE || b == NONE || self.splits.is_empty() {
            return false;
        }
        self.splits
            .iter()
            .any(|s| self.in_zone(a, &s.zone) != self.in_zone(b, &s.zone))
    }

    /// A split routing has learned about (after two health checks).
    fn known_cut(&self, a: u32, b: u32) -> bool {
        if a == NONE || b == NONE || self.splits.is_empty() {
            return false;
        }
        self.splits
            .iter()
            .any(|s| s.known && self.in_zone(a, &s.zone) != self.in_zone(b, &s.zone))
    }

    fn broker_accept(&mut self, r: u32, b: u32) {
        if self.nodes[b as usize].down {
            self.publish_failed(r, b);
            return;
        }
        self.check_residency(r, b);
        let depth = self.brokers[b as usize].queue.len() as f64;
        if depth >= (self.c.nodes[b as usize].spec.max_depth * self.c.sample).max(1.0) {
            let w = self.c.weight;
            self.nodes[b as usize].drops += w;
            self.nodes[b as usize].sec_drops += w;
            self.publish_failed(r, b);
            return;
        }
        if std::mem::take(&mut self.reqs[r as usize].cdc_pending) {
            let q = &self.reqs[r as usize];
            let entry = (q.key, q.cdc_at);
            if let Some(audit) = &mut self.cdc_audit {
                audit.push(entry);
            }
        }
        let w = self.c.weight;
        self.nodes[b as usize].served += w;
        self.nodes[b as usize].bill_w += w * self.bill_units(b, self.reqs[r as usize].ty as usize);
        let caller = self.reqs[r as usize].caller;
        if !self.reqs[r as usize].resp_sent {
            // The publish is acknowledged once the broker has stored it (its median publish
            // latency, calibrated per flavour); the client's answer leaves then.
            let store = (self.c.nodes[b as usize].spec.service_ms * 1000.0).round() as u64;
            let ack = hop_latency_us(&self.c, b, caller, Protocol::Publish) + store;
            self.record_response(r, self.now + ack);
        }
        // The message's async life starts.
        let ty = self.reqs[r as usize].ty as usize;
        let units = match self.c.types[ty].def.fanout {
            Some(f) => {
                f.mean
                    * if self.rng_misc.uniform() < f.heavy_share {
                        f.heavy_x
                    } else {
                        1.0
                    }
            }
            None => 1.0,
        };
        let q = &mut self.reqs[r as usize];
        q.seg += 1;
        q.step = 0;
        q.broker = b;
        q.attempts = 0;
        q.units = units;
        q.caller = NONE;
        q.reserved = false;
        q.in_flight = false;
        if self.c.nodes[b as usize].kind == ComponentKind::Pubsub {
            self.fan_out(r, b);
        } else {
            self.enqueue_broker(r, b);
        }
    }

    fn enqueue_broker(&mut self, r: u32, b: u32) {
        self.reqs[r as usize].reserved = false;
        self.reqs[r as usize].in_flight = false;
        self.reqs[r as usize].pending += 1;
        self.brokers[b as usize].queue.push_back(r);
        let len = self.brokers[b as usize].queue.len();
        let rt = &mut self.nodes[b as usize];
        rt.max_queue = rt.max_queue.max(len);
        rt.sec_max_queue = rt.sec_max_queue.max(len);
        self.try_dispatch(b);
    }

    /// Pub/sub pushes one copy to every subscriber; the message completes when all do.
    fn fan_out(&mut self, r: u32, b: u32) {
        let (ty, seg) = (
            self.reqs[r as usize].ty as usize,
            self.reqs[r as usize].seg as usize,
        );
        let subs = self.c.types[ty].plan.consumers[seg][b as usize].clone();
        if subs.is_empty() {
            self.fail(r);
            return;
        }
        // Each delivery is billed as a read (free on SNS).
        let units = self.c.weight * self.bill_units(b, ty) * subs.len() as f64;
        self.nodes[b as usize].bill_r += units;
        let g = self.groups.len() as u32;
        self.groups.push(Group {
            remaining: subs.len() as u32,
            start: self.reqs[r as usize].start,
        });
        for (i, s) in subs.iter().enumerate() {
            let copy = if i == 0 {
                r
            } else {
                let mut c = self.reqs[r as usize].clone();
                c.pending = 0;
                let id = self.alloc(c.ty, c.key);
                self.reqs[id as usize] = c;
                id
            };
            let q = &mut self.reqs[copy as usize];
            q.group = g;
            q.phase = Phase::Consumer;
            self.send(copy, b, *s, Protocol::Consume, false);
        }
    }

    fn try_dispatch(&mut self, b: u32) {
        let partitions = self.c.nodes[b as usize].spec.partitions;
        let cap = if partitions.is_finite() {
            (partitions * self.c.sample).round().max(1.0) as u32
        } else {
            u32::MAX
        };
        if self.nodes[b as usize].down {
            return;
        }
        while let Some(&r) = self.brokers[b as usize].queue.front() {
            if self.c.nodes[b as usize].kind == ComponentKind::Stream
                && self.brokers[b as usize].in_flight >= cap
            {
                break;
            }
            let (ty, seg) = (
                self.reqs[r as usize].ty as usize,
                self.reqs[r as usize].seg as usize,
            );
            let consumers = &self.c.types[ty].plan.consumers[seg][b as usize];
            let n = consumers.len();
            let start = self.nodes[b as usize].rr;
            let mut chosen = None;
            for k in 0..n {
                let c = consumers[(start + k) % n];
                let rt = &self.nodes[c as usize];
                if !rt.down && rt.lanes[0].busy < rt.lane_servers && !self.cut(b, c) {
                    chosen = Some(c);
                    self.nodes[b as usize].rr = start + k + 1;
                    break;
                }
            }
            let Some(c) = chosen else { break };
            self.brokers[b as usize].queue.pop_front();
            self.reqs[r as usize].pending -= 1;
            self.brokers[b as usize].in_flight += 1;
            self.set_busy(c, 0, 1);
            let q = &mut self.reqs[r as usize];
            q.reserved = true;
            q.in_flight = true;
            q.phase = Phase::Consumer;
            self.send(r, b, c, Protocol::Consume, false);
        }
    }

    /// The broker's in-flight slot for a dispatched message frees (done or failed).
    fn message_done(&mut self, r: u32) {
        let q = &mut self.reqs[r as usize];
        if q.in_flight && q.broker != NONE {
            q.in_flight = false;
            let b = q.broker;
            self.brokers[b as usize].in_flight =
                self.brokers[b as usize].in_flight.saturating_sub(1);
            self.try_dispatch(b);
        }
    }

    // --- Outcomes ---------------------------------------------------------------------

    fn record_response(&mut self, r: u32, at: u64) {
        let w = self.c.weight;
        let q = &mut self.reqs[r as usize];
        q.resp_sent = true;
        let lat = (at - q.start + q.net) as f64;
        let ty = q.ty as usize;
        let measure = self.c.types[ty].def.measure;
        let m = &mut self.types[ty];
        m.ok += w;
        m.response.add(lat, w);
        if measure == Measure::Response {
            m.measured.add(lat, w);
        }
        if at <= self.c.duration_us {
            let s = self.second_mut();
            s.ok += w;
            if measure == Measure::Response {
                s.measured.add(lat, w);
            }
        }
    }

    fn succeed_sync(&mut self, r: u32) {
        self.record_response(r, self.now);
        let ty = self.reqs[r as usize].ty as usize;
        if self.c.types[ty].def.measure == Measure::Completion {
            // No async part: completion is the response.
            let lat = (self.now - self.reqs[r as usize].start + self.reqs[r as usize].net) as f64;
            let w = self.c.weight;
            self.types[ty].completion.add(lat, w);
            self.types[ty].measured.add(lat, w);
            if self.now <= self.c.duration_us {
                self.second_mut().measured.add(lat, w);
            }
        }
        self.finish(r);
    }

    fn complete_async(&mut self, r: u32) {
        if self.reqs[r as usize].job != NO_JOB {
            self.job_task_done(r);
        }
        self.message_done(r);
        let q = &self.reqs[r as usize];
        let (ty, g) = (q.ty as usize, q.group);
        let mut start = q.start;
        if g != NONE {
            let grp = &mut self.groups[g as usize];
            grp.remaining -= 1;
            if grp.remaining > 0 {
                self.finish(r);
                return;
            }
            start = grp.start;
        }
        let lat = (self.now - start) as f64;
        let w = self.c.weight;
        let scheduler = self.c.types[ty].def.source == Source::Scheduler;
        let m = &mut self.types[ty];
        m.completion.add(lat, w);
        if scheduler {
            m.ok += w;
        }
        let measured = self.c.types[ty].def.measure == Measure::Completion;
        if measured {
            self.types[ty].measured.add(lat, w);
        }
        if self.now <= self.c.duration_us {
            let s = self.second_mut();
            if scheduler {
                s.ok += w;
            }
            if measured {
                s.measured.add(lat, w);
            }
        }
        self.finish(r);
    }

    fn shed(&mut self, r: u32) {
        let ty = self.reqs[r as usize].ty as usize;
        let w = self.c.weight;
        if self.c.types[ty].def.abusive {
            self.types[ty].shed += w;
            self.reqs[r as usize].resp_sent = true;
            self.finish(r);
        } else {
            self.fail(r);
        }
    }

    fn fail(&mut self, r: u32) {
        let w = self.c.weight;
        self.end_call(r, false);
        if self.reqs[r as usize].job != NO_JOB && !self.reqs[r as usize].dead {
            self.job_task_failed(r);
            return;
        }
        if !self.reqs[r as usize].resp_sent && self.retry_sync(r) {
            return;
        }
        if !self.reqs[r as usize].resp_sent {
            let ty = self.reqs[r as usize].ty as usize;
            self.types[ty].errors += w;
            if self.now <= self.c.duration_us {
                self.second_mut().errors += w;
            }
            let q = &mut self.reqs[r as usize];
            q.resp_sent = true;
            q.dead = true;
            self.release_caller(r);
            self.finish(r);
            return;
        }
        // An async message failed: retry, then dead-letter, else lost.
        self.release_caller(r);
        self.message_done(r);
        let q = &mut self.reqs[r as usize];
        let b = q.broker;
        q.attempts += 1;
        q.step = 0;
        q.caller = NONE;
        q.hit_end = false;
        q.fill_cache = NONE;
        if b == NONE {
            self.lost += w;
            self.finish(r);
            return;
        }
        let retries = self.c.nodes[b as usize].spec.max_retries;
        if q.attempts <= retries {
            self.retried += w;
            self.nodes[b as usize].bill_r += w;
            let backoff = 1_000_000u64 << (q.attempts - 1).min(5);
            self.push_req(self.now + backoff, Ev::Requeue { r }, r);
        } else if let Some(d) = self.c.dlq_of[b as usize] {
            self.dead_lettered += w;
            self.nodes[d as usize].bill_w += w;
            self.finish(r);
        } else {
            self.lost += w;
            self.finish(r);
        }
    }

    fn on_timeout(&mut self, r: u32) {
        if !self.reqs[r as usize].resp_sent {
            self.fail(r);
        } else {
            self.maybe_free(r);
        }
    }

    fn finish(&mut self, r: u32) {
        self.reqs[r as usize].done = true;
        self.maybe_free(r);
    }

    fn maybe_free(&mut self, r: u32) {
        let q = &self.reqs[r as usize];
        if q.pending == 0 && (q.done || q.dead) {
            self.reqs[r as usize].done = false;
            self.reqs[r as usize].dead = false;
            self.reqs[r as usize].pending = u32::MAX / 2; // poisoned until reused
            self.free.push(r);
        }
    }

    // --- Caches and rate limits -------------------------------------------------------

    fn capacity_keys(&self, node: u32, ty: usize) -> u64 {
        let n = &self.c.nodes[node as usize];
        if n.kind == ComponentKind::Cdn {
            return u64::MAX;
        }
        let value_kb = self.c.types[ty].def.value_kb.max(0.001);
        (n.spec.memory_gb * 1024.0 * 1024.0 / value_kb) as u64
    }

    fn tracked_len(&self, node: u32, ty: usize) -> u64 {
        self.capacity_keys(node, ty)
            .min(self.c.types[ty].def.key_space)
            .min(TRACKED_KEYS)
    }

    fn ttl_us(&self, node: u32) -> u64 {
        let ttl = self.c.nodes[node as usize].spec.ttl_s;
        if ttl <= 0.0 {
            u64::MAX / 4
        } else {
            (ttl * 1e6) as u64
        }
    }

    fn cache_hit(&mut self, node: u32, ty: usize, key: u64) -> bool {
        if key > self.capacity_keys(node, ty) {
            return false;
        }
        let tracked = self.tracked_len(node, ty);
        let ttl = self.ttl_us(node);
        let now = self.now as i64;
        if key <= tracked {
            let cache = self.caches[node as usize].as_mut().expect("cache state");
            let fills = &mut cache.fills[ty];
            if fills.is_empty() {
                return false;
            }
            let f = fills[(key - 1) as usize];
            return f != i64::MIN && now - f < ttl as i64;
        }
        // A cold key beyond the tracked ones: the chance it was requested within its TTL.
        let t = &self.c.types[ty];
        let pmf = if t.skew > 0.0 {
            detmath::powf(key as f64, -t.skew) / t.harmonic
        } else {
            1.0 / t.def.key_space as f64
        };
        let mut window_us = ttl.min(3_600_000_000);
        if let Some(at) = self.caches[node as usize]
            .as_ref()
            .and_then(|c| c.flushed_at)
        {
            window_us = window_us.min(self.now.saturating_sub(at));
        }
        let window = window_us as f64 / 1e6;
        let p = 1.0 - detmath::exp(-t.rate * pmf * window);
        self.rng_misc.uniform() < p
    }

    fn record_write(&mut self, key: u64) {
        if key == 0 || key > TRACKED_KEYS {
            return;
        }
        if self.last_write.is_empty() {
            self.last_write = vec![i64::MIN; TRACKED_KEYS as usize + 1];
        }
        self.last_write[key as usize] = self.now as i64;
    }

    fn last_write_of(&self, key: u64) -> i64 {
        self.last_write
            .get(key as usize)
            .copied()
            .unwrap_or(i64::MIN)
    }

    /// Is this a replica whose copy of `key` is older than the key's last write?
    fn replica_behind(&self, node: u32, key: u64) -> bool {
        let Some(cl) = self.c.cluster_of[node as usize] else {
            return false;
        };
        if self.cluster_primary[cl as usize] == node {
            return false;
        }
        let written = self.last_write_of(key);
        if written == i64::MIN {
            return false;
        }
        // Cut off from its primary by a split: it has nothing written since the split.
        let primary = self.cluster_primary[cl as usize];
        if let Some(s) = self
            .splits
            .iter()
            .find(|s| self.in_zone(node, &s.zone) != self.in_zone(primary, &s.zone))
            && written >= s.since as i64
        {
            return true;
        }
        let lag_us = (self.c.nodes[node as usize].spec.replication_lag_ms * 1000.0) as i64;
        (self.now as i64) - written < lag_us
    }

    /// Was the cached copy of `key` filled before the key's last write? (Tracked keys only:
    /// the analytic tail has no per-key fill times.)
    fn fill_before_write(&self, node: u32, ty: usize, key: u64) -> bool {
        let written = self.last_write_of(key);
        if written == i64::MIN || key == 0 {
            return false;
        }
        let Some(cache) = self.caches[node as usize].as_ref() else {
            return false;
        };
        match cache.fills[ty].get((key - 1) as usize) {
            Some(&f) if f != i64::MIN => f < written,
            _ => false,
        }
    }

    fn has_fills(&self, node: u32, ty: usize) -> bool {
        self.caches[node as usize]
            .as_ref()
            .is_some_and(|c| !c.fills[ty].is_empty())
    }

    fn fill(&mut self, node: u32, ty: usize, key: u64) {
        let tracked = self.tracked_len(node, ty);
        if key == 0 || key > tracked {
            return;
        }
        let now = self.now as i64;
        let cache = self.caches[node as usize].as_mut().expect("cache state");
        if cache.fills[ty].is_empty() {
            cache.fills[ty] = vec![i64::MIN; tracked as usize];
        }
        cache.fills[ty][(key - 1) as usize] = now;
    }

    /// Caches start warm: every key expected at least once per TTL is present, with
    /// staggered ages (no artificial expiry wave).
    fn warm_caches(&mut self) {
        for node in 0..self.c.nodes.len() as u32 {
            if self.caches[node as usize].is_none() {
                continue;
            }
            let ttl_us = self.ttl_us(node);
            let window = (ttl_us.min(3_600_000_000) as f64) / 1e6;
            for ty in 0..self.c.types.len() {
                let t = &self.c.types[ty];
                if self.c.nodes[node as usize].kind == ComponentKind::Cdn && !t.def.cacheable {
                    continue;
                }
                let tracked = self.tracked_len(node, ty);
                let mut fills = vec![i64::MIN; tracked as usize];
                for k in 1..=tracked {
                    let pmf = if t.skew > 0.0 {
                        detmath::powf(k as f64, -t.skew) / t.harmonic
                    } else {
                        1.0 / t.def.key_space as f64
                    };
                    if t.rate * pmf * window < 1.0 {
                        break;
                    }
                    let u =
                        (mix(k ^ (u64::from(node) << 40)) >> 11) as f64 / 9_007_199_254_740_992.0;
                    fills[(k - 1) as usize] = -((u * window * 1e6) as i64);
                }
                self.caches[node as usize].as_mut().expect("cache").fills[ty] = fills;
            }
        }
    }

    fn take_token(&mut self, r: u32, node: u32) -> bool {
        let spec = &self.c.nodes[node as usize].spec;
        if !spec.limit_rps.is_finite() {
            return true;
        }
        let q = &self.reqs[r as usize];
        let def = &self.c.types[q.ty as usize].def;
        let id = match def.api_keys {
            Some(k) if k > 0 => (u64::from(q.ty) << 40) | (q.key % u64::from(k)),
            _ => u64::from(q.ty) << 40,
        };
        let rate = spec.limit_rps * self.c.sample;
        let cap = (spec.burst * self.c.sample).max(1.0);
        let now = self.now;
        let b = self.buckets[node as usize].entry(id).or_insert((cap, now));
        b.0 = (b.0 + rate * (now - b.1) as f64 / 1e6).min(cap);
        b.1 = now;
        if b.0 >= 1.0 {
            b.0 -= 1.0;
            true
        } else {
            false
        }
    }

    // --- Seconds and results ----------------------------------------------------------

    fn second_mut(&mut self) -> &mut Second {
        let i = ((self.now / 1_000_000) as usize).min(self.seconds.len() - 1);
        &mut self.seconds[i]
    }

    fn on_second(&mut self, _sec: u32) {
        // Changes waiting for a CDC stream: send what can go, expire what waited too long.
        for src in 0..self.cdc_backlog.len() as u32 {
            if !self.cdc_backlog[src as usize].is_empty() {
                self.cdc_flush(src);
            }
        }
        let now = self.now;
        for (i, rt) in self.nodes.iter_mut().enumerate() {
            let mut integral = 0.0;
            for l in &mut rt.lanes {
                l.busy_integral += f64::from(l.busy) * (now - l.busy_since) as f64;
                l.busy_since = now;
                integral += l.busy_integral;
            }
            let cap = f64::from(rt.lane_servers) * f64::from(self.c.nodes[i].spec.lanes) * 1e6;
            rt.util_series
                .push(((integral - rt.last_integral) / cap).clamp(0.0, 1.0));
            rt.last_integral = integral;
            rt.queue_series
                .push(rt.sec_max_queue as f64 * self.c.weight);
            rt.drop_series.push(rt.sec_drops);
            if rt.down {
                rt.down_seconds.push(rt.util_series.len() as u32 - 1);
            }
            let qlen: usize = rt.lanes.iter().map(|l| l.queue.len()).sum();
            rt.sec_max_queue = qlen;
            rt.sec_drops = 0.0;
        }
    }

    /// The run's results so far (complete once `finished()`).
    pub fn result(&self) -> RunResult {
        let c = &self.c;
        let dur = c.scenario.duration_s;
        let pct = |h: &Hist| Percentiles {
            p50_ms: h.quantile_ms(0.5),
            p95_ms: h.quantile_ms(0.95),
            p99_ms: h.quantile_ms(0.99),
        };
        let (mut measured, mut response, mut completion) =
            (Hist::default(), Hist::default(), Hist::default());
        let mut ttft = Hist::default();
        let (mut req, mut ok, mut err, mut shed) = (0.0, 0.0, 0.0, 0.0);
        let mut per_type = Vec::new();
        for (i, m) in self.types.iter().enumerate() {
            measured.merge(&m.measured);
            response.merge(&m.response);
            completion.merge(&m.completion);
            req += m.requests;
            ok += m.ok;
            err += m.errors;
            shed += m.shed;
            per_type.push(TypeResult {
                id: c.types[i].def.id.clone(),
                requests: m.requests,
                errors: m.errors,
                latency: pct(&m.measured),
                response: pct(&m.response),
                completion: (m.completion.total() > 0.0).then(|| pct(&m.completion)),
                ttft: (m.ttft.total() > 0.0).then(|| pct(&m.ttft)),
            });
            ttft.merge(&m.ttft);
        }
        let done = ok + err + shed;
        let mut cost_total = 0.0;
        let nodes: Vec<NodeResult> = c
            .nodes
            .iter()
            .zip(&self.nodes)
            .enumerate()
            .map(|(i, (n, rt))| {
                let secs = rt.util_series.len().max(1) as f64;
                let utilisation = rt.util_series.iter().sum::<f64>() / secs;
                let cost = self.node_cost(i, dur);
                cost_total += cost;
                NodeResult {
                    id: n.id.clone(),
                    kind: n.kind,
                    utilisation,
                    max_queue: rt.max_queue as f64 * c.weight,
                    drops: rt.drops,
                    served: rt.served,
                    cost_month: round2(cost),
                    util_series: rt.util_series.clone(),
                    queue_series: rt.queue_series.clone(),
                    drop_series: rt.drop_series.clone(),
                    down_seconds: rt.down_seconds.clone(),
                }
            })
            .collect();
        let series = self
            .seconds
            .iter()
            .enumerate()
            .map(|(i, s)| SecondResult {
                t: i as u32,
                requests: s.requests,
                ok: s.ok,
                errors: s.errors,
                p95_ms: s.measured.quantile_ms(0.95),
            })
            .collect();
        RunResult {
            seed: self.seed,
            duration_s: dur,
            sample_factor: c.sample,
            requests: req,
            ok,
            errors: err,
            shed,
            error_rate: if done > 0.0 { err / done } else { 0.0 },
            availability: if done > 0.0 { (ok + shed) / done } else { 1.0 },
            throughput_rps: ok / dur,
            latency: pct(&measured),
            response: pct(&response),
            completion: (completion.total() > 0.0).then(|| pct(&completion)),
            lost: self.lost,
            dead_lettered: self.dead_lettered,
            retried: self.retried,
            stale_reads: self.stale_reads,
            stale_rate: if self.reads > 0.0 {
                self.stale_reads / self.reads
            } else {
                0.0
            },
            conflicts: self.conflicts,
            connected_min: self.connected_min(),
            conn_drops: self.conn_drops,
            ttft: (ttft.total() > 0.0).then(|| pct(&ttft)),
            rate_limited: self.rate_limited,
            late: self.late,
            late_rate: if self.windowed > 0.0 {
                self.late / self.windowed
            } else {
                0.0
            },
            duplicates: self.duplicates,
            residency_violations: self.residency_violations,
            jobs: self
                .jobs
                .iter()
                .map(|j| ka_sim_job_result(&c.types[j.ty as usize].def.id, j))
                .collect(),
            tasks_retried: self.tasks_retried,
            cost_month: round2(cost_total),
            per_type,
            nodes,
            series,
        }
    }
}

// --- Chaos, health checks, failover, autoscaling (M1.5) ------------------------------------

impl Engine {
    /// The cards a selector names now: by id, or by kind and cluster role; the first in id
    /// order unless `all`.
    fn resolve(&self, sel: &ka_core::scenario::Selector) -> Vec<u32> {
        use ka_core::scenario::Role as R;
        let mut hits: Vec<u32> = (0..self.c.nodes.len() as u32)
            .filter(|&i| {
                let n = &self.c.nodes[i as usize];
                if let Some(id) = &sel.id {
                    return &n.id == id;
                }
                if sel.kind.is_some_and(|k| k != n.kind) {
                    return false;
                }
                let primary = match self.c.cluster_of[i as usize] {
                    Some(cl) => self.cluster_primary[cl as usize] == i,
                    None => true,
                };
                match sel.role {
                    Some(R::Primary) => primary,
                    Some(R::Replica) => !primary,
                    Some(R::Any) | None => true,
                }
            })
            .collect();
        hits.sort_by(|a, b| {
            self.c.nodes[*a as usize]
                .id
                .cmp(&self.c.nodes[*b as usize].id)
        });
        if !sel.all {
            hits.truncate(1);
        }
        hits
    }

    /// A chaos zone as written, or `@primary`: the Region holding the current primary of
    /// the design's main data store (its first SQL cluster, else its first NoSQL one, by
    /// card id). With no Region around that primary the whole design is the zone (`*`),
    /// so leaving Regions out can't dodge a region test.
    fn resolve_zone(&self, zone: &str) -> String {
        if zone != "@primary" {
            return zone.to_owned();
        }
        let primary = [ComponentKind::Sql, ComponentKind::Nosql]
            .iter()
            .find_map(|k| {
                self.c
                    .clusters
                    .iter()
                    .enumerate()
                    .filter(|(_, cl)| self.c.nodes[cl.members[0] as usize].kind == *k)
                    .map(|(i, _)| self.cluster_primary[i])
                    .min_by(|a, b| {
                        self.c.nodes[*a as usize]
                            .id
                            .cmp(&self.c.nodes[*b as usize].id)
                    })
            });
        primary
            .and_then(|p| self.c.nodes[p as usize].region)
            .map_or_else(|| "*".to_owned(), |z| self.c.zones[z as usize].id.clone())
    }

    fn in_zone(&self, node: u32, zone: &str) -> bool {
        let n = &self.c.nodes[node as usize];
        if zone == "*" {
            // The whole design, except the cards that stand for the outside world.
            return !matches!(
                n.kind,
                ComponentKind::Client | ComponentKind::Dns | ComponentKind::Cdn
            );
        }
        [n.az, n.region].into_iter().flatten().any(|z| {
            let z = &self.c.zones[z as usize];
            z.id == zone || z.label == zone
        })
    }

    fn chaos_start(&mut self, i: u16) {
        use ka_core::scenario::ChaosEvent as C;
        let ev = self.c.scenario.chaos[i as usize].clone();
        let mut targets = Vec::new();
        let mut for_s = None;
        match &ev {
            C::Kill {
                target, for_s: f, ..
            } => {
                targets = self.resolve(target);
                for &n in &targets {
                    self.kill(n);
                }
                for_s = *f;
            }
            C::Partition { zone, for_s: f, .. } => {
                let zone = self.resolve_zone(zone);
                targets = (0..self.c.nodes.len() as u32)
                    .filter(|&n| self.in_zone(n, &zone))
                    .collect();
                for &n in &targets {
                    self.kill(n);
                }
                for_s = *f;
            }
            C::Latency {
                target,
                add_ms,
                for_s: f,
                ..
            } => {
                targets = self.resolve(target);
                let add = (add_ms.max(0.0) * 1000.0) as u64;
                for &n in &targets {
                    self.nodes[n as usize].add_us += add;
                }
                for_s = *f;
            }
            C::Flush { target, .. } => {
                for n in self.resolve(target) {
                    self.flush(n);
                }
            }
            C::External {
                name,
                for_s: f,
                down,
                add_ms,
                rate,
                ..
            } => {
                if let Some(x) = self.c.externals.iter().position(|x| &x.name == name) {
                    let sample = self.c.sample;
                    let ext = &mut self.externals[x];
                    ext.down |= *down;
                    ext.add_ms += add_ms.max(0.0);
                    if let Some(rate) = rate {
                        let rate = rate.max(0.0);
                        ext.rate_cut = Some(rate);
                        ext.tokens = ext.tokens.min((rate * sample).max(1.0));
                    }
                }
                for_s = *f;
            }
            C::Split { zone, for_s: f, .. } => {
                let zone = self.resolve_zone(zone);
                self.splits.push(SplitRt {
                    chaos: i,
                    zone,
                    since: self.now,
                    checks: 0,
                    known: false,
                });
                for_s = *f;
            }
            // Traffic is read from the scenario when arrivals are generated.
            C::Traffic { .. } => {}
            C::Deploy {
                target,
                error_rate,
                add_ms,
                for_s: f,
                ..
            } => {
                targets = self
                    .resolve(target)
                    .into_iter()
                    .filter(|&n| self.c.nodes[n as usize].spec.role.is_caller())
                    .collect();
                for &n in &targets {
                    let rt = &mut self.nodes[n as usize];
                    rt.faulty_err = error_rate.clamp(0.0, 1.0);
                    rt.faulty_add_us = (add_ms.max(0.0) * 1000.0) as u64;
                    self.rollout_stage(i, n, 1);
                }
                for_s = *f;
            }
            C::ConfigPush {
                error_rate,
                add_ms,
                for_s: f,
                ..
            } => {
                targets = (0..self.c.nodes.len() as u32)
                    .filter(|&n| self.c.nodes[n as usize].spec.role.is_caller())
                    .collect();
                for &n in &targets {
                    let rt = &mut self.nodes[n as usize];
                    rt.faulty_err = error_rate.clamp(0.0, 1.0);
                    rt.faulty_add_us = (add_ms.max(0.0) * 1000.0) as u64;
                }
                self.chaos_targets[i as usize] = targets.clone();
                self.rollout_stage(i, NONE, 1);
                for_s = *f;
            }
            C::DnsOutage { for_s: f, .. } => {
                self.dns_out_since = Some(self.now);
                for_s = *f;
            }
            C::ControlPlane { for_s: f, .. } => {
                self.control_plane += 1;
                for_s = *f;
            }
            C::Spot {
                target,
                share,
                for_s: f,
                ..
            } => {
                targets = self
                    .resolve(target)
                    .into_iter()
                    .filter(|&n| self.c.nodes[n as usize].kind == ComponentKind::Batch)
                    .collect();
                let t = targets.clone();
                self.spot_start(i, &t, *share);
                for_s = *f;
            }
        }
        self.chaos_targets[i as usize] = targets;
        if let Some(f) = for_s {
            let end = self.now + (f.max(0.0) * 1e6) as u64;
            self.push(end, Ev::ChaosEnd(i));
        }
    }

    fn chaos_end(&mut self, i: u16) {
        use ka_core::scenario::ChaosEvent as C;
        let ev = self.c.scenario.chaos[i as usize].clone();
        let targets = std::mem::take(&mut self.chaos_targets[i as usize]);
        match &ev {
            C::Kill { .. } | C::Partition { .. } => {
                for n in targets {
                    if self.control_plane > 0 {
                        // No replacement instances while the control plane is down.
                        self.held_revives.push(n);
                    } else {
                        self.revive(n);
                    }
                }
            }
            C::Deploy { .. } | C::ConfigPush { .. } => {
                // Rolled back by hand.
                for n in targets {
                    self.nodes[n as usize].faulty = 0.0;
                    self.rollout_done.insert((i, n));
                }
                self.rollout_done.insert((i, NONE));
            }
            C::DnsOutage { .. } => self.dns_out_since = None,
            C::Spot { .. } => self.spot_end(i),
            C::ControlPlane { .. } => {
                self.control_plane = self.control_plane.saturating_sub(1);
                if self.control_plane == 0 {
                    for n in std::mem::take(&mut self.held_revives) {
                        self.revive(n);
                    }
                }
            }
            C::Latency { add_ms, .. } => {
                let add = (add_ms.max(0.0) * 1000.0) as u64;
                for n in targets {
                    let rt = &mut self.nodes[n as usize];
                    rt.add_us = rt.add_us.saturating_sub(add);
                }
            }
            C::External {
                name,
                down,
                add_ms,
                rate,
                ..
            } => {
                if let Some(x) = self.c.externals.iter().position(|x| &x.name == name) {
                    let ext = &mut self.externals[x];
                    if *down {
                        ext.down = false;
                    }
                    ext.add_ms = (ext.add_ms - add_ms.max(0.0)).max(0.0);
                    if rate.is_some() {
                        ext.rate_cut = None;
                    }
                }
            }
            C::Split { .. } => {
                self.splits.retain(|s| s.chaos != i);
                // Work held back by the split may move again.
                for b in 0..self.c.nodes.len() as u32 {
                    if self.c.nodes[b as usize].spec.role == Role::Broker {
                        self.try_dispatch(b);
                    }
                }
                for src in 0..self.cdc_backlog.len() as u32 {
                    if !self.cdc_backlog[src as usize].is_empty() {
                        self.cdc_flush(src);
                    }
                }
            }
            C::Flush { .. } | C::Traffic { .. } => {}
        }
    }

    /// S1: the traffic multiplier for request type `ty` at time `t` (curve, type multiplier,
    /// traffic chaos), as arrivals use.
    fn traffic_mult(&self, ty: usize, t: u64) -> f64 {
        let secs = t as f64 / 1e6;
        let def = &self.c.types[ty].def;
        let mut x = self.c.scenario.traffic.at(secs, self.c.scenario.duration_s)
            * self
                .c
                .scenario
                .type_multipliers
                .get(&def.id)
                .copied()
                .unwrap_or(1.0);
        for e in &self.c.scenario.chaos {
            if let ka_core::scenario::ChaosEvent::Traffic { x: m, at, for_s } = e
                && secs >= *at
                && for_s.is_none_or(|f| secs < at + f)
            {
                x *= m;
            }
        }
        x
    }

    /// S1, every 100 ms: connections follow their target; a dead holder's connections
    /// reconnect, spread over the client's jitter; holders admit what their capacity and
    /// accept rate allow, the rest retry after the client's backoff (counted as drops); each
    /// holder pushes `push_rps` per connection against its message rate, and a backlog
    /// delays delivery (the type's measured latency).
    fn conn_tick(&mut self) {
        let now = self.now;
        let tick = (now / CONN_TICK_US) as usize;
        let dt = CONN_TICK_US as f64 / 1e6;
        let sec = (now / 1_000_000) as usize;
        for ti in 0..self.conns.len() {
            let Some(mut rt) = self.conns[ti].take() else {
                continue;
            };
            let c = Arc::clone(&self.c);
            let ty = &c.types[ti];
            let plan = ty.conn.as_ref().expect("a connection type");
            let def = &ty.def;
            let client = &c.nodes[plan.client as usize].spec;
            let slot_of = |k: usize| k % CONN_SLOTS;
            let spread = |rt: &mut ConnRt, count: f64, from: usize, width_s: f64| {
                let width = ((width_s / dt).round() as usize).clamp(1, CONN_SLOTS - 1);
                let first = from.min(tick + CONN_SLOTS - 1 - width);
                for k in 0..width {
                    rt.slots[slot_of(first + k)] += count / width as f64;
                }
                rt.pending += count;
            };
            // Holders that died drop their connections; the clients reconnect.
            for (h, &node) in plan.holders.iter().enumerate() {
                if self.nodes[node as usize].down && rt.conns[h] > 0.0 {
                    let n = std::mem::take(&mut rt.conns[h]);
                    rt.backlog[h] = 0.0;
                    spread(&mut rt, n, tick, client.reconnect_jitter_s);
                }
            }
            let mult = self.traffic_mult(ti, now);
            let ramp = if def.ramp_s > 0.0 {
                (now as f64 / 1e6 / def.ramp_s).min(1.0)
            } else {
                1.0
            };
            let target = def.connected * mult * ramp;
            if tick == 0 && def.ramp_s <= 0.0 {
                // `ramp_s = 0`: the clients are connected from the start, as far as the
                // holders can hold them (spread evenly; the rest connect as usual).
                let n = plan.holders.len().max(1) as f64;
                for (h, &node) in plan.holders.iter().enumerate() {
                    let spec = &c.nodes[node as usize].spec;
                    if !self.nodes[node as usize].down {
                        rt.conns[h] = (target / n).min(spec.conn_capacity * spec.instances);
                    }
                }
            }
            let have: f64 = rt.conns.iter().sum();
            if target > have + rt.pending {
                // New clients connect.
                let new = target - have - rt.pending;
                spread(&mut rt, new, tick, dt);
            } else if have > target && have > 0.0 {
                // Clients leave.
                let keep = target / have;
                for c in &mut rt.conns {
                    *c *= keep;
                }
            }
            // Admission.
            let due = std::mem::take(&mut rt.slots[slot_of(tick)]);
            rt.pending = (rt.pending - due).max(0.0);
            let room: Vec<f64> = plan
                .holders
                .iter()
                .enumerate()
                .map(|(h, &node)| {
                    let n = &c.nodes[node as usize].spec;
                    if self.nodes[node as usize].down {
                        return 0.0;
                    }
                    let inst = n.instances;
                    let cap = (n.conn_capacity * inst - rt.conns[h]).max(0.0);
                    cap.min(n.accept_per_s * inst * dt)
                })
                .collect();
            let allow: f64 = room.iter().sum();
            let admitted = due.min(allow);
            if allow > 0.0 {
                for (h, r) in room.iter().enumerate() {
                    rt.conns[h] += admitted * r / allow;
                }
            }
            let refused = due - admitted;
            if refused > 0.0 {
                self.conn_drops += refused;
                let back = tick + (client.reconnect_backoff_s / dt).round().max(1.0) as usize;
                spread(&mut rt, refused, back, client.reconnect_jitter_s.max(dt));
            }
            if due > 0.0 {
                let m = &mut self.types[ti];
                m.requests += due;
                m.ok += admitted;
                m.errors += refused;
                if now <= self.c.duration_us {
                    let s = self.second_mut();
                    s.requests += due;
                    s.ok += admitted;
                    s.errors += refused;
                }
            }
            for (h, &node) in plan.holders.iter().enumerate() {
                if now < self.c.duration_us {
                    self.nodes[node as usize].bill_conn_s += rt.conns[h] * dt;
                }
            }
            // Pushes.
            for (h, &node) in plan.holders.iter().enumerate() {
                let n = &c.nodes[node as usize].spec;
                let rate = n.msgs_per_s * n.instances;
                let inflow = rt.conns[h] * def.push_rps * mult * dt;
                if inflow <= 0.0 || !rate.is_finite() && rt.backlog[h] == 0.0 {
                    if inflow > 0.0 {
                        self.types[ti].measured.add(1_000.0, inflow);
                    }
                    continue;
                }
                rt.backlog[h] = (rt.backlog[h] + inflow - rate * dt).max(0.0);
                self.nodes[node as usize].bill_msgs += inflow;
                let wait_us = if rate > 0.0 {
                    rt.backlog[h] / rate * 1e6
                } else {
                    1e9
                };
                let lat = wait_us + 1_000.0;
                self.types[ti].measured.add(lat, inflow);
                self.types[ti].response.add(lat, inflow);
                if now <= self.c.duration_us {
                    self.second_mut().measured.add(lat, inflow);
                }
            }
            // The connected share, judged from ramp + 10 s.
            let judged_from = def.ramp_s + 10.0;
            if now as f64 / 1e6 >= judged_from && now < self.c.duration_us && target > 0.0 {
                let share = rt.conns.iter().sum::<f64>() / target;
                if let Some(m) = rt.min_share.get_mut(sec) {
                    *m = m.min(share);
                }
            }
            self.conns[ti] = Some(rt);
        }
    }

    /// S6: a scan. Data scanned = the query's `gb`, pruned by a lake table's partitioning
    /// (day: what its time range needs, at least a day; hour: just its range); a table
    /// without compaction holds small files. Time = overhead + scanned ÷ scan rate + 20 ms per
    /// file over 8 parallel readers. A warehouse's result cache answers a repeat in a tenth of
    /// the overhead. A batch task reading a table splits the scan over the job's tasks.
    fn scan_us(&mut self, r: u32, node: u32, scan: ka_core::scenario::Scan) -> u64 {
        let n = &self.c.nodes[node as usize];
        let ty = self.reqs[r as usize].ty as usize;
        let table = if n.kind == ComponentKind::Lakehouse {
            Some(node)
        } else {
            self.c.table_of[node as usize]
        };
        let mut gb = scan.gb;
        if let Some(job) = &self.c.types[ty].def.job {
            gb /= f64::from(job.tasks.max(1));
        }
        let (scanned, file_mb) = match table {
            Some(t) => {
                let ts = &self.c.nodes[t as usize].spec;
                let span = scan.span_days.max(1e-6);
                let prune = match ts.partitioning {
                    1 => scan.range_share.max(1.0 / span),
                    2 => scan.range_share.max(1.0 / (span * 24.0)),
                    _ => 1.0,
                };
                let file = if ts.compaction {
                    ts.target_file_mb
                } else {
                    ts.target_file_mb.min(scan.small_file_mb)
                };
                (gb * prune.min(1.0), file)
            }
            None => (gb, 1024.0),
        };
        let spec = &n.spec;
        let overhead_s = spec.overhead_ms / 1000.0;
        if n.kind == ComponentKind::Warehouse
            && spec.result_cache
            && self.rng_s8.uniform() < scan.repeat_share
        {
            return ((overhead_s / 10.0) * 1e6) as u64 + 1;
        }
        let files = scanned * 1024.0 / file_mb.max(1e-3);
        let secs = overhead_s + scanned / spec.scan_gbps + files * SCAN_FILE_S / 8.0;
        let w = self.c.weight;
        self.nodes[node as usize].bill_tb += scanned / 1024.0 * w;
        (secs * 1e6 * self.nodes[node as usize].stretch) as u64
            + self.nodes[node as usize].add_us
            + 1
    }

    /// S9: a store, cache or broker holds this request's data: outside a Region tagged with
    /// the request's residency (or in no tagged Region at all), that's a violation.
    fn check_residency(&mut self, r: u32, node: u32) {
        let ty = self.reqs[r as usize].ty as usize;
        let Some(want) = self.c.types[ty].def.residency.as_deref() else {
            return;
        };
        let tag = self.c.nodes[node as usize]
            .region
            .and_then(|z| self.c.zones[z as usize].tag.as_deref());
        if tag != Some(want) {
            self.residency_violations += self.c.weight;
        }
    }

    /// S5: a stream processor counts records toward its next checkpoint.
    fn count_for_checkpoint(&mut self, node: u32) {
        let every = self.c.nodes[node as usize].spec.checkpoint_us.max(1);
        let idx = self.now / every;
        let w = self.c.weight;
        let rt = &mut self.nodes[node as usize];
        if rt.ckpt_idx != idx {
            rt.ckpt_idx = idx;
            rt.since_ckpt = 0.0;
        }
        rt.since_ckpt += w;
    }

    /// S5: a windowed record. Its event time (arrival minus the type's event delay) picks its
    /// window; the result leaves when the window closes (end + watermark). Until then the
    /// record holds no slot and no stream partition. Past the allowed lateness it is late:
    /// counted, left out. Returns true when the record waits or is done.
    fn window_wait(&mut self, r: u32, node: u32) -> bool {
        let spec = &self.c.nodes[node as usize].spec;
        let (kind, win, slide, mark, late_ok) = (
            spec.window,
            spec.window_us,
            spec.slide_us,
            spec.watermark_us,
            spec.lateness_us,
        );
        let q = &self.reqs[r as usize];
        let delay = self.c.types[q.ty as usize].def.event_delay;
        let start = q.start;
        let delay_us = delay.map_or(0.0, |d| {
            let ratio = (d.p99_ms / d.median_ms.max(1e-9)).max(1.0);
            self.rng_s5.lognormal(d.median_ms.max(0.0), ratio) * 1000.0
        }) as u64;
        let event = start.saturating_sub(delay_us);
        let step = if kind == 2 { slide } else { win };
        let emit = (event / step + 1) * step + mark;
        let w = self.c.weight;
        self.windowed += w;
        self.release_caller(r);
        self.message_done(r);
        let q = &mut self.reqs[r as usize];
        q.caller = node;
        q.step = 0;
        if self.now <= emit {
            self.push_req(emit, Ev::WindowEmit { r }, r);
            return true;
        }
        if self.now <= emit + late_ok {
            // An update to a closed window, within the allowed lateness.
            self.run_calls(r);
            return true;
        }
        self.late += w;
        self.finish(r);
        true
    }

    /// S4: a job run starts. Task sizes follow a Zipf skew (shuffled by the `jobs` stream)
    /// and sum to the job's work; a batch card with `salting` splits the largest 1% of
    /// tasks that many ways.
    fn job_start(&mut self, j: u16) {
        let ty = self.jobs[j as usize].ty as usize;
        let spec = self.c.types[ty].def.job.clone().expect("a job type");
        let runner = self.c.types[ty].job.as_ref().expect("job runners")[0];
        let salting = self.c.nodes[runner as usize].spec.salting;
        let n = spec.tasks.max(1) as usize;
        let mut weights: Vec<f64> = (0..n)
            .map(|i| detmath::powf((i + 1) as f64, -spec.skew.max(0.0)))
            .collect();
        for i in (1..n).rev() {
            let k = self.rng_jobs.below(i as u64 + 1) as usize;
            weights.swap(i, k);
        }
        let total: f64 = weights.iter().sum();
        let mut sizes: Vec<f64> = weights
            .iter()
            .map(|w| w / total * spec.work_units)
            .collect();
        if salting > 1 {
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|&a, &b| sizes[b].total_cmp(&sizes[a]).then(a.cmp(&b)));
            let big = n.div_ceil(100);
            let mut extra = Vec::new();
            for &i in &order[..big] {
                let part = sizes[i] / f64::from(salting);
                sizes[i] = part;
                extra.extend(std::iter::repeat_n(part, salting as usize - 1));
            }
            sizes.extend(extra);
        }
        let job = &mut self.jobs[j as usize];
        job.left = sizes.len() as u32;
        job.done = vec![false; sizes.len()];
        job.sizes = sizes;
        for t in 0..self.jobs[j as usize].sizes.len() as u32 {
            let work = self.jobs[j as usize].sizes[t as usize];
            self.launch_task(j, t, work);
        }
    }

    /// S4: one copy of task `t` with `work` executor-seconds goes to the next batch card;
    /// its time gets per-task noise (lognormal, median 1, p99 1.5) and the launch overhead.
    fn launch_task(&mut self, j: u16, t: u32, work: f64) {
        let ty = self.jobs[j as usize].ty;
        let runners = self.c.types[ty as usize].job.clone().expect("job runners");
        let rr = self.jobs[j as usize].rr;
        self.jobs[j as usize].rr = rr.wrapping_add(1);
        let node = runners[rr % runners.len()];
        let noise = self.rng_jobs.lognormal(1.0, 1.5);
        let overhead = self.c.nodes[node as usize].spec.task_overhead_ms / 1000.0;
        let r = self.alloc(ty, u64::from(t) + 1);
        {
            let q = &mut self.reqs[r as usize];
            q.job = j;
            q.task = t;
            q.task_us = ((work * noise + overhead) * 1e6) as u64;
            q.resp_sent = true;
            q.phase = Phase::Entry;
            q.runner = node;
        }
        let w = self.c.weight;
        self.types[ty as usize].requests += w;
        self.jobs[j as usize].running.entry(t).or_default().push(r);
        self.push_req(self.now, Ev::Arrive { r, node }, r);
    }

    /// S4: a task copy finished. The first copy of a task wins; the others stop. Near the
    /// end (95% done) a batch card with `speculation` copies tasks running longer than
    /// 1.5 × the median task: that helps a slow executor, not a task that is just big.
    fn job_task_done(&mut self, r: u32) {
        let (j, t, started) = {
            let q = &self.reqs[r as usize];
            (q.job, q.task, q.started)
        };
        let now = self.now;
        let job = &mut self.jobs[j as usize];
        if job.done[t as usize] {
            return;
        }
        job.done[t as usize] = true;
        job.left -= 1;
        job.finished.push(now.saturating_sub(started) as f64);
        for other in job.running.remove(&t).unwrap_or_default() {
            if other != r {
                self.reqs[other as usize].dead = true;
            }
        }
        let job = &self.jobs[j as usize];
        if job.left == 0 {
            self.jobs[j as usize].completed_at = Some(now);
            return;
        }
        let ty = job.ty as usize;
        let runner = self.c.types[ty].job.as_ref().expect("job runners")[0];
        let total = job.done.len() as f64;
        if !self.c.nodes[runner as usize].spec.speculation
            || (total - f64::from(job.left)) / total < 0.95
        {
            return;
        }
        let mut times = job.finished.clone();
        times.sort_by(f64::total_cmp);
        let median = times[times.len() / 2];
        let slow: Vec<u32> = job
            .running
            .iter()
            .filter(|(_, copies)| copies.len() == 1)
            .filter(|(_, copies)| {
                let q = &self.reqs[copies[0] as usize];
                q.started != u64::MAX && now.saturating_sub(q.started) as f64 > 1.5 * median
            })
            .map(|(&t, _)| t)
            .collect();
        for t in slow {
            let work = self.jobs[j as usize].sizes[t as usize];
            self.launch_task(j, t, work);
        }
    }

    /// S4: a task copy failed (a call failed): it starts again unless another copy runs.
    fn job_task_failed(&mut self, r: u32) {
        let (j, t) = (self.reqs[r as usize].job, self.reqs[r as usize].task);
        {
            let q = &mut self.reqs[r as usize];
            q.dead = true;
        }
        self.release_caller(r);
        self.finish(r);
        let job = &mut self.jobs[j as usize];
        if let Some(copies) = job.running.get_mut(&t) {
            copies.retain(|&x| x != r);
        }
        if job.done[t as usize] || job.running.get(&t).is_some_and(|c| !c.is_empty()) {
            return;
        }
        self.tasks_retried += 1.0;
        let work = self.jobs[j as usize].sizes[t as usize];
        self.launch_task(j, t, work);
    }

    /// S4 spot chaos: `share` of each selected batch card's executors go away; that share of
    /// the tasks running on it stop and start again, from their last checkpoint (each 10%
    /// of the task) when the card has `checkpoint`.
    fn spot_start(&mut self, chaos: u16, nodes: &[u32], share: f64) {
        let share = share.clamp(0.0, 1.0);
        for &node in nodes {
            let servers = self.nodes[node as usize].lane_servers;
            let take = (f64::from(servers) * share).round() as u32;
            self.nodes[node as usize].lane_servers = servers - take.min(servers);
            self.spot_taken.push((chaos, node, take.min(servers)));
            let checkpoint = self.c.nodes[node as usize].spec.checkpoint;
            // The tasks in service on this card, in job then task order.
            let mut running: Vec<(u16, u32, u32)> = Vec::new();
            for (j, job) in self.jobs.iter().enumerate() {
                for (&t, copies) in &job.running {
                    for &r in copies {
                        let q = &self.reqs[r as usize];
                        if !q.dead
                            && q.phase == Phase::Entry
                            && q.started != u64::MAX
                            && q.runner == node
                        {
                            running.push((j as u16, t, r));
                        }
                    }
                }
            }
            let kill = (running.len() as f64 * share).round() as usize;
            for &(j, t, r) in running.iter().take(kill) {
                let q = &self.reqs[r as usize];
                let progress = (self.now.saturating_sub(q.started) as f64
                    / q.task_us.max(1) as f64)
                    .clamp(0.0, 1.0);
                let size = self.jobs[j as usize].sizes[t as usize];
                let left = if checkpoint {
                    size * (1.0 - (progress * 10.0).floor() / 10.0)
                } else {
                    size
                };
                let q = &mut self.reqs[r as usize];
                q.dead = true;
                q.released = true;
                self.set_busy(node, 0, -1);
                if let Some(copies) = self.jobs[j as usize].running.get_mut(&t) {
                    copies.retain(|&x| x != r);
                }
                self.tasks_retried += 1.0;
                self.launch_task(j, t, left);
            }
        }
    }

    fn spot_end(&mut self, chaos: u16) {
        let taken: Vec<(u16, u32, u32)> = self
            .spot_taken
            .iter()
            .copied()
            .filter(|x| x.0 == chaos)
            .collect();
        self.spot_taken.retain(|x| x.0 != chaos);
        for (_, node, k) in taken {
            self.nodes[node as usize].lane_servers += k;
            // The executors are back: start waiting tasks.
            while self.nodes[node as usize].lanes[0].busy < self.nodes[node as usize].lane_servers {
                let Some(next) = self.nodes[node as usize].lanes[0].queue.pop_front() else {
                    break;
                };
                self.reqs[next as usize].pending -= 1;
                if self.reqs[next as usize].dead {
                    self.maybe_free(next);
                    continue;
                }
                self.set_busy(node, 0, 1);
                self.start_service(next, node, 0);
            }
        }
    }

    /// S1: the least connected share over the judged seconds of every connection type.
    fn connected_min(&self) -> f64 {
        self.conns
            .iter()
            .flatten()
            .flat_map(|rt| rt.min_share.iter().copied())
            .filter(|v| v.is_finite())
            .fold(f64::INFINITY, f64::min)
            .clamp(0.0, 1.0)
            * f64::from(u8::from(self.conns.iter().any(Option::is_some)))
    }

    /// S8 rollouts. A deploy reaches `node` in stages of its `rollout_percent`, one every
    /// `bake_s`; with `auto_rollback`, a stage is noticed one health interval after it starts
    /// and rolled back. A config push (node = NONE) reaches every service at once, or, with a
    /// staged Config card, one Region per stage, halting at the first stage noticed; with a
    /// kill switch it is rolled back a second after that.
    fn rollout_stage(&mut self, chaos: u16, node: u32, stage: u16) {
        if self.rollout_done.contains(&(chaos, node)) {
            return;
        }
        let k = f64::from(stage);
        if node != NONE {
            let spec = &self.c.nodes[node as usize].spec;
            let (share, bake, auto) = (spec.rollout_share, spec.bake_us, spec.auto_rollback);
            let reach = if bake == 0 { 1.0 } else { (k * share).min(1.0) };
            self.nodes[node as usize].faulty = reach;
            if auto {
                self.push(self.now + HEALTH_EVERY_US, Ev::Noticed { chaos, node });
            }
            if reach < 1.0 {
                self.push(
                    self.now + bake,
                    Ev::Stage {
                        chaos,
                        node,
                        stage: stage + 1,
                    },
                );
            }
            return;
        }
        // A config push.
        let config =
            (0..self.c.nodes.len()).find(|&n| self.c.nodes[n].kind == ComponentKind::Config);
        let targets: Vec<u32> = self.chaos_targets[chaos as usize].clone();
        let staged = config.map(|c| &self.c.nodes[c].spec).filter(|s| s.staged);
        let Some(spec) = staged else {
            for &n in &targets {
                self.nodes[n as usize].faulty = 1.0;
            }
            if config.is_some_and(|c| self.c.nodes[c].spec.kill_switch) {
                self.push(self.now + HEALTH_EVERY_US, Ev::Noticed { chaos, node });
            }
            return;
        };
        // Stages: the services' Regions in zone order; services outside any Region last.
        let mut regions: Vec<Option<u32>> = targets
            .iter()
            .map(|&n| self.c.nodes[n as usize].region)
            .collect();
        regions.sort_by_key(|r| r.unwrap_or(u32::MAX));
        regions.dedup();
        let Some(&region) = regions.get(usize::from(stage) - 1) else {
            return;
        };
        for &n in &targets {
            if self.c.nodes[n as usize].region == region {
                self.nodes[n as usize].faulty = 1.0;
            }
        }
        let bake = spec.bake_us;
        self.push(self.now + HEALTH_EVERY_US, Ev::Noticed { chaos, node });
        if usize::from(stage) < regions.len() {
            self.push(
                self.now + bake.max(HEALTH_EVERY_US + 1),
                Ev::Stage {
                    chaos,
                    node,
                    stage: stage + 1,
                },
            );
        }
    }

    fn rollout_noticed(&mut self, chaos: u16, node: u32) {
        if self.rollout_done.contains(&(chaos, node)) {
            return;
        }
        if node != NONE {
            // Auto-rollback.
            self.nodes[node as usize].faulty = 0.0;
            self.rollout_done.insert((chaos, node));
            return;
        }
        // A config push: halt the stages; with a kill switch, roll back a second later.
        self.rollout_done.insert((chaos, NONE));
        let config =
            (0..self.c.nodes.len()).find(|&n| self.c.nodes[n].kind == ComponentKind::Config);
        if config.is_some_and(|c| self.c.nodes[c].spec.kill_switch) {
            self.push(self.now + 1_000_000, Ev::Rollback { chaos });
        }
    }

    /// A card stops answering: queued work fails now, work in service fails when it ends,
    /// callers waiting on it fail when their call comes back. A cluster primary fails over
    /// after its `failover_s`.
    fn kill(&mut self, n: u32) {
        if self.nodes[n as usize].down {
            return;
        }
        if self.c.nodes[n as usize].kind == ComponentKind::Streamproc {
            // S5: restarting from the last checkpoint replays what it processed since; the
            // sinks receive those results again unless they write idempotently.
            let replay = std::mem::take(&mut self.nodes[n as usize].since_ckpt);
            if !self.c.dedup_sinks[n as usize] {
                self.duplicates += replay;
            }
        }
        self.nodes[n as usize].down = true;
        let queued: Vec<u32> = self.nodes[n as usize]
            .lanes
            .iter_mut()
            .flat_map(|l| std::mem::take(&mut l.queue))
            .collect();
        for r in queued {
            self.reqs[r as usize].pending -= 1;
            if self.reqs[r as usize].dead {
                self.maybe_free(r);
            } else {
                self.fail(r);
            }
        }
        if let Some(cl) = self.c.cluster_of[n as usize]
            && self.cluster_primary[cl as usize] == n
            && self.c.clusters[cl as usize].members.len() > 1
        {
            let after = (self.c.nodes[n as usize].spec.failover_s.max(0.0) * 1e6) as u64;
            self.push(self.now + after, Ev::Failover(cl));
        }
    }

    /// A card comes back. A cache comes back empty.
    fn revive(&mut self, n: u32) {
        if !self.nodes[n as usize].down {
            return;
        }
        self.nodes[n as usize].down = false;
        if self.caches[n as usize].is_some() {
            self.flush(n);
        }
        for b in self.c.feeds[n as usize].clone() {
            self.try_dispatch(b);
        }
        if matches!(
            self.c.nodes[n as usize].kind,
            ComponentKind::Queue
                | ComponentKind::Stream
                | ComponentKind::Pubsub
                | ComponentKind::Dlq
        ) {
            self.try_dispatch(n);
        }
        if self.c.nodes[n as usize].kind == ComponentKind::Stream {
            for src in 0..self.cdc_backlog.len() as u32 {
                if !self.cdc_backlog[src as usize].is_empty() {
                    self.cdc_flush(src);
                }
            }
        }
    }

    fn flush(&mut self, n: u32) {
        let now = self.now;
        if let Some(cache) = self.caches[n as usize].as_mut() {
            for f in &mut cache.fills {
                f.clear();
            }
            cache.flushed_at = Some(now);
            cache.seen = 0.0;
        }
    }

    /// Health checks: a card whose state differs from what routing believes for two checks
    /// in a row is believed (dead cards leave rotation 2-4 s after dying).
    fn health_check(&mut self) {
        for s in &mut self.splits {
            if !s.known {
                s.checks += 1;
                s.known = s.checks >= HEALTH_CHECKS;
            }
        }
        self.eject_overloaded();
        for rt in &mut self.nodes {
            if rt.down != rt.known_down {
                rt.checks += 1;
                if rt.checks >= HEALTH_CHECKS {
                    rt.known_down = rt.down;
                    rt.checks = 0;
                }
            } else {
                rt.checks = 0;
            }
        }
    }

    /// S8: load balancers with `health_timeout_ms` eject a target whose queue wait passes it
    /// on two checks in a row, and take it back after two checks under it; at least
    /// `min_healthy_pct` of its targets always stay in rotation.
    fn eject_overloaded(&mut self) {
        for lb in 0..self.c.nodes.len() {
            let spec = &self.c.nodes[lb].spec;
            if spec.health_timeout_us == 0 {
                continue;
            }
            let (limit_ms, floor) = (spec.health_timeout_us as f64 / 1000.0, spec.min_healthy);
            let targets: Vec<u32> = self.c.nodes[lb]
                .out
                .iter()
                .filter(|(_, p)| matches!(p, Protocol::Http | Protocol::Grpc | Protocol::Ws))
                .map(|(t, _)| *t)
                .collect();
            let n = targets.len();
            let keep = (floor * n as f64).ceil() as usize;
            for &t in &targets {
                let cn = &self.c.nodes[t as usize];
                let rt = &self.nodes[t as usize];
                let per_lane = f64::from(rt.lane_servers.max(1));
                let wait_ms = rt
                    .lanes
                    .iter()
                    .map(|l| l.queue.len() as f64 / per_lane * cn.spec.service_ms * rt.stretch)
                    .fold(0.0, f64::max);
                let over = wait_ms > limit_ms;
                let healthy = targets
                    .iter()
                    .filter(|&&x| !self.nodes[x as usize].ejected)
                    .count();
                let rt = &mut self.nodes[t as usize];
                if over != rt.ejected {
                    rt.eject_checks += 1;
                    if rt.eject_checks >= HEALTH_CHECKS {
                        rt.eject_checks = 0;
                        if over && healthy <= keep {
                            continue;
                        }
                        rt.ejected = over;
                    }
                } else {
                    rt.eject_checks = 0;
                }
            }
        }
    }

    /// Promote the next healthy member (id order after the old primary) if the primary is
    /// still down.
    fn failover(&mut self, cl: u32) {
        let primary = self.cluster_primary[cl as usize];
        if !self.nodes[primary as usize].down {
            return;
        }
        let members = &self.c.clusters[cl as usize].members;
        let at = members.iter().position(|&m| m == primary).unwrap_or(0);
        let next = (1..members.len())
            .map(|k| members[(at + k) % members.len()])
            .find(|&m| !self.nodes[m as usize].down);
        if let Some(m) = next {
            self.cluster_primary[cl as usize] = m;
        }
    }

    /// A card's monthly cost, with its billed units spread over `secs` of traffic and
    /// extrapolated to a month (ADR 0006).
    fn node_cost(&self, i: usize, secs: f64) -> f64 {
        let (n, rt) = (&self.c.nodes[i], &self.nodes[i]);
        let c = &n.cost;
        let per_month = MONTH_S / secs.max(1e-6) / 1e6 * n.spec.bill_ratio;
        c.per_instance_month
            * self.mean_instances(i).max(1.0)
            * n.spec.cost_lanes
            * self.active_share(i)
            + c.per_lane_month * n.spec.bill_lanes
            + c.per_gb_month * n.spec.memory_gb
            + (c.per_million * rt.bill_r + c.per_million_write.unwrap_or(c.per_million) * rt.bill_w)
                * per_month
            + self.meter_cost(i) * per_month
    }

    /// S6: the share of the run an `active_only` card (an auto-suspending warehouse) is billed:
    /// seconds it was busy, each extended by its `auto_suspend_s`. 1 for every other card.
    fn active_share(&self, i: usize) -> f64 {
        let n = &self.c.nodes[i];
        if !n.cost.active_only {
            return 1.0;
        }
        let series = &self.nodes[i].util_series;
        if series.is_empty() {
            return 1.0;
        }
        let tail = n.spec.auto_suspend_s.ceil() as usize;
        let mut until = 0usize;
        let mut billed = 0usize;
        for (s, u) in series.iter().enumerate() {
            if *u > 0.0 {
                until = s + 1 + tail;
            }
            if s < until {
                billed += 1;
            }
        }
        billed as f64 / series.len() as f64
    }

    /// M8 usage meters, in millions of units over the run (per_month turns them monthly):
    /// tokens (half price on the batch API), connection-minutes and messages.
    fn meter_cost(&self, i: usize) -> f64 {
        let (n, rt) = (&self.c.nodes[i], &self.nodes[i]);
        let m = &n.cost.meters;
        let batch = if n.spec.batch_api { 0.5 } else { 1.0 };
        (m.tokens_in_m * rt.bill_tin + m.tokens_out_m * rt.bill_tout) * batch
            + m.conn_minutes_m * rt.bill_conn_s / 60.0
            + m.messages_m * rt.bill_msgs
            // A price per TB, over per_month's per-million scale.
            + m.tb_scanned * rt.bill_tb * 1e6
    }

    /// Billing units one request of type `ty` counts at card `node`.
    fn bill_units(&self, node: u32, ty: usize) -> f64 {
        let unit = self.c.nodes[node as usize].cost.unit_kb;
        if unit > 0.0 {
            (self.c.types[ty].def.payload_kb / unit).max(1.0)
        } else {
            1.0
        }
    }

    fn mean_instances(&self, i: usize) -> f64 {
        let rt = &self.nodes[i];
        if !self
            .c
            .scaling
            .iter()
            .any(|g| g.members.contains(&(i as u32)))
            || self.now == 0
        {
            return self.c.nodes[i].spec.instances;
        }
        (rt.inst_integral + rt.instances * self.now.saturating_sub(rt.inst_since) as f64)
            / self.now as f64
    }

    /// Every 5 s: mean utilisation of each group's cards since the last tick. Above target,
    /// scale out to bring it back to target; below half the target, scale in by one. Each
    /// decision takes effect after the policy delay.
    fn scale_tick(&mut self) {
        let now = self.now;
        for g in 0..self.c.scaling.len() {
            let group = self.c.scaling[g].clone();
            let (mut busy, mut cap) = (0.0, 0.0);
            for &m in &group.members {
                let rt = &mut self.nodes[m as usize];
                let mut integral = 0.0;
                for l in &rt.lanes {
                    integral += l.busy_integral
                        + f64::from(l.busy) * now.saturating_sub(l.busy_since) as f64;
                }
                busy += integral - rt.scale_busy;
                rt.scale_busy = integral;
                cap += f64::from(rt.lane_servers)
                    * f64::from(self.c.nodes[m as usize].spec.lanes)
                    * SCALE_EVERY_US as f64;
            }
            if self.scale_pending[g] || cap <= 0.0 || self.control_plane > 0 {
                // S8: no scaling decisions while the control plane is down.
                continue;
            }
            let util = busy / cap;
            let inst = self.nodes[group.members[0] as usize].instances;
            let want = if util > group.target {
                (inst * util / group.target).ceil().min(group.max)
            } else if util < group.target / 2.0 {
                (inst - 1.0).max(group.min)
            } else {
                inst
            };
            if want != inst {
                self.scale_pending[g] = true;
                self.push(
                    now + group.delay_us,
                    Ev::ScaleApply {
                        group: g as u32,
                        instances: want as u32,
                    },
                );
            }
        }
    }

    fn scale_apply(&mut self, g: u32, instances: f64) {
        self.scale_pending[g as usize] = false;
        if self.control_plane > 0 {
            // S8: the decision can't be carried out (no new instances start).
            return;
        }
        let now = self.now;
        let sample = self.c.sample;
        for m in self.c.scaling[g as usize].members.clone() {
            let n = &self.c.nodes[m as usize];
            let per_instance = n.spec.servers / n.spec.instances / f64::from(n.spec.lanes);
            let scaled = per_instance * instances * sample;
            let eff = scaled.round().max(1.0);
            let rt = &mut self.nodes[m as usize];
            rt.inst_integral += rt.instances * now.saturating_sub(rt.inst_since) as f64;
            rt.inst_since = now;
            rt.instances = instances;
            rt.lane_servers = eff as u32;
            rt.stretch = eff / scaled;
            // More servers: start queued work on them.
            for lane in 0..rt.lanes.len() as u16 {
                self.fill_free_servers(m, lane);
            }
        }
    }

    fn fill_free_servers(&mut self, node: u32, lane: u16) {
        let cap = self.nodes[node as usize].lane_servers;
        while self.nodes[node as usize].lanes[lane as usize].busy < cap {
            let Some(next) = self.nodes[node as usize].lanes[lane as usize]
                .queue
                .pop_front()
            else {
                break;
            };
            self.reqs[next as usize].pending -= 1;
            if self.reqs[next as usize].dead {
                self.maybe_free(next);
                continue;
            }
            self.set_busy(node, lane, 1);
            self.start_service(next, node, lane);
        }
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A 64-bit mixer (splitmix64's finaliser) for stable hashing of keys.
pub fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// S4: a job run as reported.
fn ka_sim_job_result(ty: &str, j: &JobRt) -> crate::result::JobResult {
    crate::result::JobResult {
        ty: ty.to_owned(),
        start_s: j.start as f64 / 1e6,
        completion_s: j.completed_at.map(|t| (t - j.start) as f64 / 1e6),
    }
}

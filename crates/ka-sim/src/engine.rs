//! The discrete-event engine (ADR 0006). One thread, integer µs time, a total event order
//! by (time, sequence), and every random draw from a named seeded stream: the same input
//! and seed always give the same run, bit for bit. `Engine: Clone` is the snapshot used by
//! live-run scrubbing.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
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
const DRAIN_US: u64 = 30_000_000;
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
    CallBack {
        r: u32,
    },
    CallFail {
        r: u32,
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
}

#[derive(Debug, Clone, Default)]
struct CacheRt {
    /// Per request type: the last fill time per tracked key rank (i64::MIN: never).
    fills: Vec<Vec<i64>>,
    /// When the cache was last emptied (a flush, or coming back after a kill).
    flushed_at: Option<u64>,
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
}

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
            caches: c
                .nodes
                .iter()
                .map(|n| {
                    matches!(n.kind, ComponentKind::Cache | ComponentKind::Cdn).then(|| CacheRt {
                        fills: vec![Vec::new(); c.types.len()],
                        flushed_at: None,
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
        if !e.c.scenario.chaos.is_empty() || !e.c.scaling.is_empty() {
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
        let elapsed_s = (now as f64 / 1e6).max(1e-6);
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
                cost_month +=
                    n.cost.per_instance_month * self.mean_instances(i).max(1.0) * n.spec.cost_lanes
                        + n.cost.per_gb_month * n.spec.memory_gb
                        + n.cost.per_million * (rt.served / elapsed_s) * MONTH_S / 1e6;
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
            Ev::CallBack { r } => {
                self.reqs[r as usize].pending -= 1;
                self.on_callback(r);
            }
            Ev::CallFail { r } => {
                self.reqs[r as usize].pending -= 1;
                self.fail(r);
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
        let r = self.alloc(src.ty, key);
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
            self.push_req(self.now + TIMEOUT_US, Ev::Timeout { r }, r);
            self.hop_from(r, src.node);
        }
    }

    // --- Hops -------------------------------------------------------------------------

    fn pick(&mut self, from: u32, cands: &[u32], key: u64) -> Option<u32> {
        let live: Vec<u32> = cands
            .iter()
            .copied()
            .filter(|c| !self.nodes[*c as usize].known_down && !self.known_cut(from, *c))
            .collect();
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
        let Some(next) = self.pick(from, &cands, key) else {
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
            if self.reqs[r as usize].phase == Phase::Consumer {
                // A message handed to a consumer that died (or was cut off) on the way.
                self.lease_lost(r);
            } else {
                self.fail(r);
            }
            return;
        }
        let lane = self.lane_of(node, r);
        if reserved {
            self.start_service(r, node, lane);
            return;
        }
        let cap = self.nodes[node as usize].lane_servers;
        if self.nodes[node as usize].lanes[lane as usize].busy < cap {
            self.set_busy(node, lane, 1);
            self.start_service(r, node, lane);
        } else if self.nodes[node as usize].lanes[lane as usize].queue.len()
            < self.c.nodes[node as usize].lane_queue
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
            self.fail(r);
        }
    }

    fn service_us(&mut self, r: u32, node: u32) -> u64 {
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

    fn start_service(&mut self, r: u32, node: u32, lane: u16) {
        let us = self.service_us(r, node);
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
        let phase = self.reqs[r as usize].phase;
        let holds = self.c.nodes[node as usize].spec.holds
            && matches!(phase, Phase::Entry | Phase::Consumer);
        if self.reqs[r as usize].dead {
            self.release(node, lane);
            self.maybe_free(r);
            return;
        }
        if self.nodes[node as usize].down {
            // The card died while working on it. A consumer's message comes back when its
            // lease runs out; anything else fails now.
            self.release(node, lane);
            if phase == Phase::Consumer {
                self.lease_lost(r);
            } else {
                self.fail(r);
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
                    self.call_external(r, x);
                    return;
                }
                Target::Node(n) => n,
                Target::Cluster(cl) => self.cluster_member(cl, writes, caller),
            };
            let q = &mut self.reqs[r as usize];
            q.phase = Phase::Call;
            q.call_write = writes;
            q.call_aside = aside;
            let proto = match self.c.nodes[node as usize].spec.role {
                Role::Cache | Role::Datastore => Protocol::Query,
                _ => edge_protocol(&self.c, caller, node),
            };
            self.send(r, caller, node, proto, false);
            return;
        }
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
        let live: Vec<u32> = members
            .iter()
            .copied()
            .filter(|m| !self.nodes[*m as usize].known_down && !self.known_cut(caller, *m))
            .collect();
        if live.is_empty() {
            return primary;
        }
        self.cluster_rr[cl as usize] = self.cluster_rr[cl as usize].wrapping_add(1);
        live[self.cluster_rr[cl as usize] % live.len()]
    }

    fn after_call(&mut self, r: u32, node: u32) {
        let (ty, key) = (self.reqs[r as usize].ty as usize, self.reqs[r as usize].key);
        let q = &self.reqs[r as usize];
        let (write, aside, fill_cache) = (q.call_write, q.call_aside, q.fill_cache);
        let w = self.c.weight;
        match self.c.nodes[node as usize].spec.role {
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
                if self.cache_hit(node, ty, key) {
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
            self.push_req(self.now + EXTERNAL_DOWN_US, Ev::CallFail { r }, r);
            return;
        }
        let lat = hop_latency_us(&self.c, node, caller, Protocol::Query);
        self.push_req(self.now + lat, Ev::CallBack { r }, r);
    }

    fn on_callback(&mut self, r: u32) {
        if self.reqs[r as usize].dead {
            self.maybe_free(r);
            return;
        }
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
            self.push_req(now + EXTERNAL_DOWN_US, Ev::CallFail { r }, r);
            return;
        }
        if let Some(limit) = ext.rate_cut.or(def.rate_limit) {
            let rate = limit * sample;
            ext.tokens = (ext.tokens + rate * (now - ext.last) as f64 / 1e6).min(rate.max(1.0));
            ext.last = now;
            if ext.tokens < 1.0 {
                let fast = (def.median_ms * 1000.0) as u64;
                self.push_req(now + fast, Ev::CallFail { r }, r);
                return;
            }
            ext.tokens -= 1.0;
        }
        let add = ext.add_ms;
        let ms = self.rng_misc.lognormal(
            def.median_ms,
            (def.p99_ms / def.median_ms.max(1e-9)).max(1.0),
        ) + add;
        self.push_req(now + (ms * 1000.0) as u64, Ev::CallBack { r }, r);
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
            self.enqueue_broker(r, b);
        } else if self.c.dlq_of[b as usize].is_some() {
            self.dead_lettered += w;
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
            let backoff = 1_000_000u64 << (q.attempts - 1).min(5);
            self.push_req(self.now + backoff, Ev::Requeue { r }, r);
        } else if self.c.dlq_of[b as usize].is_some() {
            self.dead_lettered += w;
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
            });
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
                let per_s = rt.served / dur;
                let cost =
                    n.cost.per_instance_month * self.mean_instances(i).max(1.0) * n.spec.cost_lanes
                        + n.cost.per_gb_month * n.spec.memory_gb
                        + n.cost.per_million * per_s * MONTH_S / 1e6;
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
            stale_reads: self.stale_reads,
            stale_rate: if self.reads > 0.0 {
                self.stale_reads / self.reads
            } else {
                0.0
            },
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

    fn in_zone(&self, node: u32, zone: &str) -> bool {
        let n = &self.c.nodes[node as usize];
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
                targets = (0..self.c.nodes.len() as u32)
                    .filter(|&n| self.in_zone(n, zone))
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
                self.splits.push(SplitRt {
                    chaos: i,
                    zone: zone.clone(),
                    since: self.now,
                    checks: 0,
                    known: false,
                });
                for_s = *f;
            }
            // Traffic is read from the scenario when arrivals are generated.
            C::Traffic { .. } => {}
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
                    self.revive(n);
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

    /// A card stops answering: queued work fails now, work in service fails when it ends,
    /// callers waiting on it fail when their call comes back. A cluster primary fails over
    /// after its `failover_s`.
    fn kill(&mut self, n: u32) {
        if self.nodes[n as usize].down {
            return;
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
            if self.scale_pending[g] || cap <= 0.0 {
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

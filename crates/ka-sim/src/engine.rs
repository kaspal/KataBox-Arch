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
    Arrive { r: u32, node: u32 },
    SvcDone { r: u32, node: u32, lane: u16 },
    CallBack { r: u32 },
    CallFail { r: u32 },
    Timeout { r: u32 },
    Requeue { r: u32 },
    Second(u32),
}

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
    /// A queue/stream consumer slot reserved at dispatch.
    reserved: bool,
    units: f64,
    attempts: u32,
    broker: u32,
    group: u32,
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
    add_us: u64,
    served: f64,
    drops: f64,
    max_queue: usize,
    sec_max_queue: usize,
    sec_drops: f64,
    last_integral: f64,
    util_series: Vec<f64>,
    queue_series: Vec<f64>,
    drop_series: Vec<f64>,
    rr: usize,
    warm: u32,
}

#[derive(Debug, Clone, Default)]
struct CacheRt {
    /// Per request type: the last fill time per tracked key rank (i64::MIN: never).
    fills: Vec<Vec<i64>>,
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
        for n in &mut nodes {
            n.util_series.reserve(nsec);
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
            Ev::Requeue { r } => {
                self.reqs[r as usize].pending -= 1;
                let b = self.reqs[r as usize].broker;
                self.enqueue_broker(r, b);
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
            resp_sent: false,
            dead: false,
            done: false,
            pending: 0,
            reserved: false,
            units: 1.0,
            attempts: 0,
            broker: NONE,
            group: NONE,
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
            .filter(|c| !self.nodes[*c as usize].down)
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
        } else if n.kind == ComponentKind::Lb && n.spec.algorithm == "ip_hash" {
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
        if hop {
            self.reqs[r as usize].net += lat;
        }
        self.push_req(self.now + lat, Ev::Arrive { r, node: to }, r);
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
        if self.reqs[r as usize].phase == Phase::BrokerIn {
            self.broker_accept(r, node);
            return;
        }
        if self.nodes[node as usize].down {
            if self.reqs[r as usize].reserved {
                self.set_busy(node, 0, -1);
            }
            self.fail(r);
            return;
        }
        let lane = self.lane_of(node, r);
        if self.reqs[r as usize].reserved {
            self.start_service(r, node, lane);
            return;
        }
        let cap = self.c.nodes[node as usize].lane_servers;
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
        if q.seg >= 1 {
            median *= q.units;
        }
        let ratio = n.spec.p99_ratio;
        let stretch = n.service_stretch;
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
        let cap = self.c.nodes[node as usize].lane_servers;
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
                Target::Cluster(cl) => self.cluster_member(cl, writes),
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

    fn cluster_member(&mut self, cl: u32, write: bool) -> u32 {
        let primary = self.cluster_primary[cl as usize];
        if write {
            return primary;
        }
        let members = &self.c.clusters[cl as usize].members;
        let live: Vec<u32> = members
            .iter()
            .copied()
            .filter(|m| !self.nodes[*m as usize].down)
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
        match self.c.nodes[node as usize].spec.role {
            Role::Cache if write => self.fill(node, ty, key),
            Role::Cache if aside => {
                self.reads += self.c.weight;
                if self.cache_hit(node, ty, key) {
                    self.reqs[r as usize].hit_end = true;
                } else {
                    self.reqs[r as usize].fill_cache = node;
                }
            }
            Role::Datastore => {
                if !write {
                    self.reads += self.c.weight;
                }
                if fill_cache != NONE {
                    self.fill(fill_cache, ty, key);
                    self.reqs[r as usize].fill_cache = NONE;
                }
            }
            _ => {}
        }
        let caller = self.reqs[r as usize].caller;
        let lat = hop_latency_us(&self.c, node, caller, Protocol::Query);
        self.push_req(self.now + lat, Ev::CallBack { r }, r);
    }

    fn on_callback(&mut self, r: u32) {
        if self.reqs[r as usize].dead {
            self.maybe_free(r);
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
        if let Some(limit) = def.rate_limit {
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
        self.release_caller(r);
        let q = &self.reqs[r as usize];
        let (ty, seg, caller) = (q.ty as usize, q.seg as usize, q.caller);
        let nseg = self.c.types[ty].route.segments.len();
        if q.hit_end || seg + 1 >= nseg {
            if !self.reqs[r as usize].resp_sent {
                self.succeed_sync(r);
            } else {
                self.complete_async(r);
            }
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

    fn broker_accept(&mut self, r: u32, b: u32) {
        if self.nodes[b as usize].down {
            self.fail(r);
            return;
        }
        let depth = self.brokers[b as usize].queue.len() as f64;
        if depth >= (self.c.nodes[b as usize].spec.max_depth * self.c.sample).max(1.0) {
            let w = self.c.weight;
            self.nodes[b as usize].drops += w;
            self.nodes[b as usize].sec_drops += w;
            self.fail(r);
            return;
        }
        let w = self.c.weight;
        self.nodes[b as usize].served += w;
        let caller = self.reqs[r as usize].caller;
        if !self.reqs[r as usize].resp_sent {
            // The publish is acknowledged: the client's answer leaves now.
            let ack = hop_latency_us(&self.c, b, caller, Protocol::Publish);
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
        if self.c.nodes[b as usize].kind == ComponentKind::Pubsub {
            self.fan_out(r, b);
        } else {
            self.enqueue_broker(r, b);
        }
    }

    fn enqueue_broker(&mut self, r: u32, b: u32) {
        self.reqs[r as usize].reserved = false;
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
                if !rt.down && rt.lanes[0].busy < self.c.nodes[c as usize].lane_servers {
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
            q.phase = Phase::Consumer;
            self.send(r, b, c, Protocol::Consume, false);
        }
    }

    /// The broker's in-flight slot for a dispatched message frees (done or failed).
    fn message_done(&mut self, r: u32) {
        let q = &mut self.reqs[r as usize];
        if q.reserved && q.broker != NONE {
            q.reserved = false;
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
        let window = (ttl.min(3_600_000_000) as f64) / 1e6;
        let p = 1.0 - detmath::exp(-t.rate * pmf * window);
        self.rng_misc.uniform() < p
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
        let now = self.now;
        for (i, rt) in self.nodes.iter_mut().enumerate() {
            let mut integral = 0.0;
            for l in &mut rt.lanes {
                l.busy_integral += f64::from(l.busy) * (now - l.busy_since) as f64;
                l.busy_since = now;
                integral += l.busy_integral;
            }
            let cap = f64::from(self.c.nodes[i].lane_servers)
                * f64::from(self.c.nodes[i].spec.lanes)
                * 1e6;
            rt.util_series
                .push(((integral - rt.last_integral) / cap).clamp(0.0, 1.0));
            rt.last_integral = integral;
            rt.queue_series
                .push(rt.sec_max_queue as f64 * self.c.weight);
            rt.drop_series.push(rt.sec_drops);
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
            .map(|(n, rt)| {
                let secs = rt.util_series.len().max(1) as f64;
                let utilisation = rt.util_series.iter().sum::<f64>() / secs;
                let per_s = rt.served / dur;
                let cost = n.cost.per_instance_month * n.spec.instances.max(1.0)
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

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A 64-bit mixer (splitmix64's finaliser) for stable hashing of keys.
pub fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

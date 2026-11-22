//! Live runs (plan M1.3): a simulation played at wall-clock pace, sending binary frames to
//! the UI.
//!
//! - `LiveCore` is the deterministic part: it steps the engine, keeps a snapshot every
//!   5 simulated seconds, seeks, and encodes frames. It has no clock and no thread, so tests
//!   drive it directly.
//! - `LiveRun` owns a `LiveCore` on a worker thread and paces it to wall time × speed.
//!   Wall time is allowed here; it never enters `ka-sim` (ADR 0006).
//! - A `FrameSink` takes the frames, which decouples this from Tauri.
//!
//! **Frame format** (`KAF1`, little-endian; `lib/runFrames.ts` decodes it):
//!
//! ```text
//! header   magic "KAF1" [4] | kind u8 (1 = frame) | flags u8 (1 playing, 2 finished)
//!          | frame u32 | t_ms u32 | window_ms u16 (simulated ms since the previous frame)
//!          | speed f32 | p95_ms f32 | rps f32 | error_rate f32 | cost_month f32
//!          | node_count u16 | packet_count u16
//!          | packet_total u32 (hops and events in the window before thinning)   (44 bytes)
//! node     idx u16 | util % u8 | queue u16 (saturating) | flags u8 (1 down, 2 overloaded)
//! packet   edge idx u16 | type u8 (bits 0-2 the request type, bit 3 async, bit 4 cache
//!          hit at the edge's far end, bit 5 dropped there, bit 6 travels the edge from its
//!          target to its source; hits and drops have travel 0)
//!          | start u16 (ms after the window began) | travel u16 (tenths of a ms)
//! ```
//!
//! Hops and events are thinned separately (a deterministic stride), so drops stay visible.
//!
//! Tiles (p95, rps, error rate) describe the last completed simulated second; cost is per
//! month at the traffic seen so far. Utilisation is measured over the frame's window.

use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ka_core::Design;
use ka_sim::{Compiled, Engine, RunResult};

pub const MAGIC: &[u8; 4] = b"KAF1";
pub const HEADER_BYTES: usize = 44;
pub const NODE_BYTES: usize = 6;
pub const PACKET_BYTES: usize = 7;
/// A snapshot every 5 simulated seconds bounds the re-simulation a seek needs.
pub const SNAPSHOT_EVERY_US: u64 = 5_000_000;
/// About 30 frames per wall-clock second.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(33);
/// At most this many hops and this many hit/drop events per frame (about 3,000 hops a
/// second, ~1,500 on screen at once).
pub const MAX_HOPS_PER_FRAME: usize = 100;
pub const MAX_EVENTS_PER_FRAME: usize = 30;
pub const SPEEDS: [f64; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

const FLAG_PLAYING: u8 = 1;
const FLAG_FINISHED: u8 = 2;
const NODE_DOWN: u8 = 1;
const NODE_OVERLOADED: u8 = 2;
const PACKET_ASYNC: u8 = 8;
const PACKET_HIT: u8 = 16;
const PACKET_DROP: u8 = 32;
const PACKET_REVERSE: u8 = 64;

/// The deterministic heart of a live run.
pub struct LiveCore {
    engine: Engine,
    /// Engine states at 0, 5, 10… simulated seconds (each taken on first arrival).
    snapshots: BTreeMap<u64, Engine>,
    /// (from node, to node) → design edge index, and whether that runs against the edge.
    edge_of: BTreeMap<(u32, u32), (u16, bool)>,
    /// Busy server-µs per node at the start of the current window.
    prev_busy: Vec<f64>,
    prev_t: u64,
    last_util: Vec<u8>,
    frame_no: u32,
}

impl LiveCore {
    pub fn new(compiled: Arc<Compiled>, design: &Design, seed: u64) -> LiveCore {
        let index: BTreeMap<&str, u32> = compiled
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i as u32))
            .collect();
        let mut edge_of = BTreeMap::new();
        for (i, e) in design.edges.iter().enumerate() {
            if let (Some(&a), Some(&b)) = (index.get(e.from.as_str()), index.get(e.to.as_str())) {
                edge_of.insert((a, b), (i as u16, false));
            }
        }
        // A hop with no edge its way may travel one backwards (a pull, an answer).
        for (i, e) in design.edges.iter().enumerate() {
            if let (Some(&a), Some(&b)) = (index.get(e.from.as_str()), index.get(e.to.as_str())) {
                edge_of.entry((b, a)).or_insert((i as u16, true));
            }
        }
        let n = compiled.nodes.len();
        let mut engine = Engine::new(compiled, seed);
        engine.set_trace(true);
        let mut snapshots = BTreeMap::new();
        snapshots.insert(0, engine.clone());
        LiveCore {
            engine,
            snapshots,
            edge_of,
            prev_busy: vec![0.0; n],
            prev_t: 0,
            last_util: vec![0; n],
            frame_no: 0,
        }
    }

    pub fn now_us(&self) -> u64 {
        self.engine.now_us()
    }

    pub fn duration_us(&self) -> u64 {
        self.engine.duration_us()
    }

    pub fn finished(&self) -> bool {
        self.engine.finished()
    }

    pub fn result(&self) -> RunResult {
        self.engine.result()
    }

    /// A fingerprint of the engine state, for tests (equal states print equal results).
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Advance to `t` (µs), keeping a snapshot at every 5 s boundary crossed.
    fn advance(&mut self, t: u64) {
        let mut next = (self.engine.now_us() / SNAPSHOT_EVERY_US + 1) * SNAPSHOT_EVERY_US;
        while next <= t && next <= self.engine.duration_us() {
            self.engine.advance_to(next);
            if !self.snapshots.contains_key(&next) {
                let mut snap = self.engine.clone();
                snap.take_packets();
                self.snapshots.insert(next, snap);
            }
            next += SNAPSHOT_EVERY_US;
        }
        self.engine.advance_to(t);
    }

    /// Advance by `dt_us` simulated µs (0 while paused) and encode a frame.
    pub fn step(&mut self, dt_us: u64, playing: bool, speed: f64) -> Vec<u8> {
        let target = self.engine.now_us() + dt_us;
        self.advance(target);
        self.frame(playing, speed)
    }

    /// Jump to `t_us`: restore the nearest snapshot at or before it, then re-simulate. The
    /// state is exactly the one playing through would reach (tested).
    pub fn seek(&mut self, t_us: u64) {
        let t = t_us.min(self.engine.duration_us());
        let (_, snap) = self
            .snapshots
            .range(..=t)
            .next_back()
            .expect("the snapshot at 0 is always there");
        self.engine = snap.clone();
        self.advance(t);
        self.engine.take_packets();
        let snap = self.engine.snapshot();
        self.prev_busy = snap.nodes.iter().map(|n| n.busy_us).collect();
        self.prev_t = snap.t_us;
    }

    fn frame(&mut self, playing: bool, speed: f64) -> Vec<u8> {
        let snap = self.engine.snapshot();
        let window_us = snap.t_us.saturating_sub(self.prev_t);
        let packets = self.engine.take_packets();
        let finished = self.engine.finished();
        self.frame_no = self.frame_no.wrapping_add(1);

        let mut out = Vec::with_capacity(
            HEADER_BYTES
                + snap.nodes.len() * NODE_BYTES
                + (MAX_HOPS_PER_FRAME + MAX_EVENTS_PER_FRAME) * PACKET_BYTES,
        );
        out.extend_from_slice(MAGIC);
        out.push(1);
        out.push(
            if playing && !finished {
                FLAG_PLAYING
            } else {
                0
            } | if finished { FLAG_FINISHED } else { 0 },
        );
        out.extend_from_slice(&self.frame_no.to_le_bytes());
        out.extend_from_slice(&((snap.t_us / 1000) as u32).to_le_bytes());
        out.extend_from_slice(&((window_us / 1000).min(u64::from(u16::MAX)) as u16).to_le_bytes());
        let (p95, rps, err) = match &snap.last_second {
            Some(s) => {
                let done = s.ok + s.errors;
                (
                    s.p95_ms,
                    s.requests,
                    if done > 0.0 { s.errors / done } else { 0.0 },
                )
            }
            None => (0.0, 0.0, 0.0),
        };
        for v in [speed, p95, rps, err, snap.cost_month] {
            out.extend_from_slice(&(v as f32).to_le_bytes());
        }
        out.extend_from_slice(&(snap.nodes.len() as u16).to_le_bytes());
        // Thin hops and events evenly to their caps (a deterministic stride).
        let mapped: Vec<_> = packets
            .iter()
            .filter_map(|p| self.edge_of.get(&(p.from, p.to)).map(|e| (*e, p)))
            .collect();
        let thin = |events: bool, cap: usize| {
            let all: Vec<_> = mapped
                .iter()
                .filter(|(_, p)| (p.hit || p.drop) == events)
                .collect();
            let stride = all.len().div_ceil(cap).max(1);
            all.into_iter().step_by(stride).collect::<Vec<_>>()
        };
        let mut chosen = thin(false, MAX_HOPS_PER_FRAME);
        chosen.extend(thin(true, MAX_EVENTS_PER_FRAME));
        out.extend_from_slice(&(chosen.len() as u16).to_le_bytes());
        out.extend_from_slice(&(mapped.len().min(u32::MAX as usize) as u32).to_le_bytes());
        debug_assert_eq!(out.len(), HEADER_BYTES);

        for (i, n) in snap.nodes.iter().enumerate() {
            let util = if window_us > 0 && n.servers > 0.0 {
                let u = (n.busy_us - self.prev_busy[i]) / (n.servers * window_us as f64);
                (u.clamp(0.0, 1.0) * 100.0).round() as u8
            } else {
                self.last_util[i]
            };
            self.last_util[i] = util;
            self.prev_busy[i] = n.busy_us;
            let flags = if n.down { NODE_DOWN } else { 0 }
                | if util >= 95 || n.queue >= 1.0 {
                    NODE_OVERLOADED
                } else {
                    0
                };
            out.extend_from_slice(&(i as u16).to_le_bytes());
            out.push(util);
            out.extend_from_slice(&(n.queue.round().min(f64::from(u16::MAX)) as u16).to_le_bytes());
            out.push(flags);
        }
        let window_start = self.prev_t;
        for ((edge, reverse), p) in chosen {
            let ty = (p.ty.min(7) as u8)
                | if *reverse { PACKET_REVERSE } else { 0 }
                | if p.is_async { PACKET_ASYNC } else { 0 }
                | if p.hit { PACKET_HIT } else { 0 }
                | if p.drop { PACKET_DROP } else { 0 };
            let start_ms =
                (p.start_us.saturating_sub(window_start) / 1000).min(u64::from(u16::MAX));
            let travel = (p.travel_us / 100).min(u64::from(u16::MAX));
            out.extend_from_slice(&edge.to_le_bytes());
            out.push(ty);
            out.extend_from_slice(&(start_ms as u16).to_le_bytes());
            out.extend_from_slice(&(travel as u16).to_le_bytes());
        }
        self.prev_t = snap.t_us;
        out
    }
}

/// Where frames go. `send` returns false once the receiver is gone (the run then stops).
pub trait FrameSink: Send + 'static {
    fn send(&mut self, frame: Vec<u8>) -> bool;
}

/// Controls for a running `LiveRun`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Control {
    Pause,
    Resume,
    /// Jump to this simulated time, in seconds; playback pauses there.
    Seek(f64),
    /// One of `SPEEDS`; other values snap to the nearest.
    Speed(f64),
    Stop,
}

/// Snap a requested speed to the nearest allowed one.
pub fn snap_speed(speed: f64) -> f64 {
    SPEEDS
        .iter()
        .copied()
        .min_by(|a, b| (a - speed).abs().total_cmp(&(b - speed).abs()))
        .unwrap_or(1.0)
}

/// A live run on its own thread.
pub struct LiveRun {
    tx: Sender<Control>,
    result: Arc<Mutex<Option<RunResult>>>,
    handle: Option<JoinHandle<()>>,
}

impl LiveRun {
    /// Start playing `core` at `speed`, sending frames to `sink` about 30 times a second.
    pub fn start(core: LiveCore, sink: impl FrameSink, speed: f64) -> LiveRun {
        let (tx, rx) = channel();
        let result = Arc::new(Mutex::new(None));
        let out = result.clone();
        let handle = std::thread::Builder::new()
            .name("ka-live-run".into())
            .spawn(move || run_loop(core, sink, rx, snap_speed(speed), &out))
            .expect("spawn the live-run thread");
        LiveRun {
            tx,
            result,
            handle: Some(handle),
        }
    }

    pub fn control(&self, c: Control) {
        let _ = self.tx.send(c);
    }

    /// The final result, once the run has reached its end.
    pub fn result(&self) -> Option<RunResult> {
        self.result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn is_alive(&self) -> bool {
        self.handle.as_ref().is_some_and(|h| !h.is_finished())
    }
}

impl Drop for LiveRun {
    fn drop(&mut self) {
        let _ = self.tx.send(Control::Stop);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run_loop(
    mut core: LiveCore,
    mut sink: impl FrameSink,
    rx: Receiver<Control>,
    mut speed: f64,
    result: &Mutex<Option<RunResult>>,
) {
    let mut playing = true;
    let mut last = Instant::now();
    let mut sent_final = false;
    loop {
        let wait = FRAME_INTERVAL.saturating_sub(last.elapsed());
        match rx.recv_timeout(wait) {
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Ok(Control::Pause) => playing = false,
            Ok(Control::Resume) => playing = !core.finished(),
            Ok(Control::Speed(s)) => speed = snap_speed(s),
            Ok(Control::Seek(t_s)) => {
                core.seek((t_s.max(0.0) * 1e6) as u64);
                playing = false;
                sent_final = false;
                last = Instant::now();
                if !sink.send(core.step(0, playing, speed)) {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        if last.elapsed() < FRAME_INTERVAL {
            continue;
        }
        let wall = last.elapsed();
        last = Instant::now();
        if core.finished() {
            if !sent_final {
                *result.lock().unwrap_or_else(|e| e.into_inner()) = Some(core.result());
                sent_final = true;
                if !sink.send(core.step(0, false, speed)) {
                    return;
                }
            }
            continue;
        }
        if !playing {
            continue;
        }
        // Never jump more than a few frames after a stall (a slow machine slows the run).
        let dt = (wall.min(FRAME_INTERVAL * 4).as_secs_f64() * speed * 1e6) as u64;
        if !sink.send(core.step(dt, true, speed)) {
            return;
        }
    }
}

/// A frame from a small fixed design three simulated seconds in, for the UI decoder's
/// test (`apps/desktop/src/test/fixtures/frame.kaf`, written by the export-bindings bin).
#[doc(hidden)]
pub fn fixture_frame() -> Vec<u8> {
    use ka_core::{ComponentKind as K, Edge, Flavour, Node, Point, PropValue, Protocol, registry};
    let node = |id: &str, kind, flavour, props: &[(&str, f64)]| Node {
        id: id.into(),
        kind,
        flavour,
        label: String::new(),
        position: Point { x: 0.0, y: 0.0 },
        props: props
            .iter()
            .map(|(k, v)| ((*k).to_owned(), PropValue::Number(*v)))
            .collect(),
        group_id: None,
    };
    let edge = |id: &str, from: &str, to: &str, protocol| Edge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        protocol,
    };
    let mut d = Design::empty("fixture", "Fixture");
    d.nodes = vec![
        node("c", K::Client, Flavour::Web, &[("rps", 2000.0)]),
        node("a", K::App, Flavour::Go, &[("instances", 2.0)]),
        node("r", K::Cache, Flavour::Redis, &[]),
        node("db", K::Sql, Flavour::Postgresql, &[]),
    ];
    d.edges = vec![
        edge("e1", "c", "a", Protocol::Http),
        edge("e2", "a", "r", Protocol::Query),
        edge("e3", "a", "db", Protocol::Query),
    ];
    let r = registry();
    let s = crate::sandbox::sandbox_scenario(&d, &r, crate::sandbox::SandboxPreset::Normal)
        .expect("the fixture design runs");
    let c =
        Arc::new(ka_sim::compile(&d, &r, &s, &ka_sim::SimOptions::default()).expect("compiles"));
    let mut core = LiveCore::new(c, &d, 1);
    let mut f = Vec::new();
    for _ in 0..90 {
        f = core.step(33_333, true, 1.0);
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::{SandboxPreset, sandbox_scenario};
    use ka_core::{ComponentKind as K, Edge, Flavour, Node, Point, PropValue, Protocol, registry};
    use ka_sim::{SimOptions, compile};

    fn node(id: &str, kind: K, flavour: Flavour, props: &[(&str, f64)]) -> Node {
        Node {
            id: id.into(),
            kind,
            flavour,
            label: String::new(),
            position: Point { x: 0.0, y: 0.0 },
            props: props
                .iter()
                .map(|(k, v)| ((*k).to_owned(), PropValue::Number(*v)))
                .collect(),
            group_id: None,
        }
    }

    fn design() -> Design {
        let mut d = Design::empty("live", "Live");
        d.nodes = vec![
            node("c", K::Client, Flavour::Web, &[("rps", 2000.0)]),
            node("a", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("r", K::Cache, Flavour::Redis, &[]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ];
        let e = |id: &str, from: &str, to: &str, protocol| Edge {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            protocol,
        };
        d.edges = vec![
            e("e1", "c", "a", Protocol::Http),
            e("e2", "a", "r", Protocol::Query),
            e("e3", "a", "db", Protocol::Query),
        ];
        d
    }

    fn core(seed: u64) -> LiveCore {
        let d = design();
        let r = registry();
        let s = sandbox_scenario(&d, &r, SandboxPreset::Normal).unwrap();
        let c = Arc::new(compile(&d, &r, &s, &SimOptions::default()).unwrap());
        LiveCore::new(c, &d, seed)
    }

    fn u16_at(b: &[u8], i: usize) -> u16 {
        u16::from_le_bytes([b[i], b[i + 1]])
    }
    fn u32_at(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }
    fn f32_at(b: &[u8], i: usize) -> f32 {
        f32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }

    #[test]
    fn frames_follow_the_documented_layout() {
        let mut c = core(1);
        let mut f = Vec::new();
        for _ in 0..90 {
            f = c.step(33_333, true, 1.0);
        }
        assert_eq!(&f[0..4], MAGIC);
        assert_eq!(f[4], 1);
        assert_eq!(f[5], FLAG_PLAYING);
        assert_eq!(u32_at(&f, 6), 90);
        assert_eq!(u32_at(&f, 10), 2999, "t in ms");
        assert_eq!(u16_at(&f, 14), 33, "window in ms");
        assert_eq!(f32_at(&f, 16), 1.0, "speed");
        let rps = f32_at(&f, 24);
        assert!((1500.0..2500.0).contains(&rps), "rps {rps}");
        let nodes = usize::from(u16_at(&f, 36));
        let packets = usize::from(u16_at(&f, 38));
        let total = u32_at(&f, 40) as usize;
        assert_eq!(nodes, 4);
        assert!(
            (1..=MAX_HOPS_PER_FRAME + MAX_EVENTS_PER_FRAME).contains(&packets),
            "{packets}"
        );
        assert!(total >= packets);
        assert_eq!(
            f.len(),
            HEADER_BYTES + nodes * NODE_BYTES + packets * PACKET_BYTES
        );
        // The app (node 1) is busy, the client (node 0) never is.
        let node = |i: usize| HEADER_BYTES + i * NODE_BYTES;
        assert_eq!(u16_at(&f, node(1)), 1);
        assert!(f[node(1) + 2] > 0);
        assert_eq!(f[node(0) + 2], 0);
        // Every packet names a design edge; cache hits are marked on the app → cache edge.
        let mut hits = 0;
        for p in 0..packets {
            let at = HEADER_BYTES + nodes * NODE_BYTES + p * PACKET_BYTES;
            assert!(u16_at(&f, at) < 3);
            assert!(u16_at(&f, at + 3) <= 34, "starts within the window");
            if f[at + 2] & PACKET_HIT != 0 {
                hits += 1;
                assert_eq!(u16_at(&f, at), 1, "the cache edge");
                assert_eq!(u16_at(&f, at + 5), 0, "an event has no travel");
            }
        }
        assert!(hits > 0, "a warm cache hits");
    }

    #[test]
    fn an_overloaded_card_shows_drops_and_its_flags() {
        let mut d = design();
        d.nodes[1] = node(
            "a",
            K::App,
            Flavour::Go,
            &[("instances", 1.0), ("concurrency", 1.0)],
        );
        let r = registry();
        let s = sandbox_scenario(&d, &r, SandboxPreset::Normal).unwrap();
        let c = Arc::new(compile(&d, &r, &s, &SimOptions::default()).unwrap());
        let mut core = LiveCore::new(c, &d, 1);
        let mut f = Vec::new();
        for _ in 0..30 {
            f = core.step(33_333, true, 1.0);
        }
        let nodes = usize::from(u16_at(&f, 36));
        let packets = usize::from(u16_at(&f, 38));
        let app = HEADER_BYTES + NODE_BYTES;
        assert_eq!(f[app + 2], 100, "saturated");
        assert_eq!(f[app + 5] & NODE_OVERLOADED, NODE_OVERLOADED);
        let drops = (0..packets)
            .map(|p| HEADER_BYTES + nodes * NODE_BYTES + p * PACKET_BYTES)
            .filter(|at| f[at + 2] & PACKET_DROP != 0)
            .inspect(|at| assert_eq!(u16_at(&f, *at), 0, "dropped on the client → app edge"))
            .count();
        assert!(drops > 0);
    }

    #[test]
    fn a_seek_lands_in_the_state_playing_through_reaches() {
        let mut played = core(7);
        while played.now_us() < 12_300_000 {
            played.step(33_333, true, 1.0);
        }
        let t = played.now_us();
        let mut seeked = core(7);
        while seeked.now_us() < 20_000_000 {
            seeked.step(500_000, true, 8.0);
        }
        seeked.seek(t);
        assert_eq!(seeked.now_us(), t);
        assert_eq!(seeked.engine().snapshot(), played.engine().snapshot());
        // And they finish identically.
        while !played.finished() {
            played.step(1_000_000, true, 8.0);
        }
        while !seeked.finished() {
            seeked.step(1_000_000, true, 8.0);
        }
        assert_eq!(
            serde_json::to_string(&played.result()).unwrap(),
            serde_json::to_string(&seeked.result()).unwrap()
        );
    }

    #[test]
    fn a_seek_forward_past_the_snapshots_simulates_the_gap() {
        let mut c = core(3);
        c.step(1_000_000, true, 1.0);
        c.seek(42_000_000);
        assert_eq!(c.now_us(), 42_000_000);
        let f = c.step(0, false, 1.0);
        assert_eq!(f[5], 0, "paused");
        assert_eq!(u32_at(&f, 10), 42_000);
    }

    #[test]
    fn speeds_snap_to_the_allowed_steps() {
        assert_eq!(snap_speed(3.0), 2.0);
        assert_eq!(snap_speed(100.0), 8.0);
        assert_eq!(snap_speed(0.1), 0.25);
    }

    struct Collect(std::sync::mpsc::Sender<Vec<u8>>);
    impl FrameSink for Collect {
        fn send(&mut self, frame: Vec<u8>) -> bool {
            self.0.send(frame).is_ok()
        }
    }

    #[test]
    fn a_live_run_paces_frames_and_obeys_controls() {
        let (tx, rx) = channel();
        let run = LiveRun::start(core(1), Collect(tx), 8.0);
        let first = rx.recv_timeout(Duration::from_secs(5)).expect("a frame");
        assert_eq!(&first[0..4], MAGIC);
        run.control(Control::Seek(30.0));
        // Frames after the seek are paused at 30 s.
        let paused = loop {
            let f = rx.recv_timeout(Duration::from_secs(5)).expect("a frame");
            if f[5] & FLAG_PLAYING == 0 {
                break f;
            }
        };
        assert_eq!(u32_at(&paused, 10), 30_000);
        run.control(Control::Resume);
        let resumed = rx.recv_timeout(Duration::from_secs(5)).expect("a frame");
        assert!(u32_at(&resumed, 10) > 30_000);
        assert!(run.result().is_none(), "not finished yet");
        drop(run);
        // The thread stopped: the sink's channel closes.
        while rx.recv_timeout(Duration::from_secs(5)).is_ok() {}
    }
}

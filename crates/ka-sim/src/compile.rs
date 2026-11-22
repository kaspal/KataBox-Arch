//! Design + scenario → an immutable simulation graph with precomputed routing (ADR 0006).
//! Everything that can be decided before the first event is decided here, once, so the
//! engine only follows tables. A request type that can't complete is a lint error.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use ka_core::rng::Zipf;
use ka_core::scenario::{RequestType, Scenario, Source};
use ka_core::{ComponentKind, Design, Flavour, GroupKind, Protocol, Registry, detmath};

use crate::behaviour::{NodeSpec, node_spec};
use crate::route::{Role, Route, Step, parse_route};

/// Simulated arrivals per second above which requests are sampled (ADR 0006, ★1).
pub const SAMPLE_CAP_RPS: f64 = 5_000.0;
/// Every multi-server lane keeps at least this many simulated servers (review A1).
pub const MIN_EFFECTIVE_SERVERS: f64 = 4.0;
/// Cache keys tracked one by one; colder keys use the analytic hit probability.
pub const TRACKED_KEYS: u64 = 200_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimError {
    /// A request type can't complete in this design (the judge's Invalid Design).
    NoPath {
        request_type: String,
        reason: String,
    },
    /// The scenario itself is malformed (content bug).
    BadScenario(String),
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::NoPath {
                request_type,
                reason,
            } => write!(f, "no path for `{request_type}`: {reason}"),
            SimError::BadScenario(m) => write!(f, "bad scenario: {m}"),
        }
    }
}

/// Where a call goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Node(u32),
    /// A data cluster (primary + replicas): writes to the primary, reads spread.
    Cluster(u32),
    External(u32),
}

#[derive(Debug, Clone)]
pub struct CNode {
    pub id: String,
    pub kind: ComponentKind,
    pub flavour: Flavour,
    pub spec: NodeSpec,
    pub az: Option<u32>,
    pub region: Option<u32>,
    /// Outgoing edges: (target, protocol), in edge order.
    pub out: Vec<(u32, Protocol)>,
    /// Sampling: simulated servers per lane, queue slots per lane, service-time stretch.
    pub lane_servers: u32,
    pub lane_queue: usize,
    pub service_stretch: f64,
    /// The flavour's monthly price model.
    pub cost: ka_core::Cost,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    /// Members in id order; the first is the initial primary (the root of the Replicate
    /// edges); on failover the next healthy member in this order is promoted.
    pub members: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct TypePlan {
    /// Per node: the next hops a request heading for an entry may take.
    pub next_hops: Vec<Vec<u32>>,
    /// Per node: is it an entry this type can complete from?
    pub entry_ok: Vec<bool>,
    /// [segment][caller node] → per call step, the targets (empty: optional, skipped).
    pub calls: Vec<Vec<Option<Vec<Vec<Target>>>>>,
    /// [segment ≥ 1][caller of the previous segment] → brokers it publishes to.
    pub brokers: Vec<Vec<Vec<u32>>>,
    /// [segment ≥ 1][broker] → consumers that take over as caller.
    pub consumers: Vec<Vec<Vec<u32>>>,
}

#[derive(Debug, Clone)]
pub struct CType {
    pub def: RequestType,
    pub route: Route,
    /// Real requests per second at a traffic multiplier of 1.
    pub rate: f64,
    pub zipf: Option<Zipf>,
    /// The Zipf normaliser H(N, s), for analytic hit probabilities.
    pub harmonic: f64,
    pub skew: f64,
    pub plan: TypePlan,
}

#[derive(Debug, Clone)]
pub struct CSource {
    pub node: u32,
    pub ty: u16,
    /// Real requests per second at a multiplier of 1.
    pub rate: f64,
}

/// A Region or AZ, for partitions (`ChaosEvent::Partition` names its id or label).
#[derive(Debug, Clone)]
pub struct Zone {
    pub id: String,
    pub label: String,
}

/// An autoscaling wrapper: the compute cards inside it and its policy.
#[derive(Debug, Clone)]
pub struct ScaleGroup {
    pub members: Vec<u32>,
    pub min: f64,
    pub max: f64,
    /// Target utilisation, 0–1.
    pub target: f64,
    pub delay_us: u64,
}

#[derive(Debug, Clone)]
pub struct Compiled {
    pub nodes: Vec<CNode>,
    pub clusters: Vec<Cluster>,
    pub cluster_of: Vec<Option<u32>>,
    pub types: Vec<CType>,
    pub sources: Vec<CSource>,
    pub externals: Vec<ka_core::scenario::External>,
    /// Broker → its dead-letter queue, if a Publish edge leads to one.
    pub dlq_of: Vec<Option<u32>>,
    /// Consumer → the brokers it consumes from (to dispatch when a slot frees).
    pub feeds: Vec<Vec<u32>>,
    /// Sampling: the share of requests simulated, and the weight of each (1/f).
    pub sample: f64,
    pub weight: f64,
    pub scenario: Scenario,
    pub duration_us: u64,
    /// Zones by group index (CNode::az / region point here).
    pub zones: Vec<Zone>,
    pub scaling: Vec<ScaleGroup>,
}

const SYNC: [Protocol; 3] = [Protocol::Http, Protocol::Grpc, Protocol::Ws];

/// Network latency of one hop, in µs (ADR 0006).
pub fn hop_latency_us(c: &Compiled, a: u32, b: u32, proto: Protocol) -> u64 {
    let base = match proto {
        Protocol::Http => 500,
        Protocol::Grpc | Protocol::Ws => 300,
        Protocol::Query => 400,
        Protocol::Publish | Protocol::Consume => 1000,
        Protocol::Replicate => 0,
    };
    let (na, nb) = (&c.nodes[a as usize], &c.nodes[b as usize]);
    let zone = match (na.region, nb.region) {
        (Some(ra), Some(rb)) if ra != rb => 70_000,
        _ => match (na.az, nb.az) {
            (Some(x), Some(y)) if x != y => 1_500,
            _ => 200,
        },
    };
    base + zone
}

/// The protocol of the edge from `a` to `b` (the first one, if several).
pub fn edge_protocol(c: &Compiled, a: u32, b: u32) -> Protocol {
    c.nodes[a as usize]
        .out
        .iter()
        .find(|(t, _)| *t == b)
        .map_or(Protocol::Http, |(_, p)| *p)
}

/// Knobs for how a run is simulated (never for what it models).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimOptions {
    /// Simulated arrivals per second above which requests are sampled.
    pub sample_cap: f64,
}

impl Default for SimOptions {
    fn default() -> Self {
        SimOptions {
            sample_cap: SAMPLE_CAP_RPS,
        }
    }
}

pub fn compile(
    design: &Design,
    registry: &Registry,
    scenario: &Scenario,
    opts: &SimOptions,
) -> Result<Compiled, SimError> {
    if scenario.duration_s.is_nan() || scenario.duration_s <= 0.0 {
        return Err(SimError::BadScenario("duration_s must be positive".into()));
    }
    let share_sum: f64 = scenario.request_types.iter().map(|t| t.share).sum();
    if scenario.request_types.is_empty() || share_sum.is_nan() || share_sum <= 0.0 {
        return Err(SimError::BadScenario("no request types".into()));
    }

    // --- Nodes, zones, edges -----------------------------------------------------------
    let index: BTreeMap<&str, u32> = design
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i as u32))
        .collect();
    let groups: BTreeMap<&str, &ka_core::Group> =
        design.groups.iter().map(|g| (g.id.as_str(), g)).collect();
    let group_index: BTreeMap<&str, u32> = design
        .groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.id.as_str(), i as u32))
        .collect();
    let zone_of = |start: Option<&str>| {
        let (mut az, mut region) = (None, None);
        let mut at = start;
        let mut guard = 0;
        while let Some(id) = at {
            guard += 1;
            if guard > 64 {
                break;
            }
            let Some(g) = groups.get(id) else { break };
            match g.kind {
                GroupKind::Az if az.is_none() => az = group_index.get(id).copied(),
                GroupKind::Region if region.is_none() => region = group_index.get(id).copied(),
                _ => {}
            }
            at = g.parent_id.as_deref();
        }
        (az, region)
    };
    let mut nodes: Vec<CNode> = design
        .nodes
        .iter()
        .map(|n| {
            let (az, region) = zone_of(n.group_id.as_deref());
            CNode {
                id: n.id.clone(),
                kind: n.kind,
                flavour: n.flavour,
                spec: node_spec(registry, n),
                az,
                region,
                out: Vec::new(),
                lane_servers: 1,
                lane_queue: 1,
                service_stretch: 1.0,
                cost: registry
                    .kind(n.kind)
                    .flavour(n.flavour)
                    .map(|f| f.cost)
                    .unwrap_or_default(),
            }
        })
        .collect();
    for e in &design.edges {
        if let (Some(&a), Some(&b)) = (index.get(e.from.as_str()), index.get(e.to.as_str())) {
            nodes[a as usize].out.push((b, e.protocol));
        }
    }
    let n_nodes = nodes.len();

    // --- Autoscaling wrappers ------------------------------------------------------------
    let autoscale = registry.kind(ComponentKind::Autoscale);
    let mut scaling = Vec::new();
    for g in design
        .groups
        .iter()
        .filter(|g| g.kind == GroupKind::Autoscale)
    {
        let Some(policy) = &g.scaling else { continue };
        let num = |key: &str, fallback: f64| match policy.props.get(key) {
            Some(ka_core::PropValue::Number(v)) => *v,
            _ => match autoscale.prop(key).map(|p| &p.default) {
                Some(ka_core::PropValue::Number(v)) => *v,
                _ => fallback,
            },
        };
        let members: Vec<u32> = design
            .nodes
            .iter()
            .filter(|n| n.group_id.as_deref() == Some(g.id.as_str()))
            .filter_map(|n| index.get(n.id.as_str()).copied())
            .filter(|i| {
                nodes[*i as usize].spec.role == Role::Compute
                    || nodes[*i as usize].spec.role == Role::Worker
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let min = num("min_instances", 1.0).max(1.0);
        let max = num("max_instances", min).max(min);
        for &m in &members {
            let spec = &mut nodes[m as usize].spec;
            let start = spec.instances.clamp(min, max);
            spec.servers *= start / spec.instances;
            spec.queue_cap *= start / spec.instances;
            spec.instances = start;
        }
        scaling.push(ScaleGroup {
            members,
            min,
            max,
            target: (num("target_util", 60.0) / 100.0).clamp(0.05, 0.99),
            delay_us: (num("scale_delay_s", 60.0).max(0.0) * 1e6) as u64,
        });
    }
    let kind = |i: u32| nodes[i as usize].kind;

    // --- Data clusters (Replicate edges within one data kind) ---------------------------
    let is_data = |k: ComponentKind| matches!(Role::of(k), Role::Datastore | Role::Cache);
    let mut incoming_rep = vec![false; n_nodes];
    for (a, n) in nodes.iter().enumerate() {
        for (b, p) in &n.out {
            if *p == Protocol::Replicate
                && is_data(n.kind)
                && nodes[*b as usize].kind == n.kind
                && *b as usize != a
            {
                incoming_rep[*b as usize] = true;
            }
        }
    }
    let mut cluster_of: Vec<Option<u32>> = vec![None; n_nodes];
    let mut clusters = Vec::new();
    for root in 0..n_nodes {
        if !is_data(nodes[root].kind) || incoming_rep[root] || cluster_of[root].is_some() {
            continue;
        }
        let id = clusters.len() as u32;
        let mut members = vec![root as u32];
        cluster_of[root] = Some(id);
        let mut queue = VecDeque::from([root as u32]);
        while let Some(m) = queue.pop_front() {
            for (b, p) in &nodes[m as usize].out {
                if *p == Protocol::Replicate
                    && cluster_of[*b as usize].is_none()
                    && nodes[*b as usize].kind == nodes[m as usize].kind
                {
                    cluster_of[*b as usize] = Some(id);
                    members.push(*b);
                    queue.push_back(*b);
                }
            }
        }
        // The root stays first; the rest in id order (deterministic failover order).
        let rest: BTreeSet<(String, u32)> = members[1..]
            .iter()
            .map(|m| (nodes[*m as usize].id.clone(), *m))
            .collect();
        members.truncate(1);
        members.extend(rest.into_iter().map(|(_, m)| m));
        clusters.push(Cluster { members });
    }
    // Replicas whose root isn't a data node (cycles): their own cluster.
    for i in 0..n_nodes {
        if is_data(nodes[i].kind) && cluster_of[i].is_none() {
            cluster_of[i] = Some(clusters.len() as u32);
            clusters.push(Cluster {
                members: vec![i as u32],
            });
        }
    }

    let dlq_of: Vec<Option<u32>> = nodes
        .iter()
        .map(|n| {
            n.out
                .iter()
                .find(|(b, p)| *p == Protocol::Publish && kind(*b) == ComponentKind::Dlq)
                .map(|(b, _)| *b)
        })
        .collect();

    // --- Request types and their plans --------------------------------------------------
    let externals = scenario.externals.clone();
    let mut types = Vec::new();
    for def in &scenario.request_types {
        let route = parse_route(&def.route, def.source == Source::Scheduler)
            .map_err(|e| SimError::BadScenario(format!("route of `{}`: {e}", def.id)))?;
        let plan = plan_type(&nodes, &clusters, &cluster_of, &externals, def, &route)?;
        let skew = scenario
            .key_skew_override
            .or(def.key_skew)
            .map_or(0.0, |k| k.zipf);
        let zipf = (skew > 0.0).then(|| Zipf::new(def.key_space, skew));
        types.push(CType {
            def: def.clone(),
            route,
            rate: scenario.peak_rps * def.share / share_sum,
            zipf,
            harmonic: harmonic(def.key_space, skew),
            skew,
            plan,
        });
    }

    // --- Sources -------------------------------------------------------------------------
    let client_rps: Vec<f64> = design
        .nodes
        .iter()
        .map(|n| match n.props.get("rps") {
            Some(ka_core::PropValue::Number(v)) => *v,
            _ => match registry.kind(n.kind).prop("rps").map(|p| p.default.clone()) {
                Some(ka_core::PropValue::Number(v)) => v,
                _ => 1.0,
            },
        })
        .collect();
    let mut sources = Vec::new();
    // Sandbox scenarios (peak 0) take the load from the client cards.
    let total_client_rps: f64 = (0..n_nodes)
        .filter(|&i| nodes[i].kind == ComponentKind::Client)
        .map(|i| client_rps[i])
        .sum();
    for (ti, t) in types.iter_mut().enumerate() {
        let want = t.def.source;
        let candidates: Vec<u32> = (0..n_nodes as u32)
            .filter(|&i| match want {
                Source::Client => {
                    kind(i) == ComponentKind::Client && !t.plan.next_hops[i as usize].is_empty()
                }
                Source::Scheduler => {
                    kind(i) == ComponentKind::Scheduler && t.plan.calls[0][i as usize].is_some()
                }
            })
            .collect();
        if candidates.is_empty() {
            let what = if want == Source::Client {
                "client"
            } else {
                "scheduler"
            };
            return Err(SimError::NoPath {
                request_type: t.def.id.clone(),
                reason: format!("no {what} card reaches a component that can serve it"),
            });
        }
        if scenario.peak_rps <= 0.0 {
            t.rate = total_client_rps * t.def.share / share_sum;
        }
        let weights: Vec<f64> = candidates
            .iter()
            .map(|&i| {
                if want == Source::Client {
                    client_rps[i as usize].max(0.0)
                } else {
                    1.0
                }
            })
            .collect();
        let wsum: f64 = weights.iter().sum::<f64>().max(1e-12);
        for (i, w) in candidates.iter().zip(weights) {
            sources.push(CSource {
                node: *i,
                ty: ti as u16,
                rate: t.rate * w / wsum,
            });
        }
    }

    // --- Sampling (★1, review A1) -------------------------------------------------------
    let total_rate: f64 = types.iter().map(|t| t.rate).sum();
    let mut sample = if total_rate > opts.sample_cap {
        opts.sample_cap / total_rate
    } else {
        1.0
    };
    for n in &nodes {
        let per_lane = n.spec.servers / f64::from(n.spec.lanes);
        if per_lane >= 2.0 && n.spec.role != Role::None && n.spec.role != Role::Source {
            sample = sample.max((MIN_EFFECTIVE_SERVERS / per_lane).min(1.0));
        }
    }
    for n in &mut nodes {
        let per_lane = n.spec.servers / f64::from(n.spec.lanes);
        let scaled = per_lane * sample;
        let eff = scaled.round().max(1.0);
        n.lane_servers = eff as u32;
        n.service_stretch = eff / scaled;
        n.lane_queue = ((n.spec.queue_cap / f64::from(n.spec.lanes)) * sample)
            .round()
            .max(1.0) as usize;
    }

    let mut feeds: Vec<Vec<u32>> = vec![Vec::new(); n_nodes];
    for (b, n) in nodes.iter().enumerate() {
        for (c, p) in &n.out {
            if *p == Protocol::Consume && !feeds[*c as usize].contains(&(b as u32)) {
                feeds[*c as usize].push(b as u32);
            }
        }
    }

    Ok(Compiled {
        nodes,
        clusters,
        cluster_of,
        types,
        sources,
        externals,
        dlq_of,
        feeds,
        sample,
        weight: 1.0 / sample,
        scenario: scenario.clone(),
        duration_us: (scenario.duration_s * 1e6) as u64,
        zones: design
            .groups
            .iter()
            .map(|g| Zone {
                id: g.id.clone(),
                label: g.label.clone(),
            })
            .collect(),
        scaling,
    })
}

/// H(N, s) = Σ k^-s for k in 1..=N (exact up to 1000 terms, then the integral).
fn harmonic(n: u64, s: f64) -> f64 {
    if s <= 0.0 {
        return n as f64;
    }
    let exact = n.min(1000);
    let mut h = 0.0;
    for k in 1..=exact {
        h += detmath::powf(k as f64, -s);
    }
    if n > exact {
        let (a, b) = (exact as f64 + 0.5, n as f64 + 0.5);
        h += if (s - 1.0).abs() < 1e-9 {
            detmath::ln(b / a)
        } else {
            (detmath::powf(b, 1.0 - s) - detmath::powf(a, 1.0 - s)) / (1.0 - s)
        };
    }
    h
}

fn targets_for_step(
    nodes: &[CNode],
    clusters: &[Cluster],
    cluster_of: &[Option<u32>],
    externals: &[ka_core::scenario::External],
    caller: u32,
    step: &Step,
) -> Result<Vec<Target>, String> {
    let mut out: Vec<Target> = Vec::new();
    for alt in &step.alts {
        match alt.role {
            Role::External => {
                let name = alt.external.as_deref().unwrap_or_default();
                let i = externals
                    .iter()
                    .position(|x| x.name == name)
                    .ok_or_else(|| format!("the scenario has no external `{name}`"))?;
                out.push(Target::External(i as u32));
            }
            Role::Cache | Role::Datastore => {
                for (b, p) in &nodes[caller as usize].out {
                    if *p == Protocol::Query && alt.accepts(nodes[*b as usize].kind) {
                        let t = cluster_of[*b as usize].map_or(Target::Node(*b), Target::Cluster);
                        if !out.contains(&t) {
                            out.push(t);
                        }
                    }
                }
            }
            Role::Compute | Role::Worker => {
                for (b, p) in &nodes[caller as usize].out {
                    if matches!(p, Protocol::Http | Protocol::Grpc)
                        && alt.accepts(nodes[*b as usize].kind)
                    {
                        out.push(Target::Node(*b));
                    }
                }
            }
            _ => {}
        }
    }
    let _ = clusters;
    Ok(out)
}

/// Routing tables for one request type. Errors name what's missing.
fn plan_type(
    nodes: &[CNode],
    clusters: &[Cluster],
    cluster_of: &[Option<u32>],
    externals: &[ka_core::scenario::External],
    def: &RequestType,
    route: &Route,
) -> Result<TypePlan, SimError> {
    let n = nodes.len();
    let nseg = route.segments.len();
    let bad = |m: String| SimError::BadScenario(format!("route of `{}`: {m}", def.id));
    // Valid callers per segment, computed from the last segment back.
    let mut calls: Vec<Vec<Option<Vec<Vec<Target>>>>> = vec![vec![None; n]; nseg];
    let mut brokers: Vec<Vec<Vec<u32>>> = vec![vec![Vec::new(); n]; nseg];
    let mut consumers: Vec<Vec<Vec<u32>>> = vec![vec![Vec::new(); n]; nseg];
    for s in (0..nseg).rev() {
        let seg = &route.segments[s];
        // Consumers of this segment (s ≥ 1): checked per broker below.
        for caller in 0..n as u32 {
            let role = nodes[caller as usize].spec.role;
            let may_call = role.is_caller()
                || (s == 0 && nodes[caller as usize].kind == ComponentKind::Scheduler);
            if !may_call {
                continue;
            }
            let mut per_step = Vec::new();
            let mut ok = true;
            for step in &seg.calls {
                let t = targets_for_step(nodes, clusters, cluster_of, externals, caller, step)
                    .map_err(bad)?;
                if t.is_empty() && !step.optional {
                    ok = false;
                    break;
                }
                per_step.push(t);
            }
            if !ok {
                continue;
            }
            // Publish to the next segment's broker, which needs a valid consumer.
            if s + 1 < nseg {
                let next = &route.segments[s + 1];
                let bstep = next
                    .broker
                    .as_ref()
                    .expect("segments after the first have a broker");
                if bstep.cdc {
                    // The database written by the last call publishes (change data capture):
                    // keep the write targets with a member that feeds a usable stream.
                    let streams_of = |m: u32| -> Vec<u32> {
                        nodes[m as usize]
                            .out
                            .iter()
                            .filter(|(b, p)| {
                                *p == Protocol::Publish && bstep.accepts(nodes[*b as usize].kind)
                            })
                            .map(|(b, _)| *b)
                            .filter(|b| !consumers[s + 1][*b as usize].is_empty())
                            .collect()
                    };
                    let members = |t: &Target| -> Vec<u32> {
                        match t {
                            Target::Cluster(cl) => clusters[*cl as usize].members.clone(),
                            Target::Node(n) => vec![*n],
                            Target::External(_) => Vec::new(),
                        }
                    };
                    let Some(last) = per_step.last_mut() else {
                        continue;
                    };
                    last.retain(|t| members(t).into_iter().any(|m| !streams_of(m).is_empty()));
                    if last.is_empty() {
                        continue;
                    }
                    for t in last.iter() {
                        for m in members(t) {
                            brokers[s + 1][m as usize] = streams_of(m);
                        }
                    }
                    calls[s][caller as usize] = Some(per_step);
                    continue;
                }
                let found: Vec<u32> = nodes[caller as usize]
                    .out
                    .iter()
                    .filter(|(b, p)| {
                        *p == Protocol::Publish && bstep.accepts(nodes[*b as usize].kind)
                    })
                    .map(|(b, _)| *b)
                    .filter(|b| !consumers[s + 1][*b as usize].is_empty())
                    .collect();
                if found.is_empty() {
                    continue;
                }
                brokers[s + 1][caller as usize] = found;
            }
            calls[s][caller as usize] = Some(per_step);
        }
        // For s ≥ 1: which consumers each broker can hand this segment to.
        if s >= 1 {
            let cstep = route.segments[s]
                .consumer
                .as_ref()
                .expect("segments after the first have a consumer");
            for b in 0..n {
                if nodes[b].spec.role != Role::Broker {
                    continue;
                }
                consumers[s][b] = nodes[b]
                    .out
                    .iter()
                    .filter(|(c, p)| {
                        *p == Protocol::Consume && cstep.accepts(nodes[*c as usize].kind)
                    })
                    .map(|(c, _)| *c)
                    .filter(|c| calls[s][*c as usize].is_some())
                    .collect();
            }
        }
    }

    // Entries and hops.
    let mut entry_ok = vec![false; n];
    if let Some(entry) = &route.entry {
        for i in 0..n {
            let k = nodes[i].kind;
            if !entry.accepts(k) {
                continue;
            }
            entry_ok[i] = match nodes[i].spec.role {
                Role::Compute | Role::Worker => calls[0][i].is_some(),
                // Served directly (behind a CDN): nothing may follow.
                Role::Datastore | Role::Cache => nseg == 1 && route.segments[0].calls.is_empty(),
                _ => false,
            };
        }
    }
    // Nodes from which an entry is reachable through edge hops.
    let mut good = entry_ok.clone();
    if route.hops_allowed {
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..n {
                if good[i] || nodes[i].spec.role != Role::Edge {
                    continue;
                }
                if nodes[i]
                    .out
                    .iter()
                    .any(|(b, p)| SYNC.contains(p) && good[*b as usize])
                {
                    good[i] = true;
                    changed = true;
                }
            }
        }
    }
    let mut next_hops = vec![Vec::new(); n];
    for i in 0..n {
        let from_client = nodes[i].kind == ComponentKind::Client;
        if !(from_client || (nodes[i].spec.role == Role::Edge && good[i])) {
            continue;
        }
        next_hops[i] = nodes[i]
            .out
            .iter()
            .filter(|(b, p)| {
                let b = *b as usize;
                let proto_ok = if from_client && route.ws {
                    *p == Protocol::Ws
                } else {
                    SYNC.contains(p)
                };
                let target_ok = (entry_ok[b] && !(from_client && route.hops_required))
                    || (route.hops_allowed && nodes[b].spec.role == Role::Edge && good[b]);
                proto_ok && target_ok
            })
            .map(|(b, _)| *b)
            .collect();
    }
    Ok(TypePlan {
        next_hops,
        entry_ok,
        calls,
        brokers,
        consumers,
    })
}

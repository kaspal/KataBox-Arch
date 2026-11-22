//! docker-compose import (M7b.1): a `docker-compose.yml` becomes a new Sandbox design.
//!
//! The YAML is read event by event (`yaml_rust2::parser::Parser`), never through a loader,
//! so every cap is checked before anything is built (review A6): file size, events, aliases,
//! nesting depth, and the size of the document once aliases are expanded (the "billion
//! laughs" guard). Anchors are fine; an anchored or aliased mapping *key* is refused.
//!
//! Services map to cards by image name; `depends_on` and `links` become connections whose
//! protocol the wiring table allows. Anything that can't be drawn is listed, never guessed
//! silently, and the result must pass `ka_core::validate`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use ka_core::{
    ComponentKind, Design, Edge, Flavour, Node, Point, PropValue, Protocol, Registry, WiringReason,
};
use serde::Serialize;
use yaml_rust2::parser::{Event, Parser};

use crate::error::{AppError, AppErrorCode};

/// Largest compose file read (A6).
pub const MAX_BYTES: usize = 1024 * 1024;
/// Parser events before the file counts as too complex.
pub const MAX_EVENTS: usize = 200_000;
/// Aliases (`*name`) in one file.
pub const MAX_ALIASES: usize = 100;
/// Nesting depth of sequences and mappings.
pub const MAX_DEPTH: usize = 64;
/// Nodes in the document with every alias expanded (the billion-laughs guard).
pub const MAX_EXPANDED: u64 = 200_000;
/// Services in one file.
pub const MAX_SERVICES: usize = 200;
/// `deploy.replicas` above this is clamped and listed (review B4): the registry's
/// `instances` range.
pub const MAX_REPLICAS: f64 = 64.0;

/// The card size the canvas uses (`CARD_W`, `CARD_H` in `solve/store.ts`; a test keeps the
/// two in step), and the layout gaps.
pub const CARD_W: f64 = 208.0;
pub const CARD_H: f64 = 64.0;
const COL_GAP: f64 = 96.0;
const ROW_GAP: f64 = 48.0;

/// What the import did, for the summary dialog.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ComposeImport {
    pub design: Design,
    /// Services mapped by their image, with the card they became.
    pub mapped: Vec<MappedService>,
    /// Services with no known image: drawn as App Servers.
    pub guessed: Vec<MappedService>,
    /// Services whose `deploy.replicas` was above the card's range and was clamped.
    pub clamped: Vec<String>,
    /// Dependencies that weren't drawn, and why.
    pub skipped_edges: Vec<SkippedEdge>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MappedService {
    pub service: String,
    pub image: Option<String>,
    pub kind: ComponentKind,
    pub flavour: Flavour,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SkippedEdge {
    pub from: String,
    pub to: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case", tag = "type", content = "wiring")]
pub enum SkipReason {
    /// The dependency names no service in the file.
    UnknownService,
    /// The wiring table doesn't allow these two kinds to connect.
    NotAllowed(WiringReason),
}

fn err(code: AppErrorCode, msg: impl Into<String>) -> AppError {
    AppError::new(code, msg)
}

fn too_complex(what: &str) -> AppError {
    err(
        AppErrorCode::ComposeTooComplex,
        format!("the compose file has too many {what}"),
    )
}

// --- The YAML tree ------------------------------------------------------------------------

/// A node of the document. Aliases point at an existing node's index, so nothing is copied
/// and the expanded size is computed, not built.
#[derive(Debug)]
enum Y {
    Str(String),
    Seq(Vec<usize>),
    Map(Vec<(String, usize)>),
}

enum Frame {
    Seq(Vec<usize>, usize),
    Map(Vec<(String, usize)>, Option<String>, usize),
}

struct Tree {
    nodes: Vec<Y>,
    root: Option<usize>,
}

impl Tree {
    fn get(&self, map: usize, key: &str) -> Option<usize> {
        match &self.nodes[map] {
            Y::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| *v),
            _ => None,
        }
    }

    fn str(&self, i: usize) -> Option<&str> {
        match &self.nodes[i] {
            Y::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A mapping's entries with `<<` merge keys applied: explicit keys win, then the
    /// merged maps in order (YAML merge-key rules).
    fn merged(&self, map: usize) -> Vec<(String, usize)> {
        let Y::Map(entries) = &self.nodes[map] else {
            return Vec::new();
        };
        let mut out: Vec<(String, usize)> =
            entries.iter().filter(|(k, _)| k != "<<").cloned().collect();
        for (_, v) in entries.iter().filter(|(k, _)| k == "<<") {
            let sources: Vec<usize> = match &self.nodes[*v] {
                Y::Map(_) => vec![*v],
                Y::Seq(items) => items.clone(),
                Y::Str(_) => Vec::new(),
            };
            for s in sources {
                for (k, val) in self.merged(s) {
                    if !out.iter().any(|(ok, _)| *ok == k) {
                        out.push((k, val));
                    }
                }
            }
        }
        out
    }

    /// Nodes in the document with every alias expanded, saturating.
    fn expanded_size(&self) -> u64 {
        // Children always have smaller indices than their parent (a node is pushed when it
        // ends) and aliases only point at finished nodes, so one pass in order suffices.
        let mut size = vec![0u64; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            let kids: u64 = match n {
                Y::Str(_) => 0,
                Y::Seq(items) => items.iter().map(|c| size[*c]).fold(0, u64::saturating_add),
                Y::Map(entries) => entries
                    .iter()
                    .map(|(_, c)| size[*c].saturating_add(1))
                    .fold(0, u64::saturating_add),
            };
            size[i] = kids.saturating_add(1).min(u64::MAX / 2);
        }
        self.root.map_or(0, |r| size[r])
    }
}

fn parse(text: &str) -> Result<Tree, AppError> {
    let mut parser = Parser::new_from_str(text);
    let mut nodes: Vec<Y> = Vec::new();
    let mut anchors: HashMap<usize, usize> = HashMap::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut root = None;
    let (mut events, mut aliases) = (0usize, 0usize);
    let invalid = |e: String| err(AppErrorCode::ComposeInvalid, format!("not valid YAML: {e}"));

    // Hand a finished node to its parent (or make it the root).
    fn attach(stack: &mut [Frame], root: &mut Option<usize>, node: usize) -> Result<(), AppError> {
        match stack.last_mut() {
            None => {
                if root.is_none() {
                    *root = Some(node);
                }
            }
            Some(Frame::Seq(items, _)) => items.push(node),
            Some(Frame::Map(entries, key, _)) => match key.take() {
                Some(k) => entries.push((k, node)),
                None => {
                    return Err(err(
                        AppErrorCode::ComposeUnsupported,
                        "only plain text keys are supported",
                    ));
                }
            },
        }
        Ok(())
    }

    loop {
        let (ev, _) = parser.next_token().map_err(|e| invalid(e.to_string()))?;
        events += 1;
        if events > MAX_EVENTS {
            return Err(too_complex("parts"));
        }
        // A mapping waiting for its key takes only a plain, unanchored scalar.
        let wants_key = matches!(stack.last(), Some(Frame::Map(_, None, _)));
        match ev {
            Event::StreamStart | Event::DocumentStart | Event::Nothing => {}
            // Only the first document is read.
            Event::DocumentEnd | Event::StreamEnd => break,
            Event::Scalar(v, _, anchor, _) => {
                if wants_key {
                    if anchor != 0 {
                        return Err(err(
                            AppErrorCode::ComposeUnsupported,
                            "anchors on keys are not supported",
                        ));
                    }
                    if let Some(Frame::Map(_, key, _)) = stack.last_mut() {
                        *key = Some(v);
                    }
                    continue;
                }
                nodes.push(Y::Str(v));
                let id = nodes.len() - 1;
                if anchor != 0 {
                    anchors.insert(anchor, id);
                }
                attach(&mut stack, &mut root, id)?;
            }
            Event::Alias(a) => {
                aliases += 1;
                if aliases > MAX_ALIASES {
                    return Err(too_complex("aliases"));
                }
                if wants_key {
                    return Err(err(
                        AppErrorCode::ComposeUnsupported,
                        "aliases as keys are not supported",
                    ));
                }
                // An alias to a node still open (a cycle) has no entry yet.
                let Some(&id) = anchors.get(&a) else {
                    return Err(err(
                        AppErrorCode::ComposeUnsupported,
                        "an alias refers to a node that contains it",
                    ));
                };
                attach(&mut stack, &mut root, id)?;
            }
            Event::SequenceStart(anchor, _) | Event::MappingStart(anchor, _) => {
                if wants_key {
                    return Err(err(
                        AppErrorCode::ComposeUnsupported,
                        "only plain text keys are supported",
                    ));
                }
                if stack.len() >= MAX_DEPTH {
                    return Err(too_complex("levels of nesting"));
                }
                stack.push(if matches!(ev, Event::SequenceStart(..)) {
                    Frame::Seq(Vec::new(), anchor)
                } else {
                    Frame::Map(Vec::new(), None, anchor)
                });
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let (node, anchor) = match stack.pop() {
                    Some(Frame::Seq(items, a)) => (Y::Seq(items), a),
                    Some(Frame::Map(entries, _, a)) => (Y::Map(entries), a),
                    None => return Err(invalid("unbalanced document".into())),
                };
                nodes.push(node);
                let id = nodes.len() - 1;
                if anchor != 0 {
                    anchors.insert(anchor, id);
                }
                attach(&mut stack, &mut root, id)?;
            }
        }
    }
    let tree = Tree { nodes, root };
    if tree.expanded_size() > MAX_EXPANDED {
        return Err(too_complex("parts once its aliases are expanded"));
    }
    Ok(tree)
}

// --- Mapping ------------------------------------------------------------------------------

/// The card an image becomes: by image name, without registry, tag or digest.
pub fn map_image(image: &str) -> Option<(ComponentKind, Flavour)> {
    use ComponentKind as K;
    use Flavour as F;
    let no_digest = image.split('@').next().unwrap_or(image);
    let last = no_digest.rsplit('/').next().unwrap_or(no_digest);
    let name = last.split(':').next().unwrap_or(last).to_ascii_lowercase();
    Some(match name.as_str() {
        "postgres" | "postgresql" | "postgis" => (K::Sql, F::Postgresql),
        "mysql" | "mariadb" | "percona" => (K::Sql, F::Mysql),
        "redis" | "valkey" | "redis-stack" | "redis-stack-server" => (K::Cache, F::Redis),
        "memcached" => (K::Cache, F::Memcached),
        "nginx" | "openresty" => (K::Lb, F::Nginx),
        "haproxy" => (K::Lb, F::Haproxy),
        "kafka" | "cp-kafka" | "redpanda" => (K::Stream, F::Kafka),
        "rabbitmq" => (K::Queue, F::Rabbitmq),
        "elasticsearch" => (K::Search, F::Elasticsearch),
        "opensearch" => (K::Search, F::Opensearch),
        "mongo" | "mongodb" | "mongodb-community-server" => (K::Nosql, F::Mongodb),
        "cassandra" | "scylla" | "scylladb" => (K::Nosql, F::Cassandra),
        _ => return None,
    })
}

/// A card id from a service name: `[A-Za-z0-9_-]`, at most 60 characters, unique.
fn node_id(name: &str, taken: &mut BTreeSet<String>) -> String {
    let mut base: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .take(56)
        .collect();
    if base.is_empty() {
        base = "service".into();
    }
    let mut id = base.clone();
    let mut n = 2;
    while !taken.insert(id.clone()) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Layout column of a kind: clients, edge, compute, async, data, ops.
fn column(kind: ComponentKind) -> usize {
    use ka_core::Category as C;
    match (kind, kind.category()) {
        (ComponentKind::Client, _) => 0,
        (_, C::Edge) => 1,
        (_, C::Compute) => 2,
        (_, C::Async) => 3,
        (_, C::Data) => 4,
        (_, C::Ops) => 5,
    }
}

fn snap(v: f64) -> f64 {
    (v / 8.0).round() * 8.0
}

/// FNV-1a, for a design id that is stable for the same file.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Read a compose file's text into a design named `name`.
pub fn import(text: &str, name: &str, registry: &Registry) -> Result<ComposeImport, AppError> {
    if text.len() > MAX_BYTES {
        return Err(err(
            AppErrorCode::ComposeTooLarge,
            format!("the compose file is larger than {MAX_BYTES} bytes"),
        ));
    }
    let tree = parse(text)?;
    let services_node = tree
        .root
        .filter(|r| matches!(tree.nodes[*r], Y::Map(_)))
        .and_then(|r| tree.get(r, "services"))
        .filter(|s| matches!(tree.nodes[*s], Y::Map(_)))
        .ok_or_else(|| err(AppErrorCode::ComposeInvalid, "the file has no `services:`"))?;
    let services = tree.merged(services_node);
    if services.is_empty() {
        return Err(err(
            AppErrorCode::ComposeInvalid,
            "the file has no services",
        ));
    }
    if services.len() > MAX_SERVICES {
        return Err(err(
            AppErrorCode::ComposeTooManyServices,
            format!("the file has more than {MAX_SERVICES} services"),
        ));
    }

    struct Svc {
        name: String,
        id: String,
        kind: ComponentKind,
        flavour: Flavour,
        replicas: Option<f64>,
        deps: Vec<String>,
    }
    let mut taken = BTreeSet::new();
    let mut svcs: Vec<Svc> = Vec::new();
    let (mut mapped, mut guessed, mut clamped) = (Vec::new(), Vec::new(), Vec::new());
    let mut sorted = services.clone();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, node) in sorted {
        let fields = tree.merged(node);
        let field = |k: &str| fields.iter().find(|(f, _)| f == k).map(|(_, v)| *v);
        let image = field("image").and_then(|i| tree.str(i)).map(str::to_owned);
        let (kind, flavour, known) = match image.as_deref().and_then(map_image) {
            Some((k, f)) => (k, f, true),
            None => (ComponentKind::App, Flavour::Go, false),
        };
        let entry = MappedService {
            service: name.clone(),
            image: image.clone(),
            kind,
            flavour,
        };
        if known {
            mapped.push(entry);
        } else {
            guessed.push(entry);
        }
        let replicas = field("deploy")
            .and_then(|d| tree.get(d, "replicas"))
            .and_then(|r| tree.str(r))
            .and_then(|r| r.trim().parse::<f64>().ok())
            .filter(|r| r.is_finite() && *r >= 1.0)
            .map(f64::trunc);
        let replicas = replicas.map(|r| {
            if r > MAX_REPLICAS {
                clamped.push(name.clone());
                MAX_REPLICAS
            } else {
                r
            }
        });
        let mut deps = Vec::new();
        if let Some(d) = field("depends_on") {
            match &tree.nodes[d] {
                Y::Seq(items) => {
                    deps.extend(items.iter().filter_map(|i| tree.str(*i)).map(str::to_owned))
                }
                Y::Map(entries) => deps.extend(entries.iter().map(|(k, _)| k.clone())),
                Y::Str(s) => deps.push(s.clone()),
            }
        }
        if let Some(Y::Seq(items)) = field("links").map(|l| &tree.nodes[l]) {
            deps.extend(
                items
                    .iter()
                    .filter_map(|i| tree.str(*i))
                    .map(|l| l.split(':').next().unwrap_or(l).to_owned()),
            );
        }
        svcs.push(Svc {
            id: node_id(&name, &mut taken),
            name,
            kind,
            flavour,
            replicas,
            deps,
        });
    }

    // Layout: used columns in order, rows by service name (the list is sorted).
    let used: BTreeSet<usize> = svcs.iter().map(|s| column(s.kind)).collect();
    let col_index: BTreeMap<usize, usize> = used.iter().enumerate().map(|(i, c)| (*c, i)).collect();
    let mut rows: BTreeMap<usize, usize> = BTreeMap::new();
    let max_label = registry.limits.max_label as usize;
    let mut design = Design::empty(format!("compose-{:016x}", fnv(text.as_bytes())), name);
    for s in &svcs {
        let col = col_index[&column(s.kind)];
        let row = rows.entry(col).or_default();
        let mut props = BTreeMap::new();
        if let Some(r) = s.replicas
            && registry.kind(s.kind).prop("instances").is_some()
        {
            props.insert("instances".to_owned(), PropValue::Number(r));
        }
        design.nodes.push(Node {
            id: s.id.clone(),
            kind: s.kind,
            flavour: s.flavour,
            label: s.name.chars().take(max_label).collect(),
            position: Point {
                x: snap(col as f64 * (CARD_W + COL_GAP)),
                y: snap(*row as f64 * (CARD_H + ROW_GAP)),
            },
            props,
            group_id: None,
        });
        *row += 1;
    }

    // Edges: a depends_on b draws a → b with the first protocol the table allows.
    let by_name: HashMap<&str, &Svc> = svcs.iter().map(|s| (s.name.as_str(), s)).collect();
    let mut skipped_edges = Vec::new();
    let mut seen = BTreeSet::new();
    for s in &svcs {
        for d in &s.deps {
            let Some(t) = by_name.get(d.as_str()) else {
                skipped_edges.push(SkippedEdge {
                    from: s.name.clone(),
                    to: d.clone(),
                    reason: SkipReason::UnknownService,
                });
                continue;
            };
            if t.id == s.id || !seen.insert((s.id.clone(), t.id.clone())) {
                continue;
            }
            let allowed = registry.wiring.allowed(s.kind, t.kind);
            let pick = [Protocol::Query, Protocol::Publish, Protocol::Http]
                .into_iter()
                .find(|p| allowed.contains(p))
                .or_else(|| allowed.first().copied());
            match pick {
                Some(protocol) => design.edges.push(Edge {
                    id: format!("e{}", design.edges.len() + 1),
                    from: s.id.clone(),
                    to: t.id.clone(),
                    protocol,
                }),
                None => skipped_edges.push(SkippedEdge {
                    from: s.name.clone(),
                    to: t.name.clone(),
                    reason: SkipReason::NotAllowed(registry.wiring.reason(s.kind, t.kind)),
                }),
            }
        }
    }
    let issues = ka_core::validate(&design, registry);
    if !issues.is_empty() {
        return Err(err(
            AppErrorCode::ComposeInvalid,
            format!("the imported design breaks a rule: {issues:?}"),
        ));
    }
    Ok(ComposeImport {
        design,
        mapped,
        guessed,
        clamped,
        skipped_edges,
    })
}

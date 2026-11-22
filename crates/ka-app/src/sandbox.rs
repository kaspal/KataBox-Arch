//! Sandbox traffic (plan ★8): the Sandbox has no problem, so the client cards' `rps` set the
//! load, with a generic read/write mix whose routes adapt to what the design contains.

use ka_core::scenario::Scenario;
use ka_core::{Design, Registry};
use ka_sim::{SimError, SimOptions, compile};
use serde::{Deserialize, Serialize};

/// How long a Sandbox run lasts, in simulated seconds.
pub const SANDBOX_DURATION_S: f64 = 60.0;

/// The traffic presets of the Sandbox's Run menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum SandboxPreset {
    /// Steady traffic at the clients' rate.
    Normal,
    /// ×4 traffic from 20 s to 35 s.
    Spike,
    /// Normal traffic; the busiest data store (or, without one, the busiest service) dies at
    /// 20 s and stays down.
    KillNode,
}

/// When the Kill a node preset strikes.
pub const KILL_AT_S: f64 = 20.0;

/// The read routes to try, most complete first: the first that has a path through the
/// design is used.
const READ_ROUTES: [&[&str]; 3] = [
    &["edge?", "compute", "cache?", "datastore"],
    &["edge?", "compute", "cache"],
    &["edge?", "compute"],
];
const WRITE_ROUTES: [&[&str]; 2] = [
    &["edge?", "compute", "datastore:write"],
    &["edge?", "compute"],
];

/// Reads favour popular items (Zipf 1.0 over a million keys); writes touch any item
/// (uniform), so a cached Sandbox design shows some stale reads without every hot key being
/// rewritten every second.
fn request_type(id: &str, share: f64, route: &[&str]) -> serde_json::Value {
    let mut t = serde_json::json!({
        "id": id,
        "share": share,
        "route": route,
        "key_space": 1_000_000,
    });
    if id == "read" {
        t["key_skew"] = serde_json::json!({ "zipf": 1.0 });
    }
    t
}

fn scenario(types: Vec<serde_json::Value>, preset: SandboxPreset) -> Scenario {
    let traffic = match preset {
        SandboxPreset::Normal | SandboxPreset::KillNode => serde_json::json!({ "flat": 1.0 }),
        SandboxPreset::Spike => {
            serde_json::json!({ "spike": { "at": 20.0, "x": 4.0, "for": 15.0 } })
        }
    };
    serde_json::from_value(serde_json::json!({
        "peak_rps": 0.0,
        "duration_s": SANDBOX_DURATION_S,
        "request_types": types,
        "traffic": traffic,
    }))
    .expect("the Sandbox scenario is well formed")
}

/// The Sandbox scenario for `design`: 90% reads and 10% writes, each on the most complete
/// route the design supports. Fails with the read type's `NoPath` when not even a client →
/// service path exists.
pub fn sandbox_scenario(
    design: &Design,
    registry: &Registry,
    preset: SandboxPreset,
) -> Result<Scenario, SimError> {
    let opts = SimOptions::default();
    let fits = |id: &str, route: &[&str]| {
        let s = scenario(vec![request_type(id, 1.0, route)], preset);
        compile(design, registry, &s, &opts).map(|_| ())
    };
    let mut first_err = None;
    let read = READ_ROUTES.iter().find(|r| match fits("read", r) {
        Ok(()) => true,
        Err(e) => {
            first_err.get_or_insert(e);
            false
        }
    });
    let Some(read) = read else {
        return Err(first_err.expect("at least one route was tried"));
    };
    let write = WRITE_ROUTES
        .iter()
        .find(|r| fits("write", r).is_ok())
        .copied()
        .unwrap_or(read);
    let mut s = scenario(
        vec![
            request_type("read", 0.9, read),
            request_type("write", 0.1, write),
        ],
        preset,
    );
    if preset == SandboxPreset::KillNode
        && let Some(id) = busiest(design, registry, &s)
    {
        s.chaos.push(ka_core::scenario::ChaosEvent::Kill {
            target: ka_core::scenario::Selector {
                id: Some(id),
                ..Default::default()
            },
            at: KILL_AT_S,
            for_s: None,
        });
    }
    Ok(s)
}

/// The card the Kill a node preset takes down: the busiest data store over a short normal
/// run (deterministic), else the busiest service.
fn busiest(design: &Design, registry: &Registry, s: &Scenario) -> Option<String> {
    let mut probe = s.clone();
    probe.duration_s = 10.0;
    probe.chaos.clear();
    let r = ka_sim::simulate(design, registry, &probe, 1, &SimOptions::default()).ok()?;
    let pick = |pred: &dyn Fn(ka_core::ComponentKind) -> bool| {
        r.nodes
            .iter()
            .filter(|n| pred(n.kind))
            .max_by(|a, b| {
                a.utilisation
                    .total_cmp(&b.utilisation)
                    .then(b.id.cmp(&a.id))
            })
            .map(|n| n.id.clone())
    };
    use ka_core::ComponentKind as K;
    pick(&|k| matches!(k, K::Sql | K::Nosql | K::Cache | K::Object | K::Search))
        .or_else(|| pick(&|k| matches!(k, K::App | K::Micro | K::Serverless | K::Worker)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ka_core::{ComponentKind as K, Edge, Flavour, Node, Point, Protocol, registry};

    fn node(id: &str, kind: K, flavour: Flavour) -> Node {
        Node {
            id: id.into(),
            kind,
            flavour,
            label: String::new(),
            position: Point { x: 0.0, y: 0.0 },
            props: Default::default(),
            group_id: None,
        }
    }

    fn edge(id: &str, from: &str, to: &str, protocol: Protocol) -> Edge {
        Edge {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            protocol,
        }
    }

    fn design(nodes: Vec<Node>, edges: Vec<Edge>) -> Design {
        let mut d = Design::empty("sandbox", "Sandbox");
        d.nodes = nodes;
        d.edges = edges;
        d
    }

    fn routes(s: &Scenario) -> Vec<Vec<String>> {
        s.request_types.iter().map(|t| t.route.clone()).collect()
    }

    #[test]
    fn a_full_stack_gets_cached_reads_and_primary_writes() {
        let d = design(
            vec![
                node("c", K::Client, Flavour::Web),
                node("a", K::App, Flavour::Go),
                node("r", K::Cache, Flavour::Redis),
                node("db", K::Sql, Flavour::Postgresql),
            ],
            vec![
                edge("e1", "c", "a", Protocol::Http),
                edge("e2", "a", "r", Protocol::Query),
                edge("e3", "a", "db", Protocol::Query),
            ],
        );
        let s = sandbox_scenario(&d, &registry(), SandboxPreset::Normal).unwrap();
        assert_eq!(
            routes(&s),
            vec![
                vec!["edge?", "compute", "cache?", "datastore"],
                vec!["edge?", "compute", "datastore:write"],
            ]
        );
        assert_eq!(s.duration_s, SANDBOX_DURATION_S);
    }

    #[test]
    fn a_lone_app_server_still_runs() {
        let d = design(
            vec![
                node("c", K::Client, Flavour::Web),
                node("a", K::App, Flavour::Go),
            ],
            vec![edge("e1", "c", "a", Protocol::Http)],
        );
        let s = sandbox_scenario(&d, &registry(), SandboxPreset::Spike).unwrap();
        assert_eq!(routes(&s), vec![vec!["edge?", "compute"]; 2]);
        assert!(s.traffic.peak() > 3.9);
    }

    #[test]
    fn kill_a_node_takes_down_the_busiest_data_store_at_20_s() {
        let d = design(
            vec![
                node("c", K::Client, Flavour::Web),
                node("a", K::App, Flavour::Go),
                node("r", K::Cache, Flavour::Redis),
                node("db", K::Sql, Flavour::Postgresql),
            ],
            vec![
                edge("e1", "c", "a", Protocol::Http),
                edge("e2", "a", "r", Protocol::Query),
                edge("e3", "a", "db", Protocol::Query),
            ],
        );
        let s = sandbox_scenario(&d, &registry(), SandboxPreset::KillNode).unwrap();
        assert_eq!(s.chaos.len(), 1);
        match &s.chaos[0] {
            ka_core::scenario::ChaosEvent::Kill { target, at, for_s } => {
                assert_eq!(*at, KILL_AT_S);
                assert!(for_s.is_none());
                // By utilisation: a 0.01 ms cache barely works; the database is busier.
                assert_eq!(target.id.as_deref(), Some("db"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn no_path_from_a_client_is_an_error_naming_the_reads() {
        let d = design(vec![node("a", K::App, Flavour::Go)], vec![]);
        let err = sandbox_scenario(&d, &registry(), SandboxPreset::Normal).unwrap_err();
        assert!(
            matches!(err, SimError::NoPath { ref request_type, .. } if request_type == "read"),
            "{err:?}"
        );
    }
}

//! Hand-built designs and scenarios shared by the simulator's tests.
#![allow(dead_code)]

use std::collections::BTreeMap;

use ka_core::scenario::{Curve, KeySkew, Measure, RequestType, Scenario, Source};
use ka_core::{ComponentKind, Design, Edge, Flavour, Node, Point, PropValue, Protocol};

pub fn node(id: &str, kind: ComponentKind, flavour: Flavour, props: &[(&str, f64)]) -> Node {
    Node {
        id: id.into(),
        kind,
        flavour,
        label: String::new(),
        position: Point { x: 0.0, y: 0.0 },
        props: props
            .iter()
            .map(|(k, v)| ((*k).to_owned(), PropValue::Number(*v)))
            .collect::<BTreeMap<_, _>>(),
        group_id: None,
    }
}

pub fn edge(from: &str, to: &str, protocol: Protocol) -> Edge {
    Edge {
        id: format!("{from}-{to}-{protocol:?}"),
        from: from.into(),
        to: to.into(),
        protocol,
    }
}

pub fn design(nodes: Vec<Node>, edges: Vec<Edge>) -> Design {
    let mut d = Design::empty("golden", "Golden");
    d.nodes = nodes;
    d.edges = edges;
    d
}

pub fn rtype(id: &str, share: f64, route: &[&str]) -> RequestType {
    RequestType {
        id: id.into(),
        name: None,
        name_es: None,
        share,
        route: route.iter().map(|s| (*s).to_owned()).collect(),
        key_space: 1_000_000,
        key_skew: None,
        cacheable: false,
        payload_kb: 1.0,
        value_kb: 1.0,
        api_keys: None,
        abusive: false,
        fanout: None,
        measure: Measure::Response,
        source: Source::Client,
        contended: None,
        kind: Default::default(),
        connected: 0.0,
        ramp_s: 0.0,
        push_rps: 0.0,
    }
}

pub fn scenario(peak_rps: f64, duration_s: f64, types: Vec<RequestType>) -> Scenario {
    Scenario {
        peak_rps,
        duration_s,
        request_types: types,
        externals: Vec::new(),
        traffic: Curve::Flat(1.0),
        chaos: Vec::new(),
        type_multipliers: BTreeMap::new(),
        key_skew_override: None,
        cold_caches: false,
    }
}

/// (a) client → app (Go ×2) → PostgreSQL.
pub fn golden_a() -> (Design, Scenario) {
    use ComponentKind as K;
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
        ],
    );
    let s = scenario(
        1000.0,
        30.0,
        vec![
            rtype("read", 0.9, &["compute", "datastore"]),
            rtype("write", 0.1, &["compute", "datastore:write"]),
        ],
    );
    (d, s)
}

/// (b) client → NGINX → 2 apps → Redis + PostgreSQL primary/replica (URL-shortener-like).
pub fn golden_b(peak_rps: f64) -> (Design, Scenario) {
    use ComponentKind as K;
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("lb", K::Lb, Flavour::Nginx, &[]),
            node("app-1", K::App, Flavour::Go, &[("instances", 3.0)]),
            node("app-2", K::App, Flavour::Go, &[("instances", 3.0)]),
            node("cache", K::Cache, Flavour::Redis, &[("memory_gb", 2.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
            node("db-replica", K::Sql, Flavour::Postgresql, &[]),
        ],
        vec![
            edge("client", "lb", Protocol::Http),
            edge("lb", "app-1", Protocol::Http),
            edge("lb", "app-2", Protocol::Http),
            edge("app-1", "cache", Protocol::Query),
            edge("app-2", "cache", Protocol::Query),
            edge("app-1", "db", Protocol::Query),
            edge("app-2", "db", Protocol::Query),
            edge("db", "db-replica", Protocol::Replicate),
        ],
    );
    let mut redirect = rtype(
        "redirect",
        0.95,
        &["edge?", "compute", "cache?", "datastore"],
    );
    redirect.key_space = 10_000_000;
    redirect.key_skew = Some(KeySkew { zipf: 1.1 });
    let s = scenario(
        peak_rps,
        30.0,
        vec![
            redirect,
            rtype("create", 0.05, &["edge?", "compute", "datastore:write"]),
        ],
    );
    (d, s)
}

/// (c) client → app → Kafka (16 partitions) → 2 Celery workers → DynamoDB, measured at completion.
pub fn golden_c() -> (Design, Scenario) {
    use ComponentKind as K;
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("stream", K::Stream, Flavour::Kafka, &[("partitions", 16.0)]),
            node(
                "worker-1",
                K::Worker,
                Flavour::Celery,
                &[("service_ms", 20.0)],
            ),
            node(
                "worker-2",
                K::Worker,
                Flavour::Celery,
                &[("service_ms", 20.0)],
            ),
            node("store", K::Nosql, Flavour::Dynamodb, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "stream", Protocol::Publish),
            edge("stream", "worker-1", Protocol::Consume),
            edge("stream", "worker-2", Protocol::Consume),
            edge("worker-1", "store", Protocol::Query),
            edge("worker-2", "store", Protocol::Query),
        ],
    );
    let mut event = rtype(
        "event",
        1.0,
        &["compute", "broker", "worker", "datastore:write"],
    );
    event.measure = Measure::Completion;
    let s = scenario(400.0, 30.0, vec![event]);
    (d, s)
}

/// (d) Change data capture: client → app → PostgreSQL --CDC--> Kafka → indexer →
/// Elasticsearch, with the stream down from t=10 s for 8 s (changes wait, none lost).
pub fn golden_d() -> (Design, Scenario) {
    use ComponentKind as K;
    let d = design(
        vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("app", K::App, Flavour::Go, &[("instances", 2.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
            node("cdc", K::Stream, Flavour::Kafka, &[]),
            node(
                "indexer",
                K::Worker,
                Flavour::Celery,
                &[("service_ms", 5.0)],
            ),
            node("search", K::Search, Flavour::Elasticsearch, &[]),
        ],
        vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
            edge("db", "cdc", Protocol::Publish),
            edge("cdc", "indexer", Protocol::Consume),
            edge("indexer", "search", Protocol::Query),
        ],
    );
    let mut update = rtype(
        "update",
        1.0,
        &[
            "compute",
            "datastore:write:sql",
            "cdc",
            "worker",
            "datastore:write:search",
        ],
    );
    update.measure = Measure::Completion;
    let mut s = scenario(300.0, 30.0, vec![update]);
    s.chaos.push(ka_core::scenario::ChaosEvent::Kill {
        target: ka_core::scenario::Selector {
            id: Some("cdc".into()),
            ..Default::default()
        },
        at: 10.0,
        for_s: Some(8.0),
    });
    (d, s)
}

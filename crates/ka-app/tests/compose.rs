//! docker-compose import (M7b.1, review A6/B4): mapping, edges, layout and every cap.

use std::path::PathBuf;

use ka_app::compose::{self, SkipReason};
use ka_app::error::AppErrorCode;
use ka_core::{ComponentKind as K, Flavour as F, PropValue, Protocol, WiringReason, registry};

fn fixture(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/compose")
        .join(name);
    std::fs::read_to_string(p).unwrap().replace("\r\n", "\n")
}

fn code(text: &str) -> AppErrorCode {
    compose::import(text, "x", &registry())
        .expect_err("the import is refused")
        .code
}

#[test]
fn every_image_maps_to_its_card() {
    let r = compose::import(&fixture("shop.yml"), "shop", &registry()).unwrap();
    let card = |s: &str| {
        let n = r.design.nodes.iter().find(|n| n.label == s).unwrap();
        (n.kind, n.flavour)
    };
    assert_eq!(card("web"), (K::Lb, F::Nginx));
    assert_eq!(card("lb"), (K::Lb, F::Haproxy));
    assert_eq!(card("db"), (K::Sql, F::Postgresql));
    assert_eq!(card("replica"), (K::Sql, F::Mysql));
    assert_eq!(card("cache"), (K::Cache, F::Valkey));
    assert_eq!(card("sessions"), (K::Cache, F::Memcached));
    assert_eq!(card("events"), (K::Stream, F::Kafka));
    assert_eq!(card("stream2"), (K::Stream, F::Redpanda));
    assert_eq!(card("queue"), (K::Queue, F::Rabbitmq));
    assert_eq!(card("search"), (K::Search, F::Elasticsearch));
    assert_eq!(card("search2"), (K::Search, F::Opensearch));
    assert_eq!(card("docs"), (K::Nosql, F::Mongodb));
    assert_eq!(card("wide"), (K::Nosql, F::Scylladb));
    assert_eq!(card("ledger"), (K::Nosql, F::Cassandra));
    // A build-only service (through a merge key) and an unknown image are guessed.
    let guessed: Vec<&str> = r.guessed.iter().map(|g| g.service.as_str()).collect();
    assert_eq!(guessed, ["api", "worker"]);
    assert_eq!(card("api"), (K::App, F::Go));
    assert_eq!(r.mapped.len() + r.guessed.len(), r.design.nodes.len());
}

#[test]
fn replicas_become_instances_and_are_clamped() {
    let r = compose::import(&fixture("shop.yml"), "shop", &registry()).unwrap();
    let instances = |s: &str| {
        r.design
            .nodes
            .iter()
            .find(|n| n.label == s)
            .unwrap()
            .props
            .get("instances")
            .cloned()
    };
    assert_eq!(instances("api"), Some(PropValue::Number(3.0)));
    assert_eq!(instances("worker"), Some(PropValue::Number(64.0)));
    assert_eq!(r.clamped, ["worker"]);
    assert_eq!(instances("db"), None, "cards keep sparse props");
}

#[test]
fn dependencies_become_allowed_connections_or_are_listed() {
    let r = compose::import(&fixture("shop.yml"), "shop", &registry()).unwrap();
    let edge = |a: &str, b: &str| {
        r.design
            .edges
            .iter()
            .find(|e| e.from == a && e.to == b)
            .map(|e| e.protocol)
    };
    // depends_on as a map, as a list through a merge key, and links (with an alias).
    assert_eq!(edge("web", "api"), Some(Protocol::Http));
    assert_eq!(edge("api", "db"), Some(Protocol::Query));
    assert_eq!(edge("api", "cache"), Some(Protocol::Query));
    assert_eq!(edge("worker", "db"), Some(Protocol::Query));
    assert_eq!(edge("worker", "queue"), Some(Protocol::Publish));
    // A database doesn't call a cache, and a missing service can't be drawn.
    assert_eq!(edge("db", "cache"), None);
    assert!(r.skipped_edges.iter().any(|s| s.from == "db"
        && s.to == "cache"
        && s.reason == SkipReason::NotAllowed(WiringReason::DataCallsOut)));
    assert!(
        r.skipped_edges.iter().any(|s| s.from == "ledger"
            && s.to == "ghost"
            && s.reason == SkipReason::UnknownService)
    );
    assert!(ka_core::validate(&r.design, &registry()).is_empty());
}

#[test]
fn the_layout_is_deterministic() {
    let text = fixture("shop.yml");
    let a = compose::import(&text, "shop", &registry()).unwrap();
    let b = compose::import(&text, "shop", &registry()).unwrap();
    assert_eq!(a, b);
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/compose/shop.snapshot.json");
    let got = serde_json::to_string_pretty(&a).unwrap() + "\n";
    if std::env::var_os("KA_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &got).unwrap();
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        got == want,
        "compose snapshot changed; review, then run with KA_UPDATE_SNAPSHOTS=1"
    );
    // Columns: edge, compute, async, data; rows by name; on the 8 px grid.
    for n in &a.design.nodes {
        assert_eq!(n.position.x % 8.0, 0.0);
        assert_eq!(n.position.y % 8.0, 0.0);
    }
}

#[test]
fn the_canvas_card_size_matches() {
    let store = include_str!("../../../apps/desktop/src/solve/store.ts");
    assert!(store.contains(&format!("CARD_W = {}", compose::CARD_W)));
    assert!(store.contains(&format!("CARD_H = {}", compose::CARD_H)));
}

#[test]
fn caps_refuse_hostile_files() {
    // Over 1 MiB.
    let big = format!(
        "services:\n  app:\n    image: nginx\n# {}\n",
        "x".repeat(compose::MAX_BYTES)
    );
    assert_eq!(code(&big), AppErrorCode::ComposeTooLarge);
    // 201 services.
    let many: String = (0..=compose::MAX_SERVICES)
        .map(|i| format!("  s{i}:\n    image: nginx\n"))
        .collect();
    assert_eq!(
        code(&format!("services:\n{many}")),
        AppErrorCode::ComposeTooManyServices
    );
    // Nesting 65 deep.
    let deep = format!(
        "services:\n  app:\n    x: {}{}\n",
        "[".repeat(65),
        "]".repeat(65)
    );
    assert_eq!(code(&deep), AppErrorCode::ComposeTooComplex);
    // 101 aliases.
    let aliases = format!(
        "x: &a 1\nservices:\n  app:\n    image: nginx\n    l: [{}]\n",
        vec!["*a"; 101].join(",")
    );
    assert_eq!(code(&aliases), AppErrorCode::ComposeTooComplex);
    // Billion laughs: few aliases, 9^9 nodes once expanded.
    assert_eq!(
        code(&fixture("billion-laughs.yml")),
        AppErrorCode::ComposeTooComplex
    );
    // Over 200,000 events.
    let events = format!(
        "services:\n  app:\n    image: nginx\n    l: [{}]\n",
        vec!["1"; 200_001].join(",")
    );
    assert!(events.len() < compose::MAX_BYTES * 2);
    if events.len() <= compose::MAX_BYTES {
        assert_eq!(code(&events), AppErrorCode::ComposeTooComplex);
    }
}

#[test]
fn unusual_yaml_is_refused_plainly() {
    assert_eq!(code("services: [unclosed\n"), AppErrorCode::ComposeInvalid);
    assert_eq!(code("version: '3'\n"), AppErrorCode::ComposeInvalid);
    assert_eq!(code("services: {}\n"), AppErrorCode::ComposeInvalid);
    assert_eq!(code("- just\n- a list\n"), AppErrorCode::ComposeInvalid);
    // Anchors on keys, aliases as keys and structured keys.
    assert_eq!(
        code("services:\n  &k app:\n    image: nginx\n"),
        AppErrorCode::ComposeUnsupported
    );
    assert_eq!(
        code("k: &k app\nservices:\n  *k :\n    image: nginx\n"),
        AppErrorCode::ComposeUnsupported
    );
    assert_eq!(
        code("services:\n  ? [a, b]\n  : {image: nginx}\n"),
        AppErrorCode::ComposeUnsupported
    );
    // Anchors used from an `x-` block are fine (B4).
    let ok = "x-base: &base\n  image: redis\nservices:\n  cache:\n    <<: *base\n";
    let r = compose::import(ok, "x", &registry()).unwrap();
    assert_eq!(r.design.nodes[0].kind, K::Cache);
}

//! `.kbarch` migrations, with a real file of each old version (ADR 0003): v1 (M0) and v2
//! (M1–M3) both open as v3, where cards keep only the props the user set; v3 (M5–M7b) opens as
//! v4, unchanged except its version (M8: zones may carry `region_tag` and `mesh`).

use std::collections::BTreeMap;

use ka_app::kbarch::{PLACEHOLDERS_V1, parse, to_bytes};
use ka_core::{Design, PropValue, SCHEMA_VERSION, registry};

const V1: &str = include_str!("fixtures/v1.kbarch");
const V2: &str = include_str!("fixtures/v2.kbarch");
const V3: &str = include_str!("fixtures/v3.kbarch");

fn open(text: &str) -> Design {
    parse(text.as_bytes(), &registry()).expect("the old file opens")
}

fn props<'a>(d: &'a Design, id: &str) -> &'a BTreeMap<String, PropValue> {
    &d.nodes.iter().find(|n| n.id == id).expect("card").props
}

fn num(v: f64) -> PropValue {
    PropValue::Number(v)
}

#[test]
fn a_v2_file_keeps_only_what_the_user_set() {
    let d = open(V2);
    assert_eq!(d.schema_version, SCHEMA_VERSION);
    // Defaults gone; the user's own values stay.
    assert!(props(&d, "client").is_empty(), "rps 1000 is the default");
    assert_eq!(props(&d, "api").get("instances"), Some(&num(4.0)));
    assert_eq!(props(&d, "cache").get("memory_gb"), Some(&num(8.0)));
    assert_eq!(
        props(&d, "jobs").get("visibility_timeout_s"),
        Some(&num(60.0))
    );
    assert_eq!(props(&d, "jobs").len(), 1);
    assert_eq!(props(&d, "db").len(), 0);
    // The autoscaling policy is thinned the same way.
    let scaling = d.groups[0].scaling.as_ref().unwrap();
    assert_eq!(scaling.props.len(), 1);
    assert_eq!(scaling.props.get("max_instances"), Some(&num(12.0)));
}

#[test]
fn ip_hash_is_renamed_key_hash() {
    let d = open(V2);
    assert_eq!(
        props(&d, "lb").get("algorithm"),
        Some(&PropValue::Text("key_hash".into()))
    );
}

/// One check per placeholder kind: the M0 value is dropped, so the card picks up the
/// calibrated default.
#[test]
fn each_m0_placeholder_is_dropped() {
    let d = open(V2);
    assert!(
        !props(&d, "cache").contains_key("service_ms"),
        "cache 0.5 ms"
    );
    assert!(!props(&d, "api").contains_key("concurrency"), "django 64");
    assert!(!props(&d, "db").contains_key("concurrency"), "mysql 100");
    assert!(
        !props(&d, "worker").contains_key("concurrency"),
        "sidekiq 8"
    );
    assert!(
        !props(&d, "thumbs").contains_key("concurrency_limit"),
        "cloud run 1000"
    );
    let v1 = open(V1);
    assert!(
        !props(&v1, "cache").contains_key("service_ms"),
        "memcached 0.5 ms"
    );
    // Every placeholder in the table is covered by the fixtures above.
    assert_eq!(PLACEHOLDERS_V1.len(), 5);
}

#[test]
fn a_placeholder_only_counts_on_its_own_flavour() {
    // 64 threads is the M0 default for Django only; on Node.js it is today's default anyway,
    // and 100 connections is a placeholder only on MySQL.
    let text = V2.replace(r#""flavour": "mysql""#, r#""flavour": "postgresql""#);
    let d = open(&text);
    assert!(
        props(&d, "db").is_empty(),
        "100 is PostgreSQL's real default"
    );
    let text = V2.replace(
        r#""concurrency": 100, "service_ms": 5"#,
        r#""concurrency": 120, "service_ms": 5"#,
    );
    let d = open(&text);
    assert_eq!(props(&d, "db").get("concurrency"), Some(&num(120.0)));
}

#[test]
fn a_v1_file_goes_all_the_way_to_v4() {
    let d = open(V1);
    assert_eq!(d.schema_version, SCHEMA_VERSION);
    assert_eq!(props(&d, "client").get("rps"), Some(&num(2500.0)));
    assert!(props(&d, "app").is_empty());
    assert_eq!(props(&d, "db").get("shards"), Some(&num(2.0)));
    assert_eq!(props(&d, "db").len(), 1, "read_replicas gone in v2");
    assert!(props(&d, "edge").is_empty(), "hit_rate gone, ttl default");
    let w = d
        .groups
        .iter()
        .find(|g| g.id == "scaler")
        .expect("the wrapper");
    let scaling = w.scaling.as_ref().unwrap();
    assert_eq!(scaling.props.get("target_util"), Some(&num(70.0)));
    assert_eq!(scaling.props.len(), 1);
}

#[test]
fn migrated_files_save_as_v4_and_read_back_the_same() {
    for text in [V1, V2, V3] {
        let d = open(text);
        let bytes = to_bytes(&d, &registry()).unwrap();
        let again = parse(&bytes, &registry()).unwrap();
        assert_eq!(again, d);
        assert!(
            String::from_utf8(bytes)
                .unwrap()
                .contains("\"schema_version\": 4")
        );
    }
}

/// v3 → v4 changes nothing but the version: the same cards, edges and zones, and no zone
/// gains a tag or a mesh.
#[test]
fn a_v3_file_opens_as_v4_unchanged() {
    let d = open(V3);
    assert_eq!(d.schema_version, 4);
    let raw: serde_json::Value = serde_json::from_str(V3).unwrap();
    let mut expected: Design = serde_json::from_value({
        let mut v = raw;
        v["schema_version"] = serde_json::Value::from(4);
        v
    })
    .unwrap();
    expected.schema_version = 4;
    assert_eq!(d, expected);
    assert!(
        d.groups
            .iter()
            .all(|g| g.region_tag.is_none() && g.mesh.is_none())
    );
}

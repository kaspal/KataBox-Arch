//! The Component Guide: the shipped entries validate, and the "not simulated" list is true.

use ka_core::registry;
use ka_pack::guide::NOT_SIMULATED;
use ka_pack::{load_guide, load_problems, validate_guide};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_simulator_really_ignores_the_not_simulated_props() {
    let mut src = String::new();
    for e in std::fs::read_dir(root().join("crates/ka-sim/src")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "rs") {
            src.push_str(&std::fs::read_to_string(p).unwrap());
        }
    }
    for key in NOT_SIMULATED {
        assert!(
            !src.contains(&format!("\"{key}\"")),
            "ka-sim now reads `{key}`: take it off NOT_SIMULATED so the Guide can describe it"
        );
    }
}

#[test]
fn every_kind_has_a_valid_entry() {
    let r = registry();
    let content = root().join("content");
    let problems = load_problems(&content, &r).unwrap();
    let guide = load_guide(&content).unwrap();
    assert_eq!(validate_guide(&guide, &problems, &r), vec![]);
    assert_eq!(guide.len(), r.kinds.len());
}

/// H7: a new problem may not let a compute card answer a file request in place of storage.
#[test]
fn object_routes_answered_by_compute_alone_are_refused() {
    let r = registry();
    let problems = load_problems(&root().join("content"), &r).unwrap();
    assert!(
        ka_pack::validate_problems(&problems, &r).is_empty(),
        "the shipped problems (three grandfathered) pass"
    );
    let mut p = problems
        .iter()
        .find(|p| p.def.id == "weather-proxy")
        .unwrap()
        .clone();
    p.def.request_types[0].route = vec!["edge?".into(), "compute|datastore:object".into()];
    let issues = ka_pack::validate_problems(&[p], &r);
    assert!(
        issues
            .iter()
            .any(|i| i.to_string().contains("compute|datastore:object")),
        "{issues:?}"
    );
}

//! The framing fixture for the UI's player test: every sample and concept-card step, with its
//! focus, camera and card positions (`apps/desktop/src/test/fixtures/stories.json`). The UI
//! test checks that each step's view shows all its focused cards (M1–M3 review F2). This test
//! fails when the fixture drifts from the content; `KA_UPDATE_SNAPSHOTS=1` rewrites it.

use std::path::Path;

use ka_core::registry;
use serde_json::{Value, json};

fn stories() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let r = registry();
    let mut out = Vec::new();
    for (kind, list) in [
        (
            "sample",
            ka_pack::load_samples(&root, &r).expect("samples load"),
        ),
        (
            "concept",
            ka_pack::load_concepts(&root, &r).expect("concepts load"),
        ),
    ] {
        for s in list {
            let steps: Vec<Value> = s
                .def
                .steps
                .iter()
                .map(|step| {
                    let d = &s.snapshots[&step.snapshot];
                    json!({
                        "focus": step.focus,
                        "camera": step.camera,
                        "nodes": d.nodes.iter().map(|n| json!({
                            "id": n.id, "x": n.position.x, "y": n.position.y,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            out.push(json!({ "kind": kind, "id": s.def.id, "steps": steps }));
        }
    }
    Value::from(out)
}

#[test]
fn the_story_framing_fixture_matches_the_content() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src/test/fixtures/stories.json");
    let want = serde_json::to_string_pretty(&stories()).unwrap() + "\n";
    if std::env::var_os("KA_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &want).unwrap();
        return;
    }
    let got = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        got == want,
        "stories.json is out of date: run `KA_UPDATE_SNAPSHOTS=1 cargo test -p ka-pack --test stories`"
    );
}

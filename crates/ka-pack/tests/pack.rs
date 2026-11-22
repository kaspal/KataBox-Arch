//! ka-pack: loading, structural validation, building content.db.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ka_core::{
    ComponentKind as K, Design, Edge, Flavour, Node, Point, PropValue, Protocol, registry,
};
use ka_pack::problems::parse_hints;
use ka_pack::{build_content, load_problems, scaffold, validate_problems, write_db};

fn temp(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ka-pack-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

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

fn design(id: &str, app_instances: f64) -> Design {
    let mut d = Design::empty(id, id);
    d.nodes = vec![
        node("client", K::Client, Flavour::Web, &[]),
        node("app", K::App, Flavour::Go, &[("instances", app_instances)]),
        node("db", K::Sql, Flavour::Postgresql, &[]),
    ];
    let e = |id: &str, from: &str, to: &str, protocol| Edge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        protocol,
    };
    d.edges = vec![
        e("e1", "client", "app", Protocol::Http),
        e("e2", "app", "db", Protocol::Query),
    ];
    d
}

const TOML: &str = r#"
id = "demo"
number = 1
set = 1
difficulty = "easy"
title = "Demo"
title_es = "Demostración"
teaches = "Scaling out"
teaches_es = "Escalar"

[nfr]
peak_rps = 2000
p95_ms = 100
availability = 0.999
budget_usd = 1000

[[request_types]]
id = "read"
share = 1.0
route = ["compute", "datastore"]

[[tests]]
id = "normal"
name = "Normal day"
name_es = "Un día normal"
duration_s = 10

[[tests]]
id = "spike"
name = "Spike"
name_es = "Pico"
hidden = true
duration_s = 10
traffic = { spike = { at = 2, x = 4, for = 5 } }

[pass]
error_rate = 0.001
"#;

const HINTS: &str =
    "## 1\n\nLook at the app tier.\n\n## 2\n\nMore instances.\n\n## 3\n\nFour of them.\n";

fn write_problem(root: &Path, id: &str, toml: &str) -> PathBuf {
    let dir = root.join("problems").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let r = registry();
    let files: [(&str, Vec<u8>); 9] = [
        ("problem.toml", toml.as_bytes().to_vec()),
        (
            "statement.md",
            b"# Demo\n\n## Goal\n\nServe reads.\n".to_vec(),
        ),
        (
            "statement.es.md",
            "# Demo\n\n## Objetivo\n\nServir lecturas.\n"
                .as_bytes()
                .to_vec(),
        ),
        ("hints.md", HINTS.as_bytes().to_vec()),
        ("hints.es.md", HINTS.as_bytes().to_vec()),
        (
            "editorial.md",
            b"# Editorial\n\n## Why\n\nBecause.\n".to_vec(),
        ),
        (
            "editorial.es.md",
            "# Editorial\n\n## Por qué\n\nPorque.\n".as_bytes().to_vec(),
        ),
        (
            "reference.kbarch",
            ka_app::kbarch::to_bytes(&design("ref", 4.0), &r).unwrap(),
        ),
        (
            "naive.kbarch",
            ka_app::kbarch::to_bytes(&design("naive", 1.0), &r).unwrap(),
        ),
    ];
    for (n, b) in files {
        std::fs::write(dir.join(n), b).unwrap();
    }
    dir
}

#[test]
fn a_valid_problem_loads_validates_and_builds() {
    let root = temp("valid");
    write_problem(&root, "demo", TOML);
    let r = registry();
    let problems = load_problems(&root, &r).unwrap();
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].hints_en.len(), 3);
    assert_eq!(validate_problems(&problems, &r), vec![]);
    let data = build_content(&root, &r).unwrap();
    assert_eq!(data.problems.len(), 1);
    let row = &data.problems[0];
    assert_eq!(row.difficulty, "easy");
    assert!(row.ref_cost > 0.0);
    assert!(row.ref_p95_ms > 0.0);
    assert!(
        data.meta
            .iter()
            .any(|(k, v)| k == "content_hash" && v.len() == 64)
    );
    let out = root.join("out").join("content.db");
    write_db(&out, &data).unwrap();
    let db = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(async {
            let db = ka_store::ContentDb::open(&out).await.unwrap();
            let p = db.problem("demo").await.unwrap().unwrap();
            db.close().await;
            p
        });
    assert_eq!(&db, row);
}

#[test]
fn the_content_hash_changes_with_any_file() {
    let root = temp("hash");
    write_problem(&root, "demo", TOML);
    let a = ka_pack::content_hash(&root).unwrap();
    assert_eq!(a, ka_pack::content_hash(&root).unwrap());
    std::fs::write(
        root.join("problems/demo/hints.es.md"),
        HINTS.replace("Four", "Cuatro"),
    )
    .unwrap();
    assert_ne!(a, ka_pack::content_hash(&root).unwrap());
}

#[test]
fn structural_problems_are_reported_with_their_file() {
    let root = temp("broken");
    let bad = TOML
        .replace("id = \"demo\"", "id = \"other\"")
        .replace("share = 1.0", "share = 0.5")
        .replace("hidden = true\n", "")
        .replace(
            "route = [\"compute\", \"datastore\"]",
            "route = [\"compute\", \"external:mail\"]",
        );
    write_problem(&root, "demo", &bad);
    std::fs::write(root.join("problems/demo/editorial.es.md"), "# Editorial\n").unwrap();
    let r = registry();
    let problems = load_problems(&root, &r).unwrap();
    let issues = validate_problems(&problems, &r);
    let text: Vec<String> = issues.iter().map(|i| i.to_string()).collect();
    let has = |s: &str| text.iter().any(|t| t.contains(s));
    assert!(has("differs from the folder"), "{text:#?}");
    assert!(has("add up to 0.5"), "{text:#?}");
    assert!(has("needs at least one hidden test"), "{text:#?}");
    assert!(has("external \"mail\""), "{text:#?}");
    assert!(has("editorial.es.md: 0 sections"), "{text:#?}");
    assert!(build_content(&root, &r).is_err());
}

#[test]
fn a_missing_file_or_bad_hints_fail_loading() {
    let root = temp("missing");
    let dir = write_problem(&root, "demo", TOML);
    std::fs::remove_file(dir.join("naive.kbarch")).unwrap();
    std::fs::write(dir.join("hints.es.md"), "## 1\n\nUna.\n\n## 2\n\nDos.\n").unwrap();
    let issues = load_problems(&root, &registry()).unwrap_err();
    let text: Vec<String> = issues.iter().map(|i| i.to_string()).collect();
    assert!(
        text.iter().any(|t| t.contains("naive.kbarch: missing")),
        "{text:#?}"
    );
}

#[test]
fn a_scaffolded_problem_is_refused_until_its_todos_are_filled() {
    let root = temp("scaffold");
    let dir = scaffold::new_problem(&root, "my-problem").unwrap();
    assert!(dir.join("hints.es.md").exists());
    assert!(
        scaffold::new_problem(&root, "my-problem").is_err(),
        "exists"
    );
    assert!(scaffold::new_problem(&root, "Bad Id").is_err());
    let r = registry();
    let problems = load_problems(&root, &r).unwrap();
    let issues = validate_problems(&problems, &r);
    assert!(
        issues.iter().any(|i| i.message.contains("TODO(ka-pack)")),
        "{issues:#?}"
    );
}

#[test]
fn hints_need_three_tiers() {
    assert_eq!(parse_hints(HINTS).unwrap().len(), 3);
    assert!(parse_hints("## 1\na\n## 3\nc\n## 2\nb\n").is_err());
    assert!(parse_hints("intro\n## 1\na\n## 2\nb\n## 3\nc\n").is_err());
    assert!(parse_hints("## 1\na\n## 2\n\n## 3\nc\n").is_err());
}

#[test]
fn run_checks_judge_the_reference_and_the_naive_design() {
    let root = temp("run");
    write_problem(&root, "demo", TOML);
    let r = registry();
    let problems = load_problems(&root, &r).unwrap();
    // The reference (4 app instances) passes; the naive one (1 instance) fails the hidden
    // spike.
    let (issues, reference, naive) = ka_pack::run::check_problem(&problems[0], &r);
    assert!(issues.is_empty(), "{issues:#?}");
    assert_eq!(reference.verdict, ka_judge::Verdict::Accepted);
    assert_ne!(naive.verdict, ka_judge::Verdict::Accepted);

    // A naive design that passes everything is refused.
    let same = std::fs::read(root.join("problems/demo/reference.kbarch")).unwrap();
    std::fs::write(root.join("problems/demo/naive.kbarch"), same).unwrap();
    let problems = load_problems(&root, &r).unwrap();
    let issues = ka_pack::run::run_checks(&problems, &r, false);
    assert!(
        issues
            .iter()
            .any(|i| i.message.contains("must fail at least one hidden test")),
        "{issues:#?}"
    );
}

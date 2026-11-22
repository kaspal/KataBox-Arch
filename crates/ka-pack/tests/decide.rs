//! Decision guides and the glossary: structural mistakes are reported.

use ka_core::decide::{DecideFlow, GlossaryTerm};
use ka_pack::decide::{FlowFile, validate_decide, validate_glossary};

const FLOW: &str = r#"
id = "sql-or-nosql"
number = 1
title = "SQL or NoSQL?"
title_es = "¿SQL o NoSQL?"
intro = "i"
intro_es = "i"
start = "joins"

[[question]]
id = "joins"
text = "Do you need joins?"
text_es = "¿Necesitas joins?"
options = [
  { label = "Yes", label_es = "Sí", result = "sql", why = "w", why_es = "w" },
  { label = "No", label_es = "No", next = "scale", why = "w", why_es = "w" },
]

[[question]]
id = "scale"
text = "Huge scale?"
text_es = "¿Escala enorme?"
options = [
  { label = "Yes", label_es = "Sí", result = "nosql", why = "w", why_es = "w" },
  { label = "No", label_es = "No", result = "sql", why = "w", why_es = "w" },
]
"#;

fn file(flow: DecideFlow) -> FlowFile {
    FlowFile {
        path: format!("decide/{}.toml", flow.id),
        flow,
        toml: String::new(),
    }
}

fn messages(f: DecideFlow) -> Vec<String> {
    validate_decide(&[file(f)])
        .into_iter()
        .map(|i| i.message)
        .collect()
}

#[test]
fn a_good_flow_passes_and_mistakes_are_named() {
    let good: DecideFlow = toml::from_str(FLOW).unwrap();
    assert_eq!(messages(good.clone()), Vec::<String>::new());

    let mut looped = good.clone();
    looped.questions[1].options[1].result = None;
    looped.questions[1].options[1].next = Some("joins".into());
    assert!(messages(looped).iter().any(|m| m.contains("loop")));

    let mut orphan = good.clone();
    orphan.questions[0].options[1].next = None;
    orphan.questions[0].options[1].result = Some(ka_core::ComponentKind::Nosql);
    assert!(
        messages(orphan)
            .iter()
            .any(|m| m.contains("can't be reached"))
    );

    let mut dangling = good.clone();
    dangling.questions[0].options[1].next = Some("nowhere".into());
    assert!(
        messages(dangling)
            .iter()
            .any(|m| m.contains("isn't a question"))
    );

    let mut both = good;
    both.questions[0].options[0].next = Some("scale".into());
    assert!(messages(both).iter().any(|m| m.contains("exactly one")));
}

#[test]
fn glossary_terms_must_be_used_in_the_content() {
    let t = |id: &str, term: &str| GlossaryTerm {
        id: id.into(),
        term: term.into(),
        term_es: term.into(),
        definition: "d".into(),
        definition_es: "d".into(),
        see: None,
    };
    let issues = validate_glossary(
        &[t("p95", "p95"), t("zz", "Zebra sharding")],
        "the p95 latency",
    );
    assert_eq!(issues.len(), 1);
    assert!(issues[0].message.contains("Zebra sharding"));
}

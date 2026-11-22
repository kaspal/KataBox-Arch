//! Sample folders: the shipped ones validate, and each structural mistake is reported.

use ka_core::registry;
use ka_core::story::{Callout, CalloutMetric, StepRun};
use ka_pack::{load_problems, load_samples, validate_samples};

fn content() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

#[test]
fn the_shipped_samples_are_valid_and_build_into_rows() {
    let r = registry();
    let problems = load_problems(&content(), &r).unwrap();
    let samples = load_samples(&content(), &r).unwrap();
    assert!(!samples.is_empty());
    assert_eq!(validate_samples(&samples, &problems, &r), vec![]);
    assert_eq!(samples.iter().filter(|s| s.def.tour).count(), 1);
    for s in &samples {
        let row = ka_pack::samples::sample_row(s, &r).unwrap();
        assert_eq!(row.snapshots.len(), s.snapshots.len());
        // The stored definition reads back.
        let back: ka_core::story::SampleDef = serde_json::from_str(&row.def).unwrap();
        assert_eq!(back, s.def);
    }
}

#[test]
fn structural_mistakes_are_reported() {
    let r = registry();
    let problems = load_problems(&content(), &r).unwrap();
    let good = load_samples(&content(), &r).unwrap().remove(0);
    let messages = |s: &ka_pack::SampleDir| -> Vec<String> {
        validate_samples(std::slice::from_ref(s), &problems, &r)
            .into_iter()
            .map(|i| i.message)
            .collect()
    };
    let has = |s: &ka_pack::SampleDir, needle: &str| {
        let m = messages(s);
        assert!(
            m.iter().any(|x| x.contains(needle)),
            "expected `{needle}` in {m:?}"
        );
    };

    let mut s = good.clone();
    s.def.steps[0].focus = vec!["ghost".into()];
    has(&s, "focus `ghost`");

    let mut s = good.clone();
    s.def.steps[0].snapshot = "missing.kbarch".into();
    has(&s, "isn't in the folder");

    let mut s = good.clone();
    s.def.steps[0].run = Some(StepRun {
        test: Some("normal".into()),
        preset: Some("normal".into()),
        start_s: 0.0,
        speed: 1.0,
    });
    has(&s, "exactly one of");

    let mut s = good.clone();
    s.def.steps[0].run = Some(StepRun {
        test: None,
        preset: Some("hurricane".into()),
        start_s: 0.0,
        speed: 1.0,
    });
    has(&s, "unknown preset");

    let mut s = good.clone();
    s.def.steps[0].run = Some(StepRun {
        test: Some("normal".into()),
        preset: None,
        start_s: 0.0,
        speed: 1.0,
    });
    has(&s, "needs the sample's `problem`");

    let mut s = good.clone();
    s.def.steps[3].callout = Some(Callout {
        metric: CalloutMetric::P95Ms,
        before: None,
        after: 1.0,
        target: None,
    });
    has(&s, "a callout needs a run");

    let mut s = good.clone();
    s.def.steps[0].caption_es = " ".into();
    has(&s, "`caption_es` is empty");

    let mut s = good.clone();
    s.def.problem = Some("no-such-problem".into());
    has(&s, "doesn't exist");

    // Two tours, and the same number twice.
    let twin = good.clone();
    let issues = validate_samples(&[good.clone(), twin], &problems, &r);
    assert!(issues.iter().any(|i| i.message.contains("marked `tour`")));
    assert!(issues.iter().any(|i| i.message.contains("used twice")));
}

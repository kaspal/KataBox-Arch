//! Review A1 at every gate: every problem's reference and naive design, and every
//! Find-the-flaw drill's broken and fixed design, judged with sampling (the default) and
//! without, gets the same verdict on every test; where a passing test was sampled, p95 is
//! within 10%, and so is p99 on tests that judge it (review A4). Release only:
//! `cargo test -p ka-pack --release --test accuracy -- --ignored --nocapture`.

use ka_core::Design;
use ka_core::problem::ProblemDef;
use ka_core::registry;
use ka_judge::{Verdict, judge_with};
use ka_pack::load_problems;
use ka_sim::SimOptions;

#[test]
#[ignore = "release gate check over all content"]
fn sampled_and_full_runs_agree_on_every_shipped_design() {
    let r = registry();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let problems = load_problems(&root, &r).expect("content loads");
    let flaws = ka_pack::flaws::load_flaws(&root, &r).expect("flaws load");
    let mut cases: Vec<(String, ProblemDef, &Design)> = Vec::new();
    for p in &problems {
        cases.push((
            format!("{} reference", p.def.id),
            p.def.clone(),
            &p.reference,
        ));
        cases.push((format!("{} naive", p.def.id), p.def.clone(), &p.naive));
    }
    for f in &flaws {
        let def = f.def.problem();
        cases.push((format!("{} broken", def.id), def.clone(), &f.broken));
        cases.push((format!("{} fixed", def.id), def, &f.fixed));
    }
    let full = SimOptions {
        sample_cap: f64::INFINITY,
    };
    let mut worst = 0.0f64;
    let mut compared = 0;
    let mut problems_found: Vec<String> = Vec::new();
    for (label, def, design) in &cases {
        let a = judge_with(def, design, &r, 1.0, &ka_judge::options(def));
        let b = judge_with(def, design, &r, 1.0, &full);
        for (x, y) in a.tests.iter().zip(&b.tests) {
            let line = format!(
                "{label} {}: sampled {:?} p95 {:.1} err {:.3}% | full {:?} p95 {:.1} err {:.3}%",
                x.id,
                x.verdict,
                x.p95_ms,
                x.error_rate * 100.0,
                y.verdict,
                y.p95_ms,
                y.error_rate * 100.0
            );
            if x.verdict != y.verdict {
                problems_found.push(format!("verdict differs: {line}"));
            }
            if y.p95_ms > 0.0 && (x.p95_ms - y.p95_ms).abs() > f64::EPSILON {
                compared += 1;
                let d = (x.p95_ms - y.p95_ms) / y.p95_ms;
                worst = worst.max(d.abs());
                println!("{line} ({:+.1}%)", d * 100.0);
                if x.verdict == Verdict::Accepted && d.abs() > 0.10 {
                    problems_found.push(format!("p95 off by {:.1}%: {line}", d * 100.0));
                }
            }
            if x.limits.p99_ms.is_some() && y.p99_ms > 0.0 {
                let d = (x.p99_ms - y.p99_ms) / y.p99_ms;
                println!(
                    "{label} {}: p99 sampled {:.2} full {:.2} ({:+.1}%)",
                    x.id,
                    x.p99_ms,
                    y.p99_ms,
                    d * 100.0
                );
                if x.verdict == Verdict::Accepted && d.abs() > 0.10 {
                    problems_found.push(format!("p99 off by {:.1}%: {line}", d * 100.0));
                }
            }
        }
    }
    println!(
        "{} designs; {compared} sampled tests differ from full runs; worst p95 gap {:.1}%",
        cases.len(),
        worst * 100.0
    );
    assert!(problems_found.is_empty(), "{}", problems_found.join("\n"));
}

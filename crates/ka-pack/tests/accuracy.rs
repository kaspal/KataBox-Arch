//! Review A1 at the M2 and M3 gates: every Set 1 reference and naive design, judged with
//! sampling (the default) and without, gets the same verdict on every test; where a passing
//! test was sampled, p95 is within 10%. Release only:
//! `cargo test -p ka-pack --release --test accuracy -- --ignored --nocapture`.

use ka_core::registry;
use ka_judge::{Verdict, judge_with};
use ka_pack::load_problems;
use ka_sim::SimOptions;

#[test]
#[ignore = "release gate check over all content"]
fn sampled_and_full_runs_agree_on_every_set_1_design() {
    let r = registry();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let problems = load_problems(&root, &r).expect("content loads");
    let full = SimOptions {
        sample_cap: f64::INFINITY,
    };
    let mut worst = 0.0f64;
    let mut compared = 0;
    let mut problems_found: Vec<String> = Vec::new();
    for p in &problems {
        for (which, design) in [("reference", &p.reference), ("naive", &p.naive)] {
            let a = judge_with(&p.def, design, &r, 1.0, &ka_judge::options(&p.def));
            let b = judge_with(&p.def, design, &r, 1.0, &full);
            for (x, y) in a.tests.iter().zip(&b.tests) {
                let line = format!(
                    "{} {which} {}: sampled {:?} p95 {:.1} err {:.3}% | full {:?} p95 {:.1} err {:.3}%",
                    p.def.id,
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
            }
        }
    }
    println!(
        "{compared} sampled tests differ from full runs; worst p95 gap {:.1}%",
        worst * 100.0
    );
    assert!(problems_found.is_empty(), "{}", problems_found.join("\n"));
}

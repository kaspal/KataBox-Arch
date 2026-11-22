//! `ka-pack validate --run`: judge each problem's reference design (Accepted on every test)
//! and naive design (fails at least one hidden test), with `ka-judge`.

use ka_core::Registry;
use ka_judge::{Judgement, Verdict, judge};

use crate::problems::ProblemDir;
use crate::{Issue, issue};

fn summary(j: &Judgement) -> String {
    j.tests
        .iter()
        .map(|t| {
            format!(
                "{}{}: {:?} (p95 {:.1} ms, errors {:.3}%, cost ${:.0})",
                t.id,
                if t.hidden { " [hidden]" } else { "" },
                t.verdict,
                t.p95_ms,
                t.error_rate * 100.0,
                t.cost_month
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Judge one problem's two designs; returns the reference's judgement for reporting.
pub fn check_problem(p: &ProblemDir, registry: &Registry) -> (Vec<Issue>, Judgement, Judgement) {
    let mut issues = Vec::new();
    // The reference sets the cost bar, so it is judged against itself.
    let first = judge(&p.def, &p.reference, registry, 1.0);
    let reference = judge(&p.def, &p.reference, registry, first.cost_month);
    if reference.verdict != Verdict::Accepted {
        issues.push(issue(
            format!("{}/reference.kbarch", p.dir),
            format!("must be Accepted on every test: {}", summary(&reference)),
        ));
    }
    let naive = judge(&p.def, &p.naive, registry, reference.cost_month);
    if naive.verdict == Verdict::InvalidDesign {
        issues.push(issue(
            format!("{}/naive.kbarch", p.dir),
            format!("must be a valid design: {:?}", naive.lint),
        ));
    } else if !naive
        .tests
        .iter()
        .any(|t| t.hidden && t.verdict != Verdict::Accepted)
    {
        issues.push(issue(
            format!("{}/naive.kbarch", p.dir),
            format!("must fail at least one hidden test: {}", summary(&naive)),
        ));
    }
    (issues, reference, naive)
}

pub fn run_checks(problems: &[ProblemDir], registry: &Registry) -> Vec<Issue> {
    let mut issues = Vec::new();
    for p in problems {
        let (mut found, reference, naive) = check_problem(p, registry);
        println!(
            "{}: reference {:?} score {} (${:.0}); naive {:?} score {}",
            p.def.id,
            reference.verdict,
            reference.score.total,
            reference.cost_month,
            naive.verdict,
            naive.score.total
        );
        issues.append(&mut found);
    }
    issues
}

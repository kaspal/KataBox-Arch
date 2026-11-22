//! `ka-pack validate --run`: judge each problem's reference design (Accepted on every test)
//! and naive design (fails at least one hidden test). Needs `ka-judge` (M2.2).

use ka_core::Registry;

use crate::problems::ProblemDir;
use crate::{Issue, issue};

pub fn run_checks(problems: &[ProblemDir], _registry: &Registry) -> Vec<Issue> {
    problems
        .iter()
        .map(|p| issue(p.dir.clone(), "--run needs ka-judge, which arrives in M2.2"))
        .collect()
}

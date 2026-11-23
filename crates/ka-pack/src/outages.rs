//! Outage replays (M10): `content/outages/<id>/`.
//!
//! - Structural: ids, EN/ES text, three hint tiers in both languages, the target test and a
//!   `normal` test exist, the kinds are real, both designs pass design validation, every
//!   walkthrough step names `as_it_was.kbarch` or `fixed.kbarch` and plays a scenario test.
//! - `--run` (`check_outage`): `as_it_was` fails the incident (its target test) and passes
//!   `normal`; `fixed` is Accepted on every test (the aftershock included); every callout's
//!   numbers match the simulator.

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::outage::{OutageDef, OutageDoc};
use ka_core::{Design, Registry};
use ka_judge::Verdict;

use crate::problems::parse_hints;
use crate::{Issue, TODO_MARKER, issue};

pub const AS_IT_WAS: &str = "as_it_was.kbarch";
pub const FIXED: &str = "fixed.kbarch";

#[derive(Debug, Clone)]
pub struct OutageDir {
    /// `outages/<id>` (relative to the content root).
    pub dir: String,
    pub def: OutageDef,
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    pub as_it_was: Design,
    pub fixed: Design,
}

impl OutageDir {
    pub fn snapshot(&self, name: &str) -> Option<&Design> {
        match name {
            AS_IT_WAS => Some(&self.as_it_was),
            FIXED => Some(&self.fixed),
            _ => None,
        }
    }
}

pub fn load_outages(root: &Path, registry: &Registry) -> Result<Vec<OutageDir>, Vec<Issue>> {
    let base = root.join("outages");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<_> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue("outages", e.to_string())])?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for d in dirs {
        let name = d.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let rel = format!("outages/{name}");
        let read = |f: &str| -> Result<String, Issue> {
            std::fs::read_to_string(d.join(f))
                .map_err(|e| issue(format!("{rel}/{f}"), e.to_string()))
        };
        let design = |f: &str| -> Result<Design, Issue> {
            let text = read(f)?;
            ka_app::kbarch::parse(text.as_bytes(), registry)
                .map_err(|e| issue(format!("{rel}/{f}"), e.message))
        };
        let def = read("outage.toml").and_then(|t| {
            toml::from_str::<OutageDef>(&t)
                .map_err(|e| issue(format!("{rel}/outage.toml"), e.to_string()))
        });
        let hints = |f: &str| -> Result<Vec<String>, Issue> {
            parse_hints(&read(f)?).map_err(|m| issue(format!("{rel}/{f}"), m))
        };
        match (
            def,
            hints("hints.md"),
            hints("hints.es.md"),
            design(AS_IT_WAS),
            design(FIXED),
        ) {
            (Ok(def), Ok(hints_en), Ok(hints_es), Ok(as_it_was), Ok(fixed)) => {
                out.push(OutageDir {
                    dir: rel,
                    def,
                    hints_en,
                    hints_es,
                    as_it_was,
                    fixed,
                })
            }
            (a, b, c, d2, e) => {
                issues.extend(a.err());
                issues.extend(b.err());
                issues.extend(c.err());
                issues.extend(d2.err());
                issues.extend(e.err());
            }
        }
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

pub fn validate_outages(outages: &[OutageDir], registry: &Registry) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut numbers = BTreeSet::new();
    for o in outages {
        let d = &o.def;
        let file = format!("{}/outage.toml", o.dir);
        let mut bad = |m: String| issues.push(issue(&file, m));
        if o.dir != format!("outages/{}", d.id) {
            bad(format!("id `{}` differs from the folder", d.id));
        }
        if !numbers.insert(d.number) {
            bad(format!("number {} is used twice", d.number));
        }
        for m in crate::problems::loose_limits(&d.problem()) {
            bad(m);
        }
        for (name, text) in [
            ("title", &d.title),
            ("title_es", &d.title_es),
            ("summary", &d.summary),
            ("summary_es", &d.summary_es),
            ("story", &d.story),
            ("story_es", &d.story_es),
            ("lesson", &d.lesson),
            ("lesson_es", &d.lesson_es),
        ] {
            if text.trim().is_empty() || text.contains(TODO_MARKER) {
                bad(format!("`{name}` is empty or unfinished"));
            }
        }
        if d.kinds.is_empty() {
            bad("`kinds` names no component".into());
        }
        for url in &d.real_world {
            if !url.starts_with("https://") {
                bad(format!("real_world link `{url}` must be an https URL"));
            }
        }
        let tests = &d.scenario.tests;
        if !tests.iter().any(|t| t.id == d.target_test) {
            bad(format!("target test `{}` doesn't exist", d.target_test));
        }
        if !tests.iter().any(|t| t.id == "normal") {
            bad("needs a `normal` test".into());
        }
        let mut ids = BTreeSet::new();
        for t in tests {
            if !ids.insert(&t.id) {
                bad(format!("test `{}` is defined twice", t.id));
            }
            if t.name.trim().is_empty() || t.name_es.trim().is_empty() {
                bad(format!("test `{}` needs names in both languages", t.id));
            }
        }
        if d.steps.is_empty() {
            bad("needs at least one walkthrough `[[step]]`".into());
        }
        for (i, s) in d.steps.iter().enumerate() {
            if o.snapshot(&s.snapshot).is_none() {
                bad(format!(
                    "step {}: snapshot `{}` must be `{AS_IT_WAS}` or `{FIXED}`",
                    i + 1,
                    s.snapshot
                ));
            }
            for (name, text) in [
                ("title", &s.title),
                ("title_es", &s.title_es),
                ("caption", &s.caption),
                ("caption_es", &s.caption_es),
            ] {
                if text.trim().is_empty() || text.contains(TODO_MARKER) {
                    bad(format!("step {}: `{name}` is empty or unfinished", i + 1));
                }
            }
            if let Some(run) = &s.run {
                match &run.test {
                    Some(t) if !tests.iter().any(|x| &x.id == t) => {
                        bad(format!("step {}: test `{t}` doesn't exist", i + 1))
                    }
                    None => bad(format!("step {}: a replay step plays a test", i + 1)),
                    _ => {}
                }
            }
            if s.callout.is_some() && s.run.is_none() {
                bad(format!("step {}: a callout needs a run", i + 1));
            }
            if s.callout.is_some_and(|c| c.before.is_some()) && i == 0 {
                bad("step 1: `before` needs a previous step".into());
            }
        }
        if o.hints_en.len() != 3 || o.hints_es.len() != 3 {
            bad("hints need three tiers in each language".into());
        }
        let problem = d.problem();
        for (which, design) in [("as_it_was", &o.as_it_was), ("fixed", &o.fixed)] {
            let lint = ka_judge::lint(&problem, design, registry);
            if !lint.is_empty() {
                issues.push(issue(
                    format!("{}/{which}.kbarch", o.dir),
                    format!("must be a valid design for the scenario: {lint:?}"),
                ));
            }
        }
    }
    issues
}

/// `--run`: the as-it-was design fails the incident, the fix holds through the aftershock,
/// and the walkthrough's numbers are the simulator's. Returns issues and detail lines.
pub fn check_outage(
    o: &OutageDir,
    registry: &Registry,
    verbose: bool,
) -> (Vec<Issue>, Vec<String>) {
    let mut issues = Vec::new();
    let problem = o.def.problem();
    let first = ka_judge::judge(&problem, &o.fixed, registry, 1.0);
    let fixed = ka_judge::judge(&problem, &o.fixed, registry, first.cost_month);
    if fixed.verdict != Verdict::Accepted {
        issues.push(issue(
            format!("{}/{FIXED}", o.dir),
            format!(
                "must be Accepted on every test: {}",
                crate::run::summary(&fixed)
            ),
        ));
    }
    let was = ka_judge::judge(&problem, &o.as_it_was, registry, fixed.cost_month);
    let verdict_of = |id: &str| was.tests.iter().find(|t| t.id == id).map(|t| t.verdict);
    if verdict_of(&o.def.target_test) == Some(Verdict::Accepted) {
        issues.push(issue(
            format!("{}/{AS_IT_WAS}", o.dir),
            format!(
                "must fail the incident `{}`: {}",
                o.def.target_test,
                crate::run::summary(&was)
            ),
        ));
    }
    if o.def.target_test != "normal" && verdict_of("normal") != Some(Verdict::Accepted) {
        issues.push(issue(
            format!("{}/{AS_IT_WAS}", o.dir),
            format!(
                "must pass `normal` (it ran fine until the incident): {}",
                crate::run::summary(&was)
            ),
        ));
    }
    let file = format!("{}/outage.toml", o.dir);
    let mut details = vec![
        format!("  fixed:     {}", crate::run::summary(&fixed)),
        format!("  as it was: {}", crate::run::summary(&was)),
    ];
    for (i, step) in o.def.steps.iter().enumerate() {
        let (Some(c), Some(run)) = (&step.callout, &step.run) else {
            continue;
        };
        let mut check = |snapshot: &str, stated: f64, which: &str| {
            let Some(design) = o.snapshot(snapshot) else {
                return;
            };
            match crate::samples::simulate_step(design, run, Some(&problem), registry) {
                Ok(r) => {
                    let got = crate::samples::metric(&r, c.metric);
                    if verbose {
                        details.push(format!(
                            "  step {} {which}: {:?} stated {stated}, simulated {got:.4}",
                            i + 1,
                            c.metric
                        ));
                    }
                    if !crate::samples::close(c.metric, stated, got) {
                        issues.push(issue(
                            &file,
                            format!(
                                "step {}: `{which}` says {stated} but the simulator gives {got:.4}",
                                i + 1
                            ),
                        ));
                    }
                }
                Err(e) => issues.push(issue(&file, format!("step {}: {e}", i + 1))),
            }
        };
        check(&step.snapshot, c.after, "after");
        if let Some(b) = c.before
            && i > 0
        {
            check(&o.def.steps[i - 1].snapshot, b, "before");
        }
    }
    (issues, details)
}

/// The stored replay: its definition, hints, both designs and the fix's cost on the first
/// test (the cost bar for the score).
pub fn outage_doc(o: &OutageDir, registry: &Registry) -> Result<OutageDoc, String> {
    let problem = o.def.problem();
    let first = problem.tests.first().ok_or("no tests")?;
    let run = ka_judge::run_test(&problem, first, &o.fixed, registry).map_err(|e| e.to_string())?;
    let bytes = |d: &Design| -> Result<String, String> {
        String::from_utf8(ka_app::kbarch::to_bytes(d, registry).map_err(|e| e.message)?)
            .map_err(|e| e.to_string())
    };
    Ok(OutageDoc {
        def: o.def.clone(),
        hints_en: o.hints_en.clone(),
        hints_es: o.hints_es.clone(),
        as_it_was: bytes(&o.as_it_was)?,
        fixed: bytes(&o.fixed)?,
        ref_cost: run.cost_month,
        ref_p95_ms: run.latency.p95_ms,
    })
}

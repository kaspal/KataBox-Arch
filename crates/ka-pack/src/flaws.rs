//! Find-the-flaw drills (M5.3): `content/flaws/<id>/`.
//!
//! - Structural: ids, EN/ES text, three hint tiers in both languages, the target test
//!   exists, a `normal` test exists, the kinds are real, both designs pass design validation.
//! - `--run` (`check_flaw`): `broken` fails its target test and passes `normal` (unless the
//!   flaw is about `normal` itself: one clear flaw, not a broken system; review A9), and is
//!   flagged when it fails more than two tests; `fixed` is Accepted on every test, which
//!   includes staying under budget.

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::flaw::{FlawDef, FlawDoc};
use ka_core::{Design, Registry};
use ka_judge::Verdict;

use crate::problems::parse_hints;
use crate::{Issue, TODO_MARKER, issue};

#[derive(Debug, Clone)]
pub struct FlawDir {
    /// `flaws/<id>` (relative to the content root).
    pub dir: String,
    pub def: FlawDef,
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    pub broken: Design,
    pub fixed: Design,
}

pub fn load_flaws(root: &Path, registry: &Registry) -> Result<Vec<FlawDir>, Vec<Issue>> {
    let base = root.join("flaws");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<_> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue("flaws", e.to_string())])?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for d in dirs {
        let name = d.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let rel = format!("flaws/{name}");
        let read = |f: &str| -> Result<String, Issue> {
            std::fs::read_to_string(d.join(f))
                .map_err(|e| issue(format!("{rel}/{f}"), e.to_string()))
        };
        let design = |f: &str| -> Result<Design, Issue> {
            let text = read(f)?;
            ka_app::kbarch::parse(text.as_bytes(), registry)
                .map_err(|e| issue(format!("{rel}/{f}"), e.message))
        };
        let def = read("flaw.toml").and_then(|t| {
            toml::from_str::<FlawDef>(&t)
                .map_err(|e| issue(format!("{rel}/flaw.toml"), e.to_string()))
        });
        let hints = |f: &str| -> Result<Vec<String>, Issue> {
            parse_hints(&read(f)?).map_err(|m| issue(format!("{rel}/{f}"), m))
        };
        match (
            def,
            hints("hints.md"),
            hints("hints.es.md"),
            design("broken.kbarch"),
            design("fixed.kbarch"),
        ) {
            (Ok(def), Ok(hints_en), Ok(hints_es), Ok(broken), Ok(fixed)) => out.push(FlawDir {
                dir: rel,
                def,
                hints_en,
                hints_es,
                broken,
                fixed,
            }),
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

pub fn validate_flaws(flaws: &[FlawDir], registry: &Registry) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut numbers = BTreeSet::new();
    for f in flaws {
        let d = &f.def;
        let file = format!("{}/flaw.toml", f.dir);
        let mut bad = |m: String| issues.push(issue(&file, m));
        if f.dir != format!("flaws/{}", d.id) {
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
            ("story", &d.story),
            ("story_es", &d.story_es),
            ("symptom", &d.symptom),
            ("symptom_es", &d.symptom_es),
            ("explanation", &d.explanation),
            ("explanation_es", &d.explanation_es),
        ] {
            if text.trim().is_empty() || text.contains(TODO_MARKER) {
                bad(format!("`{name}` is empty or unfinished"));
            }
        }
        if d.kinds.is_empty() {
            bad("`kinds` names no component".into());
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
        if f.hints_en.len() != 3 || f.hints_es.len() != 3 {
            bad("hints need three tiers in each language".into());
        }
        let problem = d.problem();
        for (which, design) in [("broken", &f.broken), ("fixed", &f.fixed)] {
            let lint = ka_judge::lint(&problem, design, registry);
            if !lint.is_empty() {
                issues.push(issue(
                    format!("{}/{which}.kbarch", f.dir),
                    format!("must be a valid design for the scenario: {lint:?}"),
                ));
            }
        }
    }
    issues
}

/// `--run`: the broken design shows its one flaw and the fix works. Returns issues and
/// warnings (printed, not failing).
pub fn check_flaw(f: &FlawDir, registry: &Registry) -> (Vec<Issue>, Vec<String>, Vec<String>) {
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    let problem = f.def.problem();
    let first = ka_judge::judge(&problem, &f.fixed, registry, 1.0);
    let fixed = ka_judge::judge(&problem, &f.fixed, registry, first.cost_month);
    if fixed.verdict != Verdict::Accepted {
        issues.push(issue(
            format!("{}/fixed.kbarch", f.dir),
            format!(
                "must be Accepted on every test: {}",
                crate::run::summary(&fixed)
            ),
        ));
    }
    let broken = ka_judge::judge(&problem, &f.broken, registry, fixed.cost_month);
    let verdict_of = |id: &str| broken.tests.iter().find(|t| t.id == id).map(|t| t.verdict);
    if verdict_of(&f.def.target_test) == Some(Verdict::Accepted) {
        issues.push(issue(
            format!("{}/broken.kbarch", f.dir),
            format!(
                "must fail its target test `{}`: {}",
                f.def.target_test,
                crate::run::summary(&broken)
            ),
        ));
    }
    if f.def.target_test != "normal" && verdict_of("normal") != Some(Verdict::Accepted) {
        issues.push(issue(
            format!("{}/broken.kbarch", f.dir),
            format!(
                "must pass `normal` (one clear flaw, not a broken system): {}",
                crate::run::summary(&broken)
            ),
        ));
    }
    let failing = broken
        .tests
        .iter()
        .filter(|t| t.verdict != Verdict::Accepted)
        .count();
    if failing > 2 {
        warnings.push(format!(
            "{}: broken.kbarch fails {failing} tests; a drill reads best with one clear symptom",
            f.dir
        ));
    }
    let details = vec![
        format!("  fixed:  {}", crate::run::summary(&fixed)),
        format!("  broken: {}", crate::run::summary(&broken)),
    ];
    (issues, warnings, details)
}

/// The stored drill: its definition, hints, both designs and the fix's cost on the first
/// test (the cost bar for the score).
pub fn flaw_doc(f: &FlawDir, registry: &Registry) -> Result<FlawDoc, String> {
    let problem = f.def.problem();
    let first = problem.tests.first().ok_or("no tests")?;
    let run = ka_judge::run_test(&problem, first, &f.fixed, registry).map_err(|e| e.to_string())?;
    let bytes = |d: &Design| -> Result<String, String> {
        String::from_utf8(ka_app::kbarch::to_bytes(d, registry).map_err(|e| e.message)?)
            .map_err(|e| e.to_string())
    };
    Ok(FlawDoc {
        def: f.def.clone(),
        hints_en: f.hints_en.clone(),
        hints_es: f.hints_es.clone(),
        broken: bytes(&f.broken)?,
        fixed: bytes(&f.fixed)?,
        ref_cost: run.cost_month,
        ref_p95_ms: run.latency.p95_ms,
    })
}

//! Pattern folders: `content/patterns/<id>/` with `pattern.toml`
//! (`ka_core::pattern::PatternEntry`), `pattern.md`, `pattern.es.md` and `template.kbarch`.
//!
//! Structural checks: the id matches the folder, numbers are unique, text is in both
//! languages (same list lengths, same paragraph count), the template passes design
//! validation and contains every kind the pattern names, and practice problems exist.

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::pattern::PatternEntry;
use ka_core::{Design, Registry};

use crate::problems::ProblemDir;
use crate::{Issue, TODO_MARKER, issue};

#[derive(Debug, Clone)]
pub struct PatternDir {
    pub dir: String,
    pub entry: PatternEntry,
    pub md_en: String,
    pub md_es: String,
    pub template: Design,
    pub toml: String,
}

pub fn load_patterns(root: &Path, registry: &Registry) -> Result<Vec<PatternDir>, Vec<Issue>> {
    let base = root.join("patterns");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<_> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue("patterns", e.to_string())])?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for name in dirs {
        let rel = format!("patterns/{name}");
        let dir = base.join(&name);
        let read = |f: &str| {
            std::fs::read_to_string(dir.join(f))
                .map_err(|e| issue(format!("{rel}/{f}"), format!("missing or unreadable: {e}")))
        };
        let parts = (
            read("pattern.toml"),
            read("pattern.md"),
            read("pattern.es.md"),
            read("template.kbarch"),
        );
        let (Ok(toml), Ok(md_en), Ok(md_es), Ok(template)) = parts else {
            let (a, b, c, d) = parts;
            for r in [a, b, c, d] {
                if let Err(i) = r {
                    issues.push(i);
                }
            }
            continue;
        };
        let entry = match toml::from_str::<PatternEntry>(&toml) {
            Ok(e) => e,
            Err(e) => {
                issues.push(issue(format!("{rel}/pattern.toml"), e.to_string()));
                continue;
            }
        };
        let template = match ka_app::kbarch::parse(template.as_bytes(), registry) {
            Ok(d) => d,
            Err(e) => {
                issues.push(issue(
                    format!("{rel}/template.kbarch"),
                    format!("{:?}: {}", e.code, e.message),
                ));
                continue;
            }
        };
        out.push(PatternDir {
            dir: rel,
            entry,
            md_en,
            md_es,
            template,
            toml,
        });
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

fn paragraphs(md: &str) -> usize {
    md.split("\n\n").filter(|p| !p.trim().is_empty()).count()
}

pub fn validate_patterns(
    patterns: &[PatternDir],
    problems: &[ProblemDir],
    registry: &Registry,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let problem_ids: BTreeSet<&str> = problems.iter().map(|p| p.def.id.as_str()).collect();
    let mut numbers = BTreeSet::new();
    for p in patterns {
        let e = &p.entry;
        let file = format!("{}/pattern.toml", p.dir);
        let mut bad = |m: String| issues.push(issue(&file, m));
        if p.dir != format!("patterns/{}", e.id) {
            bad(format!("id `{}` doesn't match its folder", e.id));
        }
        if !numbers.insert(e.number) {
            bad(format!("number {} is used twice", e.number));
        }
        if p.toml.contains(TODO_MARKER)
            || p.md_en.contains(TODO_MARKER)
            || p.md_es.contains(TODO_MARKER)
        {
            bad(format!("unfinished: contains {TODO_MARKER}"));
        }
        for (name, en, es) in [
            ("title", &e.title, &e.title_es),
            ("one_liner", &e.one_liner, &e.one_liner_es),
            ("problem", &e.problem, &e.problem_es),
            ("in_simulator", &e.in_simulator, &e.in_simulator_es),
        ] {
            if en.trim().is_empty() || es.trim().is_empty() {
                bad(format!("`{name}` needs English and Spanish"));
            }
        }
        for (name, en, es) in [
            ("use_when", &e.use_when, &e.use_when_es),
            ("gains", &e.gains, &e.gains_es),
            ("costs", &e.costs, &e.costs_es),
        ] {
            if en.len() != es.len() || en.is_empty() {
                bad(format!(
                    "`{name}` needs items, as many in English as in Spanish"
                ));
            }
            if en.iter().chain(es).any(|s| s.trim().is_empty()) {
                bad(format!("`{name}` has an empty item"));
            }
        }
        let (pe, ps) = (paragraphs(&p.md_en), paragraphs(&p.md_es));
        if pe == 0 || pe != ps {
            bad(format!(
                "pattern.md has {pe} paragraphs and pattern.es.md {ps}"
            ));
        }
        for i in ka_core::validate(&p.template, registry) {
            bad(format!("template.kbarch is invalid: {i:?}"));
        }
        if p.template.nodes.is_empty() {
            bad("the template has no cards".into());
        }
        if e.kinds.is_empty() {
            bad("`kinds` names none".into());
        }
        for k in &e.kinds {
            if !p.template.nodes.iter().any(|n| n.kind == *k) {
                bad(format!("the template has no {k:?} card"));
            }
        }
        if p.template.problem_id.is_some() {
            bad("the template belongs to no problem (problem_id = null)".into());
        }
        for pid in &e.practice {
            if !problem_ids.contains(pid.as_str()) {
                bad(format!("practice problem `{pid}` doesn't exist"));
            }
        }
    }
    issues
}

/// The `content.db` row: the entry as JSON, the prose and the template as `.kbarch` JSON.
pub fn pattern_row(p: &PatternDir, registry: &Registry) -> Result<ka_store::PatternRow, String> {
    let template = ka_app::kbarch::to_bytes(&p.template, registry).map_err(|e| e.message)?;
    Ok(ka_store::PatternRow {
        id: p.entry.id.clone(),
        entry: serde_json::to_string(&p.entry).map_err(|e| e.to_string())?,
        md_en: p.md_en.clone(),
        md_es: p.md_es.clone(),
        template: String::from_utf8(template).map_err(|e| e.to_string())?,
    })
}

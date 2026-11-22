//! Component Guide folders: `content/guide/<kind>/` with `entry.toml` (`ka_core::guide::
//! GuideEntry`), `entry.md` and `entry.es.md` ("What it does", two or three paragraphs).
//!
//! The checks (all structural, no simulation):
//! - every registry kind has exactly one entry, in the folder named after it;
//! - English and Spanish match: every list has as many items in each language, and the prose
//!   as many paragraphs;
//! - simulator rows name a real registry value (a prop of the kind, a simulator default or a
//!   price), and their notes contain no digits: the page reads the numbers from the registry;
//! - compared flavours belong to the kind; related kinds and problems exist;
//! - every flavour of the kind has a source (an `https://` page; unchecked ones are listed).

use std::collections::BTreeSet;
use std::path::Path;

use ka_core::guide::{GuideEntry, SimValue};
use ka_core::{ComponentKind, Registry};

use crate::problems::ProblemDir;
use crate::{Issue, TODO_MARKER, issue};

pub const SIM_FIELDS: [&str; 6] = [
    "servers_per_instance",
    "service_ms",
    "p99_ratio",
    "queue_per_server",
    "failover_s",
    "write_slots",
];
pub const COST_FIELDS: [&str; 5] = [
    "per_instance_month",
    "per_lane_month",
    "per_gb_month",
    "per_million",
    "per_million_write",
];
/// Props the simulator doesn't read (design notes only): an "In the simulator" row may not
/// name them. `tests/guide.rs` checks `ka-sim` still ignores each one.
pub const NOT_SIMULATED: [&str; 9] = [
    "consistency",
    "eviction",
    "interval_s",
    "persistence",
    "replicas",
    "replication_factor",
    "retention_days",
    "retention_h",
    "sample_rate",
];

#[derive(Debug, Clone)]
pub struct GuideDir {
    /// `guide/<kind>` (relative to the content root).
    pub dir: String,
    pub entry: GuideEntry,
    pub md_en: String,
    pub md_es: String,
    pub toml: String,
}

/// The serde name of a kind (`lb`, `ratelimit`): its folder name.
pub fn kind_id(kind: ComponentKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub fn load_guide(root: &Path) -> Result<Vec<GuideDir>, Vec<Issue>> {
    let base = root.join("guide");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<_> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue("guide", e.to_string())])?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for name in dirs {
        let rel = format!("guide/{name}");
        let dir = base.join(&name);
        let read = |f: &str| {
            std::fs::read_to_string(dir.join(f))
                .map_err(|e| issue(format!("{rel}/{f}"), format!("missing or unreadable: {e}")))
        };
        let (toml, md_en, md_es) = (read("entry.toml"), read("entry.md"), read("entry.es.md"));
        let (Ok(toml), Ok(md_en), Ok(md_es)) = (toml, md_en, md_es) else {
            for r in [read("entry.toml"), read("entry.md"), read("entry.es.md")] {
                if let Err(i) = r {
                    issues.push(i);
                }
            }
            continue;
        };
        match toml::from_str::<GuideEntry>(&toml) {
            Ok(entry) => out.push(GuideDir {
                dir: rel,
                entry,
                md_en,
                md_es,
                toml,
            }),
            Err(e) => issues.push(issue(format!("{rel}/entry.toml"), e.to_string())),
        }
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

/// Blocks of text separated by blank lines.
fn paragraphs(md: &str) -> usize {
    md.split("\n\n").filter(|p| !p.trim().is_empty()).count()
}

pub fn validate_guide(
    entries: &[GuideDir],
    problems: &[ProblemDir],
    registry: &Registry,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let problem_ids: BTreeSet<&str> = problems.iter().map(|p| p.def.id.as_str()).collect();
    let mut seen = BTreeSet::new();
    for g in entries {
        let e = &g.entry;
        let file = format!("{}/entry.toml", g.dir);
        let id = kind_id(e.kind);
        let mut bad = |m: String| issues.push(issue(&file, m));
        for m in ka_core::quiz::check(&e.quiz) {
            bad(format!("quiz: {m}"));
        }
        if g.dir != format!("guide/{id}") {
            bad(format!("kind `{id}` doesn't match its folder"));
        }
        if !seen.insert(e.kind) {
            bad(format!("`{id}` has two entries"));
        }
        if g.toml.contains(TODO_MARKER)
            || g.md_en.contains(TODO_MARKER)
            || g.md_es.contains(TODO_MARKER)
        {
            bad(format!("unfinished: contains {TODO_MARKER}"));
        }
        let spec = registry.kind(e.kind);
        // Text in both languages.
        for (name, en, es) in [
            ("one_liner", &e.one_liner, &e.one_liner_es),
            ("formula", &e.formula, &e.formula_es),
            ("interview_tip", &e.interview_tip, &e.interview_tip_es),
        ] {
            if en.trim().is_empty() || es.trim().is_empty() {
                bad(format!("`{name}` needs English and Spanish"));
            }
        }
        for (name, en, es, min) in [
            ("use_when", &e.use_when, &e.use_when_es, 2),
            ("skip_when", &e.skip_when, &e.skip_when_es, 2),
            ("gains", &e.gains, &e.gains_es, 1),
            ("costs", &e.costs, &e.costs_es, 1),
        ] {
            if en.len() != es.len() {
                bad(format!(
                    "`{name}` has {} items in English and {} in Spanish",
                    en.len(),
                    es.len()
                ));
            }
            if en.len() < min {
                bad(format!("`{name}` needs at least {min} items"));
            }
            if en.iter().chain(es).any(|s| s.trim().is_empty()) {
                bad(format!("`{name}` has an empty item"));
            }
        }
        let (pe, ps) = (paragraphs(&g.md_en), paragraphs(&g.md_es));
        if pe == 0 || pe != ps {
            bad(format!("entry.md has {pe} paragraphs and entry.es.md {ps}"));
        }
        // The simulator rows name real registry values; numbers come from the registry.
        if e.sim.is_empty() {
            bad("`sim` needs at least one row".into());
        }
        for row in &e.sim {
            match &row.value {
                SimValue::Prop(k) if !spec.props.iter().any(|p| &p.key == k) => {
                    bad(format!("sim prop `{k}` isn't a property of `{id}`"))
                }
                SimValue::Prop(k) if NOT_SIMULATED.contains(&k.as_str()) => bad(format!(
                    "sim prop `{k}` isn't simulated (describe it in prose instead)"
                )),
                SimValue::Sim(k) if !SIM_FIELDS.contains(&k.as_str()) => bad(format!(
                    "sim `{k}` isn't a simulator default ({SIM_FIELDS:?})"
                )),
                SimValue::Cost(k) if !COST_FIELDS.contains(&k.as_str()) => {
                    bad(format!("cost `{k}` isn't a price ({COST_FIELDS:?})"))
                }
                _ => {}
            }
            if row.note.trim().is_empty() || row.note_es.trim().is_empty() {
                bad("a sim row needs a note in English and Spanish".into());
            }
            if row
                .note
                .chars()
                .chain(row.note_es.chars())
                .any(|c| c.is_ascii_digit())
            {
                bad(format!(
                    "sim note for {:?} has a digit (numbers come from the registry)",
                    row.value
                ));
            }
        }
        if let Some(c) = &e.flavours_compared {
            if c.flavours.len() < 2 {
                bad("`flavours_compared` needs at least two flavours".into());
            }
            for f in &c.flavours {
                if !spec.flavours.iter().any(|x| &x.flavour == f) {
                    bad(format!("compared flavour {f:?} isn't a flavour of `{id}`"));
                }
            }
            for r in &c.rows {
                if r.cells.len() != c.flavours.len() || r.cells_es.len() != c.flavours.len() {
                    bad(format!(
                        "compare row `{}` needs one cell per flavour in each language",
                        r.label
                    ));
                }
                if r.label.trim().is_empty() || r.label_es.trim().is_empty() {
                    bad("a compare row needs a label in English and Spanish".into());
                }
            }
        }
        if e.pitfalls.is_empty() {
            bad("`pitfalls` needs at least one".into());
        }
        for p in &e.pitfalls {
            for (name, en, es) in [
                ("title", &p.title, &p.title_es),
                ("body", &p.body, &p.body_es),
                ("fix", &p.fix, &p.fix_es),
            ] {
                if en.trim().is_empty() || es.trim().is_empty() {
                    bad(format!(
                        "pitfall `{}`: `{name}` needs English and Spanish",
                        p.title
                    ));
                }
            }
            if let Some(pid) = &p.seen_in
                && !problem_ids.contains(pid.as_str())
            {
                bad(format!(
                    "pitfall `{}`: problem `{pid}` doesn't exist",
                    p.title
                ));
            }
        }
        for k in &e.works_with {
            if *k == e.kind {
                bad("`works_with` names the entry's own kind".into());
            }
        }
        if e.works_with.is_empty() {
            bad("`works_with` needs at least one kind".into());
        }
        for pid in &e.practice {
            if !problem_ids.contains(pid.as_str()) {
                bad(format!("practice problem `{pid}` doesn't exist"));
            }
        }
        for f in &spec.flavours {
            if !e.sources.iter().any(|s| s.flavour == f.flavour) {
                bad(format!("flavour {:?} has no source", f.flavour));
            }
        }
        for s in &e.sources {
            if !s.url.starts_with("https://") {
                bad(format!("source `{}` needs an https:// link", s.title));
            }
            if !spec.flavours.iter().any(|f| f.flavour == s.flavour) {
                bad(format!(
                    "source `{}` names flavour {:?}, not one of `{id}`",
                    s.title, s.flavour
                ));
            }
        }
    }
    for k in &registry.kinds {
        if !seen.contains(&k.kind) {
            issues.push(issue(
                "guide",
                format!("`{}` has no entry", kind_id(k.kind)),
            ));
        }
    }
    issues
}

/// The `content.db` row.
pub fn guide_row(g: &GuideDir) -> Result<ka_store::GuideRow, String> {
    Ok(ka_store::GuideRow {
        kind: kind_id(g.entry.kind),
        entry: serde_json::to_string(&g.entry).map_err(|e| e.to_string())?,
        md_en: g.md_en.clone(),
        md_es: g.md_es.clone(),
    })
}

/// Sources not yet checked against the vendor page (for the report).
pub fn unchecked_sources(entries: &[GuideDir]) -> Vec<(String, String)> {
    entries
        .iter()
        .flat_map(|g| {
            g.entry
                .sources
                .iter()
                .filter(|s| !s.checked)
                .map(move |s| (kind_id(g.entry.kind), s.url.clone()))
        })
        .collect()
}

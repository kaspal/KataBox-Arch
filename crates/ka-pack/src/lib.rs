//! `ka-pack`: reads `content/`, validates it and builds `content.db` (ADR 0008).
//!
//! - `load_problems` reads every `content/problems/<id>/` folder.
//! - `validate_problems` is the structural check: files, EN/ES parity, ids, routes, designs.
//!   It is fast and needs no simulation, so the desktop build script can run it (review A5).
//! - `run::run_checks` is `validate --run`: with `ka-judge`, the reference passes every
//!   test and the naive design fails at least one hidden test.
//! - `load_samples` / `validate_samples` do the same for `content/samples/<id>/`, and
//!   `samples::check_callouts` is their part of `validate --run`.
//! - `build_content` turns validated content into rows, and `write_db` writes the file.

pub mod guide;
pub mod problems;
pub mod run;
pub mod samples;
pub mod scaffold;

use std::path::Path;

use sha2::{Digest, Sha256};

pub use guide::{GuideDir, load_guide, validate_guide};
pub use problems::{ProblemDir, load_problems, validate_problems};
pub use samples::{SampleDir, load_samples, validate_samples};

/// A problem with the content: the file (relative to the content root) and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

pub(crate) fn issue(path: impl Into<String>, message: impl Into<String>) -> Issue {
    Issue {
        path: path.into(),
        message: message.into(),
    }
}

/// The marker `ka-pack new` leaves in templates; content with it is unfinished.
pub const TODO_MARKER: &str = "TODO(ka-pack)";

/// A hash of every file under `root`, in path order: the content version in `meta`.
pub fn content_hash(root: &Path) -> std::io::Result<String> {
    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort();
    let mut h = Sha256::new();
    for rel in files {
        h.update(rel.as_bytes());
        h.update([0]);
        h.update(std::fs::read(root.join(&rel))?);
        h.update([0]);
    }
    Ok(h.finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            collect(root, &p, out)?;
        } else {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

/// Build the rows of `content.db` from validated content.
pub fn build_content(
    root: &Path,
    registry: &ka_core::Registry,
) -> Result<ka_store::ContentData, Vec<Issue>> {
    let problems = load_problems(root, registry)?;
    let samples = load_samples(root, registry)?;
    let guide = load_guide(root)?;
    let mut issues = validate_problems(&problems, registry);
    issues.extend(validate_samples(&samples, &problems, registry));
    issues.extend(validate_guide(&guide, &problems, registry));
    if !issues.is_empty() {
        return Err(issues);
    }
    let hash =
        content_hash(root).map_err(|e| vec![issue(root.display().to_string(), e.to_string())])?;
    let mut rows = Vec::new();
    for p in &problems {
        rows.push(p.row(registry).map_err(|m| vec![issue(p.dir.clone(), m)])?);
    }
    let mut sample_rows = Vec::new();
    for s in &samples {
        sample_rows
            .push(samples::sample_row(s, registry).map_err(|m| vec![issue(s.dir.clone(), m)])?);
    }
    Ok(ka_store::ContentData {
        meta: vec![
            ("content_hash".into(), hash),
            ("schema".into(), ka_store::CONTENT_SCHEMA.to_string()),
            ("app_version".into(), env!("CARGO_PKG_VERSION").into()),
        ],
        problems: rows,
        samples: sample_rows,
        guide: guide
            .iter()
            .map(guide::guide_row)
            .collect::<Result<_, _>>()
            .map_err(|m| vec![issue("guide", m)])?,
    })
}

/// Write `content.db` (a current-thread runtime of its own, so build scripts can call it).
pub fn write_db(out: &Path, data: &ka_store::ContentData) -> Result<(), String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|e| e.to_string())?
        .block_on(ka_store::write_content(out, data))
        .map_err(|e| e.to_string())
}

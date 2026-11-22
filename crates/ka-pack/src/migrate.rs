//! `ka-pack migrate`: rewrite every `.kbarch` under the content root in the current format
//! (ADR 0003), so content and newly saved files share one format. A file opens through the
//! app's own reader (which migrates it) and is written back with the app's writer only when
//! the bytes change. Simulation results don't change: review A2 checks `validate --run`
//! before and after.

use std::path::{Path, PathBuf};

use ka_core::Registry;

use crate::{Issue, issue};

/// Every `.kbarch` file under `root`, in path order.
pub fn kbarch_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some(ka_app::kbarch::EXTENSION) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Rewrite the files that aren't in the current format. Returns (rewritten, total).
pub fn rewrite(root: &Path, registry: &Registry) -> Result<(usize, usize), Vec<Issue>> {
    let files = kbarch_files(root);
    let mut issues = Vec::new();
    let mut changed = 0;
    for f in &files {
        let name = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let result = std::fs::read(f)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                let d = ka_app::kbarch::parse(&bytes, registry).map_err(|e| e.message)?;
                let out = ka_app::kbarch::to_bytes(&d, registry).map_err(|e| e.message)?;
                Ok((bytes, out))
            });
        match result {
            Ok((before, after)) if before != after => {
                if let Err(e) = std::fs::write(f, after) {
                    issues.push(issue(name, e.to_string()));
                } else {
                    changed += 1;
                }
            }
            Ok(_) => {}
            Err(e) => issues.push(issue(name, e)),
        }
    }
    if issues.is_empty() {
        Ok((changed, files.len()))
    } else {
        Err(issues)
    }
}

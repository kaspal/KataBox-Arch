//! `ka-pack new <id>`: a problem folder from templates. Every place to fill in carries the
//! `TODO(ka-pack)` marker, which validation refuses, so an unfinished problem never ships.

use std::path::{Path, PathBuf};

use crate::TODO_MARKER;

const TOML: &str = r#"# TODO(ka-pack): fill in this problem (see docs/problem-authoring.md).
id = "{id}"
number = 0
set = 1
difficulty = "easy"
title = "TODO(ka-pack)"
title_es = "TODO(ka-pack)"
teaches = "TODO(ka-pack)"
teaches_es = "TODO(ka-pack)"
tags = []

[nfr]
peak_rps = 1000
p95_ms = 100
availability = 0.999
budget_usd = 1000

[[request_types]]
id = "read"
share = 1.0
route = ["edge?", "compute", "datastore"]

[[tests]]
id = "normal"
name = "Normal day"
name_es = "Un día normal"

[[tests]]
id = "spike"
name = "TODO(ka-pack)"
name_es = "TODO(ka-pack)"
hidden = true
traffic = { spike = { at = 20, x = 4, for = 15 } }

[pass]
error_rate = 0.001
"#;

const PROSE: &str = "# TODO(ka-pack)\n\n## The problem\n\nTODO(ka-pack)\n";
const HINTS: &str = "## 1\n\nTODO(ka-pack)\n\n## 2\n\nTODO(ka-pack)\n\n## 3\n\nTODO(ka-pack)\n";

/// Create `root/problems/<id>/`; refuses an existing folder or an id that isn't kebab-case.
pub fn new_problem(root: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!("{id:?}: ids are kebab-case (a-z, 0-9, -)"));
    }
    let dir = root.join("problems").join(id);
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let registry = ka_core::registry();
    let empty = |name: &str| {
        let d = ka_core::Design::empty(format!("{id}-{name}"), format!("{TODO_MARKER} {name}"));
        ka_app::kbarch::to_bytes(&d, &registry).map_err(|e| e.message)
    };
    let files: [(&str, Vec<u8>); 9] = [
        ("problem.toml", TOML.replace("{id}", id).into_bytes()),
        ("statement.md", PROSE.as_bytes().to_vec()),
        ("statement.es.md", PROSE.as_bytes().to_vec()),
        ("hints.md", HINTS.as_bytes().to_vec()),
        ("hints.es.md", HINTS.as_bytes().to_vec()),
        ("editorial.md", PROSE.as_bytes().to_vec()),
        ("editorial.es.md", PROSE.as_bytes().to_vec()),
        ("reference.kbarch", empty("reference")?),
        ("naive.kbarch", empty("naive")?),
    ];
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    Ok(dir)
}

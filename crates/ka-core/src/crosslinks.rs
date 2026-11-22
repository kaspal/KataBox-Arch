//! KataBox cross-links (M6.7): from an Arch page (`guide:<kind>`, `pattern:<id>`,
//! `problem:<id>`) to the KataBox Learn topics and problems that drill the algorithm behind
//! it. `content/katabox-ids.toml` is a read-only snapshot of the KataBox ids and titles
//! linked to; `content/crosslinks.toml` holds the links. Shown as titles only for now.

use serde::{Deserialize, Serialize};

/// A KataBox id with its titles, as snapshotted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KataboxEntry {
    pub id: String,
    pub title: String,
    pub title_es: String,
}

/// `katabox-ids.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KataboxIds {
    pub katabox_commit: String,
    #[serde(default)]
    pub learn: Vec<KataboxEntry>,
    #[serde(default)]
    pub problem: Vec<KataboxEntry>,
}

/// One `[[link]]` of `crosslinks.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkDef {
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrosslinkFile {
    #[serde(default)]
    pub link: Vec<LinkDef>,
}

/// What a KataBox target is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum KataboxKind {
    Learn,
    Problem,
}

/// A resolved link, as the UI shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct KataboxLink {
    pub kind: KataboxKind,
    /// `<subject>/<topic>` for Learn, the problem id for problems.
    pub id: String,
    pub title: String,
    pub title_es: String,
}

impl KataboxIds {
    /// `learn:<subject>/<topic>` or `problem:<id>`, if it is in the snapshot.
    pub fn resolve(&self, target: &str) -> Option<KataboxLink> {
        let (kind, list, id) = match target.split_once(':')? {
            ("learn", id) => (KataboxKind::Learn, &self.learn, id),
            ("problem", id) => (KataboxKind::Problem, &self.problem, id),
            _ => return None,
        };
        list.iter().find(|e| e.id == id).map(|e| KataboxLink {
            kind,
            id: e.id.clone(),
            title: e.title.clone(),
            title_es: e.title_es.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_resolve_by_kind_and_id() {
        let ids: KataboxIds = toml::from_str(
            r#"
katabox_commit = "abc"
[[learn]]
id = "linear/lru-cache"
title = "LRU cache"
title_es = "Caché LRU"
[[problem]]
id = "league-rank"
title = "League Rank"
title_es = "Clasificación"
"#,
        )
        .unwrap();
        assert_eq!(
            ids.resolve("learn:linear/lru-cache").unwrap().title,
            "LRU cache"
        );
        assert_eq!(
            ids.resolve("problem:league-rank").unwrap().kind,
            KataboxKind::Problem
        );
        assert!(ids.resolve("problem:linear/lru-cache").is_none());
        assert!(ids.resolve("guide:cache").is_none());
        assert!(ids.resolve("league-rank").is_none());
    }
}

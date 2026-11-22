//! The pattern library from `content.db`, with read marks from `user.db` (v3 `reads`).

use ka_core::pattern::PatternEntry;
use ka_core::{ComponentKind, Design};
use serde::Serialize;

use crate::{App, AppError, AppErrorCode, Result};

const READ_KIND: &str = "pattern";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PatternSummary {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub one_liner: String,
    pub one_liner_es: String,
    pub kinds: Vec<ComponentKind>,
    pub template: Design,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PatternPage {
    pub entry: PatternEntry,
    pub md_en: String,
    pub md_es: String,
    pub template: Design,
    pub read: bool,
}

fn bad_content(e: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::Internal, format!("content.db: {e}"))
}

impl App {
    fn pattern_page(&self, row: &ka_store::PatternRow, read: &[String]) -> Result<PatternPage> {
        Ok(PatternPage {
            entry: serde_json::from_str(&row.entry).map_err(bad_content)?,
            md_en: row.md_en.clone(),
            md_es: row.md_es.clone(),
            template: crate::kbarch::parse(row.template.as_bytes(), &self.registry)
                .map_err(|e| bad_content(e.message))?,
            read: read.contains(&row.id),
        })
    }

    /// Every pattern, in library order.
    pub async fn list_patterns(&self) -> Result<Vec<PatternSummary>> {
        let rows = self.content()?.patterns().await.map_err(AppError::from)?;
        let read = self.user.reads(READ_KIND).await.map_err(AppError::from)?;
        let mut out = Vec::new();
        for r in &rows {
            let p = self.pattern_page(r, &read)?;
            out.push(PatternSummary {
                id: p.entry.id.clone(),
                number: p.entry.number,
                title: p.entry.title,
                title_es: p.entry.title_es,
                one_liner: p.entry.one_liner,
                one_liner_es: p.entry.one_liner_es,
                kinds: p.entry.kinds,
                template: p.template,
                read: p.read,
            });
        }
        out.sort_by_key(|p| p.number);
        Ok(out)
    }

    pub async fn pattern(&self, id: &str) -> Result<PatternPage> {
        let rows = self.content()?.patterns().await.map_err(AppError::from)?;
        let row = rows
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("pattern {id}")))?;
        let read = self.user.reads(READ_KIND).await.map_err(AppError::from)?;
        self.pattern_page(row, &read)
    }

    pub async fn set_pattern_read(&self, id: &str, read: bool) -> Result<()> {
        self.user
            .set_read(READ_KIND, id, read)
            .await
            .map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;

    const ENTRY: &str = r#"
id = "cache-aside"
number = 1
title = "Cache-aside"
title_es = "Cache-aside"
one_liner = "Read the cache first."
one_liner_es = "Lee primero la caché."
problem = "p"
problem_es = "p"
use_when = ["a"]
use_when_es = ["a"]
gains = ["g"]
gains_es = ["g"]
costs = ["c"]
costs_es = ["c"]
kinds = ["app"]
in_simulator = "Fully"
in_simulator_es = "Del todo"
"#;

    #[tokio::test]
    async fn patterns_list_in_order_with_templates_and_read_marks() {
        let dir = temp_dir("patterns");
        let r = ka_core::registry();
        let mut d = Design::empty("t", "T");
        d.nodes = vec![ka_core::Node {
            id: "app".into(),
            kind: ComponentKind::App,
            flavour: ka_core::Flavour::Go,
            label: String::new(),
            position: ka_core::Point { x: 0.0, y: 0.0 },
            props: Default::default(),
            group_id: None,
        }];
        let template = String::from_utf8(crate::kbarch::to_bytes(&d, &r).unwrap()).unwrap();
        let entry: PatternEntry = toml::from_str(ENTRY).unwrap();
        let mut second = entry.clone();
        second.id = "read-replicas".into();
        second.number = 2;
        let row = |e: &PatternEntry| ka_store::PatternRow {
            id: e.id.clone(),
            entry: serde_json::to_string(e).unwrap(),
            md_en: "How".into(),
            md_es: "Cómo".into(),
            template: template.clone(),
        };
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                patterns: vec![row(&second), row(&entry)],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let app = App::open(AppPaths {
            user_db: dir.join("user.db"),
            dev_data: true,
            content_db: Some(content),
        })
        .await
        .unwrap();
        let list = app.list_patterns().await.unwrap();
        assert_eq!(
            list.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["cache-aside", "read-replicas"]
        );
        assert_eq!(list[0].template.nodes.len(), 1);
        app.set_pattern_read("cache-aside", true).await.unwrap();
        assert!(app.pattern("cache-aside").await.unwrap().read);
        assert!(!app.pattern("read-replicas").await.unwrap().read);
        assert_eq!(
            app.pattern("nope").await.unwrap_err().code,
            AppErrorCode::NotFound
        );
    }
}

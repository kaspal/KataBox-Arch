//! The Component Guide from `content.db`, with read marks from `user.db` (v3 `reads`).

use ka_core::ComponentKind;
use ka_core::guide::GuideEntry;
use serde::Serialize;

use crate::{App, AppError, AppErrorCode, Result};

/// Read marks for Guide entries live under this kind in `reads`.
const READ_KIND: &str = "guide";

/// An index card: the one-line definition and whether it was read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GuideSummary {
    pub kind: ComponentKind,
    pub one_liner: String,
    pub one_liner_es: String,
    pub read: bool,
}

/// A whole entry: the structured fields, the prose in both languages and the read mark.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct GuidePage {
    pub entry: GuideEntry,
    pub md_en: String,
    pub md_es: String,
    pub read: bool,
}

fn bad_content(e: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::Internal, format!("content.db: {e}"))
}

fn kind_id(kind: ComponentKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

impl App {
    /// Every entry's card, in registry order (the index groups them by category).
    pub async fn list_guide(&self) -> Result<Vec<GuideSummary>> {
        let rows = self.content()?.guide().await.map_err(AppError::from)?;
        let read = self.user.reads(READ_KIND).await.map_err(AppError::from)?;
        let mut out = Vec::new();
        for k in &self.registry.kinds {
            let id = kind_id(k.kind);
            let Some(row) = rows.iter().find(|r| r.kind == id) else {
                continue;
            };
            let entry: GuideEntry = serde_json::from_str(&row.entry).map_err(bad_content)?;
            out.push(GuideSummary {
                kind: k.kind,
                one_liner: entry.one_liner,
                one_liner_es: entry.one_liner_es,
                read: read.contains(&id),
            });
        }
        Ok(out)
    }

    pub async fn guide_page(&self, kind: ComponentKind) -> Result<GuidePage> {
        let id = kind_id(kind);
        let row = self
            .content()?
            .guide_entry(&id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("guide entry {id}")))?;
        let read = self.user.reads(READ_KIND).await.map_err(AppError::from)?;
        Ok(GuidePage {
            entry: serde_json::from_str(&row.entry).map_err(bad_content)?,
            md_en: row.md_en,
            md_es: row.md_es,
            read: read.contains(&id),
        })
    }

    pub async fn set_guide_read(&self, kind: ComponentKind, read: bool) -> Result<()> {
        self.user
            .set_read(READ_KIND, &kind_id(kind), read)
            .await
            .map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;

    #[tokio::test]
    async fn the_guide_lists_entries_in_registry_order_with_read_marks() {
        let dir = temp_dir("guide");
        let toml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/guide/cache/entry.toml"),
        )
        .unwrap();
        let entry: GuideEntry = toml::from_str(&toml).unwrap();
        let mut lb = entry.clone();
        lb.kind = ComponentKind::Lb;
        lb.one_liner = "Spreads traffic".into();
        let row = |e: &GuideEntry| ka_store::GuideRow {
            kind: kind_id(e.kind),
            entry: serde_json::to_string(e).unwrap(),
            md_en: "What it does.".into(),
            md_es: "Qué hace.".into(),
        };
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                guide: vec![row(&entry), row(&lb)],
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
        let list = app.list_guide().await.unwrap();
        // Registry order: the load balancer (edge) before the cache (data).
        assert_eq!(
            list.iter().map(|g| g.kind).collect::<Vec<_>>(),
            [ComponentKind::Lb, ComponentKind::Cache]
        );
        assert!(list.iter().all(|g| !g.read));
        app.set_guide_read(ComponentKind::Cache, true)
            .await
            .unwrap();
        let page = app.guide_page(ComponentKind::Cache).await.unwrap();
        assert!(page.read);
        assert_eq!(page.entry.kind, ComponentKind::Cache);
        assert_eq!(page.md_es, "Qué hace.");
        assert!(app.list_guide().await.unwrap()[1].read);
        app.set_guide_read(ComponentKind::Cache, false)
            .await
            .unwrap();
        assert!(!app.guide_page(ComponentKind::Cache).await.unwrap().read);
        assert_eq!(
            app.guide_page(ComponentKind::Dns).await.unwrap_err().code,
            AppErrorCode::NotFound
        );
    }
}

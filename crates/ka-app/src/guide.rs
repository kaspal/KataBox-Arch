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
        let mut entry: GuideEntry = serde_json::from_str(&row.entry).map_err(bad_content)?;
        // Quiz answers stay in Rust: the questions come through `quiz` without them.
        entry.quiz.clear();
        Ok(GuidePage {
            entry,
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

    #[tokio::test]
    async fn quizzes_hide_answers_and_feed_the_review_queue() {
        use ka_core::quiz::QuizQuestion;
        let dir = temp_dir("quiz");
        let toml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/guide/cache/entry.toml"),
        )
        .unwrap();
        let mut entry: GuideEntry = toml::from_str(&toml).unwrap();
        let q = |answer: u32| QuizQuestion {
            q: "Q?".into(),
            q_es: "¿P?".into(),
            options: vec!["a".into(), "b".into(), "c".into()],
            options_es: vec!["a".into(), "b".into(), "c".into()],
            answer,
            why: "Why.".into(),
            why_es: "Por qué.".into(),
        };
        entry.quiz = vec![q(0), q(1), q(2)];
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                guide: vec![ka_store::GuideRow {
                    kind: "cache".into(),
                    entry: serde_json::to_string(&entry).unwrap(),
                    md_en: "x".into(),
                    md_es: "x".into(),
                }],
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
        // The page and the quiz never carry the answers.
        assert!(
            app.guide_page(ComponentKind::Cache)
                .await
                .unwrap()
                .entry
                .quiz
                .is_empty()
        );
        let items = app.quiz("guide:cache").await.unwrap();
        assert_eq!(
            items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["guide:cache:1", "guide:cache:2", "guide:cache:3"]
        );
        assert!(!serde_json::to_string(&items).unwrap().contains("Why."));
        // Right first time: never queued. Wrong: due tomorrow.
        let r1 = app
            .answer_quiz("guide:cache:1", 0, "2026-11-22")
            .await
            .unwrap();
        assert!(r1.correct && r1.due_on.is_none());
        let r2 = app
            .answer_quiz("guide:cache:2", 0, "2026-11-22")
            .await
            .unwrap();
        assert!(!r2.correct);
        assert_eq!((r2.answer, r2.due_on.as_deref()), (1, Some("2026-11-23")));
        assert_eq!(r2.why, "Why.");
        assert!(app.reviews_due("2026-11-22", 10).await.unwrap().is_empty());
        let due = app.reviews_due("2026-11-23", 10).await.unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "guide:cache:2");
        // Reviewed correctly: next in 3 days.
        let r3 = app
            .answer_quiz("guide:cache:2", 1, "2026-11-23")
            .await
            .unwrap();
        assert_eq!(r3.due_on.as_deref(), Some("2026-11-26"));
        // The page's quiz counts as done once all three were answered.
        assert!(app.user.reads("quiz").await.unwrap().is_empty());
        app.answer_quiz("guide:cache:3", 2, "2026-11-23")
            .await
            .unwrap();
        assert_eq!(app.user.reads("quiz").await.unwrap(), ["guide:cache"]);
        // Bad input is refused.
        for (id, day) in [
            ("guide:cache:9", "2026-11-23"),
            ("guide:cache:1", "23/11/2026"),
        ] {
            assert_eq!(
                app.answer_quiz(id, 0, day).await.unwrap_err().code,
                AppErrorCode::InvalidInput
            );
        }
    }
}

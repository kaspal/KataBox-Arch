//! Decision guides and the glossary from `content.db`.

use ka_core::decide::{DecideFlow, GlossaryTerm};

use crate::{App, AppError, AppErrorCode, Result};

fn bad_content(e: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::Internal, format!("content.db: {e}"))
}

impl App {
    /// The "Which one do I need?" flows, in order.
    pub async fn decide_flows(&self) -> Result<Vec<DecideFlow>> {
        let rows = self.content()?.decide().await.map_err(AppError::from)?;
        let mut flows = rows
            .iter()
            .map(|r| serde_json::from_str::<DecideFlow>(&r.json).map_err(bad_content))
            .collect::<Result<Vec<_>>>()?;
        flows.sort_by_key(|f| f.number);
        Ok(flows)
    }

    /// Every glossary term, alphabetically by id.
    pub async fn glossary(&self) -> Result<Vec<GlossaryTerm>> {
        let rows = self.content()?.glossary().await.map_err(AppError::from)?;
        rows.iter()
            .map(|r| serde_json::from_str::<GlossaryTerm>(&r.json).map_err(bad_content))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;

    #[tokio::test]
    async fn flows_come_in_order_and_terms_read_back() {
        let dir = temp_dir("decide");
        let flow = |id: &str, number: u32| DecideFlow {
            id: id.into(),
            number,
            title: id.into(),
            title_es: id.into(),
            intro: "i".into(),
            intro_es: "i".into(),
            start: "q".into(),
            questions: vec![],
        };
        let term = GlossaryTerm {
            id: "ttl".into(),
            term: "TTL".into(),
            term_es: "TTL".into(),
            definition: "How long".into(),
            definition_es: "Cuánto".into(),
            see: Some(ka_core::ComponentKind::Cache),
        };
        let row = |id: &str, json: String| ka_store::JsonRow {
            id: id.into(),
            json,
        };
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                decide: vec![
                    row("b", serde_json::to_string(&flow("b", 1)).unwrap()),
                    row("a", serde_json::to_string(&flow("a", 2)).unwrap()),
                ],
                glossary: vec![row("ttl", serde_json::to_string(&term).unwrap())],
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let app = App::open(AppPaths {
            user_db: dir.join("user.db"),
            dev_data: true,
            content_db: Some(content),
            packs_dir: None,
        })
        .await
        .unwrap();
        let flows = app.decide_flows().await.unwrap();
        assert_eq!(
            flows.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert_eq!(app.glossary().await.unwrap(), vec![term]);
    }
}

//! Practice content from `content.db` (M5): the numbers cheat sheet, and from later steps
//! estimation drills, Find-the-flaw drills and learning paths.

use ka_core::numbers::NumberFigure;

use crate::{App, AppError, AppErrorCode, Result};

pub(crate) fn bad_content(e: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::Internal, format!("content.db: {e}"))
}

impl App {
    /// The numbers cheat sheet, in sheet order (empty if the content has none).
    pub async fn numbers(&self) -> Result<Vec<NumberFigure>> {
        match self
            .content()?
            .doc("numbers", "sheet")
            .await
            .map_err(AppError::from)?
        {
            Some(row) => serde_json::from_str(&row.json).map_err(bad_content),
            None => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;
    use ka_core::numbers::NumberGroup;

    pub(crate) async fn app_with(dir: &std::path::Path, docs: Vec<ka_store::DocRow>) -> App {
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                docs,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        App::open(AppPaths {
            user_db: dir.join("user.db"),
            dev_data: true,
            content_db: Some(content),
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn the_numbers_sheet_reads_back_in_order() {
        let fig = |id: &str| NumberFigure {
            id: id.into(),
            group: NumberGroup::Network,
            label: id.into(),
            label_es: id.into(),
            value: 1.5,
            unit: "ms".into(),
            note: "n".into(),
            note_es: "n".into(),
            source: "arithmetic".into(),
        };
        let sheet = vec![fig("b"), fig("a")];
        let dir = temp_dir("numbers");
        let app = app_with(
            &dir,
            vec![ka_store::DocRow {
                kind: "numbers".into(),
                id: "sheet".into(),
                json: serde_json::to_string(&sheet).unwrap(),
            }],
        )
        .await;
        assert_eq!(app.numbers().await.unwrap(), sheet);
        let empty = app_with(&temp_dir("numbers-none"), vec![]).await;
        assert!(empty.numbers().await.unwrap().is_empty());
    }
}

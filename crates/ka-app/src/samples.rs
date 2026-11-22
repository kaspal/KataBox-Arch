//! Samples from `content.db`: the gallery list and one sample with its design snapshots, for
//! the walkthrough player. Snapshots are parsed and validated like any opened file.

use std::collections::BTreeMap;

use ka_core::Design;
use ka_core::story::{SampleDef, StepRun};
use serde::Serialize;

use crate::{App, AppError, AppErrorCode, Result};

/// A gallery card: the text, and the first step's design and traffic for the preview.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SampleSummary {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub summary: String,
    pub summary_es: String,
    pub tour: bool,
    pub steps: u32,
    pub problem: Option<String>,
    pub preview: Design,
    pub preview_run: Option<StepRun>,
}

/// One sample for the player: its steps and every snapshot they name.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Sample {
    pub def: SampleDef,
    pub snapshots: BTreeMap<String, Design>,
}

fn bad_content(e: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::Internal, format!("content.db: {e}"))
}

impl App {
    fn sample_from_row(&self, row: &ka_store::SampleRow) -> Result<Sample> {
        let def: SampleDef = serde_json::from_str(&row.def).map_err(bad_content)?;
        let mut snapshots = BTreeMap::new();
        for (name, json) in &row.snapshots {
            let d = crate::kbarch::parse(json.as_bytes(), &self.registry)
                .map_err(|e| bad_content(format!("{name}: {}", e.message)))?;
            snapshots.insert(name.clone(), d);
        }
        Ok(Sample { def, snapshots })
    }

    /// Every sample, in gallery order.
    pub async fn list_samples(&self) -> Result<Vec<SampleSummary>> {
        let rows = self.content()?.samples().await.map_err(AppError::from)?;
        let mut out = Vec::new();
        for row in &rows {
            let s = self.sample_from_row(row)?;
            let first = s
                .def
                .steps
                .first()
                .ok_or_else(|| bad_content(format!("sample {} has no steps", s.def.id)))?;
            let preview = s
                .snapshots
                .get(&first.snapshot)
                .cloned()
                .ok_or_else(|| bad_content(format!("{} is missing", first.snapshot)))?;
            out.push(SampleSummary {
                id: s.def.id.clone(),
                number: s.def.number,
                title: s.def.title.clone(),
                title_es: s.def.title_es.clone(),
                summary: s.def.summary.clone(),
                summary_es: s.def.summary_es.clone(),
                tour: s.def.tour,
                steps: s.def.steps.len() as u32,
                problem: s.def.problem.clone(),
                preview_run: s.def.steps.iter().find_map(|x| x.run.clone()),
                preview,
            });
        }
        Ok(out)
    }

    /// The concept card of a component kind (`None`: it has no card yet).
    pub async fn concept(&self, kind: ka_core::ComponentKind) -> Result<Option<Sample>> {
        let id = serde_json::to_value(kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        let Some(row) = self.content()?.concept(&id).await.map_err(AppError::from)? else {
            return Ok(None);
        };
        let row: ka_store::SampleRow = serde_json::from_str(&row.def).map_err(bad_content)?;
        self.sample_from_row(&row).map(Some)
    }

    pub async fn sample(&self, id: &str) -> Result<Sample> {
        let row = self
            .content()?
            .sample(id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("sample {id}")))?;
        self.sample_from_row(&row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;
    use ka_core::{Edge, Node, Point, Protocol};

    fn design() -> Design {
        let mut d = Design::empty("hello", "Hello");
        let node = |id: &str, kind: &str, flavour: &str, x: f64| Node {
            id: id.into(),
            kind: serde_json::from_value(serde_json::json!(kind)).unwrap(),
            flavour: serde_json::from_value(serde_json::json!(flavour)).unwrap(),
            label: String::new(),
            position: Point { x, y: 0.0 },
            props: Default::default(),
            group_id: None,
        };
        d.nodes = vec![
            node("client", "client", "web", 0.0),
            node("app", "app", "go", 300.0),
        ];
        d.edges = vec![Edge {
            id: "e1".into(),
            from: "client".into(),
            to: "app".into(),
            protocol: Protocol::Http,
        }];
        d
    }

    const DEF: &str = r#"
id = "hello"
number = 1
title = "Hello, Traffic"
title_es = "Hola, tráfico"
summary = "One server."
summary_es = "Un servidor."
tour = true

[[step]]
snapshot = "one.kbarch"
title = "One"
title_es = "Uno"
caption = "c"
caption_es = "c"
run = { preset = "normal" }
"#;

    #[tokio::test]
    async fn samples_list_with_a_preview_and_open_with_their_snapshots() {
        let dir = temp_dir("samples");
        let r = ka_core::registry();
        let def: SampleDef = toml::from_str(DEF).unwrap();
        let kbarch = String::from_utf8(crate::kbarch::to_bytes(&design(), &r).unwrap()).unwrap();
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                meta: vec![],
                problems: vec![],
                samples: vec![ka_store::SampleRow {
                    id: "hello".into(),
                    number: 1,
                    def: serde_json::to_string(&def).unwrap(),
                    snapshots: [("one.kbarch".to_string(), kbarch)].into(),
                }],
                guide: vec![],
                concepts: vec![],
                patterns: vec![],
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
        let list = app.list_samples().await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].tour);
        assert_eq!(list[0].steps, 1);
        assert_eq!(list[0].preview.nodes.len(), 2);
        assert_eq!(
            list[0].preview_run.as_ref().unwrap().preset.as_deref(),
            Some("normal")
        );
        let s = app.sample("hello").await.unwrap();
        assert_eq!(s.def.steps[0].snapshot, "one.kbarch");
        assert!(s.snapshots.contains_key("one.kbarch"));
        assert_eq!(
            app.sample("nope").await.unwrap_err().code,
            AppErrorCode::NotFound
        );
    }
}

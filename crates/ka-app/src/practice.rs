//! Practice content from `content.db` (M5): the numbers cheat sheet, and from later steps
//! estimation drills, Find-the-flaw drills and learning paths.

use ka_core::estimate::{Drill, DrillSet, EstimateCheck};
use ka_core::flaw::{FlawDoc, FlawSummary, flaw_problem_id};
use ka_core::numbers::NumberFigure;
use ka_store::EstimateStatus;

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

    async fn drill_sets(&self) -> Result<Vec<DrillSet>> {
        let rows = self
            .content()?
            .docs("estimation")
            .await
            .map_err(AppError::from)?;
        rows.iter()
            .map(|r| serde_json::from_str::<DrillSet>(&r.json).map_err(bad_content))
            .collect()
    }

    /// Estimation drills (every problem's, or one problem's), without their answers: the
    /// answer and the worked solution only come back from [`App::check_estimate`].
    pub async fn drills(&self, problem_id: Option<&str>) -> Result<Vec<Drill>> {
        Ok(self
            .drill_sets()
            .await?
            .into_iter()
            .filter(|s| problem_id.is_none_or(|p| p == s.problem_id))
            .flat_map(|s| {
                let pid = s.problem_id.clone();
                s.drills.into_iter().map(move |d| Drill {
                    problem_id: pid.clone(),
                    id: d.id,
                    kind: d.kind,
                    prompt: d.prompt,
                    prompt_es: d.prompt_es,
                    inputs: d.inputs,
                    unit: d.unit,
                })
            })
            .collect())
    }

    /// Judge an answer against the drill's band, record it, and reveal the worked solution.
    pub async fn check_estimate(
        &self,
        problem_id: &str,
        drill_id: &str,
        given: f64,
    ) -> Result<EstimateCheck> {
        if !given.is_finite() {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                "the answer must be a number",
            ));
        }
        let sets = self.drill_sets().await?;
        let found = sets
            .iter()
            .find(|s| s.problem_id == problem_id)
            .and_then(|s| {
                s.drills
                    .iter()
                    .zip(&s.answers)
                    .find(|(d, _)| d.id == drill_id)
            });
        let Some((d, &answer)) = found else {
            return Err(AppError::new(
                AppErrorCode::NotFound,
                format!("no drill {problem_id}/{drill_id}"),
            ));
        };
        let closeness = ka_core::estimate::judge(answer, given, d.tolerance);
        let ok = closeness == ka_core::estimate::Closeness::Close;
        self.user
            .record_estimate(problem_id, drill_id, given, ok)
            .await
            .map_err(AppError::from)?;
        Ok(EstimateCheck {
            ok,
            closeness,
            answer,
            solution: d.solution.clone(),
            solution_es: d.solution_es.clone(),
            inputs: d.inputs.clone(),
        })
    }

    /// The drills answered so far.
    pub async fn estimate_statuses(&self) -> Result<Vec<EstimateStatus>> {
        self.user.estimate_statuses().await.map_err(AppError::from)
    }

    async fn flaw_doc(&self, id: &str) -> Result<FlawDoc> {
        let row = self
            .content()?
            .doc("flaw", id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("flaw {id}")))?;
        serde_json::from_str(&row.json).map_err(bad_content)
    }

    /// A drill as a problem row: its story and symptom as the statement, the explanation as
    /// the editorial and the fix as the reference (both only shown once solved, as for any
    /// problem).
    pub(crate) async fn flaw_row(&self, id: &str) -> Result<ka_store::ProblemRow> {
        let doc = self.flaw_doc(id).await?;
        let d = &doc.def;
        let problem = d.problem();
        let statement = |story: &str, symptom: &str, h1: &str, h2: &str| {
            format!("## {h1}\n\n{story}\n\n## {h2}\n\n{symptom}\n")
        };
        Ok(ka_store::ProblemRow {
            id: flaw_problem_id(&d.id),
            number: i64::from(d.number),
            set: 0,
            difficulty: "medium".into(),
            title_en: d.title.clone(),
            title_es: d.title_es.clone(),
            teaches_en: String::new(),
            teaches_es: String::new(),
            tags: Vec::new(),
            nfr: serde_json::to_string(&problem.nfr).map_err(bad_content)?,
            def: serde_json::to_string(&problem).map_err(bad_content)?,
            statement_en: statement(&d.story, &d.symptom, "The system", "What goes wrong"),
            statement_es: statement(&d.story_es, &d.symptom_es, "El sistema", "Qué falla"),
            hints_en: doc.hints_en.clone(),
            hints_es: doc.hints_es.clone(),
            editorial_en: d.explanation.clone(),
            editorial_es: d.explanation_es.clone(),
            reference: doc.fixed.clone(),
            ref_cost: doc.ref_cost,
            ref_p95_ms: doc.ref_p95_ms,
        })
    }

    /// The Find-the-flaw drills in order, each marked solved or not.
    pub async fn list_flaws(&self) -> Result<Vec<FlawSummary>> {
        let rows = self.content()?.docs("flaw").await.map_err(AppError::from)?;
        let statuses = self.user.problem_statuses().await.map_err(AppError::from)?;
        let mut out = rows
            .iter()
            .map(|r| {
                let doc: FlawDoc = serde_json::from_str(&r.json).map_err(bad_content)?;
                let pid = flaw_problem_id(&doc.def.id);
                Ok(FlawSummary {
                    solved: statuses
                        .iter()
                        .any(|s| s.problem_id == pid && s.status == "solved"),
                    id: doc.def.id,
                    number: doc.def.number,
                    title: doc.def.title,
                    title_es: doc.def.title_es,
                    kinds: doc.def.kinds,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        out.sort_by_key(|f| f.number);
        Ok(out)
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

    #[tokio::test]
    async fn drills_hide_their_answers_until_checked() {
        use ka_core::estimate::{Closeness, DrillDef, DrillKind};
        let def = DrillDef {
            id: "qps".into(),
            kind: DrillKind::Qps,
            prompt: "{daily} a day".into(),
            prompt_es: "{daily} al día".into(),
            inputs: [("daily".to_string(), 8_640_000.0)].into(),
            formula: "daily / 86400".into(),
            unit: "rps".into(),
            tolerance: 0.5,
            solution: "{daily} / 86,400 = {answer}".into(),
            solution_es: "{daily} / 86 400 = {answer}".into(),
        };
        let set = DrillSet {
            problem_id: "url-shortener".into(),
            drills: vec![def],
            answers: vec![100.0],
        };
        let dir = temp_dir("drills");
        let app = app_with(
            &dir,
            vec![ka_store::DocRow {
                kind: "estimation".into(),
                id: "url-shortener".into(),
                json: serde_json::to_string(&set).unwrap(),
            }],
        )
        .await;
        let drills = app.drills(None).await.unwrap();
        assert_eq!(drills.len(), 1);
        let json = serde_json::to_string(&drills).unwrap();
        for hidden in ["answer", "formula", "solution", "tolerance"] {
            assert!(!json.contains(hidden), "{hidden}: {json}");
        }
        assert!(app.drills(Some("pastebin")).await.unwrap().is_empty());

        let miss = app
            .check_estimate("url-shortener", "qps", 400.0)
            .await
            .unwrap();
        assert_eq!((miss.ok, miss.closeness), (false, Closeness::TooHigh));
        let hit = app
            .check_estimate("url-shortener", "qps", 120.0)
            .await
            .unwrap();
        assert!(hit.ok);
        assert_eq!(hit.answer, 100.0);
        assert_eq!(hit.solution, "{daily} / 86,400 = {answer}");
        assert_eq!(hit.inputs.get("daily"), Some(&8_640_000.0));
        let s = app.estimate_statuses().await.unwrap();
        assert_eq!((s[0].tries, s[0].done), (2, true));
        assert_eq!(
            app.check_estimate("url-shortener", "nope", 1.0)
                .await
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
        assert_eq!(
            app.check_estimate("url-shortener", "qps", f64::NAN)
                .await
                .unwrap_err()
                .code,
            AppErrorCode::InvalidInput
        );
    }

    #[tokio::test]
    async fn a_flaw_drill_is_judged_and_recorded_as_a_problem() {
        use ka_core::flaw::FlawDef;
        let def: FlawDef = toml::from_str(
            r#"
id = "tiny"
number = 1
title = "Tiny"
title_es = "Diminuto"
story = "One app."
story_es = "Una app."
symptom = "It breaks."
symptom_es = "Se rompe."
target_test = "normal"
explanation = "Because."
explanation_es = "Porque."
kinds = ["app"]

[scenario.nfr]
peak_rps = 100
p95_ms = 100
availability = 0.99
budget_usd = 1000

[[scenario.request_types]]
id = "read"
share = 1.0
route = ["compute"]

[[scenario.tests]]
id = "normal"
name = "Normal day"
name_es = "Un día normal"
duration_s = 5

[scenario.pass]
error_rate = 0.01
"#,
        )
        .unwrap();
        let reg = ka_core::registry();
        let mut fixed = ka_core::Design::empty("fixed", "Fixed");
        let node = |id: &str, kind, flavour| ka_core::Node {
            id: id.into(),
            kind,
            flavour,
            label: String::new(),
            position: ka_core::Point { x: 0.0, y: 0.0 },
            props: Default::default(),
            group_id: None,
        };
        fixed.nodes = vec![
            node("c", ka_core::ComponentKind::Client, ka_core::Flavour::Web),
            node("a", ka_core::ComponentKind::App, ka_core::Flavour::Go),
        ];
        fixed.edges = vec![ka_core::Edge {
            id: "e".into(),
            from: "c".into(),
            to: "a".into(),
            protocol: ka_core::Protocol::Http,
        }];
        let kbarch = String::from_utf8(crate::kbarch::to_bytes(&fixed, &reg).unwrap()).unwrap();
        let doc = FlawDoc {
            def,
            hints_en: vec!["a".into(), "b".into(), "c".into()],
            hints_es: vec!["a".into(), "b".into(), "c".into()],
            broken: kbarch.clone(),
            fixed: kbarch,
            ref_cost: 70.0,
            ref_p95_ms: 6.0,
        };
        let app = app_with(
            &temp_dir("flaw"),
            vec![ka_store::DocRow {
                kind: "flaw".into(),
                id: "tiny".into(),
                json: serde_json::to_string(&doc).unwrap(),
            }],
        )
        .await;
        let list = app.list_flaws().await.unwrap();
        assert_eq!((list[0].id.as_str(), list[0].solved), ("tiny", false));
        let brief = app.problem_brief("flaw--tiny").await.unwrap();
        assert!(
            brief.statement_en.contains("One app.") && brief.statement_en.contains("It breaks.")
        );
        assert!(brief.statement_es.contains("Qué falla"));
        assert_eq!(brief.editorial_en, None);
        assert!(brief.reference.is_none());
        let (problem, ref_cost) = app.problem_for_judging("flaw--tiny").await.unwrap();
        let j = crate::problems::judge_design(&problem, &fixed, &reg, ref_cost);
        assert_eq!(j.verdict, ka_judge::Verdict::Accepted);
        app.record_submission(&fixed, j, "2026-11-22")
            .await
            .unwrap();
        assert!(app.list_flaws().await.unwrap()[0].solved);
        let after = app.problem_brief("flaw--tiny").await.unwrap();
        assert_eq!(after.editorial_en.as_deref(), Some("Because."));
        assert!(after.reference.is_some());
        // A drill never shows in the problem list or as "Continue".
        assert!(app.list_problems().await.unwrap().is_empty());
        assert!(app.problem_brief("flaw--nope").await.is_err());
    }
}

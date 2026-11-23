//! Outage replays (M10) from `content.db` (`docs` rows of kind `outage`): the Replays list,
//! the "Watch it happen" story for the walkthrough player, and the replay as the problem
//! `outage--<id>` ("Fix it": judged, run, drafted and recorded like any problem).

use std::collections::BTreeMap;

use ka_core::outage::{OutageDoc, OutageSummary, outage_problem_id};
use ka_core::story::SampleDef;
use serde::Serialize;

use crate::practice::bad_content;
use crate::samples::Sample;
use crate::{App, AppError, AppErrorCode, Result};

/// "Watch it happen": the replay's story and its walkthrough (a sample whose steps play the
/// replay's tests).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct OutageStory {
    pub summary: OutageSummary,
    pub story: String,
    pub story_es: String,
    pub lesson: String,
    pub lesson_es: String,
    pub real_world: Vec<String>,
    pub walkthrough: Sample,
}

impl App {
    async fn outage_doc(&self, id: &str) -> Result<OutageDoc> {
        let row = self
            .content()?
            .doc("outage", id)
            .await
            .map_err(AppError::from)?
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("replay {id}")))?;
        serde_json::from_str(&row.json).map_err(bad_content)
    }

    fn outage_summary(&self, doc: &OutageDoc, solved: bool) -> OutageSummary {
        let d = &doc.def;
        OutageSummary {
            id: d.id.clone(),
            number: d.number,
            difficulty: d.difficulty,
            title: d.title.clone(),
            title_es: d.title_es.clone(),
            summary: d.summary.clone(),
            summary_es: d.summary_es.clone(),
            kinds: d.kinds.clone(),
            solved,
        }
    }

    async fn outage_solved(&self, id: &str) -> Result<bool> {
        let pid = outage_problem_id(id);
        Ok(self
            .user
            .problem_statuses()
            .await
            .map_err(AppError::from)?
            .iter()
            .any(|s| s.problem_id == pid && s.status == "solved"))
    }

    /// The replays in order, each marked fixed (solved) or not.
    pub async fn list_outages(&self) -> Result<Vec<OutageSummary>> {
        let rows = self
            .content()?
            .docs("outage")
            .await
            .map_err(AppError::from)?;
        let statuses = self.user.problem_statuses().await.map_err(AppError::from)?;
        let mut out = rows
            .iter()
            .map(|r| {
                let doc: OutageDoc = serde_json::from_str(&r.json).map_err(bad_content)?;
                let pid = outage_problem_id(&doc.def.id);
                let solved = statuses
                    .iter()
                    .any(|s| s.problem_id == pid && s.status == "solved");
                Ok(self.outage_summary(&doc, solved))
            })
            .collect::<Result<Vec<_>>>()?;
        out.sort_by_key(|o| o.number);
        Ok(out)
    }

    /// "Watch it happen": the story and the walkthrough, with both designs as snapshots.
    pub async fn outage_story(&self, id: &str) -> Result<OutageStory> {
        let doc = self.outage_doc(id).await?;
        let solved = self.outage_solved(id).await?;
        let d = &doc.def;
        let mut snapshots = BTreeMap::new();
        for (name, json) in [
            ("as_it_was.kbarch", &doc.as_it_was),
            ("fixed.kbarch", &doc.fixed),
        ] {
            let design = crate::kbarch::parse(json.as_bytes(), &self.registry)
                .map_err(|e| bad_content(format!("{name}: {}", e.message)))?;
            snapshots.insert(name.to_owned(), design);
        }
        Ok(OutageStory {
            summary: self.outage_summary(&doc, solved),
            story: d.story.clone(),
            story_es: d.story_es.clone(),
            lesson: d.lesson.clone(),
            lesson_es: d.lesson_es.clone(),
            real_world: d.real_world.clone(),
            walkthrough: Sample {
                def: SampleDef {
                    id: d.id.clone(),
                    number: d.number,
                    title: d.title.clone(),
                    title_es: d.title_es.clone(),
                    summary: d.summary.clone(),
                    summary_es: d.summary_es.clone(),
                    problem: Some(outage_problem_id(&d.id)),
                    tour: false,
                    steps: d.steps.clone(),
                },
                snapshots,
            },
        })
    }

    /// A replay as a problem row: what happened as the statement, the lesson as the
    /// editorial and the fix as the reference (both shown once solved, as for any problem).
    pub(crate) async fn outage_row(&self, id: &str) -> Result<ka_store::ProblemRow> {
        let doc = self.outage_doc(id).await?;
        let d = &doc.def;
        let problem = d.problem();
        let target = problem
            .tests
            .iter()
            .find(|t| t.id == d.target_test)
            .ok_or_else(|| bad_content(format!("replay {id}: no target test")))?;
        let statement = |story: &str, test: &str, h1: &str, h2: &str| {
            format!("## {h1}\n\n{story}\n\n## {h2}\n\n{test}\n")
        };
        let difficulty = serde_json::to_value(d.difficulty)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "medium".into());
        Ok(ka_store::ProblemRow {
            id: outage_problem_id(&d.id),
            number: i64::from(d.number),
            set: 0,
            difficulty,
            title_en: d.title.clone(),
            title_es: d.title_es.clone(),
            teaches_en: String::new(),
            teaches_es: String::new(),
            tags: Vec::new(),
            nfr: serde_json::to_string(&problem.nfr).map_err(bad_content)?,
            def: serde_json::to_string(&problem).map_err(bad_content)?,
            statement_en: statement(
                &d.story,
                &format!(
                    "Fix the design so that it passes **{}**, a normal day, and the aftershock that follows.",
                    target.name
                ),
                "What happened",
                "Your task",
            ),
            statement_es: statement(
                &d.story_es,
                &format!(
                    "Arregla el diseño para que pase **{}**, un día normal y la réplica que viene después.",
                    target.name_es
                ),
                "Qué pasó",
                "Tu tarea",
            ),
            hints_en: doc.hints_en.clone(),
            hints_es: doc.hints_es.clone(),
            editorial_en: d.lesson.clone(),
            editorial_es: d.lesson_es.clone(),
            reference: doc.fixed.clone(),
            ref_cost: doc.ref_cost,
            ref_p95_ms: doc.ref_p95_ms,
            track: "core".into(),
            challenge_ratio: None,
        })
    }

    /// A replay's as-it-was design: where "Fix it" starts.
    pub(crate) async fn outage_as_it_was(&self, id: &str) -> Result<ka_core::Design> {
        let doc = self.outage_doc(id).await?;
        let mut d = crate::kbarch::parse(doc.as_it_was.as_bytes(), &self.registry)
            .map_err(|e| bad_content(e.message))?;
        d.problem_id = Some(outage_problem_id(id));
        Ok(d)
    }
}

#[cfg(test)]
mod tests {
    use ka_core::outage::OutageDef;

    use crate::testutil::temp_dir;

    #[tokio::test]
    async fn a_replay_is_told_judged_and_recorded_as_a_problem() {
        let def: OutageDef = toml::from_str(
            r#"
id = "tiny"
number = 1
difficulty = "easy"
title = "Tiny"
title_es = "Diminuto"
summary = "One app."
summary_es = "Una app."
story = "It fell over."
story_es = "Se cayó."
lesson = "Add another."
lesson_es = "Pon otra."
target_test = "normal"
kinds = ["app"]

[[step]]
snapshot = "as_it_was.kbarch"
title = "As it was"
title_es = "Como era"
caption = "One app."
caption_es = "Una app."

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
        let doc = ka_core::outage::OutageDoc {
            def,
            hints_en: vec!["a".into(), "b".into(), "c".into()],
            hints_es: vec!["a".into(), "b".into(), "c".into()],
            as_it_was: kbarch.clone(),
            fixed: kbarch,
            ref_cost: 70.0,
            ref_p95_ms: 6.0,
        };
        let app = crate::practice::tests::app_with(
            &temp_dir("outage"),
            vec![ka_store::DocRow {
                kind: "outage".into(),
                id: "tiny".into(),
                json: serde_json::to_string(&doc).unwrap(),
            }],
        )
        .await;
        let list = app.list_outages().await.unwrap();
        assert_eq!((list[0].id.as_str(), list[0].solved), ("tiny", false));
        let story = app.outage_story("tiny").await.unwrap();
        assert_eq!(
            story.walkthrough.def.problem.as_deref(),
            Some("outage--tiny")
        );
        assert_eq!(story.walkthrough.snapshots.len(), 2);
        let brief = app.problem_brief("outage--tiny").await.unwrap();
        assert!(
            brief.statement_en.contains("It fell over.")
                && brief.statement_en.contains("Normal day")
        );
        assert!(brief.statement_es.contains("Qué pasó"));
        assert_eq!(brief.editorial_en, None);
        // "Fix it" opens on the design as it was.
        assert_eq!(brief.draft.as_ref().map(|d| d.nodes.len()), Some(2));
        let (problem, ref_cost) = app.problem_for_judging("outage--tiny").await.unwrap();
        let j = crate::problems::judge_design(&problem, &fixed, &reg, ref_cost);
        assert_eq!(j.verdict, ka_judge::Verdict::Accepted);
        app.record_submission(&fixed, j, "2026-11-23")
            .await
            .unwrap();
        assert!(app.list_outages().await.unwrap()[0].solved);
        assert!(app.outage_story("tiny").await.unwrap().summary.solved);
        // A replay is not a pack's problem.
        assert_eq!(app.pack_name_of("outage--tiny").await.unwrap(), None);
    }
}

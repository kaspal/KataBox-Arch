//! Problems behind the commands: the catalogue with the user's progress, a problem's brief
//! (never a hidden test's definition), hints, drafts, submissions and progress counts.
//! Judging is pure and slow-ish, so `judge_design` takes no `&App`: the command runs it off
//! the async runtime, then `record_submission` stores it.

use ka_core::problem::{Difficulty, Nfr, ProblemDef};
use ka_core::{Design, Registry};
use ka_judge::{Judgement, Verdict};
use serde::Serialize;

use crate::{App, AppError, AppErrorCode, Result};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProblemSummary {
    pub id: String,
    pub number: u32,
    pub set: u32,
    pub difficulty: Difficulty,
    pub title_en: String,
    pub title_es: String,
    pub teaches_en: String,
    pub teaches_es: String,
    pub tags: Vec<String>,
    /// None until a submission; then "attempted" or "solved".
    pub status: Option<String>,
    pub best_score: Option<i64>,
    /// The cheapest Accepted submission's monthly cost.
    pub best_cost: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TestBrief {
    pub id: String,
    pub name: String,
    pub name_es: String,
    pub hidden: bool,
    pub duration_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RequestTypeBrief {
    pub id: String,
    pub name: Option<String>,
    pub name_es: Option<String>,
    pub share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExternalBrief {
    pub name: String,
    pub median_ms: f64,
    pub p99_ms: f64,
    pub rate_limit: Option<f64>,
}

/// What the Solve page shows. Hidden tests come with their names only; hints only up to the
/// unlocked tier; the editorial and the reference only once solved.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProblemBrief {
    pub summary: ProblemSummary,
    pub nfr: Nfr,
    pub statement_en: String,
    pub statement_es: String,
    pub request_types: Vec<RequestTypeBrief>,
    pub externals: Vec<ExternalBrief>,
    pub tests: Vec<TestBrief>,
    /// Unlocked tiers (0–3) and their text.
    pub hints_unlocked: i64,
    pub hints_en: Vec<String>,
    pub hints_es: Vec<String>,
    pub solved: bool,
    pub editorial_en: Option<String>,
    pub editorial_es: Option<String>,
    pub reference: Option<Design>,
    /// The reference's monthly cost and first-test p95, once solved (the cost vs speed chart).
    pub reference_point: Option<ReferencePoint>,
    pub draft: Option<Design>,
    pub runs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReferencePoint {
    pub cost_usd: f64,
    pub p95_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SubmitResult {
    pub submission_id: i64,
    pub judgement: Judgement,
    /// The problem was solved by this submission for the first time.
    pub first_solve: bool,
    pub best_score: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DifficultyCount {
    pub total: u32,
    pub attempted: u32,
    pub solved: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Progress {
    pub easy: DifficultyCount,
    pub medium: DifficultyCount,
    pub hard: DifficultyCount,
    pub streak: i64,
    /// The next unsolved problem in catalogue order, and the most recently attempted one.
    pub recommended: Option<String>,
    pub continue_with: Option<String>,
}

fn store(e: ka_store::StoreError) -> AppError {
    AppError::from(e)
}

fn bad_content(e: impl std::fmt::Display) -> AppError {
    AppError::new(AppErrorCode::Internal, format!("content.db: {e}"))
}

/// Judge `design` on `problem` (pure; the command runs it off the async runtime).
pub fn judge_design(
    problem: &ProblemDef,
    design: &Design,
    registry: &Registry,
    ref_cost: f64,
) -> Judgement {
    ka_judge::judge(problem, design, registry, ref_cost)
}

impl App {
    pub(crate) async fn problem_row(&self, id: &str) -> Result<ka_store::ProblemRow> {
        self.content()?
            .problem(id)
            .await
            .map_err(store)?
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("problem {id}")))
    }

    /// The full definition, hidden tests included (stays in Rust).
    pub async fn problem_def(&self, id: &str) -> Result<ProblemDef> {
        serde_json::from_str(&self.problem_row(id).await?.def).map_err(bad_content)
    }

    /// The definition and reference cost a judgement needs.
    pub async fn problem_for_judging(&self, id: &str) -> Result<(ProblemDef, f64)> {
        let row = self.problem_row(id).await?;
        let def = serde_json::from_str(&row.def).map_err(bad_content)?;
        Ok((def, row.ref_cost))
    }

    fn summary(
        row: &ka_store::ProblemRow,
        def: &ProblemDef,
        status: Option<&ka_store::ProblemStatus>,
        best_cost: Option<f64>,
    ) -> ProblemSummary {
        ProblemSummary {
            id: row.id.clone(),
            number: def.number,
            set: def.set,
            difficulty: def.difficulty,
            title_en: row.title_en.clone(),
            title_es: row.title_es.clone(),
            teaches_en: row.teaches_en.clone(),
            teaches_es: row.teaches_es.clone(),
            tags: row.tags.clone(),
            status: status.map(|s| s.status.clone()),
            best_score: status.map(|s| s.best_score),
            best_cost,
        }
    }

    async fn best_cost(&self, id: &str) -> Result<Option<f64>> {
        Ok(self
            .user
            .submissions(id)
            .await
            .map_err(store)?
            .iter()
            .filter(|s| s.verdict == "accepted")
            .map(|s| s.cost_usd)
            .min_by(f64::total_cmp))
    }

    /// The catalogue with the user's status and best score, in catalogue order.
    pub async fn list_problems(&self) -> Result<Vec<ProblemSummary>> {
        let rows = self.content()?.problems().await.map_err(store)?;
        let statuses = self.user.problem_statuses().await.map_err(store)?;
        let mut out = Vec::new();
        for row in &rows {
            let def: ProblemDef = serde_json::from_str(&row.def).map_err(bad_content)?;
            let status = statuses.iter().find(|s| s.problem_id == row.id);
            let best = if status.is_some() {
                self.best_cost(&row.id).await?
            } else {
                None
            };
            out.push(Self::summary(row, &def, status, best));
        }
        Ok(out)
    }

    pub async fn problem_brief(&self, id: &str) -> Result<ProblemBrief> {
        let row = self.problem_row(id).await?;
        let def: ProblemDef = serde_json::from_str(&row.def).map_err(bad_content)?;
        let statuses = self.user.problem_statuses().await.map_err(store)?;
        let status = statuses.iter().find(|s| s.problem_id == id);
        let solved = status.is_some_and(|s| s.status == "solved");
        let unlocked = self.user.hints_unlocked(id).await.map_err(store)?;
        let take = |h: &[String]| h.iter().take(unlocked as usize).cloned().collect();
        let draft = match self.user.draft(id).await.map_err(store)? {
            Some(json) => crate::kbarch::parse(json.as_bytes(), &self.registry).ok(),
            None => None,
        };
        let reference = if solved {
            crate::kbarch::parse(row.reference.as_bytes(), &self.registry).ok()
        } else {
            None
        };
        Ok(ProblemBrief {
            summary: Self::summary(&row, &def, status, self.best_cost(id).await?),
            nfr: def.nfr.clone(),
            statement_en: row.statement_en.clone(),
            statement_es: row.statement_es.clone(),
            request_types: def
                .request_types
                .iter()
                .map(|t| RequestTypeBrief {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    name_es: t.name_es.clone(),
                    share: t.share,
                })
                .collect(),
            externals: def
                .externals
                .iter()
                .map(|x| ExternalBrief {
                    name: x.name.clone(),
                    median_ms: x.median_ms,
                    p99_ms: x.p99_ms,
                    rate_limit: x.rate_limit,
                })
                .collect(),
            tests: def
                .tests
                .iter()
                .map(|t| TestBrief {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    name_es: t.name_es.clone(),
                    hidden: t.hidden,
                    duration_s: t.duration_s,
                })
                .collect(),
            hints_unlocked: unlocked,
            hints_en: take(&row.hints_en),
            hints_es: take(&row.hints_es),
            solved,
            editorial_en: solved.then(|| row.editorial_en.clone()),
            editorial_es: solved.then(|| row.editorial_es.clone()),
            reference,
            reference_point: solved.then_some(ReferencePoint {
                cost_usd: row.ref_cost,
                p95_ms: row.ref_p95_ms,
            }),
            draft,
            runs: self.user.run_count(id).await.map_err(store)?,
        })
    }

    pub async fn unlock_hint(&self, id: &str, tier: i64) -> Result<()> {
        self.problem_row(id).await?;
        if !(1..=3).contains(&tier) {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                format!("hint tier {tier}"),
            ));
        }
        self.user.unlock_hint(id, tier).await.map_err(store)
    }

    pub async fn save_draft(&self, id: &str, design: &Design) -> Result<()> {
        self.problem_row(id).await?;
        let bytes = crate::kbarch::to_bytes(design, &self.registry)?;
        self.user
            .save_draft(id, &String::from_utf8_lossy(&bytes))
            .await
            .map_err(store)
    }

    /// A problem's submissions, newest first (for the history and the cost/speed chart).
    pub async fn submissions(&self, id: &str) -> Result<Vec<ka_store::SubmissionSummary>> {
        self.user.submissions(id).await.map_err(store)
    }

    pub async fn record_problem_run(&self, id: &str) -> Result<()> {
        self.user.record_run(id).await.map_err(store)
    }

    /// Store a judged submission (the design as submitted, the judgement) and update the
    /// problem's status. `local_day` is the user's date, `YYYY-MM-DD`.
    pub async fn record_submission(
        &self,
        design: &Design,
        judgement: Judgement,
        local_day: &str,
    ) -> Result<SubmitResult> {
        let before = self.user.problem_statuses().await.map_err(store)?;
        let was_solved = before
            .iter()
            .any(|s| s.problem_id == judgement.problem_id && s.status == "solved");
        let solved = judgement.verdict == Verdict::Accepted;
        let verdict = serde_json::to_value(judgement.verdict)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        let p95 = judgement.tests.first().map_or(0.0, |t| t.p95_ms);
        let design_json = serde_json::to_string(design)
            .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
        let results = serde_json::to_string(&judgement)
            .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
        let id = self
            .user
            .record_submission(&ka_store::NewSubmission {
                problem_id: judgement.problem_id.clone(),
                design: design_json,
                verdict,
                score: i64::from(judgement.score.total),
                correctness: i64::from(judgement.score.correctness),
                cost_score: i64::from(judgement.score.cost),
                latency_score: i64::from(judgement.score.latency),
                cost_usd: judgement.cost_month,
                p95_ms: p95,
                results,
                solved,
                local_day: local_day.into(),
            })
            .await
            .map_err(|e| match e {
                ka_store::StoreError::Corrupt(m) => AppError::new(AppErrorCode::InvalidInput, m),
                other => store(other),
            })?;
        let best = self
            .user
            .problem_statuses()
            .await
            .map_err(store)?
            .into_iter()
            .find(|s| s.problem_id == judgement.problem_id)
            .map_or(0, |s| s.best_score);
        Ok(SubmitResult {
            submission_id: id,
            first_solve: solved && !was_solved,
            best_score: best,
            judgement,
        })
    }

    /// Solved and attempted per difficulty, the streak, and what to do next.
    pub async fn progress(&self, today: &str) -> Result<Progress> {
        let all = self.list_problems().await?;
        let mut p = Progress {
            easy: DifficultyCount::default(),
            medium: DifficultyCount::default(),
            hard: DifficultyCount::default(),
            streak: self.user.streak(today).await.map_err(|e| match e {
                ka_store::StoreError::Corrupt(m) => AppError::new(AppErrorCode::InvalidInput, m),
                other => store(other),
            })?,
            recommended: all
                .iter()
                .find(|s| s.status.as_deref() != Some("solved"))
                .map(|s| s.id.clone()),
            continue_with: None,
        };
        for s in &all {
            let c = match s.difficulty {
                Difficulty::Easy => &mut p.easy,
                Difficulty::Medium => &mut p.medium,
                Difficulty::Hard => &mut p.hard,
            };
            c.total += 1;
            match s.status.as_deref() {
                Some("solved") => c.solved += 1,
                Some(_) => c.attempted += 1,
                None => {}
            }
        }
        let statuses = self.user.problem_statuses().await.map_err(store)?;
        p.continue_with = statuses
            .iter()
            .filter(|s| s.status == "attempted")
            .max_by_key(|s| s.updated_at)
            .map(|s| s.problem_id.clone());
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;
    use crate::live::FrameSink;
    use crate::runs::{ProblemTest, RunRequest};
    use crate::testutil::temp_dir;
    use crate::{AppPaths, SandboxPreset};
    use ka_core::{ComponentKind as K, Edge, Flavour, Node, Point, PropValue, Protocol};

    const PROBLEM: &str = r#"
id = "demo"
number = 1
set = 1
difficulty = "easy"
title = "Demo"
title_es = "Demo"
teaches = "Scaling"
teaches_es = "Escalar"

[nfr]
peak_rps = 2000
p95_ms = 100
availability = 0.999
budget_usd = 2000

[[request_types]]
id = "read"
share = 1.0
route = ["compute", "datastore"]

[[tests]]
id = "normal"
name = "Normal day"
name_es = "Un dia normal"
duration_s = 10

[[tests]]
id = "dbdies"
name = "Database dies"
name_es = "La base de datos cae"
hidden = true
duration_s = 10
chaos = [{ latency = { kind = "sql", add_ms = 1, at = 5, for = 2 } }]

[pass]
error_rate = 0.001
"#;

    fn node(id: &str, kind: K, flavour: Flavour, props: &[(&str, f64)]) -> Node {
        Node {
            id: id.into(),
            kind,
            flavour,
            label: String::new(),
            position: Point { x: 0.0, y: 0.0 },
            props: props
                .iter()
                .map(|(k, v)| ((*k).to_owned(), PropValue::Number(*v)))
                .collect(),
            group_id: None,
        }
    }

    fn design() -> Design {
        let mut d = Design::empty("d", "D");
        d.problem_id = Some("demo".into());
        d.nodes = vec![
            node("client", K::Client, Flavour::Web, &[]),
            node("app", K::App, Flavour::Go, &[("instances", 4.0)]),
            node("db", K::Sql, Flavour::Postgresql, &[]),
        ];
        let e = |id: &str, from: &str, to: &str, protocol| Edge {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            protocol,
        };
        d.edges = vec![
            e("e1", "client", "app", Protocol::Http),
            e("e2", "app", "db", Protocol::Query),
        ];
        d
    }

    async fn app() -> App {
        let dir = temp_dir("problems");
        let def: ProblemDef = toml::from_str(PROBLEM).unwrap();
        let r = ka_core::registry();
        let reference = String::from_utf8(crate::kbarch::to_bytes(&design(), &r).unwrap()).unwrap();
        let row = ka_store::ProblemRow {
            id: "demo".into(),
            number: 1,
            set: 1,
            difficulty: "easy".into(),
            title_en: "Demo".into(),
            title_es: "Demo".into(),
            teaches_en: "Scaling".into(),
            teaches_es: "Escalar".into(),
            tags: vec!["scaling".into()],
            nfr: serde_json::to_string(&def.nfr).unwrap(),
            def: serde_json::to_string(&def).unwrap(),
            statement_en: "# Demo".into(),
            statement_es: "# Demo".into(),
            hints_en: vec!["one".into(), "two".into(), "three".into()],
            hints_es: vec!["uno".into(), "dos".into(), "tres".into()],
            editorial_en: "Editorial".into(),
            editorial_es: "Editorial".into(),
            reference,
            ref_cost: 500.0,
            ref_p95_ms: 30.0,
        };
        let content = dir.join("content.db");
        ka_store::write_content(
            &content,
            &ka_store::ContentData {
                meta: vec![],
                problems: vec![row],
                samples: vec![],
                guide: vec![],
                concepts: vec![],
                patterns: vec![],
                decide: vec![],
                glossary: vec![],
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
    async fn a_problem_from_brief_to_solved() {
        let app = app().await;
        let list = app.list_problems().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, None);

        let brief = app.problem_brief("demo").await.unwrap();
        assert_eq!(brief.tests.len(), 2);
        assert!(brief.tests[1].hidden);
        assert_eq!(brief.hints_unlocked, 0);
        assert!(brief.hints_en.is_empty());
        assert!(brief.reference.is_none() && brief.editorial_en.is_none());
        assert!(brief.reference_point.is_none());
        // The brief carries no hidden-test definition.
        let json = serde_json::to_string(&brief).unwrap();
        assert!(!json.contains("chaos") && !json.contains("latency"));

        app.unlock_hint("demo", 2).await.unwrap();
        app.save_draft("demo", &design()).await.unwrap();
        let brief = app.problem_brief("demo").await.unwrap();
        assert_eq!(brief.hints_en, vec!["one", "two"]);
        assert_eq!(brief.draft.as_ref().map(|d| d.nodes.len()), Some(3));

        let (def, ref_cost) = app.problem_for_judging("demo").await.unwrap();
        let j = judge_design(&def, &design(), app.registry(), ref_cost);
        assert_eq!(j.verdict, Verdict::Accepted, "{j:#?}");
        let res = app
            .record_submission(&design(), j, "2026-11-22")
            .await
            .unwrap();
        assert!(res.first_solve);
        assert!(res.best_score > 0);
        let again = app
            .record_submission(&design(), res.judgement.clone(), "2026-11-22")
            .await
            .unwrap();
        assert!(!again.first_solve);

        let brief = app.problem_brief("demo").await.unwrap();
        assert!(brief.solved);
        assert!(brief.editorial_en.is_some() && brief.reference.is_some());
        assert_eq!(
            brief.reference_point,
            Some(ReferencePoint {
                cost_usd: 500.0,
                p95_ms: 30.0
            })
        );
        let list = app.list_problems().await.unwrap();
        assert_eq!(list[0].status.as_deref(), Some("solved"));
        assert!(list[0].best_cost.is_some());
        let p = app.progress("2026-11-22").await.unwrap();
        assert_eq!((p.easy.total, p.easy.solved), (1, 1));
        assert_eq!(p.streak, 1);
        assert_eq!(p.recommended, None);
        assert!(app.problem_brief("nope").await.is_err());
    }

    struct Collect(std::sync::mpsc::Sender<Vec<u8>>);
    impl FrameSink for Collect {
        fn send(&mut self, frame: Vec<u8>) -> bool {
            self.0.send(frame).is_ok()
        }
    }

    #[tokio::test]
    async fn a_hidden_test_replays_without_showing_its_chaos() {
        let app = app().await;
        let (tx, _rx) = channel();
        let req = |test: &str| RunRequest {
            preset: SandboxPreset::Normal,
            problem: Some(ProblemTest {
                problem_id: "demo".into(),
                test_id: test.into(),
            }),
            speed: 1.0,
            chaos: Vec::new(),
            start_s: 3.0,
        };
        let info = app
            .start_run(&design(), req("dbdies"), Collect(tx.clone()))
            .await
            .unwrap();
        assert!(info.chaos.is_empty(), "hidden chaos stays in Rust");
        assert_eq!(info.duration_s, 10.0);
        assert_eq!(info.start_s, 3.0);
        assert_eq!(
            app.start_run(&design(), req("missing"), Collect(tx))
                .await
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
    }

    #[tokio::test]
    async fn without_a_catalogue_problems_say_so() {
        let app = App::open(AppPaths {
            user_db: temp_dir("nocontent").join("user.db"),
            dev_data: true,
            content_db: None,
        })
        .await
        .unwrap();
        assert_eq!(
            app.list_problems().await.unwrap_err().code,
            AppErrorCode::ContentMissing
        );
    }
}

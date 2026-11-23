//! Practice content from `content.db` (M5): the numbers cheat sheet, and from later steps
//! estimation drills, Find-the-flaw drills and learning paths.

use ka_core::estimate::{Drill, DrillSet, EstimateCheck};
use ka_core::flaw::{FlawDoc, FlawSummary, flaw_problem_id};
use ka_core::numbers::NumberFigure;
use ka_core::paths::{PathDef, PathItem, PathItemKind, PathView, parse_item};
use ka_core::quiz::{QuizItem, QuizQuestion, QuizResult, question_id, split_question_id};
use ka_core::review::{Card, Day, Outcome};
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

    /// KataBox Learn topics and problems related to an Arch page (`guide:<kind>`,
    /// `pattern:<id>`, `problem:<id>`), titles only (M6.7). Empty when there are none.
    pub async fn katabox_links(
        &self,
        target: &str,
    ) -> Result<Vec<ka_core::crosslinks::KataboxLink>> {
        match self
            .content()?
            .doc("crosslinks", target)
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
            track: "core".into(),
            challenge_ratio: None,
        })
    }

    /// A page's quiz questions with their answers (stays in Rust). `page` is
    /// `guide:<kind>` or `pattern:<id>`.
    async fn quiz_questions(&self, page: &str) -> Result<Vec<QuizQuestion>> {
        let not_found = || AppError::new(AppErrorCode::NotFound, format!("quiz {page}"));
        let content = self.content()?;
        match page.split_once(':') {
            Some(("guide", kind)) => {
                let row = content
                    .guide_entry(kind)
                    .await
                    .map_err(AppError::from)?
                    .ok_or_else(not_found)?;
                let e: ka_core::guide::GuideEntry =
                    serde_json::from_str(&row.entry).map_err(bad_content)?;
                Ok(e.quiz)
            }
            Some(("pattern", id)) => {
                let row = content
                    .patterns()
                    .await
                    .map_err(AppError::from)?
                    .into_iter()
                    .find(|r| r.id == id)
                    .ok_or_else(not_found)?;
                let e: ka_core::pattern::PatternEntry =
                    serde_json::from_str(&row.entry).map_err(bad_content)?;
                Ok(e.quiz)
            }
            _ => Err(not_found()),
        }
    }

    fn quiz_item(page: &str, n: usize, q: &QuizQuestion) -> QuizItem {
        QuizItem {
            id: question_id(page, n),
            q: q.q.clone(),
            q_es: q.q_es.clone(),
            options: q.options.clone(),
            options_es: q.options_es.clone(),
        }
    }

    /// A page's questions, without their answers.
    pub async fn quiz(&self, page: &str) -> Result<Vec<QuizItem>> {
        Ok(self
            .quiz_questions(page)
            .await?
            .iter()
            .enumerate()
            .map(|(i, q)| Self::quiz_item(page, i + 1, q))
            .collect())
    }

    /// Judge an answer (`choice` is the option's index), record it, and move the question
    /// through the review queue (ADR 0010). `today` is the user's local date. Once all of a
    /// page's questions have been answered, the page's quiz counts as done (`quiz` read).
    pub async fn answer_quiz(&self, id: &str, choice: u32, today: &str) -> Result<QuizResult> {
        let bad = |m: String| AppError::new(AppErrorCode::InvalidInput, m);
        let day = Day::parse(today).ok_or_else(|| bad(format!("bad local day {today:?}")))?;
        let (page, n) = split_question_id(id).ok_or_else(|| bad(format!("bad question {id:?}")))?;
        let questions = self.quiz_questions(page).await?;
        let q = questions
            .get(n - 1)
            .ok_or_else(|| AppError::new(AppErrorCode::NotFound, format!("question {id}")))?;
        let correct = choice == q.answer;
        self.user
            .record_quiz_attempt(id, correct, today)
            .await
            .map_err(AppError::from)?;
        let card = self
            .user
            .review_card(id)
            .await
            .map_err(AppError::from)?
            .and_then(|r| {
                Some(Card {
                    step: u8::try_from(r.step).ok()?,
                    lapses: u32::try_from(r.lapses).ok()?,
                    due: Day::parse(&r.due_on)?,
                    last: Day::parse(&r.last_on)?,
                })
            });
        let next = match ka_core::review::answer(card.as_ref(), correct, day) {
            Outcome::None => None,
            Outcome::Card(c) => Some(c),
        };
        let row = next.as_ref().map(|c| ka_store::ReviewCardRow {
            question_id: id.to_owned(),
            step: i64::from(c.step),
            lapses: i64::from(c.lapses),
            due_on: c.due.iso(),
            last_on: c.last.iso(),
        });
        self.user
            .put_review_card(id, row.as_ref())
            .await
            .map_err(AppError::from)?;
        let answered = self
            .user
            .answered_questions()
            .await
            .map_err(AppError::from)?;
        if (1..=questions.len()).all(|k| answered.contains(&question_id(page, k))) {
            self.user
                .set_read("quiz", page, true)
                .await
                .map_err(AppError::from)?;
        }
        Ok(QuizResult {
            correct,
            answer: q.answer,
            why: q.why.clone(),
            why_es: q.why_es.clone(),
            due_on: next.map(|c| c.due.iso()),
        })
    }

    /// The questions due for review on `today`, in review order, at most `limit`.
    pub async fn reviews_due(&self, today: &str, limit: usize) -> Result<Vec<QuizItem>> {
        let day = Day::parse(today).ok_or_else(|| {
            AppError::new(
                AppErrorCode::InvalidInput,
                format!("bad local day {today:?}"),
            )
        })?;
        let mut due: Vec<(String, Card, ())> = self
            .user
            .review_cards()
            .await
            .map_err(AppError::from)?
            .into_iter()
            .filter_map(|r| {
                let card = Card {
                    step: u8::try_from(r.step).ok()?,
                    lapses: u32::try_from(r.lapses).ok()?,
                    due: Day::parse(&r.due_on)?,
                    last: Day::parse(&r.last_on)?,
                };
                (card.due <= day).then_some((r.question_id, card, ()))
            })
            .collect();
        ka_core::review::order(&mut due);
        let mut out = Vec::new();
        for (id, _, ()) in due.into_iter().take(limit) {
            let Some((page, n)) = split_question_id(&id) else {
                continue;
            };
            // Content may have changed since the card was made: skip what's gone.
            if let Ok(qs) = self.quiz_questions(page).await
                && let Some(q) = qs.get(n - 1)
            {
                out.push(Self::quiz_item(page, n, q));
            }
        }
        Ok(out)
    }

    /// Record that a concept card was played to its end (a learning-path item).
    pub async fn set_concept_played(&self, kind: ka_core::ComponentKind) -> Result<()> {
        self.user
            .set_read("concept", kind.id(), true)
            .await
            .map_err(AppError::from)
    }

    /// The learning paths in order, each item marked done from what the user has already
    /// done: Guide entries, patterns and concept cards read or played; problems and flaw
    /// drills solved; a problem's estimation drills all answered within their band.
    pub async fn paths(&self) -> Result<Vec<PathView>> {
        let rows = self.content()?.docs("path").await.map_err(AppError::from)?;
        let mut defs = rows
            .iter()
            .map(|r| serde_json::from_str::<PathDef>(&r.json).map_err(bad_content))
            .collect::<Result<Vec<_>>>()?;
        defs.sort_by_key(|p| p.number);
        let read =
            |kind: &'static str| async move { self.user.reads(kind).await.map_err(AppError::from) };
        let guides = read("guide").await?;
        let patterns_read = read("pattern").await?;
        let concepts = read("concept").await?;
        let quizzes = read("quiz").await?;
        let statuses = self.user.problem_statuses().await.map_err(AppError::from)?;
        let solved = |pid: &str| {
            statuses
                .iter()
                .any(|s| s.problem_id == pid && s.status == "solved")
        };
        let estimates = self.estimate_statuses().await?;
        let sets = self.drill_sets().await?;
        let problems = self.content()?.problems().await.map_err(AppError::from)?;
        let patterns = self.list_patterns().await?;
        let flaws = self.list_flaws().await?;
        let problem_title = |id: &str| {
            problems
                .iter()
                .find(|p| p.id == id)
                .map(|p| (p.title_en.clone(), p.title_es.clone()))
        };
        Ok(defs
            .into_iter()
            .map(|d| {
                let items = d
                    .items
                    .iter()
                    .filter_map(|item| {
                        let (kind, target) = parse_item(item)?;
                        let (title, done) = match kind {
                            PathItemKind::Guide => (None, guides.iter().any(|g| g == target)),
                            PathItemKind::Concept => (None, concepts.iter().any(|c| c == target)),
                            PathItemKind::Pattern => (
                                patterns
                                    .iter()
                                    .find(|p| p.id == target)
                                    .map(|p| (p.title.clone(), p.title_es.clone())),
                                patterns_read.iter().any(|p| p == target),
                            ),
                            PathItemKind::Problem => (problem_title(target), solved(target)),
                            PathItemKind::Estimate => {
                                let all = sets
                                    .iter()
                                    .find(|s| s.problem_id == target)
                                    .map(|s| {
                                        s.drills.iter().map(|x| x.id.clone()).collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                let done = !all.is_empty()
                                    && all.iter().all(|id| {
                                        estimates.iter().any(|e| {
                                            e.problem_id == target && &e.drill_id == id && e.done
                                        })
                                    });
                                (problem_title(target), done)
                            }
                            PathItemKind::Flaw => (
                                flaws
                                    .iter()
                                    .find(|f| f.id == target)
                                    .map(|f| (f.title.clone(), f.title_es.clone())),
                                solved(&flaw_problem_id(target)),
                            ),
                            PathItemKind::Quiz => (
                                // A pattern quiz is named after its pattern; a Guide quiz by
                                // the UI, from the component kind.
                                target.strip_prefix("pattern:").and_then(|id| {
                                    patterns
                                        .iter()
                                        .find(|p| p.id == id)
                                        .map(|p| (p.title.clone(), p.title_es.clone()))
                                }),
                                quizzes.iter().any(|q| q == target),
                            ),
                        };
                        Some(PathItem {
                            kind,
                            target: target.to_owned(),
                            title: title.as_ref().map(|t| t.0.clone()),
                            title_es: title.map(|t| t.1),
                            done,
                        })
                    })
                    .collect();
                PathView {
                    id: d.id,
                    number: d.number,
                    title: d.title,
                    title_es: d.title_es,
                    blurb: d.blurb,
                    blurb_es: d.blurb_es,
                    items,
                }
            })
            .collect())
    }

    /// A drill's broken design: where the user starts.
    pub(crate) async fn flaw_broken(&self, id: &str) -> Result<ka_core::Design> {
        let doc = self.flaw_doc(id).await?;
        let mut d = crate::kbarch::parse(doc.broken.as_bytes(), &self.registry)
            .map_err(|e| bad_content(e.message))?;
        d.problem_id = Some(flaw_problem_id(id));
        Ok(d)
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
            packs_dir: None,
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
        // The drill opens on its broken design until the user has a draft of their own.
        assert_eq!(brief.draft.as_ref().map(|d| d.nodes.len()), Some(2));
        // An empty draft (autosaved from an empty canvas) still opens on the broken design...
        let mut empty = fixed.clone();
        empty.nodes.clear();
        empty.edges.clear();
        app.save_draft("flaw--tiny", &empty).await.unwrap();
        let brief = app.problem_brief("flaw--tiny").await.unwrap();
        assert_eq!(brief.draft.as_ref().map(|d| d.nodes.len()), Some(2));
        // ...and the user's own work is kept.
        app.save_draft("flaw--tiny", &fixed).await.unwrap();
        let brief = app.problem_brief("flaw--tiny").await.unwrap();
        assert_eq!(
            brief.draft.as_ref().map(|d| d.nodes.len()),
            Some(fixed.nodes.len())
        );
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

    #[tokio::test]
    async fn path_progress_comes_from_what_was_done() {
        let path = PathDef {
            id: "foundations".into(),
            number: 1,
            title: "F".into(),
            title_es: "F".into(),
            blurb: "b".into(),
            blurb_es: "b".into(),
            items: vec![
                "guide:cache".into(),
                "concept:cache".into(),
                "guide:lb".into(),
                "flaw:tiny".into(),
            ],
        };
        let app = app_with(
            &temp_dir("paths"),
            vec![ka_store::DocRow {
                kind: "path".into(),
                id: "foundations".into(),
                json: serde_json::to_string(&path).unwrap(),
            }],
        )
        .await;
        app.set_guide_read(ka_core::ComponentKind::Cache, true)
            .await
            .unwrap();
        app.set_concept_played(ka_core::ComponentKind::Cache)
            .await
            .unwrap();
        let views = app.paths().await.unwrap();
        let done: Vec<bool> = views[0].items.iter().map(|i| i.done).collect();
        assert_eq!(done, [true, true, false, false]);
        assert_eq!(views[0].items[3].kind, PathItemKind::Flaw);
        assert_eq!(
            views[0].items[3].title, None,
            "a flaw that isn't in the content"
        );
    }
}

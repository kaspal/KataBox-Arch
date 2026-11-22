//! Practice progress in `user.db` (migration 0002): submissions, per-problem status, drafts,
//! hint unlocks, run counts and the daily streak.

use serde::Serialize;
use sqlx::Row;

use crate::user::UserDb;
use crate::{Result, StoreError, now_ms};

/// A judged submission to record. `results` is the judgement JSON; `design` the `.kbarch`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSubmission {
    pub problem_id: String,
    pub design: String,
    pub verdict: String,
    pub score: i64,
    pub correctness: i64,
    pub cost_score: i64,
    pub latency_score: i64,
    pub cost_usd: f64,
    pub p95_ms: f64,
    pub results: String,
    /// Every test Accepted.
    pub solved: bool,
    /// The user's local date, `YYYY-MM-DD` (the UI knows the time zone).
    pub local_day: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SubmissionSummary {
    pub id: i64,
    pub problem_id: String,
    pub created_at: i64,
    pub verdict: String,
    pub score: i64,
    pub correctness: i64,
    pub cost_score: i64,
    pub latency_score: i64,
    pub cost_usd: f64,
    pub p95_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Submission {
    pub summary: SubmissionSummary,
    pub design: String,
    pub results: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ProblemStatus {
    pub problem_id: String,
    /// "attempted" or "solved".
    pub status: String,
    pub best_score: i64,
    pub best_submission_id: Option<i64>,
    pub first_solved_at: Option<i64>,
    pub updated_at: i64,
}

/// Days since 1970-01-01 of a `YYYY-MM-DD` date (proleptic Gregorian).
pub fn day_number(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn summary(r: &sqlx::sqlite::SqliteRow) -> Result<SubmissionSummary> {
    Ok(SubmissionSummary {
        id: r.try_get("id")?,
        problem_id: r.try_get("problem_id")?,
        created_at: r.try_get("created_at")?,
        verdict: r.try_get("verdict")?,
        score: r.try_get("score")?,
        correctness: r.try_get("correctness")?,
        cost_score: r.try_get("cost_score")?,
        latency_score: r.try_get("latency_score")?,
        cost_usd: r.try_get("cost_usd")?,
        p95_ms: r.try_get("p95_ms")?,
    })
}

const SUMMARY_COLUMNS: &str = "id, problem_id, created_at, verdict, score, correctness, \
    cost_score, latency_score, cost_usd, p95_ms";

impl UserDb {
    /// Store a submission and update the problem's status (solved once any submission is
    /// Accepted; the best score and its submission) and today's activity. Returns its id.
    pub async fn record_submission(&self, s: &NewSubmission) -> Result<i64> {
        if day_number(&s.local_day).is_none() {
            return Err(StoreError::Corrupt(format!(
                "bad local day {:?}",
                s.local_day
            )));
        }
        let now = now_ms();
        let mut tx = self.pool.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO submissions (problem_id, created_at, design, verdict, score, correctness,
             cost_score, latency_score, cost_usd, p95_ms, results)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&s.problem_id)
        .bind(now)
        .bind(&s.design)
        .bind(&s.verdict)
        .bind(s.score)
        .bind(s.correctness)
        .bind(s.cost_score)
        .bind(s.latency_score)
        .bind(s.cost_usd)
        .bind(s.p95_ms)
        .bind(&s.results)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO problem_status (problem_id, status, best_score, best_submission_id,
                 first_solved_at, updated_at)
             VALUES (?1, CASE WHEN ?2 THEN 'solved' ELSE 'attempted' END, ?3, ?4,
                 CASE WHEN ?2 THEN ?5 END, ?5)
             ON CONFLICT (problem_id) DO UPDATE SET
                 status = CASE WHEN ?2 OR status = 'solved' THEN 'solved' ELSE 'attempted' END,
                 best_submission_id = CASE WHEN ?3 > best_score THEN ?4 ELSE best_submission_id END,
                 best_score = max(best_score, ?3),
                 first_solved_at = COALESCE(first_solved_at, CASE WHEN ?2 THEN ?5 END),
                 updated_at = ?5",
        )
        .bind(&s.problem_id)
        .bind(s.solved)
        .bind(s.score)
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT OR IGNORE INTO activity_days (day) VALUES (?)")
            .bind(&s.local_day)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// A problem's submissions, newest first.
    pub async fn submissions(&self, problem_id: &str) -> Result<Vec<SubmissionSummary>> {
        let rows = sqlx::query(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM submissions WHERE problem_id = ?
             ORDER BY created_at DESC, id DESC"
        ))
        .bind(problem_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(summary).collect()
    }

    pub async fn submission(&self, id: i64) -> Result<Option<Submission>> {
        let row = sqlx::query(&format!(
            "SELECT {SUMMARY_COLUMNS}, design, results FROM submissions WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| {
            Ok(Submission {
                summary: summary(&r)?,
                design: r.try_get("design")?,
                results: r.try_get("results")?,
            })
        })
        .transpose()
    }

    pub async fn problem_statuses(&self) -> Result<Vec<ProblemStatus>> {
        let rows = sqlx::query(
            "SELECT problem_id, status, best_score, best_submission_id, first_solved_at,
             updated_at FROM problem_status ORDER BY problem_id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                Ok(ProblemStatus {
                    problem_id: r.try_get("problem_id")?,
                    status: r.try_get("status")?,
                    best_score: r.try_get("best_score")?,
                    best_submission_id: r.try_get("best_submission_id")?,
                    first_solved_at: r.try_get("first_solved_at")?,
                    updated_at: r.try_get("updated_at")?,
                })
            })
            .collect()
    }

    pub async fn save_draft(&self, problem_id: &str, design: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO problem_drafts (problem_id, design, updated_at) VALUES (?, ?, ?)
             ON CONFLICT (problem_id) DO UPDATE SET design = excluded.design,
                 updated_at = excluded.updated_at",
        )
        .bind(problem_id)
        .bind(design)
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn draft(&self, problem_id: &str) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT design FROM problem_drafts WHERE problem_id = ?")
                .bind(problem_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Reveal hint `tier` (1–3). Tiers unlock in order: asking for 3 also unlocks 1 and 2.
    pub async fn unlock_hint(&self, problem_id: &str, tier: i64) -> Result<()> {
        if !(1..=3).contains(&tier) {
            return Err(StoreError::Corrupt(format!("hint tier {tier}")));
        }
        let now = now_ms();
        for t in 1..=tier {
            sqlx::query(
                "INSERT OR IGNORE INTO hint_unlocks (problem_id, tier, at) VALUES (?, ?, ?)",
            )
            .bind(problem_id)
            .bind(t)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// The highest unlocked hint tier (0: none).
    pub async fn hints_unlocked(&self, problem_id: &str) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(max(tier), 0) FROM hint_unlocks WHERE problem_id = ?",
        )
        .bind(problem_id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn record_run(&self, problem_id: &str) -> Result<()> {
        sqlx::query("INSERT INTO runs (problem_id, at) VALUES (?, ?)")
            .bind(problem_id)
            .bind(now_ms())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn run_count(&self, problem_id: &str) -> Result<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM runs WHERE problem_id = ?")
                .bind(problem_id)
                .fetch_one(&self.pool)
                .await?,
        )
    }

    /// Consecutive days with a submission, ending today or yesterday (`today` is the user's
    /// local date, `YYYY-MM-DD`).
    pub async fn streak(&self, today: &str) -> Result<i64> {
        let Some(today) = day_number(today) else {
            return Err(StoreError::Corrupt(format!("bad local day {today:?}")));
        };
        let days: Vec<String> =
            sqlx::query_scalar("SELECT day FROM activity_days ORDER BY day DESC")
                .fetch_all(&self.pool)
                .await?;
        let mut numbers: Vec<i64> = days.iter().filter_map(|d| day_number(d)).collect();
        numbers.sort_unstable_by(|a, b| b.cmp(a));
        numbers.dedup();
        let mut expect = match numbers.first() {
            Some(&d) if d == today || d == today - 1 => d,
            _ => return Ok(0),
        };
        let mut streak = 0;
        for d in numbers {
            if d != expect {
                break;
            }
            streak += 1;
            expect -= 1;
        }
        Ok(streak)
    }
}

/// Read marks (user.db v3): Guide entries now, patterns later.
impl UserDb {
    /// Mark `id` of `kind` ("guide", "pattern") as read (or unread).
    pub async fn set_read(&self, kind: &str, id: &str, read: bool) -> Result<()> {
        if read {
            sqlx::query("INSERT OR IGNORE INTO reads (kind, id, at) VALUES (?, ?, ?)")
                .bind(kind)
                .bind(id)
                .bind(now_ms())
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("DELETE FROM reads WHERE kind = ? AND id = ?")
                .bind(kind)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    /// The ids of `kind` marked read, sorted.
    pub async fn reads(&self, kind: &str) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar("SELECT id FROM reads WHERE kind = ? ORDER BY id")
                .bind(kind)
                .fetch_all(&self.pool)
                .await?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::temp_dir;

    fn sub(problem: &str, score: i64, solved: bool, day: &str) -> NewSubmission {
        NewSubmission {
            problem_id: problem.into(),
            design: "{}".into(),
            verdict: if solved { "accepted" } else { "slo_breach" }.into(),
            score,
            correctness: score.min(70),
            cost_score: 0,
            latency_score: 0,
            cost_usd: 1200.0,
            p95_ms: 42.0,
            results: "{\"tests\":[]}".into(),
            solved,
            local_day: day.into(),
        }
    }

    #[test]
    fn day_numbers() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2000-03-01"), Some(11_017));
        assert_eq!(
            day_number("2026-11-22").unwrap() - day_number("2026-11-21").unwrap(),
            1
        );
        assert_eq!(
            day_number("2024-03-01").unwrap() - day_number("2024-02-28").unwrap(),
            2
        );
        assert_eq!(day_number("2026-13-01"), None);
        assert_eq!(day_number("nope"), None);
    }

    #[tokio::test]
    async fn submissions_track_status_best_score_and_first_solve() {
        let db = UserDb::open(&temp_dir("progress").join("user.db"))
            .await
            .unwrap();
        let a = db
            .record_submission(&sub("url", 40, false, "2026-11-20"))
            .await
            .unwrap();
        let s = &db.problem_statuses().await.unwrap()[0];
        assert_eq!(
            (s.status.as_str(), s.best_score, s.best_submission_id),
            ("attempted", 40, Some(a))
        );
        assert!(s.first_solved_at.is_none());
        let b = db
            .record_submission(&sub("url", 85, true, "2026-11-21"))
            .await
            .unwrap();
        let c = db
            .record_submission(&sub("url", 60, false, "2026-11-21"))
            .await
            .unwrap();
        let s = &db.problem_statuses().await.unwrap()[0];
        assert_eq!(s.status, "solved", "a later failure doesn't unsolve");
        assert_eq!((s.best_score, s.best_submission_id), (85, Some(b)));
        let solved_at = s.first_solved_at.unwrap();
        let list = db.submissions("url").await.unwrap();
        assert_eq!(list.iter().map(|x| x.id).collect::<Vec<_>>(), [c, b, a]);
        let full = db.submission(b).await.unwrap().unwrap();
        assert_eq!(full.results, "{\"tests\":[]}");
        db.record_submission(&sub("url", 90, true, "2026-11-22"))
            .await
            .unwrap();
        assert_eq!(
            db.problem_statuses().await.unwrap()[0].first_solved_at,
            Some(solved_at)
        );
        assert!(
            db.record_submission(&sub("url", 1, false, "22/11/2026"))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn drafts_hints_runs_and_streaks() {
        let db = UserDb::open(&temp_dir("progress2").join("user.db"))
            .await
            .unwrap();
        assert_eq!(db.draft("url").await.unwrap(), None);
        db.save_draft("url", "{\"v\":1}").await.unwrap();
        db.save_draft("url", "{\"v\":2}").await.unwrap();
        assert_eq!(db.draft("url").await.unwrap().as_deref(), Some("{\"v\":2}"));
        assert_eq!(db.hints_unlocked("url").await.unwrap(), 0);
        db.unlock_hint("url", 2).await.unwrap();
        assert_eq!(db.hints_unlocked("url").await.unwrap(), 2);
        assert!(db.unlock_hint("url", 4).await.is_err());
        db.record_run("url").await.unwrap();
        db.record_run("url").await.unwrap();
        assert_eq!(db.run_count("url").await.unwrap(), 2);
        assert_eq!(db.streak("2026-11-22").await.unwrap(), 0);
        for day in ["2026-11-18", "2026-11-20", "2026-11-21"] {
            db.record_submission(&sub("url", 10, false, day))
                .await
                .unwrap();
        }
        // Yesterday counts: the streak isn't broken until a whole day passes.
        assert_eq!(db.streak("2026-11-22").await.unwrap(), 2);
        assert_eq!(db.streak("2026-11-21").await.unwrap(), 2);
        assert_eq!(db.streak("2026-11-23").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn read_marks_toggle_per_kind() {
        let dir = temp_dir("reads");
        let db = UserDb::open(&dir.join("user.db")).await.unwrap();
        db.set_read("guide", "cache", true).await.unwrap();
        db.set_read("guide", "cache", true).await.unwrap();
        db.set_read("guide", "lb", true).await.unwrap();
        db.set_read("pattern", "cache", true).await.unwrap();
        assert_eq!(db.reads("guide").await.unwrap(), ["cache", "lb"]);
        db.set_read("guide", "cache", false).await.unwrap();
        assert_eq!(db.reads("guide").await.unwrap(), ["lb"]);
        assert_eq!(db.reads("pattern").await.unwrap(), ["cache"]);
    }
}

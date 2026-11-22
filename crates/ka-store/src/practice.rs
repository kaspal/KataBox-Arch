//! Practice records in `user.db` (M5): estimation answers (v4), quiz answers and the
//! spaced-review queue (v5). Find-the-flaw drills use the problem tables (`flaw--<id>`).

use serde::Serialize;

use crate::user::UserDb;
use crate::{Result, now_ms};

/// One drill's record: how many checks, and whether any was within its band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EstimateStatus {
    pub problem_id: String,
    pub drill_id: String,
    pub tries: i64,
    pub done: bool,
}

/// A question in the review queue, as stored (dates are local `YYYY-MM-DD`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCardRow {
    pub question_id: String,
    pub step: i64,
    pub lapses: i64,
    pub due_on: String,
    pub last_on: String,
}

impl UserDb {
    pub async fn record_quiz_attempt(
        &self,
        question_id: &str,
        correct: bool,
        day: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO quiz_attempts (question_id, correct, day, at) VALUES (?, ?, ?, ?)",
        )
        .bind(question_id)
        .bind(i64::from(correct))
        .bind(day)
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The questions answered at least once, sorted.
    pub async fn answered_questions(&self) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            "SELECT DISTINCT question_id FROM quiz_attempts ORDER BY question_id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn review_card(&self, question_id: &str) -> Result<Option<ReviewCardRow>> {
        let row: Option<(String, i64, i64, String, String)> = sqlx::query_as(
            "SELECT question_id, step, lapses, due_on, last_on FROM review_cards WHERE question_id = ?",
        )
        .bind(question_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(card_row))
    }

    /// Insert or replace a card; `None` removes it (answered right first time, or retired).
    pub async fn put_review_card(
        &self,
        question_id: &str,
        card: Option<&ReviewCardRow>,
    ) -> Result<()> {
        match card {
            Some(c) => {
                sqlx::query(
                    "INSERT INTO review_cards (question_id, step, lapses, due_on, last_on)
                     VALUES (?, ?, ?, ?, ?)
                     ON CONFLICT (question_id) DO UPDATE SET
                       step = excluded.step, lapses = excluded.lapses,
                       due_on = excluded.due_on, last_on = excluded.last_on",
                )
                .bind(question_id)
                .bind(c.step)
                .bind(c.lapses)
                .bind(&c.due_on)
                .bind(&c.last_on)
                .execute(&self.pool)
                .await?;
            }
            None => {
                sqlx::query("DELETE FROM review_cards WHERE question_id = ?")
                    .bind(question_id)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    /// Every card in the queue (the caller orders them and picks the due ones).
    pub async fn review_cards(&self) -> Result<Vec<ReviewCardRow>> {
        let rows: Vec<(String, i64, i64, String, String)> = sqlx::query_as(
            "SELECT question_id, step, lapses, due_on, last_on FROM review_cards ORDER BY question_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(card_row).collect())
    }

    pub async fn record_estimate(
        &self,
        problem_id: &str,
        drill_id: &str,
        answer: f64,
        ok: bool,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO estimates (problem_id, drill_id, answer, ok, at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(problem_id)
        .bind(drill_id)
        .bind(answer)
        .bind(i64::from(ok))
        .bind(now_ms())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Every drill with at least one check, by problem then drill.
    pub async fn estimate_statuses(&self) -> Result<Vec<EstimateStatus>> {
        let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT problem_id, drill_id, count(*), max(ok) FROM estimates
             GROUP BY problem_id, drill_id ORDER BY problem_id, drill_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(problem_id, drill_id, tries, ok)| EstimateStatus {
                problem_id,
                drill_id,
                tries,
                done: ok == 1,
            })
            .collect())
    }
}

fn card_row(
    (question_id, step, lapses, due_on, last_on): (String, i64, i64, String, String),
) -> ReviewCardRow {
    ReviewCardRow {
        question_id,
        step,
        lapses,
        due_on,
        last_on,
    }
}

#[cfg(test)]
mod tests {
    use crate::testutil::temp_dir;
    use crate::user::UserDb;

    #[tokio::test]
    async fn estimates_count_tries_and_remember_a_hit() {
        let db = UserDb::open(&temp_dir("estimates").join("user.db"))
            .await
            .unwrap();
        db.record_estimate("url-shortener", "qps", 1.0, false)
            .await
            .unwrap();
        db.record_estimate("url-shortener", "qps", 12_000.0, true)
            .await
            .unwrap();
        db.record_estimate("url-shortener", "qps", 1.0, false)
            .await
            .unwrap();
        db.record_estimate("pastebin", "storage", 3.0, false)
            .await
            .unwrap();
        let s = db.estimate_statuses().await.unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            (s[0].problem_id.as_str(), s[0].done, s[0].tries),
            ("pastebin", false, 1)
        );
        assert_eq!(
            (s[1].drill_id.as_str(), s[1].done, s[1].tries),
            ("qps", true, 3)
        );
        db.close().await;
    }

    #[tokio::test]
    async fn quiz_attempts_and_review_cards_round_trip() {
        use super::ReviewCardRow;
        let db = UserDb::open(&temp_dir("reviews").join("user.db"))
            .await
            .unwrap();
        db.record_quiz_attempt("guide:cache:1", false, "2026-11-22")
            .await
            .unwrap();
        db.record_quiz_attempt("guide:cache:1", true, "2026-11-23")
            .await
            .unwrap();
        db.record_quiz_attempt("pattern:saga:2", true, "2026-11-23")
            .await
            .unwrap();
        assert_eq!(
            db.answered_questions().await.unwrap(),
            ["guide:cache:1", "pattern:saga:2"]
        );
        let card = ReviewCardRow {
            question_id: "guide:cache:1".into(),
            step: 0,
            lapses: 0,
            due_on: "2026-11-23".into(),
            last_on: "2026-11-22".into(),
        };
        db.put_review_card("guide:cache:1", Some(&card))
            .await
            .unwrap();
        let moved = ReviewCardRow {
            step: 1,
            due_on: "2026-11-26".into(),
            ..card.clone()
        };
        db.put_review_card("guide:cache:1", Some(&moved))
            .await
            .unwrap();
        assert_eq!(db.review_card("guide:cache:1").await.unwrap(), Some(moved));
        assert_eq!(db.review_cards().await.unwrap().len(), 1);
        db.put_review_card("guide:cache:1", None).await.unwrap();
        assert_eq!(db.review_card("guide:cache:1").await.unwrap(), None);
        db.close().await;
    }
}

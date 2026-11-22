# ADR 0010: Quizzes and spaced review ("SM-2-lite")

## Context
Every Guide entry and pattern ends with three quick-check questions (M5.5, 105 in all). The spec asks that
missed questions "return days later in a short review queue". Full SM-2 (Anki's algorithm) tracks an ease factor per
card from graded answers (0–5). Our answers are binary (right or wrong) and the queue is small, so a simpler,
explainable schedule fits better. It must be deterministic and testable: no wall clock in Rust.

## Decision
- **Content:** `[[quiz]]` in `entry.toml` / `pattern.toml`: a question, 3–4 options, the index of the right one, and
  an explanation, all in English and Spanish. `ka-pack` requires exactly three per page, the answer among the
  options, no repeated option in either language, and the same option count in both. Question ids are
  `guide:<kind>:<n>` and `pattern:<id>:<n>`.
- **Answers stay in Rust.** Pages and quizzes reach the UI without answers or explanations; `answer_quiz` judges the
  choice, records it (`quiz_attempts`) and only then returns the right option and why.
- **The schedule** (`ka_core::review`, pure functions over calendar days):
  - a question answered right the first time never enters the queue;
  - a miss enters it at step 0, due tomorrow;
  - a correct review moves it up a step, due `INTERVALS[step]` days later, with `INTERVALS = [1, 3, 7, 21]`; a correct
    answer at step 3 retires it;
  - a wrong review sends it back to step 0 (due tomorrow) and adds a lapse;
  - answering before the due day changes nothing if right, and counts as a miss if wrong.
- **Order:** due cards come most lapses first, then oldest due, then by id. This order key never changes an interval
  (it is not SM-2's ease).
- **Dates:** the UI sends its local date (`YYYY-MM-DD`, `localDay()`), as for streaks; Rust counts days since
  1970-01-01 with no time zone (Hinnant's civil-day arithmetic), so month and year boundaries and leap days are exact.
- **Storage:** `user.db` migration `0005_reviews`: `quiz_attempts(question_id, correct, day, at)` and
  `review_cards(question_id, step, lapses, due_on, last_on)`.
- **Done for paths:** once all three of a page's questions have been answered, the page's quiz counts as done (a
  `quiz` read mark), which learning paths use.

## Consequences
- The schedule fits in one table-driven test (`review::tests::the_schedule_follows_its_table`); no clock to mock.
- A question removed from the content later simply stops appearing (its card is skipped).
- If graded answers or per-card ease are wanted later, `review_cards` can gain columns through a new migration.

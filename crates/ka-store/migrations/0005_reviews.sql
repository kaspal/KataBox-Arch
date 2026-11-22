-- user.db v5 (KataBox Arch, M5.5): quiz answers and the spaced-review queue (ADR 0010).
-- Released migrations are never edited; add a new file. `at` is UTC milliseconds since the
-- Unix epoch; `day`, `due_on` and `last_on` are the user's local dates, `YYYY-MM-DD`.

-- Every answer to a quiz question (`guide:<kind>:<n>` or `pattern:<id>:<n>`).
CREATE TABLE quiz_attempts (
    question_id TEXT NOT NULL,
    correct     INTEGER NOT NULL CHECK (correct IN (0, 1)),
    day         TEXT NOT NULL,
    at          INTEGER NOT NULL
) STRICT;

CREATE INDEX quiz_attempts_by_question ON quiz_attempts (question_id);

-- Questions in the review queue (a missed question, until it retires).
CREATE TABLE review_cards (
    question_id TEXT PRIMARY KEY,
    step        INTEGER NOT NULL,
    lapses      INTEGER NOT NULL,
    due_on      TEXT NOT NULL,
    last_on     TEXT NOT NULL
) STRICT;

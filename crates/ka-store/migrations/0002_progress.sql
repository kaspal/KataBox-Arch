-- user.db v2 (KataBox Arch, M2.3): practice progress. Released migrations are never edited;
-- add a new file. Times are UTC milliseconds since the Unix epoch.

-- Every submission: the design as submitted and the judgement (ka-judge JSON).
CREATE TABLE submissions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    problem_id    TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    design        TEXT NOT NULL,
    verdict       TEXT NOT NULL,
    score         INTEGER NOT NULL,
    correctness   INTEGER NOT NULL,
    cost_score    INTEGER NOT NULL,
    latency_score INTEGER NOT NULL,
    cost_usd      REAL NOT NULL,
    p95_ms        REAL NOT NULL,
    results       TEXT NOT NULL
) STRICT;
CREATE INDEX submissions_by_problem ON submissions (problem_id, created_at DESC);

-- One row per problem touched: attempted until a submission is Accepted.
CREATE TABLE problem_status (
    problem_id         TEXT PRIMARY KEY NOT NULL,
    status             TEXT NOT NULL CHECK (status IN ('attempted', 'solved')),
    best_score         INTEGER NOT NULL,
    best_submission_id INTEGER,
    first_solved_at    INTEGER,
    updated_at         INTEGER NOT NULL
) STRICT;

-- The design being worked on, per problem (autosaved).
CREATE TABLE problem_drafts (
    problem_id TEXT PRIMARY KEY NOT NULL,
    design     TEXT NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

-- Hint tiers the user chose to reveal.
CREATE TABLE hint_unlocks (
    problem_id TEXT NOT NULL,
    tier       INTEGER NOT NULL CHECK (tier BETWEEN 1 AND 3),
    at         INTEGER NOT NULL,
    PRIMARY KEY (problem_id, tier)
) STRICT;

-- Live runs started on a problem (the attempts count).
CREATE TABLE runs (
    problem_id TEXT NOT NULL,
    at         INTEGER NOT NULL
) STRICT;
CREATE INDEX runs_by_problem ON runs (problem_id);

-- Days with a submission, as the user's local date (YYYY-MM-DD, sent by the UI): the streak.
CREATE TABLE activity_days (
    day TEXT PRIMARY KEY NOT NULL
) STRICT;

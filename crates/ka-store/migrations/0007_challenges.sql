-- user.db v7 (KataBox Arch, M8.5): the "same SLO, half the cost" challenge.
-- Released migrations are never edited; add a new file. `solved_at` is UTC milliseconds since
-- the Unix epoch.

-- One row per problem whose challenge was attempted: the cheapest design that passed every
-- test within the challenge budget, when it was first solved, and how many tries it took.
CREATE TABLE challenge_results (
    problem_id TEXT PRIMARY KEY,
    best_cost  REAL,
    solved_at  INTEGER,
    attempts   INTEGER NOT NULL DEFAULT 0
) STRICT;

-- user.db v4 (KataBox Arch, M5.2): estimation drill answers. Released migrations are never
-- edited; add a new file. Times are UTC milliseconds since the Unix epoch.

-- One row per check; a drill counts as done once any check of it was within its band.
CREATE TABLE estimates (
    problem_id TEXT NOT NULL,
    drill_id   TEXT NOT NULL,
    answer     REAL NOT NULL,
    ok         INTEGER NOT NULL CHECK (ok IN (0, 1)),
    at         INTEGER NOT NULL
) STRICT;

CREATE INDEX estimates_by_drill ON estimates (problem_id, drill_id);

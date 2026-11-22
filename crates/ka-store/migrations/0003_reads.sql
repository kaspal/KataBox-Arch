-- user.db v3 (KataBox Arch, M3.4): read marks for Guide entries (and, later, patterns).
-- Released migrations are never edited; add a new file. Times are UTC milliseconds since the
-- Unix epoch.

-- `kind` is what was read ("guide", "pattern"); `id` is the entry (a component kind, a
-- pattern id).
CREATE TABLE reads (
    kind TEXT NOT NULL,
    id   TEXT NOT NULL,
    at   INTEGER NOT NULL,
    PRIMARY KEY (kind, id)
) STRICT;

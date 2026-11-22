-- user.db v6 (KataBox Arch, M6.4): installed problem packs (ADR 0011).
-- Released migrations are never edited; add a new file. `installed_at` and `removed_at` are
-- UTC milliseconds since the Unix epoch.

-- One row per pack ever installed. Uninstalling sets `removed_at` and keeps the row, so
-- the history of its problems still has a name ("from a removed pack").
CREATE TABLE packs (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    author       TEXT NOT NULL,
    version      TEXT NOT NULL,
    licence      TEXT NOT NULL,
    sha256       TEXT NOT NULL,
    installed_at INTEGER NOT NULL,
    removed_at   INTEGER
) STRICT;

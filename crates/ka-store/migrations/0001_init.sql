-- user.db v1 (KataBox Arch). Released migrations are never edited; add a new file.
-- Times are UTC milliseconds since the Unix epoch.

-- App settings: JSON values by key (keys allow-listed in ka-app).
CREATE TABLE settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

-- Recently opened or saved .kbarch files, newest first by opened_at.
CREATE TABLE recent_files (
    path      TEXT PRIMARY KEY NOT NULL,
    name      TEXT NOT NULL,
    opened_at INTEGER NOT NULL
) STRICT;
CREATE INDEX recent_files_by_time ON recent_files (opened_at DESC);

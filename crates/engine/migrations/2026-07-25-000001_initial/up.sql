CREATE TABLE spaces (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

-- `spaces::create` checks uniqueness for a readable error; this index guarantees
-- it even if a write slipped past.
CREATE UNIQUE INDEX spaces_name_unique ON spaces (name COLLATE NOCASE);

CREATE TABLE notes (
    id                   TEXT PRIMARY KEY,
    space_id             TEXT NOT NULL REFERENCES spaces (id) ON DELETE CASCADE,
    title                TEXT NOT NULL,
    language             TEXT NOT NULL,
    content              TEXT NOT NULL,
    source               TEXT NOT NULL,
    pinned               INTEGER NOT NULL CHECK (pinned IN (0, 1)),
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    lifecycle_kind       TEXT NOT NULL CHECK (lifecycle_kind IN ('permanent', 'expires')),
    lifecycle_expires_at TEXT,
    CHECK ((lifecycle_kind = 'expires') = (lifecycle_expires_at IS NOT NULL))
);

CREATE INDEX notes_space_id ON notes (space_id);
CREATE INDEX notes_updated_at ON notes (updated_at DESC);

CREATE TABLE note_tags (
    note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (note_id, tag)
);

CREATE INDEX note_tags_tag ON note_tags (tag);

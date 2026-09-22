-- The body as it was before an edit, so a snippet that worked can be put back.
--
-- ⚠️ The trash protects a deletion and nothing protected an edit. The point is not the
-- restoring, it is the ease: a text you know is recoverable is a text you edit freely.
--
-- ⚠️ `content` is **sealed**, like the column it is a copy of. A history of every body in
-- plaintext beside a sealed library would undo the encryption entirely.
--
-- ⚠️ The body only, and snippets only. Title, tags and language are almost never what
-- anyone wants back, and a checklist's items live in `note_items` — a second table to
-- snapshot and a two-step restore, deliberately out of this first version.
CREATE TABLE note_revisions (
    id       TEXT PRIMARY KEY NOT NULL,
    note_id  TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    content  TEXT NOT NULL,
    taken_at TEXT NOT NULL
);

-- What every read of this table does: the newest few of one note.
CREATE INDEX idx_note_revisions_note ON note_revisions (note_id, taken_at DESC);

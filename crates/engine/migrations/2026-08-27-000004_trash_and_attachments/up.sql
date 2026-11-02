-- `deleted_at` rather than a boolean: the 30-day purge needs the date, and NULL for
-- a living note is what lets the partial index cover only the deleted rows.

ALTER TABLE notes ADD COLUMN deleted_at TEXT;

CREATE INDEX notes_deleted_at ON notes (deleted_at) WHERE deleted_at IS NOT NULL;

-- The bytes are not here: they live in `app_data_dir()/attachments/` under a name
-- derived from the id.
CREATE TABLE attachments (
    id         TEXT PRIMARY KEY,
    note_id    TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    file_name  TEXT NOT NULL,
    mime_type  TEXT NOT NULL,
    byte_size  INTEGER NOT NULL CHECK (byte_size >= 0),
    created_at TEXT NOT NULL
);

CREATE INDEX attachments_note_id ON attachments (note_id);

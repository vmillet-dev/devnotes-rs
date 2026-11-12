-- A folder cuts a space into regions. It belongs to one space and dies with it, but
-- ⚠️ `ON DELETE SET NULL` on the note side: deleting a folder must never delete a note.
-- "No folder" is a legitimate state, which is why this needs no refuge argument the way
-- `delete_space(id, targetSpaceId)` does.
CREATE TABLE folders (
    id         TEXT PRIMARY KEY NOT NULL,
    space_id   TEXT NOT NULL REFERENCES spaces (id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    colour     TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX folders_space ON folders (space_id);

-- No `DEFAULT`, so the column is nullable: SQLite only accepts an added `REFERENCES`
-- column whose default is NULL, and unfiled is what every existing note is.
ALTER TABLE notes ADD COLUMN folder_id TEXT REFERENCES folders (id) ON DELETE SET NULL;

CREATE INDEX notes_folder ON notes (folder_id) WHERE folder_id IS NOT NULL;

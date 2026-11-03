-- `DEFAULT 'snippet'` is not a convenience: SQLite refuses an `ADD COLUMN NOT NULL`
-- without one, and it is what gives the notes already stored their value. No
-- `CHECK`, like `notes.language`: the list lives in the domain.
ALTER TABLE notes ADD COLUMN kind TEXT NOT NULL DEFAULT 'snippet';

-- The position is part of the primary key: it carries the order, and a write
-- rewrites the whole list.
CREATE TABLE note_items (
    note_id  TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    text     TEXT NOT NULL,
    done     INTEGER NOT NULL CHECK (done IN (0, 1)),
    PRIMARY KEY (note_id, position)
);

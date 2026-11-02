DROP TABLE note_items;

-- Every note reads back as a snippet, which is what `NoteKind::default()` already
-- answers for a row written before this column existed.
ALTER TABLE notes DROP COLUMN kind;

-- ⚠️ No `COLLATE NOCASE` here, unlike `note_tags`: `{{Host}}` and `{{host}}` are two
-- distinct fields in the text, and conflating them in the database would fill one
-- with the other's value.
CREATE TABLE note_placeholders (
    note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    name    TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (note_id, name)
);

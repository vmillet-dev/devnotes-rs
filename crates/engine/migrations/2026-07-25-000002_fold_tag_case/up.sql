-- Folds tag case **across** notes, where `normalize_tags` only folded it within one.
-- A column's collation cannot be altered, hence the recreated table; `INSERT OR
-- IGNORE` absorbs the duplicates the new primary key merges.

CREATE TABLE note_tags_v2 (
    note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    tag     TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (note_id, tag)
);

INSERT OR IGNORE INTO note_tags_v2 (note_id, tag) SELECT note_id, tag FROM note_tags;

DROP TABLE note_tags;
ALTER TABLE note_tags_v2 RENAME TO note_tags;

CREATE INDEX note_tags_tag ON note_tags (tag);

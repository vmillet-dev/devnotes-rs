-- Tags merged by the up migration do not split again: that information was lost
-- there, not here.

CREATE TABLE note_tags_v1 (
    note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (note_id, tag)
);

INSERT OR IGNORE INTO note_tags_v1 (note_id, tag) SELECT note_id, tag FROM note_tags;

DROP TABLE note_tags;
ALTER TABLE note_tags_v1 RENAME TO note_tags;

CREATE INDEX note_tags_tag ON note_tags (tag);

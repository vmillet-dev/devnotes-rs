-- `{{field}}` values valid across the whole corpus, sitting underneath the per-note
-- ones — see `notes::placeholder::resolve`. No `COLLATE NOCASE`, for the reason that
-- already applies to `note_placeholders`.
CREATE TABLE global_placeholders (
    name  TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);

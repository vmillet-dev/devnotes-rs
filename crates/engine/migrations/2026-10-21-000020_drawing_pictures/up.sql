-- A Dessin's two pictures, sealed, beside its body rather than in it: a list opens every body
-- it searches, and a picture is opened only where it is shown — a card, a Note that embeds it.
CREATE TABLE drawing_pictures (
    note_id TEXT PRIMARY KEY NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    thumbnail BLOB NOT NULL,
    picture BLOB NOT NULL
);

-- Where a zone sits, and where a loose card sits beside it. ⚠️ Local state, never domain:
-- it is kept out of the `Note` and `Folder` models on purpose, because `transfer::Bundle`
-- deserialises both — a coordinate stored there would travel in every export and land on
-- top of the arrangement the receiving machine already has.
--
-- Nullable, and all four together: NULL means "never laid out", which is what every
-- folder made before this is, and what the board materialises on its first read.
ALTER TABLE folders ADD COLUMN x INTEGER;
ALTER TABLE folders ADD COLUMN y INTEGER;
ALTER TABLE folders ADD COLUMN w INTEGER;
ALTER TABLE folders ADD COLUMN h INTEGER;

-- A row exists only for a note that is loose: a filed note flows inside its zone and has
-- no position of its own to keep consistent.
CREATE TABLE note_positions (
    note_id TEXT PRIMARY KEY NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    x       INTEGER NOT NULL,
    y       INTEGER NOT NULL
);

-- No `CHECK` on the column: the list of languages lives in the domain and moves
-- between versions — freezing it here would force a migration per addition.

CREATE INDEX notes_language ON notes (language);

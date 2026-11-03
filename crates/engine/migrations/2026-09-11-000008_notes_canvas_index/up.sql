-- `notes_space_id` served the filter and left SQLite to sort afterwards, which shows
-- from a few thousand notes on. Partial, so trashed rows keep it small, and `id`
-- closes the ordering: two notes written in the same millisecond could otherwise
-- swap places between two reads.
CREATE INDEX notes_canvas ON notes (space_id, updated_at DESC, id) WHERE deleted_at IS NULL;

-- A folder inside a folder, of the same space. `SET NULL` is only a guard: deleting a folder
-- hands its sub-folders to its own parent in Rust first, and a space's cascade takes them all.
ALTER TABLE folders ADD COLUMN parent_id TEXT REFERENCES folders (id) ON DELETE SET NULL;
CREATE INDEX folders_parent ON folders (space_id, parent_id);

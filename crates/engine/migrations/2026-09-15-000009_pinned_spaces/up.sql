-- Spaces came back in one order and only one: `name COLLATE NOCASE`. The space opened
-- every morning sat wherever its initial put it, under archives nobody touches.
--
-- A boolean and not a `position` column: an order the user maintains is a second thing
-- to keep consistent on every insert and delete, and pinning is the gesture the app
-- already has for "keep this within reach".
ALTER TABLE spaces ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;

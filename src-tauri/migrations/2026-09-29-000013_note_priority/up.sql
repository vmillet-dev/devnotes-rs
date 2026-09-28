-- In the clear, like `kind` and `language`: the canvas filters, sorts and groups on it.
ALTER TABLE notes ADD COLUMN priority TEXT NOT NULL DEFAULT 'none';

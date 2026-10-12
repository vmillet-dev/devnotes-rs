-- The environments and the globals a request resolves its `{{variables}}` from: the library's,
-- beside the collections. Sealed: a set whole — its name, colour, protection and variables. In
-- the clear: the id, whether it is the globals or an environment, its rank, when it was made.
CREATE TABLE http_environments (
    id         TEXT PRIMARY KEY NOT NULL,
    kind       TEXT NOT NULL,
    position   INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    document   TEXT NOT NULL
);

CREATE UNIQUE INDEX http_environments_one_globals ON http_environments (kind) WHERE kind = 'globals';

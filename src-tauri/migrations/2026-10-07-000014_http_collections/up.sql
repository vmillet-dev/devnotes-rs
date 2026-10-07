-- HTTP collections belong to the library, not to a space. Sealed: every name, a request's
-- document and a container's settings. In the clear: ids, parents, positions, the kind and
-- the method, instants — what SQL orders and joins on, the line the notes draw.
CREATE TABLE http_collections (
    id         TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    settings   TEXT NOT NULL,
    position   INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

-- Folders and requests under one parent share one order: `position` interleaves them.
CREATE TABLE http_folders (
    id            TEXT PRIMARY KEY NOT NULL,
    collection_id TEXT NOT NULL REFERENCES http_collections (id) ON DELETE CASCADE,
    parent_id     TEXT REFERENCES http_folders (id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    settings      TEXT NOT NULL,
    position      INTEGER NOT NULL,
    created_at    TEXT NOT NULL
);

CREATE INDEX http_folders_parent ON http_folders (collection_id, parent_id);

CREATE TABLE http_requests (
    id            TEXT PRIMARY KEY NOT NULL,
    collection_id TEXT NOT NULL REFERENCES http_collections (id) ON DELETE CASCADE,
    folder_id     TEXT REFERENCES http_folders (id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    kind          TEXT NOT NULL,
    method        TEXT NOT NULL,
    document      TEXT NOT NULL,
    position      INTEGER NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE INDEX http_requests_folder ON http_requests (collection_id, folder_id);

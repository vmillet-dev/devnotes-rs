-- The library's cookie jar. Sealed: the cookie whole, its domain included — which sites one
-- talks to is not for the file to say. In the clear: the expiry, what the purge deletes by.
CREATE TABLE http_cookies (
    id         TEXT PRIMARY KEY NOT NULL,
    expires_at TEXT,
    cookie     TEXT NOT NULL
);

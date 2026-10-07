-- What was sent. Sealed: the summary a list shows (name, URL, time, size, failure) and the
-- record an entry opens. In the clear: the instant, the method, the status and the request it
-- came from — what the list orders on and what retention deletes by.
CREATE TABLE http_history (
    id         TEXT PRIMARY KEY NOT NULL,
    sent_at    TEXT NOT NULL,
    request_id TEXT REFERENCES http_requests (id) ON DELETE SET NULL,
    method     TEXT NOT NULL,
    status     INTEGER,
    summary    TEXT NOT NULL,
    record     TEXT NOT NULL
);

CREATE INDEX http_history_sent_at ON http_history (sent_at);

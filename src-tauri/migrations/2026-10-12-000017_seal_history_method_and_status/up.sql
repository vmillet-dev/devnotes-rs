-- The method and the status of a send were in the clear, and no query reads them: the list
-- orders on the instant, retention deletes by it. A reader of the raw file learnt « a DELETE
-- answered 401 at 14:02 » for nothing. Both are in the sealed summary from here on.
--
-- What was recorded before carries them in these two columns alone, and SQL cannot move them
-- under a key it does not hold: those entries go, rather than come back with a method and a
-- status they never had. A history is thirty days of sends.
DELETE FROM http_history;
ALTER TABLE http_history DROP COLUMN method;
ALTER TABLE http_history DROP COLUMN status;

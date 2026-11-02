-- Reverting drops the attachment records; the files under `app_data_dir()/attachments/`
-- are not this migration's to delete, and the startup sweep collects them.
DROP INDEX attachments_note_id;
DROP TABLE attachments;

DROP INDEX notes_deleted_at;
-- Takes the trash with it: a note that was only soft-deleted comes back visible.
ALTER TABLE notes DROP COLUMN deleted_at;

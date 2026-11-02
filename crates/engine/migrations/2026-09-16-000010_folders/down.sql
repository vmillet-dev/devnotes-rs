DROP INDEX notes_folder;
ALTER TABLE notes DROP COLUMN folder_id;
DROP INDEX folders_space;
DROP TABLE folders;

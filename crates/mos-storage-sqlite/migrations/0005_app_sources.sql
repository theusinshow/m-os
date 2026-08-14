BEGIN IMMEDIATE;

ALTER TABLE apps ADD COLUMN source_url TEXT;

DROP TABLE app_search;

CREATE VIRTUAL TABLE app_search USING fts5(
    name,
    description,
    source_url,
    launch_target,
    content='apps',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

INSERT INTO app_search(app_search) VALUES('rebuild');

PRAGMA user_version = 5;

COMMIT;

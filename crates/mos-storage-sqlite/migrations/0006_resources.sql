BEGIN IMMEDIATE;

CREATE TABLE resources (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('link')),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    url TEXT NOT NULL CHECK (
        url LIKE 'https://%' OR url LIKE 'http://%'
    ),
    note TEXT NOT NULL DEFAULT '',
    source_capture_id TEXT REFERENCES captures(id) ON DELETE RESTRICT,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    deleted_at TEXT
) STRICT;

CREATE UNIQUE INDEX resources_source_capture_unique
    ON resources(source_capture_id)
    WHERE source_capture_id IS NOT NULL;

CREATE INDEX resources_lifecycle_order
    ON resources(lifecycle_state, updated_at DESC);

CREATE VIRTUAL TABLE resource_search USING fts5(
    title,
    url,
    note,
    content='resources',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

PRAGMA user_version = 6;

COMMIT;

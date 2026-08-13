BEGIN IMMEDIATE;

CREATE TABLE captures (
    id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    source_kind TEXT NOT NULL CHECK (source_kind IN ('home', 'quick_capture')),
    processing_state TEXT NOT NULL DEFAULT 'inbox'
        CHECK (processing_state IN ('inbox', 'processed')),
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    captured_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    deleted_at TEXT
) STRICT;

CREATE INDEX captures_inbox_order
    ON captures(processing_state, lifecycle_state, captured_at DESC);

CREATE INDEX captures_lifecycle_order
    ON captures(lifecycle_state, captured_at DESC);

CREATE VIRTUAL TABLE capture_search USING fts5(
    content,
    content='captures',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE app_metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
) STRICT;

PRAGMA user_version = 1;

COMMIT;

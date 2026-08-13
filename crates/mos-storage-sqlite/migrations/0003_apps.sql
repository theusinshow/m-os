BEGIN IMMEDIATE;

CREATE TABLE apps (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    launch_kind TEXT CHECK (launch_kind IN ('url', 'path') OR launch_kind IS NULL),
    launch_target TEXT,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    deleted_at TEXT,
    last_opened_at TEXT,
    CHECK (
        (launch_kind IS NULL AND launch_target IS NULL)
        OR (
            launch_kind IS NOT NULL
            AND launch_target IS NOT NULL
            AND length(trim(launch_target)) > 0
        )
    )
) STRICT;

CREATE INDEX apps_lifecycle_order
    ON apps(lifecycle_state, updated_at DESC);

CREATE INDEX apps_recent_order
    ON apps(lifecycle_state, last_opened_at DESC, updated_at DESC);

CREATE VIRTUAL TABLE app_search USING fts5(
    name,
    description,
    launch_target,
    content='apps',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

PRAGMA user_version = 3;

COMMIT;

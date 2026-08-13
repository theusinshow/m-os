BEGIN IMMEDIATE;

CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    deleted_at TEXT
) STRICT;

CREATE INDEX projects_lifecycle_order
    ON projects(lifecycle_state, updated_at DESC);

CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    source_capture_id TEXT UNIQUE REFERENCES captures(id) ON DELETE RESTRICT,
    work_state TEXT NOT NULL DEFAULT 'backlog'
        CHECK (work_state IN ('backlog', 'doing', 'done')),
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    archived_at TEXT,
    deleted_at TEXT
) STRICT;

CREATE INDEX tasks_board_order
    ON tasks(lifecycle_state, work_state, updated_at DESC);

CREATE INDEX tasks_project_order
    ON tasks(project_id, lifecycle_state, updated_at DESC);

CREATE VIRTUAL TABLE project_search USING fts5(
    name,
    description,
    content='projects',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE task_search USING fts5(
    title,
    description,
    content='tasks',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

PRAGMA user_version = 2;

COMMIT;

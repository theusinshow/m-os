BEGIN IMMEDIATE;

CREATE TABLE workspaces (
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

CREATE INDEX workspaces_lifecycle_order
    ON workspaces(lifecycle_state, updated_at DESC);

CREATE TABLE project_workspaces (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id, workspace_id)
) STRICT;

CREATE INDEX project_workspaces_workspace_order
    ON project_workspaces(workspace_id, created_at DESC);

CREATE TABLE app_workspaces (
    app_id TEXT NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (app_id, workspace_id)
) STRICT;

CREATE INDEX app_workspaces_workspace_order
    ON app_workspaces(workspace_id, created_at DESC);

CREATE VIRTUAL TABLE workspace_search USING fts5(
    name,
    description,
    content='workspaces',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

PRAGMA user_version = 4;

COMMIT;

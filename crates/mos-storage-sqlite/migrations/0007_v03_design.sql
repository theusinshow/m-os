-- v0.3 — o schema que o design v0.7 exige.
--
-- Tres mudancas num passo so:
--   1. tasks.work_state passa de tres para seis estados (as colunas do kanban);
--   2. resources.kind passa de 'link' para site/library/image/note (filtros da Library),
--      e a validacao de url passa a variar por tipo, porque note nao tem url;
--   3. apps ganha as quatro capacidades e projects ganha repository.
--
-- SQLite nao altera CHECK, entao 1 e 2 recriam a tabela. 3 e aditivo e usa
-- ALTER TABLE, seguindo o padrao de 0005_app_sources.sql.
--
-- As tabelas FTS de conteudo externo sao recriadas e reconstruidas: depois de um
-- swap de tabela as contagens continuam batendo enquanto os rowids mudam, entao
-- ensure_search_projection() nao detectaria a divergencia sozinha.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. tasks — seis estados
--
-- NOTA: work_state 'inbox' NAO e a Inbox de Captures. Captures tem
-- processing_state; Tasks tem work_state. Compartilham o nome porque o design
-- usa INBOX como rotulo da primeira coluna do kanban, e nada alem disso.
-- ---------------------------------------------------------------------------

CREATE TABLE tasks_new (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    source_capture_id TEXT UNIQUE REFERENCES captures(id) ON DELETE RESTRICT,
    work_state TEXT NOT NULL DEFAULT 'backlog'
        CHECK (work_state IN ('inbox', 'backlog', 'planned', 'doing', 'review', 'done')),
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    archived_at TEXT,
    deleted_at TEXT
) STRICT;

INSERT INTO tasks_new (
    id, title, description, project_id, source_capture_id, work_state,
    lifecycle_state, created_at, updated_at, completed_at, archived_at, deleted_at
)
SELECT
    id, title, description, project_id, source_capture_id, work_state,
    lifecycle_state, created_at, updated_at, completed_at, archived_at, deleted_at
FROM tasks;

DROP TABLE task_search;
DROP TABLE tasks;

ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX tasks_board_order
    ON tasks(lifecycle_state, work_state, updated_at DESC);

CREATE INDEX tasks_project_order
    ON tasks(project_id, lifecycle_state, updated_at DESC);

CREATE VIRTUAL TABLE task_search USING fts5(
    title,
    description,
    content='tasks',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

INSERT INTO task_search(task_search) VALUES('rebuild');

-- ---------------------------------------------------------------------------
-- 2. resources — quatro tipos
--
-- Todo 'link' vira 'site': era o unico valor possivel e sempre significou
-- endereco na web. Nenhum dado se perde.
--
-- A CHECK de url espelha a validacao do dominio (NewResource::create):
-- site e library exigem http(s); image aceita url ou caminho local; note nao
-- tem url. O banco passa a recusar exatamente o que o core recusa.
-- ---------------------------------------------------------------------------

CREATE TABLE resources_new (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('site', 'library', 'image', 'note')),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    url TEXT NOT NULL DEFAULT '' CHECK (
        (kind = 'note' AND url = '')
        OR (kind IN ('site', 'library') AND (url LIKE 'https://%' OR url LIKE 'http://%'))
        OR (kind = 'image' AND length(trim(url)) > 0)
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

INSERT INTO resources_new (
    id, kind, title, url, note, source_capture_id, lifecycle_state,
    created_at, updated_at, archived_at, deleted_at
)
SELECT
    id, 'site', title, url, note, source_capture_id, lifecycle_state,
    created_at, updated_at, archived_at, deleted_at
FROM resources;

DROP TABLE resource_search;
DROP TABLE resources;

ALTER TABLE resources_new RENAME TO resources;

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

INSERT INTO resource_search(resource_search) VALUES('rebuild');

-- ---------------------------------------------------------------------------
-- 3. capacidades de App e repositorio de Project
--
-- can_open nasce verdadeiro para quem ja tem launch_target: abrir e o que
-- esses apps comprovadamente ja fazem, e declarar falso seria mentir sobre
-- uma capacidade em uso.
-- ---------------------------------------------------------------------------

ALTER TABLE apps ADD COLUMN can_open INTEGER NOT NULL DEFAULT 0
    CHECK (can_open IN (0, 1));
ALTER TABLE apps ADD COLUMN can_read INTEGER NOT NULL DEFAULT 0
    CHECK (can_read IN (0, 1));
ALTER TABLE apps ADD COLUMN can_write INTEGER NOT NULL DEFAULT 0
    CHECK (can_write IN (0, 1));
ALTER TABLE apps ADD COLUMN can_automate INTEGER NOT NULL DEFAULT 0
    CHECK (can_automate IN (0, 1));

UPDATE apps SET can_open = 1
    WHERE launch_target IS NOT NULL AND length(trim(launch_target)) > 0;

ALTER TABLE projects ADD COLUMN repository TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 7;

COMMIT;

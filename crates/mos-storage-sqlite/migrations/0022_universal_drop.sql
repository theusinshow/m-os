-- Universal Drop Zone — a entrada unica do M/OS.
--
-- Quatro mudancas:
--
--   1. `captures.source_kind` aceita 'drop'. Soltar algo sobre a janela e uma
--      superficie nova, e a ADR-004 exige que a Capture preserve a ORIGEM —
--      gravar um arquivo arrastado como se tivesse sido digitado na Home
--      apagaria o unico fato que a proveniencia existe para guardar.
--
--   2. `resources.kind` aceita 'file'. Um arquivo preservado nao e site, nao e
--      biblioteca e nao e nota. Ele nao tem `url`: o caminho do original mora
--      na linha de ingestao, enderecado pelo hash do conteudo.
--
--   3. `ingestions` — o registro do que entrou, por onde, o que virou e onde
--      parou. E a memoria do pipeline, e nao um segundo lugar onde a Library
--      procura coisas.
--
--   4. `resource_projects` — o elo que faltava para o drop contextual. Copia
--      estrutural de `resource_workspaces` (0009), N-para-N pelo mesmo motivo:
--      o mesmo memorial pode servir a dois Projects.
--
-- SQLite nao altera CHECK, entao 1 e 2 recriam a tabela. Diferente de 0007, as
-- duas tabelas AGORA TEM FILHAS — `tasks` e `resources` apontam para `captures`,
-- `resource_workspaces` aponta para `resources`. Por isso o procedimento e o da
-- propria documentacao do SQLite ("Making Other Kinds Of Table Schema Changes"):
--
--   * `foreign_keys=OFF` antes de abrir a transacao, para que o DROP da tabela
--     antiga nao dispare RESTRICT nem CASCADE nas filhas;
--   * `legacy_alter_table=ON` para que o RENAME **nao** reescreva as referencias
--     das filhas — elas devem continuar apontando para o nome antigo, que e o
--     nome que a tabela nova vai assumir;
--   * as duas voltam ao normal depois do COMMIT, e o `migrate()` do Rust roda
--     `foreign_key_check` em seguida: um orfao aqui seria pior que uma migration
--     que falha.

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. captures — a origem 'drop'
-- ---------------------------------------------------------------------------

DROP TABLE capture_search;

ALTER TABLE captures RENAME TO captures_old;

CREATE TABLE captures (
    id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    source_kind TEXT NOT NULL CHECK (source_kind IN ('home', 'quick_capture', 'drop')),
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

INSERT INTO captures (
    id, content, source_kind, processing_state, lifecycle_state,
    captured_at, created_at, updated_at, archived_at, deleted_at
)
SELECT
    id, content, source_kind, processing_state, lifecycle_state,
    captured_at, created_at, updated_at, archived_at, deleted_at
FROM captures_old;

DROP TABLE captures_old;

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

INSERT INTO capture_search(capture_search) VALUES('rebuild');

-- ---------------------------------------------------------------------------
-- 2. resources — o tipo 'file'
--
-- A CHECK de url continua espelhando `NewResource::create`: file entra junto de
-- note, do lado de quem nao tem url.
-- ---------------------------------------------------------------------------

DROP TABLE resource_search;

ALTER TABLE resources RENAME TO resources_old;

CREATE TABLE resources (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('site', 'library', 'image', 'note', 'file')),
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    url TEXT NOT NULL DEFAULT '' CHECK (
        (kind IN ('note', 'file') AND url = '')
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

INSERT INTO resources (
    id, kind, title, url, note, source_capture_id, lifecycle_state,
    created_at, updated_at, archived_at, deleted_at
)
SELECT
    id, kind, title, url, note, source_capture_id, lifecycle_state,
    created_at, updated_at, archived_at, deleted_at
FROM resources_old;

DROP TABLE resources_old;

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
-- 3. ingestions — o registro do pipeline
--
-- `state` e a maquina de estados do §7 do spec, e a fronteira que importa e
-- entre 'receiving' e 'preserved': dali para tras ainda se pode perder bytes,
-- dali para frente nao.
--
-- `added_project_link` e `added_workspace_link` existem por causa do desfazer.
-- Quando o drop cai sobre um arquivo que JA estava no M/OS, o contexto novo e
-- aplicado no Resource antigo — e desfazer precisa remover exatamente o que
-- esta ingestao acrescentou, sem levar junto uma relacao que ja existia.
--
-- `extracted_text` mora aqui, e nao so no indice: um FTS que guarda o unico
-- exemplar do texto deixa de ser reconstruivel, e a ADR-009 exige que o indice
-- seja derivado.
-- ---------------------------------------------------------------------------

CREATE TABLE ingestions (
    id TEXT PRIMARY KEY NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('drop_file', 'drop_text', 'drop_url')),
    original_name TEXT NOT NULL CHECK (length(trim(original_name)) > 0),
    mime TEXT NOT NULL DEFAULT '',
    byte_size INTEGER NOT NULL DEFAULT 0 CHECK (byte_size >= 0),
    sha256 TEXT NOT NULL DEFAULT '',
    stored_path TEXT NOT NULL DEFAULT '',
    detected_kind TEXT NOT NULL DEFAULT 'unknown' CHECK (detected_kind IN (
        'pdf', 'image', 'text', 'markdown', 'data', 'code', 'archive', 'url', 'unknown'
    )),
    state TEXT NOT NULL CHECK (state IN (
        'receiving', 'preserved', 'completed', 'interrupted', 'failed', 'undone'
    )),
    failure TEXT NOT NULL DEFAULT '',

    capture_id TEXT REFERENCES captures(id) ON DELETE SET NULL,
    resource_id TEXT REFERENCES resources(id) ON DELETE SET NULL,
    duplicate_of TEXT REFERENCES resources(id) ON DELETE SET NULL,

    context_page TEXT NOT NULL DEFAULT '',
    context_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    context_workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
    context_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    suggested_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    relation_confidence REAL NOT NULL DEFAULT 0,
    relation_reason TEXT NOT NULL DEFAULT '',
    added_project_link INTEGER NOT NULL DEFAULT 0 CHECK (added_project_link IN (0, 1)),
    added_workspace_link INTEGER NOT NULL DEFAULT 0 CHECK (added_workspace_link IN (0, 1)),

    extraction_state TEXT NOT NULL DEFAULT 'pending' CHECK (extraction_state IN (
        'pending', 'done', 'empty', 'unsupported', 'failed'
    )),
    extraction_error TEXT NOT NULL DEFAULT '',
    extracted_text TEXT NOT NULL DEFAULT '',
    page_count INTEGER,
    image_width INTEGER,
    image_height INTEGER,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX ingestions_state_order
    ON ingestions(state, created_at DESC);

CREATE INDEX ingestions_resource
    ON ingestions(resource_id);

-- A deduplicacao consulta por hash em todo drop de arquivo; sem indice isso
-- seria varredura da tabela inteira no caminho quente do pipeline.
CREATE INDEX ingestions_sha256
    ON ingestions(sha256)
    WHERE sha256 <> '';

CREATE VIRTUAL TABLE ingestion_search USING fts5(
    original_name,
    extracted_text,
    content='ingestions',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

-- ---------------------------------------------------------------------------
-- 4. resource_projects — o drop contextual
-- ---------------------------------------------------------------------------

CREATE TABLE resource_projects (
    resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (resource_id, project_id)
) STRICT;

CREATE INDEX resource_projects_project_order
    ON resource_projects(project_id, created_at DESC);

PRAGMA user_version = 22;

COMMIT;

PRAGMA legacy_alter_table = OFF;
PRAGMA foreign_keys = ON;

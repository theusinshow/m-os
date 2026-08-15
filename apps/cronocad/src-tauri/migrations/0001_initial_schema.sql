-- HoraCAD — Migration 0001: esquema inicial
--
-- Convencoes:
--  * ids: TEXT (UUID) gerados pela aplicacao;
--  * timestamps: TEXT em ISO 8601 UTC (ex.: "2026-07-11T13:34:00Z");
--  * valores monetarios: INTEGER em centavos;
--  * duracoes: INTEGER em segundos;
--  * booleanos: INTEGER (0/1).
--
-- O banco e a fonte persistente da verdade (secao 21). O tempo real e sempre
-- preservado; arredondamento e desconto de inatividade sao aplicados apenas em
-- visualizacao/cobranca, nunca sobrescrevendo os valores originais.

PRAGMA foreign_keys = ON;

-- ---------- clients ----------
CREATE TABLE clients (
    id            TEXT PRIMARY KEY NOT NULL,
    name          TEXT NOT NULL,
    company_name  TEXT,
    email         TEXT,
    phone         TEXT,
    notes         TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    archived_at   TEXT
);

-- ---------- projects ----------
CREATE TABLE projects (
    id                 TEXT PRIMARY KEY NOT NULL,
    client_id          TEXT REFERENCES clients (id) ON DELETE SET NULL,
    name               TEXT NOT NULL,
    code               TEXT,
    description        TEXT,
    hourly_rate_cents  INTEGER NOT NULL DEFAULT 0,
    status             TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'paused', 'completed', 'archived')),
    color              TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    archived_at        TEXT
);

CREATE INDEX idx_projects_client ON projects (client_id);
CREATE INDEX idx_projects_status ON projects (status);

-- ---------- time_entries ----------
CREATE TABLE time_entries (
    id                          TEXT PRIMARY KEY NOT NULL,
    project_id                  TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    started_at                  TEXT NOT NULL,
    ended_at                    TEXT,
    duration_seconds            INTEGER NOT NULL DEFAULT 0,
    idle_seconds                INTEGER NOT NULL DEFAULT 0,
    description                 TEXT,
    activity_type               TEXT NOT NULL DEFAULT 'other'
        CHECK (activity_type IN
            ('drawing', 'detailing', 'revision', 'meeting', 'study', 'other')),
    billable                    INTEGER NOT NULL DEFAULT 1,
    hourly_rate_snapshot_cents  INTEGER NOT NULL DEFAULT 0,
    source                      TEXT NOT NULL DEFAULT 'timer'
        CHECK (source IN ('timer', 'manual', 'reconstructed')),
    created_at                  TEXT NOT NULL,
    updated_at                  TEXT NOT NULL,
    deleted_at                  TEXT
);

CREATE INDEX idx_time_entries_project ON time_entries (project_id);
CREATE INDEX idx_time_entries_started ON time_entries (started_at);
CREATE INDEX idx_time_entries_deleted ON time_entries (deleted_at);

-- ---------- active_timer ----------
-- No maximo UM cronometro ativo: a coluna `singleton` e unica e restrita a 1,
-- garantindo no nivel do banco a impossibilidade de dois cronometros ativos.
CREATE TABLE active_timer (
    id                   TEXT PRIMARY KEY NOT NULL,
    singleton            INTEGER NOT NULL DEFAULT 1 UNIQUE CHECK (singleton = 1),
    project_id           TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    started_at           TEXT NOT NULL,
    last_resumed_at      TEXT NOT NULL,
    accumulated_seconds  INTEGER NOT NULL DEFAULT 0,
    status               TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'paused')),
    description          TEXT,
    activity_type        TEXT NOT NULL DEFAULT 'other'
        CHECK (activity_type IN
            ('drawing', 'detailing', 'revision', 'meeting', 'study', 'other')),
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

-- ---------- monitored_apps ----------
CREATE TABLE monitored_apps (
    id              TEXT PRIMARY KEY NOT NULL,
    display_name    TEXT NOT NULL,
    process_name    TEXT NOT NULL UNIQUE,
    enabled         INTEGER NOT NULL DEFAULT 1,
    remind_on_open  INTEGER NOT NULL DEFAULT 1,
    remind_on_close INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- ---------- activity_events ----------
CREATE TABLE activity_events (
    id             TEXT PRIMARY KEY NOT NULL,
    event_type     TEXT NOT NULL
        CHECK (event_type IN
            ('app_opened', 'app_closed', 'idle_started', 'idle_ended',
             'timer_started', 'timer_paused', 'timer_resumed', 'timer_stopped')),
    process_name   TEXT,
    detected_at    TEXT NOT NULL,
    metadata_json  TEXT,
    processed      INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL
);

CREATE INDEX idx_activity_events_detected ON activity_events (detected_at);
CREATE INDEX idx_activity_events_type ON activity_events (event_type);
CREATE INDEX idx_activity_events_processed ON activity_events (processed);

-- ---------- settings ----------
-- Tabela de linha unica (id = 1) com campos explicitos e tipados.
CREATE TABLE settings (
    id                              INTEGER PRIMARY KEY CHECK (id = 1),
    idle_detection_enabled          INTEGER NOT NULL DEFAULT 1,
    idle_threshold_minutes          INTEGER NOT NULL DEFAULT 10,
    process_monitoring_enabled      INTEGER NOT NULL DEFAULT 1,
    process_check_interval_seconds  INTEGER NOT NULL DEFAULT 5,
    remind_when_monitored_app_opens INTEGER NOT NULL DEFAULT 1,
    remind_when_monitored_app_closes INTEGER NOT NULL DEFAULT 1,
    rounding_enabled                INTEGER NOT NULL DEFAULT 0,
    rounding_interval_minutes       INTEGER NOT NULL DEFAULT 15,
    rounding_mode                   TEXT NOT NULL DEFAULT 'nearest'
        CHECK (rounding_mode IN ('nearest', 'up', 'down')),
    start_with_windows              INTEGER NOT NULL DEFAULT 0,
    minimize_to_tray                INTEGER NOT NULL DEFAULT 1,
    close_to_tray                   INTEGER NOT NULL DEFAULT 1,
    currency                        TEXT NOT NULL DEFAULT 'BRL',
    locale                          TEXT NOT NULL DEFAULT 'pt-BR'
);

-- Linha unica de configuracoes com os valores iniciais recomendados (secao 8).
INSERT INTO settings (id) VALUES (1);

-- Programas monitorados sugeridos (secao 8). Nem todos estarao instalados.
INSERT INTO monitored_apps
    (id, display_name, process_name, created_at, updated_at)
VALUES
    ('app-acad',     'AutoCAD',  'acad.exe',      '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-revit',    'Revit',    'revit.exe',     '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-sketchup', 'SketchUp', 'sketchup.exe',  '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-eberick',  'Eberick',  'eberick.exe',   '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-qibuilder','QiBuilder','qibuilder.exe', '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z');

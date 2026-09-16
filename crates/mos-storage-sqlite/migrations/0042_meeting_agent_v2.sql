-- Meeting Agent V2: o pipeline que anda sozinho, o Guardian que percebe o fim,
-- o corte, a lixeira, os momentos marcados e os itens com prazo.
--
-- Spec: `docs/superpowers/specs/2026-09-16-meeting-agent-v2-design.md`.
--
-- ---------------------------------------------------------------------------
-- Por que `meetings` e `meeting_insights` sao RECRIADAS
-- ---------------------------------------------------------------------------
--
-- **`meetings` tinha um defeito real.** O estado `paused` nasceu no dominio em
-- 19/08 (card de gravacao) e o CHECK da 0020 nunca o admitiu: pausar gravava um
-- estado que o banco recusava, e o botao falhava. Ninguem tinha apertado. O
-- SQLite nao altera CHECK, entao a unica correcao honesta e recriar — afrouxar
-- a CHECK deixaria de recusar o estado inventado, que e o que ela existe para
-- recusar.
--
-- `meeting_insights` ganha tres tipos (`commitment`, `dependency`, `reference`)
-- pelo mesmo motivo.
--
-- O procedimento e o da 0023/0025, sem reexplicar: `foreign_keys=OFF` e
-- `legacy_alter_table=ON` fora da transacao, RENAME para que as filhas
-- (`meeting_segments`, `meeting_analyses`, `meeting_evidence`, os indices)
-- continuem apontando para o NOME certo, e o `verify_foreign_keys` do
-- `migrate()` conferindo que nenhum orfao sobrou.
--
-- Nada e apagado. Toda coluna nova tem default que significa "como era antes".

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. meetings — `paused`, motivo da parada, corte, lixeira, app associado
-- ---------------------------------------------------------------------------

DROP INDEX IF EXISTS meetings_lifecycle_order;
DROP INDEX IF EXISTS meetings_capturing;
DROP INDEX IF EXISTS meetings_by_project;

ALTER TABLE meetings RENAME TO meetings_old;

CREATE TABLE meetings (
    id                TEXT PRIMARY KEY NOT NULL,
    title             TEXT NOT NULL,
    status            TEXT NOT NULL,
    failed_stage      TEXT,
    failure_message   TEXT,
    lifecycle_state   TEXT NOT NULL DEFAULT 'active',
    source            TEXT NOT NULL DEFAULT 'manual',
    started_at        TEXT NOT NULL,
    ended_at          TEXT,
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    project_id        TEXT REFERENCES projects(id) ON DELETE SET NULL,
    audio_dir         TEXT NOT NULL,
    retention         TEXT NOT NULL DEFAULT 'delete_after_processing',
    audio_deleted_at  TEXT,
    mic_state         TEXT NOT NULL DEFAULT 'capturing',
    mic_lost_at_ms    INTEGER,
    mic_reason        TEXT,
    system_state      TEXT NOT NULL DEFAULT 'capturing',
    system_lost_at_ms INTEGER,
    system_reason     TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    cancelled_at      TEXT,
    notes             TEXT NOT NULL DEFAULT '',

    -- Por que a gravacao parou. NULL em reuniao anterior a V2 e em gravacao
    -- ainda em curso. Nao aparece para a pessoa: e diagnostico e metrica.
    stop_reason       TEXT,
    -- O corte NAO destrutivo. Os chunks ficam; a transcricao le so a faixa.
    -- Os dois em ms relativos ao inicio, na mesma regua dos segmentos.
    trim_start_ms     INTEGER,
    trim_end_ms       INTEGER,
    trim_origin       TEXT,
    -- Quando foi para a lixeira. A exclusao definitiva vem 30 dias depois.
    trashed_at        TEXT,
    -- O programa que tinha o microfone quando a reuniao comecou. Nome de
    -- executavel ou familia de pacote — exatamente o dado da ADR-047, e nada
    -- alem.
    associated_app    TEXT,
    -- Onde o Guardian acha que a conversa acabou, em ms relativos. E o que
    -- alimenta a sugestao de corte depois de uma gravacao esquecida.
    suggested_end_ms  INTEGER,

    CONSTRAINT meetings_title_present CHECK (length(trim(title)) > 0),
    CONSTRAINT meetings_status_known CHECK (status IN (
        'recording', 'paused', 'stopping', 'interrupted', 'recorded', 'transcribing',
        'transcribed', 'analyzing', 'ready', 'failed', 'cancelled'
    )),
    CONSTRAINT meetings_failed_has_stage CHECK (
        (status = 'failed'
            AND failed_stage IS NOT NULL
            AND failed_stage IN ('audio', 'transcription', 'analysis'))
        OR (status <> 'failed' AND failed_stage IS NULL)
    ),
    CONSTRAINT meetings_lifecycle_known CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    CONSTRAINT meetings_source_known CHECK (source IN ('manual', 'calendar', 'detected')),
    CONSTRAINT meetings_retention_known CHECK (retention IN (
        'delete_after_processing', 'keep_24h', 'keep'
    )),
    CONSTRAINT meetings_channel_states_known CHECK (
        mic_state IN ('capturing', 'captured', 'unavailable', 'lost')
        AND system_state IN ('capturing', 'captured', 'unavailable', 'lost')
    ),
    CONSTRAINT meetings_duration_not_negative CHECK (duration_ms >= 0),
    CONSTRAINT meetings_stop_reason_known CHECK (stop_reason IS NULL OR stop_reason IN (
        'manual', 'auto_meeting_ended', 'auto_inactivity', 'crash_recovery',
        'device_failure', 'app_exit'
    )),
    CONSTRAINT meetings_trim_sane CHECK (
        (trim_start_ms IS NULL OR trim_start_ms >= 0)
        AND (trim_end_ms IS NULL OR trim_end_ms > COALESCE(trim_start_ms, 0))
    ),
    CONSTRAINT meetings_trim_origin_known CHECK (trim_origin IS NULL OR trim_origin IN (
        'manual', 'auto', 'suggested'
    )),
    -- Lixeira tem data, e so lixeira tem data.
    CONSTRAINT meetings_trash_has_date CHECK (
        (lifecycle_state = 'trashed' AND trashed_at IS NOT NULL)
        OR (lifecycle_state <> 'trashed' AND trashed_at IS NULL)
    )
) STRICT;

INSERT INTO meetings (
    id, title, status, failed_stage, failure_message, lifecycle_state, source,
    started_at, ended_at, duration_ms, project_id, audio_dir, retention,
    audio_deleted_at, mic_state, mic_lost_at_ms, mic_reason, system_state,
    system_lost_at_ms, system_reason, created_at, updated_at, cancelled_at, notes,
    trashed_at
)
SELECT
    id, title, status, failed_stage, failure_message, lifecycle_state, source,
    started_at, ended_at, duration_ms, project_id, audio_dir, retention,
    audio_deleted_at, mic_state, mic_lost_at_ms, mic_reason, system_state,
    system_lost_at_ms, system_reason, created_at, updated_at, cancelled_at, notes,
    -- Uma reuniao ja 'trashed' antes da V2 (nenhum caminho do app produzia, mas
    -- a CHECK antiga admitia) ganha a data da ultima mudanca, para a CHECK nova
    -- aceitar a linha em vez de recusar a migration inteira.
    CASE WHEN lifecycle_state = 'trashed' THEN updated_at ELSE NULL END
FROM meetings_old;

DROP TABLE meetings_old;

CREATE INDEX meetings_lifecycle_order ON meetings (lifecycle_state, started_at DESC);
CREATE INDEX meetings_capturing ON meetings (status)
    WHERE status IN ('recording', 'paused', 'stopping');
CREATE INDEX meetings_by_project ON meetings (project_id)
    WHERE project_id IS NOT NULL;
CREATE INDEX meetings_trash ON meetings (trashed_at)
    WHERE lifecycle_state = 'trashed';

-- ---------------------------------------------------------------------------
-- 2. meeting_insights — tipos novos, origem e prazo resolvido
-- ---------------------------------------------------------------------------

DROP INDEX IF EXISTS meeting_insights_order;
DROP INDEX IF EXISTS meeting_insights_by_kind;
DROP INDEX IF EXISTS meeting_insights_open_commitments;

ALTER TABLE meeting_insights RENAME TO meeting_insights_old;

CREATE TABLE meeting_insights (
    id                  TEXT PRIMARY KEY NOT NULL,
    meeting_id          TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    kind                TEXT NOT NULL,
    seq                 INTEGER NOT NULL,
    text                TEXT NOT NULL,
    owner               TEXT,
    -- A EXPRESSAO original ("sexta", "amanha"). O nome da coluna fica por
    -- compatibilidade; o dominio a chama de `due_expression`.
    due_hint            TEXT,
    confidence          TEXT NOT NULL DEFAULT 'medium',
    status              TEXT NOT NULL DEFAULT 'proposed',
    created_task_id     TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    created_reminder_id TEXT REFERENCES reminders(id) ON DELETE SET NULL,

    -- De onde o item veio. `spoken`: o Hermes, com evidencia na transcricao.
    -- `written`: um marcador que a pessoa escreveu nas notas (`!task`). `manual`:
    -- a pessoa criou a partir de um trecho da transcricao.
    origin              TEXT NOT NULL DEFAULT 'spoken',
    -- O prazo INTERPRETADO, deterministicamente, a partir de `due_hint` e do
    -- inicio da reuniao. NULL quando a expressao nao resolve.
    due_at              TEXT,
    due_confidence      TEXT,

    CONSTRAINT insights_text_present CHECK (length(trim(text)) > 0),
    CONSTRAINT insights_kind_known CHECK (kind IN (
        'decision', 'my_action', 'other_action', 'commitment', 'deadline',
        'follow_up', 'open_question', 'risk', 'dependency', 'reference', 'topic'
    )),
    CONSTRAINT insights_confidence_known CHECK (confidence IN ('high', 'medium', 'low')),
    CONSTRAINT insights_status_known CHECK (status IN ('proposed', 'accepted', 'dismissed')),
    CONSTRAINT insights_origin_known CHECK (origin IN ('spoken', 'written', 'manual')),
    CONSTRAINT insights_due_confidence_known CHECK (
        due_confidence IS NULL OR due_confidence IN ('high', 'medium', 'low')
    )
) STRICT;

INSERT INTO meeting_insights (
    id, meeting_id, kind, seq, text, owner, due_hint, confidence, status,
    created_task_id, created_reminder_id
)
SELECT
    id, meeting_id, kind, seq, text, owner, due_hint, confidence, status,
    created_task_id, created_reminder_id
FROM meeting_insights_old;

DROP TABLE meeting_insights_old;

CREATE UNIQUE INDEX meeting_insights_order ON meeting_insights (meeting_id, seq);
CREATE INDEX meeting_insights_by_kind ON meeting_insights (meeting_id, kind);
CREATE INDEX meeting_insights_open_commitments ON meeting_insights (kind, status)
    WHERE kind = 'my_action' AND status = 'proposed';

-- ---------------------------------------------------------------------------
-- 3. meeting_segments — a transcricao normalizada, ao lado da crua
-- ---------------------------------------------------------------------------
--
-- `text` NUNCA muda: e o que o whisper disse. `text_normalized` e a leitura com
-- o vocabulario aplicado, e `corrections` lista, em JSON, cada troca feita —
-- original, termo, e se ela e incerta. Vazio significa "igual ao cru".

ALTER TABLE meeting_segments ADD COLUMN text_normalized TEXT;
ALTER TABLE meeting_segments ADD COLUMN corrections TEXT NOT NULL DEFAULT '[]';

-- ---------------------------------------------------------------------------
-- 4. meeting_jobs — o pipeline persistente
-- ---------------------------------------------------------------------------
--
-- UMA linha por reuniao: a reuniao tem um pipeline, e nao uma fila de pedidos.
-- `last_error_message` e a frase para a pessoa, e nunca contem fala.

CREATE TABLE meeting_jobs (
    meeting_id         TEXT PRIMARY KEY NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    stage              TEXT NOT NULL,
    status             TEXT NOT NULL,
    attempt_count      INTEGER NOT NULL DEFAULT 0,
    progress           REAL NOT NULL DEFAULT 0,
    started_at         TEXT,
    finished_at        TEXT,
    last_error_code    TEXT,
    last_error_message TEXT,
    next_retry_at      TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,

    CONSTRAINT jobs_stage_known CHECK (stage IN ('transcription', 'analysis')),
    CONSTRAINT jobs_status_known CHECK (status IN (
        'queued', 'running', 'waiting_retry', 'needs_attention', 'done', 'cancelled'
    )),
    CONSTRAINT jobs_attempts_sane CHECK (attempt_count >= 0),
    CONSTRAINT jobs_progress_sane CHECK (progress >= 0 AND progress <= 1)
) STRICT;

CREATE INDEX meeting_jobs_pending ON meeting_jobs (status, next_retry_at)
    WHERE status IN ('queued', 'running', 'waiting_retry', 'needs_attention');

-- ---------------------------------------------------------------------------
-- 5. meeting_bookmarks — "marcar momento"
-- ---------------------------------------------------------------------------

CREATE TABLE meeting_bookmarks (
    id          TEXT PRIMARY KEY NOT NULL,
    meeting_id  TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    at_ms       INTEGER NOT NULL,
    note        TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,

    CONSTRAINT bookmarks_at_sane CHECK (at_ms >= 0)
) STRICT;

CREATE INDEX meeting_bookmarks_by_meeting ON meeting_bookmarks (meeting_id, at_ms);

-- ---------------------------------------------------------------------------
-- 6. meeting_guardian_events — o que o Guardian sugeriu, e o que a pessoa fez
-- ---------------------------------------------------------------------------
--
-- Metrica local, sem conteudo. E o que responde "o Guardian e irritante?" com
-- numero, e nao com impressao.

CREATE TABLE meeting_guardian_events (
    id          TEXT PRIMARY KEY NOT NULL,
    meeting_id  TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    at          TEXT NOT NULL,
    kind        TEXT NOT NULL,
    confidence  REAL,
    trigger     TEXT NOT NULL DEFAULT '',
    excess_ms   INTEGER,

    CONSTRAINT guardian_kind_known CHECK (kind IN (
        'suggested', 'continued', 'stopped_from_prompt', 'countdown_started',
        'countdown_cancelled', 'auto_stopped', 'long_prompted', 'trim_suggested',
        'trim_applied', 'trim_reverted', 'health_warning'
    ))
) STRICT;

CREATE INDEX meeting_guardian_events_by_meeting ON meeting_guardian_events (meeting_id, at);

PRAGMA user_version = 42;

COMMIT;

PRAGMA legacy_alter_table = OFF;
PRAGMA foreign_keys = ON;

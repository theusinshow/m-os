-- Voice Inbox: a origem `voice` em Capture, e a nota que guarda o audio ate o
-- texto existir.
--
-- ---------------------------------------------------------------------------
-- Por que `captures` e recriada DE NOVO
-- ---------------------------------------------------------------------------
--
-- A 0023 ja a recriou para admitir 'drop', e o SQLite continua sem alterar
-- CHECK. Widening de dominio custa uma recriacao por vez, e nao ha atalho: a
-- alternativa seria uma CHECK frouxa, que deixaria de recusar a origem
-- inventada — exatamente o que ela existe para recusar.
--
-- O procedimento e o MESMO da 0023, e por isso ele nao e reexplicado aqui:
-- `foreign_keys=OFF` e `legacy_alter_table=ON` fora da transacao, RENAME em vez
-- de swap para que as filhas continuem apontando para o nome certo, e o
-- `verify_foreign_keys` do `migrate()` conferindo depois que nenhum orfao
-- sobrou. `tasks` e `resources` apontam para `captures`, entao o DROP da antiga
-- dispararia RESTRICT sem isso.

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. captures — a origem 'voice'
-- ---------------------------------------------------------------------------

DROP TABLE capture_search;

ALTER TABLE captures RENAME TO captures_old;

CREATE TABLE captures (
    id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    source_kind TEXT NOT NULL
        CHECK (source_kind IN ('home', 'quick_capture', 'drop', 'voice')),
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

-- O `source_kind` ganha indice proprio: "o que eu falei ontem" e uma pergunta
-- que o Historico faz, e sem ele ela varre a tabela inteira.
CREATE INDEX captures_source_order
    ON captures(source_kind, captured_at DESC);

CREATE VIRTUAL TABLE capture_search USING fts5(
    content,
    content='captures',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

INSERT INTO capture_search(capture_search) VALUES('rebuild');

-- ---------------------------------------------------------------------------
-- 2. voice_notes — o audio, entre "parei de falar" e "existe texto"
--
-- Ela existe porque `captures.content` e NOT NULL com CHECK de nao-vazio, e o
-- dominio nao tem operacao de reescrever conteudo. Uma Capture nao pode nascer
-- antes da transcricao sem inventar conteudo falso, e reescrever depois
-- destruiria a garantia de que a transcricao original e preservada.
-- ---------------------------------------------------------------------------

CREATE TABLE voice_notes (
    id                 TEXT PRIMARY KEY NOT NULL,

    -- Uma maquina de estados, e nao campos de etapa. `gravou` + `transcreveu` +
    -- `virou capture` como booleanos independentes permitiriam representar
    -- "capturado sem ter gravado", que e impossivel.
    status             TEXT NOT NULL,

    -- Relativo ao diretorio de dados, e derivado do id. Nunca vem do renderer.
    audio_dir          TEXT NOT NULL,

    duration_ms        INTEGER NOT NULL DEFAULT 0,
    -- Pico de RMS na escala 0..1000 que a thread de captura ja produz. Fica
    -- gravado porque e a evidencia de que houve fala: uma nota recusada por
    -- silencio nunca chega aqui, e uma que chegou precisa poder provar por que.
    peak_level         INTEGER NOT NULL DEFAULT 0,

    -- A transcricao ORIGINAL. Nao ha UPDATE dela em lugar nenhum do adapter.
    transcript         TEXT NOT NULL DEFAULT '',
    provider           TEXT NOT NULL DEFAULT '',

    -- ON DELETE SET NULL, e nao RESTRICT: a Capture pode ser excluida
    -- definitivamente pela lixeira, e isso nao pode travar. A nota perde o
    -- destino, nao a existencia.
    capture_id         TEXT REFERENCES captures(id) ON DELETE SET NULL,

    -- O contexto de quando o atalho tocou. SINAL, e nao verdade: quem le
    -- decide o quanto ele vale, e apagar um Project nao pode apagar a memoria
    -- de que a fala aconteceu ali.
    context_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    context_task_id    TEXT REFERENCES tasks(id) ON DELETE SET NULL,

    failure_message    TEXT NOT NULL DEFAULT '',
    audio_deleted_at   TEXT,

    started_at         TEXT NOT NULL,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,

    CONSTRAINT voice_notes_status_known CHECK (status IN (
        'recording', 'recorded', 'transcribing', 'captured', 'failed', 'cancelled'
    )),

    -- `captured` exige as duas coisas que o estado significa.
    --
    -- O `IS NOT NULL` explicito NAO e redundante — e a mesma armadilha que a
    -- 0020 documentou: no SQLite um CHECK que avalia para NULL PASSA, entao
    -- uma comparacao sozinha com coluna nula seria uma guarda que nao guarda.
    CONSTRAINT voice_notes_captured_has_capture CHECK (
        (status = 'captured'
            AND capture_id IS NOT NULL
            AND length(trim(transcript)) > 0)
        OR status <> 'captured'
    ),

    -- `failed` exige a frase. "Falhou" sem motivo e um beco sem saida na tela.
    CONSTRAINT voice_notes_failed_has_message CHECK (
        (status = 'failed' AND length(trim(failure_message)) > 0)
        OR (status <> 'failed' AND length(trim(failure_message)) = 0)
    )
) STRICT;

-- A consulta da reconciliacao de abertura e a do retry: as notas cujo audio
-- ainda guarda informacao que o banco nao tem. Indice parcial porque a
-- esmagadora maioria das linhas esta em `captured` e nunca sera lida por aqui.
CREATE INDEX voice_notes_unfinished
    ON voice_notes(status, started_at)
    WHERE status IN ('recording', 'recorded', 'transcribing', 'failed');

CREATE INDEX voice_notes_capture
    ON voice_notes(capture_id)
    WHERE capture_id IS NOT NULL;

PRAGMA user_version = 25;

COMMIT;

PRAGMA legacy_alter_table = OFF;
PRAGMA foreign_keys = ON;

-- Meeting Agent, V1: a reuniao, sua transcricao, sua analise e a proveniencia
-- que liga uma coisa a outra.
--
-- Cinco tabelas. A escolha que merece justificativa e a QUARTA: `meeting_insights`
-- guarda decisao, acao minha, acao de outro, prazo, follow-up, questao aberta,
-- risco e topico numa tabela so, discriminados por `kind`. Oito tabelas foram
-- consideradas e recusadas pelo argumento textual da ADR-025, que enfrentou a
-- mesma escolha e ficou com tres em vez de nove: tipo vira tabela propria quando
-- precisar de lifecycle ou consulta propria, e nenhum destes oito precisa.

BEGIN IMMEDIATE;

CREATE TABLE meetings (
    id                TEXT PRIMARY KEY NOT NULL,
    title             TEXT NOT NULL,

    -- Uma maquina de estados, e nao tres campos de etapa. `audio_state`,
    -- `transcription_state` e `analysis_state` separados permitiriam
    -- representar estados impossiveis — analisando antes de transcrever.
    status            TEXT NOT NULL,
    -- So existe quando `status = 'failed'`, e existe SEMPRE que ele e failed:
    -- "a gravacao esta segura e a transcricao falhou" e "a gravacao se perdeu"
    -- pedem respostas opostas na tela.
    failed_stage      TEXT,
    failure_message   TEXT,

    -- Ortogonal ao status, como em Capture, Task, Resource e Reminder (ADR-015).
    lifecycle_state   TEXT NOT NULL DEFAULT 'active',
    source            TEXT NOT NULL DEFAULT 'manual',

    started_at        TEXT NOT NULL,
    ended_at          TEXT,
    -- Medida em FRAMES GRAVADOS, nunca por diferenca de relogio. Se um canal
    -- perdeu quatro segundos, a duracao precisa refletir o que existe; um numero
    -- derivado do relogio mentiria justamente no caso em que a verdade importa.
    duration_ms       INTEGER NOT NULL DEFAULT 0,

    -- ON DELETE SET NULL, e nao RESTRICT: apagar um Project nao pode apagar a
    -- memoria de uma reuniao que aconteceu. Ela perde o contexto, nao a
    -- existencia.
    project_id        TEXT REFERENCES projects(id) ON DELETE SET NULL,

    -- Relativo ao diretorio de dados, e derivado do id. Nunca vem do renderer.
    audio_dir         TEXT NOT NULL,
    retention         TEXT NOT NULL DEFAULT 'delete_after_processing',
    audio_deleted_at  TEXT,

    -- Dois canais, dois destinos independentes. Nao e booleano espalhado: sao
    -- dispositivos fisicos distintos cujos destinos divergem de verdade — o
    -- headset cai e o audio do sistema continua.
    mic_state         TEXT NOT NULL DEFAULT 'capturing',
    mic_lost_at_ms    INTEGER,
    mic_reason        TEXT,
    system_state      TEXT NOT NULL DEFAULT 'capturing',
    system_lost_at_ms INTEGER,
    system_reason     TEXT,

    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    cancelled_at      TEXT,

    CONSTRAINT meetings_title_present CHECK (length(trim(title)) > 0),
    CONSTRAINT meetings_status_known CHECK (status IN (
        'recording', 'stopping', 'interrupted', 'recorded', 'transcribing',
        'transcribed', 'analyzing', 'ready', 'failed', 'cancelled'
    )),
    -- O estagio e obrigatorio em `failed` e proibido fora dele.
    --
    -- O `IS NOT NULL` explicito NAO e redundante, e a primeira versao desta
    -- migration errou exatamente aqui. Com `status = 'failed'` e
    -- `failed_stage = NULL`, `failed_stage IN (...)` vale NULL, o primeiro ramo
    -- vira `TRUE AND NULL` = NULL, o segundo vira FALSE, e `NULL OR FALSE` = NULL.
    -- **No SQLite, um CHECK que avalia para NULL PASSA.** A guarda existia e nao
    -- guardava nada; o teste que a exercita e que descobriu.
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
    CONSTRAINT meetings_duration_not_negative CHECK (duration_ms >= 0)
) STRICT;

-- A consulta da lista: ativas, da mais recente para a mais antiga.
CREATE INDEX meetings_lifecycle_order ON meetings (lifecycle_state, started_at DESC);

-- A consulta da RECONCILIACAO DE ABERTURA. Ela roda uma vez por inicializacao e
-- precisa achar zero ou uma linha entre milhares; sem indice parcial ela viraria
-- varredura completa no caminho de abertura do aplicativo.
CREATE INDEX meetings_capturing ON meetings (status)
    WHERE status IN ('recording', 'stopping');

CREATE INDEX meetings_by_project ON meetings (project_id)
    WHERE project_id IS NOT NULL;

CREATE TABLE meeting_segments (
    id            TEXT PRIMARY KEY NOT NULL,
    meeting_id    TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,

    -- Ordem de leitura, ja intercalada entre os dois canais.
    seq           INTEGER NOT NULL,
    -- Relativos ao inicio da reuniao e COMUNS aos dois canais. Isso so e verdade
    -- por causa do keep-alive de silencio provado na Fase 1: sem ele o canal
    -- SYSTEM para no silencio e as duas linhas do tempo divergem.
    start_ms      INTEGER NOT NULL,
    end_ms        INTEGER NOT NULL,

    -- A informacao que a V1 protege acima de qualquer outra.
    channel       TEXT NOT NULL,
    text          TEXT NOT NULL,
    speaker       TEXT,
    confidence    REAL,

    CONSTRAINT segments_channel_known CHECK (channel IN ('mic', 'system')),
    CONSTRAINT segments_range_sane CHECK (start_ms >= 0 AND end_ms >= start_ms)
) STRICT;

CREATE UNIQUE INDEX meeting_segments_order ON meeting_segments (meeting_id, seq);
CREATE INDEX meeting_segments_time ON meeting_segments (meeting_id, start_ms);

CREATE TABLE meeting_analyses (
    -- Uma por reuniao. Reanalisar SUBSTITUI, e nao acumula: duas analises da
    -- mesma reuniao seriam duas verdades sobre o mesmo fato, e a interface teria
    -- de escolher uma sem criterio.
    meeting_id    TEXT PRIMARY KEY NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    summary       TEXT NOT NULL DEFAULT '',
    model         TEXT NOT NULL DEFAULT '',
    produced_at   TEXT NOT NULL,
    -- Quantas janelas de transcricao foram enviadas. Aparece na interface: um
    -- corte de cobertura que nao aparece na tela le-se como "cobriu tudo".
    windows       INTEGER NOT NULL DEFAULT 1,

    CONSTRAINT analyses_windows_positive CHECK (windows >= 1)
) STRICT;

CREATE TABLE meeting_insights (
    id                  TEXT PRIMARY KEY NOT NULL,
    meeting_id          TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    kind                TEXT NOT NULL,
    seq                 INTEGER NOT NULL,
    text                TEXT NOT NULL,

    -- Como foi dito. NAO e chave estrangeira: nao existe entidade Pessoa no
    -- M/OS, e inventar uma a partir de um nome falado criaria um cadastro que
    -- ninguem pediu.
    owner               TEXT,
    -- O texto natural — "amanha", "sexta". NUNCA um instante. Resolver na
    -- analise congelaria uma interpretacao; resolver na confirmacao poe a
    -- interpretacao na tela (`UX-PRINCIPLES` §19).
    due_hint            TEXT,

    confidence          TEXT NOT NULL DEFAULT 'medium',
    status              TEXT NOT NULL DEFAULT 'proposed',

    -- ON DELETE SET NULL nos dois: apagar a Task nao apaga o item. Ele volta a
    -- `proposed` com o vinculo perdido, do mesmo jeito que o Attention System
    -- trata alvo orfao — perder o item porque o objeto mudou de estado seria
    -- apagar a memoria da reuniao.
    created_task_id     TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    created_reminder_id TEXT REFERENCES reminders(id) ON DELETE SET NULL,

    CONSTRAINT insights_text_present CHECK (length(trim(text)) > 0),
    CONSTRAINT insights_kind_known CHECK (kind IN (
        'decision', 'my_action', 'other_action', 'deadline',
        'follow_up', 'open_question', 'risk', 'topic'
    )),
    CONSTRAINT insights_confidence_known CHECK (confidence IN ('high', 'medium', 'low')),
    CONSTRAINT insights_status_known CHECK (status IN ('proposed', 'accepted', 'dismissed'))
) STRICT;

CREATE UNIQUE INDEX meeting_insights_order ON meeting_insights (meeting_id, seq);
CREATE INDEX meeting_insights_by_kind ON meeting_insights (meeting_id, kind);

-- A consulta de "quais compromissos de reunioes eu ainda nao conclui?". Ela e
-- SQL, e nao pergunta de linguagem: onde a regra deterministica serve, ela ganha
-- da IA (`MEETING-AGENT.md` §15.3).
CREATE INDEX meeting_insights_open_commitments ON meeting_insights (kind, status)
    WHERE kind = 'my_action' AND status = 'proposed';

CREATE TABLE meeting_evidence (
    insight_id  TEXT NOT NULL REFERENCES meeting_insights(id) ON DELETE CASCADE,
    -- ON DELETE CASCADE: se o segmento sumir, a evidencia que aponta para ele
    -- deixa de fazer sentido. E ela SO pode sumir junto com a transcricao
    -- inteira, porque segmento nao e apagado individualmente.
    segment_id  TEXT NOT NULL REFERENCES meeting_segments(id) ON DELETE CASCADE,
    seq         INTEGER NOT NULL,

    -- Recorte dentro do texto do segmento. O TEXTO DA CITACAO NAO E GUARDADO:
    -- ele e o texto do segmento. Assim a evidencia nao pode divergir da
    -- transcricao, porque ela e a transcricao.
    char_start  INTEGER,
    char_end    INTEGER,

    PRIMARY KEY (insight_id, seq),
    CONSTRAINT evidence_range_sane CHECK (
        (char_start IS NULL AND char_end IS NULL)
        OR (char_start >= 0 AND char_end > char_start)
    )
) STRICT;

CREATE INDEX meeting_evidence_by_segment ON meeting_evidence (segment_id);

-- ============================================================================
-- Dois indices de busca, e nao um. A separacao e a decisao, nao um detalhe.
-- ============================================================================

-- O GLOBAL indexa a reuniao: titulo, resumo e o texto dos itens. Uma reuniao de
-- uma hora tem ~600 segmentos, e indexa-los aqui faria tres reunioes dominarem
-- qualquer busca por qualquer palavra comum.
CREATE VIRTUAL TABLE meeting_search USING fts5(
    title,
    summary,
    insights,
    tokenize='unicode61 remove_diacritics 2'
);

-- O da TRANSCRICAO serve dois consumidores e nenhum outro: a busca dentro de uma
-- reuniao, e a pergunta que atravessa reunioes ("quando falamos sobre Hermes?").
-- Na Search global esta segunda promove a Meeting e deduplica por reuniao — uma
-- reuniao, um resultado, mesmo que a palavra apareca quarenta vezes.
CREATE VIRTUAL TABLE meeting_transcript_search USING fts5(
    text,
    tokenize='unicode61 remove_diacritics 2'
);

-- As duas tabelas de vinculo, e por que elas existem.
--
-- Um FTS5 com `content=` sabe se reconstruir a partir da tabela fonte, mas exige
-- que a fonte tenha um `rowid` estavel — e os nossos ids sao UUID em coluna
-- TEXT, entao o rowid do SQLite nao tem relacao com a entidade. Sem vinculo, a
-- unica forma de apagar a linha certa do indice seria varrer a tabela virtual.
--
-- Estas duas tabelas guardam esse vinculo. Elas sao DERIVADAS: `rebuild` as
-- reconstroi do zero, e nenhuma delas e fonte de verdade de coisa nenhuma.
CREATE TABLE meeting_search_index (
    rowid      INTEGER PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX meeting_search_index_by_meeting ON meeting_search_index (meeting_id);

CREATE TABLE meeting_transcript_index (
    rowid      INTEGER PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    segment_id TEXT NOT NULL
) STRICT;

CREATE INDEX meeting_transcript_index_by_meeting ON meeting_transcript_index (meeting_id);

PRAGMA user_version = 20;

COMMIT;

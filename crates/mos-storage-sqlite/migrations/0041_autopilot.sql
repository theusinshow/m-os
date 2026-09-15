-- O M/OS deixa de ser um sistema que a pessoa mantem e passa a ser um sistema
-- que mantem a pessoa. Esta migration e a base de tres coisas:
--
--   1. a saude da sincronizacao, persistida — para o app que reabre saber que
--      a ultima rodada de ontem falhou por credencial, e nao fingir que nunca
--      tentou;
--   2. a Task saber QUANDO a pessoa planejou trabalhar nela, separado de QUANDO
--      ela vence — mover para amanha nao pode mudar o prazo;
--   3. o registro do que o Autopilot ja avisou, para nunca avisar duas vezes.
--
-- Nada aqui recria tabela. Tudo e coluna nova com default de "como era antes",
-- ou tabela nova e LOCAL.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. sync_saude — a ultima rodada, entre execucoes
-- ---------------------------------------------------------------------------
--
-- Uma linha so, como `sync_clock`. Local pela mesma razao: descreve o que ESTE
-- aparelho viu. `falhas_seguidas` alimenta a escada do backoff (mos-sync
-- `saude.rs`); `proxima_tentativa_em` e o que o laco respeita e o botao ignora.
CREATE TABLE sync_saude (
    only_row             INTEGER PRIMARY KEY CHECK (only_row = 1),
    ultimo_ok_em         TEXT,
    ultima_rodada_em     TEXT,
    ultimo_erro          TEXT,
    tipo_do_erro         TEXT,
    falhas_seguidas      INTEGER NOT NULL DEFAULT 0,
    proxima_tentativa_em TEXT,
    updated_at           TEXT NOT NULL
) STRICT;

-- ---------------------------------------------------------------------------
-- 2. tasks — planejado, comecado, adiado
-- ---------------------------------------------------------------------------

-- O dia em que a pessoa planejou trabalhar nisto. Data CIVIL `AAAA-MM-DD`, no
-- fuso de quem decidiu, como `daily_sessions.day`. NAO e o prazo: `due_at` diz
-- quando o trabalho vence; `scheduled_for` diz quando a pessoa pretende faze-lo.
-- O End My Day move o segundo e nunca o primeiro.
ALTER TABLE tasks ADD COLUMN scheduled_for TEXT
    CHECK (scheduled_for IS NULL OR scheduled_for GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]');

-- Quando a pessoa clicou "Comecar". NULL e "nao comecou" ou "parou". Diferente
-- de `work_state = 'doing'`: o estado e a coluna do Kanban; o instante e o que
-- permite dizer "voce comecou isto ha 40 minutos e nao concluiu".
ALTER TABLE tasks ADD COLUMN started_at TEXT;

-- Quantas vezes o planejamento foi empurrado para frente. E o sinal de
-- "isto esta sendo evitado" do Attention Engine. Nunca decresce.
ALTER TABLE tasks ADD COLUMN postponed_count INTEGER NOT NULL DEFAULT 0;

-- O planejador pergunta "o que esta marcado para hoje" a cada tela.
CREATE INDEX tasks_scheduled ON tasks(scheduled_for)
    WHERE scheduled_for IS NOT NULL AND lifecycle_state = 'active';

-- ---------------------------------------------------------------------------
-- 3. autopilot_avisos — o que ja foi dito, para nao repetir
-- ---------------------------------------------------------------------------
--
-- LOCAL: o iPhone tocar nao significa que o PC tocou (mesma regra de
-- `attention_notifications`). `chave` e a deduplicacao: tipo + entidade + a
-- janela em que o aviso vale (um dia, uma hora). `adiado_ate` e o snooze.
CREATE TABLE autopilot_avisos (
    id           TEXT PRIMARY KEY NOT NULL,
    chave        TEXT NOT NULL,
    tipo         TEXT NOT NULL,
    entity_kind  TEXT NOT NULL DEFAULT '',
    entity_id    TEXT NOT NULL DEFAULT '',
    titulo       TEXT NOT NULL,
    corpo        TEXT NOT NULL DEFAULT '',
    entregue_em  TEXT NOT NULL,
    adiado_ate   TEXT,
    resolvido_em TEXT
) STRICT;

CREATE UNIQUE INDEX autopilot_avisos_chave ON autopilot_avisos(chave);
CREATE INDEX autopilot_avisos_recentes ON autopilot_avisos(entregue_em DESC);

PRAGMA user_version = 41;

COMMIT;

-- O Reminder deixa de ser "um instante e um titulo" e passa a ser a CAMADA de
-- atencao do M/OS.
--
-- A 0015 ja tinha acertado a fronteira que importa: `reminders` guarda a
-- INTENCAO, `attention_notifications` guarda o que SAIU. Nada aqui reabre essa
-- decisao. O que esta migration acrescenta e o que faltava para a intencao
-- sobreviver a ser ignorada:
--
--   1. persistir ate ser resolvida, com re-alerta que desacelera;
--   2. repetir, com a diferenca entre repetir por CALENDARIO e repetir depois
--      de CONCLUIR;
--   3. varios alertas para a MESMA intencao, sem virar varios lembretes;
--   4. existir sem data nenhuma (Someday);
--   5. cobrar terceiro (follow-up) sem virar uma copia da Task;
--   6. ter historico proprio;
--   7. respeitar silencio.
--
-- ---------------------------------------------------------------------------
-- 1. Por que colunas em `reminders`, e nao uma tabela `reminder_policies`
-- ---------------------------------------------------------------------------
--
-- Porque toda linha teria exatamente uma politica, e uma tabela 1:1 e um JOIN
-- cobrado em toda leitura para guardar o que cabe na propria linha. O agendador
-- le `reminders` a cada acordada; encarecer essa consulta e encarecer a unica
-- coisa que roda sempre.
--
-- Toda coluna nova nasce com default que significa "como era antes": lembrete
-- antigo continua one-time, nao persistente, sem recorrencia e do tipo padrao.
-- Nenhum dado e reescrito.
--
-- ---------------------------------------------------------------------------
-- 2. Por que `reminder_triggers` E tabela
-- ---------------------------------------------------------------------------
--
-- Pelo motivo oposto ao de cima, e e a mesma razao que fez `task_checklist_items`
-- ser tabela na 0039: um ARRAY numa coluna sincroniza como CAMPO, e merge por
-- campo nao serve para conjunto (`SYNC.md` §13). Marcar um alerta como disparado
-- no PC enquanto o celular acrescenta outro terminaria com um dos dois gestos
-- apagado.
--
-- Com uma linha por alerta, os dois gestos sao operacoes sobre ENTIDADES
-- DIFERENTES, e nao ha conflito a resolver.
--
-- E a diferenca de produto que ela compra: "Entrega do projeto" com alerta um
-- dia antes, quatro horas antes, uma hora antes e no prazo e UM lembrete com
-- quatro alertas — nao quatro lembretes competindo pela mesma linha da tela.
--
-- ---------------------------------------------------------------------------
-- 3. Por que `reminder_events` NAO sincroniza
-- ---------------------------------------------------------------------------
--
-- Porque ele descreve o que ESTE aparelho fez, e nao o que a pessoa decidiu. O
-- sync do M/OS e merge por campo sobre entidades (`SYNC.md`); um log
-- append-only e outra forma de dado, e faze-lo viajar exigiria um segundo motor
-- de sincronizacao — que e exatamente o que o pedido proibe.
--
-- O que a pessoa decidiu ja viaja: adiar mexe em `next_due_at` e `snooze_count`,
-- concluir mexe em `status` e `completed_at`. O historico e a memoria local de
-- COMO aquilo apareceu aqui, na mesma familia de `attention_notifications`, que
-- tambem nao viaja e pelo mesmo motivo.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. reminders — a intencao ganha profundidade
-- ---------------------------------------------------------------------------

-- "Nao me deixa esquecer disso."
--
-- Um lembrete persistente NAO se resolve por ter sido entregue. Ele volta a
-- cobrar, em intervalos que crescem, ate a pessoa concluir, adiar, reagendar ou
-- cancelar. Zero e o default porque a maioria dos lembretes e um aviso e nao uma
-- promessa, e transformar todos em promessa e a receita da fadiga que o proprio
-- sistema existe para evitar.
ALTER TABLE reminders ADD COLUMN persistent INTEGER NOT NULL DEFAULT 0
    CHECK (persistent IN (0, 1));

-- Em que degrau do re-alerta ele esta. A politica e deterministica e vive no
-- dominio (`escalation_delay`): 30 min, 1 h, 2 h, e depois disso PARA de tocar
-- e fica no Needs Attention.
--
-- Persistido, e nao derivado de `delivered_count`, porque sao perguntas
-- diferentes: quantas vezes apareceu (escrituracao deste aparelho, que nao
-- viaja) e em que ponto do escalonamento a intencao esta (decisao sobre a
-- intencao, que viaja).
ALTER TABLE reminders ADD COLUMN escalation_step INTEGER NOT NULL DEFAULT 0
    CHECK (escalation_step >= 0);

-- Quando ele tocou pela ultima vez. Base do proximo re-alerta e do "Disparado
-- as 20:30" da folha.
ALTER TABLE reminders ADD COLUMN last_triggered_at TEXT;

-- Quando o proximo re-alerta e devido, para lembrete persistente ja entregue e
-- nao resolvido.
--
-- Coluna propria, e nao `next_due_at` reaproveitada: `next_due_at` carrega o
-- instante ORIGINAL do vencimento, e e ele que sustenta o "atrasado ha 2 h" da
-- tela. Empurrar aquela coluna a cada re-alerta apagaria o tamanho do atraso —
-- a unica informacao que distingue esquecer por dez minutos de esquecer por
-- tres dias.
ALTER TABLE reminders ADD COLUMN retry_at TEXT;

-- A regra de repeticao, como JSON. NULL e o normal, e significa "acontece uma
-- vez" — nao "repeticao desconhecida".
--
-- JSON pelo mesmo motivo que `trigger`: cada forma de repetir (diaria, dias
-- uteis, dias da semana, mensal por dia, mensal por ordinal, anual, a cada N)
-- teria colunas proprias, e a maioria seria NULL em toda linha. O dominio
-- valida ao ler, e regra ilegivel vira erro de integridade em vez de lembrete
-- que silenciosamente para de repetir.
ALTER TABLE reminders ADD COLUMN recurrence TEXT;

-- O tipo do lembrete, para a superficie saber que PERGUNTA fazer.
--
--   'standard'  — "Enviar as bases". Acoes: concluir, adiar, reagendar.
--   'follow_up' — "O Victor respondeu?". Acoes: sim, ainda nao, adiar.
--
-- Uma coluna, e nao uma tabela `follow_ups`: e o mesmo lembrete, com o mesmo
-- ciclo de vida e o mesmo agendador. O que muda e o texto dos dois botoes. Criar
-- uma entidade para isso seria criar a "copia da Task chamada Cobrar Victor"
-- que o pedido recusa explicitamente.
ALTER TABLE reminders ADD COLUMN kind TEXT NOT NULL DEFAULT 'standard'
    CHECK (kind IN ('standard', 'follow_up'));

-- De quem se esta esperando, quando `kind = 'follow_up'`. TEXTO, pela mesma
-- razao que `tasks.waiting_for` e texto na 0039: nao existe cadastro de pessoas
-- no M/OS, e criar um CRM para escrever "Victor" seria a sindrome que o produto
-- recusa.
ALTER TABLE reminders ADD COLUMN waiting_for TEXT NOT NULL DEFAULT '';

-- O indice do re-alerta. Parcial pelo mesmo motivo de `reminders_waiting`: so
-- lembrete persistente com re-alerta marcado participa, e num banco com anos de
-- lembretes isso e um punhado de linhas.
CREATE INDEX reminders_retry ON reminders (retry_at)
    WHERE retry_at IS NOT NULL AND lifecycle_state = 'active';

-- ---------------------------------------------------------------------------
-- 2. reminder_triggers — os varios alertas de UMA intencao
-- ---------------------------------------------------------------------------
--
-- `position` nao existe aqui: a ordem de um empilhamento e a do RELOGIO, e ela
-- ja esta em `scheduled_at`. Guardar uma segunda ordem seria abrir a
-- possibilidade de as duas discordarem.
--
-- `lifecycle_state` existe pelo mesmo motivo da 0039: o apagamento do sync e
-- LOGICO (`sync_projecao.rs` materializa `Delete` como `trashed`), e sem a
-- coluna um alerta apagado no celular ficaria vivo para sempre neste PC.
CREATE TABLE reminder_triggers (
    id              TEXT PRIMARY KEY NOT NULL,
    reminder_id     TEXT NOT NULL REFERENCES reminders(id) ON DELETE CASCADE,

    -- Instante absoluto, em UTC. Calculado por quem criou o empilhamento, a
    -- partir do prazo e do adiantamento escolhido — a mesma regra da
    -- `CORE-FOUNDATION.md` §5: quem conhece o fuso e o lado que recebeu o
    -- clique, e o banco guarda UTC.
    scheduled_at    TEXT NOT NULL,

    -- Como esta linha nasceu. Serve para a tela dizer "1 dia antes" em vez de
    -- repetir uma data que a pessoa ja leu na linha do prazo.
    --   'lead'    — antes do vencimento, por um adiantamento escolhido
    --   'at_due'  — no proprio vencimento
    --   'extra'   — acrescentado a mao, sem relacao com o prazo
    kind            TEXT NOT NULL DEFAULT 'extra'
        CHECK (kind IN ('lead', 'at_due', 'extra')),

    -- Minutos de adiantamento, quando `kind = 'lead'`. NULL nos outros.
    lead_minutes    INTEGER,

    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'fired', 'skipped', 'cancelled')),

    fired_at        TEXT,

    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
) STRICT;

-- A consulta do agendador: qual o proximo alerta pendente deste lembrete.
CREATE INDEX reminder_triggers_pending
    ON reminder_triggers (reminder_id, scheduled_at)
    WHERE status = 'pending' AND lifecycle_state = 'active';

-- A consulta da folha: todos os alertas de um lembrete, na ordem do relogio.
CREATE INDEX reminder_triggers_by_reminder
    ON reminder_triggers (reminder_id, scheduled_at);

-- ---------------------------------------------------------------------------
-- 3. reminder_events — o historico da intencao
-- ---------------------------------------------------------------------------
--
-- So o que responde uma pergunta que alguem faz de verdade: "por que isso
-- apareceu?", "quando isso tocou?", "quantas vezes eu adiei?". Registrar cada
-- leitura seria ruido, e ruido num historico e o mesmo que nao ter historico.
--
-- `detail` e JSON opcional, e nao colunas por tipo de evento: um adiamento
-- guarda ate quando, um escalonamento guarda o degrau, uma recorrencia guarda o
-- proximo instante. Colunas para isso seriam tres colunas nulas em toda linha.
CREATE TABLE reminder_events (
    id           TEXT PRIMARY KEY NOT NULL,
    reminder_id  TEXT NOT NULL REFERENCES reminders(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN (
        'created', 'triggered', 'delivered', 'acknowledged', 'snoozed',
        'rescheduled', 'completed', 'cancelled', 'missed', 'escalated',
        'recurrence_generated', 'edited'
    )),
    at           TEXT NOT NULL,
    detail       TEXT,
    created_at   TEXT NOT NULL
) STRICT;

CREATE INDEX reminder_events_by_reminder ON reminder_events (reminder_id, at DESC);

-- ---------------------------------------------------------------------------
-- 4. attention_settings — o silencio
-- ---------------------------------------------------------------------------
--
-- Linha unica, mesmo desenho de `tracking_settings` (0011): a configuracao e do
-- APARELHO, nao da pessoa, e por isso nao sincroniza. Silenciar o celular a
-- noite nao pode silenciar o PC do escritorio.
--
-- Minutos desde a meia-noite LOCAL, e nao "HH:MM" em texto: comparar dois
-- inteiros nao tem como dar errado, e formatar para a tela e trabalho de quem
-- desenha. A janela pode ATRAVESSAR a meia-noite (start 1380 = 23:00,
-- end 480 = 08:00), e o dominio trata esse caso — e o caso normal, alias.
CREATE TABLE attention_settings (
    id                    INTEGER PRIMARY KEY CHECK (id = 1),

    quiet_enabled         INTEGER NOT NULL DEFAULT 1 CHECK (quiet_enabled IN (0, 1)),
    quiet_start_minute    INTEGER NOT NULL DEFAULT 0
        CHECK (quiet_start_minute BETWEEN 0 AND 1439),
    quiet_end_minute      INTEGER NOT NULL DEFAULT 480
        CHECK (quiet_end_minute BETWEEN 0 AND 1439),

    -- Urgente pode furar o silencio, e isso e OPT-IN. O default e respeitar: um
    -- sistema que decide sozinho que algo merece acordar a pessoa perde o
    -- direito de ser levado a serio quando algo realmente merecer.
    quiet_allow_urgent    INTEGER NOT NULL DEFAULT 0
        CHECK (quiet_allow_urgent IN (0, 1)),

    -- Canal do sistema operacional ligado. Opt-out e nao opt-in: quem instalou
    -- um sistema de lembretes quer ser lembrado fora da janela do app.
    os_channel_enabled    INTEGER NOT NULL DEFAULT 1
        CHECK (os_channel_enabled IN (0, 1)),

    -- O deslocamento local DESTE aparelho, em minutos.
    --
    -- Existe porque "silencio das 23 as 08" e uma pergunta sobre o relogio de
    -- parede de quem olha, e o processo que decide isso nem sempre tem como
    -- perguntar ao sistema: o `time` sem a feature `local-offset` nao sabe o
    -- fuso, e a VPS que roda o `mos-web` esta em UTC enquanto a pessoa nao esta.
    --
    -- O desktop reescreve esta coluna a cada abertura, com o deslocamento real
    -- da maquina; o servidor fica com o que estiver gravado. Nao sincroniza,
    -- pela mesma razao do resto desta tabela: e do APARELHO.
    --
    -- -180 e o horario de Brasilia, e e o default por ser onde o dono do M/OS
    -- esta. Um default de zero faria o silencio da primeira noite acontecer tres
    -- horas cedo demais, que e pior do que um default explicito e corrigivel.
    local_offset_minutes  INTEGER NOT NULL DEFAULT -180
        CHECK (local_offset_minutes BETWEEN -840 AND 840)
) STRICT;

INSERT INTO attention_settings (id) VALUES (1);

PRAGMA user_version = 40;

COMMIT;

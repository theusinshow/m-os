-- Attention System, P0: a intencao de ser lembrado, e cada entrega dela.
--
-- Duas tabelas e nao uma, porque sao duas perguntas diferentes: `reminders`
-- guarda a INTENCAO, `attention_notifications` guarda o que saiu. Sem a
-- segunda nao ha como distinguir "o lembrete falhou" de "o lembrete foi
-- entregue e ignorado" — e sao situacoes que pedem respostas opostas.

BEGIN IMMEDIATE;

CREATE TABLE reminders (
    id                TEXT PRIMARY KEY NOT NULL,
    title             TEXT NOT NULL,
    body              TEXT NOT NULL DEFAULT '',

    -- Alvo polimorfico como par (tipo, id), e nao tabela generica de arestas:
    -- a ADR-012 recusou grafo generico e aceitou o custo de que tipo novo exige
    -- migration. Sem foreign key porque uma FK por braco multiplicaria colunas
    -- nulas; a integridade fica na aplicacao, que tambem precisa tratar alvo
    -- orfao sem apagar o Reminder junto.
    target_type       TEXT,
    target_id         TEXT,

    -- O trigger vai como JSON com discriminante `kind`. So `at` existe no P0;
    -- guardar JSON evita uma coluna nova por braco futuro, e o dominio ja
    -- recusa `kind` que nao conhece.
    trigger_kind      TEXT NOT NULL,
    trigger           TEXT NOT NULL,

    priority          TEXT NOT NULL,
    status            TEXT NOT NULL,
    source            TEXT NOT NULL,

    snooze_allowed    INTEGER NOT NULL DEFAULT 1,
    privacy           TEXT NOT NULL DEFAULT 'show_content',

    -- Derivado do trigger e persistido mesmo assim: e a coluna que o agendador
    -- consulta a cada acordada. Recalcular o trigger de todos os Reminders por
    -- tick trocaria uma query indexada por um laco.
    next_due_at       TEXT,

    snooze_count      INTEGER NOT NULL DEFAULT 0,
    delivered_count   INTEGER NOT NULL DEFAULT 0,

    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    completed_at      TEXT,
    lifecycle_state   TEXT NOT NULL DEFAULT 'active',

    CONSTRAINT reminders_title_present CHECK (length(trim(title)) > 0),
    -- Alvo e tudo ou nada: metade de um alvo e um vinculo que nao resolve.
    CONSTRAINT reminders_target_whole CHECK (
        (target_type IS NULL AND target_id IS NULL)
        OR (target_type IS NOT NULL AND target_id IS NOT NULL)
    ),
    CONSTRAINT reminders_status_known CHECK (status IN (
        'scheduled', 'due', 'delivered', 'acknowledged', 'snoozed',
        'completed', 'cancelled', 'missed', 'expired'
    )),
    CONSTRAINT reminders_priority_known CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    CONSTRAINT reminders_source_known CHECK (source IN ('user', 'hermes', 'capture', 'system')),
    CONSTRAINT reminders_privacy_known CHECK (privacy IN ('show_content', 'title_only', 'hidden')),
    CONSTRAINT reminders_lifecycle_known CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    -- `completed_at` e exclusivo de `completed`: entrar carimba, sair limpa.
    -- Mesma regra que `tasks.completed_at` ja segue desde a 0007.
    CONSTRAINT reminders_completed_stamp CHECK (
        (status = 'completed' AND completed_at IS NOT NULL)
        OR (status <> 'completed' AND completed_at IS NULL)
    )
);

-- O indice que o agendador usa a cada acordada. Parcial de proposito: so
-- interessa o que ainda espera, e num banco com anos de lembretes concluidos a
-- diferenca entre varrer tudo e varrer o pendente e a diferenca entre o app
-- abrir rapido e nao abrir.
CREATE INDEX reminders_waiting ON reminders (next_due_at)
    WHERE status IN ('scheduled', 'snoozed') AND lifecycle_state = 'active';

-- Para o badge e para o Attention Center.
CREATE INDEX reminders_attention ON reminders (status)
    WHERE lifecycle_state = 'active';

-- Para achar os lembretes de uma Task ao abrir a Task.
CREATE INDEX reminders_target ON reminders (target_type, target_id)
    WHERE target_type IS NOT NULL;

CREATE TABLE attention_notifications (
    id              TEXT PRIMARY KEY NOT NULL,
    reminder_id     TEXT NOT NULL REFERENCES reminders(id) ON DELETE CASCADE,
    channel         TEXT NOT NULL,

    -- Enquanto existir uma entrega viva com a mesma chave, outra nao e criada:
    -- a existente e atualizada. E o que impede "Task atrasada" quatro vezes
    -- seguidas sem impedir que a intencao continue una.
    dedupe_key      TEXT NOT NULL,

    status          TEXT NOT NULL,
    level           TEXT NOT NULL DEFAULT 'normal',

    created_at      TEXT NOT NULL,
    delivered_at    TEXT,
    resolved_at     TEXT,

    -- Motivo da falha, quando houve. Guardado porque "nao apareceu as 15h"
    -- precisa ter resposta, e porque falha de canal nunca resolve o Reminder.
    failure         TEXT,

    CONSTRAINT notifications_channel_known CHECK (channel IN ('in_app', 'windows', 'tray')),
    CONSTRAINT notifications_status_known CHECK (status IN (
        'queued', 'delivering', 'delivered', 'seen', 'acted', 'dismissed', 'failed'
    )),
    CONSTRAINT notifications_level_known CHECK (level IN ('quiet', 'normal', 'important', 'critical'))
);

CREATE INDEX attention_notifications_by_reminder ON attention_notifications (reminder_id);

-- O indice do dedupe: so entrega viva participa.
CREATE INDEX attention_notifications_live_dedupe ON attention_notifications (dedupe_key)
    WHERE status IN ('queued', 'delivering', 'delivered');

-- ON DELETE CASCADE acima apaga as entregas junto com o Reminder. Isso NAO
-- contradiz a ADR-035 ("desfazer arquiva, nunca apaga"): o caminho normal de
-- sumir da tela e `lifecycle_state`, e exclusao definitiva de Reminder e
-- operacao explicita. O cascade so evita entrega orfa quando ela acontece.

PRAGMA user_version = 15;

COMMIT;

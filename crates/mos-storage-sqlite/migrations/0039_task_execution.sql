-- A Task deixa de ser uma linha de titulo e passa a representar TRABALHO.
--
-- O pedido que originou isto e de uma frase so: uma Task nao deve ser apenas
-- "fazer X"; ela precisa conseguir representar "fazer X" com varios passos
-- menores. O que faltava nao era tela — era esquema. Nada aqui inventa um
-- sistema paralelo: o lembrete continua sendo o do `ATTENTION-SYSTEM.md`, a
-- referencia continua sendo um Resource, e o Project continua sendo o Project.
--
-- ---------------------------------------------------------------------------
-- 1. Por que checklist e TABELA, e nao um JSON dentro de `tasks`
-- ---------------------------------------------------------------------------
--
-- Um array numa coluna sincroniza como CAMPO, e merge por campo nao serve para
-- conjunto — e a mesma razao que fez as juncoes do Knowledge Graph virarem
-- relacao de primeira classe (`SYNC.md` §13). Com JSON, marcar o item A no PC
-- enquanto o celular acrescenta o item B terminaria com um dos dois gestos
-- apagado, porque o campo inteiro seria substituido pelo mais recente.
--
-- Com uma linha por item, os dois gestos sao operacoes sobre ENTIDADES
-- DIFERENTES: nao ha conflito nenhum a resolver. E o unico desenho em que o
-- cenario do pedido — desktop marca A, mobile acrescenta B, os dois sobrevivem
-- — e verdadeiro por construcao, e nao por sorte de ordem.
--
-- Alem disso: `GROUP BY task_id` responde o progresso do card do Kanban numa
-- consulta indexada. O mesmo numero, tirado de um JSON, exigiria ler e
-- desserializar a coluna de toda Task do quadro.
--
-- ---------------------------------------------------------------------------
-- 2. Por que `due_at` entra AGORA, contra a decisao D-1
-- ---------------------------------------------------------------------------
--
-- A D-1 (`ATTENTION-SYSTEM.md` §35) recusou `Task.due_at` em 2026-08-18, e o
-- custo aceito foi escrito: sem prazo, o M/OS nao tem ancora de tempo futuro, e
-- o Reminder ocupa esse lugar. A decisao foi boa enquanto a Task era um titulo.
--
-- O que mudou: o proprietario pediu, em 2026-09-08, que prazo e lembrete sejam
-- coisas DISTINTAS na mesma Task — prazo hoje 17:00, lembrete 16:30. Isso e
-- exatamente o que a D-1 tornava inexprimivel. Ver a ADR-066.
--
-- A distincao que a coluna carrega:
--   `due_at`  = quando o trabalho VENCE. Vive na Task, aparece no Calendar.
--   Reminder  = quando o M/OS INTERROMPE. Vive em `reminders`, aponta para a
--               Task pelo par (target_type, target_id) que a 0015 ja criou.
-- Um nao substitui o outro, e nenhum dos dois ganha uma segunda implementacao.
--
-- ---------------------------------------------------------------------------
-- 3. Por que ALTER, e nao a recriacao que a 0007 fez
-- ---------------------------------------------------------------------------
--
-- `FEATURE-DEVELOPMENT.md` §4: prefira acrescentar a alterar. Aqui nao ha CHECK
-- existente para trocar — sao colunas novas —, e recriar `tasks` significaria
-- soltar e reconstruir `task_search` e atravessar as chaves estrangeiras que
-- apontam para ela (`meeting_insights`, `academic_assignments`, `voice_notes`,
-- `ingestions`, e agora ela mesma). Risco sem ganho.
--
-- Toda coluna nova nasce com um default que significa "como era antes": Task
-- antiga continua valida, sem prazo, prioridade normal e sem checklist. Nenhum
-- dado e reescrito.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- 1. tasks — o que a execucao precisa saber
-- ---------------------------------------------------------------------------

-- Quando o trabalho vence. NULL e o estado normal, e significa "sem prazo" —
-- nao "prazo desconhecido". A maioria das Tasks continua sem prazo nenhum, e
-- e assim que tem de ser: prazo obrigatorio e como cobrar atraso de tarefa que
-- ninguem prometeu para dia nenhum.
ALTER TABLE tasks ADD COLUMN due_at TEXT;

-- Quatro degraus, os mesmos que `reminders.priority` ja gravou na 0015. Uma
-- segunda escala aqui faria "alta" significar duas coisas no mesmo sistema.
-- `normal` e o default e e visualmente neutro: prioridade que colore tudo nao
-- prioriza nada.
ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'normal'
    CHECK (priority IN ('low', 'normal', 'high', 'urgent'));

-- Em MINUTOS, pelo mesmo motivo que `mos.time.record` recebe minutos: tres
-- jeitos de escrever uma hora e meia dao tres jeitos de errar um quarto dela.
-- NULL significa "nao estimei", que e diferente de zero.
ALTER TABLE tasks ADD COLUMN estimate_minutes INTEGER;

-- A Subtask e uma Task de verdade com pai, e nao um terceiro tipo.
--
-- Assim ela ganha estado, prazo, prioridade e checklist proprios sem nenhuma
-- linha nova de codigo. O esquema aceita profundidade qualquer; a INTERFACE
-- mostra um nivel so, e essa e a fronteira certa: o banco nao precisa proibir
-- o que a tela nao oferece.
--
-- ON DELETE SET NULL, e nao CASCADE: apagar o pai nao pode levar trabalho
-- junto. E a mesma escolha que `tasks.project_id` fez na 0002.
ALTER TABLE tasks ADD COLUMN parent_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL;

-- "Bloqueada por" — uma so, e de proposito.
--
-- Uma lista de dependencias exigiria tabela de arestas, e a ADR-012 recusou
-- grafo generico. Uma coluna responde a pergunta real (gerar o PDF depende de
-- terminar a revisao) sem abrir a porta para Gantt. No dia em que uma Task
-- tiver duas travas de verdade, o caminho e uma tabela — nao uma segunda
-- coluna.
ALTER TABLE tasks ADD COLUMN blocked_by_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL;

-- Waiting For: TEXTO, e nao uma entidade Person.
--
-- Nao existe cadastro de pessoas no M/OS, e criar um CRM para escrever "Victor"
-- seria exatamente a sindrome que o pedido recusa. Vazio significa "nao esta
-- esperando ninguem" — e a ausencia e o estado normal.
--
-- `follow_up_at` e QUANDO COBRAR, e nao quando vence: sao perguntas diferentes,
-- e colapsa-las faria a Task parecer atrasada por culpa de terceiro.
ALTER TABLE tasks ADD COLUMN waiting_for TEXT NOT NULL DEFAULT '';
ALTER TABLE tasks ADD COLUMN follow_up_at TEXT;

-- O quadro filtra por pai a cada desenho: subtask nao vira card solto, entao a
-- consulta do Kanban pergunta `parent_task_id IS NULL` sempre.
CREATE INDEX tasks_parent ON tasks(parent_task_id) WHERE parent_task_id IS NOT NULL;

-- O que vence, e so o que vence. Parcial pelo mesmo motivo de
-- `reminders_waiting`: num banco com anos de Tasks, varrer as sem prazo para
-- achar as com prazo e trabalho por nada.
CREATE INDEX tasks_due ON tasks(due_at)
    WHERE due_at IS NOT NULL AND lifecycle_state = 'active';

-- ---------------------------------------------------------------------------
-- 2. task_checklist_items — o passo dentro da Task
-- ---------------------------------------------------------------------------
--
-- `completed_at` e DATA e nao booleano, pela mesma razao de
-- `project_tracking.paid_at` (0026): "feito" responde sim; "feito as 14:32"
-- responde a pergunta que aparece depois. NULL significa aberto.
--
-- `position` e INTEGER e a ordem e do usuario: um checklist e uma sequencia, e
-- ordenar por `created_at` faria arrastar um item nao significar nada.
--
-- `lifecycle_state` existe porque o apagamento do sync e LOGICO
-- (`sync_projecao.rs` materializa `Delete` como `trashed`). Sem a coluna, um
-- item apagado no celular chegaria aqui como UPDATE contra coluna inexistente,
-- e o item ficaria vivo para sempre neste PC.
CREATE TABLE task_checklist_items (
    id              TEXT PRIMARY KEY NOT NULL,
    task_id         TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    label           TEXT NOT NULL CHECK (length(trim(label)) > 0),
    position        INTEGER NOT NULL DEFAULT 0,
    completed_at    TEXT,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
) STRICT;

-- A consulta de toda tela que mostra checklist: os itens de uma Task, na ordem.
-- E tambem o indice do progresso do quadro, que e um `GROUP BY task_id`.
CREATE INDEX task_checklist_order
    ON task_checklist_items(task_id, position, created_at);

-- A busca acha a Task pelo texto do PASSO.
--
-- Indice proprio, e nao coluna nova em `task_search`: aquela tabela e de
-- conteudo externo (`content='tasks'`), entao ela so pode ter colunas que
-- `tasks` tem. O padrao ja existe no M/OS — a transcricao de reuniao tem indice
-- proprio e chega na busca PROMOVENDO a Meeting (`MEETING-AGENT.md` §15). Aqui
-- o acerto promove a Task, pela mesma razao: ninguem procura um checkbox, se
-- procura o trabalho em que ele esta.
--
-- A coluna do indice se chama `label` e nao `content` porque `content=` e opcao
-- do proprio fts5: uma coluna com esse nome colide com a declaracao da tabela.
CREATE VIRTUAL TABLE task_checklist_search USING fts5(
    label,
    content='task_checklist_items',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

-- ---------------------------------------------------------------------------
-- 3. resource_tasks — a referencia da Task
-- ---------------------------------------------------------------------------
--
-- Copia estrutural de `resource_projects` (0009), e pela mesma razao: uma Task
-- precisa de PDF, imagem, link e nota presos a ela, e tudo isso ja e um
-- Resource. Um segundo sistema de anexo seria duplicar o que existe.
--
-- N-para-N porque o mesmo PDF serve a duas Tasks — e forcar uma so seria uma
-- decisao que o produto nao precisa tomar.
CREATE TABLE resource_tasks (
    resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (resource_id, task_id)
) STRICT;

CREATE INDEX resource_tasks_task_order
    ON resource_tasks(task_id, created_at DESC);

PRAGMA user_version = 39;

COMMIT;

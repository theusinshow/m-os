-- O Workspace dos itens soltos (ADR-093).
--
-- A Task com Project ja herda os Workspaces dele por `project_workspaces`. O
-- que faltava era a Task SOLTA, a Capture e a conversa do Hermes poderem nascer
-- num Workspace. Uma coluna, e nao uma juncao: cada uma nasce em no maximo um,
-- e uma juncao seria pagar no sync e na tela por uma flexibilidade que ninguem
-- pediu.
--
-- NULL e "sem Workspace", que aparece em TODOS. E o estado de tudo o que ja
-- existe, entao nada some depois desta migration — o historico so deixa de
-- aparecer em todo lugar quando for triado.
--
-- `ON DELETE SET NULL`: apagar o Workspace devolve o item ao "sem Workspace",
-- e nunca leva o item junto. Arquivar nao mexe aqui; quem decide o que um
-- Workspace arquivado faz com os itens dele e `mos_core::pertence`.

BEGIN IMMEDIATE;

ALTER TABLE tasks ADD COLUMN workspace_id TEXT
    REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE captures ADD COLUMN workspace_id TEXT
    REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE conversations ADD COLUMN workspace_id TEXT
    REFERENCES workspaces(id) ON DELETE SET NULL;

CREATE INDEX tasks_workspace ON tasks(workspace_id)
    WHERE workspace_id IS NOT NULL;
CREATE INDEX captures_workspace ON captures(workspace_id)
    WHERE workspace_id IS NOT NULL;
CREATE INDEX conversations_workspace ON conversations(workspace_id)
    WHERE workspace_id IS NOT NULL;

PRAGMA user_version = 43;

COMMIT;

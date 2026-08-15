-- Resource ganha contexto: o elo que faltava na cadeia da fase 3 do ROADMAP
-- (Task -> Project -> Workspace -> Resource -> App).
--
-- Copia estrutural de app_workspaces (0004_workspaces.sql). N-para-N porque uma
-- referencia pode servir a dois contextos: motion.dev vale em Web Design e pode
-- valer em Learning. Forcar um so seria uma decisao que o produto nao precisa
-- tomar, e uma coluna em resources que um dia viraria tabela mesmo assim.
--
-- Nenhuma linha nasce preenchida. No primeiro dia todo Workspace tem zero
-- resources vinculados, e a interface precisa dizer isso em vez de parecer
-- acervo vazio — ver a secao 5.3 do spec.

BEGIN IMMEDIATE;

CREATE TABLE resource_workspaces (
    resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (resource_id, workspace_id)
) STRICT;

CREATE INDEX resource_workspaces_workspace_order
    ON resource_workspaces(workspace_id, created_at DESC);

PRAGMA user_version = 9;

COMMIT;

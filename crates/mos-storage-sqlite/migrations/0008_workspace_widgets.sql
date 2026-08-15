-- A escolha de widgets da Home, por Workspace.
--
-- A LINHA SIGNIFICA OCULTO. Ausencia de linha = o widget aparece. A inversao e
-- deliberada e tem tres consequencias, todas desejadas:
--   1. Workspace novo mostra tudo sem nenhuma escrita;
--   2. widget criado depois nasce visivel em todos os Workspaces — guardar o
--      visivel faria cada recurso novo nascer invisivel para quem ja usa o app;
--   3. a tabela fica vazia para quem nunca configurou nada, que e a maioria.
--
-- widget_id e string opaca: o core nao conhece o catalogo, que vive no front em
-- HOME_WIDGETS. O CHECK garante formato, nao vocabulario — enum aqui faria de
-- cada widget novo uma migration. Linha orfa de widget extinto e inofensiva: o
-- front ignora id que nao esta no catalogo.
--
-- Sem indice: a PRIMARY KEY (workspace_id, widget_id) ja serve as buscas por
-- workspace, e a tabela tem no maximo sete linhas por Workspace.

BEGIN IMMEDIATE;

CREATE TABLE workspace_hidden_widgets (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    widget_id TEXT NOT NULL CHECK (widget_id GLOB '[a-z][a-z0-9_]*'),
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, widget_id)
) STRICT;

PRAGMA user_version = 8;

COMMIT;

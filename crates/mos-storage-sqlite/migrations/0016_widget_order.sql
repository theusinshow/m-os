-- A ordem dos widgets da Home, por Workspace.
--
-- Irma da `workspace_hidden_widgets` (0008), e herda a mesma inversao: **a
-- AUSENCIA de linha significa a ordem do catalogo.** As consequencias sao as
-- mesmas, e todas desejadas:
--   1. Workspace novo mostra a ordem do desenho sem nenhuma escrita;
--   2. widget criado depois nao se enfia no meio de um arranjo que a pessoa
--      montou — sem linha, ele vai para o fim;
--   3. a tabela fica vazia para quem nunca arrastou nada, que e a maioria.
--
-- `widget_id` e string opaca pelo mesmo motivo da 0008: o core nao conhece o
-- catalogo, que vive no front em `HOME_WIDGETS`. O CHECK garante formato, nao
-- vocabulario — enum aqui faria de cada widget novo uma migration, e linha orfa
-- de widget extinto e inofensiva porque o dominio a ignora.
--
-- `position` NAO e UNIQUE de proposito. Reordenar grava a secao inteira numa
-- transacao, mas uma escrita interrompida no meio deixaria posicao repetida — e
-- uma constraint aqui transformaria isso em erro de gravacao em vez de um
-- desempate. `order_widgets` desempata pela ordem do catalogo e nao perde
-- widget nenhum; e melhor uma Home levemente fora de ordem que uma Home que
-- recusa abrir.
--
-- Sem indice proprio: a PRIMARY KEY ja serve a busca por workspace, e a tabela
-- tem no maximo uma linha por widget do catalogo.

BEGIN IMMEDIATE;

CREATE TABLE workspace_widget_order (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    widget_id TEXT NOT NULL CHECK (widget_id GLOB '[a-z][a-z0-9_]*'),
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, widget_id)
) STRICT;

PRAGMA user_version = 16;

COMMIT;

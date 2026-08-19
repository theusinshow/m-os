-- A Home sem Workspace tambem esconde widget.
--
-- Irma da 0018, e pelo mesmo motivo: `workspace_id` era NOT NULL, entao quem
-- nunca criou Workspace nenhum — o estado de quem instala e comeca a usar — nao
-- tinha onde gravar a escolha, e a feature ficava fora de alcance. A visao
-- "Todos" e um contexto de verdade, e o unico de muita gente.
--
-- NULL significa "Todos", e nao "faltando": "Todos" E a ausencia de Workspace.
--
-- A INVERSAO DA 0008 CONTINUA VALENDO, e e o que esta migration nao pode
-- quebrar: **a linha significa OCULTO**, e ausencia de linha significa visivel.
-- Uma linha `(NULL, 'timer')` diz "o `timer` esta oculto em Todos", e nao "esta
-- oculto em algum lugar". Widget novo continua nascendo visivel em toda parte,
-- porque nasce sem linha.
--
-- Reconstruir a tabela e obrigatorio: o SQLite adiciona coluna, mas nao tira um
-- NOT NULL. Nenhuma outra tabela referencia esta, entao nao ha FK de terceiros
-- para desligar durante a troca.
--
-- E a PRIMARY KEY vira indice unico sobre `COALESCE(workspace_id, '')` pela
-- mesma armadilha da 0018: no SQLite coluna de PRIMARY KEY aceita NULL, e NULL
-- nunca colide com NULL. Mantida como PK, esconder o mesmo widget duas vezes em
-- "Todos" empilharia linhas em silencio — inofensivo para a leitura, que so
-- pergunta se existe alguma, mas lixo que cresce a cada clique.
--
-- A FK sobrevive com o ON DELETE CASCADE: apagar um Workspace continua levando
-- as escolhas dele. As de "Todos" nao pertencem a Workspace nenhum e nao morrem
-- com nenhum — chave estrangeira nula nunca e cobrada, e aqui isso e o
-- comportamento desejado.

BEGIN IMMEDIATE;

CREATE TABLE workspace_hidden_widgets_novo (
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    widget_id TEXT NOT NULL CHECK (widget_id GLOB '[a-z][a-z0-9_]*'),
    created_at TEXT NOT NULL
) STRICT;

INSERT INTO workspace_hidden_widgets_novo (workspace_id, widget_id, created_at)
SELECT workspace_id, widget_id, created_at FROM workspace_hidden_widgets;

DROP TABLE workspace_hidden_widgets;

ALTER TABLE workspace_hidden_widgets_novo RENAME TO workspace_hidden_widgets;

CREATE UNIQUE INDEX workspace_hidden_widgets_escopo
    ON workspace_hidden_widgets (COALESCE(workspace_id, ''), widget_id);

PRAGMA user_version = 19;

COMMIT;

-- A Home sem Workspace tambem se arruma.
--
-- Ate aqui `workspace_id` era NOT NULL, e a consequencia so apareceu com o app
-- aberto: quem nao tem Workspace nenhum criado — que e o estado de quem instala
-- e comeca a usar — via o botao "Arrumar" desligado e a feature inteira fora de
-- alcance. Nao havia onde gravar. A visao "Todos" e um contexto de verdade, e o
-- unico de muita gente; ela precisa de arranjo proprio.
--
-- NULL significa "Todos", e nao "faltando". A escolha e honesta: "Todos" E a
-- ausencia de Workspace, e e assim que o front ja modela isso. Um id sentinela
-- ('*', '') diria a mesma coisa mentindo sobre o tipo, e ainda custaria a
-- integridade referencial.
--
-- POR QUE RECONSTRUIR A TABELA: o SQLite sabe adicionar coluna, mas nao sabe
-- tirar um NOT NULL. Entao e a danca padrao — tabela nova, copia, troca de nome.
-- Nenhuma outra tabela referencia esta, entao nao ha FK de terceiros para
-- desligar durante a troca.
--
-- E POR QUE A PRIMARY KEY VIRA INDICE: no SQLite, coluna de PRIMARY KEY aceita
-- NULL — uma incompatibilidade com o padrao que ele carrega por
-- retrocompatibilidade — e NULL nunca colide com NULL. Mantida como PK, a
-- tabela aceitaria QUINZE linhas de `timer` para "Todos", e o arranjo viraria
-- lixo silencioso. O indice unico sobre `COALESCE(workspace_id, '')` fecha isso:
-- ali "Todos" tem um valor concreto, e colide consigo mesmo como deve.
--
-- A FK sobrevive com o ON DELETE CASCADE que a 0016 tinha: apagar um Workspace
-- continua levando o arranjo dele junto. Linha com `workspace_id` NULL nao e
-- alcancada por FK nenhuma — no SQL, chave estrangeira nula nunca e cobrada —,
-- e e exatamente o que se quer: o arranjo de "Todos" nao pertence a Workspace
-- nenhum e nao morre com nenhum.

BEGIN IMMEDIATE;

CREATE TABLE workspace_widget_layout_novo (
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    widget_id TEXT NOT NULL CHECK (widget_id GLOB '[a-z][a-z0-9_]*'),
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    section TEXT CHECK (section IS NULL OR section GLOB '[a-z][a-z0-9_]*'),
    span INTEGER CHECK (span IS NULL OR (span >= 1 AND span <= 12))
) STRICT;

INSERT INTO workspace_widget_layout_novo
    (workspace_id, widget_id, position, created_at, section, span)
SELECT workspace_id, widget_id, position, created_at, section, span
  FROM workspace_widget_layout;

DROP TABLE workspace_widget_layout;

ALTER TABLE workspace_widget_layout_novo RENAME TO workspace_widget_layout;

CREATE UNIQUE INDEX workspace_widget_layout_escopo
    ON workspace_widget_layout (COALESCE(workspace_id, ''), widget_id);

PRAGMA user_version = 18;

COMMIT;

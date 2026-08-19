-- O leque: cinco petalas fixas no rodape ao centro.
--
-- TABELA VAZIA SIGNIFICA "O QUE O DESENHO ESCOLHEU", e nao "nada fixado". E a
-- mesma inversao das migrations 0017 e 0018, e ela paga duas contas: mudar o
-- padrao de fabrica alcanca todo mundo que ainda nao personalizou, e trocar um
-- slot nao congela os outros quatro.
--
-- `kind` e string opaca. O vocabulario de hoje e `app`, `acao` e `pagina`, e ele
-- mora em `lequePetalas.ts`; o CHECK garante FORMA, nao vocabulario. Um enum aqui faria
-- de cada tipo novo de petala uma migration.
--
-- `slot` aceita 0..11 embora o desenho use CINCO. O banco guarda "qual das
-- posicoes", que e forma; QUANTAS posicoes a interface oferece e vocabulario, e
-- a 0017 ja registrou que vocabulario muda mais rapido que migration. Ir a seis
-- petalas um dia nao custa migration nenhuma.
--
-- `workspace_id` nasce nullable com NULL significando "Todos", copiado da 0018 —
-- inclusive o indice sobre COALESCE, que existe porque no SQLite coluna de
-- PRIMARY KEY aceita NULL e NULL nunca colide com NULL. Sem ele, "Todos"
-- aceitaria doze linhas no mesmo slot e o leque viraria lixo silencioso.
-- Com isso, "um leque por Workspace" depois e comportamento novo e nao
-- estrutura nova.

BEGIN IMMEDIATE;

CREATE TABLE radial_pins (
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL CHECK (slot >= 0 AND slot <= 11),
    kind TEXT NOT NULL CHECK (kind GLOB '[a-z][a-z0-9_]*'),
    target TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX radial_pins_escopo
    ON radial_pins (COALESCE(workspace_id, ''), slot);

PRAGMA user_version = 21;

COMMIT;

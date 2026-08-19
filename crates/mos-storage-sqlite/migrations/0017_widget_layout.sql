-- O arranjo dos widgets da Home, por Workspace.
--
-- A 0016 guardava so a ordem dentro da faixa, e o nome `workspace_widget_order`
-- dizia exatamente isso. Agora a mesma linha responde por tres perguntas — em
-- que faixa o widget esta, em que posicao dentro dela, e quantas colunas ele
-- ocupa. Uma tabela chamada `order` carregando faixa e largura mentiria para
-- quem abrisse o banco, entao ela passa a se chamar `workspace_widget_layout`.
--
-- `section` e `span` nascem NULL, e o NULL e a parte que importa: ele herda a
-- inversao da 0008 e da 0016 — **ausencia de valor significa o que o desenho
-- escolheu.** Duas consequencias, as duas desejadas:
--
--   1. quem nunca redimensionou continua recebendo o span do catalogo, e mudar
--      o desenho de um widget alcanca todo mundo que nao mexeu nele;
--   2. reordenar NAO congela largura nem faixa. Se toda escrita gravasse o
--      valor efetivo, o primeiro arrasto de qualquer widget petrificaria o
--      desenho de hoje — o front so manda `section` e `span` quando a pessoa
--      mexeu neles de verdade.
--
-- `section` e string opaca pelo mesmo motivo de `widget_id`: as faixas vivem no
-- front, e enum aqui faria de cada faixa nova uma migration. O CHECK garante
-- forma, nao vocabulario.
--
-- `span` aceita 1..12 e nao a lista que o desenho oferece hoje (3,4,5,6,8,9,12).
-- O banco guarda "quantas das doze colunas", que e forma; QUAL subconjunto a
-- interface oferece e vocabulario, e vocabulario muda mais rapido que migration.
-- Um span fora da lista do desenho nao corrompe nada — a grade tem doze colunas
-- e desenha qualquer numero delas.

BEGIN IMMEDIATE;

ALTER TABLE workspace_widget_order RENAME TO workspace_widget_layout;

ALTER TABLE workspace_widget_layout
    ADD COLUMN section TEXT CHECK (section IS NULL OR section GLOB '[a-z][a-z0-9_]*');

ALTER TABLE workspace_widget_layout
    ADD COLUMN span INTEGER CHECK (span IS NULL OR (span >= 1 AND span <= 12));

PRAGMA user_version = 17;

COMMIT;

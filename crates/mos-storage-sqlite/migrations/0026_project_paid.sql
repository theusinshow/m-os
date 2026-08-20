-- Migration 0026: quando o Project foi pago.
--
-- DATA, e nao booleano. "Pago" responde "sim"; `paid_at` responde "sim, em
-- 14/07" — e a segunda pergunta e a que aparece quando o cliente liga dizendo
-- que ja pagou. Um booleano obrigaria uma segunda coluna no dia em que a data
-- fizesse falta, e ninguem guarda a data depois do fato.
--
-- NULL significa NAO PAGO, e e o estado normal. Aqui o NULL nao carrega a
-- ambiguidade que a 0014 recusou para `budget_minutes`: la o zero era um valor
-- legitimo confundivel com ausencia; aqui ausencia de data e exatamente o que
-- se quer dizer.
--
-- Eixo PROPRIO, e nao um quinto valor em `tracking_status`. O estado descreve o
-- TRABALHO — ativo, pausado, concluido, arquivado. Pagamento descreve o
-- DINHEIRO. Um Project pode estar concluido e nao pago, que e justamente o
-- estado que interessa cobrar, e colapsar os dois eixos tornaria esse estado
-- inexprimivel.
--
-- Por PROJECT, e nao por sessao ou por periodo faturado. Foi decisao do
-- proprietario: o caso real dele e projeto terminado e quitado. A limitacao
-- conhecida esta registrada aqui — faturar o mesmo Project de novo depois de
-- marca-lo deixa a marca mentindo ate alguem desmarcar a mao. No dia em que
-- isso incomodar, o caminho e um registro de faturas, e nao mais uma coluna.

BEGIN IMMEDIATE;

ALTER TABLE project_tracking ADD COLUMN paid_at TEXT;

PRAGMA user_version = 26;

COMMIT;

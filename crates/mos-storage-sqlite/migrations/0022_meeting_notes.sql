-- As anotacoes de quem gravou.
--
-- `NOT NULL DEFAULT ''`, e nao nullable: aqui a ausencia NAO significa "o que o
-- desenho escolheu", como na 0021 — significa que ninguem escreveu nada. Uma
-- nota vazia e um fato comum e completo, e um NULL so acrescentaria um segundo
-- jeito de dizer a mesma coisa.
--
-- Reuniao gravada antes desta coluna le string vazia, e nao erro.
--
-- Texto puro, sem formatacao: o M/OS nao tem editor rico em lugar nenhum, e
-- introduzir um aqui seria a maior peca da feature pela menor razao.
--
-- Elas sobem ao Hermes como CONTEXTO e nao geram item. O prompt exige "pelo
-- menos um segment" por item, e uma nota digitada nao tem segmento — ela nao foi
-- dita, foi escrita. Ver §6.1 do design.

BEGIN IMMEDIATE;

ALTER TABLE meetings ADD COLUMN notes TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 22;

COMMIT;

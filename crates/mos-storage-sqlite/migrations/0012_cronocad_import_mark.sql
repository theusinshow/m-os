-- Migration 0012: marca de que o CronoCAD ja foi importado.
--
-- A 0011 nao tinha essa marca, e a importacao se protegia de rodar duas vezes
-- recusando quando `time_entries` tivesse qualquer linha. A trava funcionava
-- para o caso que ela imaginava e falhava para o caso real: quem experimentou o
-- cronometro antes de importar criou uma sessao, e com ela a importacao passou a
-- recusar PARA SEMPRE. Aconteceu na primeira vez que alguem usou.
--
-- O erro era conceitual: "ja importei" e "tem dado" nao sao a mesma pergunta. A
-- marca responde a primeira, e a segunda deixa de importar.

BEGIN IMMEDIATE;

ALTER TABLE tracking_settings ADD COLUMN cronocad_imported_at TEXT;

PRAGMA user_version = 12;

COMMIT;

-- HoraCAD — Migration 0004: dados do emissor para faturas/recibos
--
-- Guarda os dados do profissional/empresa que emite a cobranca, usados no
-- cabecalho da fatura em PDF (secao 13 / melhoria "fatura por cliente"). Sao
-- opcionais e nao afetam nenhum calculo.

ALTER TABLE settings ADD COLUMN issuer_name TEXT NOT NULL DEFAULT '';
ALTER TABLE settings ADD COLUMN issuer_document TEXT NOT NULL DEFAULT '';
ALTER TABLE settings ADD COLUMN issuer_contact TEXT NOT NULL DEFAULT '';

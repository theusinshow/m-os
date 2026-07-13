-- HoraCAD — Migration 0002: tempo inativo acumulado no cronometro ativo
--
-- Guarda o tempo inativo descontado durante a sessao em andamento. Ao encerrar,
-- este valor e transferido para `time_entries.idle_seconds`, permitindo separar
-- duracao bruta, tempo inativo e duracao liquida (secoes 11 e 12). O tempo real
-- (bruto) continua preservado; o desconto e uma decisao explicita do usuario.

ALTER TABLE active_timer
    ADD COLUMN idle_seconds INTEGER NOT NULL DEFAULT 0;

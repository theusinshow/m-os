-- HoraCAD — Migration 0003: meta de horas por projeto
--
-- Permite orcar horas por projeto e comparar com o trabalhado (secao 24 / item
-- de melhoria "metas por projeto"). 0 = sem meta definida. O valor e opcional e
-- nao afeta calculos de cobranca — serve apenas para acompanhamento e alertas.

ALTER TABLE projects
    ADD COLUMN budget_minutes INTEGER NOT NULL DEFAULT 0;

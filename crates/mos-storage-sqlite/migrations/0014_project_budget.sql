-- Migration 0014: meta de horas por Project, e o emissor da fatura.
--
-- A tela de Projetos do CronoCAD desenha uma barra de progresso contra uma meta
-- ("16h de 40h"), e a 0011 nao trouxe a coluna. Nao foi perda de dado: os tres
-- projetos importados tinham `budget_minutes = 0`, ou seja, nenhuma meta estava
-- configurada. Foi perda de FUNCAO — e a decisao foi trazer todas as telas e
-- funcoes, nao so as que tinham dado dentro.
--
-- Em MINUTOS e nao em horas porque o CronoCAD guardava assim, e converter na
-- importacao criaria um segundo formato para a mesma grandeza. Zero significa
-- "sem meta", e nao "meta de zero": um projeto sem meta e o caso comum, e um
-- NULL aqui obrigaria toda leitura a decidir o que fazer com a ausencia.

BEGIN IMMEDIATE;

ALTER TABLE project_tracking ADD COLUMN budget_minutes INTEGER NOT NULL DEFAULT 0;

-- Quem esta cobrando. Sai no cabecalho da fatura em PDF, e em nenhum outro
-- lugar — por isso mora em `tracking_settings` e nao nas Settings do M/OS.
--
-- Os tres estavam vazios no banco importado, entao nada se perdeu por terem
-- ficado de fora ate aqui. Entram porque a fatura sem emissor e uma fatura que
-- nao identifica quem deve receber.
ALTER TABLE tracking_settings ADD COLUMN issuer_name TEXT NOT NULL DEFAULT '';
ALTER TABLE tracking_settings ADD COLUMN issuer_document TEXT NOT NULL DEFAULT '';
ALTER TABLE tracking_settings ADD COLUMN issuer_contact TEXT NOT NULL DEFAULT '';

-- FICARAM DE FORA do que o CronoCAD guardava em `settings`, com motivo:
--
--   start_with_windows, minimize_to_tray, close_to_tray
--     comportamento de janela do aplicativo inteiro, e nao do rastreio de
--     tempo. Se o M/OS quiser isso um dia, e decisao dele e vale para as nove
--     paginas — nao uma preferencia que o Tempo carrega no colo.
--
--   currency, locale
--     o M/OS assume BRL e pt-BR em toda a interface. Uma segunda fonte de
--     verdade sobre a moeda so faria sentido junto com suporte real a outras,
--     e ai a decisao e do aplicativo e nao desta tabela.

PRAGMA user_version = 14;

COMMIT;

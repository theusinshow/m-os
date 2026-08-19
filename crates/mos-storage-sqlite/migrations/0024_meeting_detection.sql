-- O toggle da deteccao de reuniao (ADR-047).
--
-- Nasceu 0023 e virou 0024: o merge da Universal Drop Zone chegou primeiro e
-- levou o 23. Renumerar aqui e seguro porque esta nunca rodou na maquina de
-- ninguem — se tivesse rodado, um banco em `user_version = 23` nao poderia ser
-- distinguido entre "aplicou o drop" e "aplicou a deteccao", e a saida seria uma
-- migration de conserto. E a mesma armadilha que o comentario da MIGRATION_020
-- registra ter acontecido antes.
--
-- Mora em `tracking_settings` porque e la que ja vivem `process_monitoring_enabled`
-- e as outras chaves de observacao, apesar de o nome da tabela falar de tracking.
-- Uma tabela nova so para um booleano seria pior que o nome imperfeito.
--
-- `DEFAULT 1`: LIGADA de fabrica, por decisao do proprietario. A ADR-047 admite o
-- custo com todas as letras — a fronteira da ADR-037 passa a ser atravessada COM
-- AVISO e nao com pedido. O toggle e a mitigacao, e ele mora em Settings >
-- REUNIOES, e nao enterrado em Avancado.

BEGIN IMMEDIATE;

ALTER TABLE tracking_settings ADD COLUMN meeting_detection_enabled INTEGER NOT NULL DEFAULT 1;

PRAGMA user_version = 24;

COMMIT;

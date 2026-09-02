-- A tarifa padrao do CronoCAD.
--
-- # Por que uma coluna, e nao uma constante no codigo
--
-- Ate aqui, hora lancada em Project sem linha de `project_tracking` nascia com
-- `hourly_rate_snapshot_cents = 0` — e valor zero nao se distingue de trabalho
-- de graca. Quem tinha a tarifa cadastrada nunca via o defeito; quem sincronizou
-- um Project criado no outro PC via a base inteira valendo nada.
--
-- O padrao mora AQUI, e nao numa constante, por duas razoes independentes: ele
-- precisa ser editavel sem recompilar, e `tracking_settings` ja atravessa a
-- sincronizacao — entao o mesmo padrao nasce nos dois PCs sem ninguem digitar
-- de novo. Uma constante no binario faria a metade da promessa.
--
-- 3000 centavos = R$ 30,00/h, que e a tarifa que os seis Projects deste banco
-- ja usavam. O DEFAULT preenche a linha existente na propria migration: nao ha
-- estado intermediario em que o padrao seja zero.
--
-- O que ele NAO faz: reescrever hora ja gravada. O snapshot de cada lancamento
-- e o registro do que valia quando o trabalho aconteceu, e mexer nele e um ato
-- deliberado — o recalculo, que so toca o que esta zerado.
ALTER TABLE tracking_settings
    ADD COLUMN default_hourly_rate_cents INTEGER NOT NULL DEFAULT 3000;

PRAGMA user_version = 37;

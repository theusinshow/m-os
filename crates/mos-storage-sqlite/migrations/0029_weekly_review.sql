-- Migration 0029: o fecho da semana.
--
-- UMA tabela nova, e nenhuma alteracao em tabela existente — a mesma regra da
-- 0027 e da 0028. Se o desenho da revisao semanal mudar, ela se apaga sem tocar
-- em uma linha do que existe.
--
-- A tabela e minuscula porque A NARRATIVA INTEIRA E DERIVADA das Daily
-- Sessions. Guardar o resumo duplicaria dado para exibir noutra superficie, e
-- ele envelheceria: reabrir um objetivo de terca mudaria a semana, e o texto
-- gravado continuaria dizendo o contrario.

BEGIN IMMEDIATE;

CREATE TABLE weekly_reviews (
    id          TEXT PRIMARY KEY NOT NULL,

    -- A data da SEGUNDA-FEIRA da semana, `AAAA-MM-DD`.
    --
    -- Nao e numero ISO, e a escolha evita duas armadilhas: semanas 53, e o 1º
    -- de janeiro que pertence a semana 52 do ano anterior. `2026-W01` obrigaria
    -- a escolher uma convencao de virada de ano e a acerta-la em todo lugar que
    -- compara; `2026-08-17` nao obriga a nada.
    week_start  TEXT NOT NULL,

    -- Vazio e legitimo: fechar a semana e o gesto, escrever e opcional.
    summary     TEXT NOT NULL DEFAULT '',

    -- Quando a semana foi fechada. NAO se move ao editar o texto: quando ela
    -- foi fechada e um fato, e o texto e outro.
    closed_at   TEXT NOT NULL,

    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,

    -- Formato exato, e nao "parece uma data": `2026-8-17` e `2026-08-17` sao a
    -- mesma semana e duas chaves diferentes, e o indice unico abaixo nao veria
    -- a duplicata. Mesma razao do CHECK da 0028.
    CONSTRAINT weekly_reviews_week_shape CHECK (
        week_start GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    )
);

-- O CHECK NAO verifica que e segunda-feira. SQLite conseguiria, com
-- `strftime('%w', week_start) = '1'`, e isso seria a regra da semana escrita num
-- segundo lugar. Quem garante e `Week`, que e o unico construtor.

CREATE UNIQUE INDEX weekly_reviews_one_per_week ON weekly_reviews (week_start);

PRAGMA user_version = 29;

COMMIT;

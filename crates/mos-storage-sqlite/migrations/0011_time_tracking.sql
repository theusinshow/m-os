-- Migration 0011: rastreio de tempo por Project.
--
-- Etapa B da absorcao do CronoCAD (ADR-032). O esquema entra FUNDIDO: as horas
-- referenciam `projects` do M/OS, e nao uma tabela de projeto paralela.
--
-- A fusao foi possivel barata porque o `projects` do M/OS tinha um registro
-- quando esta migration foi escrita, enquanto o CronoCAD tinha tres projetos com
-- 26 horas reais. Mapear tres jobs contra um Project vazio e trivial; mapear
-- contra vinte, meses depois, seria trabalho manual sujeito a erro. O
-- `PRODUCT.md` secao 7 ja listava "projetos profissionais" entre os exemplos de
-- Project, entao a fusao nao forca o conceito.
--
-- Convencoes herdadas do CronoCAD e mantidas: duracoes em segundos, dinheiro em
-- centavos, timestamps ISO 8601 UTC, booleanos como 0/1.
--
-- REGRA QUE ATRAVESSA O ESQUEMA INTEIRO: o banco guarda sempre o tempo REAL.
-- Arredondamento e desconto de inatividade se aplicam na visualizacao e na
-- cobranca, nunca sobrescrevendo o valor gravado.

BEGIN IMMEDIATE;

-- Dados de cobranca de um Project.
--
-- Satelite, e nao colunas em `projects`, porque valor/hora e codigo de obra sao
-- assunto de quem rastreia tempo. A maioria dos Projects do M/OS nunca tera
-- isso, e uma coluna nula em toda linha ensina que o campo e opcional quando na
-- verdade ele pertence a outro dominio.
CREATE TABLE project_tracking (
    project_id TEXT PRIMARY KEY NOT NULL
        REFERENCES projects (id) ON DELETE CASCADE,
    hourly_rate_cents INTEGER NOT NULL DEFAULT 0,
    -- Codigo da obra, como "043". Vem do CronoCAD e nao tem equivalente no M/OS.
    code TEXT,
    color TEXT,
    -- O CronoCAD distingue quatro estados; o `lifecycle_state` do M/OS tem tres
    -- e nenhum significa "concluido". Guardar o original aqui evita duas perdas:
    -- inventar um estado no M/OS, e esquecer que um projeto terminou em vez de
    -- ter sido arquivado por desuso.
    tracking_status TEXT NOT NULL DEFAULT 'active'
        CHECK (tracking_status IN ('active', 'paused', 'completed', 'archived')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- Uma sessao de trabalho registrada.
CREATE TABLE time_entries (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_seconds INTEGER NOT NULL DEFAULT 0,
    idle_seconds INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    activity_type TEXT NOT NULL DEFAULT 'other'
        CHECK (activity_type IN
            ('drawing', 'detailing', 'revision', 'meeting', 'study', 'other')),
    billable INTEGER NOT NULL DEFAULT 1 CHECK (billable IN (0, 1)),
    -- Preserva o valor/hora do momento da sessao: reajustar o Project nao
    -- reescreve o que ja foi trabalhado e possivelmente ja foi cobrado.
    hourly_rate_snapshot_cents INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'timer'
        CHECK (source IN ('timer', 'manual', 'reconstructed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Soft delete: hora de trabalho e registro de cobranca, e sai da vista sem
    -- sair do banco.
    deleted_at TEXT
) STRICT;

CREATE INDEX time_entries_project ON time_entries (project_id);
CREATE INDEX time_entries_started ON time_entries (started_at);
CREATE INDEX time_entries_live ON time_entries (deleted_at);

-- No maximo UM cronometro ativo, garantido pelo banco e nao pela aplicacao.
--
-- `singleton` e unica e restrita a 1: duas linhas sao impossiveis. A regra vive
-- aqui porque um segundo cronometro rodando significa hora contada duas vezes,
-- e isso o codigo de aplicacao nao pode ser a unica coisa a impedir.
CREATE TABLE active_timer (
    id TEXT PRIMARY KEY NOT NULL,
    singleton INTEGER NOT NULL DEFAULT 1 UNIQUE CHECK (singleton = 1),
    project_id TEXT NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    last_resumed_at TEXT NOT NULL,
    accumulated_seconds INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running'
        CHECK (status IN ('running', 'paused')),
    description TEXT,
    activity_type TEXT NOT NULL DEFAULT 'other'
        CHECK (activity_type IN
            ('drawing', 'detailing', 'revision', 'meeting', 'study', 'other')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- Configuracao de arredondamento e inatividade. Linha unica.
--
-- Separada das Settings do M/OS de proposito: e configuracao de um dominio, nao
-- preferencia do aplicativo, e some junto se o rastreio de tempo um dia sair.
CREATE TABLE tracking_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    rounding_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (rounding_enabled IN (0, 1)),
    rounding_interval_minutes INTEGER NOT NULL DEFAULT 15,
    rounding_mode TEXT NOT NULL DEFAULT 'nearest'
        CHECK (rounding_mode IN ('nearest', 'up', 'down')),
    idle_threshold_minutes INTEGER NOT NULL DEFAULT 10
) STRICT;

INSERT INTO tracking_settings (id) VALUES (1);

-- FICARAM DE FORA, e cada ausencia e uma decisao:
--
--   clients          o dataset real tem zero, e o M/OS nao tem conceito de
--                    cliente. Entra junto com faturamento, se entrar.
--   monitored_apps   deteccao de processo e integracao com o sistema
--   activity_events  operacional, nao dominio. Etapa C, com a superficie.
--   project_todos    o M/OS ja tem Task com project_id. Duas pendencias viram
--                    duas Tasks na importacao, em vez de um segundo modelo de
--                    lista de afazeres competindo com o que ja existe.

PRAGMA user_version = 11;

COMMIT;

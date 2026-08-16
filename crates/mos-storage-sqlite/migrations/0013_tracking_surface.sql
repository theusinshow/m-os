-- Migration 0013: o que falta para a superficie do CronoCAD caber no M/OS.
--
-- A 0011 trouxe o nucleo — sessao, cronometro, cobranca por Project — e deixou
-- tres tabelas de fora com motivo declarado: `clients` porque o dataset real
-- tinha zero e o M/OS nao tem o conceito, `monitored_apps` e `activity_events`
-- porque eram integracao com o sistema e nao dominio.
--
-- O motivo caducou. A decisao de trazer TODAS as telas do CronoCAD para dentro
-- do M/OS torna as tres necessarias: Clientes e uma tela, a fatura por cliente
-- depende dela, e a Linha do Tempo Detectada e feita inteira de evento de
-- atividade cruzado com programa monitorado.
--
-- `project_todos` continua fora, e agora por conviccao e nao por adiamento: o
-- M/OS ja tem Task com `project_id`, e as pendencias do CronoCAD viraram Tasks
-- na importacao. Os comandos `list_todos`/`create_todo` que as telas chamam vao
-- ser atendidos por Tasks filtradas — uma segunda lista de afazeres competindo
-- com a que existe seria o comeco de duas verdades sobre a mesma pergunta.

BEGIN IMMEDIATE;

-- Cliente de um Project. Existe para a fatura: e dele que sai o cabecalho do
-- PDF e o agrupamento de horas por quem paga.
CREATE TABLE clients (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    company_name TEXT,
    email TEXT,
    phone TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
) STRICT;

-- O vinculo mora em `project_tracking`, e nao em `projects`: cliente e assunto
-- de cobranca, e a maioria dos Projects do M/OS nunca tera um.
ALTER TABLE project_tracking ADD COLUMN client_id TEXT REFERENCES clients (id) ON DELETE SET NULL;

-- Programas cuja abertura sugere trabalho — AutoCAD, Revit, SketchUp.
--
-- `process_name` e unico porque o monitoramento casa por nome de processo: dois
-- cadastros do mesmo executavel gerariam dois lembretes para a mesma abertura.
CREATE TABLE monitored_apps (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    process_name TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    remind_on_open INTEGER NOT NULL DEFAULT 1 CHECK (remind_on_open IN (0, 1)),
    remind_on_close INTEGER NOT NULL DEFAULT 1 CHECK (remind_on_close IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- O que o sistema observou. E a materia-prima da Linha do Tempo Detectada: o
-- CronoCAD cruza abertura e fechamento de programa com os periodos sem sessao
-- registrada, e oferece transformar o vao em hora trabalhada.
--
-- `processed` marca o evento que ja virou sessao ou ja foi descartado, para a
-- linha do tempo nao reoferecer o mesmo periodo todo dia.
CREATE TABLE activity_events (
    id TEXT PRIMARY KEY NOT NULL,
    event_type TEXT NOT NULL
        CHECK (event_type IN
            ('app_opened', 'app_closed', 'idle_started', 'idle_ended',
             'timer_started', 'timer_paused', 'timer_resumed', 'timer_stopped')),
    process_name TEXT,
    detected_at TEXT NOT NULL,
    metadata_json TEXT,
    processed INTEGER NOT NULL DEFAULT 0 CHECK (processed IN (0, 1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX activity_events_detected ON activity_events (detected_at);
CREATE INDEX activity_events_pending ON activity_events (processed, detected_at);

-- As preferencias que o CronoCAD guardava e o M/OS ainda nao tinha lugar para
-- por. Ficam em `tracking_settings` porque sao do dominio de tempo, e somem
-- junto com ele se um dia sair.
ALTER TABLE tracking_settings ADD COLUMN idle_detection_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tracking_settings ADD COLUMN process_monitoring_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tracking_settings ADD COLUMN process_check_interval_seconds INTEGER NOT NULL DEFAULT 5;
ALTER TABLE tracking_settings ADD COLUMN remind_on_monitored_open INTEGER NOT NULL DEFAULT 1;
ALTER TABLE tracking_settings ADD COLUMN remind_on_monitored_close INTEGER NOT NULL DEFAULT 1;

-- Sugestoes do CronoCAD. Nem todos estarao instalados, e isso e esperado: a
-- lista existe para o usuario reconhecer o proprio ferramental, nao para
-- afirmar que ele tem os cinco.
INSERT INTO monitored_apps (id, display_name, process_name, created_at, updated_at)
VALUES
    ('app-acad',      'AutoCAD',   'acad.exe',      '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-revit',     'Revit',     'revit.exe',     '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-sketchup',  'SketchUp',  'sketchup.exe',  '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-eberick',   'Eberick',   'eberick.exe',   '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z'),
    ('app-qibuilder', 'QiBuilder', 'qibuilder.exe', '1970-01-01T00:00:00Z', '1970-01-01T00:00:00Z');

PRAGMA user_version = 13;

COMMIT;

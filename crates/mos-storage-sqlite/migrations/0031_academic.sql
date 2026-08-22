-- M/Academic — a camada academica do M/OS.
--
-- Cinco tabelas novas e ZERO alteracao nas existentes. A relacao com o que ja
-- existe viaja em colunas de referencia (`task_id`) e numa juncao
-- (`academic_subject_resources`), como o `resource_projects` da 0023 — nunca
-- acrescentando coluna academica em `tasks` ou `resources`. Faculdade e um
-- CONTEXTO sobre os primitivos do M/OS, e nao um campo deles.
--
-- # Por que nao existe tabela de notas
--
-- A nota mora na avaliacao que a produziu: `score` e `max_score` em
-- `academic_exams` e em `academic_assignments`. Uma tabela `grades` separada
-- seria uma TERCEIRA fonte para o mesmo fato — a prova diria 7,5 e a nota diria
-- 8,0, e nada no banco diria qual das duas esta certa. A media e derivada das
-- duas listas, em `mos_core::academic`, com teste.
--
-- # Por que o semestre nao guarda "ativo"
--
-- `status` de semestre e DERIVADO das datas: quem comeca depois de hoje esta
-- por vir, quem terminou esta concluido, o resto e o corrente. Guardar o estado
-- criaria a linha que diz "ativo" num semestre que acabou em dezembro, e o
-- sistema passaria a precisar de alguem para corrigi-la. O que se guarda e o
-- `lifecycle_state`, que e escolha da pessoa (arquivar), e nao passagem do
-- tempo.
--
-- # Datas
--
-- `starts_on` e `ends_on` sao DIA civil (`AAAA-MM-DD`), porque semestre e um
-- intervalo de calendario. `due_at` e `at` sao INSTANTE (RFC-3339, UTC), porque
-- prazo e prova tem hora e o M/OS ja guarda instante em UTC em todo lugar. Sao
-- os dois vocabularios que o `calendar.rs` ja distingue, e nao um descuido.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- Semestre — o periodo letivo
-- ---------------------------------------------------------------------------

CREATE TABLE academic_semesters (
    id               TEXT PRIMARY KEY NOT NULL,
    name             TEXT NOT NULL,
    institution      TEXT NOT NULL DEFAULT '',
    starts_on        TEXT NOT NULL,
    ends_on          TEXT NOT NULL,
    lifecycle_state  TEXT NOT NULL DEFAULT 'active',
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,

    CONSTRAINT academic_semesters_name_present CHECK (length(trim(name)) > 0),
    CONSTRAINT academic_semesters_lifecycle_known
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    -- Um semestre que termina antes de comecar nao e um estado transitorio de
    -- edicao: e um erro que envenena toda conta de progresso e de "qual e o
    -- semestre corrente".
    CONSTRAINT academic_semesters_interval_ordered CHECK (ends_on >= starts_on)
) STRICT;

CREATE INDEX academic_semesters_por_periodo
    ON academic_semesters(lifecycle_state, starts_on DESC);

-- ---------------------------------------------------------------------------
-- Disciplina
-- ---------------------------------------------------------------------------

CREATE TABLE academic_subjects (
    id               TEXT PRIMARY KEY NOT NULL,
    semester_id      TEXT NOT NULL REFERENCES academic_semesters(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    code             TEXT NOT NULL DEFAULT '',
    teacher          TEXT NOT NULL DEFAULT '',
    -- Um dos accents do design system, por NOME e nunca por hexadecimal: cor
    -- crua gravada aqui nao acompanharia a troca de tema, e o M/OS tem dois.
    accent           TEXT NOT NULL DEFAULT '',
    notes            TEXT NOT NULL DEFAULT '',
    lifecycle_state  TEXT NOT NULL DEFAULT 'active',
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,

    CONSTRAINT academic_subjects_name_present CHECK (length(trim(name)) > 0),
    CONSTRAINT academic_subjects_lifecycle_known
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed'))
) STRICT;

CREATE INDEX academic_subjects_por_semestre
    ON academic_subjects(semester_id, lifecycle_state, name);

-- ---------------------------------------------------------------------------
-- Atividade — o que se entrega
-- ---------------------------------------------------------------------------

CREATE TABLE academic_assignments (
    id               TEXT PRIMARY KEY NOT NULL,
    subject_id       TEXT NOT NULL REFERENCES academic_subjects(id) ON DELETE CASCADE,
    title            TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    -- Instante, e nao dia: "entregar ate sexta 23h59" e o formato real de um
    -- prazo academico. Vazio significa sem prazo definido, que e diferente de
    -- prazo hoje.
    due_at           TEXT,
    status           TEXT NOT NULL DEFAULT 'pending',
    priority         TEXT NOT NULL DEFAULT 'normal',
    -- Peso na media da disciplina. Zero significa "nao entra na media", que e o
    -- caso comum de lista de exercicios.
    weight           REAL NOT NULL DEFAULT 0,
    max_score        REAL,
    score            REAL,
    -- A Task de verdade do M/OS que executa esta atividade.
    --
    -- `ON DELETE SET NULL`, e nao CASCADE: apagar a Task nao pode apagar a
    -- atividade da faculdade. Ela perde o braco executor, nao a existencia — e
    -- o mesmo raciocinio de `meetings.project_id` na 0020.
    task_id          TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    lifecycle_state  TEXT NOT NULL DEFAULT 'active',
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,

    CONSTRAINT academic_assignments_title_present CHECK (length(trim(title)) > 0),
    CONSTRAINT academic_assignments_status_known CHECK (status IN (
        'pending', 'in_progress', 'submitted', 'graded', 'cancelled'
    )),
    CONSTRAINT academic_assignments_priority_known
        CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    CONSTRAINT academic_assignments_lifecycle_known
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    CONSTRAINT academic_assignments_weight_not_negative CHECK (weight >= 0),
    -- Nota sem teto nao se converte em media: 8 de quanto? O par existe junto
    -- ou nao existe.
    CONSTRAINT academic_assignments_score_has_ceiling
        CHECK (score IS NULL OR max_score IS NOT NULL),
    CONSTRAINT academic_assignments_scores_not_negative
        CHECK ((score IS NULL OR score >= 0) AND (max_score IS NULL OR max_score > 0))
) STRICT;

CREATE INDEX academic_assignments_por_prazo
    ON academic_assignments(lifecycle_state, status, due_at);
CREATE INDEX academic_assignments_por_disciplina
    ON academic_assignments(subject_id, lifecycle_state);
-- Uma Task executa UMA atividade. Sem isto, duas atividades poderiam apontar
-- para a mesma Task e concluir uma marcaria a outra como feita.
CREATE UNIQUE INDEX academic_assignments_task_unica
    ON academic_assignments(task_id) WHERE task_id IS NOT NULL;

-- ---------------------------------------------------------------------------
-- Avaliacao — o que se faz na data marcada
-- ---------------------------------------------------------------------------

CREATE TABLE academic_exams (
    id               TEXT PRIMARY KEY NOT NULL,
    subject_id       TEXT NOT NULL REFERENCES academic_subjects(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    -- NOT NULL: prova sem data e um plano, e o M/OS ja tem Task para plano. O
    -- que faz uma avaliacao ser avaliacao aqui e ela ocupar um instante.
    at               TEXT NOT NULL,
    location         TEXT NOT NULL DEFAULT '',
    topics           TEXT NOT NULL DEFAULT '',
    weight           REAL NOT NULL DEFAULT 0,
    max_score        REAL,
    score            REAL,
    status           TEXT NOT NULL DEFAULT 'scheduled',
    lifecycle_state  TEXT NOT NULL DEFAULT 'active',
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,

    CONSTRAINT academic_exams_name_present CHECK (length(trim(name)) > 0),
    CONSTRAINT academic_exams_status_known
        CHECK (status IN ('scheduled', 'done', 'graded', 'cancelled')),
    CONSTRAINT academic_exams_lifecycle_known
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    CONSTRAINT academic_exams_weight_not_negative CHECK (weight >= 0),
    CONSTRAINT academic_exams_score_has_ceiling
        CHECK (score IS NULL OR max_score IS NOT NULL),
    CONSTRAINT academic_exams_scores_not_negative
        CHECK ((score IS NULL OR score >= 0) AND (max_score IS NULL OR max_score > 0))
) STRICT;

CREATE INDEX academic_exams_por_data
    ON academic_exams(lifecycle_state, at);
CREATE INDEX academic_exams_por_disciplina
    ON academic_exams(subject_id, lifecycle_state);

-- ---------------------------------------------------------------------------
-- Materiais — a juncao com o acervo que ja existe
-- ---------------------------------------------------------------------------
--
-- Copia estrutural de `resource_projects` (0023), e pelo mesmo motivo: N-para-N
-- porque o mesmo PDF serve a duas disciplinas, e juncao em vez de coluna porque
-- `resources` nao pode ganhar um campo academico.
--
-- E o ponto de entrada do Google Drive e do KNOW/OS mais tarde: quem resolver
-- storage externo mexe em `resources`, e esta tabela continua valendo sem uma
-- linha de mudanca.

CREATE TABLE academic_subject_resources (
    subject_id   TEXT NOT NULL REFERENCES academic_subjects(id) ON DELETE CASCADE,
    resource_id  TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    created_at   TEXT NOT NULL,

    PRIMARY KEY (subject_id, resource_id)
) STRICT;

CREATE INDEX academic_subject_resources_por_resource
    ON academic_subject_resources(resource_id);

-- ---------------------------------------------------------------------------
-- Sessao de estudo
-- ---------------------------------------------------------------------------
--
-- Tabela propria, e nao `time_entries` do CronoCAD. Aquela mede hora COBRAVEL —
-- carrega cliente, arredondamento e valor por hora, e settle() converte tudo em
-- dinheiro. Estudar nao se fatura, e enfiar estudo la dentro faria a receita do
-- Painel somar horas que ninguem vai cobrar.

CREATE TABLE academic_study_sessions (
    id           TEXT PRIMARY KEY NOT NULL,
    subject_id   TEXT NOT NULL REFERENCES academic_subjects(id) ON DELETE CASCADE,
    topic        TEXT NOT NULL DEFAULT '',
    notes        TEXT NOT NULL DEFAULT '',
    started_at   TEXT NOT NULL,
    -- Vazio enquanto a sessao esta em curso. E o unico jeito de saber, ao
    -- reabrir o app, que havia um cronometro rodando quando ele fechou.
    ended_at     TEXT,
    -- Medido no fecho e GRAVADO, em vez de derivado de ended_at - started_at na
    -- leitura: o mesmo motivo do `duration_ms` de `meetings`. Uma sessao pausada
    -- ou corrigida a mao tem duracao que o relogio de parede nao reproduz.
    seconds      INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,

    CONSTRAINT academic_study_seconds_not_negative CHECK (seconds >= 0)
) STRICT;

CREATE INDEX academic_study_por_inicio
    ON academic_study_sessions(started_at DESC);
CREATE INDEX academic_study_por_disciplina
    ON academic_study_sessions(subject_id, started_at DESC);
-- Uma sessao aberta por vez, no sistema inteiro. Duas correndo juntas fariam o
-- "quanto estudei hoje" somar o mesmo minuto duas vezes.
CREATE UNIQUE INDEX academic_study_uma_aberta
    ON academic_study_sessions((1)) WHERE ended_at IS NULL;

PRAGMA user_version = 31;

COMMIT;

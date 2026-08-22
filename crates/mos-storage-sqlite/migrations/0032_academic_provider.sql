-- A ponte entre o M/Academic e um AVA externo.
--
-- Duas tabelas, e ZERO coluna nova nas cinco tabelas academicas da 0031. E o
-- mesmo raciocinio do ADR-058: faculdade e um CONTEXTO sobre os primitivos, e
-- provedor externo e um contexto sobre a faculdade. Se `academic_exams`
-- ganhasse `provider` e `external_id`, o dia em que um segundo AVA existir cada
-- uma das cinco tabelas ganharia mais um par de colunas — e uma entidade criada
-- a mao pela pessoa carregaria duas colunas eternamente nulas.
--
-- # Por que `payload_hash`
--
-- O Univirtus nao tem ETag, nem `If-Modified-Since`, nem cursor, e
-- `dataModificacao` existe em algumas entidades e nao em outras. O unico
-- criterio que serve para todas e comparar a impressao digital do que o
-- provedor mandou com a da ultima vez.
--
-- # Por que `unavailable_since` em vez de apagar
--
-- Uma avaliacao some da lista quando a janela dela fecha; um trabalho some
-- quando o semestre vira. Apagar por ausencia jogaria fora o historico da
-- pessoa por causa de uma decisao de EXIBICAO do portal. A linha fica, marcada,
-- e volta a valer quando reaparecer.
--
-- # Por que `local_id` e TEXT sem foreign key
--
-- Ele aponta para uma de cinco tabelas, escolhida por `kind`. Uma FK por tipo
-- exigiria cinco colunas mutuamente exclusivas, e o CHECK que garantisse "so uma
-- preenchida" seria mais fragil que a limpeza explicita. Quem apaga entidade
-- academica limpa a referencia junto, e o `academic_refs_orfas` da 0032 deixa o
-- rastro auditavel — e o mesmo caminho que o `verify_foreign_keys` ja percorre.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------------------
-- A referencia externa
-- ---------------------------------------------------------------------------

CREATE TABLE academic_external_refs (
    provider             TEXT NOT NULL,
    kind                 TEXT NOT NULL,
    external_id          TEXT NOT NULL,
    local_id             TEXT NOT NULL,
    payload_hash         TEXT NOT NULL,
    -- `dataModificacao` do provedor, quando ele publica uma. Informativo: quem
    -- decide se mudou e o hash.
    external_updated_at  TEXT,
    unavailable_since    TEXT,
    first_synced_at      TEXT NOT NULL,
    last_synced_at       TEXT NOT NULL,

    PRIMARY KEY (provider, kind, external_id),

    CONSTRAINT academic_refs_provider_present CHECK (length(trim(provider)) > 0),
    CONSTRAINT academic_refs_external_present CHECK (length(trim(external_id)) > 0),
    CONSTRAINT academic_refs_local_present CHECK (length(trim(local_id)) > 0),
    CONSTRAINT academic_refs_kind_known CHECK (kind IN (
        'semester', 'subject', 'assignment', 'exam', 'material'
    ))
) STRICT;

-- Uma entidade do M/OS pertence a UM item externo. Sem isto, duas avaliacoes do
-- portal poderiam apontar para a mesma `Exam` local, e sincronizar uma
-- sobrescreveria a outra em silencio — que e exatamente a duplicata invertida
-- que a chave por `idAvaliacao` existe para evitar.
CREATE UNIQUE INDEX academic_external_refs_local_unico
    ON academic_external_refs(provider, kind, local_id);

CREATE INDEX academic_external_refs_por_sincronizacao
    ON academic_external_refs(provider, kind, last_synced_at DESC);

-- ---------------------------------------------------------------------------
-- O estado da conexao
-- ---------------------------------------------------------------------------
--
-- Uma linha por provedor. NAO guarda cookie, `X-time`, senha nem qualquer
-- segredo de sessao: esses vivem SO no Credential Manager do sistema, pelo mesmo
-- caminho que `mos-hermes/src/auth.rs` e `finance.rs` ja usam. O que esta aqui e
-- o que a tela precisa mostrar sem destrancar nada.

CREATE TABLE academic_provider_state (
    provider          TEXT PRIMARY KEY NOT NULL,
    -- 'disconnected' | 'connected' | 'expired'. Expirado NAO apaga dado: os
    -- ultimos dados sincronizados continuam servindo o M/Academic offline.
    connection        TEXT NOT NULL DEFAULT 'disconnected',
    course_name       TEXT NOT NULL DEFAULT '',
    course_external_id TEXT NOT NULL DEFAULT '',
    last_sync_at      TEXT,
    last_outcome      TEXT,
    -- O relatorio da ultima rodada, em JSON, para a tela de integracao. Contem
    -- contagens e avisos — nunca payload do provedor, nunca URL assinada.
    last_report       TEXT NOT NULL DEFAULT '',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,

    CONSTRAINT academic_provider_connection_known
        CHECK (connection IN ('disconnected', 'connected', 'expired')),
    CONSTRAINT academic_provider_outcome_known CHECK (last_outcome IS NULL OR last_outcome IN (
        'completed', 'completed_with_warnings', 'requires_authentication', 'failed'
    ))
) STRICT;

-- ---------------------------------------------------------------------------
-- O endereco corrente do material
-- ---------------------------------------------------------------------------
--
-- A URL de download do Univirtus e assinada pelo CloudFront e expira em horas.
-- Ela NAO e identidade (a identidade e `sistemaRepositorio.id`, que vive em
-- `academic_external_refs.external_id`) e NAO deve virar o `url` do `Resource`,
-- porque um Resource com URL morta e pior que um Resource sem URL: ele promete
-- que abre.
--
-- Esta tabela guarda o ultimo endereco visto, com a hora em que foi visto, para
-- que a interface possa dizer "resolver de novo" em vez de abrir um link
-- quebrado. Ela e cache, e some sem prejuizo.

CREATE TABLE academic_material_urls (
    provider     TEXT NOT NULL,
    external_id  TEXT NOT NULL,
    url          TEXT NOT NULL,
    fetched_at   TEXT NOT NULL,

    PRIMARY KEY (provider, external_id)
) STRICT;

PRAGMA user_version = 32;

COMMIT;

-- A conversa do Hermes passa a existir no M/OS (ADR-025).
--
-- Ate aqui nada era guardado: o historico vive no state.db da VPS e a thread
-- existia so na memoria do componente React. O session_id, que a Spec B mandava
-- guardar, vivia num Mutex de processo e nada o escrevia em disco — entao
-- session.resume, implementado e testado, nunca rodava entre aberturas do app.
--
-- Tres tabelas, nao nove. Anexo, artifact, citacao e execucao de ferramenta
-- entram como kind de parte e so viram tabela propria quando precisarem de
-- lifecycle ou consulta propria. E para isso que message_parts existe.

BEGIN IMMEDIATE;

CREATE TABLE conversations (
    id TEXT PRIMARY KEY NOT NULL,
    -- Vazio ate session.title responder. O M/OS nao inventa titulo: um
    -- "Nova conversa" gravado aqui sobreviveria ao titulo real.
    title TEXT NOT NULL DEFAULT '',
    -- O vinculo com a sessao da VPS. NULL enquanto a sessao nao abriu.
    hermes_session_id TEXT,
    lifecycle_state TEXT NOT NULL DEFAULT 'active'
        CHECK (lifecycle_state IN ('active', 'archived', 'trashed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX conversations_lifecycle_order
    ON conversations(lifecycle_state, updated_at DESC);

CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    -- Ordem dentro da conversa. UNIQUE para o append nao produzir dois seq
    -- iguais e a thread perder a ordem em silencio.
    seq INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    -- Separado do lifecycle da conversa pelo mesmo motivo que processing_state
    -- e separado de lifecycle_state em Capture (ADR-015).
    status TEXT NOT NULL
        CHECK (status IN ('pending', 'streaming', 'complete', 'interrupted', 'failed')),
    created_at TEXT NOT NULL,
    UNIQUE (conversation_id, seq)
) STRICT;

CREATE INDEX messages_conversation_order ON messages(conversation_id, seq);

CREATE TABLE message_parts (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    -- Sem CHECK de proposito. O dominio valida a forma do payload, e
    -- PartBody::from_payload ja degrada um kind desconhecido para um erro
    -- legivel. Uma CHECK aqui transformaria "gravado por uma versao mais nova"
    -- em falha de escrita, que e pior que uma linha que o app nao sabe desenhar.
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    -- So partes de texto preenchem. Raciocinio e payload de ferramenta ficam
    -- fora da busca por decisao: crescem sem limite e empurrariam a resposta
    -- util para fora do resultado.
    search_text TEXT NOT NULL DEFAULT '',
    UNIQUE (message_id, seq)
) STRICT;

CREATE INDEX message_parts_message_order ON message_parts(message_id, seq);

CREATE VIRTUAL TABLE message_search USING fts5(
    search_text,
    content='message_parts',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

-- Triggers, e nao manutencao manual como nas outras projecoes deste schema.
--
-- A diferenca e a taxa de mudanca: Capture e Resource sao escritos uma vez e
-- editados raramente, entao manter o indice na mao no repositorio e barato e
-- visivel. Partes de mensagem sao apagadas e reinseridas a cada resposta que
-- termina, em tres caminhos diferentes (fechar, truncar, reidratar). Uma
-- chamada esquecida em um deles corromperia a busca em silencio, e o teste que
-- pegaria isso e justamente o que ninguem escreve.
CREATE TRIGGER message_parts_search_insert AFTER INSERT ON message_parts BEGIN
    INSERT INTO message_search(rowid, search_text) VALUES (new.rowid, new.search_text);
END;

CREATE TRIGGER message_parts_search_delete AFTER DELETE ON message_parts BEGIN
    INSERT INTO message_search(message_search, rowid, search_text)
        VALUES ('delete', old.rowid, old.search_text);
END;

CREATE TRIGGER message_parts_search_update AFTER UPDATE ON message_parts BEGIN
    INSERT INTO message_search(message_search, rowid, search_text)
        VALUES ('delete', old.rowid, old.search_text);
    INSERT INTO message_search(rowid, search_text) VALUES (new.rowid, new.search_text);
END;

PRAGMA user_version = 10;

COMMIT;

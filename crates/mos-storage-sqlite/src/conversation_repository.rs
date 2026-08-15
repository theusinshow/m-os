//! Persistencia da conversa do Hermes (ADR-025).
//!
//! O indice de busca e mantido por trigger, e nao a mao como nas outras
//! projecoes: partes de mensagem sao apagadas e reinseridas a cada resposta que
//! termina, em tres caminhos diferentes. Ver `0010_conversations.sql`.

use mos_core::{
    Conversation, ConversationId, ConversationRepository, ConversationSummary, CoreError,
    ErrorCode, LifecycleState, Message, MessageId, MessagePart, MessagePartId, MessageRole,
    MessageStatus, NewConversation, NewMessage, PartBody, SearchRequest,
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time, to_fts_query},
    SqliteStorage,
};

const CONVERSATION_COLUMNS: &str =
    "id, title, hermes_session_id, lifecycle_state, created_at, updated_at";

fn conversation_from_row(row: &Row<'_>) -> Result<Conversation, rusqlite::Error> {
    Ok(Conversation {
        id: ConversationId::parse(&row.get::<_, String>(0)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        title: row.get(1)?,
        hermes_session_id: row.get(2)?,
        lifecycle_state: LifecycleState::parse(&row.get::<_, String>(3)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        created_at: parse_time(&row.get::<_, String>(4)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        updated_at: parse_time(&row.get::<_, String>(5)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

/// Marca a conversa como mexida.
///
/// Explicito no repositorio, e nao em trigger, porque isto e semantica de
/// aplicacao: o que conta como "mexer" numa conversa e decisao do produto, e
/// enterrar essa decisao num trigger a esconderia de quem le o caso de uso.
fn touch(
    transaction: &Transaction<'_>,
    conversation_id: &str,
    now: OffsetDateTime,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "UPDATE conversations SET updated_at = ?2 WHERE id = ?1",
            params![conversation_id, format_time(now)?],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn insert_parts(
    transaction: &Transaction<'_>,
    message_id: &str,
    parts: &[PartBody],
) -> Result<(), CoreError> {
    for (index, body) in parts.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO message_parts (id, message_id, seq, kind, payload, search_text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    MessagePartId::new().to_string(),
                    message_id,
                    index as i64 + 1,
                    body.kind(),
                    body.to_payload()?,
                    body.searchable_text().unwrap_or_default(),
                ],
            )
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn load_parts(
    transaction: &Transaction<'_>,
    message_id: &str,
) -> Result<Vec<MessagePart>, CoreError> {
    let mut statement = transaction
        .prepare("SELECT id, seq, payload FROM message_parts WHERE message_id = ?1 ORDER BY seq")
        .map_err(map_sql_error)?;
    let rows = statement
        .query_map([message_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(map_sql_error)?;

    let mut parts = Vec::new();
    for row in rows {
        let (id, seq, payload) = row.map_err(map_sql_error)?;
        parts.push(MessagePart {
            id: MessagePartId::parse(&id)?,
            seq,
            body: PartBody::from_payload(&payload),
        });
    }
    Ok(parts)
}

fn load_message(transaction: &Transaction<'_>, message_id: &str) -> Result<Message, CoreError> {
    let (conversation_id, seq, role, status, created_at) = transaction
        .query_row(
            "SELECT conversation_id, seq, role, status, created_at FROM messages WHERE id = ?1",
            [message_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Mensagem nao encontrada.", false))?;

    Ok(Message {
        id: MessageId::parse(message_id)?,
        conversation_id: ConversationId::parse(&conversation_id)?,
        seq,
        role: MessageRole::parse(&role)?,
        status: MessageStatus::parse(&status)?,
        created_at: parse_time(&created_at)?,
        parts: load_parts(transaction, message_id)?,
    })
}

/// Monta o resumo de uma conversa a partir das linhas ja lidas.
fn summarize(
    transaction: &Transaction<'_>,
    id: &str,
    title: String,
    updated_at: &str,
) -> Result<ConversationSummary, CoreError> {
    let message_count: i64 = transaction
        .query_row(
            "SELECT count(*) FROM messages WHERE conversation_id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(map_sql_error)?;

    // Primeira linha da ultima parte de texto. E o que a lista mostra quando
    // ainda nao ha titulo — e uma conversa sem titulo e sem previa nao diz nada.
    let preview: Option<String> = transaction
        .query_row(
            "SELECT p.search_text
             FROM message_parts p
             JOIN messages m ON m.id = p.message_id
             WHERE m.conversation_id = ?1 AND p.search_text <> ''
             ORDER BY m.seq DESC, p.seq DESC
             LIMIT 1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sql_error)?;

    let preview = preview
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(140)
        .collect();

    Ok(ConversationSummary {
        id: ConversationId::parse(id)?,
        title,
        updated_at: parse_time(updated_at)?,
        message_count,
        preview,
    })
}

impl ConversationRepository for SqliteStorage {
    fn create_conversation(
        &self,
        conversation: NewConversation,
    ) -> Result<Conversation, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let now = format_time(conversation.created_at)?;
        transaction
            .execute(
                "INSERT INTO conversations (id, title, hermes_session_id, lifecycle_state,
                                            created_at, updated_at)
                 VALUES (?1, ?2, NULL, 'active', ?3, ?3)",
                params![conversation.id.to_string(), conversation.title, now],
            )
            .map_err(map_sql_error)?;
        let created = transaction
            .query_row(
                &format!("SELECT {CONVERSATION_COLUMNS} FROM conversations WHERE id = ?1"),
                [conversation.id.to_string()],
                conversation_from_row,
            )
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(created)
    }

    fn get_conversation(&self, id: ConversationId) -> Result<Conversation, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                &format!("SELECT {CONVERSATION_COLUMNS} FROM conversations WHERE id = ?1"),
                [id.to_string()],
                conversation_from_row,
            )
            .optional()
            .map_err(map_sql_error)?
            .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Conversa nao encontrada.", false))
    }

    fn conversations(
        &self,
        include_archived: bool,
        limit: usize,
    ) -> Result<Vec<ConversationSummary>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let lifecycle = if include_archived {
            "lifecycle_state IN ('active', 'archived')"
        } else {
            "lifecycle_state = 'active'"
        };

        let rows: Vec<(String, String, String)> = {
            let mut statement = transaction
                .prepare(&format!(
                    "SELECT id, title, updated_at FROM conversations
                     WHERE {lifecycle} ORDER BY updated_at DESC LIMIT ?1"
                ))
                .map_err(map_sql_error)?;
            let mapped = statement
                .query_map([limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(map_sql_error)?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(map_sql_error)?);
            }
            collected
        };

        let mut summaries = Vec::with_capacity(rows.len());
        for (id, title, updated_at) in rows {
            summaries.push(summarize(&transaction, &id, title, &updated_at)?);
        }
        Ok(summaries)
    }

    fn set_conversation_title(
        &self,
        id: ConversationId,
        title: &str,
    ) -> Result<Conversation, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE conversations SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    title,
                    format_time(OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Conversa nao encontrada.",
                false,
            ));
        }
        drop(connection);
        self.get_conversation(id)
    }

    fn set_conversation_session(
        &self,
        id: ConversationId,
        hermes_session_id: Option<&str>,
    ) -> Result<Conversation, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // updated_at nao muda aqui: vincular a sessao e manutencao interna, e
        // deixar isso reordenar a lista faria uma conversa antiga pular para o
        // topo so porque o app reconectou.
        let changed = connection
            .execute(
                "UPDATE conversations SET hermes_session_id = ?2 WHERE id = ?1",
                params![id.to_string(), hermes_session_id],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Conversa nao encontrada.",
                false,
            ));
        }
        drop(connection);
        self.get_conversation(id)
    }

    fn set_conversation_lifecycle(
        &self,
        id: ConversationId,
        lifecycle: LifecycleState,
    ) -> Result<Conversation, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE conversations SET lifecycle_state = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    lifecycle.as_str(),
                    format_time(OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Conversa nao encontrada.",
                false,
            ));
        }
        drop(connection);
        self.get_conversation(id)
    }

    fn delete_conversation(&self, id: ConversationId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        // Sem guard_deletable: diferente de Capture, Task e Resource, uma
        // conversa nao e informacao que o usuario colocou no sistema para
        // encontrar depois — e o registro de uma troca. Exigir arquivar antes
        // de apagar seria cerimonia sem a confianca que ela protege nos outros.
        let removed = transaction
            .execute("DELETE FROM conversations WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        if removed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Conversa nao encontrada.",
                false,
            ));
        }
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn append_message(&self, message: NewMessage) -> Result<Message, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let conversation_id = message.conversation_id.to_string();

        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM conversations WHERE id = ?1)",
                [&conversation_id],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        if !exists {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Conversa nao encontrada.",
                false,
            ));
        }

        let seq: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE conversation_id = ?1",
                [&conversation_id],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;

        let message_id = message.id.to_string();
        transaction
            .execute(
                "INSERT INTO messages (id, conversation_id, seq, role, status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    message_id,
                    conversation_id,
                    seq,
                    message.role.as_str(),
                    message.status.as_str(),
                    format_time(message.created_at)?,
                ],
            )
            .map_err(map_sql_error)?;
        insert_parts(&transaction, &message_id, &message.parts)?;
        touch(&transaction, &conversation_id, message.created_at)?;

        let stored = load_message(&transaction, &message_id)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(stored)
    }

    fn messages(&self, id: ConversationId) -> Result<Vec<Message>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let ids: Vec<String> = {
            let mut statement = transaction
                .prepare("SELECT id FROM messages WHERE conversation_id = ?1 ORDER BY seq")
                .map_err(map_sql_error)?;
            let mapped = statement
                .query_map([id.to_string()], |row| row.get::<_, String>(0))
                .map_err(map_sql_error)?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(map_sql_error)?);
            }
            collected
        };

        let mut messages = Vec::with_capacity(ids.len());
        for message_id in ids {
            messages.push(load_message(&transaction, &message_id)?);
        }
        Ok(messages)
    }

    fn finish_message(
        &self,
        id: MessageId,
        status: MessageStatus,
        parts: Vec<PartBody>,
    ) -> Result<Message, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let message_id = id.to_string();

        let conversation_id: String = transaction
            .query_row(
                "SELECT conversation_id FROM messages WHERE id = ?1",
                [&message_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .ok_or_else(|| {
                CoreError::new(ErrorCode::NotFound, "Mensagem nao encontrada.", false)
            })?;

        transaction
            .execute(
                "UPDATE messages SET status = ?2 WHERE id = ?1",
                params![message_id, status.as_str()],
            )
            .map_err(map_sql_error)?;
        transaction
            .execute(
                "DELETE FROM message_parts WHERE message_id = ?1",
                [&message_id],
            )
            .map_err(map_sql_error)?;
        insert_parts(&transaction, &message_id, &parts)?;
        touch(&transaction, &conversation_id, OffsetDateTime::now_utc())?;

        let stored = load_message(&transaction, &message_id)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(stored)
    }

    fn truncate_from(&self, message_id: MessageId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let id = message_id.to_string();

        let (conversation_id, seq) = transaction
            .query_row(
                "SELECT conversation_id, seq FROM messages WHERE id = ?1",
                [&id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(map_sql_error)?
            .ok_or_else(|| {
                CoreError::new(ErrorCode::NotFound, "Mensagem nao encontrada.", false)
            })?;

        transaction
            .execute(
                "DELETE FROM messages WHERE conversation_id = ?1 AND seq >= ?2",
                params![conversation_id, seq],
            )
            .map_err(map_sql_error)?;
        touch(&transaction, &conversation_id, OffsetDateTime::now_utc())?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn replace_messages(
        &self,
        id: ConversationId,
        messages: Vec<NewMessage>,
    ) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let conversation_id = id.to_string();

        transaction
            .execute(
                "DELETE FROM messages WHERE conversation_id = ?1",
                [&conversation_id],
            )
            .map_err(map_sql_error)?;

        for (index, message) in messages.iter().enumerate() {
            let message_id = message.id.to_string();
            transaction
                .execute(
                    "INSERT INTO messages (id, conversation_id, seq, role, status, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        message_id,
                        conversation_id,
                        index as i64 + 1,
                        message.role.as_str(),
                        message.status.as_str(),
                        format_time(message.created_at)?,
                    ],
                )
                .map_err(map_sql_error)?;
            insert_parts(&transaction, &message_id, &message.parts)?;
        }
        touch(&transaction, &conversation_id, OffsetDateTime::now_utc())?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn search_conversations(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<ConversationSummary>, CoreError> {
        let query = to_fts_query(&request.query);
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let lifecycle = if request.include_archived {
            "c.lifecycle_state IN ('active', 'archived')"
        } else {
            "c.lifecycle_state = 'active'"
        };

        let rows: Vec<(String, String, String)> = {
            let mut statement = transaction
                .prepare(&format!(
                    "SELECT DISTINCT c.id, c.title, c.updated_at
                     FROM message_search s
                     JOIN message_parts p ON p.rowid = s.rowid
                     JOIN messages m ON m.id = p.message_id
                     JOIN conversations c ON c.id = m.conversation_id
                     WHERE message_search MATCH ?1 AND {lifecycle}
                     ORDER BY c.updated_at DESC
                     LIMIT ?2"
                ))
                .map_err(map_sql_error)?;
            let mapped = statement
                .query_map(params![query, request.limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(map_sql_error)?;
            let mut collected = Vec::new();
            for row in mapped {
                collected.push(row.map_err(map_sql_error)?);
            }
            collected
        };

        let mut summaries = Vec::with_capacity(rows.len());
        for (id, title, updated_at) in rows {
            summaries.push(summarize(&transaction, &id, title, &updated_at)?);
        }
        Ok(summaries)
    }

    fn settle_unfinished_messages(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let settled = connection
            .execute(
                "UPDATE messages SET status = 'interrupted'
                 WHERE status IN ('pending', 'streaming')",
                [],
            )
            .map_err(map_sql_error)?;
        Ok(settled)
    }

    fn rebuild_conversation_search(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO message_search(message_search) VALUES('rebuild')",
                [],
            )
            .map_err(map_sql_error)?;
        let count = transaction
            .query_row("SELECT count(*) FROM message_search", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(map_sql_error)? as usize;
        transaction.commit().map_err(map_sql_error)?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{ContextEntity, ContextOrigin, ToolRunState};

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (directory, storage)
    }

    fn text(value: &str) -> PartBody {
        PartBody::Text {
            text: value.to_owned(),
        }
    }

    #[test]
    fn a_conversation_keeps_the_session_link_across_restarts() {
        let (directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        assert_eq!(conversation.hermes_session_id, None);

        storage
            .set_conversation_session(conversation.id, Some("a1b2c3d4"))
            .unwrap();
        drop(storage);

        // O ponto inteiro da ADR-025: o vinculo sobrevive ao fechamento do app.
        // Antes ele vivia num Mutex de processo e session.resume nunca rodava.
        let reopened = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        assert_eq!(
            reopened
                .get_conversation(conversation.id)
                .unwrap()
                .hermes_session_id
                .as_deref(),
            Some("a1b2c3d4")
        );
    }

    #[test]
    fn messages_keep_their_order_and_parts() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();

        storage
            .append_message(NewMessage::user(conversation.id, "o que falta?").unwrap())
            .unwrap();
        let answer = storage
            .append_message(NewMessage::pending_assistant(conversation.id))
            .unwrap();
        storage
            .finish_message(
                answer.id,
                MessageStatus::Complete,
                vec![
                    PartBody::ToolRun {
                        name: "mos_search".into(),
                        state: ToolRunState::Success,
                        detail: "{}".into(),
                    },
                    text("Faltam duas tasks."),
                ],
            )
            .unwrap();

        let messages = storage.messages(conversation.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].seq, 1);
        assert_eq!(messages[0].text(), "o que falta?");
        assert_eq!(messages[1].status, MessageStatus::Complete);
        assert_eq!(messages[1].parts.len(), 2);
        assert_eq!(messages[1].text(), "Faltam duas tasks.");
    }

    /// O indice e mantido por trigger justamente porque as partes sao apagadas
    /// e reinseridas em tres caminhos. Se um deles escapasse, a busca passaria
    /// a devolver conversa que nao contem mais o termo — em silencio.
    #[test]
    fn search_follows_the_parts_through_delete_and_reinsert() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        let answer = storage
            .append_message(NewMessage::pending_assistant(conversation.id))
            .unwrap();

        storage
            .finish_message(
                answer.id,
                MessageStatus::Complete,
                vec![text("biblioteca de animacao")],
            )
            .unwrap();
        assert_eq!(
            storage
                .search_conversations(SearchRequest {
                    query: "animacao".into(),
                    include_archived: false,
                    limit: 10,
                })
                .unwrap()
                .len(),
            1
        );

        // Regenerar a resposta troca as partes. O termo antigo tem que sumir do
        // indice junto com a parte que o continha.
        storage
            .finish_message(
                answer.id,
                MessageStatus::Complete,
                vec![text("outra coisa completamente")],
            )
            .unwrap();
        assert!(
            storage
                .search_conversations(SearchRequest {
                    query: "animacao".into(),
                    include_archived: false,
                    limit: 10,
                })
                .unwrap()
                .is_empty(),
            "o indice ficou com a parte que foi apagada"
        );
    }

    /// Raciocinio e ferramenta ficam fora da busca por decisao: crescem sem
    /// limite e empurrariam a resposta util para fora do resultado.
    #[test]
    fn reasoning_and_tool_parts_do_not_reach_the_index() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        let answer = storage
            .append_message(NewMessage::pending_assistant(conversation.id))
            .unwrap();
        storage
            .finish_message(
                answer.id,
                MessageStatus::Complete,
                vec![
                    PartBody::Reasoning {
                        text: "esotericamente".into(),
                    },
                    PartBody::ToolRun {
                        name: "esotericamente".into(),
                        state: ToolRunState::Success,
                        detail: String::new(),
                    },
                ],
            )
            .unwrap();

        assert!(storage
            .search_conversations(SearchRequest {
                query: "esotericamente".into(),
                include_archived: false,
                limit: 10,
            })
            .unwrap()
            .is_empty());
    }

    /// Regenerate e editar-e-reenviar dependem disto: a resposta antiga e tudo
    /// que veio atras dela deixam de valer quando a pergunta muda.
    #[test]
    fn truncating_removes_the_message_and_everything_after_it() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        storage
            .append_message(NewMessage::user(conversation.id, "primeira").unwrap())
            .unwrap();
        let second = storage
            .append_message(NewMessage::user(conversation.id, "segunda").unwrap())
            .unwrap();
        storage
            .append_message(NewMessage::user(conversation.id, "terceira").unwrap())
            .unwrap();

        storage.truncate_from(second.id).unwrap();
        let remaining = storage.messages(conversation.id).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].text(), "primeira");
    }

    /// O app pode ser fechado no meio de um turno. Sem o reparo de abertura, a
    /// resposta voltaria eternamente em curso.
    #[test]
    fn an_unfinished_message_is_settled_on_reopen() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        storage
            .append_message(NewMessage::pending_assistant(conversation.id))
            .unwrap();

        assert_eq!(storage.settle_unfinished_messages().unwrap(), 1);
        assert_eq!(
            storage.messages(conversation.id).unwrap()[0].status,
            MessageStatus::Interrupted
        );
        // Idempotente: a segunda abertura nao tem o que reparar.
        assert_eq!(storage.settle_unfinished_messages().unwrap(), 0);
    }

    #[test]
    fn the_list_shows_the_last_text_as_preview() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        storage
            .append_message(NewMessage::user(conversation.id, "primeira linha\nsegunda").unwrap())
            .unwrap();

        let listed = storage.conversations(false, 10).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].message_count, 1);
        assert_eq!(listed[0].preview, "primeira linha");
        assert!(listed[0].title.is_empty(), "o M/OS nao inventa titulo");
    }

    /// O registro do que foi enviado sobrevive a ida e volta pelo banco: e a
    /// evidencia que a ADR-027 exige.
    #[test]
    fn the_context_record_survives_persistence() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        let message = storage
            .append_message(NewMessage::user(conversation.id, "o que falta aqui?").unwrap())
            .unwrap();
        storage
            .finish_message(
                message.id,
                MessageStatus::Complete,
                vec![
                    text("o que falta aqui?"),
                    PartBody::ContextRef {
                        origin: ContextOrigin::Explicit,
                        entity: ContextEntity::Project,
                        id: "0198a7d5-a64e-7000-8000-000000000001".into(),
                        label: "M/OS".into(),
                        fields: vec!["name".into(), "openTasks".into()],
                        bytes: 412,
                    },
                ],
            )
            .unwrap();

        let stored = &storage.messages(conversation.id).unwrap()[0];
        match &stored.parts[1].body {
            PartBody::ContextRef {
                label,
                bytes,
                fields,
                ..
            } => {
                assert_eq!(label, "M/OS");
                assert_eq!(*bytes, 412);
                assert_eq!(fields.len(), 2);
            }
            other => panic!("esperava ContextRef, veio {other:?}"),
        }
    }

    #[test]
    fn deleting_a_conversation_takes_its_messages_and_parts() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        storage
            .append_message(NewMessage::user(conversation.id, "some junto").unwrap())
            .unwrap();

        storage.delete_conversation(conversation.id).unwrap();
        assert!(storage.get_conversation(conversation.id).is_err());
        assert!(storage
            .search_conversations(SearchRequest {
                query: "junto".into(),
                include_archived: false,
                limit: 10,
            })
            .unwrap()
            .is_empty());
    }

    #[test]
    fn history_from_the_vps_replaces_the_local_thread() {
        let (_directory, storage) = storage();
        let conversation = storage
            .create_conversation(NewConversation::create())
            .unwrap();
        storage
            .append_message(NewMessage::user(conversation.id, "local antiga").unwrap())
            .unwrap();

        storage
            .replace_messages(
                conversation.id,
                vec![
                    NewMessage::user(conversation.id, "da vps").unwrap(),
                    NewMessage::pending_assistant(conversation.id),
                ],
            )
            .unwrap();

        let messages = storage.messages(conversation.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text(), "da vps");
        assert!(storage
            .search_conversations(SearchRequest {
                query: "antiga".into(),
                include_archived: false,
                limit: 10,
            })
            .unwrap()
            .is_empty());
    }
}

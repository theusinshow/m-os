//! Persistencia da Daily Session.
//!
//! # As duas transacoes que este arquivo existe para garantir
//!
//! **Comecar o dia e um gesto so.** `start_day` fecha a sessao que ficou aberta
//! de um dia anterior, cria a de hoje e grava todos os objetivos — num commit.
//! Em sequencia, uma queda no meio deixaria uma sessao vazia que a pessoa
//! acabou de montar com tres objetivos, ou dois dias `active` ao mesmo tempo, e
//! ai a pergunta "qual e o dia de hoje?" passaria a ter duas respostas.
//!
//! **Encerrar o dia tambem.** Os destinos dos objetivos pendentes, a reflexao e
//! o fecho da sessao caem juntos. Metade disso gravada e um dia que mente sobre
//! o proprio placar.
//!
//! # Emissao
//!
//! Toda mutacao emite a operacao de sincronizacao **dentro da mesma transacao**,
//! pela regra do `sync_emit.rs`. Tres tipos novos viajam: `daily_session`,
//! `daily_objective` e `daily_reflection`. `EntityKind` e texto no contrato
//! justamente para isto: um cliente antigo guarda e reenvia sem saber o que sao.
//!
//! Remover um objetivo emite `OpBody::Delete`, e a assimetria e deliberada:
//! diferente de arquivar uma Capture — que e mudanca de campo porque tem volta —
//! tirar um objetivo do dia nao tem. Quem quer manter o registro usa `dropped`.

use mos_core::{
    CoreError, DailyObjective, DailyObjectiveId, DailyReflection, DailyRepository, DailySession,
    DailySessionId, Day, DayMood, ErrorCode, NewDailyObjective, NewDailyReflection,
    NewDailySession, ObjectiveLink, ObjectivePriority, ObjectiveStatus, SearchRequest,
    SessionStatus,
};
use rusqlite::{params, Connection, Row};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

const SESSION_COLUMNS: &str =
    "id, day, status, note, started_at, ended_at, created_at, updated_at";

const OBJECTIVE_COLUMNS: &str = "id, session_id, title, description, link_kind, link_id, \
     priority, status, position, carried_from, created_at, updated_at, completed_at";

/// O tipo que viaja no contrato de sincronizacao. Texto, e nunca enum: ver
/// `mos_sync::EntityKind`.
const KIND_SESSION: &str = "daily_session";
const KIND_OBJECTIVE: &str = "daily_objective";
const KIND_REFLECTION: &str = "daily_reflection";

fn not_found(what: &str) -> CoreError {
    CoreError::new(ErrorCode::NotFound, what, false)
}

fn read_session(row: &Row<'_>) -> rusqlite::Result<Result<DailySession, CoreError>> {
    let id: String = row.get(0)?;
    let day: String = row.get(1)?;
    let status: String = row.get(2)?;
    let note: String = row.get(3)?;
    let started_at: String = row.get(4)?;
    let ended_at: Option<String> = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: String = row.get(7)?;

    Ok((|| {
        Ok(DailySession {
            id: DailySessionId::parse(&id)?,
            day: Day::parse(&day)?,
            status: SessionStatus::parse(&status)?,
            note,
            started_at: parse_time(&started_at)?,
            ended_at: ended_at.as_deref().map(parse_time).transpose()?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn read_objective(row: &Row<'_>) -> rusqlite::Result<Result<DailyObjective, CoreError>> {
    let id: String = row.get(0)?;
    let session_id: String = row.get(1)?;
    let title: String = row.get(2)?;
    let description: String = row.get(3)?;
    let link_kind: Option<String> = row.get(4)?;
    let link_id: Option<String> = row.get(5)?;
    let priority: String = row.get(6)?;
    let status: String = row.get(7)?;
    let position: i64 = row.get(8)?;
    let carried_from: Option<String> = row.get(9)?;
    let created_at: String = row.get(10)?;
    let updated_at: String = row.get(11)?;
    let completed_at: Option<String> = row.get(12)?;

    Ok((|| {
        // Vinculo ILEGIVEL vira ausencia, e nao erro.
        //
        // Um objetivo e o registro do que importou naquele dia; se a entidade
        // apontada sumiu ou o par ficou meio gravado, perder a linha inteira
        // seria apagar historia por causa de um ponteiro. A tela mostra o
        // objetivo sem o atalho — que e exatamente o que o §23 do pedido chama
        // de "objetivo vinculado a entidade deletada".
        let link = match (link_kind.as_deref(), link_id.as_deref()) {
            (Some(kind), Some(value)) => ObjectiveLink::from_columns(kind, value).ok(),
            _ => None,
        };
        Ok(DailyObjective {
            id: DailyObjectiveId::parse(&id)?,
            session_id: DailySessionId::parse(&session_id)?,
            title,
            description,
            link,
            priority: ObjectivePriority::parse(&priority)?,
            status: ObjectiveStatus::parse(&status)?,
            position,
            carried_from: carried_from.as_deref().map(DailyObjectiveId::parse).transpose()?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
            completed_at: completed_at.as_deref().map(parse_time).transpose()?,
        })
    })())
}

fn objective_fields(
    objective: &DailyObjective,
) -> Result<Vec<(&'static str, serde_json::Value)>, CoreError> {
    let (kind, id) = match &objective.link {
        Some(link) => {
            let (kind, id) = link.as_columns();
            (serde_json::json!(kind), serde_json::json!(id))
        }
        None => (serde_json::Value::Null, serde_json::Value::Null),
    };
    Ok(vec![
        ("sessionId", serde_json::json!(objective.session_id.to_string())),
        ("title", serde_json::json!(objective.title)),
        ("description", serde_json::json!(objective.description)),
        ("linkKind", kind),
        ("linkId", id),
        ("priority", serde_json::json!(objective.priority.as_str())),
        ("status", serde_json::json!(objective.status.as_str())),
        ("position", serde_json::json!(objective.position)),
        (
            "carriedFrom",
            match objective.carried_from {
                Some(origin) => serde_json::json!(origin.to_string()),
                None => serde_json::Value::Null,
            },
        ),
        (
            "completedAt",
            match objective.completed_at {
                Some(at) => serde_json::json!(format_time(at)?),
                None => serde_json::Value::Null,
            },
        ),
    ])
}

/// Insere um objetivo dentro de uma transacao ja aberta, emitindo junto.
fn insert_objective(
    storage: &SqliteStorage,
    transaction: &Connection,
    objective: &NewDailyObjective,
) -> Result<(), CoreError> {
    let (kind, id) = match &objective.link {
        Some(link) => {
            let (kind, id) = link.as_columns();
            (Some(kind.to_owned()), Some(id.to_owned()))
        }
        None => (None, None),
    };
    let now = format_time(objective.created_at)?;
    transaction
        .execute(
            "INSERT INTO daily_objectives (id, session_id, title, description, link_kind, \
             link_id, priority, status, position, carried_from, created_at, updated_at, \
             completed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8, ?9, ?10, ?10, NULL)",
            params![
                objective.id.to_string(),
                objective.session_id.to_string(),
                objective.title,
                objective.description,
                kind,
                id,
                objective.priority.as_str(),
                objective.position,
                objective.carried_from.map(|origin| origin.to_string()),
                now,
            ],
        )
        .map_err(map_sql_error)?;

    let mut fields = vec![
        ("sessionId", serde_json::json!(objective.session_id.to_string())),
        ("title", serde_json::json!(objective.title)),
        ("description", serde_json::json!(objective.description)),
        (
            "linkKind",
            kind.as_deref().map_or(serde_json::Value::Null, |value| serde_json::json!(value)),
        ),
        (
            "linkId",
            id.as_deref().map_or(serde_json::Value::Null, |value| serde_json::json!(value)),
        ),
        ("priority", serde_json::json!(objective.priority.as_str())),
        ("status", serde_json::json!("pending")),
        ("position", serde_json::json!(objective.position)),
    ];
    if let Some(origin) = objective.carried_from {
        fields.push(("carriedFrom", serde_json::json!(origin.to_string())));
    }
    storage.emitir(
        transaction,
        mos_sync::EntityRef::new(KIND_OBJECTIVE, objective.id.as_uuid()),
        mos_sync::OpBody::Create {
            fields: fields
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
        },
    )
}

/// Rebaixa o principal atual de uma sessao, se houver outro.
///
/// Dentro da transacao de quem promove, e SEMPRE antes do INSERT/UPDATE que cria
/// o novo principal: o indice unico `daily_objectives_one_main` recusaria os dois
/// ao mesmo tempo, e a ordem inversa transformaria uma promocao legitima em erro
/// de banco.
fn demote_main(
    storage: &SqliteStorage,
    transaction: &Connection,
    session: DailySessionId,
    except: Option<DailyObjectiveId>,
    now: &str,
) -> Result<(), CoreError> {
    let atual: Option<String> = transaction
        .query_row(
            "SELECT id FROM daily_objectives WHERE session_id = ?1 AND priority = 'main'",
            params![session.to_string()],
            |row| row.get(0),
        )
        .ok();
    let Some(atual) = atual else { return Ok(()) };
    let atual = DailyObjectiveId::parse(&atual)?;
    if Some(atual) == except {
        return Ok(());
    }
    transaction
        .execute(
            "UPDATE daily_objectives SET priority = 'secondary', updated_at = ?2 WHERE id = ?1",
            params![atual.to_string(), now],
        )
        .map_err(map_sql_error)?;
    storage.emitir_update(
        transaction,
        KIND_OBJECTIVE,
        atual.as_uuid(),
        &[("priority", serde_json::json!("secondary"))],
    )
}

impl SqliteStorage {
    fn query_sessions(&self, tail: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<DailySession>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!("SELECT {SESSION_COLUMNS} FROM daily_sessions {tail}"))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(args, read_session)
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }

    fn query_objectives(
        &self,
        tail: &str,
        args: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<DailyObjective>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!("SELECT {OBJECTIVE_COLUMNS} FROM daily_objectives {tail}"))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(args, read_objective)
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }

    /// Conclui os objetivos que SAO esta Task, dentro da transacao que a moveu.
    ///
    /// Chamado por `work_repository::set_task_state`. Vive aqui, e nao la, para
    /// o repositorio de trabalho nao precisar conhecer o schema do dia — mas o
    /// que ele NAO faz e decidir: quem decide e `mos_core::completes_with_task`,
    /// e o filtro `link_kind = 'task'` abaixo e a traducao dela para SQL.
    ///
    /// So a sessao ATIVA. Concluir uma Task hoje nao pode reescrever o placar de
    /// um dia que ja acabou — o historico e registro, e registro nao muda
    /// sozinho tres semanas depois.
    pub(crate) fn sync_objectives_with_task(
        &self,
        transaction: &Connection,
        task: uuid::Uuid,
        done: bool,
        now: &str,
    ) -> Result<(), CoreError> {
        // O estado de origem tambem entra na consulta: concluir procura os
        // pendentes, reabrir procura os concluidos. Sem isso, mover uma Task de
        // `done` para `review` devolveria a pendente um objetivo que a pessoa
        // tinha marcado a mao.
        let (de, para) = if done { ("pending", "completed") } else { ("completed", "pending") };
        let alvos: Vec<String> = {
            let mut statement = transaction
                .prepare(
                    "SELECT o.id FROM daily_objectives o \
                     JOIN daily_sessions s ON s.id = o.session_id \
                     WHERE o.link_kind = 'task' AND o.link_id = ?1 \
                       AND o.status = ?2 AND s.status = 'active'",
                )
                .map_err(map_sql_error)?;
            let rows = statement
                .query_map(params![task.to_string(), de], |row| row.get::<_, String>(0))
                .map_err(map_sql_error)?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(map_sql_error)?);
            }
            ids
        };

        for id in alvos {
            let objective = DailyObjectiveId::parse(&id)?;
            let stamp = done.then_some(now);
            transaction
                .execute(
                    "UPDATE daily_objectives SET status = ?2, completed_at = ?3, updated_at = ?4 \
                     WHERE id = ?1",
                    params![id, para, stamp, now],
                )
                .map_err(map_sql_error)?;
            self.emitir_update(
                transaction,
                KIND_OBJECTIVE,
                objective.as_uuid(),
                &[
                    ("status", serde_json::json!(para)),
                    (
                        "completedAt",
                        stamp.map_or(serde_json::Value::Null, |value| serde_json::json!(value)),
                    ),
                ],
            )?;
        }
        Ok(())
    }
}

impl DailyRepository for SqliteStorage {
    fn session_on(&self, day: &Day) -> Result<Option<DailySession>, CoreError> {
        Ok(self
            .query_sessions("WHERE day = ?1", &[&day.as_str()])?
            .into_iter()
            .next())
    }

    fn session(&self, id: DailySessionId) -> Result<DailySession, CoreError> {
        self.query_sessions("WHERE id = ?1", &[&id.to_string()])?
            .into_iter()
            .next()
            .ok_or_else(|| not_found("Sessao do dia nao encontrada."))
    }

    fn session_before(&self, day: &Day) -> Result<Option<DailySession>, CoreError> {
        Ok(self
            .query_sessions("WHERE day < ?1 ORDER BY day DESC LIMIT 1", &[&day.as_str()])?
            .into_iter()
            .next())
    }

    fn stale_session(&self, day: &Day) -> Result<Option<DailySession>, CoreError> {
        Ok(self
            .query_sessions(
                "WHERE day < ?1 AND status = 'active' ORDER BY day DESC LIMIT 1",
                &[&day.as_str()],
            )?
            .into_iter()
            .next())
    }

    fn objectives(&self, session: DailySessionId) -> Result<Vec<DailyObjective>, CoreError> {
        // A ordem e a do dia: principal primeiro, depois a posicao. Ordenar so
        // por posicao deixaria o principal no meio da lista quando ele fosse
        // promovido depois — e ele e a ancora visual da Home.
        self.query_objectives(
            "WHERE session_id = ?1 ORDER BY CASE priority WHEN 'main' THEN 0 ELSE 1 END, \
             position, created_at",
            &[&session.to_string()],
        )
    }

    fn objectives_of(
        &self,
        sessions: &[DailySessionId],
    ) -> Result<Vec<DailyObjective>, CoreError> {
        if sessions.is_empty() {
            return Ok(Vec::new());
        }
        // Lista de ids montada por interpolacao, e nao por parametro: `IN (?)`
        // nao aceita array em SQLite. Os ids sao UUIDs que ja passaram por
        // `parse` — nao ha texto de usuario nesta string.
        let lista = sessions
            .iter()
            .map(|id| format!("'{id}'"))
            .collect::<Vec<_>>()
            .join(", ");
        self.query_objectives(
            &format!(
                "WHERE session_id IN ({lista}) ORDER BY \
                 CASE priority WHEN 'main' THEN 0 ELSE 1 END, position, created_at"
            ),
            &[],
        )
    }

    fn objective(&self, id: DailyObjectiveId) -> Result<DailyObjective, CoreError> {
        self.query_objectives("WHERE id = ?1", &[&id.to_string()])?
            .into_iter()
            .next()
            .ok_or_else(|| not_found("Objetivo do dia nao encontrado."))
    }

    fn start_day(
        &self,
        session: NewDailySession,
        objectives: Vec<NewDailyObjective>,
        now: OffsetDateTime,
    ) -> Result<DailySession, CoreError> {
        let id = session.id;
        let momento = format_time(now)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        let ja_existe: i64 = transaction
            .query_row(
                "SELECT count(*) FROM daily_sessions WHERE day = ?1",
                params![session.day.as_str()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        if ja_existe > 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Este dia ja foi iniciado.",
                false,
            ));
        }

        // Fecha o que ficou aberto de dias anteriores.
        //
        // Os objetivos pendentes ficam PENDENTES de proposito. Marca-los como
        // abandonados seria o sistema decidindo por quem nao decidiu — e como o
        // carry-over le pendentes, eles reaparecem amanha em vez de sumirem.
        let orfas: Vec<String> = {
            let mut statement = transaction
                .prepare("SELECT id FROM daily_sessions WHERE status = 'active' AND day < ?1")
                .map_err(map_sql_error)?;
            let rows = statement
                .query_map(params![session.day.as_str()], |row| row.get::<_, String>(0))
                .map_err(map_sql_error)?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(map_sql_error)?);
            }
            ids
        };
        for orfa in orfas {
            let antiga = DailySessionId::parse(&orfa)?;
            transaction
                .execute(
                    "UPDATE daily_sessions SET status = 'completed', ended_at = ?2, \
                     updated_at = ?2 WHERE id = ?1",
                    params![orfa, momento],
                )
                .map_err(map_sql_error)?;
            self.emitir_update(
                &transaction,
                KIND_SESSION,
                antiga.as_uuid(),
                &[
                    ("status", serde_json::json!("completed")),
                    ("endedAt", serde_json::json!(momento)),
                ],
            )?;
        }

        let started = format_time(session.started_at)?;
        transaction
            .execute(
                "INSERT INTO daily_sessions (id, day, status, note, started_at, ended_at, \
                 created_at, updated_at) VALUES (?1, ?2, 'active', ?3, ?4, NULL, ?5, ?5)",
                params![
                    id.to_string(),
                    session.day.as_str(),
                    session.note,
                    started,
                    momento,
                ],
            )
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_SESSION, id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    ("day".to_owned(), serde_json::json!(session.day.as_str())),
                    ("status".to_owned(), serde_json::json!("active")),
                    ("note".to_owned(), serde_json::json!(session.note)),
                    ("startedAt".to_owned(), serde_json::json!(started)),
                ]
                .into_iter()
                .collect(),
            },
        )?;

        for objective in &objectives {
            insert_objective(self, &transaction, objective)?;
        }

        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        DailyRepository::session(self, id)
    }

    fn add_objective(&self, objective: NewDailyObjective) -> Result<DailyObjective, CoreError> {
        let id = objective.id;
        let momento = format_time(objective.created_at)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        if objective.priority == ObjectivePriority::Main {
            demote_main(self, &transaction, objective.session_id, None, &momento)?;
        }
        insert_objective(self, &transaction, &objective)?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        DailyRepository::objective(self, id)
    }

    fn save_objective(&self, objective: &DailyObjective) -> Result<DailyObjective, CoreError> {
        let (kind, id) = match &objective.link {
            Some(link) => {
                let (kind, value) = link.as_columns();
                (Some(kind.to_owned()), Some(value.to_owned()))
            }
            None => (None, None),
        };
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        if objective.priority == ObjectivePriority::Main {
            demote_main(
                self,
                &transaction,
                objective.session_id,
                Some(objective.id),
                &format_time(objective.updated_at)?,
            )?;
        }

        let changed = transaction
            .execute(
                "UPDATE daily_objectives SET title = ?2, description = ?3, link_kind = ?4, \
                 link_id = ?5, priority = ?6, status = ?7, position = ?8, updated_at = ?9, \
                 completed_at = ?10 WHERE id = ?1",
                params![
                    objective.id.to_string(),
                    objective.title,
                    objective.description,
                    kind,
                    id,
                    objective.priority.as_str(),
                    objective.status.as_str(),
                    objective.position,
                    format_time(objective.updated_at)?,
                    objective.completed_at.map(format_time).transpose()?,
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Objetivo do dia nao encontrado."));
        }

        self.emitir_update(
            &transaction,
            KIND_OBJECTIVE,
            objective.id.as_uuid(),
            &objective_fields(objective)?,
        )?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        DailyRepository::objective(self, objective.id)
    }

    fn set_main_objective(
        &self,
        id: DailyObjectiveId,
        now: OffsetDateTime,
    ) -> Result<Vec<DailyObjective>, CoreError> {
        let momento = format_time(now)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let session: String = connection
            .query_row(
                "SELECT session_id FROM daily_objectives WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => not_found("Objetivo do dia nao encontrado."),
                other => map_sql_error(other),
            })?;
        let session = DailySessionId::parse(&session)?;

        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        demote_main(self, &transaction, session, Some(id), &momento)?;
        transaction
            .execute(
                "UPDATE daily_objectives SET priority = 'main', updated_at = ?2 WHERE id = ?1",
                params![id.to_string(), momento],
            )
            .map_err(map_sql_error)?;
        self.emitir_update(
            &transaction,
            KIND_OBJECTIVE,
            id.as_uuid(),
            &[("priority", serde_json::json!("main"))],
        )?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        DailyRepository::objectives(self, session)
    }

    fn remove_objective(&self, id: DailyObjectiveId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "DELETE FROM daily_objectives WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Objetivo do dia nao encontrado."));
        }
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_OBJECTIVE, id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn reorder_objectives(
        &self,
        session: DailySessionId,
        order: &[DailyObjectiveId],
        now: OffsetDateTime,
    ) -> Result<Vec<DailyObjective>, CoreError> {
        let momento = format_time(now)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        for (position, id) in order.iter().enumerate() {
            // O `session_id` entra no WHERE: um id de outra sessao mandado por
            // engano nao pode reordenar o dia de outra pessoa.
            transaction
                .execute(
                    "UPDATE daily_objectives SET position = ?3, updated_at = ?4 \
                     WHERE id = ?1 AND session_id = ?2",
                    params![id.to_string(), session.to_string(), position as i64, momento],
                )
                .map_err(map_sql_error)?;
            self.emitir_update(
                &transaction,
                KIND_OBJECTIVE,
                id.as_uuid(),
                &[("position", serde_json::json!(position as i64))],
            )?;
        }
        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        DailyRepository::objectives(self, session)
    }

    fn end_day(
        &self,
        session: DailySessionId,
        resolutions: &[(DailyObjectiveId, ObjectiveStatus)],
        reflection: Option<NewDailyReflection>,
        now: OffsetDateTime,
    ) -> Result<DailySession, CoreError> {
        let momento = format_time(now)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        for (id, status) in resolutions {
            let stamp = (*status == ObjectiveStatus::Completed).then_some(momento.as_str());
            let changed = transaction
                .execute(
                    "UPDATE daily_objectives SET status = ?3, completed_at = ?4, updated_at = ?5 \
                     WHERE id = ?1 AND session_id = ?2",
                    params![
                        id.to_string(),
                        session.to_string(),
                        status.as_str(),
                        stamp,
                        momento
                    ],
                )
                .map_err(map_sql_error)?;
            if changed == 0 {
                return Err(not_found("Objetivo do dia nao encontrado."));
            }
            self.emitir_update(
                &transaction,
                KIND_OBJECTIVE,
                id.as_uuid(),
                &[
                    ("status", serde_json::json!(status.as_str())),
                    (
                        "completedAt",
                        stamp.map_or(serde_json::Value::Null, |value| serde_json::json!(value)),
                    ),
                ],
            )?;
        }

        if let Some(reflection) = reflection {
            let mood = reflection.mood.map(DayMood::as_str);
            transaction
                .execute(
                    "INSERT INTO daily_reflections (session_id, mood, summary, created_at, \
                     updated_at) VALUES (?1, ?2, ?3, ?4, ?4) \
                     ON CONFLICT(session_id) DO UPDATE SET mood = ?2, summary = ?3, updated_at = ?4",
                    params![session.to_string(), mood, reflection.summary, momento],
                )
                .map_err(map_sql_error)?;
            // A reflexao viaja com o id da SESSAO como id de entidade: ela e
            // uma-para-uma com o dia, e um id proprio so criaria um segundo
            // jeito de enderecar a mesma linha.
            self.emitir_update(
                &transaction,
                KIND_REFLECTION,
                session.as_uuid(),
                &[
                    (
                        "mood",
                        mood.map_or(serde_json::Value::Null, |value| serde_json::json!(value)),
                    ),
                    ("summary", serde_json::json!(reflection.summary)),
                ],
            )?;
        }

        let changed = transaction
            .execute(
                "UPDATE daily_sessions SET status = 'completed', ended_at = ?2, updated_at = ?2 \
                 WHERE id = ?1",
                params![session.to_string(), momento],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Sessao do dia nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_SESSION,
            session.as_uuid(),
            &[
                ("status", serde_json::json!("completed")),
                ("endedAt", serde_json::json!(momento)),
            ],
        )?;

        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        DailyRepository::session(self, session)
    }

    fn reopen_day(
        &self,
        session: DailySessionId,
        now: OffsetDateTime,
    ) -> Result<DailySession, CoreError> {
        let momento = format_time(now)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        // Recusa se ja houver OUTRO dia aberto. Dois dias `active` e o estado
        // que o `start_day` gasta uma transacao inteira para evitar; reabrir sem
        // esta guarda seria a porta dos fundos para o mesmo problema.
        let abertos: i64 = transaction
            .query_row(
                "SELECT count(*) FROM daily_sessions WHERE status = 'active' AND id <> ?1",
                params![session.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        if abertos > 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Ja existe um dia aberto. Encerre ele antes de reabrir outro.",
                false,
            ));
        }

        let changed = transaction
            .execute(
                "UPDATE daily_sessions SET status = 'active', ended_at = NULL, updated_at = ?2 \
                 WHERE id = ?1",
                params![session.to_string(), momento],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Sessao do dia nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_SESSION,
            session.as_uuid(),
            &[
                ("status", serde_json::json!("active")),
                ("endedAt", serde_json::Value::Null),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        DailyRepository::session(self, session)
    }

    fn reflection(&self, session: DailySessionId) -> Result<Option<DailyReflection>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let found = connection
            .query_row(
                "SELECT mood, summary, created_at, updated_at FROM daily_reflections \
                 WHERE session_id = ?1",
                params![session.to_string()],
                |row| {
                    let mood: Option<String> = row.get(0)?;
                    let summary: String = row.get(1)?;
                    let created_at: String = row.get(2)?;
                    let updated_at: String = row.get(3)?;
                    Ok((mood, summary, created_at, updated_at))
                },
            )
            .ok();
        let Some((mood, summary, created_at, updated_at)) = found else {
            return Ok(None);
        };
        Ok(Some(DailyReflection {
            session_id: session,
            mood: mood.as_deref().map(DayMood::parse).transpose()?,
            summary,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        }))
    }

    fn sessions(&self, limit: usize) -> Result<Vec<DailySession>, CoreError> {
        self.query_sessions(
            "ORDER BY day DESC LIMIT ?1",
            &[&(limit.min(365) as i64)],
        )
    }

    fn carry_depth(&self, id: DailyObjectiveId) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut atual = id.to_string();
        let mut elos = 0usize;
        // Teto explicito: um ciclo em `carried_from` — que o schema nao proibe,
        // porque a coluna aponta para a propria tabela — nao pode virar um laco
        // infinito na abertura da Home. Trezentos e sessenta e cinco elos e mais
        // dias do que qualquer corrente real vai ter.
        while elos < 365 {
            let anterior: Option<String> = connection
                .query_row(
                    "SELECT carried_from FROM daily_objectives WHERE id = ?1",
                    params![atual],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            let Some(anterior) = anterior else { break };
            elos += 1;
            atual = anterior;
        }
        Ok(elos)
    }

    fn search_objectives(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<(DailyObjective, Day)>, CoreError> {
        let termo = request.query.trim();
        if termo.is_empty() {
            return Ok(Vec::new());
        }
        // LIKE e nao FTS, e a escolha e de tamanho: o volume desta tabela e
        // limitado por dias vezes um punhado de objetivos — um ano de uso
        // intenso sao ~1500 linhas curtas. Uma tabela FTS a mais custaria uma
        // projecao a manter em toda escrita para ganhar nada num conjunto que
        // cabe num scan. As outras entidades usam FTS porque crescem sem teto.
        let padrao = format!(
            "%{}%",
            termo.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
        );
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {} , s.day FROM daily_objectives o \
                 JOIN daily_sessions s ON s.id = o.session_id \
                 WHERE o.title LIKE ?1 ESCAPE '\\' OR o.description LIKE ?1 ESCAPE '\\' \
                 ORDER BY s.day DESC, o.position LIMIT ?2",
                OBJECTIVE_COLUMNS
                    .split(", ")
                    .map(|column| format!("o.{column}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![padrao, request.limit.min(100) as i64], |row| {
                let objective = read_objective(row)?;
                let day: String = row.get(13)?;
                Ok((objective, day))
            })
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            let (objective, day) = row.map_err(map_sql_error)?;
            found.push((objective?, Day::parse(&day)?));
        }
        Ok(found)
    }
}

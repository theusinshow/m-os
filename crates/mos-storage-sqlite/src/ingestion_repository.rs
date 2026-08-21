//! Persistencia do pipeline de ingestao.
//!
//! Cada metodo aqui e um PASSO do pipeline, e cada passo e uma transacao. Nao
//! ha um `update_ingestion` generico de proposito: o que existe sao as
//! transicoes que o dominio permite, e uma transicao que ninguem escreveu e uma
//! transicao que nao acontece.

use mos_core::{
    CaptureId, CoreError, DetectedKind, DropContext, ErrorCode, ExtractionState,
    ImageSize, Ingestion, IngestionId, IngestionRepository, IngestionSource, IngestionState,
    NewCapture, NewIngestion, NewResource, ProjectId, RelationPlan, Resource, ResourceId,
    TaskId, WorkspaceId,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    resource_repository::{
        insert_resource, link_resource_project, link_resource_workspace, query_resource,
    },
    SqliteStorage,
};

const INGESTION_COLUMNS: &str = "id, source, original_name, mime, byte_size, sha256, stored_path, \
     detected_kind, state, failure, capture_id, resource_id, duplicate_of, context_page, \
     context_project_id, context_workspace_id, context_task_id, suggested_project_id, \
     relation_confidence, relation_reason, extraction_state, extraction_error, page_count, \
     image_width, image_height, created_at, updated_at";

struct RawIngestion {
    id: String,
    source: String,
    original_name: String,
    mime: String,
    byte_size: i64,
    sha256: String,
    stored_path: String,
    detected_kind: String,
    state: String,
    failure: String,
    capture_id: Option<String>,
    resource_id: Option<String>,
    duplicate_of: Option<String>,
    context_page: String,
    context_project_id: Option<String>,
    context_workspace_id: Option<String>,
    context_task_id: Option<String>,
    suggested_project_id: Option<String>,
    relation_confidence: f64,
    relation_reason: String,
    extraction_state: String,
    extraction_error: String,
    page_count: Option<i64>,
    image_width: Option<i64>,
    image_height: Option<i64>,
    created_at: String,
    updated_at: String,
}

impl RawIngestion {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            source: row.get(1)?,
            original_name: row.get(2)?,
            mime: row.get(3)?,
            byte_size: row.get(4)?,
            sha256: row.get(5)?,
            stored_path: row.get(6)?,
            detected_kind: row.get(7)?,
            state: row.get(8)?,
            failure: row.get(9)?,
            capture_id: row.get(10)?,
            resource_id: row.get(11)?,
            duplicate_of: row.get(12)?,
            context_page: row.get(13)?,
            context_project_id: row.get(14)?,
            context_workspace_id: row.get(15)?,
            context_task_id: row.get(16)?,
            suggested_project_id: row.get(17)?,
            relation_confidence: row.get(18)?,
            relation_reason: row.get(19)?,
            extraction_state: row.get(20)?,
            extraction_error: row.get(21)?,
            page_count: row.get(22)?,
            image_width: row.get(23)?,
            image_height: row.get(24)?,
            created_at: row.get(25)?,
            updated_at: row.get(26)?,
        })
    }

    fn into_ingestion(self) -> Result<Ingestion, CoreError> {
        let image_size = match (self.image_width, self.image_height) {
            (Some(width), Some(height)) => Some(ImageSize {
                width: width.max(0) as u32,
                height: height.max(0) as u32,
            }),
            _ => None,
        };
        Ok(Ingestion {
            id: IngestionId::parse(&self.id)?,
            source: IngestionSource::parse(&self.source)?,
            original_name: self.original_name,
            mime: self.mime,
            byte_size: self.byte_size.max(0) as u64,
            sha256: self.sha256,
            stored_path: self.stored_path,
            detected_kind: DetectedKind::parse(&self.detected_kind)?,
            state: IngestionState::parse(&self.state)?,
            failure: self.failure,
            capture_id: self.capture_id.as_deref().map(CaptureId::parse).transpose()?,
            resource_id: self
                .resource_id
                .as_deref()
                .map(ResourceId::parse)
                .transpose()?,
            duplicate_of: self
                .duplicate_of
                .as_deref()
                .map(ResourceId::parse)
                .transpose()?,
            context: DropContext {
                page: self.context_page,
                project_id: self
                    .context_project_id
                    .as_deref()
                    .map(ProjectId::parse)
                    .transpose()?,
                workspace_id: self
                    .context_workspace_id
                    .as_deref()
                    .map(WorkspaceId::parse)
                    .transpose()?,
                task_id: self
                    .context_task_id
                    .as_deref()
                    .map(TaskId::parse)
                    .transpose()?,
            },
            suggested_project_id: self
                .suggested_project_id
                .as_deref()
                .map(ProjectId::parse)
                .transpose()?,
            relation_confidence: self.relation_confidence as f32,
            relation_reason: self.relation_reason,
            extraction_state: ExtractionState::parse(&self.extraction_state)?,
            extraction_error: self.extraction_error,
            page_count: self.page_count.map(|value| value.max(0) as u32),
            image_size,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

impl IngestionRepository for SqliteStorage {
    fn begin_ingestion(
        &self,
        ingestion: NewIngestion,
        capture: NewCapture,
    ) -> Result<Ingestion, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let now = format_time(ingestion.created_at)?;

        // A Capture primeiro, e no MESMO commit. E ela que sustenta a promessa
        // de que nada se perde: se o processo morrer no proximo milissegundo, a
        // Inbox ja sabe dizer o que a pessoa soltou.
        crate::repository::insert_capture(&transaction, &capture, &now)?;

        let id = ingestion.id;
        transaction
            .execute(
                "INSERT INTO ingestions (
                    id, source, original_name, mime, byte_size, detected_kind, state,
                    capture_id, context_page, context_project_id, context_workspace_id,
                    context_task_id, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'receiving', ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                params![
                    id.to_string(),
                    ingestion.source.as_str(),
                    ingestion.original_name,
                    ingestion.mime,
                    ingestion.declared_size as i64,
                    ingestion.detected_kind.as_str(),
                    capture.id.to_string(),
                    ingestion.context.page,
                    ingestion.context.project_id.map(|value| value.to_string()),
                    ingestion.context.workspace_id.map(|value| value.to_string()),
                    ingestion.context.task_id.map(|value| value.to_string()),
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        insert_ingestion_search(&transaction, transaction.last_insert_rowid())?;
        transaction.commit().map_err(map_sql_error)?;
        query_ingestion(&connection, id)
    }

    fn mark_preserved(
        &self,
        id: IngestionId,
        sha256: &str,
        byte_size: u64,
        stored_path: &str,
        page_count: Option<u32>,
        image_size: Option<ImageSize>,
    ) -> Result<Ingestion, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        expect_state(&transaction, id, &["receiving"])?;
        let changed = transaction
            .execute(
                "UPDATE ingestions
                 SET state = 'preserved', sha256 = ?2, byte_size = ?3, stored_path = ?4,
                     page_count = ?5, image_width = ?6, image_height = ?7, updated_at = ?8
                 WHERE id = ?1",
                params![
                    id.to_string(),
                    sha256,
                    byte_size as i64,
                    stored_path,
                    page_count.map(|value| value as i64),
                    image_size.map(|size| size.width as i64),
                    image_size.map(|size| size.height as i64),
                    format_time(OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        transaction.commit().map_err(map_sql_error)?;
        query_ingestion(&connection, id)
    }

    fn duplicate_of(
        &self,
        sha256: &str,
        except: IngestionId,
    ) -> Result<Option<ResourceId>, CoreError> {
        if sha256.is_empty() {
            return Ok(None);
        }
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // So conta como duplicata o que ainda existe e nao foi mandado para o
        // lixo: reencontrar um arquivo que o usuario descartou e recusar a
        // copia nova seria o sistema decidindo por ele duas vezes.
        let found: Option<String> = connection
            .query_row(
                "SELECT i.resource_id
                 FROM ingestions i
                 JOIN resources r ON r.id = i.resource_id
                 WHERE i.sha256 = ?1 AND i.id <> ?2
                   AND r.lifecycle_state IN ('active', 'archived')
                 ORDER BY i.created_at ASC
                 LIMIT 1",
                params![sha256, except.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;
        found.as_deref().map(ResourceId::parse).transpose()
    }

    fn complete_ingestion(
        &self,
        id: IngestionId,
        resource: NewResource,
        plan: &RelationPlan,
    ) -> Result<(Ingestion, Resource), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        expect_state(&transaction, id, &["preserved"])?;
        let resource_id = insert_resource(self, &transaction, resource)?;
        apply_plan(&transaction, resource_id, plan)?;
        finish(
            &transaction,
            id,
            Some(resource_id),
            None,
            plan,
            plan.link_project.is_some(),
            plan.link_workspace.is_some(),
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok((
            query_ingestion(&connection, id)?,
            query_resource(&connection, resource_id)?,
        ))
    }

    fn complete_as_capture(&self, id: IngestionId) -> Result<Ingestion, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        expect_state(&transaction, id, &["preserved"])?;
        let changed = transaction
            .execute(
                "UPDATE ingestions SET state = 'completed', updated_at = ?2 WHERE id = ?1",
                params![id.to_string(), format_time(OffsetDateTime::now_utc())?],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        transaction.commit().map_err(map_sql_error)?;
        query_ingestion(&connection, id)
    }

    fn complete_as_duplicate(
        &self,
        id: IngestionId,
        existing: ResourceId,
        plan: &RelationPlan,
    ) -> Result<Ingestion, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        expect_state(&transaction, id, &["preserved"])?;

        // O que ESTA ingestao acrescentou precisa ser distinguido do que ja
        // estava ligado, ou desfazer removeria contexto alheio.
        let added_project = match plan.link_project {
            Some(project) => !has_project_link(&transaction, existing, project)?,
            None => false,
        };
        let added_workspace = match plan.link_workspace {
            Some(workspace) => !has_workspace_link(&transaction, existing, workspace)?,
            None => false,
        };
        apply_plan(&transaction, existing, plan)?;

        // A Capture do drop repetido ja cumpriu o papel dela: ela registra que a
        // pessoa trouxe aquilo de novo, e nao ha decisao pendente — o arquivo ja
        // esta no M/OS. Deixa-la na Inbox criaria tarefa a partir de um acerto.
        mark_capture_processed(&transaction, id)?;
        finish(
            &transaction,
            id,
            None,
            Some(existing),
            plan,
            added_project,
            added_workspace,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_ingestion(&connection, id)
    }

    fn fail_ingestion(
        &self,
        id: IngestionId,
        state: IngestionState,
        failure: &str,
    ) -> Result<Ingestion, CoreError> {
        if !matches!(
            state,
            IngestionState::Failed | IngestionState::Interrupted | IngestionState::Undone
        ) {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Encerrar uma ingestao exige um estado terminal.",
                false,
            ));
        }
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE ingestions SET state = ?2, failure = ?3, updated_at = ?4 WHERE id = ?1",
                params![
                    id.to_string(),
                    state.as_str(),
                    failure,
                    format_time(OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        query_ingestion(&connection, id)
    }

    fn set_extraction(
        &self,
        id: IngestionId,
        state: ExtractionState,
        text: &str,
        error: &str,
        page_count: Option<u32>,
    ) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        delete_ingestion_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE ingestions
                 SET extraction_state = ?2, extracted_text = ?3, extraction_error = ?4,
                     page_count = COALESCE(?5, page_count), updated_at = ?6
                 WHERE id = ?1",
                params![
                    id.to_string(),
                    state.as_str(),
                    text,
                    error,
                    page_count.map(|value| value as i64),
                    format_time(OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        let rowid = transaction
            .query_row(
                "SELECT rowid FROM ingestions WHERE id = ?1",
                [id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(map_sql_error)?;
        insert_ingestion_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn get_ingestion(&self, id: IngestionId) -> Result<Ingestion, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_ingestion(&connection, id)
    }

    fn ingestion_for_resource(
        &self,
        resource: ResourceId,
    ) -> Result<Option<Ingestion>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let raw = connection
            .query_row(
                &format!(
                    "SELECT {INGESTION_COLUMNS} FROM ingestions
                     WHERE resource_id = ?1 ORDER BY created_at ASC LIMIT 1"
                ),
                [resource.to_string()],
                RawIngestion::from_row,
            )
            .optional()
            .map_err(map_sql_error)?;
        raw.map(RawIngestion::into_ingestion).transpose()
    }

    fn file_ingestions(&self) -> Result<Vec<Ingestion>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_ingestions(
            &connection,
            &format!(
                "SELECT {INGESTION_COLUMNS} FROM ingestions
                 WHERE resource_id IS NOT NULL
                 ORDER BY created_at DESC"
            ),
        )
    }

    fn unfinished_ingestions(&self) -> Result<Vec<Ingestion>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_ingestions(
            &connection,
            &format!(
                "SELECT {INGESTION_COLUMNS} FROM ingestions
                 WHERE state IN ('receiving', 'preserved')
                 ORDER BY created_at ASC"
            ),
        )
    }

    fn pending_extractions(&self) -> Result<Vec<Ingestion>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_ingestions(
            &connection,
            &format!(
                "SELECT {INGESTION_COLUMNS} FROM ingestions
                 WHERE state = 'completed' AND extraction_state = 'pending'
                   AND stored_path <> '' AND resource_id IS NOT NULL
                 ORDER BY created_at ASC"
            ),
        )
    }

    fn undo_ingestion(&self, id: IngestionId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let ingestion = query_ingestion_tx(&transaction, id)?;
        let now = format_time(OffsetDateTime::now_utc())?;

        // Desfazer ARQUIVA (ADR-035). O Resource criado sai das superficies
        // ativas sem deixar de existir, e a Capture volta para a Inbox — que e
        // onde uma decisao por tomar deve estar.
        if let Some(resource) = ingestion.resource_id {
            transaction
                .execute(
                    "UPDATE resources
                     SET lifecycle_state = 'archived', archived_at = ?2, updated_at = ?2
                     WHERE id = ?1 AND lifecycle_state = 'active'",
                    params![resource.to_string(), now],
                )
                .map_err(map_sql_error)?;
        }
        if let Some(capture) = ingestion.capture_id {
            transaction
                .execute(
                    "UPDATE captures
                     SET processing_state = 'inbox', updated_at = ?2
                     WHERE id = ?1",
                    params![capture.to_string(), now],
                )
                .map_err(map_sql_error)?;
        }

        // Nas duplicatas o Resource e alheio: so as relacoes que ESTA ingestao
        // acrescentou saem, e o resto do contexto fica.
        if let Some(existing) = ingestion.duplicate_of {
            let (added_project, added_workspace) = added_links(&transaction, id)?;
            if added_project {
                if let Some(project) = ingestion.context.project_id {
                    link_resource_project(&transaction, existing, project, false)?;
                }
            }
            if added_workspace {
                if let Some(workspace) = ingestion.context.workspace_id {
                    link_resource_workspace(&transaction, existing, workspace, false)?;
                }
            }
        }

        transaction
            .execute(
                "UPDATE ingestions SET state = 'undone', updated_at = ?2 WHERE id = ?1",
                params![id.to_string(), now],
            )
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn rebuild_ingestion_search(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO ingestion_search(ingestion_search) VALUES('rebuild')",
                [],
            )
            .map_err(map_sql_error)?;
        connection
            .query_row("SELECT count(*) FROM ingestion_search", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(map_sql_error)
            .map(|count| count as usize)
    }
}

/// Aplica as relacoes de confianca alta. As sugestoes nao entram aqui: sugerir e
/// oferecer, e oferecer nao escreve nada.
fn apply_plan(
    transaction: &Transaction<'_>,
    resource: ResourceId,
    plan: &RelationPlan,
) -> Result<(), CoreError> {
    if let Some(project) = plan.link_project {
        link_resource_project(transaction, resource, project, true)?;
    }
    if let Some(workspace) = plan.link_workspace {
        link_resource_workspace(transaction, resource, workspace, true)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn finish(
    transaction: &Transaction<'_>,
    id: IngestionId,
    resource: Option<ResourceId>,
    duplicate: Option<ResourceId>,
    plan: &RelationPlan,
    added_project: bool,
    added_workspace: bool,
) -> Result<(), CoreError> {
    let changed = transaction
        .execute(
            "UPDATE ingestions
             SET state = 'completed', resource_id = ?2, duplicate_of = ?3,
                 suggested_project_id = ?4, relation_confidence = ?5, relation_reason = ?6,
                 added_project_link = ?7, added_workspace_link = ?8, updated_at = ?9
             WHERE id = ?1",
            params![
                id.to_string(),
                resource.map(|value| value.to_string()),
                duplicate.map(|value| value.to_string()),
                plan.suggest_project.map(|value| value.to_string()),
                plan.confidence as f64,
                plan.reason,
                added_project as i64,
                added_workspace as i64,
                format_time(OffsetDateTime::now_utc())?,
            ],
        )
        .map_err(map_sql_error)?;
    ensure_changed(changed)
}

fn mark_capture_processed(
    transaction: &Transaction<'_>,
    id: IngestionId,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "UPDATE captures SET processing_state = 'processed', updated_at = ?2
             WHERE id = (SELECT capture_id FROM ingestions WHERE id = ?1)",
            params![id.to_string(), format_time(OffsetDateTime::now_utc())?],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn has_project_link(
    transaction: &Transaction<'_>,
    resource: ResourceId,
    project: ProjectId,
) -> Result<bool, CoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM resource_projects
             WHERE resource_id = ?1 AND project_id = ?2)",
            params![resource.to_string(), project.to_string()],
            |row| row.get(0),
        )
        .map_err(map_sql_error)
}

fn has_workspace_link(
    transaction: &Transaction<'_>,
    resource: ResourceId,
    workspace: WorkspaceId,
) -> Result<bool, CoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM resource_workspaces
             WHERE resource_id = ?1 AND workspace_id = ?2)",
            params![resource.to_string(), workspace.to_string()],
            |row| row.get(0),
        )
        .map_err(map_sql_error)
}

fn added_links(
    transaction: &Transaction<'_>,
    id: IngestionId,
) -> Result<(bool, bool), CoreError> {
    transaction
        .query_row(
            "SELECT added_project_link, added_workspace_link FROM ingestions WHERE id = ?1",
            [id.to_string()],
            |row| Ok((row.get::<_, i64>(0)? == 1, row.get::<_, i64>(1)? == 1)),
        )
        .map_err(map_sql_error)
}

/// Recusa uma transicao que nao parte do estado esperado.
///
/// Sem esta guarda, um `ingest_finish` repetido por um clique duplo criaria um
/// segundo Resource para os mesmos bytes.
fn expect_state(
    transaction: &Transaction<'_>,
    id: IngestionId,
    allowed: &[&str],
) -> Result<(), CoreError> {
    let state: Option<String> = transaction
        .query_row(
            "SELECT state FROM ingestions WHERE id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sql_error)?;
    let state = state
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Ingestao nao encontrada.", false))?;
    if allowed.contains(&state.as_str()) {
        return Ok(());
    }
    Err(CoreError::new(
        ErrorCode::InvalidTransition,
        format!("Esta ingestao ja saiu do estado '{state}'."),
        false,
    ))
}

fn ensure_changed(changed: usize) -> Result<(), CoreError> {
    if changed == 0 {
        Err(CoreError::new(
            ErrorCode::NotFound,
            "Ingestao nao encontrada.",
            false,
        ))
    } else {
        Ok(())
    }
}

fn insert_ingestion_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO ingestion_search (rowid, original_name, extracted_text)
             SELECT rowid, original_name, extracted_text FROM ingestions WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_ingestion_search(
    transaction: &Transaction<'_>,
    id: IngestionId,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO ingestion_search(ingestion_search, rowid, original_name, extracted_text)
             SELECT 'delete', rowid, original_name, extracted_text FROM ingestions WHERE id = ?1",
            [id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn query_ingestion(connection: &Connection, id: IngestionId) -> Result<Ingestion, CoreError> {
    connection
        .query_row(
            &format!("SELECT {INGESTION_COLUMNS} FROM ingestions WHERE id = ?1"),
            [id.to_string()],
            RawIngestion::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Ingestao nao encontrada.", false))?
        .into_ingestion()
}

fn query_ingestion_tx(
    transaction: &Transaction<'_>,
    id: IngestionId,
) -> Result<Ingestion, CoreError> {
    transaction
        .query_row(
            &format!("SELECT {INGESTION_COLUMNS} FROM ingestions WHERE id = ?1"),
            [id.to_string()],
            RawIngestion::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Ingestao nao encontrada.", false))?
        .into_ingestion()
}

fn query_ingestions(connection: &Connection, sql: &str) -> Result<Vec<Ingestion>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let ingestions = statement
        .query_map([], RawIngestion::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_ingestion())
        .collect();
    ingestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{
        CaptureRepository, CaptureSource, DropContext, LifecycleState, NewProject, NewWorkspace,
        ProcessingState, RelationDecision, ResourceKind, ResourceRepository, SearchRequest,
        WorkRepository,
    };

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (directory, storage)
    }

    fn plan() -> RelationPlan {
        RelationPlan {
            link_project: None,
            link_workspace: None,
            suggest_project: None,
            confidence: 0.0,
            decision: RelationDecision::None,
            reason: String::new(),
        }
    }

    /// Abre uma ingestao de arquivo com a Capture correspondente.
    fn begin(storage: &SqliteStorage, name: &str, context: DropContext) -> Ingestion {
        let request = NewIngestion::file(name, "application/pdf", 1024, context).unwrap();
        let capture = NewCapture::create(
            &mos_core::capture_content(request.source, &request.original_name),
            CaptureSource::Drop,
        )
        .unwrap();
        storage.begin_ingestion(request, capture).unwrap()
    }

    fn preserve(storage: &SqliteStorage, id: IngestionId, hash: &str) -> Ingestion {
        storage
            .mark_preserved(
                id,
                hash,
                1024,
                &mos_core::stored_path(hash, "pdf").unwrap(),
                Some(12),
                None,
            )
            .unwrap()
    }

    fn file_resource(name: &str, capture: Option<CaptureId>) -> NewResource {
        NewResource::create(ResourceKind::File, name, "", "", capture).unwrap()
    }

    /// A promessa inteira em um teste: a Capture existe ANTES de qualquer byte.
    #[test]
    fn a_capture_nasce_no_mesmo_commit_da_ingestao() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "memorial.pdf", DropContext::default());

        assert_eq!(ingestion.state, IngestionState::Receiving);
        let capture = storage.get(ingestion.capture_id.unwrap()).unwrap();
        assert_eq!(capture.source, CaptureSource::Drop);
        assert_eq!(capture.processing_state, ProcessingState::Inbox);
        assert!(capture.content.contains("memorial.pdf"));
        assert_eq!(CaptureRepository::inbox(&storage, 10).unwrap().len(), 1);
    }

    /// Preservar e criar sao passos separados, e o Resource so aparece no fim.
    #[test]
    fn completar_cria_o_resource_e_tira_a_capture_da_inbox() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "memorial.pdf", DropContext::default());
        let ingestion = preserve(&storage, ingestion.id, &"a".repeat(64));
        assert_eq!(ingestion.state, IngestionState::Preserved);
        assert_eq!(ingestion.page_count, Some(12));

        let (closed, resource) = storage
            .complete_ingestion(
                ingestion.id,
                file_resource("memorial.pdf", ingestion.capture_id),
                &plan(),
            )
            .unwrap();

        assert_eq!(closed.state, IngestionState::Completed);
        assert_eq!(closed.resource_id, Some(resource.id));
        assert_eq!(resource.kind, ResourceKind::File);
        assert_eq!(resource.url, "");
        assert!(CaptureRepository::inbox(&storage, 10).unwrap().is_empty());
        assert_eq!(
            storage.get(closed.capture_id.unwrap()).unwrap().processing_state,
            ProcessingState::Processed
        );
    }

    /// Invariante 4: falhar DEPOIS de preservar nao destroi a captura.
    #[test]
    fn falhar_depois_de_preservar_deixa_a_capture_na_inbox() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "quebrado.pdf", DropContext::default());
        let ingestion = preserve(&storage, ingestion.id, &"b".repeat(64));

        let failed = storage
            .fail_ingestion(ingestion.id, IngestionState::Failed, "o parser explodiu")
            .unwrap();

        assert_eq!(failed.state, IngestionState::Failed);
        assert_eq!(failed.failure, "o parser explodiu");
        // O original continua endereçado, e a Capture continua esperando decisao.
        assert!(!failed.stored_path.is_empty());
        assert_eq!(CaptureRepository::inbox(&storage, 10).unwrap().len(), 1);
    }

    /// Contexto de confianca alta vira relacao, e a relacao entra no mesmo
    /// commit do Resource.
    #[test]
    fn o_contexto_do_drop_vira_relacao() {
        let (_directory, storage) = storage();
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engenharia", "").unwrap())
            .unwrap();
        let context = DropContext {
            page: "projects".into(),
            project_id: Some(project.id),
            workspace_id: Some(workspace.id),
            task_id: None,
        };
        let plano = mos_core::plan_relations(&context, "pricing.pdf", &[]);
        assert_eq!(plano.decision, RelationDecision::Link);

        let ingestion = begin(&storage, "pricing.pdf", context);
        let ingestion = preserve(&storage, ingestion.id, &"c".repeat(64));
        let (closed, resource) = storage
            .complete_ingestion(
                ingestion.id,
                file_resource("pricing.pdf", ingestion.capture_id),
                &plano,
            )
            .unwrap();

        assert_eq!(
            storage.resource_projects().unwrap()[0].resource_id,
            resource.id
        );
        assert_eq!(
            storage.resource_workspaces().unwrap()[0].workspace_id,
            workspace.id
        );
        assert!(closed.relation_confidence >= mos_core::CONFIDENCE_LINK);
        assert!(!closed.relation_reason.is_empty());
    }

    /// O mesmo arquivo duas vezes nao vira dois Resources — e o contexto novo
    /// nao se perde.
    #[test]
    fn arquivo_repetido_aplica_contexto_no_que_ja_existia() {
        let (_directory, storage) = storage();
        let hash = "d".repeat(64);
        let primeira = begin(&storage, "memorial.pdf", DropContext::default());
        let primeira = preserve(&storage, primeira.id, &hash);
        let (_, resource) = storage
            .complete_ingestion(
                primeira.id,
                file_resource("memorial.pdf", primeira.capture_id),
                &plan(),
            )
            .unwrap();

        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let segunda = begin(
            &storage,
            "memorial.pdf",
            DropContext {
                page: "projects".into(),
                project_id: Some(project.id),
                workspace_id: None,
                task_id: None,
            },
        );
        let segunda = preserve(&storage, segunda.id, &hash);
        let existente = storage.duplicate_of(&hash, segunda.id).unwrap();
        assert_eq!(existente, Some(resource.id));

        let mut plano = plan();
        plano.link_project = Some(project.id);
        let fechada = storage
            .complete_as_duplicate(segunda.id, resource.id, &plano)
            .unwrap();

        assert_eq!(fechada.duplicate_of, Some(resource.id));
        assert!(fechada.resource_id.is_none());
        assert_eq!(ResourceRepository::resources(&storage, true).unwrap().len(), 1);
        assert_eq!(storage.resource_projects().unwrap().len(), 1);
        assert!(CaptureRepository::inbox(&storage, 10).unwrap().is_empty());
    }

    /// Desfazer arquiva o que nasceu e devolve a Capture — nunca apaga.
    #[test]
    fn desfazer_arquiva_o_resource_e_devolve_a_capture() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "memorial.pdf", DropContext::default());
        let ingestion = preserve(&storage, ingestion.id, &"e".repeat(64));
        let (closed, resource) = storage
            .complete_ingestion(
                ingestion.id,
                file_resource("memorial.pdf", ingestion.capture_id),
                &plan(),
            )
            .unwrap();

        storage.undo_ingestion(closed.id).unwrap();

        assert_eq!(
            storage.get_resource(resource.id).unwrap().lifecycle_state,
            LifecycleState::Archived
        );
        assert_eq!(
            storage.get(closed.capture_id.unwrap()).unwrap().processing_state,
            ProcessingState::Inbox
        );
        assert_eq!(
            storage.get_ingestion(closed.id).unwrap().state,
            IngestionState::Undone
        );
    }

    /// Desfazer uma duplicata remove SO o que ela acrescentou.
    #[test]
    fn desfazer_duplicata_preserva_a_relacao_que_ja_existia() {
        let (_directory, storage) = storage();
        let hash = "f".repeat(64);
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engenharia", "").unwrap())
            .unwrap();
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();

        let primeira = begin(&storage, "memorial.pdf", DropContext::default());
        let primeira = preserve(&storage, primeira.id, &hash);
        let (_, resource) = storage
            .complete_ingestion(
                primeira.id,
                file_resource("memorial.pdf", primeira.capture_id),
                &plan(),
            )
            .unwrap();
        // A relacao com o Workspace ja existia ANTES do segundo drop.
        storage
            .set_resource_workspace(resource.id, workspace.id, true)
            .unwrap();

        let segunda = begin(
            &storage,
            "memorial.pdf",
            DropContext {
                page: "projects".into(),
                project_id: Some(project.id),
                workspace_id: Some(workspace.id),
                task_id: None,
            },
        );
        let segunda = preserve(&storage, segunda.id, &hash);
        let mut plano = plan();
        plano.link_project = Some(project.id);
        plano.link_workspace = Some(workspace.id);
        let fechada = storage
            .complete_as_duplicate(segunda.id, resource.id, &plano)
            .unwrap();

        storage.undo_ingestion(fechada.id).unwrap();

        // O Project entrou com esta ingestao e sai com o desfazer.
        assert!(storage.resource_projects().unwrap().is_empty());
        // O Workspace ja estava ligado e continua ligado.
        assert_eq!(storage.resource_workspaces().unwrap().len(), 1);
        // E o Resource alheio nao foi arquivado.
        assert_eq!(
            storage.get_resource(resource.id).unwrap().lifecycle_state,
            LifecycleState::Active
        );
    }

    /// O texto extraido alimenta a busca de Resources sem um segundo mecanismo.
    #[test]
    fn o_texto_extraido_encontra_o_resource_na_busca() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "memorial.pdf", DropContext::default());
        let ingestion = preserve(&storage, ingestion.id, &"1".repeat(64));
        let (closed, resource) = storage
            .complete_ingestion(
                ingestion.id,
                file_resource("memorial.pdf", ingestion.capture_id),
                &plan(),
            )
            .unwrap();

        storage
            .set_extraction(
                closed.id,
                ExtractionState::Done,
                "Fundacao em radier sobre solo argiloso compactado",
                "",
                Some(42),
            )
            .unwrap();

        let encontrados = ResourceRepository::search_resources(
            &storage,
            SearchRequest {
                query: "radier".into(),
                include_archived: false,
                limit: 10,
            },
        )
        .unwrap();
        assert_eq!(encontrados.len(), 1);
        assert_eq!(encontrados[0].id, resource.id);
        assert_eq!(storage.get_ingestion(closed.id).unwrap().page_count, Some(42));
    }

    /// Extracao que falha nao mexe no que ja foi preservado.
    #[test]
    fn extracao_que_falha_nao_desfaz_a_preservacao() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "escaneado.pdf", DropContext::default());
        let ingestion = preserve(&storage, ingestion.id, &"2".repeat(64));
        let (closed, resource) = storage
            .complete_ingestion(
                ingestion.id,
                file_resource("escaneado.pdf", ingestion.capture_id),
                &plan(),
            )
            .unwrap();

        storage
            .set_extraction(closed.id, ExtractionState::Failed, "", "PDF cifrado", None)
            .unwrap();

        let depois = storage.get_ingestion(closed.id).unwrap();
        assert_eq!(depois.extraction_state, ExtractionState::Failed);
        assert_eq!(depois.state, IngestionState::Completed);
        assert!(!depois.stored_path.is_empty());
        assert_eq!(
            storage.get_resource(resource.id).unwrap().lifecycle_state,
            LifecycleState::Active
        );
        // E a busca pelo NOME continua funcionando: o arquivo nao sumiu.
        assert_eq!(
            ResourceRepository::search_resources(
                &storage,
                SearchRequest {
                    query: "escaneado".into(),
                    include_archived: false,
                    limit: 10,
                },
            )
            .unwrap()
            .len(),
            1
        );
    }

    /// Um segundo `complete` nao pode criar um segundo Resource.
    #[test]
    fn completar_duas_vezes_e_recusado() {
        let (_directory, storage) = storage();
        let ingestion = begin(&storage, "memorial.pdf", DropContext::default());
        let ingestion = preserve(&storage, ingestion.id, &"3".repeat(64));
        storage
            .complete_ingestion(
                ingestion.id,
                file_resource("memorial.pdf", ingestion.capture_id),
                &plan(),
            )
            .unwrap();

        let erro = storage
            .complete_ingestion(ingestion.id, file_resource("memorial.pdf", None), &plan())
            .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidTransition);
        assert_eq!(ResourceRepository::resources(&storage, true).unwrap().len(), 1);
    }

    /// O que a abertura precisa encontrar para reconciliar.
    #[test]
    fn a_abertura_ve_o_que_ficou_pela_metade() {
        let (_directory, storage) = storage();
        let recebendo = begin(&storage, "interrompido.pdf", DropContext::default());
        let completo = begin(&storage, "inteiro.pdf", DropContext::default());
        let completo = preserve(&storage, completo.id, &"4".repeat(64));
        let (completo, _) = storage
            .complete_ingestion(
                completo.id,
                file_resource("inteiro.pdf", completo.capture_id),
                &plan(),
            )
            .unwrap();

        let pendentes = storage.unfinished_ingestions().unwrap();
        assert_eq!(pendentes.len(), 1);
        assert_eq!(pendentes[0].id, recebendo.id);

        let extracoes = storage.pending_extractions().unwrap();
        assert_eq!(extracoes.len(), 1);
        assert_eq!(extracoes[0].id, completo.id);
    }

    /// A duplicata so vale para conteudo que ainda existe.
    #[test]
    fn arquivo_no_lixo_nao_conta_como_duplicata() {
        let (_directory, storage) = storage();
        let hash = "5".repeat(64);
        let primeira = begin(&storage, "memorial.pdf", DropContext::default());
        let primeira = preserve(&storage, primeira.id, &hash);
        let (_, resource) = storage
            .complete_ingestion(
                primeira.id,
                file_resource("memorial.pdf", primeira.capture_id),
                &plan(),
            )
            .unwrap();
        storage
            .set_resource_lifecycle(resource.id, LifecycleState::Trashed)
            .unwrap();

        let segunda = begin(&storage, "memorial.pdf", DropContext::default());
        assert_eq!(storage.duplicate_of(&hash, segunda.id).unwrap(), None);
    }
}

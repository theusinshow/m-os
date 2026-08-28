use mos_core::{
    CoreError, ErrorCode, LifecycleState, NewResource, ProjectId, Resource, ResourceId,
    ResourceKind, ResourceProject, ResourceRepository, ResourceWorkspace, SearchRequest,
    WorkspaceId,
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, guard_deletable, parse_time, to_fts_query},
    SqliteStorage,
};

pub(crate) const RESOURCE_COLUMNS: &str =
    "id, kind, title, url, note, source_capture_id, lifecycle_state, created_at, updated_at";

struct RawResource {
    id: String,
    kind: String,
    title: String,
    url: String,
    note: String,
    source_capture_id: Option<String>,
    lifecycle_state: String,
    created_at: String,
    updated_at: String,
}

impl RawResource {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            kind: row.get(1)?,
            title: row.get(2)?,
            url: row.get(3)?,
            note: row.get(4)?,
            source_capture_id: row.get(5)?,
            lifecycle_state: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }

    fn into_resource(self) -> Result<Resource, CoreError> {
        Ok(Resource {
            id: ResourceId::parse(&self.id)?,
            kind: ResourceKind::parse(&self.kind)?,
            title: self.title,
            url: self.url,
            note: self.note,
            source_capture_id: self
                .source_capture_id
                .as_deref()
                .map(mos_core::CaptureId::parse)
                .transpose()?,
            lifecycle_state: LifecycleState::parse(&self.lifecycle_state)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

impl ResourceRepository for SqliteStorage {
    fn create_resource(&self, resource: NewResource) -> Result<Resource, CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let id = insert_resource(self, &transaction, resource)?;
        transaction.commit().map_err(map_sql_error)?;
        query_resource(&connection, id)
    }
    fn update_resource(
        &self,
        id: ResourceId,
        kind: ResourceKind,
        title: &str,
        url: &str,
        note: &str,
    ) -> Result<Resource, CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        delete_resource_search(&transaction, id)?;
        let now = format_time(OffsetDateTime::now_utc())?;
        let changed = transaction
            .execute(
                "UPDATE resources
                 SET kind = ?6, title = ?1, url = ?2, note = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![title, url, note, now, id.to_string(), kind.as_str()],
            )
            .map_err(map_sql_error)?;
        ensure_resource_changed(changed)?;
        let rowid = transaction
            .query_row(
                "SELECT rowid FROM resources WHERE id = ?1",
                [id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(map_sql_error)?;
        insert_resource_search(&transaction, rowid)?;
        self.emitir_update(
            &transaction,
            "resource",
            id.as_uuid(),
            &[
                ("kind", serde_json::json!(kind.as_str())),
                ("title", serde_json::json!(title)),
                ("url", serde_json::json!(url)),
                ("note", serde_json::json!(note)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_resource(&connection, id)
    }

    fn get_resource(&self, id: ResourceId) -> Result<Resource, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_resource(&connection, id)
    }

    fn resources(&self, include_archived: bool) -> Result<Vec<Resource>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "lifecycle_state IN ('active', 'archived')"
        } else {
            "lifecycle_state = 'active'"
        };
        query_resources(
            &connection,
            &format!(
                "SELECT {RESOURCE_COLUMNS} FROM resources WHERE {lifecycle}
                 ORDER BY updated_at DESC"
            ),
        )
    }

    fn trashed_resources(&self) -> Result<Vec<Resource>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_resources(
            &connection,
            &format!(
                "SELECT {RESOURCE_COLUMNS} FROM resources
                 WHERE lifecycle_state = 'trashed'
                 ORDER BY updated_at DESC"
            ),
        )
    }

    fn set_resource_lifecycle(
        &self,
        id: ResourceId,
        lifecycle: LifecycleState,
    ) -> Result<Resource, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let (archived_at, deleted_at) = match lifecycle {
            LifecycleState::Active => (None, None),
            LifecycleState::Archived => (Some(now.as_str()), None),
            LifecycleState::Trashed => (None, Some(now.as_str())),
        };
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE resources
                 SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3, deleted_at = ?4
                 WHERE id = ?5",
                params![
                    lifecycle.as_str(),
                    now,
                    archived_at,
                    deleted_at,
                    id.to_string()
                ],
            )
            .map_err(map_sql_error)?;
        ensure_resource_changed(changed)?;
        self.emitir_update(
            &transaction,
            "resource",
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(lifecycle.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_resource(&connection, id)
    }

    fn search_resources(&self, request: SearchRequest) -> Result<Vec<Resource>, CoreError> {
        if request.query.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = to_fts_query(&request.query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let lifecycle = if request.include_archived {
            "r.lifecycle_state IN ('active', 'archived')"
        } else {
            "r.lifecycle_state = 'active'"
        };
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT r.{columns}
                 FROM resource_search s
                 JOIN resources r ON r.rowid = s.rowid
                 WHERE resource_search MATCH ?1 AND {lifecycle}
                 ORDER BY bm25(resource_search), r.updated_at DESC
                 LIMIT ?2",
                columns = RESOURCE_COLUMNS.replace(", ", ", r.")
            ))
            .map_err(map_sql_error)?;
        let mut resources = statement
            .query_map(
                params![fts_query, request.limit as i64],
                RawResource::from_row,
            )
            .map_err(map_sql_error)?
            .map(|row| row.map_err(map_sql_error)?.into_resource())
            .collect::<Result<Vec<_>, CoreError>>()?;

        // Segunda passada: o que casou DENTRO do arquivo.
        //
        // Duas consultas em vez de um UNION porque os dois indices tem escalas
        // de bm25 que nao se comparam, e porque a ordem certa e uma decisao de
        // produto e nao um artefato: quem acerta pelo nome ou pelo motivo vem
        // antes de quem acerta na pagina 143 de um memorial. Sem isso, digitar
        // "memorial" traria primeiro os PDFs que mencionam a palavra e por
        // ultimo o arquivo chamado memorial.pdf.
        let mut seen: std::collections::HashSet<String> = resources
            .iter()
            .map(|resource| resource.id.to_string())
            .collect();
        let remaining = request.limit.saturating_sub(resources.len());
        if remaining > 0 {
            let mut statement = connection
                .prepare(&format!(
                    "SELECT r.{columns}
                     FROM ingestion_search s
                     JOIN ingestions i ON i.rowid = s.rowid
                     JOIN resources r ON r.id = i.resource_id
                     WHERE ingestion_search MATCH ?1 AND {lifecycle}
                     ORDER BY bm25(ingestion_search), r.updated_at DESC
                     LIMIT ?2",
                    columns = RESOURCE_COLUMNS.replace(", ", ", r.")
                ))
                .map_err(map_sql_error)?;
            let inside = statement
                .query_map(params![fts_query, remaining as i64], RawResource::from_row)
                .map_err(map_sql_error)?
                .map(|row| row.map_err(map_sql_error)?.into_resource())
                .collect::<Result<Vec<_>, CoreError>>()?;
            for resource in inside {
                if seen.insert(resource.id.to_string()) {
                    resources.push(resource);
                }
            }
        }
        Ok(resources)
    }

    fn rebuild_resource_search(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO resource_search(resource_search) VALUES('rebuild')",
                [],
            )
            .map_err(map_sql_error)?;
        let count = transaction
            .query_row("SELECT count(*) FROM resource_search", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(map_sql_error)? as usize;
        transaction.commit().map_err(map_sql_error)?;
        Ok(count)
    }

    fn delete_resource(&self, id: ResourceId) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "resources", &id.to_string(), "Resource")?;
        delete_resource_search(&transaction, id)?;
        transaction
            .execute("DELETE FROM resources WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("resource", id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn set_resource_workspace(
        &self,
        resource_id: ResourceId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        if linked {
            let now = format_time(OffsetDateTime::now_utc())?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO resource_workspaces (resource_id, workspace_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![resource_id.to_string(), workspace_id.to_string(), now],
                )
                .map_err(map_sql_error)?;
        } else {
            transaction
                .execute(
                    "DELETE FROM resource_workspaces
                     WHERE resource_id = ?1 AND workspace_id = ?2",
                    params![resource_id.to_string(), workspace_id.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        self.emitir_relacao(
            &transaction,
            "resourceWorkspace",
            resource_id.as_uuid(),
            workspace_id.as_uuid(),
            linked,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    /// Todos os pares numa chamada. O filtro da Library responde no instante em
    /// que o contexto muda; uma consulta por Workspace faria cada troca de
    /// contexto ir ao core, e a troca deixaria de ser instantanea.
    fn resource_workspaces(&self) -> Result<Vec<ResourceWorkspace>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT resource_id, workspace_id FROM resource_workspaces
                 ORDER BY workspace_id, created_at DESC",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql_error)?;
        let mut links = Vec::new();
        for row in rows {
            let (resource_id, workspace_id) = row.map_err(map_sql_error)?;
            links.push(ResourceWorkspace {
                resource_id: ResourceId::parse(&resource_id)?,
                workspace_id: WorkspaceId::parse(&workspace_id)?,
            });
        }
        Ok(links)
    }

    fn set_resource_project(
        &self,
        resource_id: ResourceId,
        project_id: ProjectId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        {
            let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
            link_resource_project(self, &transaction, resource_id, project_id, linked)?;
            transaction.commit().map_err(map_sql_error)
        }
    }

    fn resource_projects(&self) -> Result<Vec<ResourceProject>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT resource_id, project_id FROM resource_projects
                 ORDER BY project_id, created_at DESC",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql_error)?;
        let mut links = Vec::new();
        for row in rows {
            let (resource_id, project_id) = row.map_err(map_sql_error)?;
            links.push(ResourceProject {
                resource_id: ResourceId::parse(&resource_id)?,
                project_id: ProjectId::parse(&project_id)?,
            });
        }
        Ok(links)
    }
}

/// Liga ou desliga o par, sem transacao propria.
///
/// Existe fora do trait para que o pipeline de ingestao possa aplicar a relacao
/// DENTRO da mesma transacao que cria o Resource: um Resource que nasce e uma
/// relacao que chega depois sao dois estados observaveis, e o de dentro seria
/// um Resource sem contexto piscando na Library.
pub(crate) fn link_resource_project(
    storage: &SqliteStorage,
    connection: &rusqlite::Connection,
    resource_id: ResourceId,
    project_id: ProjectId,
    linked: bool,
) -> Result<(), CoreError> {
    if linked {
        let now = format_time(OffsetDateTime::now_utc())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO resource_projects (resource_id, project_id, created_at)
                 VALUES (?1, ?2, ?3)",
                params![resource_id.to_string(), project_id.to_string(), now],
            )
            .map_err(map_sql_error)?;
    } else {
        connection
            .execute(
                "DELETE FROM resource_projects WHERE resource_id = ?1 AND project_id = ?2",
                params![resource_id.to_string(), project_id.to_string()],
            )
            .map_err(map_sql_error)?;
    }
    storage.emitir_relacao(
        connection,
        "resourceProject",
        resource_id.as_uuid(),
        project_id.as_uuid(),
        linked,
    )
}

/// Igual ao de cima, para o par Resource-Workspace.
pub(crate) fn link_resource_workspace(
    storage: &SqliteStorage,
    connection: &rusqlite::Connection,
    resource_id: ResourceId,
    workspace_id: WorkspaceId,
    linked: bool,
) -> Result<(), CoreError> {
    if linked {
        let now = format_time(OffsetDateTime::now_utc())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO resource_workspaces (resource_id, workspace_id, created_at)
                 VALUES (?1, ?2, ?3)",
                params![resource_id.to_string(), workspace_id.to_string(), now],
            )
            .map_err(map_sql_error)?;
    } else {
        connection
            .execute(
                "DELETE FROM resource_workspaces WHERE resource_id = ?1 AND workspace_id = ?2",
                params![resource_id.to_string(), workspace_id.to_string()],
            )
            .map_err(map_sql_error)?;
    }
    storage.emitir_relacao(
        connection,
        "resourceWorkspace",
        resource_id.as_uuid(),
        workspace_id.as_uuid(),
        linked,
    )
}

pub(crate) fn insert_resource_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO resource_search (rowid, title, url, note)
             SELECT rowid, title, url, note FROM resources WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_resource_search(transaction: &Transaction<'_>, id: ResourceId) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO resource_search(resource_search, rowid, title, url, note)
             SELECT 'delete', rowid, title, url, note FROM resources WHERE id = ?1",
            [id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

pub(crate) fn query_resource(
    connection: &rusqlite::Connection,
    id: ResourceId,
) -> Result<Resource, CoreError> {
    connection
        .query_row(
            &format!("SELECT {RESOURCE_COLUMNS} FROM resources WHERE id = ?1"),
            [id.to_string()],
            RawResource::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Resource nao encontrado.", false))?
        .into_resource()
}

pub(crate) fn query_resources(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<Vec<Resource>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let resources = statement
        .query_map([], RawResource::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_resource())
        .collect();
    resources
}

pub(crate) fn query_resources_all(
    connection: &rusqlite::Connection,
) -> Result<Vec<Resource>, CoreError> {
    query_resources(
        connection,
        &format!("SELECT {RESOURCE_COLUMNS} FROM resources ORDER BY created_at ASC"),
    )
}

fn ensure_resource_changed(changed: usize) -> Result<(), CoreError> {
    if changed == 0 {
        Err(CoreError::new(
            ErrorCode::NotFound,
            "Resource nao encontrado.",
            false,
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{CaptureRepository, CaptureSource, NewCapture, NewWorkspace, WorkRepository};

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (directory, storage)
    }

    #[test]
    fn create_update_search_and_archive_resource() {
        let (_directory, storage) = storage();
        let resource = storage
            .create_resource(
                NewResource::create_link("Motion", "https://motion.dev", "Hero animada", None)
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            storage
                .search_resources(SearchRequest {
                    query: "animada".into(),
                    include_archived: false,
                    limit: 20,
                })
                .unwrap()
                .len(),
            1
        );
        let updated = storage
            .update_resource(
                resource.id,
                ResourceKind::Library,
                "Motion docs",
                "https://motion.dev/docs",
                "Referencia",
            )
            .unwrap();
        assert_eq!(updated.url, "https://motion.dev/docs");
        storage
            .set_resource_lifecycle(resource.id, LifecycleState::Archived)
            .unwrap();
        assert!(storage.resources(false).unwrap().is_empty());
        assert_eq!(storage.resources(true).unwrap().len(), 1);
        storage
            .set_resource_lifecycle(resource.id, LifecycleState::Trashed)
            .unwrap();
        assert!(storage.resources(true).unwrap().is_empty());
        assert_eq!(storage.trashed_resources().unwrap().len(), 1);
    }

    #[test]
    fn capture_to_resource_is_atomic_and_unique() {
        let (_directory, storage) = storage();
        let capture = storage
            .create(NewCapture::create("https://motion.dev", CaptureSource::Home).unwrap())
            .unwrap();
        let resource = storage
            .create_resource(
                NewResource::create_link("", "https://motion.dev", "Motion", Some(capture.id))
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(resource.source_capture_id, Some(capture.id));
        assert_eq!(
            storage.get(capture.id).unwrap().processing_state,
            mos_core::ProcessingState::Processed
        );
        assert!(storage
            .create_resource(
                NewResource::create_link("Outra", "https://motion.dev/docs", "", Some(capture.id),)
                    .unwrap(),
            )
            .is_err());
        assert_eq!(storage.resources(false).unwrap().len(), 1);
    }

    fn workspace(storage: &SqliteStorage, name: &str) -> mos_core::Workspace {
        storage
            .create_workspace(NewWorkspace::create(name, "").unwrap())
            .unwrap()
    }

    fn site(storage: &SqliteStorage, title: &str) -> Resource {
        storage
            .create_resource(
                NewResource::create(ResourceKind::Site, title, "https://motion.dev", "", None)
                    .unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn resource_workspace_link_is_idempotent_and_isolated() {
        let (_directory, storage) = storage();
        let design = workspace(&storage, "Web Design");
        let finance = workspace(&storage, "Finance");
        let motion = site(&storage, "Motion");

        storage
            .set_resource_workspace(motion.id, design.id, true)
            .unwrap();
        storage
            .set_resource_workspace(motion.id, design.id, true)
            .unwrap();

        let links = storage.resource_workspaces().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].resource_id, motion.id);
        assert_eq!(links[0].workspace_id, design.id);

        // O mesmo Resource pode servir a dois contextos.
        storage
            .set_resource_workspace(motion.id, finance.id, true)
            .unwrap();
        assert_eq!(storage.resource_workspaces().unwrap().len(), 2);

        // Desvincular apaga so o par pedido, e repetir nao e erro.
        storage
            .set_resource_workspace(motion.id, finance.id, false)
            .unwrap();
        storage
            .set_resource_workspace(motion.id, finance.id, false)
            .unwrap();
        let links = storage.resource_workspaces().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].workspace_id, design.id);
    }

    /// As duas cascatas. Nao existe delete de Resource nem de Workspace no
    /// produto — arquivar e o caminho —, entao o DELETE cru prova que as FKs
    /// estao ativas: se `foreign_keys=ON` se perder em `configure_connection`
    /// (lib.rs:103), as linhas sobrevivem e este teste falha.
    #[test]
    fn deleting_either_side_takes_the_link() {
        let (_directory, storage) = storage();
        let design = workspace(&storage, "Web Design");
        let motion = site(&storage, "Motion");
        let easing = site(&storage, "Easings");
        storage
            .set_resource_workspace(motion.id, design.id, true)
            .unwrap();
        storage
            .set_resource_workspace(easing.id, design.id, true)
            .unwrap();

        storage
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM resources WHERE id = ?1",
                params![motion.id.to_string()],
            )
            .unwrap();
        assert_eq!(storage.resource_workspaces().unwrap().len(), 1);

        storage
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM workspaces WHERE id = ?1",
                params![design.id.to_string()],
            )
            .unwrap();
        assert!(storage.resource_workspaces().unwrap().is_empty());
    }
}

/// Insere o Resource e sua projecao de busca DENTRO de uma transacao ja aberta.
///
/// Existe separado do comando porque o pipeline de ingestao precisa criar o
/// Resource junto das relacoes e do fechamento da ingestao — tudo num commit so.
/// A regra de proveniencia (a Capture precisa estar ativa, e uma Capture deriva
/// no maximo um Resource) mora aqui, e nao no comando, para que os dois caminhos
/// nao possam divergir.
/// Insere o Resource e emite a operacao, na MESMA transacao.
///
/// Emitir aqui dentro, e nao no chamador, e o que garante que um caminho novo
/// de criacao nao nasca sem rastro: quem esquecer de emitir tera esquecido
/// tambem de inserir.
pub(crate) fn insert_resource(
    storage: &SqliteStorage,
    transaction: &Transaction<'_>,
    resource: NewResource,
) -> Result<ResourceId, CoreError> {
    if let Some(capture_id) = resource.source_capture_id {
        let lifecycle = transaction
            .query_row(
                "SELECT lifecycle_state FROM captures WHERE id = ?1",
                [capture_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .ok_or_else(|| {
                CoreError::new(ErrorCode::NotFound, "Capture nao encontrada.", false)
            })?;
        if lifecycle != "active" {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Somente uma Capture ativa pode originar Resource.",
                false,
            ));
        }
        let already_derived: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM resources WHERE source_capture_id = ?1)",
                [capture_id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        if already_derived {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Esta Capture ja originou um Resource.",
                false,
            ));
        }
    }

    let now = format_time(resource.created_at)?;
    let id = resource.id;
    transaction
        .execute(
            "INSERT INTO resources (
                id, kind, title, url, note, source_capture_id,
                lifecycle_state, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7)",
            params![
                resource.id.to_string(),
                resource.kind.as_str(),
                resource.title,
                resource.url,
                resource.note,
                resource.source_capture_id.map(|value| value.to_string()),
                now,
            ],
        )
        .map_err(map_sql_error)?;
    insert_resource_search(transaction, transaction.last_insert_rowid())?;
    storage.emitir(
        transaction,
        mos_sync::EntityRef::new("resource", id.as_uuid()),
        mos_sync::OpBody::Create {
            fields: [
                ("kind".to_owned(), serde_json::json!(resource.kind.as_str())),
                ("title".to_owned(), serde_json::json!(resource.title)),
                ("url".to_owned(), serde_json::json!(resource.url)),
                ("note".to_owned(), serde_json::json!(resource.note)),
                (
                    "sourceCaptureId".to_owned(),
                    serde_json::json!(resource.source_capture_id.map(|value| value.to_string())),
                ),
                ("createdAt".to_owned(), serde_json::json!(now)),
            ]
            .into_iter()
            .collect(),
        },
    )?;
    if let Some(capture_id) = resource.source_capture_id {
        transaction
            .execute(
                "UPDATE captures
                 SET processing_state = 'processed', updated_at = ?1
                 WHERE id = ?2",
                params![now, capture_id.to_string()],
            )
            .map_err(map_sql_error)?;
        // A Capture muda de estado junto, e a mudanca dela viaja como dela: sao
        // duas entidades, e do outro lado elas se reconciliam separadas.
        storage.emitir_update(
            transaction,
            "capture",
            capture_id.as_uuid(),
            &[("processingState", serde_json::json!("processed"))],
        )?;
    }
    Ok(id)
}

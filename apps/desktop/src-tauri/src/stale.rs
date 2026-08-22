//! O que esta parado ha tempo demais.
//!
//! Mesma divisao do `calendar.rs`: a regra vive em `mos_core::stale`, que e pura
//! e testada, e este arquivo so BUSCA e delega. Comando Tauri nao se testa, e as
//! decisoes que podem estar erradas — o limiar de cada coluna, o que conta como
//! atividade, a ordem — sao justamente as regras.

use mos_core::{CoreError, StaleView};
use tauri::{AppHandle, Manager, Runtime};
use time::OffsetDateTime;

use crate::AppState;

/// As paradas de agora, e a atividade real de cada Project.
///
/// Sem argumento e sem janela: obsolescencia e sempre "ate agora", e um
/// parametro de data so ofereceria uma pergunta que ninguem faz.
#[tauri::command]
pub fn stale_list<R: Runtime>(app: AppHandle<R>) -> Result<StaleView, CoreError> {
    let state = app.state::<AppState>();

    // `false` ja exclui arquivadas e lixeira nos dois repositorios. A funcao
    // pura filtra de novo por lifecycle, e as duas defesas sao de proposito: a
    // do core e a que tem teste.
    let projects = state.work.projects(false)?;
    let tasks = state.work.tasks(false)?;

    let name_of = |id: mos_core::ProjectId| {
        projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project removido".to_owned())
    };

    Ok(StaleView {
        paradas: mos_core::compose_stale(mos_core::StaleInput {
            now: OffsetDateTime::now_utc(),
            tasks: &tasks,
            projects: &projects,
            project_name: &name_of,
        }),
        activity: mos_core::project_activity(&projects, &tasks),
    })
}

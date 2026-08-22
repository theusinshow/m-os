//! O Calendario (fase 1).
//!
//! A composicao das quatro fontes vive em `mos_core::compose`, que e pura e
//! testada. Este arquivo so BUSCA e delega — e a divisao existe porque comando
//! Tauri nao se testa, e as decisoes que podem estar erradas (o que entra na
//! janela, o que vira dois itens, o que e ignorado) sao justamente as regras.
//!
//! A leitura das quatro fontes acontece aqui, e nao num servico do core, porque
//! nenhum servico existente tem os quatro repositorios e esta e a camada onde
//! eles se encontram. Mesmo lugar e mesmo motivo do `monitoring_timeline`.

use mos_core::{CalendarItem, CoreError};
use tauri::{AppHandle, Manager, Runtime};

use crate::AppState;

/// Tudo o que o M/OS registrou entre dois instantes, em ordem crescente.
///
/// A janela vem como INSTANTE e nao como data: quem decide onde um dia comeca e
/// o renderer, que conhece o fuso de quem esta olhando. Este comando so responde
/// "o que aconteceu entre X e Y".
#[tauri::command]
pub fn calendar_window<R: Runtime>(
    app: AppHandle<R>,
    since: String,
    until: String,
) -> Result<Vec<CalendarItem>, CoreError> {
    let state = app.state::<AppState>();
    let from = mos_core::parse_moment(&since)?;
    let to = mos_core::parse_moment(&until)?;
    if to < from {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "O fim da janela vem antes do inicio.",
            false,
        ));
    }

    // Cada leitura numa variavel propria: passar as chamadas direto como
    // referencia deixaria os temporarios morrerem antes de `compose` usa-los.
    let projects = state.work.projects(true)?;
    let entries = state.tracking.entries(None)?;
    let tasks = state.work.tasks(true)?;
    let captures = state.captures.between(from, to)?;
    let events = state.monitoring.events(from, to)?;
    let rounding = state.tracking.settings()?.rounding;
    // Um ano de sessoes e ~365 linhas curtas, e a janela do calendario nunca
    // passa de um mes — ler tudo e mais barato que uma consulta por faixa de
    // data sobre uma tabela deste tamanho, e a filtragem por janela ja e da
    // `compose`. Os objetivos vem numa consulta so, e nao uma por dia.
    let sessions = state.daily.sessions(365)?;
    let session_ids: Vec<_> = sessions.iter().map(|sessao| sessao.id).collect();
    let objectives = state.daily.objectives_of(&session_ids)?;

    // A faculdade entra pelo painel ja composto: quem decide se uma prova conta
    // e `academic::compose_dashboard`, e reescrever esse filtro aqui daria duas
    // respostas para a mesma pergunta. O painel ja vem no fuso da tela.
    let academico = mos_core::AcademicService::new(state.storage.clone()).compromissos_entre(
        from,
        to,
        crate::surface::now_local(&app),
    )?;

    let name_of = |id: mos_core::ProjectId| {
        projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project removido".to_owned())
    };

    Ok(mos_core::compose(mos_core::ComposeInput {
        since: from,
        until: to,
        rounding,
        entries: &entries,
        tasks: &tasks,
        captures: &captures,
        events: &events,
        sessions: &sessions,
        objectives: &objectives,
        academic: &academico,
        project_name: &name_of,
    }))
}

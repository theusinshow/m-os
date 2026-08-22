//! O que a tela esta mostrando, e em que fuso.
//!
//! # Por que isto virou um modulo, e nao mais um campo da voz
//!
//! O `VoiceRuntime` ja guardava duas coisas que nao sao da voz: o Project e a
//! Task abertos, e o offset de quem esta na frente do computador. Elas moravam
//! la por um motivo bom — o atalho global dispara do lado do Rust, e naquele
//! caminho nao ha chamada do renderer para levar contexto junto — mas o motivo
//! nunca foi exclusivo da voz.
//!
//! O Hermes precisa das mesmas duas. "Me lembra disso sexta às 15h" tem dois
//! buracos: **"disso"**, que so a tela sabe preencher, e **"sexta às 15h"**,
//! que so o fuso resolve. Copiar os campos para um segundo runtime daria dois
//! lugares para a tela publicar e um dia para eles divergirem — a Task aberta
//! numa das copias e nao na outra.
//!
//! Entao a fonte passa a ser uma so. A voz continua lendo o contexto e o fuso
//! pelas mesmas funcoes internas de antes — `voice_context` e `now_local` —, e
//! quem publica passou a ser a tela inteira, por `surface_set_context`, em vez
//! de so o par Project+Task que `voice_set_context` levava.

use std::sync::Mutex;

use mos_core::{CoreError, ErrorCode, Here, Named, ProjectId, TaskId};
use serde::Deserialize;
use tauri::{AppHandle, Manager, Runtime};

/// O contexto ambiente do M/OS.
#[derive(Default)]
pub struct SurfaceRuntime {
    /// Nome legivel da tela: "Kanban", "Inbox", "Tempo".
    screen: Mutex<String>,
    project: Mutex<Option<Named>>,
    task: Mutex<Option<Named>>,
    workspace: Mutex<Option<Named>>,
    /// O fuso de quem esta na frente do computador, em minutos.
    ///
    /// `CORE-FOUNDATION.md` §5 e o `ReminderComposer` sao explicitos: quem
    /// conhece o fuso e a tela, e o banco guarda UTC. O renderer publica isto
    /// na montagem, e "amanha as nove" e resolvido contra ele.
    /// `None` enquanto o renderer nao publicou.
    ///
    /// A distincao importa: zero e um fuso legitimo (Londres no inverno), e
    /// trata-lo como "ainda nao sei" seria errado — mas o contrario e pior. Um
    /// sync que roda antes da tela montar leria offset zero, interpretaria
    /// "vence 23h59" como UTC e gravaria um prazo tres horas adiantado. O
    /// prazo apareceria como 20h59, e ninguem saberia por que.
    offset_minutes: Mutex<Option<i32>>,
}

/// O que o renderer publica quando a tela muda.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceInput {
    #[serde(default)]
    pub screen: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub project_label: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub task_label: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub workspace_label: Option<String>,
}

fn runtime<R: Runtime>(app: &AppHandle<R>) -> Result<tauri::State<'_, SurfaceRuntime>, CoreError> {
    app.try_state::<SurfaceRuntime>().ok_or_else(|| {
        CoreError::new(
            ErrorCode::StorageUnavailable,
            "O M/OS ainda esta abrindo.",
            true,
        )
    })
}

fn read<T: Clone>(slot: &Mutex<T>) -> T {
    slot.lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|error| error.into_inner().clone())
}

fn write<T>(slot: &Mutex<T>, value: T) {
    match slot.lock() {
        Ok(mut guard) => *guard = value,
        Err(error) => *error.into_inner() = value,
    }
}

/// Um par id+rotulo, quando os dois vieram.
///
/// Id sem rotulo continua servindo — a resolucao usa o id —, e por isso o
/// rotulo cai para o proprio id em vez de descartar a entidade inteira.
fn named(id: Option<String>, label: Option<String>) -> Option<Named> {
    let id = id.filter(|value| !value.trim().is_empty())?;
    let label = label
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| id.clone());
    Some(Named::new(id, label))
}

/// O que a tela esta mostrando agora.
pub fn here<R: Runtime>(app: &AppHandle<R>) -> Here {
    let Some(state) = app.try_state::<SurfaceRuntime>() else {
        return Here::default();
    };
    Here {
        screen: read(&state.screen),
        project: read(&state.project),
        task: read(&state.task),
        workspace: read(&state.workspace),
    }
}

/// O renderer ja publicou o fuso?
///
/// Quem grava instante derivado de data sem fuso — o sync academico — precisa
/// esperar por isto. Ver o comentario de `offset_minutes`.
pub fn offset_publicado<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<SurfaceRuntime>()
        .map(|state| read(&state.offset_minutes).is_some())
        .unwrap_or(false)
}

/// O agora de quem esta na frente do computador, no fuso dele.
pub fn now_local<R: Runtime>(app: &AppHandle<R>) -> time::OffsetDateTime {
    let minutes = app
        .try_state::<SurfaceRuntime>()
        .map(|state| read(&state.offset_minutes).unwrap_or(0))
        .unwrap_or(0);
    let offset = time::UtcOffset::from_whole_seconds(minutes * 60).unwrap_or(time::UtcOffset::UTC);
    time::OffsetDateTime::now_utc().to_offset(offset)
}

/// O Project e a Task abertos, ja tipados, como a voz os quer.
pub fn voice_context<R: Runtime>(app: &AppHandle<R>) -> mos_core::VoiceContext {
    let here = here(app);
    mos_core::VoiceContext {
        project_id: here
            .project
            .as_ref()
            .and_then(|named| ProjectId::parse(&named.id).ok()),
        task_id: here
            .task
            .as_ref()
            .and_then(|named| TaskId::parse(&named.id).ok()),
    }
}

/// A tela publica o que esta olhando.
#[tauri::command]
pub fn surface_set_context<R: Runtime>(
    app: AppHandle<R>,
    input: SurfaceInput,
) -> Result<(), CoreError> {
    let state = runtime(&app)?;
    write(&state.screen, input.screen.trim().to_owned());
    write(&state.project, named(input.project_id, input.project_label));
    write(&state.task, named(input.task_id, input.task_label));
    write(
        &state.workspace,
        named(input.workspace_id, input.workspace_label),
    );
    Ok(())
}

/// O fuso, publicado na montagem.
#[tauri::command]
pub fn surface_set_locale<R: Runtime>(
    app: AppHandle<R>,
    offset_minutes: i32,
) -> Result<(), CoreError> {
    // Uma hora e meia de fuso existe (India, +5:30); trinta horas nao. O teto
    // recusa um valor absurdo em vez de deixa-lo virar um lembrete no dia
    // errado.
    if !(-14 * 60..=14 * 60).contains(&offset_minutes) {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Fuso horario fora do intervalo possivel.",
            false,
        ));
    }
    write(&runtime(&app)?.offset_minutes, Some(offset_minutes));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Id sem rotulo continua servindo: a resolucao usa o id, e descartar a
    /// entidade inteira por falta de nome perderia justamente o que identifica.
    #[test]
    fn an_id_without_a_label_falls_back_to_itself() {
        let entidade = named(Some("t1".into()), None).expect("id basta");
        assert_eq!(entidade.label, "t1");
    }

    #[test]
    fn an_empty_id_is_no_entity() {
        assert!(named(Some("   ".into()), Some("Minarum".into())).is_none());
        assert!(named(None, Some("Minarum".into())).is_none());
    }

    #[test]
    fn a_label_that_came_is_kept() {
        let entidade = named(Some("t1".into()), Some("Enviar bases".into())).unwrap();
        assert_eq!(entidade.label, "Enviar bases");
    }
}

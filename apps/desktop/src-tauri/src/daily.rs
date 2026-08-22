//! Os comandos da Daily Session, e o unico lugar que monta o contexto do dia.
//!
//! # Onde as coisas moram
//!
//! A REGRA vive em `mos_core::daily` (o que e um dia, o que e progresso, o que
//! vira carry-over) e em `mos_core::DailyService` (o que acontece ao comecar,
//! acrescentar, concluir e encerrar). Aqui so ficam duas coisas que o core nao
//! pode ter:
//!
//! 1. **Que dia e hoje.** Depende do fuso de quem esta na frente da tela, que o
//!    renderer publica em `surface.rs`. O core recebe a data pronta — mesma lei
//!    que o `voice_when` obedece.
//! 2. **Ler as outras entidades.** O contexto do dia soma Reminders, Tasks,
//!    Projects, Captures e Meetings. Um servico de dominio que dependesse dos
//!    cinco repositorios so para desenhar uma tela seria um servico que nao da
//!    para instanciar sem o sistema inteiro. Quem le e este arquivo, e quem
//!    decide o que os numeros significam e a funcao pura
//!    `mos_core::compose_context` — o mesmo desenho do `calendar.rs`.
//!
//! `contexto` e `iniciar` sao `pub` e nao `#[tauri::command]`-only de proposito:
//! o Hermes chama as MESMAS funcoes que a interface chama (§30 do pedido). Nao
//! ha um segundo caminho que pudesse divergir.

use mos_core::{
    CoreError, DailyContext, DailyObjectiveId, DailySessionId, DailySessionSummary, DailyToday,
    Day, EndDayInput, ErrorCode, LinkKind, ObjectiveDraft, ObjectiveLink, ObjectivePriority,
    ObjectiveStatus, StartDayInput,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Quantos dias a tela de historico traz de uma vez.
///
/// Trinta e o que cabe numa lista que se le rolando uma vez. O servico ja
/// limita a 365; este e o teto da TELA, e sao numeros diferentes de proposito —
/// o Hermes pode querer mais que a lista mostra.
const HISTORY_PAGE: usize = 30;

/// Hoje, no fuso de quem esta na frente da tela.
///
/// **Nunca UTC.** Quem trabalha ate 23h30 em UTC-3 esta no dia 21; em UTC ja e
/// dia 22, e o mesmo dia de trabalho viraria duas sessoes.
pub fn hoje<R: Runtime>(app: &AppHandle<R>) -> Day {
    Day::from_local(crate::surface::now_local(app))
}

fn avisar<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit("data-changed", "daily");
}

/// O rotulo de uma entidade vinculada.
///
/// Existe porque um objetivo pode nascer sem titulo quando ele E uma Task ou um
/// Project: a pessoa escolheu a Task no seletor, e digitar o titulo de novo
/// seria trabalho que o sistema ja tem como fazer. Falha vira `None`, e ai o
/// dominio recusa o objetivo sem titulo — melhor que gravar "(sem titulo)".
fn rotulo_do_vinculo<R: Runtime>(app: &AppHandle<R>, link: &ObjectiveLink) -> Option<String> {
    let state = app.state::<AppState>();
    match link.kind {
        LinkKind::Task => state.work.task(&link.id).ok().map(|task| task.title),
        LinkKind::Project => state.work.project(&link.id).ok().map(|project| project.name),
        LinkKind::Capture => state.captures.get(&link.id).ok().map(|capture| capture.content),
        LinkKind::Resource => state.memory.resource(&link.id).ok().map(|resource| resource.title),
        LinkKind::Meeting => state.meetings.meeting(&link.id).ok().map(|meeting| meeting.title),
    }
}

/// Completa o titulo de um rascunho a partir do vinculo, quando ele veio vazio.
fn completar<R: Runtime>(app: &AppHandle<R>, draft: &ObjectiveDraft) -> Result<ObjectiveDraft, CoreError> {
    if !draft.title.trim().is_empty() {
        return Ok(draft.clone());
    }
    let Some(link) = draft.link()? else {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "O objetivo precisa de um titulo.",
            false,
        ));
    };
    let Some(titulo) = rotulo_do_vinculo(app, &link) else {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Nao achei o que este objetivo aponta. Escreva um titulo.",
            false,
        ));
    };
    Ok(ObjectiveDraft {
        title: titulo,
        ..draft.clone()
    })
}

// ------------------------------------------------------------------ leitura

#[tauri::command]
pub fn daily_today<R: Runtime>(app: AppHandle<R>) -> Result<DailyToday, CoreError> {
    crate::services(&app)?.daily.today(&hoje(&app))
}

/// O contexto do dia, montado a partir do que o M/OS ja guarda.
///
/// Uma leitura por entidade, e nao uma por numero mostrado: os cinco `Vec` sao
/// lidos uma vez e a funcao pura conta tudo em cima deles. E o §39 do pedido —
/// a Home nao pode ficar lenta por causa desta tela.
pub fn contexto<R: Runtime>(app: &AppHandle<R>) -> Result<DailyContext, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(|| {
        CoreError::new(ErrorCode::StorageUnavailable, "O M/OS ainda esta abrindo.", true)
    })?;
    let day = hoje(app);

    let reminders = state.attention.open()?;
    let tasks = state.work.tasks(false)?;
    let projects = state.work.projects(false)?;
    let captures = state.captures.recent(200)?;
    let meetings = state.meetings.meetings(false)?;
    let anterior = state.daily.previous(&day)?;

    // A faculdade entra no dia pela MESMA funcao que o painel do Academic usa.
    // Uma segunda nocao de "hoje" aqui faria o Start My Day sugerir entrega que
    // a tela do Academic ja considera atrasada.
    //
    // Falhar aqui NAO impede o dia de comecar: o M/Academic e uma camada por
    // cima, e um dia que se recusa a abrir porque a faculdade nao carregou
    // seria pior que um dia sem faculdade.
    let academico = mos_core::AcademicService::new(state.storage.clone())
        .today(crate::surface::now_local(app))
        .ok();

    let profundidade = |id: DailyObjectiveId| state.daily.carry_depth(id);
    Ok(mos_core::compose_context(mos_core::ContextInput {
        now_local: crate::surface::now_local(app),
        academic: academico.as_ref(),
        reminders: &reminders,
        tasks: &tasks,
        projects: &projects,
        captures: &captures,
        meetings: &meetings,
        previous: anterior
            .as_ref()
            .map(|(session, objectives)| (session, objectives.as_slice())),
        carry_depth: &profundidade,
    }))
}

#[tauri::command]
pub fn daily_context<R: Runtime>(app: AppHandle<R>) -> Result<DailyContext, CoreError> {
    contexto(&app)
}

#[tauri::command]
pub fn daily_history<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<DailySessionSummary>, CoreError> {
    crate::services(&app)?.daily.history(HISTORY_PAGE)
}

#[tauri::command]
pub fn daily_session<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<DailyToday, CoreError> {
    crate::services(&app)?
        .daily
        .detail(DailySessionId::parse(&id)?)
}

// ------------------------------------------------------------------ escrita

/// Comeca o dia. Chamado pela interface E pelo Hermes.
pub fn iniciar<R: Runtime>(
    app: &AppHandle<R>,
    input: &StartDayInput,
) -> Result<DailyToday, CoreError> {
    let resolvido = StartDayInput {
        main: input
            .main
            .as_ref()
            .map(|draft| completar(app, draft))
            .transpose()?,
        secondaries: input
            .secondaries
            .iter()
            .map(|draft| completar(app, draft))
            .collect::<Result<Vec<_>, _>>()?,
        note: input.note.clone(),
    };
    let day = hoje(app);
    let hoje_resolvido = app.state::<AppState>().daily.start(day, &resolvido)?;
    // O carry-over so vira `carried_over` quando o objetivo novo REALMENTE
    // nasce. Marcar antes deixaria o de ontem carregado e o de hoje inexistente
    // se a criacao falhasse — e ninguem saberia onde o objetivo foi parar.
    marcar_carregados(app, &resolvido)?;
    avisar(app);
    Ok(hoje_resolvido)
}

/// Marca como `carried_over` os objetivos de ontem que viraram objetivo de hoje.
///
/// Falha aqui NAO derruba o inicio do dia: o dia de hoje ja existe e esta certo,
/// e o pior que acontece e o de ontem continuar pendente — o que faz ele
/// reaparecer no carry-over de amanha. Perder o dia inteiro por causa de um
/// carimbo em um registro passado seria a troca errada.
fn marcar_carregados<R: Runtime>(
    app: &AppHandle<R>,
    input: &StartDayInput,
) -> Result<(), CoreError> {
    let state = app.state::<AppState>();
    let origens = input
        .main
        .iter()
        .chain(input.secondaries.iter())
        .filter_map(|draft| DailyObjectiveId::parse(&draft.carried_from).ok());
    for origem in origens {
        let _ = state
            .daily
            .set_objective_status(origem, ObjectiveStatus::CarriedOver);
    }
    Ok(())
}

#[tauri::command]
pub fn daily_start<R: Runtime>(
    app: AppHandle<R>,
    input: StartDayInput,
) -> Result<DailyToday, CoreError> {
    iniciar(&app, &input)
}

#[tauri::command]
pub fn daily_add_objective<R: Runtime>(
    app: AppHandle<R>,
    draft: ObjectiveDraft,
    priority: String,
) -> Result<DailyToday, CoreError> {
    let priority = ObjectivePriority::parse(&priority)?;
    let draft = completar(&app, &draft)?;
    let hoje_resolvido = app
        .state::<AppState>()
        .daily
        .add_objective(&hoje(&app), &draft, priority)?;
    if let Ok(origem) = DailyObjectiveId::parse(&draft.carried_from) {
        let _ = app
            .state::<AppState>()
            .daily
            .set_objective_status(origem, ObjectiveStatus::CarriedOver);
    }
    avisar(&app);
    Ok(hoje_resolvido)
}

#[tauri::command]
pub fn daily_update_objective<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    title: String,
    description: String,
) -> Result<DailyToday, CoreError> {
    app.state::<AppState>().daily.update_objective(
        DailyObjectiveId::parse(&id)?,
        &title,
        &description,
    )?;
    avisar(&app);
    daily_today(app)
}

#[tauri::command]
pub fn daily_set_objective_status<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    status: String,
) -> Result<DailyToday, CoreError> {
    app.state::<AppState>()
        .daily
        .set_objective_status(DailyObjectiveId::parse(&id)?, ObjectiveStatus::parse(&status)?)?;
    avisar(&app);
    daily_today(app)
}

#[tauri::command]
pub fn daily_set_main<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<DailyToday, CoreError> {
    app.state::<AppState>()
        .daily
        .set_main(DailyObjectiveId::parse(&id)?)?;
    avisar(&app);
    daily_today(app)
}

#[tauri::command]
pub fn daily_remove_objective<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<DailyToday, CoreError> {
    app.state::<AppState>()
        .daily
        .remove_objective(DailyObjectiveId::parse(&id)?)?;
    avisar(&app);
    daily_today(app)
}

#[tauri::command]
pub fn daily_reorder<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
    order: Vec<String>,
) -> Result<DailyToday, CoreError> {
    let ids = order
        .iter()
        .map(|id| DailyObjectiveId::parse(id))
        .collect::<Result<Vec<_>, _>>()?;
    app.state::<AppState>()
        .daily
        .reorder(DailySessionId::parse(&session_id)?, &ids)?;
    avisar(&app);
    daily_today(app)
}

/// Encerra o dia. Chamado pela interface E pelo Hermes.
pub fn encerrar<R: Runtime>(
    app: &AppHandle<R>,
    session: Option<DailySessionId>,
    input: &EndDayInput,
) -> Result<DailyToday, CoreError> {
    let state = app.state::<AppState>();
    match session {
        // Encerrar UMA sessao pelo id e o caminho do "encerrar ontem": ela e de
        // outra data, por definicao, e nao da para chegar nela por `day`.
        Some(id) => {
            state.daily.end_session(id, input)?;
        }
        None => {
            state.daily.end(&hoje(app), input)?;
        }
    }
    avisar(app);
    state.daily.today(&hoje(app))
}

#[tauri::command]
pub fn daily_end<R: Runtime>(
    app: AppHandle<R>,
    session_id: Option<String>,
    input: EndDayInput,
) -> Result<DailyToday, CoreError> {
    let session = session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| DailySessionId::parse(&id))
        .transpose()?;
    encerrar(&app, session, &input)
}

#[tauri::command]
pub fn daily_reopen<R: Runtime>(
    app: AppHandle<R>,
    session_id: String,
) -> Result<DailyToday, CoreError> {
    app.state::<AppState>()
        .daily
        .reopen(DailySessionId::parse(&session_id)?)?;
    avisar(&app);
    daily_today(app)
}

// ------------------------------------------------------------------- Hermes

/// Os objetivos de hoje, no formato que o preambulo do Hermes espera.
///
/// Falha vira dia vazio, e nao erro: o Hermes tem de continuar respondendo
/// mesmo que a Daily Session esteja indisponivel. Um chat que para de funcionar
/// porque um bloco opcional do prompt nao pode ser montado troca a feature
/// inteira por um detalhe dela.
///
/// So a sessao ABERTA. Um dia encerrado nao e contexto do que se esta fazendo
/// agora — e historia, e historia se consulta pela busca.
pub fn bloco_de_hoje<R: Runtime>(app: &AppHandle<R>) -> (String, Vec<(String, String, bool)>) {
    let vazio = || (String::new(), Vec::new());
    let Some(state) = app.try_state::<AppState>() else {
        return vazio();
    };
    let day = hoje(app);
    let Ok(hoje_resolvido) = state.daily.today(&day) else {
        return vazio();
    };
    if !hoje_resolvido.is_open() {
        return vazio();
    }
    let objetivos = hoje_resolvido
        .objectives
        .iter()
        .filter(|objetivo| objetivo.status != ObjectiveStatus::Dropped)
        .map(|objetivo| {
            (
                objetivo.title.clone(),
                match objetivo.priority {
                    ObjectivePriority::Main => "principal".to_owned(),
                    ObjectivePriority::Secondary => "secundário".to_owned(),
                },
                objetivo.status == ObjectiveStatus::Completed,
            )
        })
        .collect();
    (day.to_string(), objetivos)
}

/// Resolve a referencia a um objetivo do dia, como o Hermes a escreveu.
///
/// Procura **so entre os objetivos de hoje**, e nao no historico inteiro. "Meu
/// segundo objetivo" e sobre hoje por definicao, e alcancar um objetivo de tres
/// semanas atras por semelhanca de titulo seria agir sobre o dia errado — sem
/// que nada na tela mostrasse isso.
pub fn resolver_objetivo<R: Runtime>(
    app: &AppHandle<R>,
    referencia: &str,
) -> Result<mos_core::DailyObjective, CoreError> {
    let hoje_resolvido = app.state::<AppState>().daily.today(&hoje(app))?;
    if hoje_resolvido.objectives.is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "O dia ainda nao comecou, entao nao ha objetivo para mudar.",
            false,
        ));
    }
    let achado = mos_core::resolve(
        &hoje_resolvido.objectives,
        referencia,
        |objetivo| objetivo.id.to_string(),
        |objetivo| objetivo.title.clone(),
    );
    match mos_core::resolution_error(
        &achado,
        mos_core::EntityKind::DailyObjective,
        referencia,
        |objetivo: &mos_core::DailyObjective| objetivo.title.clone(),
    ) {
        Some(erro) => Err(erro),
        None => Ok(achado.one().expect("sem erro ha exatamente um").clone()),
    }
}

// ------------------------------------------------------------------- semana

/// Como achar o Project de um vinculo de objetivo.
///
/// Vive aqui, e nao no servico, porque so este lado conhece Tasks e Projects.
/// Um objetivo ligado a uma Task resolve pelo Project DA TASK — e e essa
/// agregacao que faz "o que dominou" dizer algo: tres Tasks diferentes do mesmo
/// Project sao uma semana daquele Project, e nao tres assuntos.
///
/// As duas listas sao lidas UMA vez e capturadas. Resolver por consulta a cada
/// objetivo faria uma semana de vinte objetivos custar quarenta idas ao banco
/// para desenhar cinco linhas.
fn resolvedor_de_project<R: Runtime>(
    app: &AppHandle<R>,
) -> impl Fn(&ObjectiveLink) -> Option<String> {
    let (tasks, projects) = match app.try_state::<AppState>() {
        Some(state) => (
            state.work.tasks(true).unwrap_or_default(),
            state.work.projects(true).unwrap_or_default(),
        ),
        None => (Vec::new(), Vec::new()),
    };

    move |link: &ObjectiveLink| {
        let project_id = match link.kind {
            LinkKind::Project => mos_core::ProjectId::parse(&link.id).ok(),
            LinkKind::Task => tasks
                .iter()
                .find(|task| task.id.to_string() == link.id)
                .and_then(|task| task.project_id),
            // Capture, Resource e Meeting nao levam a Project por um caminho
            // que valha uma agregacao semanal.
            _ => None,
        }?;
        projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.name.clone())
    }
}

/// A semana pedida, ou a corrente quando nenhuma vem.
#[tauri::command]
pub fn weekly_week<R: Runtime>(
    app: AppHandle<R>,
    week: Option<String>,
) -> Result<mos_core::WeekSummary, CoreError> {
    let alvo = match week.as_deref().map(str::trim).filter(|valor| !valor.is_empty()) {
        Some(valor) => mos_core::Week::parse(valor)?,
        None => mos_core::Week::containing(&hoje(&app))?,
    };
    let project_of = resolvedor_de_project(&app);
    crate::services(&app)?.daily.week(&alvo, &project_of)
}

/// A semana que acabou e nao foi fechada, se houver.
#[tauri::command]
pub fn weekly_pending<R: Runtime>(app: AppHandle<R>) -> Result<Option<mos_core::Week>, CoreError> {
    let corrente = mos_core::Week::containing(&hoje(&app))?;
    crate::services(&app)?.daily.pending_week(&corrente)
}

#[tauri::command]
pub fn weekly_close<R: Runtime>(
    app: AppHandle<R>,
    week: String,
    summary: String,
) -> Result<mos_core::WeekSummary, CoreError> {
    let alvo = mos_core::Week::parse(&week)?;
    crate::services(&app)?.daily.close_week(&alvo, &summary)?;
    avisar(&app);
    let project_of = resolvedor_de_project(&app);
    crate::services(&app)?.daily.week(&alvo, &project_of)
}

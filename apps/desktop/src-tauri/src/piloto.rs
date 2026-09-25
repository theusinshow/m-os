//! O piloto, visto do app: monta o retrato, chama os motores e entrega.
//!
//! Nada aqui decide. As regras moram em `mos_core::piloto`, com teste; este
//! arquivo le os repositorios, monta o [`Retrato`] e devolve o que o motor
//! respondeu — e, no laco de fundo, transforma as decisoes do Notification
//! Engine em toast do Windows.
//!
//! # Uma leitura, um panorama
//!
//! A Home inteira sai de `piloto_panorama`, numa chamada. Antes eram oito
//! comandos e oito leituras para desenhar cinco widgets, e nenhum deles
//! concordava sobre o que era "hoje".

use std::sync::Mutex;
use std::time::Duration;

use mos_core::{
    AcaoDeResgate, CoreError, ErrorCode, Habitos, Panorama, PlanoDeResgate, PropostaDeEncerramento,
    Retrato, SyncNoRetrato, Task,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// O que o piloto guarda em memoria entre chamadas.
pub struct PilotoRuntime {
    /// A presenca anterior a ESTA abertura, lida uma vez no boot. E a
    /// referencia do resgate — depois de gravada a de hoje, ela so existe aqui.
    pub presenca_anterior: Mutex<Option<time::OffsetDateTime>>,
    /// Acorda o laco de avisos: mutacao, primeiro plano.
    pub acordar: tokio::sync::Notify,
}

impl Default for PilotoRuntime {
    fn default() -> Self {
        Self {
            presenca_anterior: Mutex::new(None),
            acordar: tokio::sync::Notify::new(),
        }
    }
}

/// De quanto em quanto o laco olha o mundo sem ninguem pedir.
///
/// Cinco minutos: um compromisso avisa "em 15 minutos" com folga de um tick, e
/// cinco leituras por hora de tabelas pequenas nao e polling pesado. O resto
/// dos gatilhos e por evento.
const PULSO: Duration = Duration::from_secs(5 * 60);
/// Quanto esperar depois de uma mutacao antes de reavaliar avisos.
const DEBOUNCE: Duration = Duration::from_secs(20);
/// Quanto historico de avisos entra na politica.
const JANELA_DO_HISTORICO: time::Duration = time::Duration::days(2);
/// Quantas sessoes entram na mediana do inicio habitual.
const SESSOES_PARA_HABITO: usize = 14;

fn erro_abrindo() -> CoreError {
    CoreError::new(
        ErrorCode::StorageUnavailable,
        "O M/OS ainda esta abrindo.",
        true,
    )
}

/// Tudo que o retrato precisa, lido uma vez e guardado junto para os
/// emprestimos do `Retrato` apontarem para algo vivo.
pub struct Leitura {
    pub now_local: time::OffsetDateTime,
    pub tasks: Vec<Task>,
    pub projects: Vec<mos_core::Project>,
    pub reminders: Vec<mos_core::Reminder>,
    pub inbox: Vec<mos_core::Capture>,
    pub academic: Vec<mos_core::Compromisso>,
    pub agenda: Vec<mos_core::CalendarItem>,
    pub daily: Option<mos_core::DailyToday>,
    pub previous_objectives: Vec<mos_core::DailyObjective>,
    pub sync: SyncNoRetrato,
    pub ultima_presenca: Option<time::OffsetDateTime>,
    pub habitos: Habitos,
    pub reunioes: Vec<mos_core::ReuniaoNoRetrato>,
    pub dispensados: Vec<mos_core::Dispensa>,
}

impl Leitura {
    pub fn retrato(&self) -> Retrato<'_> {
        Retrato {
            now_local: self.now_local,
            tasks: &self.tasks,
            projects: &self.projects,
            reminders: &self.reminders,
            inbox: &self.inbox,
            academic: &self.academic,
            agenda: &self.agenda,
            daily: self.daily.as_ref(),
            previous_objectives: &self.previous_objectives,
            sync: &self.sync,
            ultima_presenca: self.ultima_presenca,
            habitos: &self.habitos,
            reunioes: &self.reunioes,
            dispensados: &self.dispensados,
        }
    }
}

/// Le o mundo. E a UNICA leitura que o piloto faz — Home, avisos, Hermes e
/// resgate passam todos por aqui.
pub fn ler<R: Runtime>(app: &AppHandle<R>) -> Result<Leitura, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let now_local = crate::surface::now_local(app);
    let offset = now_local.offset();
    let hoje = mos_core::Day::from_local(now_local);

    let tasks = state.work.tasks(false)?;
    let projects = state.work.projects(false)?;
    let reminders = state.attention.open()?;
    let inbox = state.captures.inbox(500)?;
    // Falhar no academico nao derruba a Home: e uma camada por cima.
    let academic = mos_core::AcademicService::new(state.storage.clone())
        .compromissos_entre(
            now_local - time::Duration::days(14),
            now_local + time::Duration::days(30),
            now_local,
        )
        .unwrap_or_default();
    let inicio = hoje.inicio_do_dia(offset);
    let agenda =
        crate::calendar::janela(app, inicio, inicio + time::Duration::days(2)).unwrap_or_default();
    let daily = Some(state.daily.today(&hoje)?);
    let previous_objectives = state
        .daily
        .previous(&hoje)?
        .map(|(_, objectives)| objectives)
        .unwrap_or_default();
    let sync = sync_no_retrato(app);
    let ultima_presenca = app
        .try_state::<PilotoRuntime>()
        .and_then(|rt| rt.presenca_anterior.lock().ok().map(|p| *p))
        .flatten();
    let mut inicios = state
        .storage
        .inicios_de_sessao(SESSOES_PARA_HABITO, offset)
        .unwrap_or_default();
    let habitos = Habitos {
        inicio_habitual_minuto: mos_core::inicio_habitual(&mut inicios),
        fim_habitual_minuto: None,
    };

    // As reunioes das duas ultimas semanas: mais velha que isso, uma acao nao
    // revisada ja nao e pendencia da reuniao — e da memoria de quem estava la.
    let reunioes = state
        .meetings
        .overview(false)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| now_local - row.meeting.started_at <= time::Duration::days(14))
        .map(|row| mos_core::ReuniaoNoRetrato {
            id: row.meeting.id.to_string(),
            titulo: row.meeting.title.clone(),
            fase: row.phase,
            acoes_pendentes: row.pending_actions,
            atencao: row.attention.clone(),
            quando: row.meeting.ended_at.unwrap_or(row.meeting.started_at),
        })
        .collect();

    Ok(Leitura {
        now_local,
        tasks,
        projects,
        reminders,
        inbox,
        academic,
        agenda,
        daily,
        previous_objectives,
        sync,
        ultima_presenca,
        habitos,
        reunioes,
        // Sem leitura das dispensas, a atencao aparece inteira: melhor mostrar
        // de novo que esconder por engano.
        dispensados: state
            .storage
            .dispensas_de_atencao(now_local)
            .unwrap_or_default(),
    })
}

/// O estado do sync como o piloto o ve. Traduz a saude do `mos-sync`.
fn sync_no_retrato<R: Runtime>(app: &AppHandle<R>) -> SyncNoRetrato {
    let Some(saude) = crate::sync::saude(app) else {
        return SyncNoRetrato::Desligado;
    };
    match saude.estado {
        mos_sync::EstadoDeSaude::Desligado => SyncNoRetrato::Desligado,
        mos_sync::EstadoDeSaude::EmDia | mos_sync::EstadoDeSaude::Sincronizando { .. } => {
            if saude.pendentes > 0 {
                SyncNoRetrato::Pendente {
                    pendentes: saude.pendentes,
                }
            } else {
                SyncNoRetrato::EmDia
            }
        }
        mos_sync::EstadoDeSaude::Pendente { pendentes } => SyncNoRetrato::Pendente { pendentes },
        mos_sync::EstadoDeSaude::Offline { pendentes, .. } => SyncNoRetrato::Offline {
            pendentes,
            desde: saude.registro.ultimo_ok_em.clone(),
        },
        mos_sync::EstadoDeSaude::Erro {
            pendentes,
            mensagem,
            ..
        } => SyncNoRetrato::Erro {
            pendentes,
            mensagem,
        },
    }
}

fn avisar<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit("data-changed", "piloto");
}

// ------------------------------------------------------------------ leitura

#[tauri::command]
pub fn piloto_panorama<R: Runtime>(app: AppHandle<R>) -> Result<Panorama, CoreError> {
    let leitura = ler(&app)?;
    Ok(mos_core::panorama(&leitura.retrato()))
}

#[tauri::command]
pub fn piloto_proposta_de_encerramento<R: Runtime>(
    app: AppHandle<R>,
) -> Result<PropostaDeEncerramento, CoreError> {
    let leitura = ler(&app)?;
    Ok(mos_core::propor_encerramento(&leitura.retrato()))
}

#[tauri::command]
pub fn piloto_resgate<R: Runtime>(app: AppHandle<R>) -> Result<Option<PlanoDeResgate>, CoreError> {
    let leitura = ler(&app)?;
    let retrato = leitura.retrato();
    Ok(mos_core::detectar_ausencia(&retrato).map(|a| mos_core::plano_de_resgate(&retrato, &a)))
}

// ------------------------------------------------------------------ escrita

/// "Montar meu dia": grava a proposta como ela veio, sem edicao.
#[tauri::command]
pub fn piloto_iniciar_dia<R: Runtime>(
    app: AppHandle<R>,
) -> Result<mos_core::DailyToday, CoreError> {
    let leitura = ler(&app)?;
    let proposta = mos_core::propor_dia(&leitura.retrato());
    let hoje = mos_core::Day::from_local(leitura.now_local);
    let dia = app
        .state::<AppState>()
        .daily
        .start_planned(hoje, &proposta.input)?;
    avisar(&app);
    Ok(dia)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncerramentoPedido {
    /// As Tasks a mover para amanha (ids). Vem da proposta, editada ou nao.
    #[serde(default)]
    pub mover: Vec<String>,
    #[serde(default)]
    pub resolutions: Vec<mos_core::ObjectiveResolution>,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub summary: String,
}

/// "Encerrar dia": move o que foi pedido e fecha a sessao.
///
/// Mover muda `scheduled_for` e conta adiamento. `due_at` nao e tocado.
#[tauri::command]
pub fn piloto_encerrar_dia<R: Runtime>(
    app: AppHandle<R>,
    pedido: EncerramentoPedido,
) -> Result<mos_core::DailyToday, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let hoje = crate::daily::hoje(&app);
    let input = mos_core::EndDayInput {
        resolutions: pedido.resolutions,
        mood: pedido.mood,
        summary: pedido.summary,
    };
    let dia = state.daily.end_planned(&hoje, &input, &pedido.mover)?;
    let _ = app.emit("data-changed", "daily");
    avisar(&app);
    Ok(dia)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResgatePedido {
    pub acoes: Vec<AcaoDeResgate>,
}

/// Aplica o que a pessoa confirmou no resgate. Cada acao e independente: uma
/// que falha nao desfaz as outras, e o motivo volta na lista.
#[tauri::command]
pub fn piloto_resgate_aplicar<R: Runtime>(
    app: AppHandle<R>,
    pedido: ResgatePedido,
) -> Result<mos_core::ResgateResultado, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    // A regra mora no core, a mesma que o bolso usa.
    let resultado = mos_core::aplicar_resgate(
        &state.work,
        &state.captures,
        &state.attention,
        pedido.acoes,
        crate::surface::now_local(&app),
    );
    let _ = app.emit("data-changed", "resgate");
    avisar(&app);
    Ok(resultado)
}

// ------------------------------------------------------- dispensar atencao

/// Dispensa um item de atencao (ADR-083). `adiar_horas` ausente = ate haver
/// fato novo; presente = volta depois de tantas horas.
#[tauri::command]
pub fn piloto_dispensar<R: Runtime>(
    app: AppHandle<R>,
    chave: String,
    adiar_horas: Option<i64>,
) -> Result<(), CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let agora = state.clock.now();
    let ate = adiar_horas.map(|horas| agora + time::Duration::hours(horas.clamp(1, 24 * 14)));
    state.storage.dispensar_atencao(&chave, ate, agora)?;
    let _ = app.emit("data-changed", "piloto");
    Ok(())
}

// ------------------------------------------------------------ sinais de uso

/// Onde o M/OS ainda pede trabalho manual, nos ultimos 14 dias (ADR-082).
/// Local, sem conteudo; alimenta o painel de Diagnostico.
#[tauri::command]
pub fn sinais_de_uso<R: Runtime>(app: AppHandle<R>) -> Result<SinaisNaTela, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let sinais = state.storage.sinais_de_uso(state.clock.now(), 14)?;
    Ok(SinaisNaTela {
        leituras: sinais.leituras(),
        sinais,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SinaisNaTela {
    pub sinais: mos_core::SinaisDeUso,
    pub leituras: Vec<String>,
}

// ------------------------------------------------------------- task: comecar

#[tauri::command]
pub fn task_start<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Task, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    // A troca de foco é atômica no repositório, inclusive via Hermes e web.
    let task = state.work.start_task(&id, state.clock.now())?;
    let _ = app.emit("data-changed", "task");
    Ok(task)
}

#[tauri::command]
pub fn task_stop<R: Runtime>(app: AppHandle<R>, id: String) -> Result<Task, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let task = state.work.stop_task(&id)?;
    let _ = app.emit("data-changed", "task");
    Ok(task)
}

/// Planeja (ou adia) uma Task para um dia. `day` vazio tira do planejamento.
#[tauri::command]
pub fn task_plan<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    day: String,
    adiando: bool,
) -> Result<Task, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let dia = if day.trim().is_empty() {
        None
    } else {
        Some(mos_core::Day::parse(day.trim())?)
    };
    let task = state.work.plan_task(&id, dia, adiando)?;
    let _ = app.emit("data-changed", "task");
    Ok(task)
}

// ------------------------------------------------------------------ autopilot

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotStatus {
    pub ligado: bool,
}

#[tauri::command]
pub fn autopilot_status<R: Runtime>(app: AppHandle<R>) -> Result<AutopilotStatus, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    Ok(AutopilotStatus {
        ligado: state.storage.autopilot_ligado()?,
    })
}

#[tauri::command]
pub fn autopilot_set<R: Runtime>(
    app: AppHandle<R>,
    ligado: bool,
) -> Result<AutopilotStatus, CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    state.storage.set_autopilot(ligado)?;
    Ok(AutopilotStatus { ligado })
}

/// Adia um aviso. `minutos` segue `OpcaoDeAdiar`: -1 hoje a noite, -2 amanha.
#[tauri::command]
pub fn autopilot_adiar<R: Runtime>(
    app: AppHandle<R>,
    chave: String,
    minutos: i64,
) -> Result<(), CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let now_local = crate::surface::now_local(&app);
    let ate = mos_core::instante_de_adiar(now_local, Some(minutos))
        .unwrap_or(now_local + time::Duration::minutes(30));
    state.storage.adiar_aviso(&chave, ate)?;
    Ok(())
}

#[tauri::command]
pub fn autopilot_resolver<R: Runtime>(app: AppHandle<R>, chave: String) -> Result<(), CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    state.storage.resolver_aviso(&chave, state.clock.now())?;
    Ok(())
}

/// O aviso, como a tela o recebe pelo evento `autopilot-aviso`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvisoEntregueEvento {
    pub chave: String,
    pub tipo: String,
    pub titulo: String,
    pub corpo: String,
    pub alvo: mos_core::Alvo,
    pub adiar: Vec<mos_core::OpcaoDeAdiar>,
    pub acao_principal: String,
}

/// Uma passada: le, decide, entrega, registra.
fn tick<R: Runtime>(app: &AppHandle<R>) {
    let Ok(leitura) = ler(app) else { return };
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let ligado = state.storage.autopilot_ligado().unwrap_or(true);
    let silencio = state
        .attention
        .settings()
        .map(|s| s.quiet)
        .unwrap_or_default();
    let retrato = leitura.retrato();
    let candidatos = mos_core::candidatos_a_aviso(&retrato);
    if candidatos.is_empty() {
        return;
    }
    let historico = state
        .storage
        .avisos_desde(state.clock.now() - JANELA_DO_HISTORICO)
        .unwrap_or_default();
    let politica = mos_core::PoliticaDeAvisos {
        silencio: &silencio,
        ligado,
    };
    let decisoes = mos_core::decidir_avisos(&retrato, candidatos, &historico, &politica);
    let os_ligado = state
        .attention
        .settings()
        .map(|s| s.os_channel_enabled)
        .unwrap_or(true);
    for d in decisoes {
        match d.veredito {
            mos_core::VereditoDeAviso::Entregar => {
                let c = d.candidato;
                eprintln!("[autopilot] aviso {} ({})", c.tipo.as_str(), c.chave);
                let _ = state.storage.registrar_aviso(
                    &c.chave,
                    c.tipo,
                    (&c.alvo.kind, &c.alvo.id),
                    &c.titulo,
                    &c.corpo,
                    state.clock.now(),
                );
                let evento = AvisoEntregueEvento {
                    chave: c.chave.clone(),
                    tipo: c.tipo.as_str().to_owned(),
                    titulo: c.titulo.clone(),
                    corpo: c.corpo.clone(),
                    alvo: c.alvo.clone(),
                    adiar: c.adiar.clone(),
                    acao_principal: c.acao_principal.clone(),
                };
                let _ = app.emit("autopilot-aviso", evento);
                if os_ligado {
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app
                        .notification()
                        .builder()
                        .title(&c.titulo)
                        .body(&c.corpo)
                        .show();
                }
            }
            mos_core::VereditoDeAviso::Suprimir { motivo } => {
                // So o log, e so por chave: o conteudo nao precisa aparecer.
                if !motivo.starts_with("já avisado") && !motivo.starts_with("adiado") {
                    eprintln!(
                        "[autopilot] suprimido {} ({}): {motivo}",
                        d.candidato.tipo.as_str(),
                        d.candidato.chave
                    );
                }
            }
        }
    }
    let _ = state
        .storage
        .podar_avisos(state.clock.now() - time::Duration::days(7));
}

/// O laco de fundo. Pulso de cinco minutos, acordado por mutacao.
pub async fn run<R: Runtime>(app: AppHandle<R>) {
    // A primeira passada espera o app assentar: a Home ja mostra tudo isto na
    // tela, e um toast em cima da abertura seria dizer duas vezes.
    tokio::time::sleep(Duration::from_secs(45)).await;
    loop {
        let handle = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || tick(&handle)).await;
        let Some(runtime) = app.try_state::<PilotoRuntime>() else {
            return;
        };
        let _ = tokio::time::timeout(PULSO, runtime.acordar.notified()).await;
    }
}

/// Uma mutacao aconteceu: reavalia daqui a pouco.
pub fn acordar<R: Runtime>(app: &AppHandle<R>) {
    if let Some(runtime) = app.try_state::<PilotoRuntime>() {
        let handle = app.clone();
        let _ = &runtime;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(DEBOUNCE).await;
            if let Some(rt) = handle.try_state::<PilotoRuntime>() {
                rt.acordar.notify_one();
            }
        });
    }
}

/// O bloco do piloto para o preambulo do Hermes.
///
/// Curto e so quando ha algo: a recomendacao para agora, com a razao, e ate
/// cinco itens de atencao. E o que faz "o que faco agora?" e "tenho algo
/// atrasado?" serem respondidos sem uma acao — a regra determinista ja tem a
/// resposta, e gastar um turno de proposta/preview/confirmacao para devolver
/// tres linhas seria o mesmo erro que o `DAILY-SESSION.md` §6 recusou.
pub fn bloco_de_atencao<R: Runtime>(app: &AppHandle<R>) -> String {
    let Ok(leitura) = ler(app) else {
        return String::new();
    };
    let panorama = mos_core::panorama(&leitura.retrato());
    let mut linhas = Vec::new();
    if let Some(agora) = &panorama.agora.agora {
        linhas.push(format!(
            "Recomendada para agora: \"{}\"{} — {}",
            agora.titulo,
            if agora.estimativa.is_empty() {
                String::new()
            } else {
                format!(" ({})", agora.estimativa)
            },
            agora.razoes.join(", ")
        ));
    }
    for item in panorama.atencao.iter().take(5) {
        linhas.push(format!(
            "- [{}] {}: {}",
            item.tipo.as_str(),
            item.titulo,
            item.razoes.join(", ")
        ));
    }
    if linhas.is_empty() {
        return String::new();
    }
    format!(
        "[O que o piloto ve]\n{}\nO piloto e deterministico: cite estas linhas ao responder \"o que faco agora?\" ou \"tenho algo atrasado?\" em vez de propor acao. Para comecar uma Task use `mos.task.start`; para jogar para outro dia use `mos.task.plan`.\n[Fim do piloto]\n\n",
        linhas.join("\n")
    )
}

/// Na abertura: troca a presenca e guarda a anterior para o resgate.
pub fn registrar_presenca<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(runtime) = app.try_state::<PilotoRuntime>() else {
        return;
    };
    match state.storage.trocar_presenca(state.clock.now()) {
        Ok(anterior) => {
            if let Ok(mut slot) = runtime.presenca_anterior.lock() {
                *slot = anterior;
            }
        }
        Err(e) => eprintln!("[autopilot] presenca nao gravada: {}", e.message),
    }
}

/// O resgate foi concluido (ou dispensado): a ausencia deixa de valer ate a
/// proxima. Sem isto, o cartao voltaria a cada abertura do mesmo dia.
#[tauri::command]
pub fn piloto_resgate_concluir<R: Runtime>(app: AppHandle<R>) -> Result<(), CoreError> {
    let state = app.try_state::<AppState>().ok_or_else(erro_abrindo)?;
    let runtime = app.try_state::<PilotoRuntime>().ok_or_else(erro_abrindo)?;
    if let Ok(mut slot) = runtime.presenca_anterior.lock() {
        *slot = Some(state.clock.now());
    }
    avisar(&app);
    Ok(())
}

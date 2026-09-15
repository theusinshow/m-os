//! O piloto no bolso: o mesmo motor, a mesma leitura, sem Tauri.
//!
//! Nada aqui decide. `mos_core::piloto` decide; este arquivo le os servicos do
//! `Estado`, monta o `Retrato` e responde HTTP. E a garantia do
//! `FEATURE-DEVELOPMENT.md`: a logica de negocio nao se duplica — o celular e o
//! PC perguntam "o que faco agora?" ao mesmo codigo.

use std::sync::Mutex;

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use mos_core::{CoreError, Habitos, Panorama, Retrato, SyncNoRetrato};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::api::{de_core, Resultado};
use crate::estado::Estado;

/// A presenca ANTERIOR a esta visita, para o Rescue Mode.
///
/// Uma visita e uma sequencia de pedidos sem intervalo maior que `VISITA`. A
/// primeira leitura de uma visita nova guarda a presenca anterior aqui, e e
/// ela que o resgate compara com hoje — depois de gravada a de agora, ela so
/// existe nesta caixa.
#[derive(Default)]
pub struct Presenca {
    anterior: Mutex<Option<OffsetDateTime>>,
    ultima_leitura: Mutex<Option<OffsetDateTime>>,
}

/// Quanto tempo sem pedidos separa duas visitas.
const VISITA: time::Duration = time::Duration::hours(6);

impl Presenca {
    /// Chamado a cada leitura do panorama. Devolve a presenca a comparar.
    fn registrar(&self, estado: &Estado, agora: OffsetDateTime) -> Option<OffsetDateTime> {
        let mut ultima = self.ultima_leitura.lock().ok()?;
        let visita_nova = ultima.map(|u| agora - u > VISITA).unwrap_or(true);
        *ultima = Some(agora);
        drop(ultima);
        if visita_nova {
            let anterior = estado.storage.trocar_presenca(agora).ok().flatten();
            if let Ok(mut slot) = self.anterior.lock() {
                *slot = anterior;
            }
        }
        self.anterior.lock().ok().and_then(|slot| *slot)
    }

    fn concluir(&self, agora: OffsetDateTime) {
        if let Ok(mut slot) = self.anterior.lock() {
            *slot = Some(agora);
        }
    }
}

pub fn rotas() -> Router<Estado> {
    Router::new()
        .route("/api/piloto", get(panorama))
        .route("/api/piloto/iniciar-dia", post(iniciar_dia))
        .route("/api/piloto/resgate-concluir", post(resgate_concluir))
        .route("/api/tasks/{id}/comecar", post(comecar))
        .route("/api/tasks/{id}/parar", post(parar))
        .route("/api/tasks/{id}/planejar", post(planejar))
}

#[derive(Deserialize)]
pub struct QuandoPergunta {
    pub agora: Option<String>,
}

fn instante(pergunta: &QuandoPergunta) -> OffsetDateTime {
    pergunta
        .agora
        .as_deref()
        .and_then(|t| OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339).ok())
        .unwrap_or_else(OffsetDateTime::now_utc)
}

/// Tudo que o retrato precisa, lido uma vez.
struct Leitura {
    now_local: OffsetDateTime,
    tasks: Vec<mos_core::Task>,
    projects: Vec<mos_core::Project>,
    reminders: Vec<mos_core::Reminder>,
    inbox: Vec<mos_core::Capture>,
    academic: Vec<mos_core::Compromisso>,
    agenda: Vec<mos_core::CalendarItem>,
    daily: Option<mos_core::DailyToday>,
    sync: SyncNoRetrato,
    ultima_presenca: Option<OffsetDateTime>,
    habitos: Habitos,
}

impl Leitura {
    fn retrato(&self) -> Retrato<'_> {
        Retrato {
            now_local: self.now_local,
            tasks: &self.tasks,
            projects: &self.projects,
            reminders: &self.reminders,
            inbox: &self.inbox,
            academic: &self.academic,
            agenda: &self.agenda,
            daily: self.daily.as_ref(),
            sync: &self.sync,
            ultima_presenca: self.ultima_presenca,
            habitos: &self.habitos,
        }
    }
}

/// O sync como o piloto o ve, a partir da saude gravada.
pub fn sync_no_retrato(estado: &Estado) -> SyncNoRetrato {
    use mos_sync::OutboxRepository;
    if estado.hub.is_none() {
        return SyncNoRetrato::Desligado;
    }
    let registro = estado.storage.saude_do_sync().unwrap_or_default();
    let pendentes = estado.storage.quantidade_pendente().unwrap_or(0);
    match mos_sync::estado_de_saude(mos_sync::Sinais {
        ligado: true,
        rodando: false,
        pendentes,
        registro: &registro,
    }) {
        mos_sync::EstadoDeSaude::Desligado => SyncNoRetrato::Desligado,
        mos_sync::EstadoDeSaude::EmDia | mos_sync::EstadoDeSaude::Sincronizando { .. } => {
            if pendentes > 0 {
                SyncNoRetrato::Pendente { pendentes }
            } else {
                SyncNoRetrato::EmDia
            }
        }
        mos_sync::EstadoDeSaude::Pendente { pendentes } => SyncNoRetrato::Pendente { pendentes },
        mos_sync::EstadoDeSaude::Offline { pendentes, .. } => SyncNoRetrato::Offline {
            pendentes,
            desde: registro.ultimo_ok_em.clone(),
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

fn ler(
    estado: &Estado,
    now_local: OffsetDateTime,
    ultima_presenca: Option<OffsetDateTime>,
) -> Result<Leitura, CoreError> {
    let offset = now_local.offset();
    let hoje = mos_core::Day::from_local(now_local);
    let tasks = estado.work.tasks(false)?;
    let projects = estado.work.projects(false)?;
    let reminders = estado.attention.open()?;
    let inbox = estado.captures.inbox(500)?;
    let academic = estado
        .academic
        .compromissos_entre(
            now_local - time::Duration::days(14),
            now_local + time::Duration::days(30),
            now_local,
        )
        .unwrap_or_default();
    let inicio = hoje.inicio_do_dia(offset);
    let agenda =
        crate::api::compor_agenda(estado, inicio, inicio + time::Duration::days(2), now_local)
            .unwrap_or_default();
    let daily = estado.daily.today(&hoje).ok();
    let mut inicios = estado
        .storage
        .inicios_de_sessao(14, offset)
        .unwrap_or_default();
    Ok(Leitura {
        now_local,
        tasks,
        projects,
        reminders,
        inbox,
        academic,
        agenda,
        daily,
        sync: sync_no_retrato(estado),
        ultima_presenca,
        habitos: Habitos {
            inicio_habitual_minuto: mos_core::inicio_habitual(&mut inicios),
            fim_habitual_minuto: None,
        },
    })
}

async fn panorama(
    State(estado): State<Estado>,
    Query(pergunta): Query<QuandoPergunta>,
) -> Resultado<Json<Panorama>> {
    let agora = instante(&pergunta);
    let presenca = estado
        .presenca
        .registrar(&estado, OffsetDateTime::now_utc());
    let leitura = tokio::task::spawn_blocking(move || ler(&estado, agora, presenca))
        .await
        .map_err(|c| {
            crate::api::Erro(axum::http::StatusCode::INTERNAL_SERVER_ERROR, c.to_string())
        })?
        .map_err(de_core)?;
    Ok(Json(mos_core::panorama(&leitura.retrato())))
}

/// "Montar meu dia" no bolso: a proposta como veio, gravada.
async fn iniciar_dia(
    State(estado): State<Estado>,
    Query(pergunta): Query<QuandoPergunta>,
) -> Resultado<Json<serde_json::Value>> {
    let agora = instante(&pergunta);
    let dia = crate::api::escrever(&estado, move |estado| {
        let leitura = ler(estado, agora, None)?;
        let proposta = mos_core::propor_dia(&leitura.retrato());
        let hoje = mos_core::Day::from_local(agora);
        if let Some(stale) = leitura.daily.as_ref().and_then(|d| d.stale.as_ref()) {
            let resolutions = leitura
                .daily
                .as_ref()
                .map(|d| {
                    d.stale_objectives
                        .iter()
                        .filter(|o| o.status == mos_core::ObjectiveStatus::Pending)
                        .map(|o| mos_core::ObjectiveResolution {
                            objective_id: o.id.to_string(),
                            status: "carried_over".into(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let _ = estado.daily.end_session(
                stale.id,
                &mos_core::EndDayInput {
                    resolutions,
                    mood: String::new(),
                    summary: String::new(),
                },
            );
        }
        for draft in proposta
            .input
            .main
            .iter()
            .chain(proposta.input.secondaries.iter())
        {
            if draft.link_kind == "task" {
                let _ = estado
                    .work
                    .plan_task(&draft.link_id, Some(hoje.clone()), false);
            }
        }
        let dia = estado.daily.start(hoje, &proposta.input)?;
        for draft in proposta
            .input
            .main
            .iter()
            .chain(proposta.input.secondaries.iter())
        {
            if let Ok(origem) = mos_core::DailyObjectiveId::parse(&draft.carried_from) {
                let _ = estado
                    .daily
                    .set_objective_status(origem, mos_core::ObjectiveStatus::CarriedOver);
            }
        }
        Ok(dia)
    })
    .await?;
    Ok(Json(serde_json::to_value(dia).unwrap_or_default()))
}

async fn resgate_concluir(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    estado.presenca.concluir(OffsetDateTime::now_utc());
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn comecar(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let task = crate::api::escrever(&estado, move |estado| {
        for t in estado.work.tasks(false)? {
            if t.started_at.is_some() && t.id.to_string() != id {
                let _ = estado.work.stop_task(&t.id.to_string());
            }
        }
        estado.work.start_task(&id, OffsetDateTime::now_utc())
    })
    .await?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

async fn parar(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let task = crate::api::escrever(&estado, move |estado| estado.work.stop_task(&id)).await?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct Planejar {
    /// `AAAA-MM-DD`, ou vazio para tirar do planejamento.
    #[serde(default)]
    pub dia: String,
    #[serde(default)]
    pub adiando: bool,
}

async fn planejar(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<Planejar>,
) -> Resultado<Json<serde_json::Value>> {
    let task = crate::api::escrever(&estado, move |estado| {
        let dia = if pedido.dia.trim().is_empty() {
            None
        } else {
            Some(mos_core::Day::parse(pedido.dia.trim())?)
        };
        estado.work.plan_task(&id, dia, pedido.adiando)
    })
    .await?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

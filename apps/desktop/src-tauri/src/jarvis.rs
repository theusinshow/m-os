//! Camada de aplicacao do Jarvis.
//!
//! E o unico lugar onde a ponte e o dominio se encontram. `mos-hermes` continua
//! sem `mos-core` e sem SQLite; `mos-core` continua sem saber que existe rede.
//! A traducao entre `Outcome` e parte de mensagem mora aqui (ADR-024, ADR-025).
//!
//! Duas responsabilidades:
//!
//! 1. **Gravar o turno.** `TurnRecorder` acumula os deltas em memoria e escreve
//!    UMA vez, quando o turno assenta. Um INSERT por token sob `synchronous=FULL`
//!    seria um fsync por token (ADR-017).
//! 2. **Montar o contexto.** O que o `@` sempre pareceu fazer e nao fazia: ler o
//!    M/OS e prefixar um bloco estruturado ao prompt, registrando o que foi
//!    enviado (ADR-027, ADR-028).

use mos_core::{
    ContextEntity, ContextOrigin, Conversation, ConversationService, ConversationSummary,
    CoreError, Message, MessageStatus, PartBody, ProposalStatus, ToolRunState,
};
use mos_hermes::{HistoryMessage, Outcome};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::AppState;

/// Evento de streaming, com o endereco de onde ele pertence.
///
/// O `Outcome` sozinho nao dizia a qual mensagem o delta pertencia — e era por
/// isso que duas superficies assinando o mesmo barramento dividiam a resposta
/// entre si. Agora cada quadro carrega o proprio destino.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnEvent {
    pub conversation_id: String,
    pub message_id: String,
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// Contexto pedido pelo renderer antes de enviar.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextInput {
    /// `explicit` ou `automatic`.
    pub origin: String,
    /// `project`, `task`, `capture`, `resource`, `workspace` ou `screen`.
    pub entity: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
}

/// O turno em curso.
///
/// Vive na memoria do processo enquanto a resposta chega, e vira linhas no banco
/// quando ela assenta. Guardar aqui e o que permite ao streaming nao tocar o
/// disco.
#[derive(Default)]
pub struct TurnRecorder {
    pub conversation_id: String,
    pub message_id: String,
    text: String,
    reasoning: String,
    /// Execucoes na ordem em que comecaram. `tool.complete` fecha a ultima com o
    /// mesmo nome — o gateway nao correlaciona execucoes por id.
    tools: Vec<(String, ToolRunState)>,
    status: Vec<String>,
    error: Option<String>,
    /// Partes que ja existiam antes de o primeiro token chegar.
    ///
    /// Existe por causa do salto de busca: o turno que responde a um
    /// `mos-query` comeca com uma ida ao banco JA FEITA, e o que ela devolveu
    /// atravessou a ponte antes de o modelo dizer qualquer coisa. A ADR-027
    /// pede o registro do que foi enviado, e este e o unico lugar onde ele
    /// cabe — a mensagem anterior ja foi gravada, e a proxima ainda nao existe.
    seeded: Vec<PartBody>,
}

impl TurnRecorder {
    pub fn start(conversation_id: String, message_id: String) -> Self {
        Self {
            conversation_id,
            message_id,
            ..Default::default()
        }
    }

    /// Registra uma parte que precede o turno.
    pub fn seed(&mut self, part: PartBody) {
        self.seeded.push(part);
    }

    /// Acumula. Devolve `true` quando o turno assentou e precisa ser gravado.
    pub fn absorb(&mut self, outcome: &Outcome) -> bool {
        match outcome {
            Outcome::Delta { text } => {
                self.text.push_str(text);
                false
            }
            Outcome::Reasoning { text } => {
                self.reasoning.push_str(text);
                false
            }
            Outcome::Status { text } => {
                self.status.push(text.clone());
                false
            }
            Outcome::Tool { name, running } => {
                if *running {
                    self.tools.push((name.clone(), ToolRunState::Running));
                } else if let Some(entry) = self
                    .tools
                    .iter_mut()
                    .rev()
                    .find(|(candidate, _)| candidate == name)
                {
                    entry.1 = ToolRunState::Success;
                }
                false
            }
            Outcome::Approval { .. } | Outcome::Clarify { .. } => {
                // O agente parou e espera. O turno nao assentou — assentar aqui
                // fecharia a resposta no meio, e o que vier depois da aprovacao
                // nao teria mensagem para onde ir.
                if let Some((_, state)) = self.tools.last_mut() {
                    *state = ToolRunState::WaitingPermission;
                }
                false
            }
            Outcome::SudoRefused => {
                self.status.push(
                    "O Hermes pediu senha de sudo na VPS. O M/OS nao pede senha de root — se isso for mesmo necessario, responda pelo dashboard ou pela TUI."
                        .to_owned(),
                );
                false
            }
            Outcome::Failed { message } => {
                self.error = Some(message.clone());
                true
            }
            Outcome::Complete => true,
            Outcome::Busy | Outcome::History { .. } | Outcome::Title { .. } => false,
            Outcome::UnknownFrame { .. } => false,
        }
    }

    /// Marca as execucoes que ficaram no ar. Uma ferramenta em `running` quando
    /// o turno acaba nao terminou — ela foi interrompida junto.
    fn settle_tools(&mut self, cancelled: bool) {
        for (_, state) in &mut self.tools {
            if matches!(
                state,
                ToolRunState::Running | ToolRunState::WaitingPermission
            ) {
                *state = if cancelled {
                    ToolRunState::Cancelled
                } else {
                    ToolRunState::Error
                };
            }
        }
    }

    /// A busca que o modelo pediu nesta resposta, se pediu.
    ///
    /// Lida ANTES de `into_parts`, que consome o registrador: quem decide se
    /// vale um segundo salto precisa saber disso enquanto ainda pode agir.
    pub fn requested_query(&self) -> Option<String> {
        // A acao ganha da busca quando as duas vem juntas, e a razao e a ordem
        // do trabalho: se o modelo ja sabe o que propor, procurar mais seria
        // gastar um turno para confirmar o que ele acabou de afirmar.
        if mos_core::split_fenced(&self.text, "mos-action").1.is_some() {
            return None;
        }
        mos_core::split_fenced(&self.text, "mos-query").1
    }

    /// As partes na ordem em que devem ser lidas.
    pub fn into_parts(mut self, status: MessageStatus, now_local: time::OffsetDateTime) -> Vec<PartBody> {
        self.settle_tools(status == MessageStatus::Interrupted);

        let mut parts = Vec::new();
        parts.append(&mut self.seeded);
        for text in self.status.drain(..) {
            parts.push(PartBody::Status { text });
        }
        for (name, state) in self.tools.drain(..) {
            parts.push(PartBody::ToolRun {
                name,
                state,
                detail: String::new(),
            });
        }
        if !self.reasoning.is_empty() {
            parts.push(PartBody::Reasoning {
                text: self.reasoning,
            });
        }
        // A proposta sai do texto e vira parte propria. O JSON cru na thread e
        // ruido, e a mesma informacao volta logo abaixo desenhada como preview.
        let (text, proposal) = split_proposal(&self.text);
        // A busca sai pelo mesmo motivo, e vira uma execucao visivel: quem le a
        // conversa precisa ver que houve uma ida ao banco entre a pergunta e a
        // resposta, senao a pausa parece travamento.
        let (text, query) = mos_core::split_fenced(&text, "mos-query");
        if !text.is_empty() {
            parts.push(PartBody::Text { text });
        }
        if let Some(raw) = &query {
            let (state, detail) = match mos_core::parse_query(raw) {
                Ok(request) => (ToolRunState::Success, request.search),
                Err(error) => (ToolRunState::Error, error.message),
            };
            parts.push(PartBody::ToolRun {
                name: "Busca no M/OS".to_owned(),
                state,
                detail,
            });
        }
        if let Some(raw) = proposal {
            parts.push(proposal_part(&raw, now_local));
        }
        if let Some(message) = self.error {
            parts.push(PartBody::Error { message });
        }
        parts
    }

    pub fn has_content(&self) -> bool {
        !self.seeded.is_empty()
            || !self.text.is_empty()
            || !self.reasoning.is_empty()
            || !self.tools.is_empty()
            || self.error.is_some()
    }
}

/// Separa a proposta do texto da resposta.
///
/// Devolve o texto SEM o bloco e a proposta, quando houver. Tirar o bloco do
/// texto e deliberado: o JSON cru na thread e ruido, e a mesma informacao volta
/// desenhada como cartao de preview logo abaixo.
///
/// So a PRIMEIRA proposta e lida. O contrato pede uma por mensagem, e executar
/// a segunda de uma resposta que veio fora do contrato seria confiar num
/// formato que ja se provou errado naquela mesma mensagem.
pub fn split_proposal(text: &str) -> (String, Option<String>) {
    // A leitura mora no `mos-core` desde que a busca ganhou uma cerca propria:
    // duas copias da mesma varredura divergiriam na primeira vez que uma delas
    // ganhasse um caso de borda.
    mos_core::split_fenced(text, "mos-action")
}

/// Transforma a proposta crua numa parte pendente, ou numa parte recusada.
///
/// Recusa tambem vira parte: uma proposta que nao bate com o esquema precisa
/// aparecer na conversa dizendo o motivo. Descartar em silencio deixaria o
/// usuario vendo o Hermes prometer uma acao que nunca existiu.
pub fn proposal_part(raw: &str, now_local: time::OffsetDateTime) -> PartBody {
    match mos_core::parse_action_at(raw, now_local) {
        Ok(args) => PartBody::ActionProposal {
            raw: raw.to_owned(),
            preview: mos_core::preview_of(&args),
            status: ProposalStatus::Pending,
            outcome: String::new(),
            audit: None,
        },
        Err(error) => PartBody::ActionProposal {
            raw: raw.to_owned(),
            preview: mos_core::ActionPreview {
                action: "desconhecida".into(),
                title: "PROPOSTA RECUSADA".into(),
                lines: Vec::new(),
                risk: mos_core::FunctionRisk::High,
                confirmation: mos_core::FunctionConfirmation::Explicit,
            },
            status: ProposalStatus::Refused,
            outcome: error.message,
            audit: None,
        },
    }
}

/// Executa uma acao aprovada, pelos mesmos servicos que a interface usa.
///
/// Nunca SQL proprio, nunca um atalho: e a invariante que faz a acao do Hermes
/// obedecer as mesmas regras que a acao do usuario (ADR-024, ADR-032).
async fn run_action<R: Runtime>(
    app: &AppHandle<R>,
    args: &mos_core::ActionArgs,
) -> Result<mos_core::ActionEffect, CoreError> {
    let state = app.state::<AppState>();
    match args {
        mos_core::ActionArgs::CaptureCreate { content } => {
            let capture = state.captures.create(mos_core::CreateCaptureInput {
                content: content.clone(),
                source: mos_core::CaptureSource::Home,
            })?;
            Ok(mos_core::ActionEffect::new(
                format!("Capture criada: {}", capture.content),
                Some(mos_core::UndoStep::ArchiveCapture {
                    id: capture.id.to_string(),
                }),
            )
            .touching("capture", capture.id.to_string(), capture.content))
        }
        mos_core::ActionArgs::CaptureToTask {
            capture,
            title,
            project,
        } => {
            let origem = resolve_capture(&state, capture)?;
            let project_id = match project {
                Some(name) => resolve_project(&state, name)?,
                None => None,
            };
            // Titulo vazio significa "use o que esta escrito na Capture". O
            // dominio recusa Task sem titulo, e inventar um aqui seria pior:
            // a fala ja esta guardada na Capture, e copia-la e a leitura
            // honesta de "transforma isso em task".
            let titulo = if title.trim().is_empty() {
                origem.content.clone()
            } else {
                title.clone()
            };
            let task = state.work.create_task(mos_core::CreateTaskInput {
                title: titulo,
                description: String::new(),
                project_id,
                source_capture_id: Some(origem.id.to_string()),
            })?;
            let _ = app.emit("capture-changed", "processed");
            Ok(mos_core::ActionEffect::new(
                format!("Capture virou a Task: {}", task.title),
                Some(mos_core::UndoStep::UndoCaptureToTask {
                    capture_id: origem.id.to_string(),
                    task_id: task.id.to_string(),
                }),
            )
            .touching("task", task.id.to_string(), task.title)
            .touching("capture", origem.id.to_string(), origem.content))
        }
        mos_core::ActionArgs::TaskSetProject { task, project } => {
            let alvo = resolve_task(&state, task)?;
            let anterior = alvo.project_id.map(|id| id.to_string());
            let destino = resolve_project(&state, project)?;
            // Titulo e descricao vao como estao: `update_task` escreve os tres
            // campos, e mandar vazio aqui apagaria a descricao da Task ao mover
            // ela de Project.
            let atualizada = state.work.update_task(mos_core::UpdateTaskInput {
                id: alvo.id.to_string(),
                title: alvo.title.clone(),
                description: alvo.description.clone(),
                project_id: destino.clone(),
            })?;
            let nome = destino
                .as_deref()
                .and_then(|id| mos_core::ProjectId::parse(id).ok())
                .map(|id| project_name(&state, id))
                .unwrap_or_else(|| "nenhum Project".to_owned());
            Ok(mos_core::ActionEffect::new(
                format!("{} agora e de {nome}.", atualizada.title),
                Some(mos_core::UndoStep::RestoreTaskProject {
                    id: atualizada.id.to_string(),
                    project_id: anterior,
                }),
            )
            .touching("task", atualizada.id.to_string(), atualizada.title))
        }
        mos_core::ActionArgs::ReminderCreate {
            title,
            body,
            at,
            target,
            ..
        } => {
            let instant = mos_core::parse_moment(at)?;
            let alvo = match target {
                Some(target) => Some(resolve_target(&state, target)?),
                None => None,
            };
            // `ReminderSource::Hermes` existia no dominio desde o P0 e nunca
            // tinha sido escrito por ninguem. E ele que faz o lembrete saber de
            // onde veio, e e por ele que o Attention Score e a auditoria
            // distinguem o que a pessoa agendou do que o agente propos.
            let reminder = state.attention.create_at(
                title,
                body,
                instant,
                alvo.as_ref().map(|(target, _)| *target),
                mos_core::ReminderSource::Hermes,
            )?;
            // Sem isto o lembrete existe no banco e o agendador nao sabe: ele
            // so acordaria no proximo restart, e "me lembra em vinte minutos"
            // passaria em silencio.
            crate::attention::poke(app);
            let quando = mos_core::spoken_moment(instant.to_offset(crate::surface::now_local(app).offset()));
            let mensagem = match &alvo {
                Some((_, rotulo)) => {
                    format!("Lembrete para {quando}, vinculado a \"{rotulo}\".")
                }
                None => format!("Lembrete para {quando}."),
            };
            let mut effect = mos_core::ActionEffect::new(
                mensagem,
                Some(mos_core::UndoStep::CancelReminder {
                    id: reminder.id.to_string(),
                }),
            )
            .touching("reminder", reminder.id.to_string(), reminder.title.clone());
            if let Some((target, rotulo)) = alvo {
                let (kind, id) = target.as_columns();
                effect = effect.touching(kind, id, rotulo);
            }
            Ok(effect)
        }
        mos_core::ActionArgs::ReminderResolve { reminder, state: desfecho } => {
            let alvo = resolve_reminder(&state, reminder)?;
            let transition = if desfecho == "done" {
                mos_core::Transition::Complete
            } else {
                mos_core::Transition::Cancel
            };
            let resolvido = state.attention.transition(alvo.id, transition)?;
            crate::attention::poke(app);
            Ok(mos_core::ActionEffect::new(
                format!(
                    "Lembrete \"{}\" {}.",
                    resolvido.title,
                    if desfecho == "done" { "concluido" } else { "cancelado" }
                ),
                // SEM desfazer, e a ausencia e a decisao: concluir e cancelar
                // sao estados terminais no dominio (`attention.rs`), e nao ha
                // transicao de volta. Inventar uma aqui daria ao Hermes um
                // caminho que a tela nao tem.
                None,
            )
            .touching("reminder", resolvido.id.to_string(), resolvido.title))
        }
        mos_core::ActionArgs::TaskCreate {
            title,
            description,
            project,
        } => {
            let project_id = match project {
                Some(name) => resolve_project(&state, name)?,
                None => None,
            };
            let task = state.work.create_task(mos_core::CreateTaskInput {
                title: title.clone(),
                description: description.clone(),
                project_id,
                source_capture_id: None,
            })?;
            Ok(mos_core::ActionEffect::new(
                format!("Task criada: {}", task.title),
                Some(mos_core::UndoStep::ArchiveTask {
                    id: task.id.to_string(),
                }),
            )
            .touching("task", task.id.to_string(), task.title))
        }
        mos_core::ActionArgs::TaskSetState { task, state: next } => {
            // O estado anterior e lido ANTES da mudanca: depois nao ha de onde
            // tirar, e sem ele "mover" seria a unica acao sem caminho de volta.
            let target = resolve_task(&state, task)?;
            let previous = target.state;
            let updated = state
                .work
                .set_task_state(&target.id.to_string(), mos_core::TaskState::parse(next)?)?;
            Ok(mos_core::ActionEffect::new(
                format!("{} movida para {}", updated.title, next),
                Some(mos_core::UndoStep::RestoreTaskState {
                    id: updated.id.to_string(),
                    state: previous.as_str().to_owned(),
                }),
            )
            .touching("task", updated.id.to_string(), updated.title))
        }
        mos_core::ActionArgs::ProjectCreate { name, description } => {
            let project = state.work.create_project(mos_core::CreateProjectInput {
                name: name.clone(),
                description: description.clone(),
                repository: String::new(),
            })?;
            Ok(mos_core::ActionEffect::new(
                format!("Project criado: {}", project.name),
                Some(mos_core::UndoStep::ArchiveProject {
                    id: project.id.to_string(),
                }),
            )
            .touching("project", project.id.to_string(), project.name))
        }
        mos_core::ActionArgs::ResourceCreate {
            kind,
            title,
            url,
            note,
        } => {
            let resource = state
                .memory
                .create_resource(mos_core::CreateResourceInput {
                    kind: mos_core::ResourceKind::parse(kind)?,
                    title: title.clone(),
                    url: url.clone(),
                    note: note.clone(),
                    source_capture_id: None,
                })?;
            Ok(mos_core::ActionEffect::new(
                format!("Resource salvo: {}", resource.title),
                Some(mos_core::UndoStep::ArchiveResource {
                    id: resource.id.to_string(),
                }),
            )
            .touching("resource", resource.id.to_string(), resource.title))
        }
        mos_core::ActionArgs::TimeStart {
            project,
            activity,
            description,
        } => {
            let project_id = resolve_project_id(&state, project)?;
            let started = state.tracking.start_timer(mos_core::StartTimer {
                project_id,
                description: description.clone(),
                activity_type: parse_activity(activity)?,
            })?;
            let _ = app.emit("timer-changed", "started");
            Ok(mos_core::ActionEffect::new(
                format!(
                    "Cronometro iniciado em {}.",
                    project_name(&state, started.project_id)
                ),
                // SEM desfazer, e a ausencia e a decisao certa: o inverso de
                // iniciar e descartar, e descartar joga tempo fora. A ADR-035
                // diz que desfazer arquiva e nunca destroi, e um cronometro
                // recem-iniciado se resolve com Pausar ou Encerrar na tela.
                None,
            )
            .touching(
                "project",
                started.project_id.to_string(),
                project_name(&state, started.project_id),
            ))
        }
        mos_core::ActionArgs::TimeStop => {
            let entry = state.tracking.stop_timer()?;
            let _ = app.emit("timer-changed", "stopped");
            Ok(mos_core::ActionEffect::new(
                format!(
                    "Sessao de {} gravada em {}.",
                    spoken_duration(entry.duration_seconds),
                    project_name(&state, entry.project_id)
                ),
                Some(mos_core::UndoStep::TrashTimeEntry {
                    id: entry.id.to_string(),
                }),
            )
            .touching(
                "timeEntry",
                entry.id.to_string(),
                project_name(&state, entry.project_id),
            ))
        }
        mos_core::ActionArgs::TimeRecord {
            project,
            minutes,
            day,
            activity,
            description,
        } => {
            let project_id = resolve_project_id(&state, project)?;
            // A taxa vem do Project AGORA e vira snapshot, igual ao lancamento
            // feito pela tela: e a melhor aproximacao para uma hora lembrada
            // depois, e usar outro caminho aqui daria valores diferentes para a
            // mesma hora conforme quem a lancou.
            let rate = state
                .tracking
                .project_tracking()?
                .into_iter()
                .find(|entry| entry.project_id == project_id)
                .map(|entry| entry.hourly_rate_cents)
                .unwrap_or(0);
            let entry = state.tracking.record(mos_core::NewTimeEntry {
                project_id,
                started_at: moment_for_day(day)?,
                ended_at: None,
                duration_seconds: minutes * 60,
                idle_seconds: 0,
                description: description.clone(),
                activity_type: parse_activity(activity)?,
                billable: true,
                hourly_rate_snapshot_cents: rate,
                // `Manual` como qualquer hora lembrada depois. Hora proposta
                // pelo Hermes nao e mais medida que hora digitada na tela.
                source: mos_core::EntrySource::Manual,
            })?;
            let _ = app.emit("data-changed", "tracking");
            Ok(mos_core::ActionEffect::new(
                format!(
                    "{} lancadas em {}.",
                    spoken_duration(entry.duration_seconds),
                    project_name(&state, entry.project_id)
                ),
                Some(mos_core::UndoStep::TrashTimeEntry {
                    id: entry.id.to_string(),
                }),
            )
            .touching(
                "timeEntry",
                entry.id.to_string(),
                project_name(&state, entry.project_id),
            ))
        }
        // ------------------------------------------------------ Daily Session
        //
        // As cinco passam pelos MESMOS servicos que a interface usa
        // (`crate::daily`), e nao por SQL proprio. E a invariante da ADR-024: a
        // acao do Hermes obedece as mesmas regras que a acao do usuario, e nao
        // ha um segundo caminho que pudesse divergir.
        mos_core::ActionArgs::DayStart {
            main,
            main_ref,
            secondaries,
            note,
        } => {
            // O vinculo do principal e resolvido AQUI, e nao no dominio: so
            // este lado conhece o banco. Sem ele, um dia montado pelo Hermes
            // teria objetivos de texto solto e a conclusao automatica nunca
            // dispararia.
            let (link_kind, link_id) = match main_ref.trim() {
                "" => (String::new(), String::new()),
                referencia => match resolve_task(&state, referencia) {
                    Ok(task) => ("task".to_owned(), task.id.to_string()),
                    // Task nao achada NAO e erro: a referencia pode ser um
                    // Project. So depois de os dois falharem e que a proposta
                    // cai — e ai a mensagem vem do Project, que e o tipo mais
                    // amplo e o palpite mais provavel de quem escreveu.
                    Err(_) => {
                        let id = resolve_project(&state, referencia)?.ok_or_else(|| {
                            CoreError::new(
                                mos_core::ErrorCode::NotFound,
                                format!("Nao achei Task nem Project para \"{referencia}\"."),
                                false,
                            )
                        })?;
                        ("project".to_owned(), id)
                    }
                },
            };

            let input = mos_core::StartDayInput {
                main: Some(mos_core::ObjectiveDraft {
                    title: main.clone(),
                    link_kind,
                    link_id,
                    ..Default::default()
                }),
                secondaries: secondaries
                    .iter()
                    .map(|titulo| mos_core::ObjectiveDraft {
                        title: titulo.clone(),
                        ..Default::default()
                    })
                    .collect(),
                note: note.clone(),
            };
            let hoje = crate::daily::iniciar(app, &input)?;
            let (feitos, total) = hoje.progress();
            let _ = feitos;
            let sessao = hoje
                .session
                .as_ref()
                .map(|sessao| sessao.id.to_string())
                .unwrap_or_default();
            Ok(mos_core::ActionEffect::new(
                format!(
                    "Dia iniciado com {total} {}. Principal: {main}.",
                    if total == 1 { "objetivo" } else { "objetivos" }
                ),
                // SEM desfazer, e a ausencia e a decisao. O inverso de comecar o
                // dia nao e apagar o dia: seria destruir o unico registro de que
                // ele existiu, e todo Undo que o M/OS oferece e restauracao de
                // estado (ADR-035). Quem comecou por engano encerra — que e uma
                // decisao, e nao um desfazer.
                None,
            )
            .touching("daily_session", sessao, hoje.day.to_string()))
        }
        mos_core::ActionArgs::DayAddObjective {
            title,
            priority,
            link,
        } => {
            let draft = match link {
                Some(alvo) => {
                    let (kind, id) = resolve_objective_link(&state, alvo)?;
                    mos_core::ObjectiveDraft {
                        title: title.clone(),
                        link_kind: kind,
                        link_id: id,
                        ..Default::default()
                    }
                }
                None => mos_core::ObjectiveDraft {
                    title: title.clone(),
                    ..Default::default()
                },
            };
            let prioridade = mos_core::ObjectivePriority::parse(priority)?;
            let hoje = state
                .daily
                .add_objective(&crate::daily::hoje(app), &draft, prioridade)?;
            let criado = hoje
                .objectives
                .iter()
                .find(|objetivo| objetivo.title == draft.title)
                .map(|objetivo| objetivo.id.to_string())
                .unwrap_or_default();
            let _ = app.emit("data-changed", "daily");
            Ok(mos_core::ActionEffect::new(
                format!(
                    "\"{title}\" entrou no dia como {}.",
                    if prioridade == mos_core::ObjectivePriority::Main {
                        "principal"
                    } else {
                        "secundário"
                    }
                ),
                (!criado.is_empty())
                    .then(|| mos_core::UndoStep::RemoveDailyObjective { id: criado.clone() }),
            )
            .touching("daily_objective", criado, title.clone()))
        }
        mos_core::ActionArgs::DaySetObjective { objective, status } => {
            // O estado anterior e lido ANTES da mudanca: depois nao ha de onde
            // tirar, e sem ele resolver seria a unica acao do dia sem volta.
            let alvo = crate::daily::resolver_objetivo(app, objective)?;
            let anterior = alvo.status;
            let destino = mos_core::ObjectiveStatus::parse(status)?;
            state.daily.set_objective_status(alvo.id, destino)?;
            let _ = app.emit("data-changed", "daily");
            Ok(mos_core::ActionEffect::new(
                format!(
                    "\"{}\" {}.",
                    alvo.title,
                    match destino {
                        mos_core::ObjectiveStatus::Completed => "concluído",
                        mos_core::ObjectiveStatus::CarriedOver => "vai para amanhã",
                        mos_core::ObjectiveStatus::Dropped => "abandonado",
                        mos_core::ObjectiveStatus::Pending => "voltou a pendente",
                    }
                ),
                Some(mos_core::UndoStep::RestoreObjectiveStatus {
                    id: alvo.id.to_string(),
                    status: anterior.as_str().to_owned(),
                }),
            )
            .touching("daily_objective", alvo.id.to_string(), alvo.title))
        }
        mos_core::ActionArgs::DaySetMain { objective } => {
            let alvo = crate::daily::resolver_objetivo(app, objective)?;
            let anterior = state
                .daily
                .today(&crate::daily::hoje(app))?
                .main()
                .map(|principal| principal.id.to_string());
            state.daily.set_main(alvo.id)?;
            let _ = app.emit("data-changed", "daily");
            Ok(mos_core::ActionEffect::new(
                format!("\"{}\" agora é o objetivo principal de hoje.", alvo.title),
                Some(mos_core::UndoStep::RestoreDailyMain {
                    // O anterior so entra quando NAO e o proprio: promover quem
                    // ja era principal e um no-op, e guardar ele como "anterior"
                    // faria o desfazer se promover de volta e parecer que fez
                    // algo.
                    previous_id: anterior.filter(|id| *id != alvo.id.to_string()),
                    demote_id: alvo.id.to_string(),
                }),
            )
            .touching("daily_objective", alvo.id.to_string(), alvo.title))
        }
        mos_core::ActionArgs::DayEnd { mood, summary } => {
            let hoje = state.daily.today(&crate::daily::hoje(app))?;
            let sessao = hoje.session.as_ref().map(|sessao| sessao.id).ok_or_else(|| {
                CoreError::new(
                    mos_core::ErrorCode::InvalidInput,
                    "O dia ainda nao comecou, entao nao ha o que encerrar.",
                    false,
                )
            })?;
            let (feitos, total) = hoje.progress();
            // Os pendentes ficam PENDENTES: o Hermes nao decide destino de
            // objetivo por ninguem. Eles reaparecem no carry-over de amanha, que
            // e onde a pessoa escolhe.
            let input = mos_core::EndDayInput {
                resolutions: Vec::new(),
                mood: mood.clone(),
                summary: summary.clone(),
            };
            crate::daily::encerrar(app, Some(sessao), &input)?;
            Ok(mos_core::ActionEffect::new(
                format!("Dia encerrado. {feitos} de {total} objetivos concluídos."),
                Some(mos_core::UndoStep::ReopenDay {
                    session_id: sessao.to_string(),
                }),
            )
            .touching("daily_session", sessao.to_string(), hoje.day.to_string()))
        }
        mos_core::ActionArgs::MFinanceCreateBill {
            amount_cents,
            description,
            due_day,
            is_recurring,
        } => {
            let message = crate::finance::execute_create_bill(
                *amount_cents,
                description,
                *due_day,
                *is_recurring,
            )
            .await
            .map_err(|error| CoreError::new(mos_core::ErrorCode::Io, error, true))?;
            Ok(mos_core::ActionEffect::new(
                message,
                // Sem desfazer: o M/OS nao tem um comando de "apagar conta" no
                // M-Finance, e inventar um so para o Undo seria dar ao Hermes um
                // poder que a Action API (Fase 3 da spec) nao expoe. Corrigir uma
                // conta criada por engano e manual, dentro do proprio M-Finance —
                // igual e como as outras contas de la sempre foram corrigidas.
                None,
            ))
        }
    }
}

/// O mesmo `resolve_project`, ja tipado.
///
/// As acoes de tempo precisam de `ProjectId` e nao de texto, e a ambiguidade
/// ("bate com tres Projects") continua sendo recusada la — um cronometro
/// iniciado no Project errado e hora que vai para a fatura errada.
fn resolve_project_id(state: &AppState, name: &str) -> Result<mos_core::ProjectId, CoreError> {
    let id = resolve_project(state, name)?.ok_or_else(|| {
        CoreError::new(
            mos_core::ErrorCode::NotFound,
            format!("Nao achei um Project chamado \"{name}\"."),
            false,
        )
    })?;
    mos_core::ProjectId::parse(&id)
}

/// Vazio vira `other`: a atividade e opcional, e recusar por omissao faria o
/// Hermes ter de adivinhar um campo que o usuario nao disse.
fn parse_activity(value: &str) -> Result<mos_core::ActivityType, CoreError> {
    if value.is_empty() {
        return Ok(mos_core::ActivityType::Other);
    }
    mos_core::ActivityType::parse(value)
}

/// O nome do Project, para o recibo falar como o usuario fala.
fn project_name(state: &AppState, id: mos_core::ProjectId) -> String {
    state
        .work
        .projects(true)
        .ok()
        .and_then(|projects| projects.into_iter().find(|project| project.id == id))
        .map(|project| project.name)
        .unwrap_or_else(|| "Project".to_owned())
}

/// `2h30` ou `45min`, para o recibo.
fn spoken_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h{minutes:02}")
    } else {
        format!("{minutes}min")
    }
}

/// O instante de um dia `AAAA-MM-DD`, ao meio-dia local.
///
/// Meio-dia, e nao meia-noite: uma hora lancada a meia-noite fica na fronteira
/// entre dois dias, e qualquer diferenca de fuso a joga para o dia anterior. Ao
/// meio-dia sobram doze horas de folga para cada lado.
fn moment_for_day(day: &str) -> Result<time::OffsetDateTime, CoreError> {
    if day.is_empty() {
        return Ok(time::OffsetDateTime::now_utc());
    }
    mos_core::parse_moment(&format!("{day}T12:00:00Z"))
}

/// Desfaz uma acao executada.
///
/// Vive num comando proprio, e nao dentro de `action_resolve`, porque o desfazer
/// acontece depois — na janela do recibo, quando o usuario le o que aconteceu e
/// decide que nao era aquilo.
#[tauri::command]
pub async fn action_undo<R: Runtime>(
    app: AppHandle<R>,
    step: mos_core::UndoStep,
) -> Result<(), CoreError> {
    let services = app.state::<AppState>();
    match step {
        mos_core::UndoStep::ArchiveCapture { id } => {
            services.captures.archive(&id)?;
        }
        mos_core::UndoStep::ArchiveTask { id } => {
            services.work.set_task_archived(&id, true)?;
        }
        mos_core::UndoStep::ArchiveProject { id } => {
            services.work.set_project_archived(&id, true)?;
        }
        mos_core::UndoStep::ArchiveResource { id } => {
            services
                .memory
                .set_resource_lifecycle(&id, mos_core::LifecycleState::Archived)?;
        }
        mos_core::UndoStep::RestoreTaskState { id, state } => {
            services
                .work
                .set_task_state(&id, mos_core::TaskState::parse(&state)?)?;
        }
        mos_core::UndoStep::RestoreTaskProject { id, project_id } => {
            let task = services.work.task(&id)?;
            services.work.update_task(mos_core::UpdateTaskInput {
                id,
                title: task.title,
                description: task.description,
                project_id,
            })?;
        }
        mos_core::UndoStep::CancelReminder { id } => {
            services.attention.transition(
                mos_core::ReminderId::parse(&id)?,
                mos_core::Transition::Cancel,
            )?;
            crate::attention::poke(&app);
        }
        mos_core::UndoStep::UndoCaptureToTask {
            capture_id,
            task_id,
        } => {
            services.work.set_task_archived(&task_id, true)?;
            // A Capture volta para a Inbox, e nao para o arquivo: o que se
            // desfez foi a DECISAO sobre ela, e o lugar de uma Capture ainda
            // nao decidida e a Inbox.
            services.captures.move_to_inbox(&capture_id)?;
        }
        mos_core::UndoStep::TrashTimeEntry { id } => {
            services
                .tracking
                .trash(mos_core::TimeEntryId::parse(&id)?)?;
        }
        mos_core::UndoStep::UndoMeetingInsight {
            insight_id,
            task_id,
            reminder_id,
        } => {
            // A ordem e deliberada: primeiro tirar da vista o que TOCA. Um
            // lembrete que dispara enquanto o desfazer roda avisaria sobre uma
            // Task que a pessoa acabou de dizer que nao queria.
            if let Some(reminder_id) = reminder_id {
                services.attention.transition(
                    mos_core::ReminderId::parse(&reminder_id)?,
                    mos_core::Transition::Cancel,
                )?;
            }
            services.work.set_task_archived(&task_id, true)?;
            // Por ultimo o item volta a ser oferecido — e isso e o ponto: quem
            // desfez provavelmente quer refazer diferente.
            services.meetings.reopen_insight(&insight_id)?;
        }
        mos_core::UndoStep::RemoveDailyObjective { id } => {
            services
                .daily
                .remove_objective(mos_core::DailyObjectiveId::parse(&id)?)?;
        }
        mos_core::UndoStep::RestoreObjectiveStatus { id, status } => {
            services.daily.set_objective_status(
                mos_core::DailyObjectiveId::parse(&id)?,
                mos_core::ObjectiveStatus::parse(&status)?,
            )?;
        }
        mos_core::UndoStep::RestoreDailyMain {
            previous_id,
            demote_id,
        } => match previous_id {
            // Promover o anterior REBAIXA o novo na mesma transacao — e o que
            // `set_main_objective` garante —, entao um passo basta.
            Some(previous_id) => {
                services
                    .daily
                    .set_main(mos_core::DailyObjectiveId::parse(&previous_id)?)?;
            }
            // Nao havia principal antes: desfazer e REBAIXAR, e nao promover
            // ninguem. Sem este braco, um dia que nao tinha principal ganharia
            // um pelo desfazer.
            None => {
                let alvo = mos_core::DailyObjectiveId::parse(&demote_id)?;
                services.daily.set_secondary(alvo)?;
            }
        },
        mos_core::UndoStep::ReopenDay { session_id } => {
            services
                .daily
                .reopen(mos_core::DailySessionId::parse(&session_id)?)?;
        }
        mos_core::UndoStep::UndoVoiceAction {
            capture_id,
            task_id,
            reminder_id,
        } => {
            // Mesma ordem do desfazer de reuniao: primeiro sai de vista o que
            // TOCA. Um lembrete que dispara no meio do desfazer avisaria sobre
            // uma Task que a pessoa acabou de dizer que nao queria.
            if let Some(reminder_id) = reminder_id {
                services.attention.transition(
                    mos_core::ReminderId::parse(&reminder_id)?,
                    mos_core::Transition::Cancel,
                )?;
            }
            services.work.set_task_archived(&task_id, true)?;
            // A fala volta para a Inbox. Ela nao e apagada: desfazer a acao nao
            // desfaz o ter falado, e o lugar de uma fala ainda nao decidida e a
            // Inbox.
            services.captures.move_to_inbox(&capture_id)?;
        }
    }
    let _ = app.emit("data-changed", "undo");
    Ok(())
}

/// Acha uma entidade a partir do que o modelo escreveu.
///
/// # O que mudou aqui, e por que
///
/// Antes cada acao tinha a propria comparacao, e todas faziam a mesma coisa:
/// `to_lowercase().contains()` sobre o nome. Isso resolvia "Minarum" e nao
/// resolvia mais nada — nem o id que o proprio M/OS acabara de mandar no bloco
/// de candidatos, nem um titulo escrito com acento diferente.
///
/// Agora a comparacao e uma so, em `mos_core::resolve`, e ela le em degraus:
/// id inteiro, prefixo de id, titulo exato, comeco de titulo, pedaco de titulo.
/// O primeiro degrau que acerta decide. E a diferenca entre o Hermes citar
/// `7c3e2b19` — o id que ele leu nos candidatos — e o M/OS responder "nao achei".
///
/// Ambiguidade continua RECUSANDO em vez de escolher o primeiro: "Escadas"
/// batendo em dois Projects e exatamente o caso onde adivinhar cria a Task no
/// lugar errado, e o erro so aparece dias depois.
fn resolve_project(state: &AppState, name: &str) -> Result<Option<String>, CoreError> {
    let projects = state.work.projects(false)?;
    let found = mos_core::resolve(
        &projects,
        name,
        |project| project.id.to_string(),
        |project| project.name.clone(),
    );
    if let Some(error) = mos_core::resolution_error(
        &found,
        mos_core::EntityKind::Project,
        name,
        |project: &mos_core::Project| project.name.clone(),
    ) {
        return Err(error);
    }
    Ok(found.one().map(|project| project.id.to_string()))
}

/// Devolve a Task inteira, e nao so o id: quem chama precisa do estado ANTERIOR
/// para montar o desfazer, e ele ja esta aqui. Buscar de novo depois custaria
/// uma segunda varredura e leria um estado que a mudanca ja alterou.
fn resolve_task(state: &AppState, title: &str) -> Result<mos_core::Task, CoreError> {
    let tasks = state.work.tasks(false)?;
    let found = mos_core::resolve(
        &tasks,
        title,
        |task| task.id.to_string(),
        |task| task.title.clone(),
    );
    match mos_core::resolution_error(
        &found,
        mos_core::EntityKind::Task,
        title,
        |task: &mos_core::Task| task.title.clone(),
    ) {
        Some(error) => Err(error),
        None => Ok(found.one().expect("sem erro ha exatamente um").clone()),
    }
}

/// A Capture, pelo id ou por um pedaco do que esta escrito nela.
fn resolve_capture(state: &AppState, reference: &str) -> Result<mos_core::Capture, CoreError> {
    // A Inbox primeiro, e a base inteira depois. Nao e otimizacao: "essa
    // captura" quase sempre quer dizer uma das que ainda esperam decisao, e
    // procurar na Inbox primeiro faz o pedaco de texto que tambem bate com
    // Captures antigas resolver na que importa.
    let inbox = state.captures.inbox(200)?;
    let found = mos_core::resolve(
        &inbox,
        reference,
        |capture| capture.id.to_string(),
        |capture| capture.content.clone(),
    );
    if let mos_core::Resolved::One(capture) = found {
        return Ok(capture.clone());
    }

    let todas = state.captures.recent(200)?;
    let found = mos_core::resolve(
        &todas,
        reference,
        |capture| capture.id.to_string(),
        |capture| capture.content.clone(),
    );
    match mos_core::resolution_error(
        &found,
        mos_core::EntityKind::Capture,
        reference,
        |capture: &mos_core::Capture| capture.content.clone(),
    ) {
        Some(error) => Err(error),
        None => Ok(found.one().expect("sem erro ha exatamente um").clone()),
    }
}

/// O lembrete, pelo id ou pelo titulo.
///
/// So os abertos. Um lembrete ja concluido nao e o que alguem quer dizer com
/// "cancela aquele lembrete", e incluir os fechados faria o titulo repetido de
/// um lembrete recorrente virar ambiguidade toda vez.
fn resolve_reminder(state: &AppState, reference: &str) -> Result<mos_core::Reminder, CoreError> {
    let abertos = state.attention.open()?;
    let found = mos_core::resolve(
        &abertos,
        reference,
        |reminder| reminder.id.to_string(),
        |reminder| reminder.title.clone(),
    );
    match mos_core::resolution_error(
        &found,
        mos_core::EntityKind::Reminder,
        reference,
        |reminder: &mos_core::Reminder| reminder.title.clone(),
    ) {
        Some(error) => Err(error),
        None => Ok(found.one().expect("sem erro ha exatamente um").clone()),
    }
}

/// O vinculo de um objetivo, resolvido para as duas colunas que o banco guarda.
///
/// Reaproveita os mesmos resolvedores do `resolve_target`, mas com o conjunto
/// mais estreito de `LinkKind`: um objetivo do dia ligado a uma Conversa ou a um
/// App nao quer dizer nada, e recusar aqui e mais barato que descobrir na tela.
fn resolve_objective_link(
    state: &AppState,
    target: &mos_core::TargetRef,
) -> Result<(String, String), CoreError> {
    match target.kind.as_str() {
        "task" => Ok((
            "task".to_owned(),
            resolve_task(state, &target.reference)?.id.to_string(),
        )),
        "project" => {
            let id = resolve_project(state, &target.reference)?.ok_or_else(|| {
                CoreError::new(
                    mos_core::ErrorCode::NotFound,
                    format!("Nao achei Project para \"{}\".", target.reference),
                    false,
                )
            })?;
            Ok(("project".to_owned(), id))
        }
        "capture" => Ok((
            "capture".to_owned(),
            resolve_capture(state, &target.reference)?.id.to_string(),
        )),
        outro => Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            format!("`{outro}` nao e um tipo de vinculo de objetivo do dia."),
            false,
        )),
    }
}

/// O alvo de um lembrete, resolvido para o enum do dominio.
///
/// Devolve tambem o rotulo, porque o recibo fala como o usuario fala:
/// "vinculado a Task Enviar bases" e util, "vinculado a 7c3e2b19" nao e.
fn resolve_target(
    state: &AppState,
    target: &mos_core::TargetRef,
) -> Result<(mos_core::ReminderTarget, String), CoreError> {
    match target.kind.as_str() {
        "task" => {
            let task = resolve_task(state, &target.reference)?;
            Ok((mos_core::ReminderTarget::Task(task.id), task.title))
        }
        "project" => {
            let id = resolve_project(state, &target.reference)?.ok_or_else(|| {
                CoreError::new(
                    mos_core::ErrorCode::NotFound,
                    format!("Nao achei Project para \"{}\".", target.reference),
                    false,
                )
            })?;
            let id = mos_core::ProjectId::parse(&id)?;
            Ok((
                mos_core::ReminderTarget::Project(id),
                project_name(state, id),
            ))
        }
        "capture" => {
            let capture = resolve_capture(state, &target.reference)?;
            Ok((
                mos_core::ReminderTarget::Capture(capture.id),
                capture.content,
            ))
        }
        "resource" => {
            let resources = state.memory.resources(false)?;
            let found = mos_core::resolve(
                &resources,
                &target.reference,
                |resource| resource.id.to_string(),
                |resource| resource.title.clone(),
            );
            match mos_core::resolution_error(
                &found,
                mos_core::EntityKind::Resource,
                &target.reference,
                |resource: &mos_core::Resource| resource.title.clone(),
            ) {
                Some(error) => Err(error),
                None => {
                    let resource = found.one().expect("sem erro ha exatamente um");
                    Ok((
                        mos_core::ReminderTarget::Resource(resource.id),
                        resource.title.clone(),
                    ))
                }
            }
        }
        outro => Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            format!("`{outro}` nao e um tipo de alvo de lembrete."),
            false,
        )),
    }
}

/// O desfecho de uma proposta, com o caminho de volta quando existe.
///
/// A mensagem sozinha nao bastava: ela guarda que a acao foi executada, mas nao
/// carrega como reverte-la. O Undo vive na janela do recibo, e nao no cartao da
/// conversa, pelo mesmo motivo que vive assim no resto do app — oferecer
/// "desfazer" numa acao de semana passada seria surpresa, nao seguranca.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResolution {
    pub message: Message,
    /// Vazio quando nao houve execucao.
    pub receipt: String,
    pub undo: Option<mos_core::UndoStep>,
}

/// Resolve uma proposta: executa ou cancela, e grava o desfecho na mensagem.
#[tauri::command]
pub async fn action_resolve<R: Runtime>(
    app: AppHandle<R>,
    message_id: String,
    raw: String,
    approved: bool,
) -> Result<ActionResolution, CoreError> {
    let service = app.state::<AppState>().conversations.clone();
    let message = service.message(&message_id)?;

    let (status, outcome, undo, audit) = if !approved {
        (
            ProposalStatus::Cancelled,
            "Cancelado por você.".to_owned(),
            None,
            None,
        )
    } else {
        // O relogio e lido AGORA, e nao no instante da proposta: uma proposta de
        // "hoje as 20:30" confirmada as 20:31 aponta para o passado, e recusar
        // ali e o comportamento certo. Congelar o relogio da proposta agendaria
        // um lembrete que ja venceu.
        let now_local = crate::surface::now_local(&app);
        let resolved = match mos_core::parse_action_at(&raw, now_local) {
            Ok(args) => run_action(&app, &args).await,
            Err(error) => Err(error),
        };
        match resolved {
            Ok(effect) => {
                // O rastro e gravado JUNTO com o desfecho, na mesma escrita.
                // Duas escritas dariam um instante em que a acao consta como
                // executada e ninguem sabe sobre o que — e seria justamente o
                // instante de uma queda deixar a auditoria incompleta.
                let audit = mos_core::ActionAudit {
                    executed_at: now_local
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                    entities: effect.entities.clone(),
                    undo: effect.undo.clone(),
                };
                (
                    ProposalStatus::Executed,
                    effect.message,
                    effect.undo,
                    Some(audit),
                )
            }
            Err(error) => (ProposalStatus::Failed, error.message, None, None),
        }
    };

    let parts = message
        .parts
        .into_iter()
        .map(|part| match part.body {
            PartBody::ActionProposal {
                raw: ref found,
                ref preview,
                ..
            } if found == &raw => PartBody::ActionProposal {
                raw: raw.clone(),
                preview: preview.clone(),
                status,
                outcome: outcome.clone(),
                audit: audit.clone(),
            },
            body => body,
        })
        .collect();

    let updated = service.attach_parts(&message_id, MessageStatus::Complete, parts)?;
    if status == ProposalStatus::Executed {
        // A Home, a Inbox e o Kanban precisam refletir o que acabou de nascer.
        let _ = app.emit("data-changed", "action");
    }
    announce_message(&app, &updated);
    Ok(ActionResolution {
        // O recibo so existe quando algo aconteceu. Cancelar e recusar ja se
        // explicam dentro do proprio cartao, na conversa — repetir aquilo num
        // aviso flutuante seria ruido sobre uma decisao que o usuario acabou de
        // tomar.
        receipt: if status == ProposalStatus::Executed {
            outcome
        } else {
            String::new()
        },
        message: updated,
        undo,
    })
}

/// Projeta o historico da VPS em mensagens do M/OS.
///
/// Uma linha `tool` do historico nao vira mensagem: ela e uma parte, e o papel
/// `tool` nao existe no modelo local. Anexar a mensagem anterior preserva a
/// ordem sem inventar um papel que o dominio recusa.
pub fn project_history(
    conversation_id: mos_core::ConversationId,
    messages: &[HistoryMessage],
) -> Vec<mos_core::NewMessage> {
    let mut projected: Vec<mos_core::NewMessage> = Vec::new();

    for message in messages {
        match message.role.as_str() {
            "tool" => {
                let part = PartBody::ToolRun {
                    name: if message.tool_name.is_empty() {
                        "tool".to_owned()
                    } else {
                        message.tool_name.clone()
                    },
                    state: ToolRunState::Success,
                    detail: message.content.clone(),
                };
                match projected.last_mut() {
                    Some(previous) => previous.parts.push(part),
                    None => {
                        let mut seed = mos_core::NewMessage::pending_assistant(conversation_id);
                        seed.status = MessageStatus::Complete;
                        seed.parts.push(part);
                        projected.push(seed);
                    }
                }
            }
            role => {
                if message.content.trim().is_empty() {
                    continue;
                }
                let mut new = mos_core::NewMessage::pending_assistant(conversation_id);
                new.role = match role {
                    "user" => mos_core::MessageRole::User,
                    "system" => mos_core::MessageRole::System,
                    _ => mos_core::MessageRole::Assistant,
                };
                new.status = MessageStatus::Complete;
                new.parts.push(PartBody::Text {
                    text: message.content.clone(),
                });
                projected.push(new);
            }
        }
    }

    projected
}

// ------------------------------------------------------------------- busca

/// Teto de resultados por termo.
///
/// Baixo de proposito. Um termo comum — "projeto", quando escapa do filtro de
/// ruido — casaria com metade da base, e trazer cem linhas dele empurraria para
/// fora os poucos acertos do termo que realmente identifica a entidade.
const HITS_POR_TERMO: usize = 8;

/// Um candidato e quantos termos da frase bateram nele.
struct Pontuado {
    candidate: mos_core::Candidate,
    /// Quantos termos distintos acertaram esta entidade.
    termos: usize,
    /// Ordem em que apareceu. Desempata sem sortear.
    ordem: usize,
}

/// Encurta um texto para caber numa linha do prompt.
///
/// O corte e por caractere e nao por byte: cortar no meio de um `ç` produziria
/// um bloco de contexto invalido, e o erro apareceria como uma resposta
/// estranha em vez de como um erro.
fn resumo(texto: &str, teto: usize) -> String {
    let limpo = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    if limpo.chars().count() <= teto {
        return limpo;
    }
    let cortado: String = limpo.chars().take(teto).collect();
    format!("{cortado}…")
}

/// Le o M/OS procurando o que a frase do usuario cita.
///
/// # Por que uma varredura por termo, e nao uma so
///
/// O FTS do M/OS junta os termos com `AND` (`to_fts_query`), o que e certo para
/// a caixa de busca — quem digita tres palavras quer as tres. Aqui seria errado:
/// a frase e uma frase inteira, e exigir que uma Task contenha "enviar", "tipos",
/// "bases", "faltantes" E "victor" ao mesmo tempo nao acharia a Task chamada
/// "Enviar tipos de bases faltantes p/ Victor" — falta um "para", sobra um "p/".
///
/// Uma varredura por termo da a semantica de OU, e a contagem de termos que
/// bateram vira o ranking. A entidade que aparece em quatro buscas diferentes e
/// mais provavelmente a citada do que a que apareceu em uma.
pub fn candidates_for<R: Runtime>(app: &AppHandle<R>, text: &str) -> Vec<mos_core::Candidate> {
    let termos = mos_core::search_terms(text);
    if termos.is_empty() {
        return Vec::new();
    }
    let state = app.state::<AppState>();
    let mut encontrados: Vec<Pontuado> = Vec::new();

    let registrar = |candidate: mos_core::Candidate, encontrados: &mut Vec<Pontuado>| {
        match encontrados
            .iter_mut()
            .find(|existente| existente.candidate.id == candidate.id)
        {
            Some(existente) => existente.termos += 1,
            None => {
                let ordem = encontrados.len();
                encontrados.push(Pontuado {
                    candidate,
                    termos: 1,
                    ordem,
                });
            }
        }
    };

    for termo in &termos {
        // Arquivados ficam de fora: agir sobre o que ja saiu de vista e
        // exatamente o tipo de surpresa que a confirmacao existe para evitar.
        if let Ok(items) = state.work.search(termo, false) {
            for item in items.into_iter().take(HITS_POR_TERMO) {
                if let Some(candidate) = candidate_of(&item) {
                    registrar(candidate, &mut encontrados);
                }
            }
        }
        if let Ok(resources) = state.memory.search(termo, false, HITS_POR_TERMO) {
            for resource in resources {
                registrar(
                    mos_core::Candidate {
                        kind: mos_core::EntityKind::Resource,
                        id: resource.id.to_string(),
                        label: resumo(&resource.title, 70),
                        detail: resource.kind.as_str().to_owned(),
                    },
                    &mut encontrados,
                );
            }
        }
        if let Ok(meetings) = state.meetings.search(termo, HITS_POR_TERMO) {
            for meeting in meetings {
                registrar(
                    mos_core::Candidate {
                        kind: mos_core::EntityKind::Meeting,
                        id: meeting.id.to_string(),
                        label: resumo(&meeting.title, 70),
                        detail: mos_core::spoken_moment(meeting.started_at),
                    },
                    &mut encontrados,
                );
            }
        }
    }

    // Os lembretes abertos entram por comparacao direta, e nao por FTS: eles
    // nao tem indice de busca, e sao poucos por definicao — um usuario com
    // duzentos lembretes abertos tem um problema que nao e de indice.
    if let Ok(abertos) = state.attention.open() {
        for reminder in abertos {
            let alvo = mos_core::normalize(&reminder.title);
            for termo in &termos {
                if alvo.contains(termo.as_str()) {
                    registrar(
                        mos_core::Candidate {
                            kind: mos_core::EntityKind::Reminder,
                            id: reminder.id.to_string(),
                            label: resumo(&reminder.title, 70),
                            detail: reminder
                                .next_due_at
                                .map(mos_core::spoken_moment)
                                .unwrap_or_else(|| reminder.status.as_str().to_owned()),
                        },
                        &mut encontrados,
                    );
                }
            }
        }
    }

    encontrados.sort_by(|esquerda, direita| {
        direita
            .termos
            .cmp(&esquerda.termos)
            .then(esquerda.ordem.cmp(&direita.ordem))
    });
    encontrados
        .into_iter()
        .take(mos_core::MAX_CANDIDATES)
        .map(|pontuado| pontuado.candidate)
        .collect()
}

/// Projeta um resultado de busca em candidato.
///
/// `App` fica de fora: ele nao e uma entidade sobre a qual o catalogo de acoes
/// saiba agir, e um candidato que nao pode virar acao so ocupa linha no prompt.
fn candidate_of(item: &mos_core::SearchItem) -> Option<mos_core::Candidate> {
    Some(match item {
        mos_core::SearchItem::Task { task, project } => mos_core::Candidate {
            kind: mos_core::EntityKind::Task,
            id: task.id.to_string(),
            label: resumo(&task.title, 70),
            detail: match project {
                Some(project) => format!("{} · {}", task.state.as_str(), project.name),
                None => task.state.as_str().to_owned(),
            },
        },
        // A Capture que ja virou Task aparece como a TASK: e nela que se age, e
        // oferecer as duas convidaria o modelo a criar uma segunda Task a
        // partir da Capture que ja tem uma.
        mos_core::SearchItem::Capture {
            capture,
            derived_task,
            project,
        } => match derived_task {
            Some(task) => mos_core::Candidate {
                kind: mos_core::EntityKind::Task,
                id: task.id.to_string(),
                label: resumo(&task.title, 70),
                detail: match project {
                    Some(project) => format!("{} · {}", task.state.as_str(), project.name),
                    None => task.state.as_str().to_owned(),
                },
            },
            None => mos_core::Candidate {
                kind: mos_core::EntityKind::Capture,
                id: capture.id.to_string(),
                label: resumo(&capture.content, 70),
                detail: capture.processing_state.as_str().to_owned(),
            },
        },
        mos_core::SearchItem::Project { project } => mos_core::Candidate {
            kind: mos_core::EntityKind::Project,
            id: project.id.to_string(),
            label: resumo(&project.name, 70),
            detail: resumo(&project.description, 50),
        },
        mos_core::SearchItem::Workspace { workspace } => mos_core::Candidate {
            kind: mos_core::EntityKind::Workspace,
            id: workspace.id.to_string(),
            label: resumo(&workspace.name, 70),
            detail: String::new(),
        },
        mos_core::SearchItem::Meeting { meeting, .. } => mos_core::Candidate {
            kind: mos_core::EntityKind::Meeting,
            id: meeting.id.to_string(),
            label: resumo(&meeting.title, 70),
            detail: mos_core::spoken_moment(meeting.started_at),
        },
        // O objetivo do dia entra como candidato PORQUE existe acao sobre ele:
        // concluir, promover a principal, levar para amanha. O `detail` leva a
        // data, e ela e o que distingue dois dias que escreveram a mesma frase.
        mos_core::SearchItem::DailyObjective { objective, day } => mos_core::Candidate {
            kind: mos_core::EntityKind::DailyObjective,
            id: objective.id.to_string(),
            label: resumo(&objective.title, 70),
            detail: format!("{} · {}", day, objective.status.as_str()),
        },
        // A disciplina e citavel porque o Hermes precisa poder responder "como
        // estou em Estatica?" apontando para ela. A prova leva a DATA no
        // detalhe, que e o que a pergunta "quando e minha proxima prova?"
        // procura; a atividade leva o prazo pelo mesmo motivo.
        mos_core::SearchItem::Subject { subject } => mos_core::Candidate {
            kind: mos_core::EntityKind::Subject,
            id: subject.id.to_string(),
            label: resumo(&subject.name, 70),
            detail: resumo(&subject.code, 30),
        },
        mos_core::SearchItem::Exam { exam, subject } => mos_core::Candidate {
            kind: mos_core::EntityKind::Exam,
            id: exam.id.to_string(),
            label: resumo(&format!("{subject} — {}", exam.name), 70),
            // A DECISAO entra no detalhe, e nao so a data. Sem ela o Hermes nao
            // consegue responder "o que eu ja marquei como nao vou fazer?" nem
            // "o que ainda nao planejei?" — as duas perguntas que a camada
            // operacional criou.
            detail: detalhe_academico(
                mos_core::spoken_moment(exam.at),
                exam.decision,
                exam.planned_at,
            ),
        },
        mos_core::SearchItem::Assignment {
            assignment,
            subject,
        } => mos_core::Candidate {
            kind: mos_core::EntityKind::Assignment,
            id: assignment.id.to_string(),
            label: resumo(&format!("{subject} — {}", assignment.title), 70),
            detail: detalhe_academico(
                assignment
                    .due_at
                    .map(mos_core::spoken_moment)
                    .unwrap_or_else(|| assignment.status.as_str().to_owned()),
                assignment.decision,
                assignment.planned_at,
            ),
        },
        mos_core::SearchItem::App { .. } => return None,
    })
}

/// O detalhe de um compromisso academico para o Hermes.
///
/// Junta o quando com o que a pessoa resolveu. "vence sexta" sozinho nao
/// distingue o que ela ja entregou do que ela nem olhou, e as duas coisas
/// exigem respostas opostas.
fn detalhe_academico(
    quando: String,
    decision: mos_core::Decision,
    planned_at: Option<time::OffsetDateTime>,
) -> String {
    let estado = match decision {
        mos_core::Decision::Done => Some("marcada como entregue".to_owned()),
        mos_core::Decision::Skipped => Some("marcada como nao vou fazer".to_owned()),
        mos_core::Decision::None => planned_at
            .map(|quando| format!("planejada para {}", mos_core::spoken_moment(quando))),
    };
    match estado {
        Some(estado) => format!("{quando} · {estado}"),
        None => quando,
    }
}

/// Executa a busca que o modelo pediu pelo bloco `mos-query`.
///
/// Respeita o filtro de tipo quando ele veio: um pedido por `task` que devolve
/// Projects gastaria o unico salto disponivel com o que nao foi perguntado.
pub fn run_query<R: Runtime>(
    app: &AppHandle<R>,
    request: &mos_core::QueryRequest,
) -> Vec<mos_core::Candidate> {
    let achados = candidates_for(app, &request.search);
    if request.kinds.is_empty() {
        return achados;
    }
    achados
        .into_iter()
        .filter(|candidate| request.kinds.contains(&candidate.kind))
        .collect()
}

/// O bloco de contexto e o registro do que ele contem.
pub struct AssembledContext {
    /// Prefixo do prompt. Vazio quando nao ha contexto.
    pub block: String,
    /// Uma parte por contexto, com o que efetivamente foi enviado.
    pub parts: Vec<PartBody>,
}

/// Teto do bloco de contexto, em bytes.
///
/// O caminho A da ADR-028 nao tem segunda chance: o agente nao consegue pedir
/// mais dados no meio do turno, entao o que vai tem que caber de uma vez. Um
/// Project com trezentas Tasks nao pode empurrar a pergunta para fora da janela.
const CONTEXT_BUDGET: usize = 8_000;

fn parse_origin(value: &str) -> ContextOrigin {
    match value {
        "automatic" => ContextOrigin::Automatic,
        _ => ContextOrigin::Explicit,
    }
}

/// Le o M/OS e monta o bloco.
///
/// Somente leitura, pelos mesmos servicos que a UI usa — nunca SQL proprio e
/// nunca um caminho paralelo (ADR-028).
pub fn assemble_context<R: Runtime>(
    app: &AppHandle<R>,
    contexts: &[ContextInput],
) -> Result<AssembledContext, CoreError> {
    if contexts.is_empty() {
        return Ok(AssembledContext {
            block: String::new(),
            parts: Vec::new(),
        });
    }

    let state = app.state::<AppState>();
    let mut sections = Vec::new();
    let mut parts = Vec::new();
    let mut spent = 0usize;

    for context in contexts {
        let (entity, mut fields, body) = match context.entity.as_str() {
            "project" => {
                let project = state.work.project(&context.id)?;
                let tasks: Vec<_> = state
                    .work
                    .tasks(false)?
                    .into_iter()
                    .filter(|task| {
                        task.project_id.map(|id| id.to_string()).as_deref() == Some(&context.id)
                    })
                    .collect();
                let open: Vec<String> = tasks
                    .iter()
                    .filter(|task| task.state != mos_core::TaskState::Done)
                    .map(|task| format!("- [{}] {}", task.state.as_str(), task.title))
                    .collect();
                let body = format!(
                    "Project: {}\nDescricao: {}\nRepositorio: {}\nTasks abertas ({}):\n{}",
                    project.name,
                    if project.description.is_empty() {
                        "(sem descricao)"
                    } else {
                        &project.description
                    },
                    if project.repository.is_empty() {
                        "(nenhum)"
                    } else {
                        &project.repository
                    },
                    open.len(),
                    if open.is_empty() {
                        "(nenhuma)".to_owned()
                    } else {
                        open.join("\n")
                    }
                );
                (
                    ContextEntity::Project,
                    vec!["name", "description", "repository", "openTasks"],
                    body,
                )
            }
            "task" => {
                let task = state.work.task(&context.id)?;
                let body = format!(
                    "Task: {}\nEstado: {}\nDescricao: {}",
                    task.title,
                    task.state.as_str(),
                    if task.description.is_empty() {
                        "(sem descricao)"
                    } else {
                        &task.description
                    }
                );
                (
                    ContextEntity::Task,
                    vec!["title", "state", "description"],
                    body,
                )
            }
            "capture" => {
                let capture = state.captures.get(&context.id)?;
                (
                    ContextEntity::Capture,
                    vec!["content"],
                    format!("Capture: {}", capture.content),
                )
            }
            "resource" => {
                let resource = state.memory.resource(&context.id)?;
                let body = format!(
                    "Resource: {}\nURL: {}\nNota: {}",
                    resource.title,
                    if resource.url.is_empty() {
                        "(sem url)"
                    } else {
                        &resource.url
                    },
                    if resource.note.is_empty() {
                        "(sem nota)"
                    } else {
                        &resource.note
                    }
                );
                (ContextEntity::Resource, vec!["title", "url", "note"], body)
            }
            "workspace" => {
                let workspace = state.work.workspace(&context.id)?;
                let projects = state.work.workspace_projects(&context.id, false)?;
                let names: Vec<&str> = projects
                    .iter()
                    .map(|project| project.name.as_str())
                    .collect();
                let body = format!(
                    "Workspace: {}\nProjects: {}",
                    workspace.name,
                    if names.is_empty() {
                        "(nenhum)".to_owned()
                    } else {
                        names.join(", ")
                    }
                );
                (ContextEntity::Workspace, vec!["name", "projects"], body)
            }
            // A reuniao podia ser mencionada com `@` desde que a busca passou a
            // devolve-la, e caia no ramo da tela — o Hermes recebia "Tela atual
            // do M/OS: Reuniao com o Victor" e concluia que aquilo era uma
            // pagina. O resumo e o que a pessoa quer dizer ao anexar uma
            // reuniao; a transcricao inteira nao caberia no orcamento.
            "meeting" => {
                let meeting = state.meetings.meeting(&context.id)?;
                let resumo = state
                    .meetings
                    .analysis(&context.id)
                    .ok()
                    .flatten()
                    .map(|analysis| analysis.summary)
                    .unwrap_or_default();
                let body = format!(
                    "Reuniao: {}\nQuando: {}\nResumo: {}",
                    meeting.title,
                    mos_core::spoken_moment(meeting.started_at),
                    if resumo.is_empty() {
                        "(ainda sem resumo)"
                    } else {
                        &resumo
                    }
                );
                (
                    ContextEntity::Meeting,
                    vec!["title", "startedAt", "summary"],
                    body,
                )
            }
            // Tela atual: nao e entidade, e o label ja e a informacao.
            _ => (
                ContextEntity::Screen,
                vec!["screen"],
                format!("Tela atual do M/OS: {}", context.label),
            ),
        };

        // Orcamento: o que nao couber e cortado, e o corte fica registrado no
        // proprio registro — um contexto silenciosamente truncado seria pior que
        // um contexto ausente.
        let mut body = body;
        if spent + body.len() > CONTEXT_BUDGET {
            let room = CONTEXT_BUDGET.saturating_sub(spent);
            if room < 120 {
                continue;
            }
            body.truncate(
                body.char_indices()
                    .take_while(|(index, _)| *index < room)
                    .last()
                    .map(|(index, character)| index + character.len_utf8())
                    .unwrap_or(0),
            );
            body.push_str("\n(cortado por limite de contexto)");
            fields.push("truncated");
        }

        spent += body.len();
        parts.push(PartBody::ContextRef {
            origin: parse_origin(&context.origin),
            entity,
            id: context.id.clone(),
            label: context.label.clone(),
            fields: fields.into_iter().map(str::to_owned).collect(),
            bytes: body.len(),
        });
        sections.push(body);
    }

    let block = if sections.is_empty() {
        String::new()
    } else {
        format!(
            "[Contexto do M/OS, anexado pelo usuario]\n{}\n[Fim do contexto]\n\n",
            sections.join("\n\n")
        )
    };

    Ok(AssembledContext { block, parts })
}

fn conversations<R: Runtime>(app: &AppHandle<R>) -> ConversationService {
    app.state::<AppState>().conversations.clone()
}

/// Publica a mensagem gravada. O renderer troca o buffer de streaming por ela,
/// o que impede o texto da tela de divergir do texto do banco.
pub fn announce_message<R: Runtime>(app: &AppHandle<R>, message: &Message) {
    let _ = app.emit("hermes-message", message);
}

pub fn announce_conversation<R: Runtime>(app: &AppHandle<R>, conversation: &Conversation) {
    let _ = app.emit("hermes-conversation", conversation);
}

#[tauri::command]
pub fn conversation_list(
    state: State<'_, AppState>,
    include_archived: bool,
) -> Result<Vec<ConversationSummary>, CoreError> {
    state.conversations.list(include_archived)
}

#[tauri::command]
pub fn conversation_current(state: State<'_, AppState>) -> Result<Conversation, CoreError> {
    state.conversations.current_or_new()
}

#[tauri::command]
pub fn conversation_create(state: State<'_, AppState>) -> Result<Conversation, CoreError> {
    state.conversations.create()
}

#[tauri::command]
pub fn conversation_messages(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<Message>, CoreError> {
    state.conversations.messages(&id)
}

#[tauri::command]
pub fn conversation_rename(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> Result<Conversation, CoreError> {
    state.conversations.rename(&id, &title)
}

#[tauri::command]
pub fn conversation_set_archived(
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<Conversation, CoreError> {
    state.conversations.set_archived(&id, archived)
}

#[tauri::command]
pub fn conversation_delete(state: State<'_, AppState>, id: String) -> Result<(), CoreError> {
    state.conversations.delete(&id)
}

#[tauri::command]
pub fn conversation_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ConversationSummary>, CoreError> {
    state.conversations.search(&query)
}

/// Descarta uma mensagem e tudo depois dela.
///
/// E o que Regenerate e editar-e-reenviar usam antes de reenviar: a resposta
/// antiga deixa de valer quando a pergunta muda.
#[tauri::command]
pub fn conversation_truncate(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<(), CoreError> {
    state.conversations.truncate_from(&message_id)
}

/// Grava o que a VPS devolveu de `session.history`.
pub fn absorb_history<R: Runtime>(
    app: &AppHandle<R>,
    conversation_id: &str,
    messages: &[HistoryMessage],
) {
    let Ok(id) = mos_core::ConversationId::parse(conversation_id) else {
        return;
    };
    let projected = project_history(id, messages);
    if projected.is_empty() {
        return;
    }
    let service = conversations(app);
    if service
        .replace_with_history(conversation_id, projected)
        .is_ok()
    {
        let _ = app.emit("hermes-history", conversation_id);
    }
}

/// Grava o titulo que a VPS resolveu. O M/OS nao inventa titulo.
pub fn absorb_title<R: Runtime>(app: &AppHandle<R>, conversation_id: &str, title: &str) {
    if title.trim().is_empty() {
        return;
    }
    let service = conversations(app);
    if let Ok(conversation) = service.rename(conversation_id, title) {
        announce_conversation(app, &conversation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O relogio dos testes. Fixo, porque a leitura de uma proposta com data
    /// depende de quando ela e lida — e um teste que depende do relogio de
    /// parede falha sozinho as vinte e tres e cinquenta e nove.
    fn agora() -> time::OffsetDateTime {
        time::macros::datetime!(2026-08-20 14:32:00 -03:00)
    }
    use mos_core::ConversationId;

    #[test]
    fn a_proposal_leaves_the_text_and_becomes_a_part() {
        let (text, raw) = split_proposal(
            "Posso criar isso.\n\n```mos-action\n{\"action\":\"mos.task.create\",\"args\":{\"title\":\"X\"}}\n```\n",
        );
        assert_eq!(text, "Posso criar isso.");
        assert!(raw.unwrap().contains("mos.task.create"));
    }

    #[test]
    fn a_response_without_a_proposal_is_untouched() {
        let (text, raw) = split_proposal("Só uma resposta normal.");
        assert_eq!(text, "Só uma resposta normal.");
        assert!(raw.is_none());
    }

    /// Cerca aberta acontece de verdade: o turno pode ser interrompido no meio
    /// do bloco. Sem fechamento nao ha proposta, e o texto fica como veio em
    /// vez de sumir junto com o resto da mensagem.
    #[test]
    fn an_unclosed_fence_is_not_a_proposal() {
        let (text, raw) = split_proposal("Vou criar\n\n```mos-action\n{\"action\":\"mos.task");
        assert!(text.contains("Vou criar"));
        assert!(raw.is_none());
    }

    /// Proposta invalida vira parte RECUSADA, e nao desaparece. Descartar em
    /// silencio deixaria o usuario vendo o Hermes prometer uma acao que nunca
    /// existiu.
    #[test]
    fn an_invalid_proposal_becomes_a_refused_part() {
        match proposal_part("{\"action\":\"mos.task.create\",\"args\":{}}", agora()) {
            PartBody::ActionProposal {
                status, outcome, ..
            } => {
                assert_eq!(status, ProposalStatus::Refused);
                assert!(outcome.contains("title"));
            }
            other => panic!("esperava ActionProposal, veio {other:?}"),
        }
    }

    #[test]
    fn a_valid_proposal_starts_pending() {
        match proposal_part(
            "{\"action\":\"mos.capture.create\",\"args\":{\"content\":\"uma ideia\"}}",
            agora(),
        ) {
            PartBody::ActionProposal {
                status, preview, ..
            } => {
                assert_eq!(status, ProposalStatus::Pending);
                assert_eq!(preview.title, "CRIAR CAPTURE");
            }
            other => panic!("esperava ActionProposal, veio {other:?}"),
        }
    }

    #[test]
    fn the_recorder_settles_only_on_complete_or_failure() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        assert!(!recorder.absorb(&Outcome::Delta { text: "oi".into() }));
        assert!(!recorder.absorb(&Outcome::Reasoning { text: "hm".into() }));
        assert!(recorder.absorb(&Outcome::Complete));
    }

    /// Aprovacao e clarificacao param o turno sem encerra-lo. Assentar aqui
    /// fecharia a resposta no meio, e o que viesse depois nao teria mensagem
    /// para onde ir.
    #[test]
    fn waiting_for_the_user_does_not_settle_the_turn() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        assert!(!recorder.absorb(&Outcome::Approval {
            prompt: "posso?".into()
        }));
        assert!(!recorder.absorb(&Outcome::Clarify {
            request_id: "a".into(),
            question: "qual?".into(),
            choices: Vec::new(),
        }));
    }

    /// Uma ferramenta ainda em curso quando o turno acaba nao terminou: ela foi
    /// interrompida junto, e dizer "success" seria mentir sobre o que rodou.
    #[test]
    fn a_running_tool_is_cancelled_when_the_turn_is_interrupted() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Tool {
            name: "web".into(),
            running: true,
        });
        let parts = recorder.into_parts(MessageStatus::Interrupted, agora());
        match parts.iter().find(|part| part.kind() == "tool_run") {
            Some(PartBody::ToolRun { state, .. }) => {
                assert_eq!(*state, ToolRunState::Cancelled)
            }
            other => panic!("esperava ToolRun, veio {other:?}"),
        }
    }

    #[test]
    fn a_completed_tool_keeps_its_success() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Tool {
            name: "web".into(),
            running: true,
        });
        recorder.absorb(&Outcome::Tool {
            name: "web".into(),
            running: false,
        });
        let parts = recorder.into_parts(MessageStatus::Complete, agora());
        match parts.iter().find(|part| part.kind() == "tool_run") {
            Some(PartBody::ToolRun { state, .. }) => assert_eq!(*state, ToolRunState::Success),
            other => panic!("esperava ToolRun, veio {other:?}"),
        }
    }

    /// A recusa de sudo precisa aparecer na conversa. Uma recusa silenciosa
    /// deixaria o usuario sem entender por que o agente desistiu.
    #[test]
    fn a_refused_sudo_leaves_an_explanation_in_the_thread() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::SudoRefused);
        let parts = recorder.into_parts(MessageStatus::Complete, agora());
        match parts.first() {
            Some(PartBody::Status { text }) => assert!(text.contains("senha de root")),
            other => panic!("esperava Status, veio {other:?}"),
        }
    }

    #[test]
    fn text_comes_after_reasoning_and_tools() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Reasoning { text: "hm".into() });
        recorder.absorb(&Outcome::Tool {
            name: "web".into(),
            running: true,
        });
        recorder.absorb(&Outcome::Delta {
            text: "resposta".into(),
        });
        let kinds: Vec<&str> = recorder
            .into_parts(MessageStatus::Complete, agora())
            .iter()
            .map(|part| part.kind())
            .collect();
        assert_eq!(kinds, vec!["tool_run", "reasoning", "text"]);
    }

    /// `tool` nao e papel de mensagem no modelo local. Vira parte da mensagem
    /// anterior em vez de inventar um papel que o dominio recusa.
    #[test]
    fn a_tool_row_becomes_a_part_of_the_previous_message() {
        let id = ConversationId::new();
        let projected = project_history(
            id,
            &[
                HistoryMessage {
                    role: "assistant".into(),
                    content: "vou procurar".into(),
                    tool_name: String::new(),
                },
                HistoryMessage {
                    role: "tool".into(),
                    content: "3 resultados".into(),
                    tool_name: "web".into(),
                },
            ],
        );
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].parts.len(), 2);
        assert_eq!(projected[0].parts[1].kind(), "tool_run");
    }

    #[test]
    fn empty_history_rows_are_dropped() {
        let projected = project_history(
            ConversationId::new(),
            &[HistoryMessage {
                role: "assistant".into(),
                content: "   ".into(),
                tool_name: String::new(),
            }],
        );
        assert!(projected.is_empty());
    }

    /// Uma linha de ferramenta sem mensagem anterior nao pode ser descartada nem
    /// virar mensagem de papel `tool`: ela ganha um portador.
    #[test]
    fn a_leading_tool_row_gets_a_carrier_message() {
        let projected = project_history(
            ConversationId::new(),
            &[HistoryMessage {
                role: "tool".into(),
                content: "resultado".into(),
                tool_name: "web".into(),
            }],
        );
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].role, mos_core::MessageRole::Assistant);
        assert_eq!(projected[0].parts.len(), 1);
    }

    // -------------------------------------------------------------- a busca

    /// Cortar por byte partiria um `ç` no meio e produziria um bloco de
    /// contexto invalido — que apareceria como resposta estranha, e nao como
    /// erro.
    #[test]
    fn the_summary_cuts_by_character_and_never_mid_letter() {
        let cortado = resumo("çãõáé çãõáé çãõáé", 5);
        assert_eq!(cortado.chars().count(), 6, "cinco letras mais a reticencia");
        assert!(cortado.ends_with('…'));
        // O que cabe inteiro volta inteiro, sem reticencia.
        assert_eq!(resumo("  duas   palavras  ", 40), "duas palavras");
    }

    /// O modelo pede a busca por escrito, e o M/OS le o pedido depois que o
    /// turno assenta.
    #[test]
    fn the_recorder_sees_the_query_the_model_asked_for() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Delta {
            text: "Vou procurar.\n```mos-query\n{\"search\":\"victor bases\"}\n```".into(),
        });
        let raw = recorder.requested_query().expect("o pedido de busca");
        assert_eq!(mos_core::parse_query(&raw).unwrap().search, "victor bases");
    }

    /// A acao ganha da busca quando as duas vem juntas: se o modelo ja sabe o
    /// que propor, procurar mais seria gastar um turno para confirmar o que ele
    /// acabou de afirmar.
    #[test]
    fn a_proposal_cancels_the_search_request() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Delta {
            text: "```mos-query\n{\"search\":\"x\"}\n```\n\
                   ```mos-action\n{\"action\":\"mos.time.stop\"}\n```"
                .into(),
        });
        assert!(recorder.requested_query().is_none());
    }

    /// A busca vira uma execucao visivel na thread. Sem ela, a pausa entre a
    /// pergunta e a resposta parece travamento.
    #[test]
    fn the_search_shows_up_as_a_step_in_the_thread() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Delta {
            text: "Procurando.\n```mos-query\n{\"search\":\"victor\"}\n```".into(),
        });
        recorder.absorb(&Outcome::Complete);
        let parts = recorder.into_parts(MessageStatus::Complete, agora());
        let passo = parts
            .iter()
            .find_map(|part| match part {
                PartBody::ToolRun { name, state, detail } if name == "Busca no M/OS" => {
                    Some((*state, detail.clone()))
                }
                _ => None,
            })
            .expect("a busca precisa aparecer como passo");
        assert_eq!(passo.0, ToolRunState::Success);
        assert_eq!(passo.1, "victor");
        // E o JSON cru sai do texto, como acontece com a proposta.
        assert!(!parts.iter().any(|part| matches!(
            part,
            PartBody::Text { text } if text.contains("mos-query")
        )));
    }

    /// Um pedido de busca ilegivel nao pode sumir em silencio: ele vira um
    /// passo com erro, e a conversa mostra que o M/OS tentou.
    #[test]
    fn an_unreadable_search_becomes_a_failed_step() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Delta {
            text: "```mos-query\n{isso nao e json}\n```".into(),
        });
        recorder.absorb(&Outcome::Complete);
        let parts = recorder.into_parts(MessageStatus::Complete, agora());
        assert!(parts.iter().any(|part| matches!(
            part,
            PartBody::ToolRun { state, .. } if *state == ToolRunState::Error
        )));
    }

    /// A proposta de lembrete e lida contra o relogio de quem esta na tela, e o
    /// cartao mostra a hora resolvida. Sem o relogio, "hoje as 20:30" viraria um
    /// cartao dizendo o que o usuario acabou de escrever.
    #[test]
    fn a_reminder_proposal_becomes_a_card_with_the_resolved_hour() {
        match proposal_part(
            "{\"action\":\"mos.reminder.create\",\"args\":{\"title\":\"Enviar bases\",\"when\":\"hoje às 20:30\"}}",
            agora(),
        ) {
            PartBody::ActionProposal {
                status,
                preview,
                audit,
                ..
            } => {
                assert_eq!(status, ProposalStatus::Pending);
                assert_eq!(preview.title, "CRIAR LEMBRETE");
                // Pendente nao tem rastro: nada aconteceu ainda.
                assert!(audit.is_none());
                assert!(preview
                    .lines
                    .iter()
                    .any(|linha| linha.label == "Quando" && linha.value.contains("20:30")));
            }
            other => panic!("esperava ActionProposal, veio {other:?}"),
        }
    }
}

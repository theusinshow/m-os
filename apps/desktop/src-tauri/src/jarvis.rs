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
}

impl TurnRecorder {
    pub fn start(conversation_id: String, message_id: String) -> Self {
        Self {
            conversation_id,
            message_id,
            ..Default::default()
        }
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

    /// As partes na ordem em que devem ser lidas.
    pub fn into_parts(mut self, status: MessageStatus) -> Vec<PartBody> {
        self.settle_tools(status == MessageStatus::Interrupted);

        let mut parts = Vec::new();
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
        if !text.is_empty() {
            parts.push(PartBody::Text { text });
        }
        if let Some(raw) = proposal {
            parts.push(proposal_part(&raw));
        }
        if let Some(message) = self.error {
            parts.push(PartBody::Error { message });
        }
        parts
    }

    pub fn has_content(&self) -> bool {
        !self.text.is_empty()
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
    const FENCE: &str = "```mos-action";
    let Some(start) = text.find(FENCE) else {
        return (text.to_owned(), None);
    };
    let after = start + FENCE.len();
    let Some(end_offset) = text[after..].find("```") else {
        // Cerca aberta: o turno pode ter sido interrompido no meio do bloco.
        // Sem fechamento nao ha proposta valida, e o texto fica como veio.
        return (text.to_owned(), None);
    };
    let raw = text[after..after + end_offset].trim().to_owned();
    let mut cleaned = String::with_capacity(text.len());
    cleaned.push_str(&text[..start]);
    cleaned.push_str(&text[after + end_offset + 3..]);
    (cleaned.trim().to_owned(), Some(raw))
}

/// Transforma a proposta crua numa parte pendente, ou numa parte recusada.
///
/// Recusa tambem vira parte: uma proposta que nao bate com o esquema precisa
/// aparecer na conversa dizendo o motivo. Descartar em silencio deixaria o
/// usuario vendo o Hermes prometer uma acao que nunca existiu.
pub fn proposal_part(raw: &str) -> PartBody {
    match mos_core::parse_action(raw) {
        Ok(args) => PartBody::ActionProposal {
            raw: raw.to_owned(),
            preview: mos_core::preview_of(&args),
            status: ProposalStatus::Pending,
            outcome: String::new(),
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
        },
    }
}

/// Executa uma acao aprovada, pelos mesmos servicos que a interface usa.
///
/// Nunca SQL proprio, nunca um atalho: e a invariante que faz a acao do Hermes
/// obedecer as mesmas regras que a acao do usuario (ADR-024, ADR-032).
fn run_action<R: Runtime>(
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
            Ok(mos_core::ActionEffect {
                message: format!("Capture criada: {}", capture.content),
                undo: Some(mos_core::UndoStep::ArchiveCapture {
                    id: capture.id.to_string(),
                }),
            })
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
            Ok(mos_core::ActionEffect {
                message: format!("Task criada: {}", task.title),
                undo: Some(mos_core::UndoStep::ArchiveTask {
                    id: task.id.to_string(),
                }),
            })
        }
        mos_core::ActionArgs::TaskSetState { task, state: next } => {
            // O estado anterior e lido ANTES da mudanca: depois nao ha de onde
            // tirar, e sem ele "mover" seria a unica acao sem caminho de volta.
            let target = resolve_task(&state, task)?;
            let previous = target.state;
            let updated = state
                .work
                .set_task_state(&target.id.to_string(), mos_core::TaskState::parse(next)?)?;
            Ok(mos_core::ActionEffect {
                message: format!("{} movida para {}", updated.title, next),
                undo: Some(mos_core::UndoStep::RestoreTaskState {
                    id: updated.id.to_string(),
                    state: previous.as_str().to_owned(),
                }),
            })
        }
        mos_core::ActionArgs::ProjectCreate { name, description } => {
            let project = state.work.create_project(mos_core::CreateProjectInput {
                name: name.clone(),
                description: description.clone(),
                repository: String::new(),
            })?;
            Ok(mos_core::ActionEffect {
                message: format!("Project criado: {}", project.name),
                undo: Some(mos_core::UndoStep::ArchiveProject {
                    id: project.id.to_string(),
                }),
            })
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
            Ok(mos_core::ActionEffect {
                message: format!("Resource salvo: {}", resource.title),
                undo: Some(mos_core::UndoStep::ArchiveResource {
                    id: resource.id.to_string(),
                }),
            })
        }
    }
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
    }
    let _ = app.emit("data-changed", "undo");
    Ok(())
}

/// Acha o Project pelo nome que o usuario falou.
///
/// Ambiguidade RECUSA em vez de escolher o primeiro. "Escadas" batendo em dois
/// projetos e exatamente o caso onde adivinhar cria a Task no lugar errado, e o
/// erro so aparece dias depois.
fn resolve_project(state: &AppState, name: &str) -> Result<Option<String>, CoreError> {
    let needle = name.trim().to_lowercase();
    let matches: Vec<_> = state
        .work
        .projects(false)?
        .into_iter()
        .filter(|project| project.name.to_lowercase().contains(&needle))
        .collect();

    match matches.len() {
        0 => Err(CoreError::new(
            mos_core::ErrorCode::NotFound,
            format!("Nenhum Project chamado \"{name}\"."),
            false,
        )),
        1 => Ok(Some(matches[0].id.to_string())),
        _ => Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            format!("\"{name}\" bate com {} Projects. Diga qual.", matches.len()),
            false,
        )),
    }
}

/// Devolve a Task inteira, e nao so o id: quem chama precisa do estado ANTERIOR
/// para montar o desfazer, e ele ja esta aqui. Buscar de novo depois custaria
/// uma segunda varredura e leria um estado que a mudanca ja alterou.
fn resolve_task(state: &AppState, title: &str) -> Result<mos_core::Task, CoreError> {
    let needle = title.trim().to_lowercase();
    let matches: Vec<_> = state
        .work
        .tasks(false)?
        .into_iter()
        .filter(|task| task.title.to_lowercase().contains(&needle))
        .collect();

    match matches.len() {
        0 => Err(CoreError::new(
            mos_core::ErrorCode::NotFound,
            format!("Nenhuma Task chamada \"{title}\"."),
            false,
        )),
        1 => Ok(matches.into_iter().next().expect("um resultado")),
        _ => Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            format!("\"{title}\" bate com {} Tasks. Diga qual.", matches.len()),
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

    let (status, outcome, undo) = if !approved {
        (
            ProposalStatus::Cancelled,
            "Cancelado por você.".to_owned(),
            None,
        )
    } else {
        match mos_core::parse_action(&raw).and_then(|args| run_action(&app, &args)) {
            Ok(effect) => (ProposalStatus::Executed, effect.message, effect.undo),
            Err(error) => (ProposalStatus::Failed, error.message, None),
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
        match proposal_part("{\"action\":\"mos.task.create\",\"args\":{}}") {
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
        let parts = recorder.into_parts(MessageStatus::Interrupted);
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
        let parts = recorder.into_parts(MessageStatus::Complete);
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
        let parts = recorder.into_parts(MessageStatus::Complete);
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
            .into_parts(MessageStatus::Complete)
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
}

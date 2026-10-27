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

    /// O salto que o modelo pediu nesta resposta, se pediu: uma busca no M/OS
    /// ou uma consulta ao M-Finance.
    ///
    /// Lida ANTES de `into_parts`, que consome o registrador: quem decide se
    /// vale um segundo salto precisa saber disso enquanto ainda pode agir.
    pub fn requested_follow_up(&self) -> Option<FollowUp> {
        // A acao ganha de qualquer salto quando vem junto, e a razao e a ordem
        // do trabalho: se o modelo ja sabe o que propor, procurar mais seria
        // gastar um turno para confirmar o que ele acabou de afirmar.
        if mos_core::split_fenced(&self.text, "mos-action").1.is_some() {
            return None;
        }
        // Consulta financeira antes da busca local: quem pediu os dois esta
        // fazendo uma pergunta sobre dinheiro, e o numero e o que falta.
        if let Some(raw) = mos_core::split_fenced(&self.text, "mos-finance").1 {
            return Some(FollowUp::Finance(raw));
        }
        mos_core::split_fenced(&self.text, "mos-query")
            .1
            .map(FollowUp::Query)
    }

    /// As partes na ordem em que devem ser lidas.
    pub fn into_parts(
        mut self,
        status: MessageStatus,
        now_local: time::OffsetDateTime,
    ) -> Vec<PartBody> {
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
        // A consulta ao M-Finance sai pelo mesmo motivo, e vira o mesmo tipo de
        // passo visivel. O resultado dela chega no turno seguinte, registrado
        // la como o que atravessou a ponte (ADR-027).
        let (text, finance) = mos_core::split_fenced(&text, "mos-finance");
        if !text.is_empty() {
            parts.push(PartBody::Text { text });
        }
        if let Some(raw) = &finance {
            let (state, detail) = match mos_core::parse_finance_query(raw) {
                Ok(request) => (ToolRunState::Success, request.tool.as_str().to_owned()),
                Err(error) => (ToolRunState::Error, error.message),
            };
            parts.push(PartBody::ToolRun {
                name: "Consulta ao M-Finance".to_owned(),
                state,
                detail,
            });
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

/// O segundo turno que uma resposta pode pedir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FollowUp {
    /// ` ```mos-query ``` ` — busca na base local do M/OS (ADR-051).
    Query(String),
    /// ` ```mos-finance ``` ` — uma ferramenta de leitura do M-Finance (ADR-088).
    Finance(String),
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

/// O que o desktop sabe fazer alem de gravar: avisar a janela e acordar o
/// agendador de lembretes. O executor mora no core (ADR-080).
pub(crate) struct CapacidadesDesktop<R: Runtime> {
    pub app: AppHandle<R>,
}

impl<R: Runtime> mos_core::agente_exec::Capacidades for CapacidadesDesktop<R> {
    fn avisar(&self, evento: &str, tipo: &str) {
        let _ = self.app.emit(evento, tipo);
    }
    fn lembretes_mudaram(&self) {
        crate::attention::poke(&self.app);
    }
}

/// Executa uma acao aprovada, pelos mesmos servicos que a interface usa.
///
/// A regra mora em `mos_core::agente_exec` — a mesma que o bolso chama. O que
/// fica aqui e so o que e DESTE aparelho: a ponte do M-Finance, que e efeito
/// externo e assincrono.
async fn run_action<R: Runtime>(
    app: &AppHandle<R>,
    args: &mos_core::ActionArgs,
    idempotency_key: &str,
    workspace_id: Option<String>,
) -> Result<mos_core::ActionEffect, CoreError> {
    // Toda acao do M-Finance segue o mesmo caminho: o corpo sai do MESMO
    // `ActionArgs` que desenhou o cartao (`finance_payload`, no core), a chave
    // de idempotencia identifica a proposta, e o M-Finance valida de novo
    // contra o que o preview mostrou.
    if let Some(payload) = mos_core::finance_payload(args) {
        let effect = crate::finance::execute_action(args.kind().as_str(), payload, idempotency_key)
            .await
            .map_err(|error| CoreError::new(mos_core::ErrorCode::Io, error, true))?;
        let _ = app.emit("data-changed", "finance");
        // Sem desfazer: o M/OS nao tem um "apagar lancamento" no M-Finance, e
        // inventar um so para o Undo seria dar ao Hermes um poder que a Action
        // API nao expoe. Corrigir e no proprio M-Finance.
        let mut result = mos_core::ActionEffect::new(effect.message, None);
        for (kind, id, label) in effect.entities {
            result = result.touching(kind, id, label);
        }
        return Ok(result);
    }
    let state = app.state::<AppState>();
    let servicos = state.servicos();
    let capacidades = CapacidadesDesktop { app: app.clone() };
    mos_core::agente_exec::executar_acao(
        &mos_core::agente_exec::Contexto {
            workspace_id,
            servicos: &servicos,
            capacidades: &capacidades,
            agora: state.clock.now(),
            agora_local: crate::surface::now_local(app),
        },
        args,
    )
}

/// Desfaz uma acao executada.
///
/// Vive num comando proprio, e nao dentro de `action_resolve`, porque o desfazer
/// acontece depois — na janela do recibo, quando o usuario le o que aconteceu e
/// decide que nao era aquilo.
///
/// Devolve uma nota quando o desfazer foi PARCIAL — o que ficou como estava e
/// por que. Vazia quando tudo voltou.
#[tauri::command]
pub async fn action_undo<R: Runtime>(
    app: AppHandle<R>,
    step: mos_core::UndoStep,
) -> Result<String, CoreError> {
    let state = app.state::<AppState>();
    let servicos = state.servicos();
    let capacidades = CapacidadesDesktop { app: app.clone() };
    mos_core::agente_exec::desfazer_acao(
        &mos_core::agente_exec::Contexto {
            workspace_id: None,
            servicos: &servicos,
            capacidades: &capacidades,
            agora: state.clock.now(),
            agora_local: crate::surface::now_local(&app),
        },
        step,
    )
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
    // A reserva vem ANTES de qualquer efeito (MOS-OVERHAUL-PLAN F1.2). Clique
    // duplo, retry depois de timeout e reenvio depois de reiniciar chegam aqui
    // e recebem o desfecho gravado — nunca uma segunda execucao.
    match service.claim_proposal(&message_id, &raw, approved)? {
        mos_core::ProposalClaim::Claimed => {}
        mos_core::ProposalClaim::Settled(body) => {
            let message = service.message(&message_id)?;
            let (receipt, undo) = match body {
                PartBody::ActionProposal {
                    status: ProposalStatus::Executed,
                    outcome,
                    audit,
                    ..
                } => (outcome, audit.and_then(|audit| audit.undo)),
                _ => (String::new(), None),
            };
            return Ok(ActionResolution {
                receipt,
                message,
                undo,
            });
        }
        mos_core::ProposalClaim::InFlight => {
            return Err(CoreError::new(
                mos_core::ErrorCode::Conflict,
                "Esta proposta já foi enviada para execução e o resultado ainda não foi confirmado. Confira se o efeito aconteceu antes de pedir de novo.",
                false,
            ));
        }
    }
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
            Ok(args) => {
                // O que o Hermes cria nasce no Workspace da conversa que o
                // propos (ADR-093), e nao no que a tela mostra agora.
                let workspace = service
                    .get(&message.conversation_id.to_string())
                    .ok()
                    .and_then(|conversa| conversa.workspace_id)
                    .map(|id| id.to_string());
                run_action(
                    &app,
                    &args,
                    &crate::finance::idempotency_key(&message_id, &raw),
                    workspace,
                )
                .await
            }
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
                audit: audit.clone().map(Box::new),
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
pub fn candidates_for<R: Runtime>(
    app: &AppHandle<R>,
    text: &str,
    escopo: Option<&mos_core::EscopoDoWorkspace>,
) -> Vec<mos_core::Candidate> {
    candidates_for_kinds(app, text, &[], escopo)
}

/// O escopo do Workspace da conversa, ou `None` quando ela e de TUDO.
///
/// Falha de leitura vira `None`, e nao erro: sem o escopo o Hermes recebe o
/// contexto de antes da ADR-093, que e mais largo e nunca menos correto.
pub fn escopo_da_conversa<R: Runtime>(
    app: &AppHandle<R>,
    conversation_id: &str,
) -> Option<mos_core::EscopoDoWorkspace> {
    let state = app.state::<AppState>();
    let workspace = state
        .conversations
        .get(conversation_id)
        .ok()?
        .workspace_id?;
    state.work.escopo_do_workspace(&workspace.to_string()).ok()
}

fn candidates_for_kinds<R: Runtime>(
    app: &AppHandle<R>,
    text: &str,
    kinds: &[mos_core::EntityKind],
    escopo: Option<&mos_core::EscopoDoWorkspace>,
) -> Vec<mos_core::Candidate> {
    let termos = mos_core::search_terms(text);
    if termos.is_empty() {
        return Vec::new();
    }
    let state = app.state::<AppState>();
    let mut hits = Vec::new();
    for (index, termo) in termos.iter().enumerate() {
        if let Ok(items) = state.work.search(termo, false) {
            hits.extend(
                items
                    .iter()
                    .filter_map(candidate_of)
                    .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
                    .take(HITS_POR_TERMO)
                    .map(|c| (index, c)),
            );
        }
        if kinds.is_empty() || kinds.contains(&mos_core::EntityKind::Resource) {
            if let Ok(resources) = state.memory.search(termo, false, HITS_POR_TERMO) {
                hits.extend(resources.into_iter().map(|r| {
                    (
                        index,
                        mos_core::Candidate {
                            kind: mos_core::EntityKind::Resource,
                            id: r.id.to_string(),
                            label: resumo(&r.title, 70),
                            detail: r.kind.as_str().to_owned(),
                        },
                    )
                }));
            }
        }
        if kinds.is_empty() || kinds.contains(&mos_core::EntityKind::Meeting) {
            if let Ok(meetings) = state.meetings.search(termo, HITS_POR_TERMO) {
                hits.extend(meetings.into_iter().map(|m| {
                    (
                        index,
                        mos_core::Candidate {
                            kind: mos_core::EntityKind::Meeting,
                            id: m.id.to_string(),
                            label: resumo(&m.title, 70),
                            detail: mos_core::spoken_moment(m.started_at),
                        },
                    )
                }));
            }
            // Uma lembrança pode estar só no que foi dito, não no título.
            if let Ok(transcripts) = state.meetings.search_transcripts(termo, HITS_POR_TERMO) {
                hits.extend(transcripts.into_iter().map(|(m, _)| {
                    (
                        index,
                        mos_core::Candidate {
                            kind: mos_core::EntityKind::Meeting,
                            id: m.id.to_string(),
                            label: resumo(&m.title, 70),
                            detail: mos_core::spoken_moment(m.started_at),
                        },
                    )
                }));
            }
        }
    }
    // "O que o Victor ainda deve?" — o nome costuma estar em QUEM a Task
    // espera, e nao no titulo. O indice FTS nao cobre esse campo; a lista de
    // Tasks abertas cobre (F4.2, ADR-085).
    if kinds.is_empty() || kinds.contains(&mos_core::EntityKind::Task) {
        if let Ok(tasks) = state.work.tasks(false) {
            for task in tasks
                .iter()
                .filter(|t| !t.waiting_for.is_empty() && t.state != mos_core::TaskState::Done)
            {
                let quem = mos_core::normalize(&task.waiting_for);
                for (index, termo) in termos.iter().enumerate() {
                    if quem.contains(termo.as_str()) {
                        hits.push((
                            index,
                            mos_core::Candidate {
                                kind: mos_core::EntityKind::Task,
                                id: task.id.to_string(),
                                label: resumo(&task.title, 70),
                                detail: format!("aguardando {}", task.waiting_for),
                            },
                        ));
                    }
                }
            }
        }
    }
    if kinds.is_empty() || kinds.contains(&mos_core::EntityKind::Reminder) {
        if let Ok(abertos) = state.attention.open() {
            for reminder in abertos {
                let alvo = mos_core::normalize(&reminder.title);
                for (index, termo) in termos.iter().enumerate() {
                    if alvo.contains(termo.as_str()) {
                        hits.push((
                            index,
                            mos_core::Candidate {
                                kind: mos_core::EntityKind::Reminder,
                                id: reminder.id.to_string(),
                                label: resumo(&reminder.title, 70),
                                detail: reminder
                                    .next_due_at
                                    .map(mos_core::spoken_moment)
                                    .unwrap_or_else(|| reminder.status.as_str().to_owned()),
                            },
                        ));
                    }
                }
            }
        }
    }
    // O filtro vem ANTES do ranking, que aplica o teto: filtrar depois deixaria
    // uma conversa de PROSUL com tres candidatos onde cabiam doze.
    if let Some(escopo) = escopo {
        hits.retain(|(_, candidato)| escopo.admite(candidato.kind, &candidato.id));
    }
    mos_core::rank_candidates(hits, kinds)
}

/// Projeta um resultado de busca em candidato.
///
/// O que o modelo precisa saber de uma Task numa linha so.
///
/// Estado, Project, progresso e prazo — e nada mais. O checklist INTEIRO nao
/// desce aqui de proposito: doze candidatos com seis passos cada seriam setenta
/// e duas linhas em toda mensagem, e o §17 do `HERMES-ACTION-LAYER.md` conta
/// tokens. O `4/7` responde *"o que falta nessa task?"* com um numero, e quem
/// precisa dos textos anexa a Task.
fn detalhe_da_task(task: &mos_core::Task, project: Option<&mos_core::Project>) -> String {
    let mut partes = vec![task.state.as_str().to_owned()];
    if let Some(project) = project {
        partes.push(project.name.clone());
    }
    if task.checklist_total > 0 {
        partes.push(format!("{}/{}", task.checklist_done, task.checklist_total));
    }
    if let Some(prazo) = task.due_at {
        partes.push(format!("vence {}", mos_core::spoken_moment(prazo)));
    }
    if task.priority != mos_core::Priority::Normal {
        partes.push(task.priority.as_str().to_owned());
    }
    if !task.waiting_for.is_empty() {
        partes.push(format!("aguardando {}", task.waiting_for));
    }
    partes.join(" · ")
}

/// `App` fica de fora: ele nao e uma entidade sobre a qual o catalogo de acoes
/// saiba agir, e um candidato que nao pode virar acao so ocupa linha no prompt.
fn candidate_of(item: &mos_core::SearchItem) -> Option<mos_core::Candidate> {
    Some(match item {
        mos_core::SearchItem::Task { task, project } => mos_core::Candidate {
            kind: mos_core::EntityKind::Task,
            id: task.id.to_string(),
            label: resumo(&task.title, 70),
            detail: detalhe_da_task(task, project.as_ref()),
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
        mos_core::Decision::None => {
            planned_at.map(|quando| format!("planejada para {}", mos_core::spoken_moment(quando)))
        }
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
    escopo: Option<&mos_core::EscopoDoWorkspace>,
) -> Vec<mos_core::Candidate> {
    candidates_for_kinds(app, &request.search, &request.kinds, escopo)
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
                let tasks = state.work.tasks(false)?;
                // As reunioes DESTE Project, com o que elas decidiram e
                // comprometeram. Dispensado nao entra: a pessoa ja disse que
                // aquilo nao era nada.
                let mut das_reunioes = Vec::new();
                for meeting in state
                    .meetings
                    .meetings(false)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|m| m.project_id == Some(project.id))
                {
                    for insight in state
                        .meetings
                        .insights(&meeting.id.to_string())
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|i| i.status != mos_core::InsightStatus::Dismissed)
                    {
                        das_reunioes.push(mos_core::ItemDeReuniao {
                            reuniao_id: meeting.id.to_string(),
                            reuniao: meeting.title.clone(),
                            quando: meeting.started_at,
                            kind: insight.kind,
                            texto: insight.text,
                        });
                    }
                }
                (
                    ContextEntity::Project,
                    vec![
                        "name",
                        "description",
                        "openTasks",
                        "waitingFor",
                        "blockedBy",
                        "dueAt",
                        "meetingInsights",
                    ],
                    mos_core::project_context(&project, &tasks, &das_reunioes),
                )
            }
            "task" => {
                let detail = state.work.task_detail(&context.id)?;
                let body = mos_core::task_context(&detail);
                (
                    ContextEntity::Task,
                    vec![
                        "title",
                        "state",
                        "description",
                        "priority",
                        "dueAt",
                        "scheduledFor",
                        "waitingFor",
                        "followUpAt",
                        "checklist",
                        "subtasks",
                        "blockedBy",
                        "references",
                    ],
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
    workspace_id: Option<String>,
) -> Result<Vec<ConversationSummary>, CoreError> {
    state
        .conversations
        .list_em(include_archived, workspace_id.as_deref())
}

#[tauri::command]
pub fn conversation_current(
    state: State<'_, AppState>,
    workspace_id: Option<String>,
) -> Result<Conversation, CoreError> {
    state
        .conversations
        .current_or_new_em(workspace_id.as_deref())
}

#[tauri::command]
pub fn conversation_create(
    state: State<'_, AppState>,
    workspace_id: Option<String>,
) -> Result<Conversation, CoreError> {
    state.conversations.create_em(workspace_id.as_deref())
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
        let Some(FollowUp::Query(raw)) = recorder.requested_follow_up() else {
            panic!("o pedido de busca");
        };
        assert_eq!(mos_core::parse_query(&raw).unwrap().search, "victor bases");
    }

    /// A consulta financeira e lida do mesmo jeito, e ganha da busca local
    /// quando as duas vem juntas.
    #[test]
    fn the_recorder_sees_the_finance_query() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Delta {
            text: "```mos-query
{\"search\":\"x\"}
```
                   ```mos-finance
{\"tool\":\"finance.get_goals\"}
```"
            .into(),
        });
        let Some(FollowUp::Finance(raw)) = recorder.requested_follow_up() else {
            panic!("a consulta financeira");
        };
        assert_eq!(
            mos_core::parse_finance_query(&raw).unwrap().tool,
            mos_core::FinanceTool::Goals
        );
    }

    /// A consulta vira um passo visivel, e o bloco cru some do texto.
    #[test]
    fn the_finance_query_shows_up_as_a_step() {
        let mut recorder = TurnRecorder::start("c".into(), "m".into());
        recorder.absorb(&Outcome::Delta {
            text: "Consultando.
```mos-finance
{\"tool\":\"finance.get_card_exposure\"}
```"
            .into(),
        });
        recorder.absorb(&Outcome::Complete);
        let parts = recorder.into_parts(MessageStatus::Complete, agora());
        assert!(parts.iter().any(|part| matches!(
            part,
            PartBody::ToolRun { name, detail, .. }
                if name == "Consulta ao M-Finance" && detail == "finance.get_card_exposure"
        )));
        assert!(parts.iter().all(|part| match part {
            PartBody::Text { text } => !text.contains("mos-finance"),
            _ => true,
        }));
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
        assert!(recorder.requested_follow_up().is_none());
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
                PartBody::ToolRun {
                    name,
                    state,
                    detail,
                } if name == "Busca no M/OS" => Some((*state, detail.clone())),
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

//! Conversa do Hermes, do lado do M/OS.
//!
//! A VPS continua dona do historico do agente. O M/OS guarda a sua projecao e o
//! vinculo (`hermes_session_id`), porque sem conversa local nao existe lista,
//! busca, rename nem acao sobre uma mensagem — nao existia mensagem, existia um
//! triplo de strings por turno. Ver ADR-025.
//!
//! Tres entidades, nao nove. Anexo, artifact, citacao e execucao de ferramenta
//! entram como corpo de parte e so viram tabela quando precisarem de lifecycle
//! ou consulta propria.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CoreError, ErrorCode};

macro_rules! conversation_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn parse(value: &str) -> Result<Self, CoreError> {
                Uuid::parse_str(value).map(Self).map_err(|_| {
                    CoreError::new(
                        ErrorCode::InvalidInput,
                        concat!($label, " invalido."),
                        false,
                    )
                })
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

conversation_id!(ConversationId, "Conversation ID");
conversation_id!(MessageId, "Message ID");
conversation_id!(MessagePartId, "Message part ID");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    /// Reservado ao que o proprio M/OS insere na conversa — recusa de sudo,
    /// aviso de queda no meio do streaming. Nunca vira `prompt.submit`.
    System,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            // `tool` existe no historico da VPS e e projetado como parte de uma
            // mensagem de assistant, nao como papel proprio.
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Papel de mensagem desconhecido.",
                false,
            )),
        }
    }
}

/// Estado de uma mensagem. Separado do lifecycle da conversa pelo mesmo motivo
/// que `ProcessingState` e separado de `LifecycleState` em Capture (ADR-015):
/// misturar os dois torna a recuperacao ambigua.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    /// Enviada, sem primeiro token ainda.
    Pending,
    Streaming,
    Complete,
    /// O usuario cancelou, ou o socket caiu. O texto recebido ate ali fica.
    Interrupted,
    Failed,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Streaming => "streaming",
            Self::Complete => "complete",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "pending" => Ok(Self::Pending),
            "streaming" => Ok(Self::Streaming),
            "complete" => Ok(Self::Complete),
            "interrupted" => Ok(Self::Interrupted),
            "failed" => Ok(Self::Failed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de mensagem desconhecido.",
                false,
            )),
        }
    }

    /// Uma mensagem parada nao volta a andar sozinha. Serve para reparar o
    /// estado na abertura: o app pode ter sido fechado no meio de um turno, e
    /// `streaming` gravado no banco viraria uma resposta eternamente em curso.
    pub fn is_settled(self) -> bool {
        matches!(self, Self::Complete | Self::Interrupted | Self::Failed)
    }
}

/// Onde uma proposta parou.
///
/// `Refused` e diferente de `Cancelled`: a primeira o M/OS recusou por a
/// proposta nao bater com o esquema, a segunda o usuario decidiu nao fazer. As
/// duas terminam sem efeito, e distinguir importa — uma e defeito de
/// interpretacao, a outra e escolha.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Executed,
    Cancelled,
    Refused,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRunState {
    Queued,
    Running,
    Success,
    Error,
    Cancelled,
    WaitingPermission,
}

/// De onde veio o contexto anexado. Explicito e automatico se distinguem na UI
/// por rotulo e peso, nunca so por cor (ADR-027).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextOrigin {
    /// O usuario pediu, mencionando com `@`.
    Explicit,
    /// O sistema ofereceu a partir da tela atual. Nasce desligado.
    Automatic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextEntity {
    Project,
    Task,
    Capture,
    Resource,
    Workspace,
    Screen,
}

/// O corpo de uma parte.
///
/// O discriminador vai para uma coluna propria, porque e por ele que a busca
/// filtra; o corpo inteiro e persistido como JSON. Promover qualquer variante a
/// tabela propria depois e migration mecanica — foi para isso que a parte
/// existe (ADR-025).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PartBody {
    Text {
        text: String,
    },
    /// Acumulado e escondido por padrao. Fora da busca: cresce sem limite e
    /// polui resultado.
    Reasoning {
        text: String,
    },
    ToolRun {
        name: String,
        state: ToolRunState,
        /// Payload tecnico, atras de expansao. Vazio quando nao ha.
        detail: String,
    },
    /// `status.update`: o que o agente faz entre um token e outro.
    Status {
        text: String,
    },
    Error {
        message: String,
    },
    /// Uma acao que o Hermes PROPOS. Ela nasce pendente e so vira efeito depois
    /// que o usuario confirma — o modelo nunca executa nada.
    ///
    /// A proposta e guardada como parte, e nao so desenhada a partir do texto,
    /// porque a decisao precisa sobreviver a recarregar a conversa: reabrir uma
    /// thread e ver de novo o botao "Criar" de algo que ja foi criado seria a
    /// forma mais rapida de duplicar uma Task.
    ActionProposal {
        /// A proposta crua, para executar depois da confirmacao.
        raw: String,
        preview: crate::ActionPreview,
        status: ProposalStatus,
        /// Resultado depois de resolvida. Vazio enquanto pendente.
        outcome: String,
    },
    /// O que EFETIVAMENTE atravessou a ponte. A pergunta "o que foi para a VPS?"
    /// precisa de resposta depois do envio, nao so antes (ADR-027).
    ContextRef {
        origin: ContextOrigin,
        entity: ContextEntity,
        /// Id da entidade no M/OS. Vazio para `screen`, que nao e entidade.
        id: String,
        label: String,
        /// Campos incluidos no bloco enviado.
        fields: Vec<String>,
        /// Tamanho do que foi enviado, em bytes.
        bytes: usize,
    },
}

impl PartBody {
    /// Nome estavel do tipo. Vai para coluna propria porque a busca filtra por
    /// ele sem desserializar o payload.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::Reasoning { .. } => "reasoning",
            Self::ToolRun { .. } => "tool_run",
            Self::Status { .. } => "status",
            Self::Error { .. } => "error",
            Self::ActionProposal { .. } => "action_proposal",
            Self::ContextRef { .. } => "context_ref",
        }
    }

    /// O que entra no FTS5.
    ///
    /// So `text`. Raciocinio e payload de ferramenta ficam fora por decisao,
    /// nao por esquecimento: os dois crescem sem limite e empurrariam a resposta
    /// util para fora do resultado.
    pub fn searchable_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    pub fn to_payload(&self) -> Result<String, CoreError> {
        serde_json::to_string(self).map_err(|error| {
            CoreError::new(
                ErrorCode::DataIntegrity,
                format!("Nao foi possivel serializar a parte da mensagem: {error}"),
                false,
            )
        })
    }

    /// Le o payload guardado.
    ///
    /// Uma parte com forma desconhecida vira `Error` legivel em vez de derrubar
    /// a conversa inteira: o banco pode ter sido escrito por uma versao mais
    /// nova, e perder uma linha e melhor que perder a thread.
    pub fn from_payload(payload: &str) -> Self {
        serde_json::from_str(payload).unwrap_or_else(|_| Self::Error {
            message: "Esta parte da mensagem foi gravada num formato que esta versao do M/OS nao conhece.".to_owned(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePart {
    pub id: MessagePartId,
    /// Ordem dentro da mensagem.
    pub seq: i64,
    pub body: PartBody,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    /// Ordem dentro da conversa.
    pub seq: i64,
    pub role: MessageRole,
    pub status: MessageStatus,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub parts: Vec<MessagePart>,
}

impl Message {
    /// Junta as partes de texto. E o que Copy copia e o que o reenvio de uma
    /// mensagem editada reaproveita.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| part.body.searchable_text())
            .collect::<Vec<_>>()
            .concat()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: ConversationId,
    /// Vazio ate `session.title` responder. O M/OS nao inventa titulo.
    pub title: String,
    /// O vinculo com a sessao da VPS. `None` enquanto a sessao nao abriu.
    pub hermes_session_id: Option<String>,
    pub lifecycle_state: crate::LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Linha da lista de conversas.
///
/// Projecao propria porque a lista nao precisa das mensagens: carregar a thread
/// inteira de cada conversa para mostrar um titulo seria pagar o custo de abrir
/// tudo para nao abrir nada.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: ConversationId,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub message_count: i64,
    /// Primeira linha da ultima mensagem com texto. Vazio numa conversa nova.
    pub preview: String,
}

#[derive(Clone, Debug)]
pub struct NewConversation {
    pub id: ConversationId,
    pub title: String,
    pub created_at: OffsetDateTime,
}

impl NewConversation {
    /// Conversa nasce sem titulo de proposito: quem nomeia e `session.title`,
    /// depois do primeiro turno. Um "Nova conversa" gravado no banco seria um
    /// titulo falso que sobreviveria ao titulo real.
    pub fn create() -> Self {
        Self {
            id: ConversationId::new(),
            title: String::new(),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewMessage {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub role: MessageRole,
    pub status: MessageStatus,
    pub created_at: OffsetDateTime,
    pub parts: Vec<PartBody>,
}

impl NewMessage {
    pub fn user(conversation_id: ConversationId, text: &str) -> Result<Self, CoreError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A mensagem nao pode estar vazia.",
                false,
            ));
        }
        Ok(Self {
            id: MessageId::new(),
            conversation_id,
            role: MessageRole::User,
            status: MessageStatus::Complete,
            created_at: OffsetDateTime::now_utc(),
            parts: vec![PartBody::Text {
                text: text.to_owned(),
            }],
        })
    }

    /// Resposta que ainda vai chegar. Nasce sem partes e sem texto — o primeiro
    /// delta e que a povoa.
    pub fn pending_assistant(conversation_id: ConversationId) -> Self {
        Self {
            id: MessageId::new(),
            conversation_id,
            role: MessageRole::Assistant,
            status: MessageStatus::Pending,
            created_at: OffsetDateTime::now_utc(),
            parts: Vec::new(),
        }
    }

    /// Nota do proprio M/OS na conversa. Nunca vira `prompt.submit`.
    pub fn system(conversation_id: ConversationId, text: &str) -> Self {
        Self {
            id: MessageId::new(),
            conversation_id,
            role: MessageRole::Assistant,
            status: MessageStatus::Complete,
            created_at: OffsetDateTime::now_utc(),
            parts: vec![PartBody::Status {
                text: text.to_owned(),
            }],
        }
    }
}

/// Titulo aceito, ou o motivo da recusa.
///
/// Vazio e valido: significa "deixe o Hermes nomear". O teto existe porque o
/// titulo vive numa coluna de lista de largura fixa, e um titulo de mil
/// caracteres nao e um titulo.
pub fn validate_title(title: &str) -> Result<String, CoreError> {
    let title = title.trim();
    if title.chars().count() > 120 {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "O titulo da conversa passa de 120 caracteres.",
            false,
        ));
    }
    Ok(title.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_user_message_requires_text() {
        let error = NewMessage::user(ConversationId::new(), "   ").unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    /// A resposta nasce vazia: o primeiro delta e que a povoa. Exigir parte
    /// aqui obrigaria a inventar uma parte de texto vazia so para satisfazer o
    /// construtor.
    #[test]
    fn a_pending_answer_starts_with_no_parts() {
        let message = NewMessage::pending_assistant(ConversationId::new());
        assert!(message.parts.is_empty());
        assert_eq!(message.status, MessageStatus::Pending);
    }

    /// So texto entra na busca. Raciocinio e payload de ferramenta crescem sem
    /// limite e empurrariam a resposta util para fora do resultado.
    #[test]
    fn only_text_parts_are_searchable() {
        assert_eq!(
            PartBody::Text {
                text: "achavel".into()
            }
            .searchable_text(),
            Some("achavel")
        );
        assert_eq!(
            PartBody::Reasoning { text: "hm".into() }.searchable_text(),
            None
        );
        assert_eq!(
            PartBody::ToolRun {
                name: "web".into(),
                state: ToolRunState::Success,
                detail: "{}".into(),
            }
            .searchable_text(),
            None
        );
    }

    #[test]
    fn a_part_survives_a_round_trip_through_its_payload() {
        let body = PartBody::ContextRef {
            origin: ContextOrigin::Explicit,
            entity: ContextEntity::Project,
            id: "0198a7d5-a64e-7000-8000-000000000001".into(),
            label: "M/OS".into(),
            fields: vec!["name".into(), "openTasks".into()],
            bytes: 412,
        };
        assert_eq!(PartBody::from_payload(&body.to_payload().unwrap()), body);
    }

    /// O banco pode ter sido escrito por uma versao mais nova. Perder uma parte
    /// e ruim; perder a thread inteira por causa dela e pior.
    #[test]
    fn an_unknown_payload_degrades_to_a_readable_error() {
        match PartBody::from_payload(r#"{"kind":"hologram","depth":3}"#) {
            PartBody::Error { message } => assert!(message.contains("nao conhece")),
            other => panic!("esperava Error legivel, veio {other:?}"),
        }
    }

    #[test]
    fn the_kind_matches_the_variant() {
        assert_eq!(
            PartBody::Text {
                text: String::new()
            }
            .kind(),
            "text"
        );
        assert_eq!(
            PartBody::Status {
                text: String::new()
            }
            .kind(),
            "status"
        );
    }

    /// Uma mensagem gravada como `streaming` nao volta a andar depois de o app
    /// fechar no meio do turno. `is_settled` e o que permite reparar isso na
    /// abertura em vez de mostrar uma resposta eternamente em curso.
    #[test]
    fn only_settled_statuses_are_settled() {
        assert!(!MessageStatus::Pending.is_settled());
        assert!(!MessageStatus::Streaming.is_settled());
        assert!(MessageStatus::Complete.is_settled());
        assert!(MessageStatus::Interrupted.is_settled());
        assert!(MessageStatus::Failed.is_settled());
    }

    /// Conversa nasce sem titulo: quem nomeia e o Hermes. Um "Nova conversa"
    /// gravado sobreviveria ao titulo real e viraria mentira na lista.
    #[test]
    fn a_new_conversation_has_no_invented_title() {
        assert!(NewConversation::create().title.is_empty());
    }

    #[test]
    fn an_empty_title_is_valid_and_a_huge_one_is_not() {
        assert_eq!(validate_title("  ").unwrap(), "");
        assert_eq!(validate_title(" Hermes ").unwrap(), "Hermes");
        assert!(validate_title(&"a".repeat(121)).is_err());
    }

    #[test]
    fn message_text_joins_only_the_text_parts() {
        let message = Message {
            id: MessageId::new(),
            conversation_id: ConversationId::new(),
            seq: 1,
            role: MessageRole::Assistant,
            status: MessageStatus::Complete,
            created_at: OffsetDateTime::now_utc(),
            parts: vec![
                MessagePart {
                    id: MessagePartId::new(),
                    seq: 1,
                    body: PartBody::Text { text: "Bom".into() },
                },
                MessagePart {
                    id: MessagePartId::new(),
                    seq: 2,
                    body: PartBody::Reasoning { text: "hm".into() },
                },
                MessagePart {
                    id: MessagePartId::new(),
                    seq: 3,
                    body: PartBody::Text {
                        text: " dia".into(),
                    },
                },
            ],
        };
        assert_eq!(message.text(), "Bom dia");
    }

    #[test]
    fn tool_is_not_a_message_role() {
        assert!(MessageRole::parse("tool").is_err());
        assert_eq!(
            MessageRole::parse("assistant").unwrap(),
            MessageRole::Assistant
        );
    }
}

//! Reminder: a intenção de trazer algo de volta à atenção.
//!
//! `CORE.md` §25 já definia o conceito e `CORE-FOUNDATION.md` §3.8 já fixava a
//! fronteira que este módulo respeita: Reminder **não é** prazo de Task, data
//! planejada, evento de calendário nem notificação entregue. Aqui vive a
//! intenção; a entrega vive em [`Notification`].
//!
//! A promessa que o módulo inteiro existe para sustentar, de
//! `ATTENTION-SYSTEM.md` §1.1: **nenhum Reminder é perdido em silêncio.** Um
//! Reminder pode chegar tarde, chegar discreto ou nunca virar toast — não pode
//! deixar de existir. Por isso falha de entrega não é estado terminal, e por
//! isso `missed` é estado de verdade em vez de ausência.
//!
//! Tudo aqui é puro e recebe o tempo de fora, por [`crate::Clock`]. Regra
//! temporal que lê o relógio direto é regra que ninguém testa.
//!
//! **Escopo do P0.** Só [`Trigger::At`] existe. Recorrência, follow-up e
//! watches condicionais chegam nas fases seguintes, cada um com o formato de
//! persistência decidido junto. `Relative` continua sem existir, e desde
//! 2026-09-08 por um motivo mais estreito: a ADR-066 trouxe `Task.due_at`, mas
//! a decisão D-4 continua sem entidade `Event` — e o que `Relative` precisaria
//! é de âncora de EVENTO, não de prazo. Ver o §35.3 do `ATTENTION-SYSTEM.md`.
//!
//! O prazo da Task **não muda a fronteira deste módulo**: ele diz quando o
//! trabalho vence; o Reminder diz quando o M/OS interrompe. Ter prazo não gera
//! aviso nenhum sozinho.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    AppId, CaptureId, Clock, ConversationId, CoreError, ErrorCode, LifecycleState, ProjectId,
    ResourceId, TaskId,
};

macro_rules! attention_id {
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

            /// O UUID cru, para quem enderessa esta entidade FORA do M/OS.
            ///
            /// Existe para a sincronizacao: o id que viaja entre dispositivos e
            /// o mesmo que identifica aqui. Ver `mos-sync`.
            pub fn as_uuid(&self) -> Uuid {
                self.0
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

attention_id!(ReminderId, "Reminder ID");
attention_id!(NotificationId, "Notification ID");

// ---------------------------------------------------------------- prioridade

/// Quanto direito de interromper o Reminder tem.
///
/// Não é cor (`ATTENTION-SYSTEM.md` §21). Ela decide canal, direito de furar
/// silêncio, elegibilidade para agrupamento e agressividade de escalonamento.
///
/// `Urgent` nunca é atribuída por regra automática — só pelo usuário. Uma
/// prioridade que o sistema distribui sozinho deixa de significar algo.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                // Sem o "de Reminder" desde 2026-09-08: a Task passou a usar a
                // MESMA escala (migration 0039), e duas escalas de quatro
                // degraus fariam "alta" significar duas coisas no mesmo sistema.
                "Prioridade desconhecida.",
                false,
            )),
        }
    }
}

// -------------------------------------------------------------------- estado

/// Onde a intenção está no seu ciclo.
///
/// Separado de [`LifecycleState`] pelo mesmo motivo que a ADR-015 separou
/// `processing_state` de `lifecycle_state` em Capture: uma dimensão diz o que
/// aconteceu com a intenção, a outra diz se ela aparece nas superfícies. Um
/// Reminder concluído e arquivado volta a ser concluído ao ser restaurado.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStatus {
    Scheduled,
    Due,
    Delivered,
    Acknowledged,
    Snoozed,
    Completed,
    Cancelled,
    /// Venceu e ninguém viu — máquina desligada, app fechado, sono.
    ///
    /// Estado de verdade e não ausência: ele carrega o instante ORIGINAL do
    /// vencimento, para a superfície poder dizer "perdido há 50 min" em vez de
    /// fingir que acabou de vencer.
    Missed,
    Expired,
}

impl ReminderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Due => "due",
            Self::Delivered => "delivered",
            Self::Acknowledged => "acknowledged",
            Self::Snoozed => "snoozed",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Missed => "missed",
            Self::Expired => "expired",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "due" => Ok(Self::Due),
            "delivered" => Ok(Self::Delivered),
            "acknowledged" => Ok(Self::Acknowledged),
            "snoozed" => Ok(Self::Snoozed),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "missed" => Ok(Self::Missed),
            "expired" => Ok(Self::Expired),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de Reminder desconhecido.",
                false,
            )),
        }
    }

    /// Terminal: a intenção acabou e o agendador não olha mais para ela.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Expired)
    }

    /// Conta para o badge (§21.1).
    ///
    /// Só o que realmente espera uma ação da pessoa. `Snoozed` não conta — ela
    /// já decidiu quando quer ver. `Scheduled` não conta — ainda não é hora. Um
    /// badge que sobe com coisa que não pede ação é um badge que se aprende a
    /// ignorar.
    pub fn needs_attention(self) -> bool {
        matches!(self, Self::Due | Self::Delivered | Self::Missed)
    }

    /// O agendador precisa acordar por causa dela.
    pub fn is_waiting(self) -> bool {
        matches!(self, Self::Scheduled | Self::Snoozed)
    }
}

// --------------------------------------------------------------------- alvo

/// Para onde o Reminder aponta, quando aponta.
///
/// Enum fechado com id tipado por braço, e não tabela genérica de arestas: a
/// ADR-012 recusou grafo genérico e aceitou explicitamente o custo de que
/// "novos tipos exigirão migration explícita no início". É esse custo que este
/// enum paga.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "id")]
pub enum ReminderTarget {
    Task(TaskId),
    Project(ProjectId),
    Capture(CaptureId),
    Resource(ResourceId),
    Conversation(ConversationId),
    App(AppId),
    /// O setimo braco, e ele destrava o que o `ATTENTION-SYSTEM.md` §0.2
    /// registrou como bloqueado: *"Smart Snooze 'apos a reuniao' — bloqueado —
    /// nao ha reuniao no sistema"*. Agora ha.
    ///
    /// Custo: uma migration e uma linha em cada `match`. E exatamente a
    /// consequencia que a ADR-012 aceitou ao recusar tabela generica de arestas.
    Meeting(crate::MeetingId),
}

impl ReminderTarget {
    /// O par que vai para o banco: `target_type` e `target_id`.
    pub fn as_columns(self) -> (&'static str, String) {
        match self {
            Self::Task(id) => ("task", id.to_string()),
            Self::Project(id) => ("project", id.to_string()),
            Self::Capture(id) => ("capture", id.to_string()),
            Self::Resource(id) => ("resource", id.to_string()),
            Self::Conversation(id) => ("conversation", id.to_string()),
            Self::App(id) => ("app", id.to_string()),
            Self::Meeting(id) => ("meeting", id.to_string()),
        }
    }

    pub fn from_columns(kind: &str, id: &str) -> Result<Self, CoreError> {
        match kind {
            "task" => Ok(Self::Task(TaskId::parse(id)?)),
            "project" => Ok(Self::Project(ProjectId::parse(id)?)),
            "capture" => Ok(Self::Capture(CaptureId::parse(id)?)),
            "resource" => Ok(Self::Resource(ResourceId::parse(id)?)),
            "conversation" => Ok(Self::Conversation(ConversationId::parse(id)?)),
            "app" => Ok(Self::App(AppId::parse(id)?)),
            "meeting" => Ok(Self::Meeting(crate::MeetingId::parse(id)?)),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de alvo de Reminder desconhecido.",
                false,
            )),
        }
    }
}

/// Quem criou o Reminder. Importa para o Attention Score e para auditoria.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderSource {
    /// Criado à mão numa superfície do M/OS.
    User,
    /// Proposto pelo Hermes e confirmado pelo usuário.
    Hermes,
    /// Derivado de uma Capture ao processar a Inbox.
    Capture,
    /// Criado por regra interna do próprio M/OS.
    System,
}

impl ReminderSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Hermes => "hermes",
            Self::Capture => "capture",
            Self::System => "system",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "user" => Ok(Self::User),
            "hermes" => Ok(Self::Hermes),
            "capture" => Ok(Self::Capture),
            "system" => Ok(Self::System),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Origem de Reminder desconhecida.",
                false,
            )),
        }
    }
}

// ------------------------------------------------------------------ trigger

/// A regra que decide quando o Reminder vence.
///
/// Só `At` no P0. Cada braço novo traz decisão de persistência própria, e
/// persistir formato de regra que ainda não foi desenhada é criar migration
/// para depois.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Trigger {
    /// Instante exato, em UTC.
    At {
        #[serde(with = "time::serde::rfc3339")]
        instant: OffsetDateTime,
    },
    /// Sem data. "Comprar o cabo HDMI" — importa, mas não numa hora.
    ///
    /// Não é um lembrete quebrado nem uma intenção pela metade: é a resposta
    /// honesta para o que se quer não esquecer sem se querer ser interrompido.
    /// Não gera notificação nenhuma enquanto não ganhar hora, e o único lugar em
    /// que ele aparece é a lista — e no Começar/Encerrar o Dia, que é onde uma
    /// revisão de "algum dia" cabe.
    ///
    /// Sem isto, a única forma de guardar algo sem hora seria inventar uma hora
    /// — e uma hora inventada é uma notificação que a pessoa aprende a ignorar.
    Someday,
}

impl Trigger {
    /// Quando este trigger vence a partir de agora, se vencer.
    pub fn next_due(&self, _now: OffsetDateTime) -> Option<OffsetDateTime> {
        match self {
            // Um instante no passado continua sendo o vencimento dele. Devolver
            // `None` aqui apagaria o Reminder atrasado — exatamente o que a
            // promessa da §1.1 proibe. Quem decide o que fazer com atraso e a
            // reconciliacao, nao este calculo.
            Self::At { instant } => Some(*instant),
            Self::Someday => None,
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::At { .. } => "at",
            Self::Someday => "someday",
        }
    }

    /// O instante deste trigger, quando ele tem um.
    pub fn instant(&self) -> Option<OffsetDateTime> {
        match self {
            Self::At { instant } => Some(*instant),
            Self::Someday => None,
        }
    }
}

// ------------------------------------------------------------------ política

/// Como este Reminder quer ser entregue.
///
/// Poucos campos com defaults fortes: `UX-PRINCIPLES.md` §8 pede revelar
/// complexidade sob demanda, e §88 mede a experiência por decisões
/// desnecessárias. Quem cria um lembrete quer ser lembrado, não configurar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryPolicy {
    pub snooze_allowed: bool,
    /// O que pode aparecer no corpo da notificação (§39).
    pub privacy: ContentPrivacy,
}

impl Default for DeliveryPolicy {
    fn default() -> Self {
        Self {
            snooze_allowed: true,
            privacy: ContentPrivacy::ShowContent,
        }
    }
}

/// Quanto do Reminder pode ir no payload da notificação.
///
/// Controla o que **nós** colocamos na mensagem. Onde o Windows decide mostrar
/// não está nas nossas mãos, e prometer "não aparece na tela bloqueada" seria
/// prometer o que não podemos cumprir (§39).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentPrivacy {
    ShowContent,
    TitleOnly,
    Hidden,
}

impl ContentPrivacy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShowContent => "show_content",
            Self::TitleOnly => "title_only",
            Self::Hidden => "hidden",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "show_content" => Ok(Self::ShowContent),
            "title_only" => Ok(Self::TitleOnly),
            "hidden" => Ok(Self::Hidden),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Política de privacidade desconhecida.",
                false,
            )),
        }
    }

    /// O par (título, corpo) que pode sair numa notificação.
    pub fn redact(self, title: &str, body: &str) -> (String, String) {
        match self {
            Self::ShowContent => (title.to_owned(), body.to_owned()),
            Self::TitleOnly => (title.to_owned(), String::new()),
            Self::Hidden => (
                "M/OS".to_owned(),
                "Um lembrete precisa de atenção.".to_owned(),
            ),
        }
    }
}

// ------------------------------------------------------------- notificação

/// Por onde uma entrega sai.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    InApp,
    Windows,
    Tray,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InApp => "in_app",
            Self::Windows => "windows",
            Self::Tray => "tray",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "in_app" => Ok(Self::InApp),
            "windows" => Ok(Self::Windows),
            "tray" => Ok(Self::Tray),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Canal de entrega desconhecido.",
                false,
            )),
        }
    }
}

/// Quanto a entrega se impõe (§21).
///
/// `Critical` existe e é para não ser usada. Urgência que aparece toda semana
/// deixa de ser urgência.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualLevel {
    Quiet,
    Normal,
    Important,
    Critical,
}

impl VisualLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Normal => "normal",
            Self::Important => "important",
            Self::Critical => "critical",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "quiet" => Ok(Self::Quiet),
            "normal" => Ok(Self::Normal),
            "important" => Ok(Self::Important),
            "critical" => Ok(Self::Critical),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Nivel visual desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Queued,
    Delivering,
    Delivered,
    Seen,
    Acted,
    Dismissed,
    Failed,
}

impl NotificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Delivering => "delivering",
            Self::Delivered => "delivered",
            Self::Seen => "seen",
            Self::Acted => "acted",
            Self::Dismissed => "dismissed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "queued" => Ok(Self::Queued),
            "delivering" => Ok(Self::Delivering),
            "delivered" => Ok(Self::Delivered),
            "seen" => Ok(Self::Seen),
            "acted" => Ok(Self::Acted),
            "dismissed" => Ok(Self::Dismissed),
            "failed" => Ok(Self::Failed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de notificacao desconhecido.",
                false,
            )),
        }
    }

    /// Viva o bastante para bloquear uma cópia com a mesma `dedupe_key` (§17).
    ///
    /// `Seen` NÃO conta: depois de vista, a próxima entrega é um lembrete novo
    /// e legítimo, não uma cópia. Contar `Seen` faria um Reminder recorrente
    /// silenciar para sempre depois da primeira vez.
    pub fn blocks_duplicate(self) -> bool {
        matches!(self, Self::Queued | Self::Delivering | Self::Delivered)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: NotificationId,
    pub reminder_id: ReminderId,
    pub channel: Channel,
    pub dedupe_key: String,
    pub status: NotificationStatus,
    pub level: VisualLevel,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub delivered_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resolved_at: Option<OffsetDateTime>,
    pub failure: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewNotification {
    pub id: NotificationId,
    pub reminder_id: ReminderId,
    pub channel: Channel,
    pub dedupe_key: String,
    pub level: VisualLevel,
    pub created_at: OffsetDateTime,
}

impl NewNotification {
    /// A chave que impede cópia enquanto uma equivalente está viva.
    ///
    /// `{assunto}:{id}` — o assunto separa "este lembrete venceu" de "este
    /// lembrete está atrasado há muito", que são avisos diferentes sobre o
    /// mesmo Reminder e não devem se bloquear.
    pub fn dedupe_key(subject: &str, reminder: ReminderId) -> String {
        format!("{subject}:{reminder}")
    }

    pub fn queued(
        reminder: ReminderId,
        channel: Channel,
        subject: &str,
        level: VisualLevel,
        now: OffsetDateTime,
    ) -> Self {
        Self {
            id: NotificationId::new(),
            reminder_id: reminder,
            channel,
            dedupe_key: Self::dedupe_key(subject, reminder),
            level,
            created_at: now,
        }
    }
}

// ----------------------------------------------------------------- reminder

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: ReminderId,
    pub title: String,
    pub body: String,
    pub target: Option<ReminderTarget>,
    pub trigger: Trigger,
    pub priority: Priority,
    pub status: ReminderStatus,
    pub policy: DeliveryPolicy,
    pub source: ReminderSource,
    /// Quando vence, ou quando venceu. Persistido porque é a coluna que o
    /// agendador consulta; recalcular o trigger de todos a cada tick trocaria
    /// uma query indexada por um laço.
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_due_at: Option<OffsetDateTime>,
    pub snooze_count: u32,
    pub delivered_count: u32,

    /// Que PERGUNTA a superfície faz sobre este lembrete.
    ///
    /// Um campo, e não uma segunda entidade: um follow-up tem o mesmo ciclo de
    /// vida, o mesmo agendador e o mesmo estado. O que muda é o texto dos dois
    /// botões — "Concluir/Adiar" vira "Respondeu/Ainda não".
    pub kind: ReminderKind,
    /// De quem se está esperando, quando [`ReminderKind::FollowUp`].
    pub waiting_for: String,

    /// "Não me deixa esquecer disso."
    ///
    /// Um lembrete persistente NÃO se resolve por ter sido entregue: ele volta,
    /// em intervalos que crescem, até a pessoa concluir, adiar, reagendar ou
    /// cancelar. É a resposta direta ao ciclo que o produto existe para quebrar
    /// — dispara, ignoro, some, esqueço.
    pub persistent: bool,
    /// Em que degrau do re-alerta ele está. Ver [`escalation_delay`].
    pub escalation_step: u32,
    /// Quando tocou pela última vez.
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_triggered_at: Option<OffsetDateTime>,
    /// Quando o próximo re-alerta é devido.
    ///
    /// Coluna própria e não `next_due_at` reaproveitada: `next_due_at` carrega o
    /// vencimento ORIGINAL, e é ele que sustenta o "atrasado há 2 h". Empurrá-lo
    /// a cada re-alerta apagaria o tamanho do atraso.
    #[serde(with = "time::serde::rfc3339::option")]
    pub retry_at: Option<OffsetDateTime>,
    /// A regra de repetição. `None` é o normal: acontece uma vez.
    pub recurrence: Option<crate::Recurrence>,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub lifecycle_state: LifecycleState,
}

impl Reminder {
    /// Há quanto tempo passou do vencimento, se passou.
    pub fn overdue_by(&self, now: OffsetDateTime) -> Option<Duration> {
        let due = self.next_due_at?;
        (now > due).then(|| now - due)
    }

    /// A partir do quinto adiamento a superfície oferece reagendar ou cancelar
    /// junto do adiar (§13). Adiar quinze vezes é o sistema falhando em ajudar a
    /// decidir; oferecer só "adiar" é cumplicidade.
    pub fn snooze_fatigue(&self) -> bool {
        self.snooze_count >= 5
    }
}

/// Um Reminder a ser criado.
#[derive(Clone, Debug)]
pub struct NewReminder {
    pub id: ReminderId,
    pub title: String,
    pub body: String,
    pub target: Option<ReminderTarget>,
    pub trigger: Trigger,
    pub priority: Priority,
    pub policy: DeliveryPolicy,
    pub source: ReminderSource,
    pub next_due_at: Option<OffsetDateTime>,
    pub kind: ReminderKind,
    pub waiting_for: String,
    pub persistent: bool,
    pub recurrence: Option<crate::Recurrence>,
    pub created_at: OffsetDateTime,
}

/// Quanto no passado ainda vale criar um Reminder.
///
/// Não é zero de propósito: entre o usuário escolher "em 1 minuto" e o comando
/// chegar ao domínio passam milissegundos, e um relógio ligeiramente à frente
/// tornaria a criação impossível por motivo invisível. Passado além disso é
/// erro de entrada de verdade — pedir para ser lembrado ontem.
const CREATION_GRACE: Duration = Duration::minutes(1);

impl NewReminder {
    pub fn at(
        title: &str,
        body: &str,
        instant: OffsetDateTime,
        clock: &dyn Clock,
    ) -> Result<Self, CoreError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O lembrete precisa de um titulo.",
                false,
            ));
        }

        let now = clock.now();
        if instant < now - CREATION_GRACE {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Nao da para ser lembrado de algo no passado.",
                false,
            ));
        }

        Ok(Self {
            id: ReminderId::new(),
            title: title.to_owned(),
            body: body.trim().to_owned(),
            target: None,
            trigger: Trigger::At { instant },
            priority: Priority::Normal,
            policy: DeliveryPolicy::default(),
            source: ReminderSource::User,
            next_due_at: Some(instant),
            kind: ReminderKind::Standard,
            waiting_for: String::new(),
            persistent: false,
            recurrence: None,
            created_at: now,
        })
    }

    /// Um lembrete SEM data. Ver [`Trigger::Someday`].
    ///
    /// Não passa pela checagem de passado porque não há instante a checar — e é
    /// justamente essa ausência que o torna útil.
    pub fn someday(title: &str, body: &str, clock: &dyn Clock) -> Result<Self, CoreError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O lembrete precisa de um titulo.",
                false,
            ));
        }
        Ok(Self {
            id: ReminderId::new(),
            title: title.to_owned(),
            body: body.trim().to_owned(),
            target: None,
            trigger: Trigger::Someday,
            priority: Priority::Normal,
            policy: DeliveryPolicy::default(),
            source: ReminderSource::User,
            next_due_at: None,
            kind: ReminderKind::Standard,
            waiting_for: String::new(),
            persistent: false,
            recurrence: None,
            created_at: clock.now(),
        })
    }

    /// "Não me deixa esquecer": o lembrete passa a insistir.
    pub fn persisting(mut self) -> Self {
        self.persistent = true;
        self
    }

    /// A regra de repetição. Recusa regra impossível AQUI, e não ao repetir:
    /// uma regra inválida gravada é um lembrete que para de repetir sem ninguém
    /// perceber.
    pub fn repeating(mut self, recurrence: crate::Recurrence) -> Result<Self, CoreError> {
        recurrence.validate()?;
        self.recurrence = Some(recurrence);
        Ok(self)
    }

    /// Cobrar terceiro. O título continua sendo o da pessoa que criou; o que
    /// muda é a pergunta que a superfície faz quando ele toca.
    pub fn following_up(mut self, who: &str) -> Self {
        self.kind = ReminderKind::FollowUp;
        self.waiting_for = who.trim().to_owned();
        self
    }

    pub fn with_policy(mut self, policy: DeliveryPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_target(mut self, target: ReminderTarget) -> Self {
        self.target = Some(target);
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn from_source(mut self, source: ReminderSource) -> Self {
        self.source = source;
        self
    }
}

// -------------------------------------------------------------- transições

/// O que se pede a um Reminder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    /// Chegou a hora.
    Ring,
    /// Uma entrega saiu.
    Deliver,
    /// A pessoa reconheceu ter visto.
    Acknowledge,
    /// Adiar até um instante.
    Snooze { until: OffsetDateTime },
    /// A pessoa resolveu.
    Complete,
    /// A pessoa desistiu.
    Cancel,
    /// A reconciliação achou vencido e nunca entregue.
    Miss,
    /// Perdeu utilidade sem acao, e havia politica de expiracao.
    Expire,
    /// O re-alerta do lembrete persistente. Sobe um degrau e remarca o próximo.
    ///
    /// **Não muda o estado.** Um lembrete que insiste continua vencido,
    /// continua contando o atraso desde o instante ORIGINAL e continua no
    /// Attention Center. Insistir não é um estado — é uma entrega a mais sobre
    /// o mesmo estado, que é a separação inteira entre Reminder e Notification.
    Escalate,
    /// A entrega foi segurada — horas de silêncio, canal indisponível.
    ///
    /// Diferente de [`Transition::Snooze`] em tudo que importa: não conta
    /// fadiga, não muda `next_due_at` e não é decisão da pessoa. Só marca
    /// quando tentar de novo.
    Defer { until: OffsetDateTime },
}

impl Transition {
    fn name(self) -> &'static str {
        match self {
            Self::Ring => "vencer",
            Self::Deliver => "entregar",
            Self::Acknowledge => "reconhecer",
            Self::Snooze { .. } => "adiar",
            Self::Complete => "concluir",
            Self::Cancel => "cancelar",
            Self::Miss => "marcar como perdido",
            Self::Expire => "expirar",
            Self::Escalate => "insistir",
            Self::Defer { .. } => "segurar",
        }
    }
}

// -------------------------------------------------------------- edição

/// O que se quer mudar num Reminder. `None` é "deixa como está".
///
/// # Por que campo a campo, e não o Reminder inteiro
///
/// Duas telas editam o mesmo lembrete, e o sync resolve conflito **por campo**.
/// Mandar o objeto inteiro faria a tela que só mexeu no título reescrever também
/// a hora — com o valor que ela tinha lido antes — e o sync não teria como saber
/// que aquilo não foi uma edição. Um `None` aqui é a diferença entre "não mexi"
/// e "mexi para o mesmo valor".
#[derive(Clone, Debug, Default)]
pub struct EditReminder {
    pub title: Option<String>,
    pub body: Option<String>,
    /// A hora nova. Reagendar, e não adiar: `Snooze` empurra e conta fadiga,
    /// porque adiar quinze vezes é um sinal; corrigir a hora que se digitou
    /// errado não é.
    pub instant: Option<OffsetDateTime>,
    pub priority: Option<Priority>,
}

impl EditReminder {
    /// Nada foi pedido. Serve para a superfície não gravar por engano.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.body.is_none()
            && self.instant.is_none()
            && self.priority.is_none()
    }
}

/// Aplica uma edição, ou explica por que não pode.
///
/// Pura e sem repositório, pela mesma razão de [`apply`]: as regras que podem
/// estar erradas são estas, e regra sem teste é regra que ninguém conferiu.
///
/// # As três regras que não são óbvias
///
/// **Lembrete terminal não se edita.** Concluído e cancelado são respostas já
/// dadas; mudar o título de algo que você já resolveu não é edição, é ressuscitar
/// pela porta dos fundos — e o caminho para isso é uma transição, não um campo.
///
/// **Mudar a hora para o futuro devolve o lembrete a `Scheduled`.** Um lembrete
/// que já tocou e foi remarcado para amanhã não pode continuar marcado como
/// vencido: ele voltaria a cobrar atenção na tela por uma hora que já não é a
/// dele. Isto é o que separa reagendar de adiar.
///
/// **A contagem de adiamentos não muda.** Reagendar não é adiar, e zerar a
/// contagem aqui apagaria justamente o sinal de fadiga que o sistema usa para
/// oferecer ajuda depois do quinto adiamento.
pub fn edit(
    reminder: &Reminder,
    mudanca: EditReminder,
    now: OffsetDateTime,
) -> Result<Reminder, CoreError> {
    if reminder.status.is_terminal() {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "Este lembrete ja foi resolvido; nao da para edita-lo.",
            false,
        ));
    }

    let mut novo = reminder.clone();

    if let Some(titulo) = mudanca.title {
        let titulo = titulo.trim();
        if titulo.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O lembrete precisa de um titulo.",
                false,
            ));
        }
        novo.title = titulo.to_owned();
    }

    if let Some(corpo) = mudanca.body {
        novo.body = corpo.trim().to_owned();
    }

    if let Some(priority) = mudanca.priority {
        novo.priority = priority;
    }

    if let Some(instante) = mudanca.instant {
        // A mesma folga da criação, e pelo mesmo motivo: entre escolher "daqui a
        // um minuto" e o comando chegar aqui passam milissegundos.
        if instante < now - CREATION_GRACE {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Nao da para ser lembrado de algo no passado.",
                false,
            ));
        }
        novo.trigger = Trigger::At { instant: instante };
        novo.next_due_at = Some(instante);
        if instante > now && !matches!(novo.status, ReminderStatus::Snoozed) {
            novo.status = ReminderStatus::Scheduled;
        }
        // Remarcar cancela a insistencia da hora antiga: o lembrete tem hora
        // nova, e continuar cobrando a antiga seria cobrar por uma decisao que
        // a pessoa acabou de tomar.
        novo.escalation_step = 0;
        novo.retry_at = None;
    }

    novo.updated_at = now;
    Ok(novo)
}

/// Aplica uma transição, ou explica por que não pode.
///
/// Função e não método com `&mut` de propósito: devolver o Reminder novo deixa o
/// caso de erro sem meio-estado, e é o que permite testar a matriz inteira sem
/// montar repositório.
pub fn apply(
    reminder: &Reminder,
    transition: Transition,
    now: OffsetDateTime,
) -> Result<Reminder, CoreError> {
    use ReminderStatus::*;

    let refused = || {
        Err(CoreError::new(
            ErrorCode::InvalidTransition,
            format!(
                "Nao da para {} um lembrete {}.",
                transition.name(),
                reminder.status.as_str()
            ),
            false,
        ))
    };

    let mut next = reminder.clone();
    next.updated_at = now;

    match (reminder.status, transition) {
        // O tempo chegou. So quem esperava pode vencer.
        (Scheduled | Snoozed, Transition::Ring) => {
            next.status = Due;
            next.last_triggered_at = Some(now);
            // Vencer zera o degrau: cada vencimento e uma cobranca nova, e
            // herdar o degrau do vencimento anterior faria um lembrete
            // recorrente insistir menos a cada semana ate parar de insistir.
            next.escalation_step = 0;
            next.retry_at = arm_retry(&next, 0, now);
        }

        // Uma entrega saiu. `Delivered` é alcançável de `Missed` porque um
        // Reminder perdido continua entregável — é assim que "enquanto você
        // esteve fora" chega até a pessoa.
        (Due | Delivered | Missed, Transition::Deliver) => {
            next.status = Delivered;
            next.delivered_count = reminder.delivered_count.saturating_add(1);
        }

        (Due | Delivered | Missed, Transition::Acknowledge) => {
            next.status = Acknowledged;
            // Ver nao e resolver. O que o reconhecimento compra e o SILENCIO:
            // o lembrete para de insistir, e continua existindo — persistente
            // ou nao, ele so sai da frente por concluir, adiar ou cancelar.
            next.retry_at = None;
        }

        // Adiar é permitido de qualquer estado não terminal, inclusive
        // `Scheduled`: empurrar algo que ainda não venceu é uso legítimo.
        (state, Transition::Snooze { until }) if !state.is_terminal() => {
            if !reminder.policy.snooze_allowed {
                return Err(CoreError::new(
                    ErrorCode::InvalidTransition,
                    "Este lembrete nao pode ser adiado.",
                    false,
                ));
            }
            if until <= now {
                return Err(CoreError::new(
                    ErrorCode::InvalidInput,
                    "Adiar para o passado nao adia nada.",
                    false,
                ));
            }
            next.status = Snoozed;
            next.next_due_at = Some(until);
            next.snooze_count = reminder.snooze_count.saturating_add(1);
            // Adiar e uma decisao: ela cancela a insistencia ate a nova hora.
            next.escalation_step = 0;
            next.retry_at = None;
        }

        // Concluir uma ocorrencia de uma serie NAO encerra a serie.
        //
        // O lembrete recorrente e UMA linha que avanca, e nao uma linha por
        // ocorrencia: um "todo dia as 08:00" criado hoje geraria centenas de
        // linhas em um ano, e nenhuma delas seria consultada de novo. O que
        // aconteceu fica no historico (`reminder_events`), que e onde alguem
        // procura — e a `ATTENTION-SYSTEM.md` §8 ja pedia que a serie nao
        // reescrevesse o passado. Ela nao reescreve: ela nao guarda o passado
        // na propria linha.
        (state, Transition::Complete) if !state.is_terminal() => {
            match advance_recurrence(reminder, now) {
                Some(proxima) => {
                    next.status = Scheduled;
                    next.trigger = Trigger::At { instant: proxima };
                    next.next_due_at = Some(proxima);
                    next.completed_at = None;
                    next.snooze_count = 0;
                    next.escalation_step = 0;
                    next.retry_at = None;
                }
                None => {
                    next.status = Completed;
                    next.completed_at = Some(now);
                    next.next_due_at = None;
                    next.retry_at = None;
                    next.escalation_step = 0;
                }
            }
        }

        (state, Transition::Cancel) if !state.is_terminal() => {
            next.status = Cancelled;
            next.next_due_at = None;
            next.retry_at = None;
        }

        // Perdido preserva `next_due_at`: é o instante original que permite
        // dizer "perdido há 50 min". Zerar aqui apagaria a única informação que
        // distingue um atraso de dez minutos de um de três dias.
        (Scheduled | Snoozed | Due, Transition::Miss) => {
            next.status = Missed;
            next.last_triggered_at = Some(now);
            next.escalation_step = 0;
            next.retry_at = arm_retry(&next, 0, now);
        }

        (state, Transition::Expire) if !state.is_terminal() => {
            next.status = Expired;
            next.next_due_at = None;
            next.retry_at = None;
        }

        // Insistir: sobe um degrau e remarca. O estado NAO muda — ver o
        // comentario em `Transition::Escalate`.
        (Due | Delivered | Missed, Transition::Escalate) => {
            let degrau = reminder.escalation_step.saturating_add(1);
            next.escalation_step = degrau;
            next.last_triggered_at = Some(now);
            next.retry_at = arm_retry(&next, degrau, now);
        }

        // Segurar a entrega. Nao e adiamento e nao conta fadiga.
        (state, Transition::Defer { until }) if !state.is_terminal() => {
            next.retry_at = Some(until);
        }

        _ => return refused(),
    }

    Ok(next)
}

/// Quando o próximo re-alerta deve sair, dado o degrau que acabou de ser
/// cumprido. `None` quando o orçamento acabou — o lembrete para de tocar e
/// continua existindo.
///
/// Fora de [`apply`] porque ela é chamada de três braços da matriz, e uma regra
/// de escalonamento escrita três vezes é uma regra que vai divergir na quarta.
fn arm_retry(reminder: &Reminder, step: u32, now: OffsetDateTime) -> Option<OffsetDateTime> {
    if step >= retry_budget(reminder) {
        return None;
    }
    escalation_delay(step).map(|delay| now + delay)
}

/// A próxima ocorrência de uma série, se houver série.
///
/// A âncora é o que separa os dois tipos de repetição, e a diferença é de
/// produto:
///
/// - **Fixa**: parte do vencimento planejado. "Toda segunda às 09:00" continua
///   sendo toda segunda, mesmo que a da semana passada só tenha sido resolvida
///   na quarta — senão a série inteira escorregaria para quarta.
/// - **Por conclusão**: parte de AGORA, o instante em que se concluiu. É o que
///   "limpar o computador a cada 30 dias depois que eu fizer" pede, e o que
///   torna a repetição útil para manutenção em vez de irritante.
fn advance_recurrence(reminder: &Reminder, now: OffsetDateTime) -> Option<OffsetDateTime> {
    let recurrence = reminder.recurrence.as_ref()?;
    if recurrence.validate().is_err() {
        // Regra corrompida: melhor a série terminar do que o lembrete voltar em
        // hora inventada. O erro fica visível porque o lembrete conclui de vez.
        return None;
    }
    let anchor = match recurrence.anchor {
        crate::RecurrenceAnchor::Completion => now,
        crate::RecurrenceAnchor::Fixed => reminder.next_due_at.unwrap_or(now),
    };
    let proxima = recurrence.next_after(anchor)?;
    // Uma série fixa muito atrasada devolveria uma ocorrência no passado, e um
    // lembrete que nasce vencido é um lembrete que vence duas vezes na mesma
    // acordada. Avança até sair na frente do relógio.
    if proxima <= now {
        return recurrence.next_after(now);
    }
    Some(proxima)
}

/// Quem já pode receber o re-alerta.
///
/// Separada de [`reconcile`] porque as duas perguntas são diferentes: aquela
/// pergunta "o que venceu?", esta pergunta "o que continua sem resposta há tempo
/// bastante para eu insistir?". Juntá-las faria um lembrete vencer duas vezes.
pub fn pending_retries(reminders: &[Reminder], now: OffsetDateTime) -> Vec<ReminderId> {
    reminders
        .iter()
        .filter(|reminder| reminder.lifecycle_state == LifecycleState::Active)
        .filter(|reminder| !reminder.status.is_terminal())
        .filter(|reminder| reminder.retry_at.is_some_and(|retry| retry <= now))
        .map(|reminder| reminder.id)
        .collect()
}

// ----------------------------------------------------------- reconciliação

/// Por que a reconciliação mexeu num Reminder.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconcileReason {
    /// Venceu há pouco: entrega normalmente, ninguém perdeu nada.
    DueNow,
    /// Venceu enquanto o M/OS não estava olhando.
    MissedWhileAway,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reconciliation {
    pub id: ReminderId,
    pub reason: ReconcileReason,
    /// Quando deveria ter vencido.
    pub was_due_at: OffsetDateTime,
}

/// Além disto, o vencimento não é "de agora" — é atraso, e a pessoa precisa
/// saber que perdeu.
///
/// Cinco minutos porque é a ordem de grandeza de um app que acabou de abrir ou
/// de um tick que atrasou; acima disso já houve máquina desligada, sono ou app
/// fechado, e apresentar aquilo como se fosse agora seria mentir sobre o
/// tamanho do atraso.
pub const MISS_GRACE: Duration = Duration::minutes(5);

/// O que fazer com os Reminders pendentes ao abrir o app, ao voltar de sono ou
/// depois de restaurar um backup.
///
/// **Idempotente.** Rodar duas vezes no mesmo estado dá o mesmo resultado, o que
/// é o que permite chamá-la em todo tick do agendador sem medo (§30).
///
/// Não entrega nada e não muda nada: devolve o que precisa mudar. Quem aplica é
/// o serviço, numa transação — separar as duas coisas é o que torna a regra
/// testável sem banco.
pub fn reconcile(reminders: &[Reminder], now: OffsetDateTime) -> Vec<Reconciliation> {
    reminders
        .iter()
        .filter(|reminder| reminder.lifecycle_state == LifecycleState::Active)
        .filter(|reminder| reminder.status.is_waiting())
        .filter_map(|reminder| {
            let due = reminder.next_due_at?;
            if due > now {
                return None;
            }
            Some(Reconciliation {
                id: reminder.id,
                was_due_at: due,
                reason: if now - due > MISS_GRACE {
                    ReconcileReason::MissedWhileAway
                } else {
                    ReconcileReason::DueNow
                },
            })
        })
        .collect()
}

/// O próximo instante em que o agendador precisa acordar.
///
/// `None` significa "dorme sem prazo": não há nada esperando, e acordar de novo
/// só gastaria energia. Um único timer para todos (§7.3) — um timer por Reminder
/// não escala e não precisa existir.
pub fn next_wake(reminders: &[Reminder]) -> Option<OffsetDateTime> {
    let vencimentos = reminders
        .iter()
        .filter(|reminder| reminder.lifecycle_state == LifecycleState::Active)
        .filter(|reminder| reminder.status.is_waiting())
        .filter_map(|reminder| reminder.next_due_at);

    // O re-alerta e a segunda razao para acordar, e ela e tao real quanto a
    // primeira: um lembrete persistente entregue as 20:30 precisa que alguem
    // esteja de pe as 21:00. Sem esta metade, a insistencia so aconteceria por
    // acaso, no teto de quinze minutos do agendador.
    let reaertas = reminders
        .iter()
        .filter(|reminder| reminder.lifecycle_state == LifecycleState::Active)
        .filter(|reminder| !reminder.status.is_terminal())
        .filter_map(|reminder| reminder.retry_at);

    vencimentos.chain(reaertas).min()
}

// ------------------------------------------------------------------- tipo

/// Que pergunta a superfície faz quando este lembrete toca.
///
/// Dois valores, e não um catálogo: cada valor novo aqui é um par de botões
/// novo em toda superfície que mostra lembrete. `FollowUp` ganhou o direito
/// porque a pergunta é genuinamente outra — "o Victor respondeu?" não se
/// responde com "concluir".
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderKind {
    #[default]
    Standard,
    /// Cobrar terceiro. Ver `tasks.waiting_for`, migration 0039.
    FollowUp,
}

impl ReminderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::FollowUp => "follow_up",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "standard" => Ok(Self::Standard),
            "follow_up" => Ok(Self::FollowUp),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de Reminder desconhecido.",
                false,
            )),
        }
    }
}

// ------------------------------------------------------- escalonamento

/// Quanto esperar antes do próximo re-alerta, dado o degrau já cumprido.
///
/// Trinta minutos, uma hora, duas horas — e então `None`, que quer dizer **para
/// de tocar**. Não para de existir: o lembrete continua em Needs Attention, no
/// Attention Center, no Começar o Dia e no Encerrar o Dia. Só deixa de
/// interromper.
///
/// # Por que os intervalos crescem, e por que eles acabam
///
/// Crescem porque um lembrete que insiste no mesmo intervalo vira alarme, e
/// alarme se desliga. Acabam porque o oposto do esquecimento não é a insistência
/// infinita: é a coisa continuar visível onde a pessoa vai olhar. Um sistema que
/// toca de duas em duas horas para sempre é um sistema que ensina a ignorar
/// notificação — e aí ele perde o único poder que tinha.
///
/// Determinístico e auditável de propósito (`ATTENTION-SYSTEM.md` §30): a pessoa
/// consegue prever quando vai ser incomodada de novo, e isso é o que permite
/// confiar no sistema em vez de negociar com ele.
pub fn escalation_delay(step: u32) -> Option<Duration> {
    match step {
        0 => Some(Duration::minutes(30)),
        1 => Some(Duration::hours(1)),
        2 => Some(Duration::hours(2)),
        _ => None,
    }
}

/// Quantos avisos este lembrete já merece ter recebido desde que venceu.
///
/// Zero é "só o primeiro"; um é "o primeiro mais uma insistência"; e assim por
/// diante, até o teto de [`retry_budget`].
///
/// # Por que ela existe, e por que ela é pura
///
/// Porque há **dois** lugares que decidem quando insistir, e eles não podem
/// discordar. No desktop, o agendador escreve `retry_at` e conta degraus na
/// própria linha. Na VPS, o laço de push do `mos-web` **lê e não escreve**
/// (`avisos.rs`) — dois agendadores disputando a mesma coluna produziriam o
/// lembrete que some do PC porque o celular achou que já tinha dado conta.
///
/// Esta função é como o segundo chega à mesma resposta sem escrever nada:
/// derivada do vencimento e do relógio, ela devolve o mesmo degrau que o
/// primeiro teria alcançado. Os intervalos são os de [`escalation_delay`], e é
/// por virem dali que os dois lados continuam iguais quando um deles mudar.
pub fn alert_slot(reminder: &Reminder, now: OffsetDateTime) -> u32 {
    let Some(atraso) = reminder.overdue_by(now) else {
        return 0;
    };
    let teto = retry_budget(reminder);
    let mut acumulado = Duration::ZERO;
    let mut degrau = 0;
    while degrau < teto {
        let Some(intervalo) = escalation_delay(degrau) else {
            break;
        };
        acumulado += intervalo;
        if atraso < acumulado {
            break;
        }
        degrau += 1;
    }
    degrau
}

/// Quantos re-alertas este lembrete tem direito a receber.
///
/// Três degraus para o persistente, um para o importante, nenhum para o normal.
///
/// A escala de prioridade sozinha NÃO promove ninguém a persistente: `Urgent`
/// nunca é atribuída por regra automática (§6.1), e persistir é uma decisão
/// explícita da pessoa — "não me deixa esquecer". Um sistema que decide sozinho
/// insistir é um sistema que se aprende a silenciar.
pub fn retry_budget(reminder: &Reminder) -> u32 {
    if reminder.persistent {
        3
    } else if reminder.priority >= Priority::High {
        1
    } else {
        0
    }
}

// --------------------------------------------------------- horas de silêncio

/// A janela em que o M/OS não interrompe.
///
/// Minutos desde a meia-noite LOCAL. A janela pode atravessar a meia-noite — e
/// atravessa, no caso normal de 23:00 às 08:00.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuietHours {
    pub enabled: bool,
    pub start_minute: u16,
    pub end_minute: u16,
    /// Se `Urgent` pode furar o silêncio. Desligado por default: um sistema que
    /// decide sozinho que algo merece acordar a pessoa perde o direito de ser
    /// levado a sério quando algo realmente merecer.
    pub allow_urgent: bool,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: true,
            start_minute: 0,
            end_minute: 8 * 60,
            allow_urgent: false,
        }
    }
}

impl QuietHours {
    /// Este minuto do dia cai dentro do silêncio?
    pub fn contains(&self, local_minute: u16) -> bool {
        if !self.enabled || self.start_minute == self.end_minute {
            return false;
        }
        if self.start_minute < self.end_minute {
            local_minute >= self.start_minute && local_minute < self.end_minute
        } else {
            // Atravessa a meia-noite: 23:00–08:00 é "depois das 23" OU "antes das 8".
            local_minute >= self.start_minute || local_minute < self.end_minute
        }
    }

    /// Quando a entrega pode sair, se ela não puder sair agora.
    ///
    /// `None` significa "pode sair agora". `Some(t)` é o fim da janela de
    /// silêncio, e é para lá que o re-alerta é remarcado.
    ///
    /// O Reminder **não é adiado** por isto: ele continua vencido, continua
    /// contando o atraso e continua no Attention Center. O que espera é a
    /// NOTIFICAÇÃO — e é exatamente por Reminder e Notification serem coisas
    /// diferentes que dá para segurar uma sem mexer na outra.
    pub fn defer(
        &self,
        instant: OffsetDateTime,
        offset: time::UtcOffset,
        priority: Priority,
    ) -> Option<OffsetDateTime> {
        if !self.enabled {
            return None;
        }
        if self.allow_urgent && priority == Priority::Urgent {
            return None;
        }

        let local = instant.to_offset(offset);
        let minute = u16::from(local.hour()) * 60 + u16::from(local.minute());
        if !self.contains(minute) {
            return None;
        }

        let end_hour = (self.end_minute / 60) as u8;
        let end_minute = (self.end_minute % 60) as u8;
        let time = time::Time::from_hms(end_hour.min(23), end_minute.min(59), 0)
            .unwrap_or(time::Time::MIDNIGHT);
        let mut fim = local.date().with_time(time).assume_offset(offset);
        if fim <= local {
            fim += Duration::days(1);
        }
        Some(fim)
    }
}

/// O que é do APARELHO, e por isso não sincroniza.
///
/// Silenciar o celular à noite não pode silenciar o PC do escritório: são duas
/// telas em dois lugares, e a mesma pessoa quer respostas diferentes de cada
/// uma. Mesmo desenho de `tracking_settings` (migration 0011).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionSettings {
    pub quiet: QuietHours,
    /// O canal do sistema operacional. Opt-out, e não opt-in: quem instalou um
    /// sistema de lembretes quer ser lembrado fora da janela do app.
    pub os_channel_enabled: bool,
    /// O deslocamento local deste aparelho, em minutos.
    ///
    /// Guardado porque o processo que decide o silêncio nem sempre consegue
    /// perguntar ao sistema: o `time` sem a feature `local-offset` não sabe o
    /// fuso, e a VPS que roda o `mos-web` está em UTC enquanto a pessoa não
    /// está. O desktop reescreve isto a cada abertura, com o valor real.
    pub local_offset_minutes: i16,
}

impl Default for AttentionSettings {
    fn default() -> Self {
        Self {
            quiet: QuietHours::default(),
            os_channel_enabled: true,
            // Horário de Brasília. Ver a migration 0040 para o porquê de um
            // default explícito em vez de zero.
            local_offset_minutes: -180,
        }
    }
}

impl AttentionSettings {
    /// O deslocamento, pronto para uso. Valor impossível vira UTC em vez de
    /// derrubar o agendador — um fuso corrompido não pode calar os lembretes.
    pub fn offset(&self) -> time::UtcOffset {
        time::UtcOffset::from_whole_seconds(i32::from(self.local_offset_minutes) * 60)
            .unwrap_or(time::UtcOffset::UTC)
    }
}

// ------------------------------------------------------- pilha de alertas

attention_id!(ReminderTriggerId, "Trigger ID");

/// Como um alerta da pilha nasceu.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackTriggerKind {
    /// Antes do vencimento, por um adiantamento escolhido. "1 dia antes".
    Lead,
    /// No próprio vencimento.
    AtDue,
    /// Acrescentado à mão, sem relação com o prazo.
    Extra,
}

impl StackTriggerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::AtDue => "at_due",
            Self::Extra => "extra",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "lead" => Ok(Self::Lead),
            "at_due" => Ok(Self::AtDue),
            "extra" => Ok(Self::Extra),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de alerta desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackTriggerStatus {
    Pending,
    Fired,
    /// Passou sem tocar — a máquina estava desligada e um alerta mais recente da
    /// mesma pilha já cobriu o assunto.
    Skipped,
    Cancelled,
}

impl StackTriggerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Fired => "fired",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "pending" => Ok(Self::Pending),
            "fired" => Ok(Self::Fired),
            "skipped" => Ok(Self::Skipped),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de alerta desconhecido.",
                false,
            )),
        }
    }
}

/// Um dos vários alertas de UMA intenção.
///
/// "Entrega do projeto, sexta 17:00" com alerta um dia antes, quatro horas
/// antes, uma hora antes e no prazo é **um** Reminder com quatro destes — e não
/// quatro Reminders competindo pela mesma linha da tela. É a diferença entre uma
/// lista que se lê e uma lista que se abandona.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderTrigger {
    pub id: ReminderTriggerId,
    pub reminder_id: ReminderId,
    #[serde(with = "time::serde::rfc3339")]
    pub scheduled_at: OffsetDateTime,
    pub kind: StackTriggerKind,
    pub lead_minutes: Option<u32>,
    pub status: StackTriggerStatus,
    #[serde(with = "time::serde::rfc3339::option")]
    pub fired_at: Option<OffsetDateTime>,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl ReminderTrigger {
    /// Como este alerta se lê na tela: "1 dia antes", "no prazo".
    pub fn describe(&self) -> String {
        match (self.kind, self.lead_minutes) {
            (StackTriggerKind::AtDue, _) => "no prazo".to_owned(),
            (StackTriggerKind::Lead, Some(minutes)) => format!("{} antes", humanize_lead(minutes)),
            _ => "alerta".to_owned(),
        }
    }
}

/// Minutos em palavras. "1 dia", "4 horas", "15 min".
fn humanize_lead(minutes: u32) -> String {
    if minutes >= 24 * 60 && minutes % (24 * 60) == 0 {
        let days = minutes / (24 * 60);
        return if days == 1 {
            "1 dia".to_owned()
        } else {
            format!("{days} dias")
        };
    }
    if minutes >= 60 && minutes % 60 == 0 {
        let hours = minutes / 60;
        return if hours == 1 {
            "1 hora".to_owned()
        } else {
            format!("{hours} horas")
        };
    }
    format!("{minutes} min")
}

#[derive(Clone, Debug)]
pub struct NewReminderTrigger {
    pub id: ReminderTriggerId,
    pub reminder_id: ReminderId,
    pub scheduled_at: OffsetDateTime,
    pub kind: StackTriggerKind,
    pub lead_minutes: Option<u32>,
    pub created_at: OffsetDateTime,
}

impl NewReminderTrigger {
    /// Um alerta no próprio vencimento.
    pub fn at_due(reminder: ReminderId, due: OffsetDateTime, now: OffsetDateTime) -> Self {
        Self {
            id: ReminderTriggerId::new(),
            reminder_id: reminder,
            scheduled_at: due,
            kind: StackTriggerKind::AtDue,
            lead_minutes: None,
            created_at: now,
        }
    }

    /// Um alerta com adiantamento. `None` quando o adiantamento cairia antes de
    /// agora — um alerta "1 dia antes" criado a três horas do prazo não é um
    /// alerta, é ruído imediato.
    pub fn lead(
        reminder: ReminderId,
        due: OffsetDateTime,
        minutes: u32,
        now: OffsetDateTime,
    ) -> Option<Self> {
        let scheduled_at = due - Duration::minutes(i64::from(minutes));
        (scheduled_at > now).then_some(Self {
            id: ReminderTriggerId::new(),
            reminder_id: reminder,
            scheduled_at,
            kind: StackTriggerKind::Lead,
            lead_minutes: Some(minutes),
            created_at: now,
        })
    }

    pub fn extra(reminder: ReminderId, instant: OffsetDateTime, now: OffsetDateTime) -> Self {
        Self {
            id: ReminderTriggerId::new(),
            reminder_id: reminder,
            scheduled_at: instant,
            kind: StackTriggerKind::Extra,
            lead_minutes: None,
            created_at: now,
        }
    }
}

/// Os adiantamentos que a superfície oferece, em minutos.
///
/// Seis, e nenhum deles marcado por default. `ATTENTION-SYSTEM.md` §17 é
/// explícito: o M/OS **pode sugerir** um alerta a mais para um prazo importante,
/// e **não pode** criar vários sozinho. Quatro notificações que a pessoa não
/// pediu para uma entrega qualquer é o começo de ela desligar todas.
pub const LEAD_PRESETS: &[(u32, &str)] = &[
    (15, "15 min antes"),
    (60, "1 hora antes"),
    (120, "2 horas antes"),
    (24 * 60, "1 dia antes"),
    (2 * 24 * 60, "2 dias antes"),
    (7 * 24 * 60, "1 semana antes"),
];

/// O próximo alerta pendente de uma pilha, e nada mais.
///
/// É o que vira `reminders.next_due_at` quando a pilha existe: o agendador
/// continua olhando UMA coluna, e a pilha não vaza para dentro dele.
pub fn next_pending_trigger(triggers: &[ReminderTrigger]) -> Option<&ReminderTrigger> {
    triggers
        .iter()
        .filter(|trigger| {
            trigger.status == StackTriggerStatus::Pending
                && trigger.lifecycle_state == LifecycleState::Active
        })
        .min_by_key(|trigger| trigger.scheduled_at)
}

// -------------------------------------------------------------- histórico

attention_id!(ReminderEventId, "Event ID");

/// O que vale registrar na vida de um lembrete.
///
/// Só o que responde a uma pergunta que alguém faz de verdade: "por que isso
/// apareceu?", "quando isso tocou?", "quantas vezes eu adiei?". Registrar cada
/// leitura seria ruído, e ruído num histórico é o mesmo que não ter histórico.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderEventKind {
    Created,
    Triggered,
    Delivered,
    Acknowledged,
    Snoozed,
    Rescheduled,
    Completed,
    Cancelled,
    Missed,
    Escalated,
    RecurrenceGenerated,
    Edited,
}

impl ReminderEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Triggered => "triggered",
            Self::Delivered => "delivered",
            Self::Acknowledged => "acknowledged",
            Self::Snoozed => "snoozed",
            Self::Rescheduled => "rescheduled",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Missed => "missed",
            Self::Escalated => "escalated",
            Self::RecurrenceGenerated => "recurrence_generated",
            Self::Edited => "edited",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "created" => Ok(Self::Created),
            "triggered" => Ok(Self::Triggered),
            "delivered" => Ok(Self::Delivered),
            "acknowledged" => Ok(Self::Acknowledged),
            "snoozed" => Ok(Self::Snoozed),
            "rescheduled" => Ok(Self::Rescheduled),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "missed" => Ok(Self::Missed),
            "escalated" => Ok(Self::Escalated),
            "recurrence_generated" => Ok(Self::RecurrenceGenerated),
            "edited" => Ok(Self::Edited),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Evento de Reminder desconhecido.",
                false,
            )),
        }
    }

    /// Como o evento se lê na folha. Frase curta, sem jargão de máquina de
    /// estados: quem abre o histórico quer saber o que aconteceu, não em que
    /// transição o código estava.
    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "Criado",
            Self::Triggered => "Venceu",
            Self::Delivered => "Notificou",
            Self::Acknowledged => "Você viu",
            Self::Snoozed => "Adiado",
            Self::Rescheduled => "Remarcado",
            Self::Completed => "Concluído",
            Self::Cancelled => "Cancelado",
            Self::Missed => "Perdido",
            Self::Escalated => "Insistiu",
            Self::RecurrenceGenerated => "Próxima ocorrência",
            Self::Edited => "Editado",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderEvent {
    pub id: ReminderEventId,
    pub reminder_id: ReminderId,
    pub kind: ReminderEventKind,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    /// JSON livre. Um adiamento guarda até quando; um escalonamento, o degrau.
    pub detail: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewReminderEvent {
    pub id: ReminderEventId,
    pub reminder_id: ReminderId,
    pub kind: ReminderEventKind,
    pub at: OffsetDateTime,
    pub detail: Option<String>,
}

impl NewReminderEvent {
    pub fn new(reminder: ReminderId, kind: ReminderEventKind, at: OffsetDateTime) -> Self {
        Self {
            id: ReminderEventId::new(),
            reminder_id: reminder,
            kind,
            at,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

// -------------------------------------------------------- needs attention

/// Por que este lembrete está cobrando atenção.
///
/// Enum e não texto livre porque a superfície precisa DIZER o motivo, e um
/// motivo escrito à mão em cada tela vira três frases diferentes para a mesma
/// situação. Também é o que torna a regra auditável: dá para olhar a lista e
/// entender por que cada item está nela.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionReason {
    /// Venceu enquanto o M/OS não estava olhando.
    Missed,
    /// Passou da hora e continua aberto.
    Overdue,
    /// Já notificou mais de uma vez e ninguém respondeu.
    Ignored,
    /// Adiado cinco vezes ou mais.
    SnoozeFatigue,
    /// Marcado como "não me deixa esquecer" e ainda não resolvido.
    Persistent,
    /// Prioridade alta ou urgente, e a hora chegou.
    HighPriority,
    /// Ficou de um dia anterior.
    CarriedOver,
}

impl AttentionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missed => "missed",
            Self::Overdue => "overdue",
            Self::Ignored => "ignored",
            Self::SnoozeFatigue => "snooze_fatigue",
            Self::Persistent => "persistent",
            Self::HighPriority => "high_priority",
            Self::CarriedOver => "carried_over",
        }
    }

    /// A frase que a tela mostra.
    pub fn label(self) -> &'static str {
        match self {
            Self::Missed => "perdido",
            Self::Overdue => "atrasado",
            Self::Ignored => "ignorado",
            Self::SnoozeFatigue => "adiado demais",
            Self::Persistent => "não me deixe esquecer",
            Self::HighPriority => "prioridade alta",
            Self::CarriedOver => "veio de ontem",
        }
    }

    /// Quanto este motivo pesa na ordenação.
    ///
    /// Peso e não posição fixa: um lembrete pode ter três motivos, e a soma é o
    /// que separa "atrasado desde ontem, persistente e ignorado" de "atrasado há
    /// dez minutos". Números pequenos e redondos de propósito — um score que
    /// ninguém consegue reproduzir de cabeça é um score que ninguém confere.
    pub fn weight(self) -> u32 {
        match self {
            Self::Missed => 30,
            Self::Overdue => 20,
            Self::Ignored => 15,
            Self::Persistent => 15,
            Self::CarriedOver => 12,
            Self::SnoozeFatigue => 10,
            Self::HighPriority => 8,
        }
    }
}

/// Um item de Needs Attention, com os motivos que o puseram lá.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionItem {
    pub reminder_id: ReminderId,
    pub reasons: Vec<AttentionReason>,
    pub weight: u32,
}

/// Quem está sendo esquecido, e por quê.
///
/// **Determinística e pura** (§7 do pedido, §30 do documento): nenhuma regra
/// aqui depende de modelo, de histórico de uso ou de qualquer coisa que a pessoa
/// não consiga prever. O Hermes pode SUGERIR prioridade em cima disto; o que
/// entra na lista é decidido aqui, por regras que cabem numa tela.
///
/// `now_local` é o instante COM o deslocamento local, porque "veio de ontem" é
/// uma pergunta sobre o calendário de quem olha, e não sobre UTC.
pub fn needs_attention(reminders: &[Reminder], now_local: OffsetDateTime) -> Vec<AttentionItem> {
    let now = now_local.to_offset(time::UtcOffset::UTC);
    let today = now_local.date();

    let mut found: Vec<AttentionItem> = reminders
        .iter()
        .filter(|reminder| reminder.lifecycle_state == LifecycleState::Active)
        .filter(|reminder| !reminder.status.is_terminal())
        .filter_map(|reminder| {
            let mut reasons = Vec::new();

            if reminder.status == ReminderStatus::Missed {
                reasons.push(AttentionReason::Missed);
            }

            // Vencido é sobre o RELÓGIO ter passado E o lembrete já ter saído da
            // espera. Um `scheduled` com hora no passado é o intervalo entre o
            // vencimento e a próxima acordada do agendador — chamá-lo de
            // atrasado ali seria acusar o sistema do próprio atraso.
            let overdue = reminder.next_due_at.is_some_and(|due| due <= now)
                && !matches!(
                    reminder.status,
                    ReminderStatus::Snoozed | ReminderStatus::Scheduled
                );
            if overdue {
                reasons.push(AttentionReason::Overdue);
            }

            // Só conta como ignorado quando REALMENTE apareceu mais de uma vez e
            // continua sem resposta. Uma entrega só é entrega, não desprezo.
            if reminder.delivered_count >= 2
                && matches!(
                    reminder.status,
                    ReminderStatus::Due | ReminderStatus::Delivered | ReminderStatus::Missed
                )
            {
                reasons.push(AttentionReason::Ignored);
            }

            if reminder.snooze_fatigue() {
                reasons.push(AttentionReason::SnoozeFatigue);
            }

            // Persistente NÃO entra sozinho: um lembrete para daqui a três dias
            // marcado como "não me deixa esquecer" não está sendo esquecido
            // ainda. Ele reforça os outros motivos, e é isso que a soma faz.
            if reminder.persistent && !reasons.is_empty() {
                reasons.push(AttentionReason::Persistent);
            }

            if reminder.priority >= Priority::High && (overdue || reminder.status.needs_attention())
            {
                reasons.push(AttentionReason::HighPriority);
            }

            if reminder
                .next_due_at
                .is_some_and(|due| due.to_offset(now_local.offset()).date() < today)
                && !matches!(
                    reminder.status,
                    ReminderStatus::Scheduled | ReminderStatus::Snoozed
                )
            {
                reasons.push(AttentionReason::CarriedOver);
            }

            if reasons.is_empty() {
                return None;
            }

            reasons.sort_unstable();
            reasons.dedup();
            let weight = reasons.iter().map(|reason| reason.weight()).sum();
            Some(AttentionItem {
                reminder_id: reminder.id,
                reasons,
                weight,
            })
        })
        .collect();

    // Maior peso primeiro; empate desfeito pelo id, para a ordem ser estável
    // entre duas leituras — uma lista que se reordena sozinha é uma lista em que
    // se clica no item errado.
    found.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then_with(|| a.reminder_id.to_string().cmp(&b.reminder_id.to_string()))
    });
    found
}

// -------------------------------------------------------- pedido de criação

/// Tudo que se pode pedir ao criar um lembrete.
///
/// Struct e não oito parâmetros porque três superfícies criam lembretes —
/// desktop, web e Hermes — e uma assinatura posicional de oito campos é uma
/// troca de dois booleanos esperando para acontecer.
///
/// Os defaults são o caso comum inteiro: título, hora, e nada mais. Persistir,
/// repetir, empilhar alertas e cobrar terceiro são campos que quem não precisa
/// nunca preenche — que é o que `UX-PRINCIPLES.md` §8 chama de revelar
/// complexidade sob demanda, escrito no tipo em vez de na tela.
#[derive(Clone, Debug)]
pub struct CreateReminder {
    pub title: String,
    pub body: String,
    /// `None` é "algum dia": o lembrete existe e não interrompe.
    pub at: Option<OffsetDateTime>,
    pub target: Option<ReminderTarget>,
    pub priority: Priority,
    pub source: ReminderSource,
    pub persistent: bool,
    pub recurrence: Option<crate::Recurrence>,
    /// De quem se está esperando. Preenchido, o lembrete vira follow-up.
    pub waiting_for: Option<String>,
    /// Adiantamentos, em minutos, para a pilha de alertas. Vazio é o normal.
    pub leads: Vec<u32>,
}

impl CreateReminder {
    pub fn at(title: &str, instant: OffsetDateTime) -> Self {
        Self {
            title: title.to_owned(),
            body: String::new(),
            at: Some(instant),
            target: None,
            priority: Priority::Normal,
            source: ReminderSource::User,
            persistent: false,
            recurrence: None,
            waiting_for: None,
            leads: Vec::new(),
        }
    }

    pub fn someday(title: &str) -> Self {
        Self {
            at: None,
            ..Self::at(title, OffsetDateTime::UNIX_EPOCH)
        }
    }

    pub fn with_body(mut self, body: &str) -> Self {
        self.body = body.to_owned();
        self
    }

    pub fn with_target(mut self, target: ReminderTarget) -> Self {
        self.target = Some(target);
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn from_source(mut self, source: ReminderSource) -> Self {
        self.source = source;
        self
    }

    /// "Não me deixa esquecer."
    pub fn persisting(mut self) -> Self {
        self.persistent = true;
        self
    }
}

// ------------------------------------------------------------- varredura

/// Por que este lembrete precisa aparecer agora.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DueReason {
    /// Venceu há pouco.
    DueNow,
    /// Venceu enquanto o M/OS não estava olhando.
    MissedWhileAway,
    /// Já tinha aparecido e continua sem resposta: é a insistência.
    Retry,
}

impl DueReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DueNow => "due_now",
            Self::MissedWhileAway => "missed",
            Self::Retry => "retry",
        }
    }
}

/// Um lembrete que o adaptador de plataforma precisa entregar.
///
/// O domínio decide O QUE entregar e POR QUÊ; o adaptador decide COMO — toast do
/// Windows, evento in-app, push no celular. É a fronteira que mantém a regra
/// fora do React e fora do Tauri.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueDelivery {
    pub reminder: Reminder,
    pub reason: DueReason,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FixedClock;

    fn epoch() -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH
    }

    fn at(hours: i64) -> OffsetDateTime {
        epoch() + Duration::hours(hours)
    }

    fn clock_at(hours: i64) -> FixedClock {
        FixedClock::at(at(hours))
    }

    fn reminder(status: ReminderStatus, due: Option<OffsetDateTime>) -> Reminder {
        Reminder {
            id: ReminderId::new(),
            title: "Enviar proposta".into(),
            body: String::new(),
            target: None,
            trigger: Trigger::At {
                instant: due.unwrap_or_else(|| at(10)),
            },
            priority: Priority::Normal,
            status,
            policy: DeliveryPolicy::default(),
            source: ReminderSource::User,
            next_due_at: due,
            snooze_count: 0,
            delivered_count: 0,
            kind: crate::ReminderKind::Standard,
            waiting_for: String::new(),
            persistent: false,
            escalation_step: 0,
            last_triggered_at: None,
            retry_at: None,
            recurrence: None,
            created_at: epoch(),
            updated_at: epoch(),
            completed_at: None,
            lifecycle_state: LifecycleState::Active,
        }
    }

    // ------------------------------------------------------------- criação

    #[test]
    fn a_reminder_needs_a_title() {
        let error = NewReminder::at("   ", "", at(10), &clock_at(1)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn a_reminder_in_the_past_is_refused() {
        let error = NewReminder::at("Ontem", "", at(1), &clock_at(10)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    /// A folga existe para o caso invisível: o relógio ligeiramente à frente
    /// entre a escolha e a chegada do comando.
    #[test]
    fn a_reminder_just_barely_in_the_past_is_accepted() {
        let clock = clock_at(10);
        let instant = clock.now() - Duration::seconds(30);
        assert!(NewReminder::at("Agorinha", "", instant, &clock).is_ok());
    }

    #[test]
    fn a_new_reminder_starts_due_at_its_instant() {
        let created = NewReminder::at("Ligar", "", at(20), &clock_at(10)).unwrap();
        assert_eq!(created.next_due_at, Some(at(20)));
        assert_eq!(created.trigger, Trigger::At { instant: at(20) });
        assert_eq!(created.priority, Priority::Normal);
    }

    #[test]
    fn a_reminder_id_is_a_uuid_v7() {
        let created = NewReminder::at("X", "", at(20), &clock_at(10)).unwrap();
        assert_eq!(created.id.0.get_version_num(), 7);
    }

    // ------------------------------------------------------------- estados

    #[test]
    fn only_waiting_states_can_ring() {
        for status in [ReminderStatus::Scheduled, ReminderStatus::Snoozed] {
            let next = apply(&reminder(status, Some(at(10))), Transition::Ring, at(10)).unwrap();
            assert_eq!(next.status, ReminderStatus::Due);
        }
        for status in [
            ReminderStatus::Due,
            ReminderStatus::Delivered,
            ReminderStatus::Completed,
        ] {
            assert!(apply(&reminder(status, Some(at(10))), Transition::Ring, at(10)).is_err());
        }
    }

    #[test]
    fn delivering_counts_and_can_repeat() {
        let first = apply(
            &reminder(ReminderStatus::Due, Some(at(10))),
            Transition::Deliver,
            at(10),
        )
        .unwrap();
        assert_eq!(first.status, ReminderStatus::Delivered);
        assert_eq!(first.delivered_count, 1);

        let second = apply(&first, Transition::Deliver, at(11)).unwrap();
        assert_eq!(second.delivered_count, 2, "entregar de novo nao zera nada");
    }

    /// É assim que "enquanto você esteve fora" alcança a pessoa.
    #[test]
    fn a_missed_reminder_can_still_be_delivered() {
        let next = apply(
            &reminder(ReminderStatus::Missed, Some(at(3))),
            Transition::Deliver,
            at(10),
        )
        .unwrap();
        assert_eq!(next.status, ReminderStatus::Delivered);
    }

    #[test]
    fn terminal_states_refuse_everything() {
        for status in [
            ReminderStatus::Completed,
            ReminderStatus::Cancelled,
            ReminderStatus::Expired,
        ] {
            let subject = reminder(status, None);
            for transition in [
                Transition::Ring,
                Transition::Deliver,
                Transition::Acknowledge,
                Transition::Snooze { until: at(50) },
                Transition::Complete,
                Transition::Cancel,
                Transition::Miss,
                Transition::Expire,
            ] {
                let result = apply(&subject, transition, at(20));
                assert!(
                    result.is_err(),
                    "{} deveria recusar {}",
                    status.as_str(),
                    transition.name()
                );
            }
        }
    }

    /// O instante original é a única coisa que distingue "perdido há 10 min" de
    /// "perdido há três dias". Zerá-lo ao marcar como perdido apagaria o tamanho
    /// do atraso sem quebrar nada visível — foi um teste de mutação que
    /// mostrou que esta garantia não estava coberta.
    #[test]
    fn missing_preserves_the_original_due_instant() {
        let next = apply(
            &reminder(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Miss,
            at(13),
        )
        .unwrap();

        assert_eq!(next.status, ReminderStatus::Missed);
        assert_eq!(next.next_due_at, Some(at(10)), "o vencimento original fica");
        assert_eq!(next.overdue_by(at(13)), Some(Duration::hours(3)));
    }

    #[test]
    fn a_refused_transition_says_which_and_why() {
        let error = apply(
            &reminder(ReminderStatus::Completed, None),
            Transition::Snooze { until: at(50) },
            at(20),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidTransition);
        assert!(error.message.contains("adiar"), "{}", error.message);
        assert!(error.message.contains("completed"), "{}", error.message);
    }

    // -------------------------------------------------------------- snooze

    #[test]
    fn snoozing_moves_the_due_date_and_counts() {
        let next = apply(
            &reminder(ReminderStatus::Due, Some(at(10))),
            Transition::Snooze { until: at(12) },
            at(10),
        )
        .unwrap();
        assert_eq!(next.status, ReminderStatus::Snoozed);
        assert_eq!(next.next_due_at, Some(at(12)));
        assert_eq!(next.snooze_count, 1);
    }

    #[test]
    fn snoozing_into_the_past_is_refused() {
        let error = apply(
            &reminder(ReminderStatus::Due, Some(at(10))),
            Transition::Snooze { until: at(9) },
            at(10),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn a_policy_can_forbid_snooze() {
        let mut subject = reminder(ReminderStatus::Due, Some(at(10)));
        subject.policy.snooze_allowed = false;
        let error = apply(&subject, Transition::Snooze { until: at(12) }, at(10)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidTransition);
    }

    #[test]
    fn something_not_yet_due_can_still_be_pushed() {
        let next = apply(
            &reminder(ReminderStatus::Scheduled, Some(at(30))),
            Transition::Snooze { until: at(40) },
            at(10),
        )
        .unwrap();
        assert_eq!(next.next_due_at, Some(at(40)));
    }

    #[test]
    fn the_fifth_snooze_flags_fatigue() {
        let mut subject = reminder(ReminderStatus::Due, Some(at(10)));
        assert!(!subject.snooze_fatigue());
        subject.snooze_count = 5;
        assert!(subject.snooze_fatigue());
    }

    // -------------------------------------------------- conclusão e badge

    #[test]
    fn completing_stamps_and_clears_the_schedule() {
        let next = apply(
            &reminder(ReminderStatus::Delivered, Some(at(10))),
            Transition::Complete,
            at(11),
        )
        .unwrap();
        assert_eq!(next.status, ReminderStatus::Completed);
        assert_eq!(next.completed_at, Some(at(11)));
        assert_eq!(next.next_due_at, None, "concluido nao acorda mais ninguem");
    }

    #[test]
    fn only_what_waits_for_a_person_counts_for_the_badge() {
        use ReminderStatus::*;
        for status in [Due, Delivered, Missed] {
            assert!(status.needs_attention(), "{}", status.as_str());
        }
        for status in [
            Scheduled,
            Snoozed,
            Acknowledged,
            Completed,
            Cancelled,
            Expired,
        ] {
            assert!(!status.needs_attention(), "{}", status.as_str());
        }
    }

    // ------------------------------------------------------- reconciliação

    #[test]
    fn a_reminder_overdue_by_minutes_is_just_due() {
        let subject = [reminder(ReminderStatus::Scheduled, Some(at(10)))];
        let found = reconcile(&subject, at(10) + Duration::minutes(2));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].reason, ReconcileReason::DueNow);
        assert_eq!(found[0].was_due_at, at(10));
    }

    /// O caso do PC que dormiu: precisa dizer que perdeu, e há quanto tempo.
    #[test]
    fn a_reminder_overdue_by_an_hour_was_missed() {
        let subject = [reminder(ReminderStatus::Scheduled, Some(at(10)))];
        let found = reconcile(&subject, at(11));
        assert_eq!(found[0].reason, ReconcileReason::MissedWhileAway);
        assert_eq!(
            found[0].was_due_at,
            at(10),
            "o instante original sobrevive, senao nao ha 'perdido ha 1h'"
        );
    }

    #[test]
    fn reconcile_ignores_the_future_and_the_finished() {
        let subject = [
            reminder(ReminderStatus::Scheduled, Some(at(50))),
            reminder(ReminderStatus::Completed, None),
            reminder(ReminderStatus::Cancelled, None),
            reminder(ReminderStatus::Delivered, Some(at(1))),
        ];
        assert!(reconcile(&subject, at(10)).is_empty());
    }

    #[test]
    fn reconcile_ignores_archived_and_trashed() {
        for state in [LifecycleState::Archived, LifecycleState::Trashed] {
            let mut subject = reminder(ReminderStatus::Scheduled, Some(at(1)));
            subject.lifecycle_state = state;
            assert!(reconcile(&[subject], at(10)).is_empty());
        }
    }

    /// Chamada em todo tick do agendador. Se não fosse idempotente, um tick
    /// duplicado entregaria duas vezes.
    #[test]
    fn reconcile_is_idempotent() {
        let subject = [reminder(ReminderStatus::Scheduled, Some(at(10)))];
        let first = reconcile(&subject, at(11));
        let second = reconcile(&subject, at(11));
        assert_eq!(first, second);
    }

    #[test]
    fn a_snoozed_reminder_whose_time_came_is_reconciled_too() {
        let subject = [reminder(ReminderStatus::Snoozed, Some(at(10)))];
        assert_eq!(reconcile(&subject, at(11)).len(), 1);
    }

    // ----------------------------------------------------------- agendador

    #[test]
    fn the_next_wake_is_the_earliest_thing_waiting() {
        let subject = [
            reminder(ReminderStatus::Scheduled, Some(at(30))),
            reminder(ReminderStatus::Snoozed, Some(at(12))),
            reminder(ReminderStatus::Scheduled, Some(at(20))),
        ];
        assert_eq!(next_wake(&subject), Some(at(12)));
    }

    #[test]
    fn nothing_waiting_means_sleeping_without_a_deadline() {
        let subject = [
            reminder(ReminderStatus::Completed, None),
            reminder(ReminderStatus::Delivered, Some(at(1))),
        ];
        assert_eq!(next_wake(&subject), None);
    }

    #[test]
    fn the_next_wake_ignores_archived() {
        let mut archived = reminder(ReminderStatus::Scheduled, Some(at(2)));
        archived.lifecycle_state = LifecycleState::Archived;
        let subject = [archived, reminder(ReminderStatus::Scheduled, Some(at(9)))];
        assert_eq!(next_wake(&subject), Some(at(9)));
    }

    // ---------------------------------------------------------- atraso e ui

    #[test]
    fn overdue_by_reports_the_gap_only_when_late() {
        let subject = reminder(ReminderStatus::Due, Some(at(10)));
        assert_eq!(subject.overdue_by(at(12)), Some(Duration::hours(2)));
        assert_eq!(subject.overdue_by(at(9)), None);
        assert_eq!(
            subject.overdue_by(at(10)),
            None,
            "no instante nao ha atraso"
        );
    }

    // ------------------------------------------------------------ privacidade

    #[test]
    fn privacy_decides_what_leaves_in_the_payload() {
        let (title, body) = ContentPrivacy::ShowContent.redact("Pagar boleto", "R$ 1.234,56");
        assert_eq!(title, "Pagar boleto");
        assert_eq!(body, "R$ 1.234,56");

        let (title, body) = ContentPrivacy::TitleOnly.redact("Pagar boleto", "R$ 1.234,56");
        assert_eq!(title, "Pagar boleto");
        assert!(body.is_empty(), "o valor nao pode sair");

        let (title, body) = ContentPrivacy::Hidden.redact("Pagar boleto", "R$ 1.234,56");
        assert_eq!(title, "M/OS");
        assert!(!body.contains("boleto"), "nem o titulo pode sair");
    }

    // ------------------------------------------------------- nomes de wire

    /// Os nomes atravessam a ponte para o TypeScript e para o SQLite. Um rename
    /// silencioso aqui faria a tela deixar de reconhecer o estado e o banco
    /// deixar de reler o que gravou, sem erro de compilação de lado nenhum. É o
    /// mesmo teste que `calendar.rs` já tem pelo mesmo motivo.
    #[test]
    fn every_status_round_trips_through_its_wire_name() {
        use ReminderStatus::*;
        for status in [
            Scheduled,
            Due,
            Delivered,
            Acknowledged,
            Snoozed,
            Completed,
            Cancelled,
            Missed,
            Expired,
        ] {
            assert_eq!(ReminderStatus::parse(status.as_str()).unwrap(), status);
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", status.as_str()));
        }
    }

    #[test]
    fn every_priority_and_source_round_trips() {
        for priority in [
            Priority::Low,
            Priority::Normal,
            Priority::High,
            Priority::Urgent,
        ] {
            assert_eq!(Priority::parse(priority.as_str()).unwrap(), priority);
        }
        for source in [
            ReminderSource::User,
            ReminderSource::Hermes,
            ReminderSource::Capture,
            ReminderSource::System,
        ] {
            assert_eq!(ReminderSource::parse(source.as_str()).unwrap(), source);
        }
    }

    #[test]
    fn an_unknown_wire_name_is_a_data_integrity_error() {
        assert_eq!(
            ReminderStatus::parse("adormecido").unwrap_err().code,
            ErrorCode::DataIntegrity
        );
    }

    #[test]
    fn every_target_round_trips_through_its_columns() {
        let targets = [
            ReminderTarget::Task(TaskId::new()),
            ReminderTarget::Project(ProjectId::new()),
            ReminderTarget::Capture(CaptureId::new()),
            ReminderTarget::Resource(ResourceId::new()),
            ReminderTarget::Conversation(ConversationId::new()),
            ReminderTarget::App(AppId::new()),
        ];
        for target in targets {
            let (kind, id) = target.as_columns();
            assert_eq!(ReminderTarget::from_columns(kind, &id).unwrap(), target);
        }
    }

    #[test]
    fn an_unknown_target_kind_is_refused() {
        let id = TaskId::new().to_string();
        assert!(ReminderTarget::from_columns("planeta", &id).is_err());
    }

    // ----------------------------------------------- o trigger no passado

    /// O detalhe que sustenta a promessa: um instante que já passou continua
    /// sendo o vencimento. Se `next_due` devolvesse `None` para o passado, todo
    /// Reminder atrasado sairia do radar em silêncio.
    #[test]
    fn a_trigger_in_the_past_still_reports_its_instant() {
        let trigger = Trigger::At { instant: at(5) };
        assert_eq!(trigger.next_due(at(50)), Some(at(5)));
    }

    // -------------------------------------------------------------- edição

    #[test]
    fn editar_troca_titulo_e_corpo() {
        let antes = reminder(ReminderStatus::Scheduled, Some(at(10)));
        let depois = edit(
            &antes,
            EditReminder {
                title: Some("  Enviar a proposta revisada  ".into()),
                body: Some("  com o anexo  ".into()),
                ..Default::default()
            },
            at(5),
        )
        .unwrap();
        assert_eq!(depois.title, "Enviar a proposta revisada");
        assert_eq!(depois.body, "com o anexo");
        // O que não foi pedido não muda.
        assert_eq!(depois.next_due_at, antes.next_due_at);
    }

    /// `None` é "não mexi", e tem que ser diferente de "mexi para vazio". Sem
    /// isso, a tela que edita só a hora apagaria o corpo do lembrete.
    #[test]
    fn o_que_nao_foi_pedido_fica_intacto() {
        let antes = reminder(ReminderStatus::Scheduled, Some(at(10)));
        let depois = edit(&antes, EditReminder::default(), at(5)).unwrap();
        assert_eq!(depois.title, antes.title);
        assert_eq!(depois.body, antes.body);
        assert_eq!(depois.priority, antes.priority);
        assert_eq!(depois.next_due_at, antes.next_due_at);
    }

    #[test]
    fn editar_para_titulo_vazio_e_recusado() {
        let antes = reminder(ReminderStatus::Scheduled, Some(at(10)));
        let erro = edit(
            &antes,
            EditReminder {
                title: Some("   ".into()),
                ..Default::default()
            },
            at(5),
        )
        .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn reagendar_move_o_vencimento_e_o_gatilho() {
        let antes = reminder(ReminderStatus::Scheduled, Some(at(10)));
        let depois = edit(
            &antes,
            EditReminder {
                instant: Some(at(20)),
                ..Default::default()
            },
            at(5),
        )
        .unwrap();
        assert_eq!(depois.next_due_at, Some(at(20)));
        assert_eq!(depois.trigger, Trigger::At { instant: at(20) });
    }

    /// A diferença entre reagendar e adiar.
    ///
    /// Um lembrete que já tocou e foi remarcado para amanhã não pode continuar
    /// vencido: ele cobraria atenção na tela por uma hora que já não é a dele.
    #[test]
    fn reagendar_para_o_futuro_tira_o_lembrete_de_vencido() {
        let vencido = reminder(ReminderStatus::Due, Some(at(4)));
        let depois = edit(
            &vencido,
            EditReminder {
                instant: Some(at(30)),
                ..Default::default()
            },
            at(5),
        )
        .unwrap();
        assert_eq!(depois.status, ReminderStatus::Scheduled);
    }

    /// Reagendar não é adiar: zerar a contagem aqui apagaria o sinal de fadiga
    /// que o sistema usa para oferecer ajuda depois do quinto adiamento.
    #[test]
    fn reagendar_nao_mexe_na_contagem_de_adiamentos() {
        let mut cansado = reminder(ReminderStatus::Due, Some(at(4)));
        cansado.snooze_count = 6;
        let depois = edit(
            &cansado,
            EditReminder {
                instant: Some(at(30)),
                ..Default::default()
            },
            at(5),
        )
        .unwrap();
        assert_eq!(depois.snooze_count, 6);
        assert!(depois.snooze_fatigue());
    }

    #[test]
    fn editar_para_o_passado_e_recusado() {
        let antes = reminder(ReminderStatus::Scheduled, Some(at(10)));
        let erro = edit(
            &antes,
            EditReminder {
                instant: Some(at(1)),
                ..Default::default()
            },
            at(10),
        )
        .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidInput);
    }

    /// Concluído e cancelado são respostas já dadas. Mudar o título de algo que
    /// você resolveu não é edição — é ressuscitar pela porta dos fundos.
    #[test]
    fn lembrete_resolvido_nao_se_edita() {
        for terminal in [
            ReminderStatus::Completed,
            ReminderStatus::Cancelled,
            ReminderStatus::Expired,
        ] {
            let erro = edit(
                &reminder(terminal, Some(at(10))),
                EditReminder {
                    title: Some("Outra coisa".into()),
                    ..Default::default()
                },
                at(20),
            )
            .unwrap_err();
            assert_eq!(erro.code, ErrorCode::InvalidTransition, "{terminal:?}");
        }
    }

    #[test]
    fn editar_carimba_a_hora_da_mudanca() {
        let antes = reminder(ReminderStatus::Scheduled, Some(at(10)));
        let depois = edit(
            &antes,
            EditReminder {
                title: Some("Novo".into()),
                ..Default::default()
            },
            at(7),
        )
        .unwrap();
        assert_eq!(depois.updated_at, at(7));
    }

    // ------------------------------------------- persistir e escalonar

    fn persistente(status: ReminderStatus, due: Option<OffsetDateTime>) -> Reminder {
        Reminder {
            persistent: true,
            ..reminder(status, due)
        }
    }

    /// O coração do pedido: uma entrega NÃO resolve a intenção.
    ///
    /// Entregar um lembrete persistente o deixa `Delivered`, e `Delivered`
    /// continua cobrando atenção. Se este teste passar a falhar, o sistema
    /// voltou a ser um despertador.
    #[test]
    fn delivering_never_resolves_a_reminder() {
        let vencido = apply(
            &reminder(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        let entregue = apply(&vencido, Transition::Deliver, at(10)).unwrap();
        assert_eq!(entregue.status, ReminderStatus::Delivered);
        assert!(entregue.status.needs_attention());
        assert!(entregue.completed_at.is_none());
    }

    /// Vencer arma o primeiro re-alerta — mas só para quem tem direito a um.
    #[test]
    fn ringing_arms_the_retry_only_for_who_earns_it() {
        let comum = apply(
            &reminder(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        assert!(comum.retry_at.is_none(), "lembrete normal nao insiste");

        let insistente = apply(
            &persistente(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        assert_eq!(insistente.retry_at, Some(at(10) + Duration::minutes(30)));
    }

    /// Importante ganha UMA segunda chance. Não três: só o persistente insiste.
    #[test]
    fn an_important_reminder_gets_exactly_one_retry() {
        let alto = Reminder {
            priority: Priority::High,
            ..reminder(ReminderStatus::Scheduled, Some(at(10)))
        };
        assert_eq!(retry_budget(&alto), 1);

        let vencido = apply(&alto, Transition::Ring, at(10)).unwrap();
        assert!(vencido.retry_at.is_some());

        let insistiu = apply(
            &vencido,
            Transition::Escalate,
            at(10) + Duration::minutes(30),
        )
        .unwrap();
        assert_eq!(insistiu.escalation_step, 1);
        assert!(
            insistiu.retry_at.is_none(),
            "acabou o orcamento: para de tocar, e continua existindo"
        );
        assert!(insistiu.status.needs_attention());
    }

    /// Os intervalos crescem, e acabam. Espinha do §8 do pedido.
    #[test]
    fn escalation_backs_off_and_then_stops() {
        assert_eq!(escalation_delay(0), Some(Duration::minutes(30)));
        assert_eq!(escalation_delay(1), Some(Duration::hours(1)));
        assert_eq!(escalation_delay(2), Some(Duration::hours(2)));
        assert_eq!(escalation_delay(3), None);

        let mut atual = apply(
            &persistente(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        for degrau in 1..=3 {
            let momento = atual.retry_at.expect("ainda deveria insistir");
            atual = apply(&atual, Transition::Escalate, momento).unwrap();
            assert_eq!(atual.escalation_step, degrau);
        }
        assert!(atual.retry_at.is_none(), "tres degraus e chega");
        // O que importa depois disso: continua vivo e continua cobrando.
        assert!(!atual.status.is_terminal());
        assert!(atual.status.needs_attention());
    }

    /// Insistir não muda o estado nem apaga o tamanho do atraso.
    #[test]
    fn escalating_keeps_the_original_due_instant() {
        let vencido = apply(
            &persistente(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        let insistiu = apply(&vencido, Transition::Escalate, at(12)).unwrap();
        assert_eq!(
            insistiu.next_due_at,
            Some(at(10)),
            "o atraso continua sendo desde as 10"
        );
        assert_eq!(insistiu.status, ReminderStatus::Due);
        assert_eq!(insistiu.overdue_by(at(12)), Some(Duration::hours(2)));
    }

    /// Qualquer decisão da pessoa cancela a insistência.
    #[test]
    fn a_human_decision_stops_the_nagging() {
        let vencido = apply(
            &persistente(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        assert!(vencido.retry_at.is_some());

        let adiado = apply(&vencido, Transition::Snooze { until: at(20) }, at(11)).unwrap();
        assert!(adiado.retry_at.is_none());
        assert_eq!(adiado.escalation_step, 0);

        let visto = apply(&vencido, Transition::Acknowledge, at(11)).unwrap();
        assert!(visto.retry_at.is_none());
        // Ver não é resolver: continua não terminal.
        assert!(!visto.status.is_terminal());

        let feito = apply(&vencido, Transition::Complete, at(11)).unwrap();
        assert!(feito.retry_at.is_none());
        assert_eq!(feito.status, ReminderStatus::Completed);
    }

    /// Segurar a entrega não é adiar: não conta fadiga e não mexe no vencimento.
    #[test]
    fn deferring_is_not_snoozing() {
        let vencido = apply(
            &reminder(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        let segurado = apply(&vencido, Transition::Defer { until: at(18) }, at(10)).unwrap();
        assert_eq!(segurado.snooze_count, 0);
        assert_eq!(segurado.next_due_at, Some(at(10)));
        assert_eq!(segurado.retry_at, Some(at(18)));
        assert_eq!(segurado.status, ReminderStatus::Due);
    }

    /// O agendador precisa acordar para o re-alerta também.
    #[test]
    fn the_scheduler_wakes_up_for_a_retry() {
        let esperando = reminder(ReminderStatus::Scheduled, Some(at(30)));
        let insistindo = Reminder {
            status: ReminderStatus::Delivered,
            retry_at: Some(at(11)),
            ..reminder(ReminderStatus::Delivered, Some(at(10)))
        };
        assert_eq!(next_wake(&[esperando, insistindo]), Some(at(11)));
    }

    #[test]
    fn pending_retries_only_finds_what_is_due() {
        let cedo = Reminder {
            retry_at: Some(at(20)),
            ..reminder(ReminderStatus::Delivered, Some(at(10)))
        };
        let agora = Reminder {
            retry_at: Some(at(11)),
            ..reminder(ReminderStatus::Delivered, Some(at(10)))
        };
        let resolvido = Reminder {
            retry_at: Some(at(11)),
            status: ReminderStatus::Completed,
            completed_at: Some(at(11)),
            ..reminder(ReminderStatus::Completed, None)
        };
        let achados = pending_retries(&[cedo, agora.clone(), resolvido], at(12));
        assert_eq!(achados, vec![agora.id]);
    }

    /// Os dois lados chegam ao mesmo degrau: o que escreve e o que só lê.
    ///
    /// Sem isto, o PC insistiria três vezes e o celular uma — ou o contrário —,
    /// e a mesma pessoa receberia números diferentes de cobranças do mesmo
    /// sistema em dois aparelhos.
    #[test]
    fn the_alert_slot_matches_what_the_scheduler_would_have_reached() {
        let insistente = persistente(ReminderStatus::Delivered, Some(at(10)));

        // Antes do primeiro intervalo, nenhum re-alerta.
        assert_eq!(alert_slot(&insistente, at(10) + Duration::minutes(29)), 0);
        // Trinta minutos: o primeiro.
        assert_eq!(alert_slot(&insistente, at(10) + Duration::minutes(30)), 1);
        // Mais uma hora: o segundo.
        assert_eq!(alert_slot(&insistente, at(10) + Duration::minutes(91)), 2);
        // Mais duas horas: o terceiro, e o teto.
        assert_eq!(alert_slot(&insistente, at(10) + Duration::hours(4)), 3);
        assert_eq!(
            alert_slot(&insistente, at(10) + Duration::days(30)),
            3,
            "o orcamento nao cresce com o atraso"
        );

        // E o degrau que o agendador de verdade alcanca e o mesmo.
        let mut atual = apply(
            &persistente(ReminderStatus::Scheduled, Some(at(10))),
            Transition::Ring,
            at(10),
        )
        .unwrap();
        while let Some(quando) = atual.retry_at {
            atual = apply(&atual, Transition::Escalate, quando).unwrap();
        }
        assert_eq!(
            atual.escalation_step,
            alert_slot(&insistente, at(10) + Duration::days(1))
        );
    }

    #[test]
    fn a_normal_reminder_never_earns_a_second_alert() {
        let comum = reminder(ReminderStatus::Delivered, Some(at(10)));
        assert_eq!(alert_slot(&comum, at(10) + Duration::days(3)), 0);
    }

    #[test]
    fn nothing_that_has_not_expired_earns_an_alert() {
        let futuro = persistente(ReminderStatus::Scheduled, Some(at(100)));
        assert_eq!(alert_slot(&futuro, at(10)), 0);
    }

    // ------------------------------------------------------- recorrência

    fn diaria() -> crate::Recurrence {
        crate::Recurrence {
            rule: crate::RecurrenceRule::Daily,
            anchor: crate::RecurrenceAnchor::Fixed,
            hour: 8,
            minute: 0,
            offset_minutes: 0,
        }
    }

    /// Concluir uma ocorrência não encerra a série: ela avança.
    #[test]
    fn completing_a_recurring_reminder_schedules_the_next_one() {
        let semanal = Reminder {
            recurrence: Some(diaria()),
            ..reminder(ReminderStatus::Due, Some(at(8)))
        };
        let depois = apply(&semanal, Transition::Complete, at(9)).unwrap();
        assert_eq!(depois.status, ReminderStatus::Scheduled);
        assert!(depois.completed_at.is_none());
        assert_eq!(depois.next_due_at, Some(at(32)), "amanha as 08:00");
        assert_eq!(depois.trigger, Trigger::At { instant: at(32) });
    }

    /// Repetição por conclusão conta a partir de QUANDO se concluiu.
    #[test]
    fn a_completion_anchored_series_counts_from_the_completion() {
        let manutencao = Reminder {
            recurrence: Some(crate::Recurrence {
                rule: crate::RecurrenceRule::EveryDays { days: 30 },
                anchor: crate::RecurrenceAnchor::Completion,
                ..diaria()
            }),
            ..reminder(ReminderStatus::Due, Some(at(8)))
        };
        // Concluído com dois dias de atraso: a próxima é 30 dias DEPOIS DISSO.
        let concluido_em = at(8) + Duration::days(2);
        let depois = apply(&manutencao, Transition::Complete, concluido_em).unwrap();
        let proxima = depois.next_due_at.unwrap();
        assert!(proxima > concluido_em + Duration::days(29));
        assert!(proxima < concluido_em + Duration::days(31));
    }

    /// Uma série fixa muito atrasada não pode nascer vencida: ela avança até
    /// passar do relógio, senão venceria duas vezes na mesma acordada.
    #[test]
    fn a_late_fixed_series_jumps_forward_past_now() {
        let atrasada = Reminder {
            recurrence: Some(diaria()),
            ..reminder(ReminderStatus::Missed, Some(at(8)))
        };
        let agora = at(8) + Duration::days(10);
        let depois = apply(&atrasada, Transition::Complete, agora).unwrap();
        assert!(depois.next_due_at.unwrap() > agora);
    }

    /// Sem recorrência, concluir continua sendo terminal. Nada mudou para o
    /// caso comum.
    #[test]
    fn completing_a_one_time_reminder_is_still_terminal() {
        let feito = apply(
            &reminder(ReminderStatus::Due, Some(at(10))),
            Transition::Complete,
            at(11),
        )
        .unwrap();
        assert_eq!(feito.status, ReminderStatus::Completed);
        assert_eq!(feito.completed_at, Some(at(11)));
        assert!(feito.status.is_terminal());
    }

    // ---------------------------------------------------------- someday

    /// Um lembrete sem data existe, e não gera despertar nenhum.
    #[test]
    fn a_someday_reminder_never_wakes_the_scheduler() {
        let algum_dia = Reminder {
            trigger: Trigger::Someday,
            ..reminder(ReminderStatus::Scheduled, None)
        };
        assert_eq!(next_wake(std::slice::from_ref(&algum_dia)), None);
        assert!(reconcile(&[algum_dia], at(100)).is_empty());
    }

    #[test]
    fn a_someday_reminder_still_needs_a_title() {
        assert!(NewReminder::someday("  ", "", &clock_at(1)).is_err());
        let criado = NewReminder::someday("Comprar cabo HDMI", "", &clock_at(1)).unwrap();
        assert_eq!(criado.trigger, Trigger::Someday);
        assert!(criado.next_due_at.is_none());
    }

    // -------------------------------------------------- horas de silêncio

    #[test]
    fn quiet_hours_can_cross_midnight() {
        let noite = QuietHours {
            enabled: true,
            start_minute: 23 * 60,
            end_minute: 8 * 60,
            allow_urgent: false,
        };
        assert!(noite.contains(23 * 60 + 30), "23:30 e silencio");
        assert!(noite.contains(3 * 60), "03:00 e silencio");
        assert!(!noite.contains(12 * 60), "meio-dia nao e");
        assert!(!noite.contains(8 * 60), "08:00 e o fim, ja pode");
    }

    /// A entrega espera; o lembrete continua vencido. É a separação inteira
    /// entre Reminder e Notification.
    #[test]
    fn a_quiet_delivery_is_pushed_to_the_end_of_the_window() {
        let noite = QuietHours {
            enabled: true,
            start_minute: 0,
            end_minute: 8 * 60,
            allow_urgent: false,
        };
        // 03:00 UTC do primeiro dia.
        let madrugada = epoch() + Duration::hours(3);
        let quando = noite
            .defer(madrugada, time::UtcOffset::UTC, Priority::Normal)
            .expect("deveria segurar");
        assert_eq!(quando, epoch() + Duration::hours(8));
    }

    #[test]
    fn nothing_is_deferred_outside_the_window() {
        let noite = QuietHours::default();
        let tarde = epoch() + Duration::hours(15);
        assert!(noite
            .defer(tarde, time::UtcOffset::UTC, Priority::Normal)
            .is_none());
    }

    /// Urgente só fura o silêncio se a pessoa tiver ligado isso.
    #[test]
    fn urgent_pierces_quiet_hours_only_when_allowed() {
        let madrugada = epoch() + Duration::hours(3);
        let fechado = QuietHours::default();
        assert!(fechado
            .defer(madrugada, time::UtcOffset::UTC, Priority::Urgent)
            .is_some());

        let aberto = QuietHours {
            allow_urgent: true,
            ..QuietHours::default()
        };
        assert!(aberto
            .defer(madrugada, time::UtcOffset::UTC, Priority::Urgent)
            .is_none());
        assert!(
            aberto
                .defer(madrugada, time::UtcOffset::UTC, Priority::High)
                .is_some(),
            "so urgente fura, e nao 'alta'"
        );
    }

    // ------------------------------------------------------ pilha de alertas

    #[test]
    fn a_stack_belongs_to_one_reminder_and_fires_in_clock_order() {
        let dono = ReminderId::new();
        let prazo = at(100);
        let gatilho = |quando: OffsetDateTime| ReminderTrigger {
            id: ReminderTriggerId::new(),
            reminder_id: dono,
            scheduled_at: quando,
            kind: StackTriggerKind::Lead,
            lead_minutes: Some(60),
            status: StackTriggerStatus::Pending,
            fired_at: None,
            lifecycle_state: LifecycleState::Active,
            created_at: epoch(),
            updated_at: epoch(),
        };

        let pilha = vec![
            gatilho(prazo - Duration::hours(1)),
            gatilho(prazo - Duration::days(1)),
        ];
        let proximo = next_pending_trigger(&pilha).unwrap();
        assert_eq!(proximo.scheduled_at, prazo - Duration::days(1));
        assert!(pilha.iter().all(|item| item.reminder_id == dono));
    }

    #[test]
    fn a_fired_trigger_is_no_longer_the_next_one() {
        let dono = ReminderId::new();
        let disparado = ReminderTrigger {
            id: ReminderTriggerId::new(),
            reminder_id: dono,
            scheduled_at: at(10),
            kind: StackTriggerKind::AtDue,
            lead_minutes: None,
            status: StackTriggerStatus::Fired,
            fired_at: Some(at(10)),
            lifecycle_state: LifecycleState::Active,
            created_at: epoch(),
            updated_at: epoch(),
        };
        assert!(next_pending_trigger(&[disparado]).is_none());
    }

    /// Um adiantamento que cairia no passado não vira alerta: seria ruído
    /// imediato, e não aviso.
    #[test]
    fn a_lead_in_the_past_is_not_created() {
        let dono = ReminderId::new();
        let prazo = at(10);
        assert!(NewReminderTrigger::lead(dono, prazo, 24 * 60, at(9)).is_none());
        assert!(NewReminderTrigger::lead(dono, prazo, 30, at(9)).is_some());
    }

    #[test]
    fn a_trigger_reads_in_words() {
        let dono = ReminderId::new();
        let prazo = at(100);
        let um_dia = NewReminderTrigger::lead(dono, prazo, 24 * 60, at(1)).unwrap();
        let materializado = ReminderTrigger {
            id: um_dia.id,
            reminder_id: dono,
            scheduled_at: um_dia.scheduled_at,
            kind: um_dia.kind,
            lead_minutes: um_dia.lead_minutes,
            status: StackTriggerStatus::Pending,
            fired_at: None,
            lifecycle_state: LifecycleState::Active,
            created_at: epoch(),
            updated_at: epoch(),
        };
        assert_eq!(materializado.describe(), "1 dia antes");
    }

    // ------------------------------------------------------ needs attention

    fn agora_local() -> OffsetDateTime {
        at(100).to_offset(time::UtcOffset::UTC)
    }

    /// Um lembrete perdido aparece em Needs Attention, com o motivo dito.
    #[test]
    fn a_missed_reminder_needs_attention_and_says_why() {
        let perdido = reminder(ReminderStatus::Missed, Some(at(50)));
        let lista = needs_attention(&[perdido], agora_local());
        assert_eq!(lista.len(), 1);
        assert!(lista[0].reasons.contains(&AttentionReason::Missed));
        assert!(lista[0].reasons.contains(&AttentionReason::Overdue));
    }

    /// O que ainda não é hora NÃO entra. Um Needs Attention que enche de futuro
    /// é uma lista que se aprende a não abrir.
    #[test]
    fn the_future_does_not_need_attention() {
        let futuro = reminder(ReminderStatus::Scheduled, Some(at(200)));
        assert!(needs_attention(&[futuro], agora_local()).is_empty());
    }

    #[test]
    fn a_snoozed_reminder_is_not_overdue() {
        let adiado = Reminder {
            snooze_count: 1,
            ..reminder(ReminderStatus::Snoozed, Some(at(200)))
        };
        assert!(needs_attention(&[adiado], agora_local()).is_empty());
    }

    /// Adiar demais é um sinal, e o sistema o mostra em vez de continuar
    /// oferecendo só "adiar".
    #[test]
    fn snooze_fatigue_shows_up_as_a_reason() {
        let cansado = Reminder {
            snooze_count: 6,
            ..reminder(ReminderStatus::Snoozed, Some(at(200)))
        };
        let lista = needs_attention(&[cansado], agora_local());
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].reasons, vec![AttentionReason::SnoozeFatigue]);
    }

    /// Duas entregas sem resposta é ignorado. Uma só não é.
    #[test]
    fn ignored_needs_two_deliveries() {
        let uma = Reminder {
            delivered_count: 1,
            ..reminder(ReminderStatus::Delivered, Some(at(50)))
        };
        let duas = Reminder {
            delivered_count: 2,
            ..reminder(ReminderStatus::Delivered, Some(at(50)))
        };
        assert!(!needs_attention(&[uma], agora_local())[0]
            .reasons
            .contains(&AttentionReason::Ignored));
        assert!(needs_attention(&[duas], agora_local())[0]
            .reasons
            .contains(&AttentionReason::Ignored));
    }

    /// Mais motivos pesa mais, e a lista mostra o pior primeiro.
    #[test]
    fn the_heaviest_reason_comes_first() {
        let leve = Reminder {
            snooze_count: 6,
            ..reminder(ReminderStatus::Snoozed, Some(at(200)))
        };
        let pesado = Reminder {
            delivered_count: 3,
            persistent: true,
            priority: Priority::High,
            ..reminder(ReminderStatus::Missed, Some(at(20)))
        };
        let lista = needs_attention(&[leve.clone(), pesado.clone()], agora_local());
        assert_eq!(lista[0].reminder_id, pesado.id);
        assert!(lista[0].weight > lista[1].weight);
    }

    /// Persistente não entra sozinho: um lembrete para daqui a três dias marcado
    /// como "não me deixa esquecer" ainda não está sendo esquecido.
    #[test]
    fn persistent_alone_does_not_need_attention() {
        let futuro = Reminder {
            persistent: true,
            ..reminder(ReminderStatus::Scheduled, Some(at(200)))
        };
        assert!(needs_attention(&[futuro], agora_local()).is_empty());
    }

    #[test]
    fn a_terminal_reminder_never_needs_attention() {
        let feito = Reminder {
            completed_at: Some(at(50)),
            ..reminder(ReminderStatus::Completed, None)
        };
        let cancelado = reminder(ReminderStatus::Cancelled, None);
        assert!(needs_attention(&[feito, cancelado], agora_local()).is_empty());
    }

    /// A ordem precisa ser estável entre duas leituras: uma lista que se
    /// reordena sozinha é uma lista em que se clica no item errado.
    #[test]
    fn the_order_is_stable_across_reads() {
        let itens: Vec<Reminder> = (0..5)
            .map(|_| reminder(ReminderStatus::Missed, Some(at(50))))
            .collect();
        let primeira = needs_attention(&itens, agora_local());
        let segunda = needs_attention(&itens, agora_local());
        let ids = |lista: &[AttentionItem]| {
            lista
                .iter()
                .map(|item| item.reminder_id.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&primeira), ids(&segunda));
    }

    // --------------------------------------------------------- follow-up

    #[test]
    fn a_follow_up_carries_who_it_is_waiting_on() {
        let cobranca = NewReminder::at("Bases estruturais", "", at(50), &clock_at(1))
            .unwrap()
            .following_up("  Victor  ");
        assert_eq!(cobranca.kind, ReminderKind::FollowUp);
        assert_eq!(cobranca.waiting_for, "Victor");
    }

    #[test]
    fn an_impossible_recurrence_is_refused_at_creation() {
        let base = NewReminder::at("Regar as plantas", "", at(50), &clock_at(1)).unwrap();
        let quebrada = crate::Recurrence {
            rule: crate::RecurrenceRule::EveryDays { days: 0 },
            ..diaria()
        };
        assert!(base.clone().repeating(quebrada).is_err());
        assert!(base.repeating(diaria()).is_ok());
    }
}

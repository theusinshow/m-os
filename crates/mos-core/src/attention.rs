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
//! persistência decidido junto. `Relative` não existe e não é esquecimento: as
//! decisões D-1 e D-4 deixaram o M/OS sem prazo em Task e sem entidade Event,
//! então não há âncora de tempo futuro para se referir (§35.1).

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
                    CoreError::new(ErrorCode::InvalidInput, concat!($label, " invalido."), false)
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
                "Prioridade de Reminder desconhecida.",
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
}

impl Trigger {
    /// Quando este trigger vence a partir de agora, se vencer.
    pub fn next_due(&self, _now: OffsetDateTime) -> Option<OffsetDateTime> {
        match self {
            // Um instante no passado continua sendo o vencimento dele. Devolver
            // `None` aqui apagaria o Reminder atrasado — exatamente o que a
            // promessa da §1.1 proíbe. Quem decide o que fazer com atraso é a
            // reconciliação, não este cálculo.
            Self::At { instant } => Some(*instant),
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::At { .. } => "at",
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
            Self::Hidden => ("M/OS".to_owned(), "Um lembrete precisa de atenção.".to_owned()),
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
            created_at: now,
        })
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
    /// Perdeu utilidade sem ação, e havia política de expiração.
    Expire,
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
        }
    }
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
        // O tempo chegou. Só quem esperava pode vencer.
        (Scheduled | Snoozed, Transition::Ring) => {
            next.status = Due;
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
        }

        (state, Transition::Complete) if !state.is_terminal() => {
            next.status = Completed;
            next.completed_at = Some(now);
            next.next_due_at = None;
        }

        (state, Transition::Cancel) if !state.is_terminal() => {
            next.status = Cancelled;
            next.next_due_at = None;
        }

        // Perdido preserva `next_due_at`: é o instante original que permite
        // dizer "perdido há 50 min". Zerar aqui apagaria a única informação que
        // distingue um atraso de dez minutos de um de três dias.
        (Scheduled | Snoozed | Due, Transition::Miss) => {
            next.status = Missed;
        }

        (state, Transition::Expire) if !state.is_terminal() => {
            next.status = Expired;
            next.next_due_at = None;
        }

        _ => return refused(),
    }

    Ok(next)
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
    reminders
        .iter()
        .filter(|reminder| reminder.lifecycle_state == LifecycleState::Active)
        .filter(|reminder| reminder.status.is_waiting())
        .filter_map(|reminder| reminder.next_due_at)
        .min()
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
        assert_eq!(subject.overdue_by(at(10)), None, "no instante nao ha atraso");
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
}

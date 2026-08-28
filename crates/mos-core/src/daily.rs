//! Daily Session: a camada de INTENÇÃO sobre o que o M/OS já guarda.
//!
//! # O que ela é, e o que ela não é
//!
//! Uma [`crate::Task`] representa algo que precisa ser feito. Um
//! [`DailyObjective`] representa algo que a pessoa **decidiu que importa hoje**.
//! São perguntas diferentes, e por isso são entidades diferentes: a lista de
//! Tasks abertas de um sistema com meses de uso tem dezenas de linhas, e nenhuma
//! delas diz qual é a de hoje.
//!
//! Isto **não é** outra base de tarefas. Um objetivo pode apontar para uma Task,
//! para um Project ou para nada — e quando ele aponta, a Task continua sendo a
//! dona do trabalho. O objetivo só diz que ela é a que importa hoje.
//!
//! # As três decisões que moldaram o módulo
//!
//! **1. O dia é decidido uma vez e guardado.** O resto do M/OS guarda UTC e
//! deixa o renderer decidir a que dia um instante pertence (ver
//! `calendar.rs`) — ali isso está certo, porque um item de calendário não tem
//! identidade de dia. Aqui tem: "uma sessão por data" é impossível de garantir
//! se cada leitor decidir sozinho que dia é hoje. Então [`Day`] é campo, e o
//! fuso entra por parâmetro, do mesmo jeito que `voice_when` faz.
//!
//! **2. Não existe `mainObjectiveId` na sessão.** Qual objetivo é o principal já
//! está em [`ObjectivePriority`], e guardar a mesma resposta em dois lugares é
//! como as duas versões divergem — o `homeLayout.ts` tem um parágrafo inteiro
//! sobre uma duplicação assim que falhou no dia em que nasceu. Com merge por
//! campo (ver `docs/SYNC.md`) seria pior ainda: um dispositivo mudaria
//! `priority` e o outro `mainObjectiveId`, e os dois venceriam.
//!
//! **3. Não existe coluna `type` no objetivo.** O tipo é a presença e o tipo do
//! vínculo: sem vínculo é intenção livre, com vínculo é o que ele aponta. Uma
//! coluna a mais só criaria um segundo jeito de a mesma pergunta ser respondida.
//!
//! Tudo aqui é puro e recebe o tempo de fora. Regra temporal que lê o relógio
//! direto é regra que ninguém testa.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{CaptureId, CoreError, ErrorCode, MeetingId, ProjectId, ResourceId, TaskId};

macro_rules! daily_id {
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

            /// O UUID cru, para quem enderessa esta entidade FORA do M/OS —
            /// hoje, a sincronizacao. Ver `docs/SYNC.md`.
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

daily_id!(DailySessionId, "ID de sessao do dia");
daily_id!(DailyObjectiveId, "ID de objetivo do dia");

// ------------------------------------------------------------------- o dia

/// Uma data civil, `AAAA-MM-DD`, no fuso de quem estava na frente da tela.
///
/// Newtype e nao `String` solta porque ela e CHAVE: a unicidade da sessao do
/// dia depende de duas datas iguais serem a mesma string. Um `"2026-8-21"` que
/// entrasse por engano criaria uma segunda sessao para o mesmo dia sem nada
/// falhar.
///
/// Nao e `time::Date` por dois motivos que se somam: o formato de serie do
/// `time::Date` nao e ISO, entao ele atravessaria a ponte para o TypeScript
/// como um par de numeros; e o banco guarda TEXT, como guarda todo instante do
/// M/OS. O tipo aqui e o que impede o texto de ser qualquer texto.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Day(String);

const DAY_FORMAT: &[time::format_description::FormatItem<'static>] =
    time::macros::format_description!("[year]-[month]-[day]");

impl Day {
    /// O dia a que um instante pertence, **no offset em que ele chega**.
    ///
    /// Quem chama tem de passar o instante ja convertido para o fuso de quem
    /// esta olhando. Passar UTC aqui joga as madrugadas para o dia seguinte —
    /// e o `calendar.rs` tem o comentario que explica por que isso e um erro
    /// silencioso em vez de uma falha.
    pub fn from_local(moment: OffsetDateTime) -> Self {
        Self(
            moment
                .date()
                .format(DAY_FORMAT)
                .unwrap_or_else(|_| String::new()),
        )
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        let value = value.trim();
        time::Date::parse(value, DAY_FORMAT).map_err(|_| {
            CoreError::new(
                ErrorCode::InvalidInput,
                "Data invalida: use AAAA-MM-DD.",
                false,
            )
        })?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A data, para quem precisa fazer conta de calendario com ela.
    ///
    /// `Result` e nao `Date` direto porque `from_local` tem um caminho de
    /// formatacao que pode falhar e hoje cai num texto vazio — um `Day`
    /// invalido nao deveria existir, e enquanto ele puder, quem faz conta
    /// precisa poder recusar em vez de inventar uma data.
    pub fn date(&self) -> Result<time::Date, CoreError> {
        time::Date::parse(&self.0, DAY_FORMAT).map_err(|_| {
            CoreError::new(
                ErrorCode::DataIntegrity,
                "Data persistida e invalida.",
                false,
            )
        })
    }

    /// O contrario: a data vira `Day`.
    pub(crate) fn from_date(date: time::Date) -> Result<Self, CoreError> {
        date.format(DAY_FORMAT)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::DataIntegrity, "Data ilegivel.", false))
    }

    /// O dia anterior. Existe para a pergunta "o que sobrou de ontem?", e faz a
    /// conta com `time` em vez de com aritmetica de string por causa dos meses
    /// de 28, 30 e 31 dias.
    /// O primeiro instante deste dia, no fuso de quem esta olhando.
    ///
    /// Existe porque comparar um `Day` com um `OffsetDateTime` exige escolher um
    /// dos dois vocabularios, e converter o dia para instante e a direcao que
    /// nao perde informacao. O offset entra como parametro pela mesma razao de
    /// sempre: `Day` nao le relogio.
    pub fn inicio_do_dia(&self, offset: time::UtcOffset) -> OffsetDateTime {
        match self.date() {
            Ok(date) => date.midnight().assume_offset(offset),
            // Um `Day` invalido no banco nao pode derrubar a composicao do
            // painel. O epoch e a escolha inofensiva: ele nunca e "depois de".
            Err(_) => OffsetDateTime::UNIX_EPOCH,
        }
    }

    pub fn previous(&self) -> Self {
        match time::Date::parse(&self.0, DAY_FORMAT) {
            Ok(date) => Self(
                (date - Duration::days(1))
                    .format(DAY_FORMAT)
                    .unwrap_or_else(|_| self.0.clone()),
            ),
            Err(_) => self.clone(),
        }
    }
}

impl std::fmt::Display for Day {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

// ---------------------------------------------------------------- a sessao

/// Em que ponto do ciclo o dia esta.
///
/// `NotStarted` **nunca e gravado**: ele e a ausencia de linha, e e o que
/// [`DailyToday`] responde quando ninguem comecou o dia. Ele existe no enum
/// porque a interface precisa de um nome para esse estado — e um `Option` sem
/// nome vira `if (!session)` espalhado por sete componentes, cada um decidindo
/// por conta o que aquilo significa.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    #[default]
    NotStarted,
    Active,
    Completed,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }

    /// So os dois estados que existem em disco. `not_started` chegar aqui e
    /// erro de integridade, e nao um estado a aceitar: significaria uma linha
    /// gravada dizendo que nao existe.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de sessao do dia desconhecido.",
                false,
            )),
        }
    }
}

/// Um dia de trabalho dentro do M/OS.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySession {
    pub id: DailySessionId,
    pub day: Day,
    pub status: SessionStatus,
    /// A justificativa curta de quem montou o dia — hoje, so o Hermes escreve.
    ///
    /// Vazio significa nenhuma. **Nao e raciocinio**: e uma frase de contexto
    /// ("voce tem duas entregas hoje e uma reuniao as 15h"), limitada por
    /// [`MAX_NOTE`], para a sessao poder explicar de onde veio sem guardar o
    /// caminho que o modelo percorreu.
    pub note: String,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewDailySession {
    pub id: DailySessionId,
    pub day: Day,
    pub note: String,
    pub started_at: OffsetDateTime,
}

impl NewDailySession {
    pub fn create(day: Day, note: &str, now: OffsetDateTime) -> Result<Self, CoreError> {
        Ok(Self {
            id: DailySessionId::new(),
            day,
            note: clamp(note, MAX_NOTE),
            started_at: now,
        })
    }
}

// ------------------------------------------------------------- o vinculo

/// A que tipo de coisa do M/OS um objetivo pode apontar.
///
/// Cinco bracos, e nao os sete de [`crate::ReminderTarget`]. A diferenca e
/// deliberada: um lembrete pode ser sobre um App ou uma Conversa, mas
/// "o que importa hoje" ser um App registrado nao quer dizer nada. Recusar no
/// tipo e mais barato que descobrir na tela.
///
/// Tipo novo continua custando uma migration e uma linha em cada `match` — e o
/// preco que a ADR-012 aceitou ao recusar tabela generica de arestas.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Task,
    Project,
    Capture,
    Resource,
    Meeting,
}

impl LinkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Project => "project",
            Self::Capture => "capture",
            Self::Resource => "resource",
            Self::Meeting => "meeting",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_lowercase().as_str() {
            "task" | "tarefa" => Ok(Self::Task),
            "project" | "projeto" => Ok(Self::Project),
            "capture" | "captura" => Ok(Self::Capture),
            "resource" | "recurso" => Ok(Self::Resource),
            "meeting" | "reuniao" | "reunião" => Ok(Self::Meeting),
            _ => Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Tipo de vinculo de objetivo desconhecido.",
                false,
            )),
        }
    }
}

/// O par `(tipo, id)` que liga um objetivo a algo que ja existe.
///
/// `id` e `String` e nao um enum de ids tipados porque e assim que ele
/// atravessa a ponte e o banco — mas ele **nunca entra sem passar por
/// [`ObjectiveLink::new`]**, que valida o UUID contra o tipo certo. O tipo forte
/// esta na porta de entrada, e nao no campo.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveLink {
    pub kind: LinkKind,
    pub id: String,
}

impl ObjectiveLink {
    pub fn new(kind: LinkKind, id: &str) -> Result<Self, CoreError> {
        let canonical = match kind {
            LinkKind::Task => TaskId::parse(id)?.to_string(),
            LinkKind::Project => ProjectId::parse(id)?.to_string(),
            LinkKind::Capture => CaptureId::parse(id)?.to_string(),
            LinkKind::Resource => ResourceId::parse(id)?.to_string(),
            LinkKind::Meeting => MeetingId::parse(id)?.to_string(),
        };
        Ok(Self {
            kind,
            id: canonical,
        })
    }

    pub fn from_columns(kind: &str, id: &str) -> Result<Self, CoreError> {
        Self::new(LinkKind::parse(kind)?, id)
    }

    pub fn as_columns(&self) -> (&'static str, &str) {
        (self.kind.as_str(), self.id.as_str())
    }

    /// O id da Task, quando o vinculo e com uma. E o que a conclusao automatica
    /// pergunta, e o unico lugar onde o tipo volta a importar.
    pub fn task_id(&self) -> Option<TaskId> {
        (self.kind == LinkKind::Task)
            .then(|| TaskId::parse(&self.id).ok())
            .flatten()
    }
}

// ------------------------------------------------------------- o objetivo

/// O peso de um objetivo no dia.
///
/// Dois valores, e nao uma escala: "principal" e a coisa que faria o dia valer
/// a pena mesmo sozinha, e uma escala de cinco degraus destruiria exatamente
/// essa pergunta. Ver `UX-PRINCIPLES.md` — foco e o que a feature entrega.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectivePriority {
    Main,
    Secondary,
}

impl ObjectivePriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Secondary => "secondary",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "main" => Ok(Self::Main),
            "secondary" => Ok(Self::Secondary),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Prioridade de objetivo desconhecida.",
                false,
            )),
        }
    }
}

/// Onde o objetivo parou.
///
/// `CarriedOver` e `Dropped` sao desfechos DIFERENTES de propósito: um diz
/// "continua valendo, so nao hoje" e o outro diz "deixei de querer". Colapsar
/// os dois num "nao concluido" apagaria a unica informacao que a revisao
/// semanal precisa — o que foi carregado repetidamente.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Pending,
    Completed,
    CarriedOver,
    Dropped,
}

impl ObjectiveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::CarriedOver => "carried_over",
            Self::Dropped => "dropped",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "pending" => Ok(Self::Pending),
            "completed" => Ok(Self::Completed),
            "carried_over" => Ok(Self::CarriedOver),
            "dropped" => Ok(Self::Dropped),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de objetivo desconhecido.",
                false,
            )),
        }
    }

    /// O dia acabou para este objetivo. `Pending` e o unico que ainda pede
    /// decisao — e e sobre ele que o End My Day pergunta.
    pub fn is_resolved(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// Algo que a pessoa decidiu que importa hoje.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyObjective {
    pub id: DailyObjectiveId,
    pub session_id: DailySessionId,
    pub title: String,
    pub description: String,
    /// `None` e uma intencao livre — o que o pedido chamou de `type: free`. O
    /// tipo do objetivo E a presenca e o tipo deste vinculo; nao ha coluna
    /// separada, porque duas colunas para a mesma pergunta divergem.
    pub link: Option<ObjectiveLink>,
    pub priority: ObjectivePriority,
    pub status: ObjectiveStatus,
    /// Posicao dentro da sessao, de zero em diante. Chama-se `position` e nao
    /// `order` porque `order` e palavra reservada em SQL, e o nome do campo
    /// segue ate o banco.
    pub position: i64,
    /// O objetivo de que este veio, quando veio de um carry-over.
    ///
    /// E o que permite responder "isto ja foi adiado quatro vezes" sem varrer o
    /// historico inteiro comparando titulos — e titulo nao serve de chave,
    /// porque a pessoa pode reescrever o objetivo ao carrega-lo.
    pub carried_from: Option<DailyObjectiveId>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug)]
pub struct NewDailyObjective {
    pub id: DailyObjectiveId,
    pub session_id: DailySessionId,
    pub title: String,
    pub description: String,
    pub link: Option<ObjectiveLink>,
    pub priority: ObjectivePriority,
    pub position: i64,
    pub carried_from: Option<DailyObjectiveId>,
    pub created_at: OffsetDateTime,
}

impl NewDailyObjective {
    pub fn create(
        session_id: DailySessionId,
        title: &str,
        description: &str,
        link: Option<ObjectiveLink>,
        priority: ObjectivePriority,
        position: i64,
        now: OffsetDateTime,
    ) -> Result<Self, CoreError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O objetivo precisa de um titulo.",
                false,
            ));
        }
        Ok(Self {
            id: DailyObjectiveId::new(),
            session_id,
            title: clamp(title, MAX_TITLE),
            description: clamp(description, MAX_DESCRIPTION),
            link,
            priority,
            position: position.max(0),
            carried_from: None,
            created_at: now,
        })
    }

    pub fn carried_from(mut self, origin: DailyObjectiveId) -> Self {
        self.carried_from = Some(origin);
        self
    }
}

// ------------------------------------------------------------- a reflexao

/// Como o dia foi, em uma palavra.
///
/// Tres, e nao cinco estrelas: a pergunta e "da para seguir amanha do mesmo
/// jeito?", e ela tem tres respostas uteis. Escala numerica viraria metrica, e
/// metrica de humor e o comeco da gamificacao que o pedido recusa.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayMood {
    Productive,
    Normal,
    Blocked,
}

impl DayMood {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Productive => "productive",
            Self::Normal => "normal",
            Self::Blocked => "blocked",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "productive" => Ok(Self::Productive),
            "normal" => Ok(Self::Normal),
            "blocked" => Ok(Self::Blocked),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Humor do dia desconhecido.",
                false,
            )),
        }
    }
}

/// O fecho opcional do dia.
///
/// Dois campos, e nao os quatro que o pedido listou como possiveis. `wins` e
/// `blockers` sao a mesma frase repartida em duas caixas, e o proprio pedido
/// manda nao mostrar quatro campos — colunas que a interface nunca preenche sao
/// schema morto, e schema morto e o que uma migration futura nao sabe se pode
/// apagar.
///
/// Nao vira Capture: a Inbox e uma fila de coisas por PROCESSAR, e uma reflexao
/// arquivada la pediria uma decisao que ela nao tem. Ela pertence ao dia.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReflection {
    pub session_id: DailySessionId,
    pub mood: Option<DayMood>,
    pub summary: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewDailyReflection {
    pub session_id: DailySessionId,
    pub mood: Option<DayMood>,
    pub summary: String,
}

impl NewDailyReflection {
    /// `None` quando nao ha nada a guardar. Uma reflexao vazia gravada seria
    /// indistinguivel de uma reflexao que a pessoa escreveu e apagou.
    pub fn create(mood: Option<DayMood>, summary: &str) -> Option<Self> {
        let summary = clamp(summary, MAX_SUMMARY);
        if mood.is_none() && summary.is_empty() {
            return None;
        }
        Some(Self {
            session_id: DailySessionId::new(),
            mood,
            summary,
        })
    }

    pub fn for_session(mut self, session_id: DailySessionId) -> Self {
        self.session_id = session_id;
        self
    }
}

// ------------------------------------------------------------- os limites

/// Quantos secundarios a interface incentiva. **Nao e trava**: o banco aceita
/// mais, e o pedido pede exatamente isso — a UX incentiva foco, a estrutura nao
/// bloqueia. Uma trava aqui transformaria um bom conselho num erro.
pub const SUGGESTED_SECONDARIES: usize = 3;

const MAX_TITLE: usize = 200;
const MAX_DESCRIPTION: usize = 2_000;
const MAX_SUMMARY: usize = 2_000;
/// A justificativa do Hermes cabe em um paragrafo curto. O teto existe porque
/// ela vem de um modelo, e texto de modelo sem teto e texto sem teto.
const MAX_NOTE: usize = 400;

fn clamp(value: &str, limit: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= limit {
        return value.to_owned();
    }
    value
        .chars()
        .take(limit)
        .collect::<String>()
        .trim()
        .to_owned()
}

// ------------------------------------------------------- o dia, resolvido

/// O dia inteiro, do jeito que a interface le.
///
/// Uma estrutura so, e nao tres chamadas, porque a Home precisa dos tres na
/// PRIMEIRA pintura: sessao, objetivos e reflexao chegando em momentos
/// diferentes fariam o cartao aparecer e mudar de forma duas vezes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyToday {
    pub day: Day,
    pub status: SessionStatus,
    /// `None` quando `status` e `not_started`.
    pub session: Option<DailySession>,
    pub objectives: Vec<DailyObjective>,
    pub reflection: Option<DailyReflection>,
    /// A sessao de um dia ANTERIOR que ficou aberta.
    ///
    /// Existe porque ignora-la seria a pior das saidas: o dia de ontem some da
    /// tela e continua `active` no banco, e o historico passa a mentir. A Home
    /// oferece encerrar antes de comecar hoje — sem bloquear, porque bloquear a
    /// abertura do app por causa de um registro e desproporcional.
    pub stale: Option<DailySession>,
    pub stale_objectives: Vec<DailyObjective>,
}

impl DailyToday {
    /// O dia ainda nao comecou. E a pergunta que a Home faz primeiro.
    pub fn is_open(&self) -> bool {
        self.status == SessionStatus::Active
    }

    /// Quantos objetivos concluidos, de quantos que contavam.
    ///
    /// `Dropped` sai do denominador de proposito: abandonar um objetivo nao
    /// pode piorar o placar do dia, senao o sistema ensina a nao abandonar nada
    /// — que e o oposto do que o End My Day existe para permitir.
    pub fn progress(&self) -> (usize, usize) {
        let counted: Vec<_> = self
            .objectives
            .iter()
            .filter(|objective| objective.status != ObjectiveStatus::Dropped)
            .collect();
        let done = counted
            .iter()
            .filter(|objective| objective.status == ObjectiveStatus::Completed)
            .count();
        (done, counted.len())
    }

    pub fn main(&self) -> Option<&DailyObjective> {
        self.objectives
            .iter()
            .find(|objective| objective.priority == ObjectivePriority::Main)
    }

    /// O que ainda pede decisao no End My Day.
    pub fn unresolved(&self) -> Vec<&DailyObjective> {
        self.objectives
            .iter()
            .filter(|objective| !objective.status.is_resolved())
            .collect()
    }
}

/// Uma sessao passada, com o placar ja calculado.
///
/// O placar vem do backend e nao do front porque a lista do historico e um
/// resumo: mandar os objetivos de trinta dias para a tela somar tres numeros
/// seria carregar trinta vezes mais dado do que a tela mostra.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySessionSummary {
    pub session: DailySession,
    pub done: usize,
    pub total: usize,
    /// O titulo do objetivo principal, ou vazio quando o dia nao teve um.
    pub main_title: String,
    pub mood: Option<DayMood>,
}

/// Monta o resumo de uma sessao a partir dos objetivos dela.
///
/// Funcao pura e no core porque a regra do placar — `dropped` fora do
/// denominador — precisa ser a MESMA da [`DailyToday::progress`]. Duas contas
/// de progresso em dois lugares e como a Home e o historico passam a discordar
/// sobre o mesmo dia.
pub fn summarize(
    session: DailySession,
    objectives: &[DailyObjective],
    mood: Option<DayMood>,
) -> DailySessionSummary {
    let counted: Vec<_> = objectives
        .iter()
        .filter(|objective| objective.status != ObjectiveStatus::Dropped)
        .collect();
    DailySessionSummary {
        done: counted
            .iter()
            .filter(|objective| objective.status == ObjectiveStatus::Completed)
            .count(),
        total: counted.len(),
        main_title: objectives
            .iter()
            .find(|objective| objective.priority == ObjectivePriority::Main)
            .map(|objective| objective.title.clone())
            .unwrap_or_default(),
        mood,
        session,
    }
}

// ------------------------------------------------------ conclusao automatica

/// Concluir a Task vinculada tambem conclui o objetivo?
///
/// **So quando o objetivo E aquela Task.** Um objetivo ligado a um Project
/// ("avancar o 063-26") nao acaba porque uma Task dele acabou — e marca-lo como
/// concluido seria o sistema decidindo, em silencio, que o dia da pessoa
/// terminou. Um objetivo livre nao tem como saber de nada.
///
/// Funcao, e nao um `if` dentro do repositorio, porque e ELA que pode estar
/// errada — e regra que ninguem consegue chamar de um teste e regra que ninguem
/// conferiu.
pub fn completes_with_task(objective: &DailyObjective, task: TaskId) -> bool {
    objective
        .link
        .as_ref()
        .and_then(ObjectiveLink::task_id)
        .is_some_and(|linked| linked == task)
}

// ------------------------------------------------------------- o contexto

/// O que o M/OS ja sabe sobre hoje, antes de a pessoa escolher qualquer coisa.
///
/// Todos os numeros vem de entidades que ja existem. **Nenhuma entidade nova
/// foi criada para esta tela** — e tres itens que o pedido listou nao aparecem
/// aqui porque o M/OS nao os tem, e inventar um numero seria pior que a
/// ausencia:
///
/// - **Task nao tem prazo** (decisao D-1, ver `attention.rs`). O prazo de uma
///   Task no M/OS e um Reminder apontado para ela, e e ele que conta aqui.
/// - **Nao existe entidade Event** (decisao D-4). Nao ha agenda futura, entao
///   nao ha "compromissos de hoje" alem dos lembretes. Reuniao no M/OS e
///   gravacao, ou seja, fato passado.
/// - **Nao existe Waiting For** (registrado em `DECISIONS.md` e no §12 do
///   `HERMES-ACTION-LAYER.md`). Nao ha o que contar.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyContext {
    /// Lembretes ativos que vencem hoje. E o mais proximo de "prazo de hoje"
    /// que este sistema tem.
    pub due_today: usize,
    /// Lembretes que venceram e nao foram resolvidos.
    pub overdue: usize,
    /// Lembretes ativos com prioridade alta ou urgente.
    pub high_priority: usize,
    /// Reunioes registradas hoje. Passado, e nao agenda.
    pub meetings_today: usize,
    /// Captures esperando processamento na Inbox.
    pub inbox: usize,
    /// Captures das ultimas 24 horas — o que acabou de entrar e pode pedir acao.
    pub fresh_captures: usize,
    /// Projects ativos.
    pub projects: usize,
    /// Tasks em `doing`.
    pub doing: usize,
    /// Tasks abertas (tudo que nao esta em `done`), ativas.
    pub open_tasks: usize,
    /// As Tasks que o dia pode querer: em `doing` primeiro, depois as mexidas
    /// mais recentemente. E a lista que o seletor de objetivos abre mostrando.
    pub suggested_tasks: Vec<TaskSuggestion>,
    /// Os Projects mexidos mais recentemente.
    pub suggested_projects: Vec<ProjectSuggestion>,
    /// O que ficou pendente na ultima sessao encerrada ou aberta.
    pub carry_over: Vec<CarryOver>,
    /// A data da sessao de onde vieram os carry-overs. Vazia quando nao ha.
    pub carry_over_day: String,
    /// O que a faculdade poe no dia: entregas de hoje, atrasos e estudo
    /// sugerido.
    ///
    /// Chega PRONTO de `academic::compose_today`, e nao como Exams e Assignments
    /// crus: a regra de o que e "hoje" e de qual disciplina sugerir ja mora la,
    /// e reescreve-la aqui daria ao Start My Day uma nocao de hoje diferente da
    /// do painel do Academic.
    pub academic: Vec<AcademicObjectiveSuggestion>,
}

/// Uma sugestao academica pronta para virar objetivo do dia.
///
/// `link` aponta para a TASK da atividade quando ela existe — nao para a
/// atividade. `ObjectiveLink` so aceita os cinco tipos que a migration 0028
/// gravou no CHECK, e o vinculo util e mesmo a Task: e ela que se conclui no
/// quadro, e concluir a Task ja fecha a atividade do outro lado.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicObjectiveSuggestion {
    pub title: String,
    pub detail: String,
    /// Id da Task, quando a atividade tem uma.
    pub task_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSuggestion {
    pub id: String,
    pub title: String,
    pub state: String,
    /// Nome do Project, ou vazio.
    pub project: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSuggestion {
    pub id: String,
    pub name: String,
    /// Quantas Tasks abertas ele tem. E o que separa um Project vivo de um
    /// Project que so foi renomeado ontem.
    pub open_tasks: usize,
}

/// Um objetivo de um dia anterior que continua sem desfecho.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryOver {
    pub objective_id: DailyObjectiveId,
    pub title: String,
    pub link: Option<ObjectiveLink>,
    /// Quantas vezes esta corrente ja foi carregada. Um significa "veio de
    /// ontem"; quatro e a informacao que faz a pessoa decidir largar.
    pub times_carried: usize,
}

/// Tudo que [`compose_context`] precisa ler.
///
/// Estrutura em vez de dez parametros soltos, pela mesma razao do
/// `calendar::ComposeInput`: trocar duas colecoes de lugar por engano
/// compilaria sem reclamacao nenhuma.
pub struct ContextInput<'a> {
    /// O instante local de quem esta olhando — ja no offset certo.
    pub now_local: OffsetDateTime,
    pub reminders: &'a [crate::Reminder],
    pub tasks: &'a [crate::Task],
    pub projects: &'a [crate::Project],
    pub captures: &'a [crate::Capture],
    pub meetings: &'a [crate::Meeting],
    /// A ultima sessao que nao e a de hoje, com os objetivos dela.
    pub previous: Option<(&'a DailySession, &'a [DailyObjective])>,
    /// O recorte academico de hoje, quando o M/Academic tem semestre.
    pub academic: Option<&'a crate::AcademicToday>,
    /// Quantas vezes cada objetivo anterior ja foi carregado. Vem do
    /// repositorio porque a corrente pode ter dez elos, e segui-la em memoria
    /// exigiria carregar o historico inteiro.
    pub carry_depth: &'a dyn Fn(DailyObjectiveId) -> usize,
}

/// Quantas sugestoes a tela recebe. Curto de proposito: o seletor tem busca, e
/// uma lista longa de "talvez isto" e ruido, nao ajuda.
const MAX_SUGGESTIONS: usize = 8;

/// Monta o contexto do dia a partir do que ja foi lido.
///
/// PURA e sem repositorio, igual ao `calendar::compose`: e ela que carrega as
/// regras que podem estar erradas — o que conta como "hoje", o que e atraso, o
/// que vira sugestao e em que ordem —, e regra sem teste e regra que ninguem
/// conferiu. O comando do desktop so busca os dados e chama isto.
pub fn compose_context(input: ContextInput<'_>) -> DailyContext {
    let offset = input.now_local.offset();
    let today = Day::from_local(input.now_local);
    let now = input.now_local;

    let mut context = DailyContext::default();

    for reminder in input.reminders {
        if reminder.lifecycle_state != crate::LifecycleState::Active {
            continue;
        }
        if reminder.status.is_terminal() {
            continue;
        }
        if matches!(
            reminder.priority,
            crate::Priority::High | crate::Priority::Urgent
        ) {
            context.high_priority += 1;
        }
        let Some(due) = reminder.next_due_at else {
            continue;
        };
        if due < now {
            context.overdue += 1;
        } else if Day::from_local(due.to_offset(offset)) == today {
            context.due_today += 1;
        }
    }

    for meeting in input.meetings {
        if meeting.lifecycle_state != crate::LifecycleState::Active {
            continue;
        }
        if Day::from_local(meeting.started_at.to_offset(offset)) == today {
            context.meetings_today += 1;
        }
    }

    let day_ago = now - Duration::hours(24);
    for capture in input.captures {
        if capture.lifecycle_state != crate::LifecycleState::Active {
            continue;
        }
        if capture.processing_state == crate::ProcessingState::Inbox {
            context.inbox += 1;
        }
        if capture.captured_at >= day_ago {
            context.fresh_captures += 1;
        }
    }

    let active_projects: Vec<_> = input
        .projects
        .iter()
        .filter(|project| project.lifecycle_state == crate::LifecycleState::Active)
        .collect();
    context.projects = active_projects.len();

    let project_name = |id: Option<ProjectId>| -> String {
        id.and_then(|id| active_projects.iter().find(|project| project.id == id))
            .map(|project| project.name.clone())
            .unwrap_or_default()
    };

    let mut open: Vec<&crate::Task> = input
        .tasks
        .iter()
        .filter(|task| {
            task.lifecycle_state == crate::LifecycleState::Active
                && task.state != crate::TaskState::Done
        })
        .collect();
    context.open_tasks = open.len();
    context.doing = open
        .iter()
        .filter(|task| task.state == crate::TaskState::Doing)
        .count();

    // `doing` primeiro, e depois o que foi mexido por ultimo. A ordem e a
    // resposta a "o que eu estava fazendo": uma Task em andamento e sempre mais
    // candidata a objetivo do dia que uma do backlog editada na mesma hora.
    open.sort_by(|left, right| {
        let rank = |task: &crate::Task| {
            if task.state == crate::TaskState::Doing {
                0
            } else {
                1
            }
        };
        rank(left)
            .cmp(&rank(right))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
    });
    context.suggested_tasks = open
        .iter()
        .take(MAX_SUGGESTIONS)
        .map(|task| TaskSuggestion {
            id: task.id.to_string(),
            title: task.title.clone(),
            state: task.state.as_str().to_owned(),
            project: project_name(task.project_id),
        })
        .collect();

    let mut projects_ranked = active_projects.clone();
    projects_ranked.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    context.suggested_projects = projects_ranked
        .iter()
        .take(MAX_SUGGESTIONS)
        .map(|project| ProjectSuggestion {
            id: project.id.to_string(),
            name: project.name.clone(),
            open_tasks: open
                .iter()
                .filter(|task| task.project_id == Some(project.id))
                .count(),
        })
        .collect();

    if let Some((session, objectives)) = input.previous {
        context.carry_over_day = session.day.to_string();
        context.carry_over = objectives
            .iter()
            .filter(|objective| {
                matches!(
                    objective.status,
                    ObjectiveStatus::Pending | ObjectiveStatus::CarriedOver
                )
            })
            .map(|objective| CarryOver {
                objective_id: objective.id,
                title: objective.title.clone(),
                link: objective.link.clone(),
                times_carried: (input.carry_depth)(objective.id),
            })
            .collect();
    }

    // A faculdade entra no fim da lista, e nao no topo: o dia comeca pelo que
    // JA estava em andamento, e a entrega de amanha nao pode empurrar a Task de
    // ontem para fora da primeira tela. Atraso vem antes de prazo de hoje, que
    // vem antes de estudo — a mesma ordem de urgencia do painel.
    if let Some(academico) = input.academic {
        for item in academico.overdue.iter().chain(academico.due_today.iter()) {
            context.academic.push(AcademicObjectiveSuggestion {
                title: format!("Entregar {}", item.title),
                detail: format!(
                    "{} · {}",
                    item.subject,
                    if item.horizonte == crate::Horizonte::Overdue {
                        "atrasada"
                    } else {
                        "vence hoje"
                    }
                ),
                task_id: item.task_id.clone(),
            });
        }
        for sugestao in &academico.study_suggestions {
            context.academic.push(AcademicObjectiveSuggestion {
                title: format!("Estudar {}", sugestao.subject),
                detail: sugestao.reason.clone(),
                task_id: None,
            });
        }
    }
    context
}

// ---------------------------------------------------------------- entradas

/// O que o Start My Day recebe.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDayInput {
    /// Vazio significa dia sem principal — legitimo, e o `UX` nao obriga.
    #[serde(default)]
    pub main: Option<ObjectiveDraft>,
    #[serde(default)]
    pub secondaries: Vec<ObjectiveDraft>,
    /// A justificativa curta, quando veio do Hermes.
    #[serde(default)]
    pub note: String,
}

/// Um objetivo como a interface (ou o Hermes) o descreve, antes de existir.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveDraft {
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// `task`, `project`, `capture`, `resource` ou `meeting`. Vazio e livre.
    #[serde(default)]
    pub link_kind: String,
    #[serde(default)]
    pub link_id: String,
    /// O objetivo de ontem de que este veio, quando veio de um carry-over.
    #[serde(default)]
    pub carried_from: String,
}

impl ObjectiveDraft {
    /// Vira o objetivo de verdade, validando o vinculo.
    ///
    /// O titulo pode vir vazio quando ha vinculo: nesse caso quem chama
    /// preenche com o titulo da entidade — o desktop faz isso em
    /// `daily::rotulo_do_vinculo`, porque so ele conhece o banco.
    pub fn build(
        &self,
        session: DailySessionId,
        priority: ObjectivePriority,
        position: i64,
        now: OffsetDateTime,
    ) -> Result<NewDailyObjective, CoreError> {
        let link = self.link()?;
        let mut objective = NewDailyObjective::create(
            session,
            &self.title,
            &self.description,
            link,
            priority,
            position,
            now,
        )?;
        if !self.carried_from.trim().is_empty() {
            objective = objective.carried_from(DailyObjectiveId::parse(&self.carried_from)?);
        }
        Ok(objective)
    }

    pub fn link(&self) -> Result<Option<ObjectiveLink>, CoreError> {
        let kind = self.link_kind.trim();
        let id = self.link_id.trim();
        match (kind.is_empty(), id.is_empty()) {
            (true, true) => Ok(None),
            (false, false) => ObjectiveLink::from_columns(kind, id).map(Some),
            // Metade de um vinculo e um vinculo que nao resolve. Mesma regra do
            // `reminders_target_whole` na migration 0015.
            _ => Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Um vinculo de objetivo precisa do tipo e do id.",
                false,
            )),
        }
    }
}

/// O que o End My Day recebe.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndDayInput {
    /// O destino de cada objetivo que ainda estava pendente.
    ///
    /// Objetivo pendente que NAO aparecer aqui fica pendente — e ele continua
    /// aparecendo no carry-over do proximo Start My Day. Nao decidir e uma
    /// resposta valida, e transformar silencio em "abandonado" seria o sistema
    /// escolhendo por quem nao escolheu.
    #[serde(default)]
    pub resolutions: Vec<ObjectiveResolution>,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveResolution {
    pub objective_id: String,
    /// `completed`, `carried_over`, `dropped` ou `pending`.
    pub status: String,
}

impl EndDayInput {
    pub fn reflection(&self) -> Result<Option<NewDailyReflection>, CoreError> {
        let mood = match self.mood.trim() {
            "" => None,
            value => Some(DayMood::parse(value)?),
        };
        Ok(NewDailyReflection::create(mood, &self.summary))
    }

    pub fn parsed_resolutions(
        &self,
    ) -> Result<Vec<(DailyObjectiveId, ObjectiveStatus)>, CoreError> {
        self.resolutions
            .iter()
            .map(|resolution| {
                Ok((
                    DailyObjectiveId::parse(&resolution.objective_id)?,
                    ObjectiveStatus::parse(&resolution.status)?,
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn session(day: &str, status: SessionStatus) -> DailySession {
        DailySession {
            id: DailySessionId::new(),
            day: Day::parse(day).unwrap(),
            status,
            note: String::new(),
            started_at: datetime!(2026-08-21 09:00 -03:00),
            ended_at: None,
            created_at: datetime!(2026-08-21 09:00 -03:00),
            updated_at: datetime!(2026-08-21 09:00 -03:00),
        }
    }

    fn objective(
        session: DailySessionId,
        title: &str,
        priority: ObjectivePriority,
        status: ObjectiveStatus,
        link: Option<ObjectiveLink>,
    ) -> DailyObjective {
        DailyObjective {
            id: DailyObjectiveId::new(),
            session_id: session,
            title: title.to_owned(),
            description: String::new(),
            link,
            priority,
            status,
            position: 0,
            carried_from: None,
            created_at: datetime!(2026-08-21 09:00 -03:00),
            updated_at: datetime!(2026-08-21 09:00 -03:00),
            completed_at: None,
        }
    }

    fn task_link() -> (TaskId, ObjectiveLink) {
        let id = TaskId::parse("018f0000-0000-7000-8000-000000000001").unwrap();
        (
            id,
            ObjectiveLink::new(LinkKind::Task, &id.to_string()).unwrap(),
        )
    }

    // ------------------------------------------------------------------ Day

    #[test]
    fn o_dia_vem_do_offset_de_quem_esta_olhando() {
        // 23:30 de 21/08 em UTC-3 e 02:30 de 22/08 em UTC. Se o dia fosse
        // decidido em UTC, o trabalho da madrugada cairia no dia seguinte.
        let noite = datetime!(2026-08-21 23:30 -03:00);
        assert_eq!(Day::from_local(noite).as_str(), "2026-08-21");
        assert_eq!(
            Day::from_local(noite.to_offset(time::UtcOffset::UTC)).as_str(),
            "2026-08-22"
        );
    }

    #[test]
    fn dia_invalido_e_recusado_em_vez_de_normalizado() {
        assert!(
            Day::parse("2026-8-21").is_err(),
            "sem zero a esquerda cria uma segunda chave para o mesmo dia"
        );
        assert!(Day::parse("21/08/2026").is_err());
        assert!(Day::parse("").is_err());
        assert!(Day::parse("2026-02-30").is_err());
        assert_eq!(Day::parse(" 2026-08-21 ").unwrap().as_str(), "2026-08-21");
    }

    #[test]
    fn o_dia_anterior_atravessa_mes_e_ano() {
        assert_eq!(
            Day::parse("2026-03-01").unwrap().previous().as_str(),
            "2026-02-28"
        );
        assert_eq!(
            Day::parse("2026-01-01").unwrap().previous().as_str(),
            "2025-12-31"
        );
        // 2028 e bissexto: a conta nao pode ser "menos um dia no numero".
        assert_eq!(
            Day::parse("2028-03-01").unwrap().previous().as_str(),
            "2028-02-29"
        );
    }

    // ----------------------------------------------------------- vocabulario

    /// Os nomes atravessam a ponte para o TypeScript. Um rename silencioso aqui
    /// faria a tela deixar de reconhecer o estado, sem erro de compilacao de
    /// nenhum dos dois lados. Mesmo teste que o `calendar.rs` tem.
    #[test]
    fn os_estados_atravessam_a_ponte_com_o_nome_de_disco() {
        for status in [
            ObjectiveStatus::Pending,
            ObjectiveStatus::Completed,
            ObjectiveStatus::CarriedOver,
            ObjectiveStatus::Dropped,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", status.as_str()));
            assert_eq!(ObjectiveStatus::parse(status.as_str()).unwrap(), status);
        }
        for priority in [ObjectivePriority::Main, ObjectivePriority::Secondary] {
            assert_eq!(
                serde_json::to_string(&priority).unwrap(),
                format!("\"{}\"", priority.as_str())
            );
        }
        for mood in [DayMood::Productive, DayMood::Normal, DayMood::Blocked] {
            assert_eq!(
                serde_json::to_string(&mood).unwrap(),
                format!("\"{}\"", mood.as_str())
            );
        }
        for kind in [
            LinkKind::Task,
            LinkKind::Project,
            LinkKind::Capture,
            LinkKind::Resource,
            LinkKind::Meeting,
        ] {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.as_str())
            );
        }
    }

    #[test]
    fn not_started_nunca_volta_do_banco() {
        // Ele existe para a interface nomear a ausencia. Uma LINHA gravada
        // dizendo "nao comecou" seria um estado que se contradiz.
        assert!(SessionStatus::parse("not_started").is_err());
        assert_eq!(
            SessionStatus::parse("active").unwrap(),
            SessionStatus::Active
        );
        assert_eq!(
            SessionStatus::parse("completed").unwrap(),
            SessionStatus::Completed
        );
        assert_eq!(SessionStatus::default(), SessionStatus::NotStarted);
    }

    // --------------------------------------------------------------- vinculo

    #[test]
    fn o_vinculo_valida_o_id_contra_o_tipo() {
        let (_, link) = task_link();
        assert_eq!(link.kind, LinkKind::Task);
        assert!(ObjectiveLink::new(LinkKind::Task, "nao-e-uuid").is_err());
        assert!(
            ObjectiveLink::from_columns("app", "018f0000-0000-7000-8000-000000000001").is_err(),
            "App nao e objetivo de dia"
        );
    }

    #[test]
    fn meio_vinculo_e_recusado() {
        let draft = ObjectiveDraft {
            title: "x".into(),
            link_kind: "task".into(),
            ..Default::default()
        };
        assert!(draft.link().is_err());
        let draft = ObjectiveDraft {
            title: "x".into(),
            link_id: "018f0000-0000-7000-8000-000000000001".into(),
            ..Default::default()
        };
        assert!(draft.link().is_err());
        let livre = ObjectiveDraft {
            title: "x".into(),
            ..Default::default()
        };
        assert!(livre.link().unwrap().is_none());
    }

    // ------------------------------------------------------------- objetivo

    #[test]
    fn objetivo_sem_titulo_e_recusado() {
        let id = DailySessionId::new();
        assert!(NewDailyObjective::create(
            id,
            "   ",
            "",
            None,
            ObjectivePriority::Main,
            0,
            datetime!(2026-08-21 09:00 UTC)
        )
        .is_err());
    }

    #[test]
    fn titulo_longo_e_cortado_e_nao_recusado() {
        let id = DailySessionId::new();
        let longo = "a".repeat(500);
        let objetivo = NewDailyObjective::create(
            id,
            &longo,
            "",
            None,
            ObjectivePriority::Main,
            0,
            datetime!(2026-08-21 09:00 UTC),
        )
        .unwrap();
        assert_eq!(
            objetivo.title.chars().count(),
            MAX_TITLE,
            "cortar mantem o gesto; recusar perderia o que a pessoa escreveu"
        );
    }

    // ------------------------------------------------------------- progresso

    #[test]
    fn abandonar_um_objetivo_nao_piora_o_placar() {
        let id = DailySessionId::new();
        let hoje = DailyToday {
            day: Day::parse("2026-08-21").unwrap(),
            status: SessionStatus::Active,
            session: Some(session("2026-08-21", SessionStatus::Active)),
            objectives: vec![
                objective(
                    id,
                    "planta",
                    ObjectivePriority::Main,
                    ObjectiveStatus::Completed,
                    None,
                ),
                objective(
                    id,
                    "memorial",
                    ObjectivePriority::Secondary,
                    ObjectiveStatus::Pending,
                    None,
                ),
                objective(
                    id,
                    "desisti",
                    ObjectivePriority::Secondary,
                    ObjectiveStatus::Dropped,
                    None,
                ),
            ],
            reflection: None,
            stale: None,
            stale_objectives: Vec::new(),
        };
        assert_eq!(
            hoje.progress(),
            (1, 2),
            "o abandonado sai dos dois lados da fracao"
        );
        assert_eq!(hoje.main().unwrap().title, "planta");
        assert_eq!(hoje.unresolved().len(), 1);
        assert!(hoje.is_open());
    }

    #[test]
    fn o_resumo_do_historico_conta_igual_a_home() {
        let sessao = session("2026-08-20", SessionStatus::Completed);
        let id = sessao.id;
        let objetivos = vec![
            objective(
                id,
                "planta",
                ObjectivePriority::Main,
                ObjectiveStatus::Completed,
                None,
            ),
            objective(
                id,
                "memorial",
                ObjectivePriority::Secondary,
                ObjectiveStatus::CarriedOver,
                None,
            ),
            objective(
                id,
                "largado",
                ObjectivePriority::Secondary,
                ObjectiveStatus::Dropped,
                None,
            ),
        ];
        let resumo = summarize(sessao, &objetivos, Some(DayMood::Normal));
        assert_eq!((resumo.done, resumo.total), (1, 2));
        assert_eq!(resumo.main_title, "planta");
    }

    // --------------------------------------------------- conclusao automatica

    #[test]
    fn so_o_objetivo_que_e_a_task_fecha_junto_com_ela() {
        let sessao = DailySessionId::new();
        let (task, link) = task_link();
        let outra = TaskId::parse("018f0000-0000-7000-8000-000000000009").unwrap();
        let project = ProjectId::parse("018f0000-0000-7000-8000-000000000002").unwrap();

        let da_task = objective(
            sessao,
            "enviar",
            ObjectivePriority::Main,
            ObjectiveStatus::Pending,
            Some(link),
        );
        let do_project = objective(
            sessao,
            "avancar 063-26",
            ObjectivePriority::Secondary,
            ObjectiveStatus::Pending,
            Some(ObjectiveLink::new(LinkKind::Project, &project.to_string()).unwrap()),
        );
        let livre = objective(
            sessao,
            "pensar",
            ObjectivePriority::Secondary,
            ObjectiveStatus::Pending,
            None,
        );

        assert!(completes_with_task(&da_task, task));
        assert!(
            !completes_with_task(&da_task, outra),
            "outra Task nao fecha este objetivo"
        );
        assert!(
            !completes_with_task(&do_project, task),
            "um objetivo de Project e maior que uma Task dele"
        );
        assert!(!completes_with_task(&livre, task));
    }

    // --------------------------------------------------------------- reflexao

    #[test]
    fn reflexao_vazia_nao_vira_linha() {
        assert!(NewDailyReflection::create(None, "   ").is_none());
        assert!(NewDailyReflection::create(Some(DayMood::Blocked), "").is_some());
        assert!(NewDailyReflection::create(None, "deu certo").is_some());
    }

    // ---------------------------------------------------------------- entrada

    #[test]
    fn o_fim_do_dia_le_destinos_e_humor() {
        let entrada = EndDayInput {
            resolutions: vec![ObjectiveResolution {
                objective_id: "018f0000-0000-7000-8000-000000000003".into(),
                status: "carried_over".into(),
            }],
            mood: "blocked".into(),
            summary: "o 063-26 tomou mais tempo".into(),
        };
        let destinos = entrada.parsed_resolutions().unwrap();
        assert_eq!(destinos.len(), 1);
        assert_eq!(destinos[0].1, ObjectiveStatus::CarriedOver);
        let reflexao = entrada.reflection().unwrap().unwrap();
        assert_eq!(reflexao.mood, Some(DayMood::Blocked));
    }

    #[test]
    fn destino_desconhecido_e_recusado() {
        let entrada = EndDayInput {
            resolutions: vec![ObjectiveResolution {
                objective_id: "018f0000-0000-7000-8000-000000000003".into(),
                status: "talvez".into(),
            }],
            ..Default::default()
        };
        assert!(entrada.parsed_resolutions().is_err());
    }

    // ---------------------------------------------------------------- contexto

    fn task(
        title: &str,
        state: crate::TaskState,
        project: Option<ProjectId>,
        updated: OffsetDateTime,
    ) -> crate::Task {
        crate::Task {
            id: TaskId::new(),
            title: title.to_owned(),
            description: String::new(),
            project_id: project,
            source_capture_id: None,
            state,
            lifecycle_state: crate::LifecycleState::Active,
            created_at: updated,
            updated_at: updated,
            completed_at: None,
        }
    }

    fn sem_profundidade() -> impl Fn(DailyObjectiveId) -> usize {
        |_| 0
    }

    #[test]
    fn o_contexto_ordena_doing_antes_do_resto() {
        let agora = datetime!(2026-08-21 09:00 -03:00);
        let tasks = [
            task("backlog recente", crate::TaskState::Backlog, None, agora),
            task(
                "em andamento antiga",
                crate::TaskState::Doing,
                None,
                agora - Duration::days(9),
            ),
            task("concluida", crate::TaskState::Done, None, agora),
        ];
        let profundidade = sem_profundidade();
        let contexto = compose_context(ContextInput {
            academic: None,
            now_local: agora,
            reminders: &[],
            tasks: &tasks,
            projects: &[],
            captures: &[],
            meetings: &[],
            previous: None,
            carry_depth: &profundidade,
        });
        assert_eq!(contexto.open_tasks, 2, "done nao e Task aberta");
        assert_eq!(contexto.doing, 1);
        assert_eq!(contexto.suggested_tasks[0].title, "em andamento antiga");
        assert_eq!(contexto.suggested_tasks[1].title, "backlog recente");
    }

    #[test]
    fn atrasado_e_de_hoje_sao_contagens_diferentes() {
        let agora = datetime!(2026-08-21 09:00 -03:00);
        let lembrete = |quando: OffsetDateTime, prioridade: crate::Priority| crate::Reminder {
            id: crate::ReminderId::new(),
            title: "x".into(),
            body: String::new(),
            target: None,
            trigger: crate::Trigger::At { instant: quando },
            priority: prioridade,
            status: crate::ReminderStatus::Scheduled,
            policy: crate::DeliveryPolicy {
                snooze_allowed: true,
                privacy: crate::ContentPrivacy::ShowContent,
            },
            source: crate::ReminderSource::User,
            next_due_at: Some(quando),
            snooze_count: 0,
            delivered_count: 0,
            created_at: agora,
            updated_at: agora,
            completed_at: None,
            lifecycle_state: crate::LifecycleState::Active,
        };
        let reminders = [
            lembrete(agora + Duration::hours(6), crate::Priority::Normal),
            lembrete(agora - Duration::hours(30), crate::Priority::Urgent),
            // Amanha: nao e de hoje nem esta atrasado.
            lembrete(agora + Duration::hours(30), crate::Priority::Normal),
        ];
        let profundidade = sem_profundidade();
        let contexto = compose_context(ContextInput {
            academic: None,
            now_local: agora,
            reminders: &reminders,
            tasks: &[],
            projects: &[],
            captures: &[],
            meetings: &[],
            previous: None,
            carry_depth: &profundidade,
        });
        assert_eq!(contexto.due_today, 1);
        assert_eq!(contexto.overdue, 1);
        assert_eq!(contexto.high_priority, 1);
    }

    #[test]
    fn o_carry_over_traz_pendente_e_ja_carregado_e_ignora_o_resto() {
        let agora = datetime!(2026-08-21 09:00 -03:00);
        let ontem = session("2026-08-20", SessionStatus::Completed);
        let id = ontem.id;
        let objetivos = vec![
            objective(
                id,
                "pendente",
                ObjectivePriority::Main,
                ObjectiveStatus::Pending,
                None,
            ),
            objective(
                id,
                "ja carregado",
                ObjectivePriority::Secondary,
                ObjectiveStatus::CarriedOver,
                None,
            ),
            objective(
                id,
                "feito",
                ObjectivePriority::Secondary,
                ObjectiveStatus::Completed,
                None,
            ),
            objective(
                id,
                "largado",
                ObjectivePriority::Secondary,
                ObjectiveStatus::Dropped,
                None,
            ),
        ];
        let profundidade = |_: DailyObjectiveId| 2usize;
        let contexto = compose_context(ContextInput {
            academic: None,
            now_local: agora,
            reminders: &[],
            tasks: &[],
            projects: &[],
            captures: &[],
            meetings: &[],
            previous: Some((&ontem, &objetivos)),
            carry_depth: &profundidade,
        });
        assert_eq!(contexto.carry_over_day, "2026-08-20");
        let titulos: Vec<_> = contexto
            .carry_over
            .iter()
            .map(|item| item.title.as_str())
            .collect();
        assert_eq!(titulos, ["pendente", "ja carregado"]);
        assert_eq!(contexto.carry_over[0].times_carried, 2);
    }
}

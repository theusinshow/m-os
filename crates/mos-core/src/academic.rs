//! M/Academic — a camada academica do M/OS.
//!
//! O que este modulo NAO e: um portal da faculdade. Ele nao guarda ementa, nao
//! fala com a instituicao e nao tenta representar um curso. Ele responde cinco
//! perguntas — o que tenho, o que esta chegando, o que preciso fazer, o que
//! preciso estudar, como estou — e todas elas se respondem com o que a pessoa
//! escreveu.
//!
//! # A regra que organiza tudo
//!
//! **Faculdade e um CONTEXTO sobre os primitivos do M/OS, e nao um segundo
//! M/OS.** A atividade que exige acao aponta para uma `Task` de verdade; o
//! material e um `Resource` de verdade; a prova entra no Calendario que ja
//! existe. O que este modulo acrescenta e o que nao tinha lugar: o periodo, a
//! disciplina, o peso na media e o tempo de estudo.
//!
//! # Puro
//!
//! Como `calendar::compose`, `daily::compose_context` e `weekly::compose_week`:
//! as decisoes que podem estar erradas moram aqui, com teste, e o comando do
//! desktop so busca e delega. O que pode estar errado aqui e bastante — o que e
//! "chegando", como uma media pondera pesos que nao somam 1, o que conta como
//! progresso, quando um prazo virou atraso.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{CoreError, Day, ErrorCode, LifecycleState};

macro_rules! academic_id {
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

academic_id!(SemesterId, "Semester ID");
academic_id!(SubjectId, "Subject ID");
academic_id!(AssignmentId, "Assignment ID");
academic_id!(ExamId, "Exam ID");
academic_id!(StudySessionId, "Study session ID");

// ===========================================================================
// Semestre
// ===========================================================================

/// Onde o semestre esta em relacao a hoje.
///
/// DERIVADO das datas, e nunca guardado. Um campo `status` no banco criaria a
/// linha que diz "ativo" num semestre que acabou em dezembro, e o sistema
/// passaria a depender de alguem lembrar de corrigi-la.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemesterStatus {
    Upcoming,
    Active,
    Completed,
}

impl SemesterStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upcoming => "upcoming",
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Semester {
    pub id: SemesterId,
    pub name: String,
    pub institution: String,
    pub starts_on: Day,
    pub ends_on: Day,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Semester {
    /// Onde este semestre esta em relacao a um dia.
    ///
    /// O dia entra como parametro, e nao e lido do relogio: funcao pura que le
    /// o relogio nao se testa, e o dia civil depende do fuso de quem olha.
    pub fn status_em(&self, hoje: &Day) -> SemesterStatus {
        if hoje < &self.starts_on {
            SemesterStatus::Upcoming
        } else if hoje > &self.ends_on {
            SemesterStatus::Completed
        } else {
            SemesterStatus::Active
        }
    }
}

/// O semestre que a tela deve abrir mostrando.
///
/// A ordem de preferencia e: o corrente; senao o proximo a comecar; senao o
/// ultimo que terminou. Um sistema que abre vazio em janeiro — entre um semestre
/// e outro — faria a pessoa procurar o proprio historico, e o historico e a
/// unica coisa que ela tem naquele momento.
pub fn semestre_corrente<'a>(semestres: &'a [Semester], hoje: &Day) -> Option<&'a Semester> {
    let ativos: Vec<&Semester> = semestres
        .iter()
        .filter(|semestre| semestre.lifecycle_state == LifecycleState::Active)
        .collect();

    ativos
        .iter()
        .copied()
        .filter(|semestre| semestre.status_em(hoje) == SemesterStatus::Active)
        .min_by(|a, b| a.ends_on.cmp(&b.ends_on))
        .or_else(|| {
            ativos
                .iter()
                .copied()
                .filter(|semestre| semestre.status_em(hoje) == SemesterStatus::Upcoming)
                .min_by(|a, b| a.starts_on.cmp(&b.starts_on))
        })
        .or_else(|| {
            ativos
                .iter()
                .copied()
                .filter(|semestre| semestre.status_em(hoje) == SemesterStatus::Completed)
                .max_by(|a, b| a.ends_on.cmp(&b.ends_on))
        })
}

// ===========================================================================
// Disciplina
// ===========================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub id: SubjectId,
    pub semester_id: SemesterId,
    pub name: String,
    pub code: String,
    pub teacher: String,
    /// Nome de accent do design system. Vazio significa o accent padrao.
    pub accent: String,
    pub notes: String,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Os accents que uma disciplina pode usar.
///
/// Lista fechada e por NOME: cor crua gravada no banco nao acompanharia a troca
/// de tema, e o M/OS tem dois. Vazio e valido e significa o accent padrao.
pub const SUBJECT_ACCENTS: [&str; 6] = ["trigo", "cobre", "musgo", "lilas", "argila", "ceu"];

pub fn validate_accent(value: &str) -> Result<String, CoreError> {
    let value = value.trim().to_lowercase();
    if value.is_empty() || SUBJECT_ACCENTS.contains(&value.as_str()) {
        return Ok(value);
    }
    Err(CoreError::new(
        ErrorCode::InvalidInput,
        "Esse accent nao existe no design system.",
        false,
    ))
}

// ===========================================================================
// Atividade e avaliacao
// ===========================================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    Pending,
    InProgress,
    Submitted,
    Graded,
    Cancelled,
}

impl AssignmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Submitted => "submitted",
            Self::Graded => "graded",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim() {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "submitted" => Ok(Self::Submitted),
            "graded" => Ok(Self::Graded),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de atividade desconhecido no banco local.",
                false,
            )),
        }
    }

    /// Acabou, de um jeito ou de outro. Nao pede mais acao e nao conta atraso.
    pub fn is_settled(self) -> bool {
        matches!(self, Self::Submitted | Self::Graded | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub id: AssignmentId,
    pub subject_id: SubjectId,
    pub title: String,
    pub description: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
    pub status: AssignmentStatus,
    pub priority: crate::Priority,
    pub weight: f64,
    pub max_score: Option<f64>,
    pub score: Option<f64>,
    /// A Task do M/OS que executa esta atividade, quando existe.
    pub task_id: Option<crate::TaskId>,
    /// A decisao da pessoa. Ver `academic_decision`.
    pub decision: crate::academic_decision::Decision,
    #[serde(with = "time::serde::rfc3339::option")]
    pub decided_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub planned_at: Option<OffsetDateTime>,
    pub planned_minutes: i64,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExamStatus {
    Scheduled,
    Done,
    Graded,
    Cancelled,
}

impl ExamStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Done => "done",
            Self::Graded => "graded",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim() {
            "scheduled" => Ok(Self::Scheduled),
            "done" => Ok(Self::Done),
            "graded" => Ok(Self::Graded),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de avaliacao desconhecido no banco local.",
                false,
            )),
        }
    }

    pub fn is_settled(self) -> bool {
        matches!(self, Self::Done | Self::Graded | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exam {
    pub id: ExamId,
    pub subject_id: SubjectId,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub location: String,
    pub topics: String,
    pub weight: f64,
    pub max_score: Option<f64>,
    pub score: Option<f64>,
    pub status: ExamStatus,
    /// A decisao da pessoa. Ver `academic_decision`.
    pub decision: crate::academic_decision::Decision,
    #[serde(with = "time::serde::rfc3339::option")]
    pub decided_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub planned_at: Option<OffsetDateTime>,
    pub planned_minutes: i64,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

// ===========================================================================
// Sessao de estudo
// ===========================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    pub id: StudySessionId,
    pub subject_id: SubjectId,
    pub topic: String,
    pub notes: String,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    pub seconds: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl StudySession {
    pub fn em_curso(&self) -> bool {
        self.ended_at.is_none()
    }

    /// Quantos segundos esta sessao vale AGORA.
    ///
    /// Fechada, vale o que foi gravado. Aberta, vale o que ja passou — senao o
    /// "estudei hoje" ficaria em zero durante a sessao inteira e so pularia no
    /// fim, que e o momento em que o numero deixa de importar.
    pub fn segundos_em(&self, agora: OffsetDateTime) -> i64 {
        if self.ended_at.is_some() {
            return self.seconds;
        }
        (agora - self.started_at).whole_seconds().max(0)
    }
}

// ===========================================================================
// Media e desempenho
// ===========================================================================

/// Uma nota que ja existe, normalizada para a mesma escala.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nota {
    pub titulo: String,
    /// A nota como a pessoa a escreveu.
    pub score: f64,
    pub max_score: f64,
    pub weight: f64,
    /// `score / max_score`, entre 0 e 1. E o que permite somar uma prova de 0 a
    /// 10 com um trabalho de 0 a 100 sem que o trabalho domine por ser maior.
    pub fracao: f64,
}

/// Como a disciplina esta, a partir do que ja foi corrigido.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Desempenho {
    /// A media na escala de 0 a 10, ou `None` quando nada foi corrigido ainda.
    ///
    /// Dez, e nao percentual, porque e a escala em que a pessoa pensa a propria
    /// nota — e a escala em que ela ouve o professor falar.
    pub media: Option<f64>,
    /// Quanto do peso total ja foi avaliado, de 0 a 1. `None` quando nenhuma
    /// avaliacao tem peso.
    pub peso_avaliado: Option<f64>,
    pub notas: Vec<Nota>,
}

/// A media da disciplina, ponderada pelos pesos que existirem.
///
/// # As tres decisoes
///
/// **So entra o que tem nota E teto.** Uma prova marcada sem nota nao e zero:
/// e uma prova que ainda nao aconteceu, e trata-la como zero faria a media
/// desabar no comeco do semestre e subir sozinha depois — exatamente o
/// contrario do que a pessoa precisa ver.
///
/// **Peso zero vira peso um.** Quem nao configurou peso nenhum ainda merece uma
/// media, e a media justa nesse caso e a aritmetica. Sem isto, a divisao por
/// peso total zero devolveria `NaN` e a tela mostraria "NaN" como nota.
///
/// **A escala e a fracao, e nao o valor cru.** Um trabalho de 0 a 100 com 80 e
/// uma prova de 0 a 10 com 8 valem a mesma coisa, e somar 80 com 8 diria que o
/// trabalho pesa dez vezes mais.
pub fn desempenho(assignments: &[Assignment], exams: &[Exam]) -> Desempenho {
    let mut notas = Vec::new();

    for exam in exams {
        if exam.lifecycle_state != LifecycleState::Active || exam.status == ExamStatus::Cancelled {
            continue;
        }
        if let (Some(score), Some(max)) = (exam.score, exam.max_score) {
            if max > 0.0 {
                notas.push(Nota {
                    titulo: exam.name.clone(),
                    score,
                    max_score: max,
                    weight: exam.weight,
                    fracao: (score / max).clamp(0.0, 1.0),
                });
            }
        }
    }

    for assignment in assignments {
        if assignment.lifecycle_state != LifecycleState::Active
            || assignment.status == AssignmentStatus::Cancelled
        {
            continue;
        }
        if let (Some(score), Some(max)) = (assignment.score, assignment.max_score) {
            if max > 0.0 {
                notas.push(Nota {
                    titulo: assignment.title.clone(),
                    score,
                    max_score: max,
                    weight: assignment.weight,
                    fracao: (score / max).clamp(0.0, 1.0),
                });
            }
        }
    }

    if notas.is_empty() {
        return Desempenho::default();
    }

    // Peso zero em TODAS: a media vira aritmetica. Peso zero em algumas, com
    // outras pesadas: as sem peso ficam de fora da media ponderada, porque
    // "peso zero" ali significa literalmente "nao conta" — e a lista de
    // exercicios que nao vale nota e o caso comum disso.
    let algum_peso = notas.iter().any(|nota| nota.weight > 0.0);
    let (soma, peso_total) = if algum_peso {
        notas
            .iter()
            .filter(|nota| nota.weight > 0.0)
            .fold((0.0, 0.0), |(soma, peso), nota| {
                (soma + nota.fracao * nota.weight, peso + nota.weight)
            })
    } else {
        (notas.iter().map(|nota| nota.fracao).sum(), notas.len() as f64)
    };

    let media = (peso_total > 0.0).then(|| (soma / peso_total) * 10.0);

    // Quanto do peso PLANEJADO ja foi avaliado. Sem isto, "media 9,0" com uma
    // prova de quatro nao se distingue de "media 9,0" com o semestre fechado.
    let peso_planejado: f64 = exams
        .iter()
        .filter(|exam| {
            exam.lifecycle_state == LifecycleState::Active && exam.status != ExamStatus::Cancelled
        })
        .map(|exam| exam.weight)
        .chain(
            assignments
                .iter()
                .filter(|item| {
                    item.lifecycle_state == LifecycleState::Active
                        && item.status != AssignmentStatus::Cancelled
                })
                .map(|item| item.weight),
        )
        .sum();
    let peso_avaliado = (peso_planejado > 0.0).then(|| (peso_total / peso_planejado).clamp(0.0, 1.0));

    Desempenho {
        media,
        peso_avaliado,
        notas,
    }
}

/// Quanto e preciso tirar na proxima avaliacao para fechar com `alvo`.
///
/// Existe agora porque a estrutura ja responde, e nao porque a tela pede: e a
/// pergunta que o §9 do pedido antecipa. Devolve a fracao necessaria (0 a 1) na
/// avaliacao de peso `peso_restante`, ou `None` quando nao ha peso restante —
/// sem peso a fazer, nao ha o que perguntar.
///
/// Pode devolver mais de 1: e uma resposta legitima, e significa "nao da mais".
pub fn nota_necessaria(
    notas: &[Nota],
    peso_restante: f64,
    alvo_em_dez: f64,
) -> Option<f64> {
    if peso_restante <= 0.0 {
        return None;
    }
    let (soma, peso_feito) = notas
        .iter()
        .filter(|nota| nota.weight > 0.0)
        .fold((0.0, 0.0), |(soma, peso), nota| {
            (soma + nota.fracao * nota.weight, peso + nota.weight)
        });
    let alvo = (alvo_em_dez / 10.0).clamp(0.0, 1.0);
    Some(((alvo * (peso_feito + peso_restante)) - soma) / peso_restante)
}

// ===========================================================================
// O tempo: agora, logo, depois
// ===========================================================================

/// Onde um prazo cai em relacao a hoje.
///
/// A ordem das variantes E a ordem de urgencia, e o `derive(Ord)` depende dela:
/// atrasado primeiro, depois hoje, e assim por diante.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Horizonte {
    Overdue,
    Today,
    Tomorrow,
    ThisWeek,
    Later,
}

impl Horizonte {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overdue => "overdue",
            Self::Today => "today",
            Self::Tomorrow => "tomorrow",
            Self::ThisWeek => "this_week",
            Self::Later => "later",
        }
    }
}

/// Em que horizonte um instante cai, do ponto de vista de quem esta olhando.
///
/// `agora_local` ja vem no offset da tela: o corte entre hoje e amanha e data
/// civil, e decidi-lo em UTC jogaria toda entrega da madrugada para o dia
/// seguinte. E a mesma divisao do `calendar.rs`.
///
/// "Esta semana" sao os proximos SETE dias, e nao "ate domingo": um prazo na
/// terca que cai no sabado seguinte importa tanto quanto um que cai na sexta, e
/// a semana civil o empurraria para "depois" so por atravessar o domingo.
pub fn horizonte_de(quando: OffsetDateTime, agora_local: OffsetDateTime) -> Horizonte {
    let quando = quando.to_offset(agora_local.offset());
    let hoje = Day::from_local(agora_local);
    let dia = Day::from_local(quando);

    if quando < agora_local && dia < hoje {
        return Horizonte::Overdue;
    }
    if dia == hoje {
        // Ja passou da hora, no dia de hoje: e atraso, e nao "hoje". Uma entrega
        // das 10h vista as 15h nao pode aparecer como se ainda desse tempo.
        return if quando < agora_local {
            Horizonte::Overdue
        } else {
            Horizonte::Today
        };
    }
    if dia == Day::from_local(agora_local + Duration::days(1)) {
        return Horizonte::Tomorrow;
    }
    if quando <= agora_local + Duration::days(7) {
        return Horizonte::ThisWeek;
    }
    Horizonte::Later
}

// ===========================================================================
// A composicao: o que a tela recebe
// ===========================================================================

/// Um compromisso academico com data, pronto para a tela.
///
/// Um tipo so para atividade e prova porque a pergunta "o que esta chegando"
/// nao distingue as duas — ela ordena por data. `kind` fica para o icone e para
/// o clique saber onde ir.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Compromisso {
    /// `assignment` ou `exam`.
    pub kind: String,
    pub id: String,
    pub title: String,
    pub subject_id: String,
    pub subject: String,
    pub subject_accent: String,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub horizonte: Horizonte,
    /// A Task que executa, quando ha. So atividade tem.
    pub task_id: Option<String>,
    /// Local da prova, ou vazio.
    pub location: String,
    /// O que a PESSOA resolveu. Nunca vem do provedor externo.
    pub decision: crate::academic_decision::Decision,
    /// Quando ela pretende fazer. Diferente de `at`, que e quando o prazo fecha.
    #[serde(with = "time::serde::rfc3339::option")]
    pub planned_at: Option<OffsetDateTime>,
    /// Minutos reservados. Zero significa sem duracao definida.
    pub planned_minutes: i64,
}

/// Como uma disciplina esta, resumida para a lista.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectOverview {
    pub id: String,
    pub name: String,
    pub code: String,
    pub accent: String,
    /// Quantas atividades ainda pedem acao.
    pub pending: usize,
    /// Quantas ja passaram do prazo sem entrega.
    pub overdue: usize,
    /// Quantas avaliacoes ainda vao acontecer.
    pub upcoming_exams: usize,
    pub media: Option<f64>,
    pub peso_avaliado: Option<f64>,
    /// Segundos estudados nos ultimos sete dias.
    pub study_seconds_week: i64,
    /// O proximo compromisso desta disciplina, se houver.
    pub next: Option<Compromisso>,
    pub materials: usize,
}

/// O painel do M/Academic.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicDashboard {
    pub semester: Option<Semester>,
    pub semester_status: Option<SemesterStatus>,
    /// Quanto do periodo ja passou, de 0 a 1. `None` fora de um semestre ativo.
    pub semester_progress: Option<f64>,
    pub subjects: Vec<SubjectOverview>,
    /// Tudo que tem data, do mais urgente ao menos, ja com o horizonte.
    pub upcoming: Vec<Compromisso>,
    pub overdue: usize,
    pub due_today: usize,
    pub study_seconds_today: i64,
    pub study_seconds_week: i64,
    /// A sessao de estudo em curso, se houver.
    pub running: Option<StudySession>,

    // --- As faixas operacionais.
    //
    // Derivadas de `upcoming` por `academic_decision::faixa_de`, e nao gravadas:
    // "precisa de atencao" muda sozinho a cada madrugada. Cada compromisso cai
    // em UMA faixa — aparecer em duas faria a pessoa decidir duas vezes sobre a
    // mesma coisa.
    /// O que pede decisao agora.
    pub needs_attention: Vec<Compromisso>,
    /// O que tem data nos proximos sete dias e ja esta encaminhado.
    pub this_week: Vec<Compromisso>,
    /// O que vem depois.
    pub later: Vec<Compromisso>,
    /// Resolvido, descartado ou resto de calendario antigo. Limitado: historico
    /// nao compete com urgencia.
    pub history: Vec<Compromisso>,
    /// Quantos itens ainda nao tem plano nem decisao. E a pergunta "o que falta
    /// eu decidir?" respondida com um numero.
    pub undecided: usize,
}

/// O que [`compose_dashboard`] precisa.
pub struct DashboardInput<'a> {
    /// Ja no offset de quem esta olhando.
    pub now_local: OffsetDateTime,
    pub semesters: &'a [Semester],
    pub subjects: &'a [Subject],
    pub assignments: &'a [Assignment],
    pub exams: &'a [Exam],
    pub sessions: &'a [StudySession],
    /// Quantos materiais cada disciplina tem. Vem contado do banco: contar aqui
    /// exigiria carregar todos os Resources para descartar quase todos.
    pub materials: &'a dyn Fn(SubjectId) -> usize,
}

/// Quantos compromissos a tela recebe. O painel mostra os proximos, e uma lista
/// sem teto vira o proprio semestre inteiro despejado na primeira viewport.
pub const MAX_UPCOMING: usize = 12;

/// Quantos itens resolvidos o painel carrega. O historico completo tem tela
/// propria; aqui ele e o rastro recente, e uma lista sem teto faria a pagina
/// crescer com o semestre.
pub const MAX_HISTORICO: usize = 20;

/// Monta o painel a partir do que ja foi lido.
pub fn compose_dashboard(input: DashboardInput<'_>) -> AcademicDashboard {
    let hoje = Day::from_local(input.now_local);
    let semester = semestre_corrente(input.semesters, &hoje).cloned();

    let Some(semester) = semester else {
        // Sem semestre nao ha o que compor, e devolver disciplinas soltas de
        // semestre nenhum seria mostrar um estado que a tela nao sabe explicar.
        return AcademicDashboard {
            study_seconds_today: segundos_no_dia(input.sessions, input.now_local, &hoje),
            study_seconds_week: segundos_na_semana(input.sessions, input.now_local),
            running: input.sessions.iter().find(|s| s.em_curso()).cloned(),
            ..AcademicDashboard::default()
        };
    };

    let status = semester.status_em(&hoje);
    let do_semestre: Vec<&Subject> = input
        .subjects
        .iter()
        .filter(|subject| {
            subject.semester_id == semester.id
                && subject.lifecycle_state == LifecycleState::Active
        })
        .collect();

    // O contexto das faixas nasce ANTES do laco das disciplinas: o card de cada
    // materia e a faixa de atencao tem de contar a mesma coisa. Sem isto o card
    // dizia "3 atrasadas" enquanto a faixa dizia "0 compromissos" — e as duas
    // frases estavam na mesma tela, uma acima da outra.
    let contexto = crate::academic_decision::ContextoDaFaixa {
        agora_local: input.now_local,
        semestre_encerrado: status == SemesterStatus::Completed,
        semestre_comecou_em: Some(semester.starts_on.inicio_do_dia(input.now_local.offset())),
    };

    let mut upcoming = Vec::new();
    let mut overview = Vec::new();

    for subject in &do_semestre {
        let assignments: Vec<Assignment> = input
            .assignments
            .iter()
            .filter(|item| {
                item.subject_id == subject.id && item.lifecycle_state == LifecycleState::Active
            })
            .cloned()
            .collect();
        let exams: Vec<Exam> = input
            .exams
            .iter()
            .filter(|item| {
                item.subject_id == subject.id && item.lifecycle_state == LifecycleState::Active
            })
            .cloned()
            .collect();

        let mut meus = Vec::new();
        for item in &assignments {
            if item.status.is_settled() {
                continue;
            }
            let Some(due) = item.due_at else { continue };
            meus.push(Compromisso {
                kind: "assignment".to_owned(),
                id: item.id.to_string(),
                title: item.title.clone(),
                subject_id: subject.id.to_string(),
                subject: subject.name.clone(),
                subject_accent: subject.accent.clone(),
                at: due,
                horizonte: horizonte_de(due, input.now_local),
                task_id: item.task_id.map(|id| id.to_string()),
                location: String::new(),
                decision: item.decision,
                planned_at: item.planned_at,
                planned_minutes: item.planned_minutes,
            });
        }
        for item in &exams {
            if item.status.is_settled() {
                continue;
            }
            meus.push(Compromisso {
                kind: "exam".to_owned(),
                id: item.id.to_string(),
                title: item.name.clone(),
                subject_id: subject.id.to_string(),
                subject: subject.name.clone(),
                subject_accent: subject.accent.clone(),
                at: item.at,
                horizonte: horizonte_de(item.at, input.now_local),
                task_id: None,
                location: item.location.clone(),
                decision: item.decision,
                planned_at: item.planned_at,
                planned_minutes: item.planned_minutes,
            });
        }
        meus.sort_by_key(|item| item.at);

        let desempenho = desempenho(&assignments, &exams);
        // Os contadores do card seguem a MESMA regra das faixas: o que a pessoa
        // ja resolveu, ou o que e resto de calendario antigo, nao e pendencia.
        let pendentes = assignments
            .iter()
            .filter(|item| !item.status.is_settled() && !item.decision.is_settled())
            .count();
        let atrasadas = meus
            .iter()
            .filter(|item| item.kind == "assignment")
            .filter(|item| {
                crate::academic_decision::faixa_de(item, contexto)
                    == crate::academic_decision::Faixa::Atencao
                    && item.horizonte == Horizonte::Overdue
            })
            .count();
        let provas = exams
            .iter()
            .filter(|item| !item.status.is_settled() && !item.decision.is_settled())
            .count();

        overview.push(SubjectOverview {
            id: subject.id.to_string(),
            name: subject.name.clone(),
            code: subject.code.clone(),
            accent: subject.accent.clone(),
            pending: pendentes,
            overdue: atrasadas,
            upcoming_exams: provas,
            media: desempenho.media,
            peso_avaliado: desempenho.peso_avaliado,
            study_seconds_week: segundos_na_semana(
                &input
                    .sessions
                    .iter()
                    .filter(|session| session.subject_id == subject.id)
                    .cloned()
                    .collect::<Vec<_>>(),
                input.now_local,
            ),
            // O PROXIMO que ainda importa, e nao o mais antigo da lista. Sem
            // este filtro o card destacava "venceu ha 151 dias" — um resto de
            // calendario que a propria faixa ja tinha mandado para o historico.
            next: meus
                .iter()
                .find(|item| {
                    crate::academic_decision::faixa_de(item, contexto)
                        != crate::academic_decision::Faixa::Historico
                })
                .cloned(),
            materials: (input.materials)(subject.id),
        });

        upcoming.extend(meus);
    }

    // A ordem e a urgencia, e a urgencia e a data. O atraso vem antes por ser a
    // data mais antiga — nao precisa de regra propria.
    upcoming.sort_by_key(|item| item.at);

    // As faixas saem da lista COMPLETA, e nao da truncada: `upcoming` e o que a
    // faixa "o que vem" mostra, e cortar antes de classificar esconderia um
    // atraso porque havia doze provas na frente.
    let mut needs_attention = Vec::new();
    let mut this_week = Vec::new();
    let mut later = Vec::new();
    let mut history = Vec::new();
    for item in &upcoming {
        match crate::academic_decision::faixa_de(item, contexto) {
            crate::academic_decision::Faixa::Atencao => needs_attention.push(item.clone()),
            crate::academic_decision::Faixa::Semana => this_week.push(item.clone()),
            crate::academic_decision::Faixa::Depois => later.push(item.clone()),
            crate::academic_decision::Faixa::Historico => history.push(item.clone()),
        }
    }
    // O historico e o unico que sai da ordem cronologica crescente: o que foi
    // resolvido por ultimo interessa mais que o que foi resolvido em marco.
    history.reverse();
    history.truncate(MAX_HISTORICO);
    let undecided = needs_attention
        .iter()
        .chain(this_week.iter())
        .filter(|item| !crate::academic_decision::esta_planejado(item))
        .count();

    // `overdue` e `due_today` contam o que PEDE acao, e nao o que tem data
    // vencida: um trabalho marcado como entregue nao e uma pendencia, e um
    // resto de calendario antigo tambem nao. Sem isto o widget da Home diria
    // "4 atrasados" apontando para uma tela que nao mostra nenhum.
    let overdue = needs_attention
        .iter()
        .filter(|item| item.horizonte == Horizonte::Overdue)
        .count();
    let due_today = needs_attention
        .iter()
        .filter(|item| item.horizonte == Horizonte::Today)
        .count();
    upcoming.truncate(MAX_UPCOMING);

    // Disciplina com problema primeiro: atraso, depois prova chegando, e o resto
    // em ordem alfabetica para a lista nao dancar entre dois refreshes.
    overview.sort_by(|a, b| {
        b.overdue
            .cmp(&a.overdue)
            .then_with(|| b.upcoming_exams.cmp(&a.upcoming_exams))
            .then_with(|| a.name.cmp(&b.name))
    });

    AcademicDashboard {
        semester_progress: progresso_do_periodo(&semester, &hoje),
        semester: Some(semester),
        semester_status: Some(status),
        subjects: overview,
        upcoming,
        overdue,
        due_today,
        study_seconds_today: segundos_no_dia(input.sessions, input.now_local, &hoje),
        study_seconds_week: segundos_na_semana(input.sessions, input.now_local),
        running: input.sessions.iter().find(|s| s.em_curso()).cloned(),
        needs_attention,
        this_week,
        later,
        history,
        undecided,
    }
}

/// Quanto do periodo ja passou, de 0 a 1.
///
/// E a unica medida de progresso que este modulo produz, e ela e honesta: conta
/// dias, que e um dado que existe. "Progresso da disciplina" em porcentagem
/// exigiria saber quantas atividades o semestre TERA, e ninguem sabe isso em
/// marco — o numero seria inventado, e o §27 do pedido pede exatamente que nao
/// se invente precisao.
pub fn progresso_do_periodo(semester: &Semester, hoje: &Day) -> Option<f64> {
    let inicio = semester.starts_on.date().ok()?;
    let fim = semester.ends_on.date().ok()?;
    let agora = hoje.date().ok()?;
    let total = (fim - inicio).whole_days();
    if total <= 0 {
        return None;
    }
    let passados = (agora - inicio).whole_days();
    Some((passados as f64 / total as f64).clamp(0.0, 1.0))
}

/// Segundos estudados num dia civil.
pub fn segundos_no_dia(sessions: &[StudySession], agora_local: OffsetDateTime, dia: &Day) -> i64 {
    let offset = agora_local.offset();
    sessions
        .iter()
        .filter(|session| &Day::from_local(session.started_at.to_offset(offset)) == dia)
        .map(|session| session.segundos_em(agora_local))
        .sum()
}

/// Segundos estudados nos ultimos sete dias, contando o de hoje.
pub fn segundos_na_semana(sessions: &[StudySession], agora_local: OffsetDateTime) -> i64 {
    let inicio = agora_local - Duration::days(7);
    sessions
        .iter()
        .filter(|session| session.started_at >= inicio)
        .map(|session| session.segundos_em(agora_local))
        .sum()
}

/// Todos os compromissos academicos com data numa janela, para o Calendario.
///
/// # Por que nao reusa o `upcoming` do painel
///
/// O painel responde "o que esta chegando": ele descarta o que ja foi entregue e
/// corta em [`MAX_UPCOMING`]. O Calendario e retrospectivo — uma prova FEITA na
/// semana passada e exatamente o material dele, e um teto de doze esconderia
/// metade do mes. As duas perguntas sao diferentes, e por isso sao duas funcoes.
///
/// O que as duas compartilham e o formato: o mesmo [`Compromisso`], para o
/// Calendario nao precisar conhecer `Exam` nem `Assignment`.
pub fn compose_compromissos(
    subjects: &[Subject],
    assignments: &[Assignment],
    exams: &[Exam],
    since: OffsetDateTime,
    until: OffsetDateTime,
    agora_local: OffsetDateTime,
) -> Vec<Compromisso> {
    let mut itens = Vec::new();
    let dentro = |quando: OffsetDateTime| quando >= since && quando <= until;

    for subject in subjects {
        if subject.lifecycle_state != LifecycleState::Active {
            continue;
        }
        for item in assignments {
            if item.subject_id != subject.id
                || item.lifecycle_state != LifecycleState::Active
                // Cancelada nao aconteceu, e nao vai acontecer: ela e a unica
                // que fica de fora dos dois lados.
                || item.status == AssignmentStatus::Cancelled
            {
                continue;
            }
            let Some(due) = item.due_at else { continue };
            if !dentro(due) {
                continue;
            }
            itens.push(Compromisso {
                kind: "assignment".to_owned(),
                id: item.id.to_string(),
                title: item.title.clone(),
                subject_id: subject.id.to_string(),
                subject: subject.name.clone(),
                subject_accent: subject.accent.clone(),
                at: due,
                horizonte: horizonte_de(due, agora_local),
                task_id: item.task_id.map(|id| id.to_string()),
                location: String::new(),
                decision: item.decision,
                planned_at: item.planned_at,
                planned_minutes: item.planned_minutes,
            });
        }
        for item in exams {
            if item.subject_id != subject.id
                || item.lifecycle_state != LifecycleState::Active
                || item.status == ExamStatus::Cancelled
                || !dentro(item.at)
            {
                continue;
            }
            itens.push(Compromisso {
                kind: "exam".to_owned(),
                id: item.id.to_string(),
                title: item.name.clone(),
                subject_id: subject.id.to_string(),
                subject: subject.name.clone(),
                subject_accent: subject.accent.clone(),
                at: item.at,
                horizonte: horizonte_de(item.at, agora_local),
                task_id: None,
                location: item.location.clone(),
                decision: item.decision,
                planned_at: item.planned_at,
                planned_minutes: item.planned_minutes,
            });
        }
    }

    itens.sort_by_key(|item| item.at);
    itens
}

/// O que a faculdade coloca no dia de hoje.
///
/// Vai para o Start My Day e para o End My Day. Nao e uma tela: e o contexto que
/// as duas cerimonias do dia recebem, junto com o que ja recebiam de Task,
/// Reminder e Capture.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcademicToday {
    /// O que vence hoje.
    pub due_today: Vec<Compromisso>,
    /// O que passou do prazo e continua aberto.
    pub overdue: Vec<Compromisso>,
    /// As proximas provas, ate sete dias.
    pub exams_soon: Vec<Compromisso>,
    /// Disciplinas com prova chegando e pouco estudo na semana — o que sugerir
    /// estudar hoje.
    pub study_suggestions: Vec<StudySuggestion>,
    pub study_seconds_today: i64,
    /// O que a pessoa **decidiu fazer hoje**.
    ///
    /// Nao e o que vence hoje: e o bloco que ela reservou. Um trabalho que vence
    /// sexta e foi planejado para hoje entra aqui, e nao em `due_today` — e essa
    /// e a diferenca entre o Start My Day mostrar prazos e mostrar acoes.
    pub planned_today: Vec<Compromisso>,
    /// O que foi resolvido hoje. Alimenta o End My Day.
    pub decided_today: Vec<Compromisso>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySuggestion {
    pub subject_id: String,
    pub subject: String,
    /// O que torna esta disciplina a sugestao de hoje, em uma frase.
    pub reason: String,
    /// Dias ate a proxima prova, quando ha uma.
    pub days_to_exam: Option<i64>,
}

/// O recorte de hoje, a partir do painel ja composto.
///
/// Recebe o dashboard em vez de recompor: as duas telas fazem a mesma leitura,
/// e compor duas vezes abriria espaco para elas discordarem sobre o que e
/// "hoje".
pub fn compose_today(dashboard: &AcademicDashboard, agora_local: OffsetDateTime) -> AcademicToday {
    // As tres primeiras listas saem de `needs_attention`, e nao de `upcoming`:
    // o que a pessoa ja marcou como entregue ou descartou nao pode voltar a
    // cobrar na cerimonia da manha. Era o que acontecia quando a leitura era
    // por horizonte puro — o horizonte so sabe de data, e nao de decisao.
    let due_today: Vec<Compromisso> = dashboard
        .needs_attention
        .iter()
        .filter(|item| item.horizonte == Horizonte::Today)
        .cloned()
        .collect();
    let overdue: Vec<Compromisso> = dashboard
        .needs_attention
        .iter()
        .filter(|item| item.horizonte == Horizonte::Overdue)
        .cloned()
        .collect();
    let hoje_civil = Day::from_local(agora_local);
    // O que eu decidi fazer hoje, venca quando vencer.
    let planned_today: Vec<Compromisso> = dashboard
        .needs_attention
        .iter()
        .chain(dashboard.this_week.iter())
        .chain(dashboard.later.iter())
        .filter(|item| {
            item.planned_at
                .map(|quando| Day::from_local(quando.to_offset(agora_local.offset())) == hoje_civil)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    // O que foi resolvido hoje. O End My Day conta o dia, e nao o semestre.
    let decided_today: Vec<Compromisso> = dashboard
        .history
        .iter()
        .filter(|item| item.decision.is_settled())
        .filter(|item| {
            item.planned_at
                .map(|quando| Day::from_local(quando.to_offset(agora_local.offset())) == hoje_civil)
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    let exams_soon: Vec<Compromisso> = dashboard
        .upcoming
        .iter()
        .filter(|item| {
            item.kind == "exam"
                && matches!(
                    item.horizonte,
                    Horizonte::Today | Horizonte::Tomorrow | Horizonte::ThisWeek
                )
        })
        .cloned()
        .collect();

    // A sugestao de estudo sai da prova mais proxima de cada disciplina, e nao
    // de um ranking de "quem estudou menos": estudar por deficit trataria como
    // urgente uma disciplina sem nada marcado, e a prova de quinta e que decide
    // o que fazer hoje.
    let mut sugestoes: Vec<StudySuggestion> = Vec::new();
    for subject in &dashboard.subjects {
        let Some(next) = &subject.next else { continue };
        if next.kind != "exam" {
            continue;
        }
        let dias = (next.at - agora_local).whole_days();
        if !(0..=14).contains(&dias) {
            continue;
        }
        let reason = if subject.study_seconds_week == 0 {
            format!("prova em {dias} dias, e nada estudado nesta semana")
        } else {
            format!("prova em {dias} dias")
        };
        sugestoes.push(StudySuggestion {
            subject_id: subject.id.clone(),
            subject: subject.name.clone(),
            reason,
            days_to_exam: Some(dias),
        });
    }
    sugestoes.sort_by_key(|item| item.days_to_exam.unwrap_or(i64::MAX));
    sugestoes.truncate(3);

    AcademicToday {
        due_today,
        overdue,
        exams_soon,
        study_suggestions: sugestoes,
        study_seconds_today: dashboard.study_seconds_today,
        planned_today,
        decided_today,
    }
}

// ===========================================================================
// O que entra
// ===========================================================================
//
// Os tipos `New*` validam na construcao, como `NewTask` e `NewCapture`: um
// titulo em branco ou um intervalo invertido morrem aqui, e nao num CHECK do
// SQLite que devolve mensagem de banco para a tela.

fn required(value: &str, message: &str) -> Result<String, CoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::new(ErrorCode::InvalidInput, message, false));
    }
    Ok(trimmed.to_owned())
}

#[derive(Clone, Debug)]
pub struct NewSemester {
    pub id: SemesterId,
    pub name: String,
    pub institution: String,
    pub starts_on: Day,
    pub ends_on: Day,
    pub created_at: OffsetDateTime,
}

impl NewSemester {
    pub fn create(
        name: &str,
        institution: &str,
        starts_on: &str,
        ends_on: &str,
    ) -> Result<Self, CoreError> {
        let starts_on = Day::parse(starts_on)?;
        let ends_on = Day::parse(ends_on)?;
        if ends_on < starts_on {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O semestre termina antes de comecar.",
                false,
            ));
        }
        Ok(Self {
            id: SemesterId::new(),
            name: required(name, "O nome do semestre nao pode estar vazio.")?,
            institution: institution.trim().to_owned(),
            starts_on,
            ends_on,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct NewSubject {
    pub id: SubjectId,
    pub semester_id: SemesterId,
    pub name: String,
    pub code: String,
    pub teacher: String,
    pub accent: String,
    pub notes: String,
    pub created_at: OffsetDateTime,
}

impl NewSubject {
    pub fn create(
        semester_id: SemesterId,
        name: &str,
        code: &str,
        teacher: &str,
        accent: &str,
        notes: &str,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            id: SubjectId::new(),
            semester_id,
            name: required(name, "O nome da disciplina nao pode estar vazio.")?,
            code: code.trim().to_owned(),
            teacher: teacher.trim().to_owned(),
            accent: validate_accent(accent)?,
            notes: notes.trim().to_owned(),
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

/// A nota de uma avaliacao: o par valor/teto, ou nada.
///
/// Tipo proprio em vez de dois `Option<f64>` soltos porque **os dois andam
/// juntos**: nota sem teto nao se converte em media (8 de quanto?), e o CHECK do
/// banco recusa esse par pela metade. Um tipo que so representa o par completo
/// impede o estado invalido de existir antes de chegar la.
#[derive(Clone, Copy, Debug)]
pub struct Pontuacao {
    pub score: f64,
    pub max_score: f64,
}

impl Pontuacao {
    pub fn nova(score: Option<f64>, max_score: Option<f64>) -> Result<Option<Self>, CoreError> {
        match (score, max_score) {
            (None, None) => Ok(None),
            (Some(score), Some(max_score)) => {
                if max_score <= 0.0 {
                    return Err(CoreError::new(
                        ErrorCode::InvalidInput,
                        "A nota maxima precisa ser maior que zero.",
                        false,
                    ));
                }
                if score < 0.0 {
                    return Err(CoreError::new(
                        ErrorCode::InvalidInput,
                        "A nota nao pode ser negativa.",
                        false,
                    ));
                }
                Ok(Some(Self { score, max_score }))
            }
            // Teto sem nota e um estado legitimo em EDICAO — "esta prova vale
            // 10, ainda nao fiz" —, e por isso ele passa como ausencia de nota
            // com teto guardado. Nota sem teto e que nao existe.
            (None, Some(max_score)) => {
                if max_score <= 0.0 {
                    return Err(CoreError::new(
                        ErrorCode::InvalidInput,
                        "A nota maxima precisa ser maior que zero.",
                        false,
                    ));
                }
                Ok(Some(Self {
                    score: f64::NAN,
                    max_score,
                }))
            }
            (Some(_), None) => Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Uma nota precisa de uma nota maxima: 8 de quanto?",
                false,
            )),
        }
    }

    /// O par pronto para o banco. `score` sai vazio quando so ha teto.
    pub fn colunas(self) -> (Option<f64>, Option<f64>) {
        (
            (!self.score.is_nan()).then_some(self.score),
            Some(self.max_score),
        )
    }
}

#[derive(Clone, Debug)]
pub struct NewAssignment {
    pub id: AssignmentId,
    pub subject_id: SubjectId,
    pub title: String,
    pub description: String,
    pub due_at: Option<OffsetDateTime>,
    pub priority: crate::Priority,
    pub weight: f64,
    pub pontuacao: Option<Pontuacao>,
    pub created_at: OffsetDateTime,
}

impl NewAssignment {
    pub fn create(
        subject_id: SubjectId,
        title: &str,
        description: &str,
        due_at: Option<OffsetDateTime>,
        priority: crate::Priority,
        weight: f64,
        score: Option<f64>,
        max_score: Option<f64>,
    ) -> Result<Self, CoreError> {
        if weight < 0.0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O peso nao pode ser negativo.",
                false,
            ));
        }
        Ok(Self {
            id: AssignmentId::new(),
            subject_id,
            title: required(title, "O titulo da atividade nao pode estar vazio.")?,
            description: description.trim().to_owned(),
            due_at,
            priority,
            weight,
            pontuacao: Pontuacao::nova(score, max_score)?,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct NewExam {
    pub id: ExamId,
    pub subject_id: SubjectId,
    pub name: String,
    pub at: OffsetDateTime,
    pub location: String,
    pub topics: String,
    pub weight: f64,
    pub pontuacao: Option<Pontuacao>,
    pub created_at: OffsetDateTime,
}

impl NewExam {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        subject_id: SubjectId,
        name: &str,
        at: OffsetDateTime,
        location: &str,
        topics: &str,
        weight: f64,
        score: Option<f64>,
        max_score: Option<f64>,
    ) -> Result<Self, CoreError> {
        if weight < 0.0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O peso nao pode ser negativo.",
                false,
            ));
        }
        Ok(Self {
            id: ExamId::new(),
            subject_id,
            name: required(name, "O nome da avaliacao nao pode estar vazio.")?,
            at,
            location: location.trim().to_owned(),
            topics: topics.trim().to_owned(),
            weight,
            pontuacao: Pontuacao::nova(score, max_score)?,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use time::macros::datetime;

    fn agora() -> OffsetDateTime {
        // Meio-dia, num fuso negativo: e o horario que revela erro de fuso na
        // conta de "hoje", porque UTC ja virou tarde e o dia ainda e o mesmo.
        datetime!(2026-08-22 12:00 -03:00)
    }

    fn dia(valor: &str) -> Day {
        Day::parse(valor).unwrap()
    }

    fn semestre(nome: &str, inicio: &str, fim: &str) -> Semester {
        Semester {
            id: SemesterId::new(),
            name: nome.to_owned(),
            institution: String::new(),
            starts_on: dia(inicio),
            ends_on: dia(fim),
            lifecycle_state: LifecycleState::Active,
            created_at: agora(),
            updated_at: agora(),
        }
    }

    fn disciplina(semestre: &Semester, nome: &str) -> Subject {
        Subject {
            id: SubjectId::new(),
            semester_id: semestre.id,
            name: nome.to_owned(),
            code: String::new(),
            teacher: String::new(),
            accent: String::new(),
            notes: String::new(),
            lifecycle_state: LifecycleState::Active,
            created_at: agora(),
            updated_at: agora(),
        }
    }

    fn prova(subject: &Subject, nome: &str, quando: OffsetDateTime) -> Exam {
        Exam {
            id: ExamId::new(),
            subject_id: subject.id,
            name: nome.to_owned(),
            at: quando,
            location: String::new(),
            topics: String::new(),
            weight: 0.0,
            max_score: None,
            score: None,
            status: ExamStatus::Scheduled,
            decision: crate::academic_decision::Decision::None,
            decided_at: None,
            planned_at: None,
            planned_minutes: 0,
            lifecycle_state: LifecycleState::Active,
            created_at: agora(),
            updated_at: agora(),
        }
    }

    fn atividade(subject: &Subject, titulo: &str, prazo: Option<OffsetDateTime>) -> Assignment {
        Assignment {
            id: AssignmentId::new(),
            subject_id: subject.id,
            title: titulo.to_owned(),
            description: String::new(),
            due_at: prazo,
            status: AssignmentStatus::Pending,
            priority: crate::Priority::Normal,
            weight: 0.0,
            max_score: None,
            score: None,
            task_id: None,
            decision: crate::academic_decision::Decision::None,
            decided_at: None,
            planned_at: None,
            planned_minutes: 0,
            lifecycle_state: LifecycleState::Active,
            created_at: agora(),
            updated_at: agora(),
        }
    }

    fn sessao(subject: &Subject, inicio: OffsetDateTime, segundos: i64) -> StudySession {
        StudySession {
            id: StudySessionId::new(),
            subject_id: subject.id,
            topic: String::new(),
            notes: String::new(),
            started_at: inicio,
            ended_at: Some(inicio + Duration::seconds(segundos)),
            seconds: segundos,
            created_at: inicio,
            updated_at: inicio,
        }
    }

    fn entrada<'a>(
        semesters: &'a [Semester],
        subjects: &'a [Subject],
        assignments: &'a [Assignment],
        exams: &'a [Exam],
        sessions: &'a [StudySession],
    ) -> DashboardInput<'a> {
        DashboardInput {
            now_local: agora(),
            semesters,
            subjects,
            assignments,
            exams,
            sessions,
            materials: &|_| 0,
        }
    }

    // =======================================================================
    // Semestre
    // =======================================================================

    #[test]
    fn o_status_do_semestre_vem_das_datas_e_nao_de_um_campo() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        assert_eq!(atual.status_em(&dia("2026-08-22")), SemesterStatus::Active);
        assert_eq!(atual.status_em(&dia("2026-07-31")), SemesterStatus::Upcoming);
        assert_eq!(atual.status_em(&dia("2026-12-16")), SemesterStatus::Completed);
    }

    /// A borda pertence ao semestre. O primeiro e o ultimo dia de aula sao dias
    /// DE aula, e um sistema que os deixa de fora mostra o semestre vazio
    /// justamente no dia da matricula e no dia da ultima prova.
    #[test]
    fn o_primeiro_e_o_ultimo_dia_pertencem_ao_semestre() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        assert_eq!(atual.status_em(&dia("2026-08-01")), SemesterStatus::Active);
        assert_eq!(atual.status_em(&dia("2026-12-15")), SemesterStatus::Active);
    }

    #[test]
    fn o_corrente_e_o_que_esta_acontecendo() {
        let passado = semestre("2026.1", "2026-02-01", "2026-06-30");
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let futuro = semestre("2027.1", "2027-02-01", "2027-06-30");
        let todos = vec![passado, atual.clone(), futuro];
        assert_eq!(
            semestre_corrente(&todos, &dia("2026-08-22")).unwrap().id,
            atual.id
        );
    }

    /// Ferias: nenhum semestre corrente. Abrir vazio faria a pessoa procurar o
    /// proprio historico bem no momento em que ele e a unica coisa que ela tem.
    #[test]
    fn entre_dois_semestres_o_proximo_a_comecar_ganha() {
        let passado = semestre("2026.1", "2026-02-01", "2026-06-30");
        let futuro = semestre("2026.2", "2026-08-01", "2026-12-15");
        let todos = vec![passado, futuro.clone()];
        assert_eq!(
            semestre_corrente(&todos, &dia("2026-07-15")).unwrap().id,
            futuro.id,
            "em julho, o que importa e o que vem"
        );
    }

    #[test]
    fn depois_de_tudo_o_ultimo_concluido_ganha() {
        let antigo = semestre("2025.2", "2025-08-01", "2025-12-15");
        let recente = semestre("2026.1", "2026-02-01", "2026-06-30");
        let todos = vec![antigo, recente.clone()];
        assert_eq!(
            semestre_corrente(&todos, &dia("2026-07-15")).unwrap().id,
            recente.id
        );
    }

    #[test]
    fn semestre_arquivado_nunca_e_o_corrente() {
        let mut guardado = semestre("2026.2", "2026-08-01", "2026-12-15");
        guardado.lifecycle_state = LifecycleState::Archived;
        assert!(semestre_corrente(&[guardado], &dia("2026-08-22")).is_none());
    }

    #[test]
    fn o_progresso_do_periodo_conta_dias() {
        let atual = semestre("2026.2", "2026-08-01", "2026-08-31");
        let meio = progresso_do_periodo(&atual, &dia("2026-08-16")).unwrap();
        assert!((meio - 0.5).abs() < 0.02, "metade do periodo: {meio}");
        assert_eq!(progresso_do_periodo(&atual, &dia("2026-07-01")), Some(0.0));
        assert_eq!(progresso_do_periodo(&atual, &dia("2026-12-01")), Some(1.0));
    }

    // =======================================================================
    // Horizonte
    // =======================================================================

    #[test]
    fn o_horizonte_separa_hoje_amanha_semana_e_depois() {
        let base = agora();
        assert_eq!(
            horizonte_de(base + Duration::hours(2), base),
            Horizonte::Today
        );
        assert_eq!(
            horizonte_de(base + Duration::days(1), base),
            Horizonte::Tomorrow
        );
        assert_eq!(
            horizonte_de(base + Duration::days(4), base),
            Horizonte::ThisWeek
        );
        assert_eq!(
            horizonte_de(base + Duration::days(30), base),
            Horizonte::Later
        );
    }

    /// A entrega das 10h, vista as 12h, JA passou. Mostra-la como "hoje" diria
    /// que ainda da tempo, e nao da.
    #[test]
    fn a_hora_que_ja_passou_hoje_e_atraso_e_nao_hoje() {
        let base = agora();
        assert_eq!(
            horizonte_de(base - Duration::hours(2), base),
            Horizonte::Overdue
        );
    }

    #[test]
    fn ontem_e_atraso() {
        let base = agora();
        assert_eq!(
            horizonte_de(base - Duration::days(1), base),
            Horizonte::Overdue
        );
    }

    /// O prazo chega em UTC e a tela pensa em fuso local. Uma entrega marcada
    /// para 23h do dia 22 em -03:00 e 02h do dia 23 em UTC: comparar sem
    /// converter a jogaria para "amanha".
    #[test]
    fn o_horizonte_respeita_o_fuso_de_quem_olha() {
        let local = datetime!(2026-08-22 12:00 -03:00);
        let prazo_em_utc = datetime!(2026-08-23 02:00 UTC);
        assert_eq!(horizonte_de(prazo_em_utc, local), Horizonte::Today);
    }

    // =======================================================================
    // Media
    // =======================================================================

    /// A escala e a FRACAO. Um trabalho de 0 a 100 com 80 e uma prova de 0 a 10
    /// com 8 valem o mesmo; somar 80 com 8 diria que o trabalho pesa dez vezes.
    #[test]
    fn escalas_diferentes_pesam_igual() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");

        let mut trabalho = atividade(&materia, "Trabalho", None);
        trabalho.max_score = Some(100.0);
        trabalho.score = Some(80.0);
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(8.0);

        let resultado = desempenho(&[trabalho], &[p1]);
        let media = resultado.media.unwrap();
        assert!((media - 8.0).abs() < 0.001, "media de 8,0: {media}");
    }

    /// Prova marcada sem nota NAO e zero. Trata-la como zero faria a media
    /// desabar no comeco do semestre e subir sozinha depois.
    #[test]
    fn prova_sem_nota_nao_derruba_a_media() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(9.0);
        let p2 = prova(&materia, "P2", agora() + Duration::days(30));

        let resultado = desempenho(&[], &[p1, p2]);
        assert_eq!(resultado.media, Some(9.0));
        assert_eq!(resultado.notas.len(), 1, "so a corrigida entra");
    }

    #[test]
    fn sem_nota_nenhuma_a_media_e_ausente_e_nao_zero() {
        let resultado = desempenho(&[], &[]);
        assert_eq!(resultado.media, None);
        assert_eq!(resultado.peso_avaliado, None);
    }

    /// Sem peso configurado, a media e a aritmetica. Dividir por peso total zero
    /// devolveria NaN, e a tela mostraria "NaN" como nota.
    #[test]
    fn sem_peso_a_media_e_aritmetica_e_nunca_nan() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(6.0);
        let mut p2 = prova(&materia, "P2", agora());
        p2.max_score = Some(10.0);
        p2.score = Some(8.0);

        let media = desempenho(&[], &[p1, p2]).media.unwrap();
        assert!(media.is_finite(), "media virou {media}");
        assert!((media - 7.0).abs() < 0.001, "{media}");
    }

    #[test]
    fn o_peso_manda_quando_existe() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(6.0);
        p1.weight = 3.0;
        let mut p2 = prova(&materia, "P2", agora());
        p2.max_score = Some(10.0);
        p2.score = Some(10.0);
        p2.weight = 1.0;

        // (0.6*3 + 1.0*1) / 4 = 0.7 -> 7,0
        let media = desempenho(&[], &[p1, p2]).media.unwrap();
        assert!((media - 7.0).abs() < 0.001, "{media}");
    }

    /// Lista de exercicios que nao vale nota nao pode puxar a media da prova
    /// que vale. Peso zero significa literalmente "nao conta".
    #[test]
    fn peso_zero_fica_fora_quando_ha_avaliacao_com_peso() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut lista = atividade(&materia, "Lista 01", None);
        lista.max_score = Some(10.0);
        lista.score = Some(10.0);
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(5.0);
        p1.weight = 1.0;

        let media = desempenho(&[lista], &[p1]).media.unwrap();
        assert!((media - 5.0).abs() < 0.001, "so a prova conta: {media}");
    }

    #[test]
    fn avaliacao_cancelada_nao_entra_na_media() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(2.0);
        p1.status = ExamStatus::Cancelled;

        assert_eq!(desempenho(&[], &[p1]).media, None);
    }

    #[test]
    fn o_peso_avaliado_diz_quanto_do_semestre_ja_foi_medido() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut p1 = prova(&materia, "P1", agora());
        p1.max_score = Some(10.0);
        p1.score = Some(7.0);
        p1.weight = 1.0;
        let mut p2 = prova(&materia, "P2", agora() + Duration::days(30));
        p2.weight = 3.0;

        let resultado = desempenho(&[], &[p1, p2]);
        assert_eq!(resultado.peso_avaliado, Some(0.25));
    }

    #[test]
    fn a_nota_necessaria_responde_quanto_falta() {
        let notas = vec![Nota {
            titulo: "P1".into(),
            score: 6.0,
            max_score: 10.0,
            weight: 1.0,
            fracao: 0.6,
        }];
        // Alvo 7,0 com metade do peso feito a 6,0: precisa de 8,0 na outra.
        let precisa = nota_necessaria(&notas, 1.0, 7.0).unwrap();
        assert!((precisa - 0.8).abs() < 0.001, "{precisa}");
    }

    #[test]
    fn sem_peso_restante_nao_ha_o_que_perguntar() {
        assert_eq!(nota_necessaria(&[], 0.0, 7.0), None);
    }

    // =======================================================================
    // Estudo
    // =======================================================================

    #[test]
    fn a_sessao_aberta_vale_o_tempo_que_ja_passou() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let mut aberta = sessao(&materia, agora() - Duration::minutes(30), 0);
        aberta.ended_at = None;
        assert!(aberta.em_curso());
        assert_eq!(aberta.segundos_em(agora()), 30 * 60);
    }

    #[test]
    fn a_sessao_fechada_vale_o_que_foi_gravado() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        // Gravada com 20 min, mesmo tendo comecado ha duas horas: uma sessao
        // pausada tem duracao que o relogio de parede nao reproduz.
        let mut fechada = sessao(&materia, agora() - Duration::hours(2), 20 * 60);
        fechada.ended_at = Some(agora());
        assert_eq!(fechada.segundos_em(agora()), 20 * 60);
    }

    #[test]
    fn o_estudo_de_hoje_nao_conta_o_de_ontem() {
        let semestre = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&semestre, "Estatica");
        let hoje = sessao(&materia, agora() - Duration::hours(1), 3600);
        let ontem = sessao(&materia, agora() - Duration::days(1), 3600);
        let sessions = vec![hoje, ontem];

        assert_eq!(
            segundos_no_dia(&sessions, agora(), &dia("2026-08-22")),
            3600
        );
        assert_eq!(segundos_na_semana(&sessions, agora()), 7200);
    }

    // =======================================================================
    // Painel
    // =======================================================================

    #[test]
    fn o_painel_abre_no_semestre_corrente_com_as_disciplinas_dele() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let antigo = semestre("2026.1", "2026-02-01", "2026-06-30");
        let estatica = disciplina(&atual, "Estatica");
        let velha = disciplina(&antigo, "Calculo I");
        let semesters = vec![atual.clone(), antigo];
        let subjects = vec![estatica, velha];

        let painel = compose_dashboard(entrada(&semesters, &subjects, &[], &[], &[]));
        assert_eq!(painel.semester.unwrap().id, atual.id);
        assert_eq!(painel.subjects.len(), 1, "so as do semestre corrente");
        assert_eq!(painel.subjects[0].name, "Estatica");
    }

    #[test]
    fn sem_semestre_o_painel_nao_inventa_disciplina_solta() {
        let painel = compose_dashboard(entrada(&[], &[], &[], &[], &[]));
        assert!(painel.semester.is_none());
        assert!(painel.subjects.is_empty());
        assert!(painel.upcoming.is_empty());
    }

    #[test]
    fn o_que_esta_chegando_sai_em_ordem_de_data() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let prova_longe = prova(&materia, "P2", agora() + Duration::days(20));
        let entrega = atividade(&materia, "Lista 03", Some(agora() + Duration::days(2)));
        let prova_perto = prova(&materia, "P1", agora() + Duration::days(5));

        let semesters = vec![atual];
        let subjects = vec![materia];
        let assignments = vec![entrega];
        let exams = vec![prova_longe, prova_perto];
        let painel = compose_dashboard(entrada(
            &semesters,
            &subjects,
            &assignments,
            &exams,
            &[],
        ));

        let titulos: Vec<&str> = painel.upcoming.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titulos, vec!["Lista 03", "P1", "P2"]);
    }

    #[test]
    fn atividade_entregue_sai_da_lista_do_que_esta_chegando() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let mut entregue = atividade(&materia, "Lista 01", Some(agora() + Duration::days(1)));
        entregue.status = AssignmentStatus::Submitted;

        let semesters = vec![atual];
        let subjects = vec![materia];
        let assignments = vec![entregue];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &assignments, &[], &[]));
        assert!(painel.upcoming.is_empty());
    }

    #[test]
    fn atividade_sem_prazo_nao_aparece_como_compromisso() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let solta = atividade(&materia, "Ler o capitulo 4", None);

        let semesters = vec![atual];
        let subjects = vec![materia];
        let assignments = vec![solta];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &assignments, &[], &[]));
        assert!(painel.upcoming.is_empty(), "sem data, sem lugar na linha do tempo");
        assert_eq!(painel.subjects[0].pending, 1, "mas continua pendente");
    }

    #[test]
    fn o_painel_conta_atraso_e_prazo_de_hoje() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let atrasada = atividade(&materia, "Lista 01", Some(agora() - Duration::days(2)));
        let hoje = atividade(&materia, "Lista 02", Some(agora() + Duration::hours(3)));

        let semesters = vec![atual];
        let subjects = vec![materia];
        let assignments = vec![atrasada, hoje];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &assignments, &[], &[]));

        assert_eq!(painel.overdue, 1);
        assert_eq!(painel.due_today, 1);
        assert_eq!(painel.subjects[0].overdue, 1);
    }

    /// A disciplina com problema vem primeiro. A lista alfabetica esconderia
    /// atras do "T" a disciplina com tres entregas vencidas.
    #[test]
    fn a_disciplina_com_atraso_vem_antes_na_lista() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let calma = disciplina(&atual, "Algebra");
        let problema = disciplina(&atual, "Zoologia");
        let atrasada = atividade(&problema, "Lista", Some(agora() - Duration::days(3)));

        let semesters = vec![atual];
        let subjects = vec![calma, problema];
        let assignments = vec![atrasada];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &assignments, &[], &[]));
        assert_eq!(painel.subjects[0].name, "Zoologia");
    }

    #[test]
    fn o_painel_encontra_a_sessao_em_curso() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let mut aberta = sessao(&materia, agora() - Duration::minutes(10), 0);
        aberta.ended_at = None;

        let semesters = vec![atual];
        let subjects = vec![materia];
        let sessions = vec![aberta];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &[], &[], &sessions));
        assert!(painel.running.is_some());
        assert_eq!(painel.study_seconds_today, 600);
    }

    // =======================================================================
    // Hoje
    // =======================================================================

    #[test]
    fn o_hoje_separa_o_que_vence_do_que_atrasou() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let atrasada = atividade(&materia, "Lista 01", Some(agora() - Duration::days(2)));
        let vence_hoje = atividade(&materia, "Lista 02", Some(agora() + Duration::hours(3)));
        let depois = atividade(&materia, "Lista 03", Some(agora() + Duration::days(20)));

        let semesters = vec![atual];
        let subjects = vec![materia];
        let assignments = vec![atrasada, vence_hoje, depois];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &assignments, &[], &[]));
        let hoje = compose_today(&painel, agora());

        assert_eq!(hoje.overdue.len(), 1);
        assert_eq!(hoje.due_today.len(), 1);
        assert_eq!(hoje.due_today[0].title, "Lista 02");
    }

    #[test]
    fn a_prova_da_semana_sugere_estudo_e_a_de_daqui_a_meses_nao() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let perto = disciplina(&atual, "Estatica");
        let longe = disciplina(&atual, "Historia");
        let p1 = prova(&perto, "P1", agora() + Duration::days(3));
        let distante = prova(&longe, "P1", agora() + Duration::days(60));

        let semesters = vec![atual];
        let subjects = vec![perto, longe];
        let exams = vec![p1, distante];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &[], &exams, &[]));
        let hoje = compose_today(&painel, agora());

        assert_eq!(hoje.study_suggestions.len(), 1);
        assert_eq!(hoje.study_suggestions[0].subject, "Estatica");
        assert!(
            hoje.study_suggestions[0].reason.contains("nada estudado"),
            "{}",
            hoje.study_suggestions[0].reason
        );
        assert_eq!(hoje.exams_soon.len(), 1);
    }

    #[test]
    fn quem_ja_estudou_na_semana_nao_e_cobrado_por_isso() {
        let atual = semestre("2026.2", "2026-08-01", "2026-12-15");
        let materia = disciplina(&atual, "Estatica");
        let p1 = prova(&materia, "P1", agora() + Duration::days(3));
        let estudo = sessao(&materia, agora() - Duration::days(1), 3600);

        let semesters = vec![atual];
        let subjects = vec![materia];
        let exams = vec![p1];
        let sessions = vec![estudo];
        let painel = compose_dashboard(entrada(&semesters, &subjects, &[], &exams, &sessions));
        let hoje = compose_today(&painel, agora());

        assert!(!hoje.study_suggestions[0].reason.contains("nada estudado"));
    }

    #[test]
    fn accent_fora_do_design_system_e_recusado() {
        assert!(validate_accent("trigo").is_ok());
        // O sodio saiu da lista: ele e a cor de atencao do M/OS, e nao a
        // identidade de uma disciplina.
        assert!(validate_accent("sodio").is_err());
        assert_eq!(validate_accent("").unwrap(), "");
        assert!(validate_accent("#ff0000").is_err());
    }

    // =======================================================================
    // As faixas operacionais
    // =======================================================================

    /// O que a tela mostra primeiro: so o que ainda pede decisao.
    #[test]
    fn a_atencao_recebe_o_que_vence_e_deixa_de_fora_o_decidido() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        let mut vence_hoje = atividade(&materia, "APOL 3", Some(agora() + Duration::hours(4)));
        let mut ja_entregue = atividade(&materia, "APOL 2", Some(agora() - Duration::days(2)));
        ja_entregue.decision = crate::academic_decision::Decision::Done;
        let mut descartada = atividade(&materia, "Extra", Some(agora() - Duration::days(1)));
        descartada.decision = crate::academic_decision::Decision::Skipped;
        vence_hoje.due_at = Some(agora() + Duration::hours(4));

        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &[vence_hoje.clone(), ja_entregue, descartada],
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });

        let titulos: Vec<&str> = painel
            .needs_attention
            .iter()
            .map(|i| i.title.as_str())
            .collect();
        assert_eq!(titulos, ["APOL 3"], "so o que ainda pede decisao");
        assert_eq!(painel.history.len(), 2, "as decididas viram historico");
    }

    /// O contador que a Home mostra tem de apontar para o que a tela mostra.
    #[test]
    fn o_contador_de_atraso_ignora_o_que_ja_foi_decidido() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        let mut resolvida = atividade(&materia, "APOL 1", Some(agora() - Duration::days(3)));
        resolvida.decision = crate::academic_decision::Decision::Done;
        let aberta = atividade(&materia, "APOL 2", Some(agora() - Duration::days(1)));

        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &[resolvida, aberta],
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });
        assert_eq!(painel.overdue, 1, "uma atrasada, e nao duas");
    }

    /// A cerimonia da manha nao pode cobrar o que ja foi resolvido ontem.
    #[test]
    fn o_start_my_day_nao_cobra_o_que_ja_foi_entregue() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        let mut entregue = atividade(&materia, "APOL 1", Some(agora() - Duration::days(1)));
        entregue.decision = crate::academic_decision::Decision::Done;

        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &[entregue],
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });
        let hoje = compose_today(&painel, agora());
        assert!(hoje.overdue.is_empty());
        assert!(hoje.due_today.is_empty());
    }

    /// O que eu planejei para hoje aparece hoje, mesmo vencendo sexta.
    #[test]
    fn o_planejado_para_hoje_entra_no_dia_mesmo_vencendo_depois() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        let mut item = atividade(&materia, "APOL 3", Some(agora() + Duration::days(4)));
        item.planned_at = Some(agora() + Duration::hours(6));

        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &[item],
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });
        let hoje = compose_today(&painel, agora());
        assert_eq!(hoje.planned_today.len(), 1);
        assert_eq!(hoje.planned_today[0].title, "APOL 3");
        assert!(
            hoje.due_today.is_empty(),
            "vence daqui a quatro dias: nao e prazo de hoje"
        );
    }

    /// Um mesmo compromisso nunca cai em duas faixas: contaria duas vezes e
    /// pediria duas decisoes.
    #[test]
    fn cada_compromisso_cai_em_uma_faixa_so() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        let itens = vec![
            atividade(&materia, "hoje", Some(agora() + Duration::hours(2))),
            atividade(&materia, "semana", Some(agora() + Duration::days(4))),
            atividade(&materia, "depois", Some(agora() + Duration::days(20))),
        ];
        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &itens,
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });
        let total = painel.needs_attention.len()
            + painel.this_week.len()
            + painel.later.len()
            + painel.history.len();
        assert_eq!(total, 3);
    }

    /// O card da disciplina e a faixa de atencao contam a MESMA coisa. Foi o
    /// defeito visto na tela: "3 atrasadas" no card, "0 compromissos" na faixa,
    /// uma frase acima da outra.
    #[test]
    fn o_card_da_disciplina_nao_conta_o_que_a_faixa_ignora() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        // Prazo de marco: resto de calendario antigo, anterior ao semestre.
        let resto = atividade(
            &materia,
            "Etapa antiga",
            Some(datetime!(2026-03-23 23:59 -03:00)),
        );
        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &[resto],
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });
        assert_eq!(painel.needs_attention.len(), 0);
        assert_eq!(
            painel.subjects[0].overdue, 0,
            "o card nao pode acusar atraso que a faixa nao mostra"
        );
        assert_eq!(painel.history.len(), 1);
    }

    /// O card destaca o proximo compromisso que ainda importa. Antes ele
    /// destacava o mais antigo da lista, que era justamente o resto de
    /// calendario que a faixa ja tinha mandado para o historico.
    #[test]
    fn o_proximo_do_card_pula_o_resto_de_calendario_antigo() {
        let semestre = semestre("2026B2", "2026-07-01", "2026-08-31");
        let materia = disciplina(&semestre, "Estatica");
        let resto = atividade(&materia, "Etapa antiga", Some(datetime!(2026-03-23 23:59 -03:00)));
        let real = atividade(&materia, "APOL 3", Some(datetime!(2026-08-24 23:59 -03:00)));
        let painel = compose_dashboard(DashboardInput {
            now_local: agora(),
            semesters: std::slice::from_ref(&semestre),
            subjects: std::slice::from_ref(&materia),
            assignments: &[resto, real],
            exams: &[],
            sessions: &[],
            materials: &|_| 0,
        });
        assert_eq!(
            painel.subjects[0].next.as_ref().map(|i| i.title.as_str()),
            Some("APOL 3")
        );
    }
}

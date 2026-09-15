//! O piloto: a camada que faz o M/OS procurar a pessoa, em vez de esperar.
//!
//! # Por que um modulo, e nao logica espalhada
//!
//! Antes disto, "o que precisa de atencao" tinha quatro respostas que nao se
//! comparavam: `attention::needs_attention` olhava so lembretes, `stale.rs`
//! olhava so Tasks paradas, o Academic tinha o proprio `needs_attention`, e o
//! carry-over da Daily Session vivia em `daily.rs`. A Home nao tinha como por
//! os quatro na mesma lista, porque eles nao falavam a mesma lingua.
//!
//! Aqui tudo le o MESMO retrato ([`Retrato`]) e devolve tipos que se ordenam
//! entre si. Cada submodulo responde a uma pergunta:
//!
//! | modulo | pergunta |
//! | --- | --- |
//! | [`atencao`] | o que precisa da pessoa, e com que urgencia |
//! | [`proximo`] | o que fazer AGORA, e por que |
//! | [`planejador`] | como comecar e como encerrar o dia com um clique |
//! | [`resgate`] | a pessoa sumiu por dias — por onde retomar |
//! | [`avisos`] | o que vale uma notificacao, e o que e spam |
//! | [`autopilot`] | o observador que junta tudo num so olhar |
//!
//! # Regras da casa
//!
//! Tudo aqui e PURO: recebe o tempo por parametro, nao le repositorio, nao sabe
//! de Tauri nem de HTTP. Deterministico e explicavel — cada recomendacao carrega
//! as razoes em texto, porque "faca isto" sem "porque" e um chute com cara de
//! sistema. Nada de aprendizado agora; a arquitetura deixa a porta aberta pelo
//! [`Retrato`] carregar sinais de habito no futuro (`Habitos`).

pub mod atencao;
pub mod autopilot;
pub mod avisos;
pub mod planejador;
pub mod proximo;
pub mod resgate;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{CalendarItem, Capture, Compromisso, DailyToday, Day, Project, Reminder, Task};

/// A que entidade um item aponta. Texto no `kind`, como o sync: um tipo novo
/// nao pode quebrar quem le.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alvo {
    pub kind: String,
    pub id: String,
}

impl Alvo {
    pub fn task(id: crate::TaskId) -> Self {
        Self {
            kind: "task".into(),
            id: id.to_string(),
        }
    }
    pub fn reminder(id: crate::ReminderId) -> Self {
        Self {
            kind: "reminder".into(),
            id: id.to_string(),
        }
    }
    pub fn capture(id: crate::CaptureId) -> Self {
        Self {
            kind: "capture".into(),
            id: id.to_string(),
        }
    }
    pub fn academic(kind: &str, id: &str) -> Self {
        Self {
            kind: format!("academic_{kind}"),
            id: id.to_owned(),
        }
    }
    pub fn nenhum() -> Self {
        Self {
            kind: String::new(),
            id: String::new(),
        }
    }
}

/// Como a sincronizacao esta, do ponto de vista do piloto. Espelha o
/// `EstadoDeSaude` do `mos-sync` sem depender do crate — o core nao depende do
/// sync, e um `enum` de tres valores nao justifica inverter isso.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncNoRetrato {
    Desligado,
    EmDia,
    Pendente {
        pendentes: usize,
    },
    /// Sem alcancar o hub desde `desde` (RFC3339). Vai tentar sozinho.
    Offline {
        pendentes: usize,
        desde: Option<String>,
    },
    /// Parou por algo que precisa da pessoa.
    Erro {
        pendentes: usize,
        mensagem: String,
    },
}

/// Sinais de habito, para o futuro. Hoje so o que ja da para saber sem
/// aprendizado: a hora em que a pessoa costuma comecar o dia, tirada da mediana
/// das sessoes anteriores. `None` e "ainda nao sei".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Habitos {
    /// Minuto do dia (0..1440) em que os dias costumam comecar.
    pub inicio_habitual_minuto: Option<u16>,
    /// Minuto do dia em que os dias costumam encerrar.
    pub fim_habitual_minuto: Option<u16>,
}

/// Tudo que os motores leem. Montado por quem tem repositorio; lido por quem
/// decide. Uma struct e nao dez parametros pela mesma razao do
/// `calendar::ComposeInput`: trocar duas colecoes de lugar compilaria.
pub struct Retrato<'a> {
    /// O instante local de quem esta olhando, ja no offset certo.
    pub now_local: OffsetDateTime,
    pub tasks: &'a [Task],
    pub projects: &'a [Project],
    pub reminders: &'a [Reminder],
    /// So o que esta na Inbox (processing_state = inbox, ativo).
    pub inbox: &'a [Capture],
    /// Compromissos academicos vivos (entregas e provas com decisao pendente).
    pub academic: &'a [Compromisso],
    /// Itens de calendario de hoje e amanha, em ordem.
    pub agenda: &'a [CalendarItem],
    pub daily: Option<&'a DailyToday>,
    pub sync: &'a SyncNoRetrato,
    /// A ultima vez que a pessoa esteve no M/OS antes desta abertura.
    pub ultima_presenca: Option<OffsetDateTime>,
    pub habitos: &'a Habitos,
}

impl<'a> Retrato<'a> {
    pub fn hoje(&self) -> Day {
        Day::from_local(self.now_local)
    }

    /// Minuto local do dia, 0..1440.
    pub fn minuto_local(&self) -> u16 {
        (self.now_local.hour() as u16) * 60 + self.now_local.minute() as u16
    }

    /// As Tasks que ainda contam: ativas, nao concluidas.
    pub fn tasks_abertas(&self) -> impl Iterator<Item = &'a Task> + '_ {
        self.tasks.iter().filter(|t| {
            t.lifecycle_state == crate::LifecycleState::Active && t.state != crate::TaskState::Done
        })
    }

    /// Lembretes que ainda podem tocar ou cobrar.
    pub fn reminders_abertos(&self) -> impl Iterator<Item = &'a Reminder> + '_ {
        self.reminders.iter().filter(|r| {
            r.lifecycle_state == crate::LifecycleState::Active && !r.status.is_terminal()
        })
    }

    /// Se uma Task esta travada por outra que ainda nao terminou.
    pub fn bloqueada(&self, task: &Task) -> bool {
        task.blocked_by_task_id
            .and_then(|id| self.tasks.iter().find(|t| t.id == id))
            .map(|t| {
                t.state != crate::TaskState::Done
                    && t.lifecycle_state == crate::LifecycleState::Active
            })
            .unwrap_or(false)
    }

    /// O proximo item de agenda depois de agora, hoje.
    pub fn proximo_compromisso(&self) -> Option<&'a CalendarItem> {
        let offset = self.now_local.offset();
        let hoje = self.hoje();
        self.agenda
            .iter()
            .filter(|i| i.at.to_offset(offset) > self.now_local)
            .filter(|i| Day::from_local(i.at.to_offset(offset)) == hoje)
            .filter(|i| e_compromisso(i))
            .min_by_key(|i| i.at)
    }

    /// Minutos livres ate o proximo compromisso de hoje. `None` = o resto do
    /// dia e livre.
    pub fn minutos_livres(&self) -> Option<i64> {
        self.proximo_compromisso()
            .map(|c| ((c.at - self.now_local.to_offset(c.at.offset())).whole_minutes()).max(0))
    }

    /// Dia de um instante, no fuso de quem olha.
    pub fn dia_de(&self, instante: OffsetDateTime) -> Day {
        Day::from_local(instante.to_offset(self.now_local.offset()))
    }
}

/// O que no calendario e AGENDA (futuro com hora) e nao rastro (fato passado).
pub fn e_compromisso(item: &CalendarItem) -> bool {
    use crate::CalendarKind::*;
    // Prazo de Task e de entrega ficam de fora: vencer as 19:47 nao e um
    // compromisso que ocupa a agenda — e o Attention Engine ja cobra o prazo.
    matches!(
        item.kind,
        Meeting | Reminder | ExamScheduled | AcademicPlanned
    )
}

/// Diferenca em dias civis entre dois `Day`. Positivo quando `ate` e depois.
pub fn dias_entre(de: &Day, ate: &Day) -> i64 {
    match (de.date(), ate.date()) {
        (Ok(a), Ok(b)) => (b - a).whole_days(),
        _ => 0,
    }
}

/// O dia seguinte.
pub fn dia_seguinte(dia: &Day) -> Day {
    dia.date()
        .ok()
        .and_then(|d| d.next_day())
        .map(|d| Day::parse(&d.to_string()).unwrap_or_else(|_| dia.clone()))
        .unwrap_or_else(|| dia.clone())
}

/// "~25 min", "~1h30", ou vazio.
pub fn estimativa_curta(minutos: Option<i64>) -> String {
    match minutos {
        None | Some(0) => String::new(),
        Some(m) if m < 60 => format!("~{m} min"),
        Some(m) if m % 60 == 0 => format!("~{}h", m / 60),
        Some(m) => format!("~{}h{:02}", m / 60, m % 60),
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Construtores curtos para os testes dos motores. Tudo em UTC-3, num
    //! sabado as 10:00, para os testes lerem como uma manha de trabalho.
    use super::*;
    use crate::{
        CalendarKind, DailySession, DailySessionId, LifecycleState, Priority, TaskId, TaskState,
    };
    use time::macros::datetime;

    pub fn agora() -> OffsetDateTime {
        datetime!(2026-09-15 10:00 -3)
    }

    pub fn task(titulo: &str) -> Task {
        Task {
            id: TaskId::new(),
            title: titulo.to_owned(),
            description: String::new(),
            project_id: None,
            source_capture_id: None,
            state: TaskState::Backlog,
            lifecycle_state: LifecycleState::Active,
            due_at: None,
            priority: Priority::Normal,
            estimate_minutes: None,
            parent_task_id: None,
            blocked_by_task_id: None,
            waiting_for: String::new(),
            follow_up_at: None,
            scheduled_for: None,
            started_at: None,
            postponed_count: 0,
            checklist_total: 0,
            checklist_done: 0,
            created_at: agora() - time::Duration::days(2),
            updated_at: agora() - time::Duration::days(1),
            completed_at: None,
        }
    }

    pub fn item(kind: CalendarKind, at: OffsetDateTime, titulo: &str) -> CalendarItem {
        CalendarItem {
            kind,
            at,
            ends_at: None,
            title: titulo.to_owned(),
            project_id: None,
            seconds: 0,
            amount_cents: 0,
        }
    }

    pub fn sessao_ativa(dia: Day) -> DailyToday {
        DailyToday {
            day: dia.clone(),
            status: crate::SessionStatus::Active,
            session: Some(DailySession {
                id: DailySessionId::new(),
                day: dia,
                status: crate::SessionStatus::Active,
                note: String::new(),
                started_at: agora() - time::Duration::hours(1),
                ended_at: None,
                created_at: agora() - time::Duration::hours(1),
                updated_at: agora() - time::Duration::hours(1),
            }),
            objectives: Vec::new(),
            reflection: None,
            stale: None,
            stale_objectives: Vec::new(),
        }
    }

    pub fn nao_iniciado(dia: Day) -> DailyToday {
        DailyToday {
            day: dia,
            status: crate::SessionStatus::NotStarted,
            session: None,
            objectives: Vec::new(),
            reflection: None,
            stale: None,
            stale_objectives: Vec::new(),
        }
    }

    pub struct Cenario {
        pub now: OffsetDateTime,
        pub tasks: Vec<Task>,
        pub projects: Vec<Project>,
        pub reminders: Vec<Reminder>,
        pub inbox: Vec<Capture>,
        pub academic: Vec<Compromisso>,
        pub agenda: Vec<CalendarItem>,
        pub daily: Option<DailyToday>,
        pub sync: SyncNoRetrato,
        pub ultima_presenca: Option<OffsetDateTime>,
        pub habitos: Habitos,
    }

    impl Default for Cenario {
        fn default() -> Self {
            Self {
                now: agora(),
                tasks: Vec::new(),
                projects: Vec::new(),
                reminders: Vec::new(),
                inbox: Vec::new(),
                academic: Vec::new(),
                agenda: Vec::new(),
                daily: None,
                sync: SyncNoRetrato::EmDia,
                ultima_presenca: Some(agora() - time::Duration::hours(12)),
                habitos: Habitos::default(),
            }
        }
    }

    impl Cenario {
        pub fn retrato(&self) -> Retrato<'_> {
            Retrato {
                now_local: self.now,
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
}

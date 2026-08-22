//! O que aconteceu, em forma de item de calendario (fase 1).
//!
//! Um tipo so para as quatro fontes que o M/OS ja registra com hora: sessao de
//! trabalho, Task, Capture e programa monitorado aberto. Sem ele, cada fonte
//! chegaria na tela com um formato proprio e o agrupamento por dia precisaria
//! conhecer os quatro.
//!
//! **O instante e UTC e o dia NAO se decide aqui.** O banco guarda UTC, o
//! usuario trabalha de madrugada, e agrupar por dia UTC joga as noites dele
//! para o dia seguinte — a grade mostraria horas em dias que ele nao trabalhou,
//! sem nada quebrar nem falhar. Quem sabe que dia e um instante e o renderer,
//! porque e o unico dos dois lados que conhece o fuso de quem esta olhando.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ProjectId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarKind {
    Session,
    TaskDone,
    TaskCreated,
    Capture,
    /// So abertura, e nao fechamento: abrir sugere que o trabalho comecou, que
    /// e a informacao. Fechar dobraria as marcas do dia sem responder nada que
    /// a abertura ja nao tenha respondido.
    AppOpened,
    /// O dia comecou. Junto com [`Self::DayEnded`] e [`Self::ObjectiveDone`],
    /// e a Daily Session entrando na Linha do Tempo — que e o §27 do pedido
    /// sem infraestrutura nova: o calendario retrospectivo JA e a linha do
    /// tempo do M/OS, e o dia e um fato tao registravel quanto uma sessao de
    /// trabalho.
    DayStarted,
    DayEnded,
    /// Um objetivo do dia foi concluido. E a marca que faz a linha do tempo
    /// contar a historia do dia, e nao so as bordas dele.
    ObjectiveDone,
    /// Uma entrega academica com prazo, e uma prova marcada.
    ///
    /// # Por que o calendario retrospectivo aceita estas duas
    ///
    /// O comentario de `Meeting` acima diz que uma variante de agenda
    /// "prometeria uma capacidade sem lastro" — porque `Event` nao existia no
    /// M/OS e nada tinha data futura de verdade.
    ///
    /// **O M/Academic deu o lastro.** `academic_exams.at` e
    /// `academic_assignments.due_at` sao compromissos com instante marcado,
    /// gravados pela propria pessoa. Nao ha promessa de sincronizar agenda
    /// externa aqui: e o que ja esta no banco, aparecendo no dia em que cai.
    ///
    /// Continua nao havendo um segundo calendario — o §15 do pedido do
    /// M/Academic pede exatamente isso, e a resposta e esta: o Calendario do
    /// M/OS ganha duas fontes, e nao um irmao.
    AssignmentDue,
    ExamScheduled,
    /// **Quando eu vou fazer**, e nao quando o prazo fecha.
    ///
    /// A terceira variante academica existe porque as duas primeiras respondem
    /// a pergunta errada para quem esta planejando o dia. "APOL 3 vence sexta
    /// as 23h59" e um fato do portal; "vou escrever a APOL quarta das 19h30 as
    /// 20h30" e uma decisao minha, e e ela que ocupa a agenda.
    ///
    /// Confundir as duas e o que faz o calendario mostrar trabalho marcado para
    /// a meia-noite de sexta — hora em que ninguem planejou fazer nada.
    AcademicPlanned,
    /// Uma reuniao que aconteceu. Este calendario e retrospectivo por
    /// construcao, e uma reuniao gravada e exatamente o material dele.
    ///
    /// Ela entra como fato passado, e NAO como compromisso futuro: `Event` nao
    /// existe no M/OS, e uma variante que sugerisse agenda prometeria uma
    /// capacidade sem lastro.
    Meeting,
}

impl CalendarKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::TaskDone => "task_done",
            Self::TaskCreated => "task_created",
            Self::Capture => "capture",
            Self::AppOpened => "app_opened",
            Self::DayStarted => "day_started",
            Self::DayEnded => "day_ended",
            Self::ObjectiveDone => "objective_done",
            Self::AssignmentDue => "assignment_due",
            Self::ExamScheduled => "exam_scheduled",
            Self::AcademicPlanned => "academic_planned",
            Self::Meeting => "meeting",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItem {
    pub kind: CalendarKind,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub title: String,
    pub project_id: Option<ProjectId>,
    /// Zero quando o item nao tem duracao.
    pub seconds: i64,
    /// Zero quando o item nao e hora cobravel. Vem de `settle`, a mesma funcao
    /// que produz o total do Painel — nao ha segundo caminho de calculo.
    pub amount_cents: i64,
}

/// O que [`compose`] precisa.
///
/// Estrutura em vez de oito parametros soltos: a lista ja tem quatro colecoes,
/// e trocar duas de lugar por engano compilaria sem reclamacao nenhuma.
pub struct ComposeInput<'a> {
    pub since: OffsetDateTime,
    pub until: OffsetDateTime,
    pub rounding: crate::Rounding,
    pub entries: &'a [crate::TimeEntry],
    pub tasks: &'a [crate::Task],
    pub captures: &'a [crate::Capture],
    pub events: &'a [crate::ActivityEvent],
    /// As sessoes do dia da janela, e os objetivos de todas elas juntos. Duas
    /// listas e nao um par por sessao porque quem le do banco ja pega os
    /// objetivos numa consulta so (`objectives_of`) — remonta-los em pares aqui
    /// so criaria trabalho para desfazer.
    pub sessions: &'a [crate::DailySession],
    pub objectives: &'a [crate::DailyObjective],
    /// Os compromissos academicos da janela, ja compostos por
    /// `academic::compose_dashboard`.
    ///
    /// Chegam prontos, e nao como `Exam` e `Assignment` crus, porque a regra de
    /// o que entra (o que ainda nao foi entregue, o que nao foi cancelado) ja
    /// mora la — reescreve-la aqui daria duas respostas para "esta prova conta?".
    pub academic: &'a [crate::Compromisso],
    /// Como achar o nome de um Project. Fechamento e nao mapa pronto porque
    /// quem chama ja tem a lista e nao deveria precisar montar um indice.
    pub project_name: &'a dyn Fn(ProjectId) -> String,
}

/// Monta os itens do calendario a partir do que ja foi lido.
///
/// Funcao PURA e sem repositorio de proposito: e ela que carrega as regras que
/// podem estar erradas — o que entra na janela, o que vira dois itens, o que e
/// ignorado, em que ordem sai — e regra sem teste e regra que ninguem
/// conferiu. O comando do desktop so busca os dados e chama isto.
pub fn compose(input: ComposeInput<'_>) -> Vec<CalendarItem> {
    let within = |moment: OffsetDateTime| moment >= input.since && moment <= input.until;
    let mut items = Vec::new();

    for compromisso in input.academic {
        // O bloco planejado entra ALEM do prazo, e nao no lugar dele: os dois
        // sao fatos diferentes sobre a mesma coisa, e quem planejou quarta
        // continua precisando ver que o prazo e sexta.
        if let Some(quando) = compromisso.planned_at {
            if within(quando) {
                items.push(CalendarItem {
                    kind: CalendarKind::AcademicPlanned,
                    at: quando,
                    ends_at: (compromisso.planned_minutes > 0)
                        .then(|| quando + time::Duration::minutes(compromisso.planned_minutes)),
                    title: format!("{} — {}", compromisso.subject, compromisso.title),
                    project_id: None,
                    seconds: compromisso.planned_minutes * 60,
                    amount_cents: 0,
                });
            }
        }
        if !within(compromisso.at) {
            continue;
        }
        items.push(CalendarItem {
            kind: if compromisso.kind == "exam" {
                CalendarKind::ExamScheduled
            } else {
                CalendarKind::AssignmentDue
            },
            at: compromisso.at,
            ends_at: None,
            // A disciplina vai junto no titulo: no calendario, "P1" sozinha nao
            // se distingue da P1 de outra materia.
            title: format!("{} — {}", compromisso.subject, compromisso.title),
            project_id: None,
            seconds: 0,
            amount_cents: 0,
        });
    }

    for entry in input.entries {
        if !within(entry.started_at) {
            continue;
        }
        let totals = crate::settle(
            &crate::TrackedSession {
                project_id: entry.project_id.to_string(),
                duration_seconds: entry.duration_seconds,
                idle_seconds: entry.idle_seconds,
                billable: entry.billable,
                hourly_rate_snapshot_cents: entry.hourly_rate_snapshot_cents,
            },
            input.rounding,
        );
        items.push(CalendarItem {
            kind: CalendarKind::Session,
            at: entry.started_at,
            ends_at: entry.ended_at,
            title: (input.project_name)(entry.project_id),
            project_id: Some(entry.project_id),
            seconds: entry.duration_seconds,
            amount_cents: totals.amount_cents,
        });
    }

    // Criada e concluida sao DOIS itens, porque sao dois momentos: a Task que
    // nasceu na segunda e fechou na sexta aconteceu nos dois dias, e um
    // calendario que mostrasse so um deles esconderia metade do trabalho.
    for task in input.tasks {
        if within(task.created_at) {
            items.push(CalendarItem {
                kind: CalendarKind::TaskCreated,
                at: task.created_at,
                ends_at: None,
                title: task.title.clone(),
                project_id: task.project_id,
                seconds: 0,
                amount_cents: 0,
            });
        }
        if let Some(done) = task.completed_at {
            if within(done) {
                items.push(CalendarItem {
                    kind: CalendarKind::TaskDone,
                    at: done,
                    ends_at: None,
                    title: task.title.clone(),
                    project_id: task.project_id,
                    seconds: 0,
                    amount_cents: 0,
                });
            }
        }
    }

    // As Captures ja vem da janela certa: quem le do banco filtrou por data.
    for capture in input.captures {
        items.push(CalendarItem {
            kind: CalendarKind::Capture,
            at: capture.captured_at,
            ends_at: None,
            title: capture.content.clone(),
            project_id: None,
            seconds: 0,
            amount_cents: 0,
        });
    }

    // O dia entra pelas BORDAS e pelos objetivos concluidos, e nao por cada
    // mudanca de estado: a linha do tempo conta o que aconteceu, e "mudei o
    // objetivo de pendente para pendente" nao aconteceu.
    for session in input.sessions {
        if within(session.started_at) {
            items.push(CalendarItem {
                kind: CalendarKind::DayStarted,
                at: session.started_at,
                ends_at: session.ended_at,
                title: "Dia iniciado".to_owned(),
                project_id: None,
                seconds: 0,
                amount_cents: 0,
            });
        }
        if let Some(ended) = session.ended_at {
            if within(ended) {
                items.push(CalendarItem {
                    kind: CalendarKind::DayEnded,
                    at: ended,
                    ends_at: None,
                    title: "Dia encerrado".to_owned(),
                    project_id: None,
                    seconds: 0,
                    amount_cents: 0,
                });
            }
        }
    }

    for objective in input.objectives {
        let Some(done) = objective.completed_at else {
            continue;
        };
        if !within(done) {
            continue;
        }
        items.push(CalendarItem {
            kind: CalendarKind::ObjectiveDone,
            at: done,
            ends_at: None,
            title: objective.title.clone(),
            // O Project do objetivo vem do VINCULO, e so quando ele aponta para
            // um Project direto: seguir a Task ate o Project dela exigiria a
            // lista de Tasks aqui dentro, e esta funcao ja recebe seis colecoes.
            project_id: objective
                .link
                .as_ref()
                .filter(|link| link.kind == crate::LinkKind::Project)
                .and_then(|link| ProjectId::parse(&link.id).ok()),
            seconds: 0,
            amount_cents: 0,
        });
    }

    for event in input.events {
        if event.kind != crate::ActivityKind::AppOpened {
            continue;
        }
        items.push(CalendarItem {
            kind: CalendarKind::AppOpened,
            at: event.detected_at,
            ends_at: None,
            title: event.process_name.clone(),
            project_id: None,
            seconds: 0,
            amount_cents: 0,
        });
    }

    items.sort_by_key(|item| item.at);
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    fn moment(hours: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::hours(hours)
    }

    fn project() -> ProjectId {
        ProjectId::parse("018f0000-0000-7000-8000-000000000001").unwrap()
    }

    fn rounding() -> crate::Rounding {
        crate::Rounding {
            enabled: false,
            interval_minutes: 15,
            mode: crate::RoundingMode::Nearest,
        }
    }

    fn entry(at: OffsetDateTime, seconds: i64) -> crate::TimeEntry {
        crate::TimeEntry {
            id: crate::TimeEntryId::new(),
            project_id: project(),
            started_at: at,
            ended_at: Some(at + Duration::seconds(seconds)),
            duration_seconds: seconds,
            idle_seconds: 0,
            description: String::new(),
            activity_type: crate::ActivityType::Drawing,
            billable: true,
            hourly_rate_snapshot_cents: 3_000,
            source: crate::EntrySource::Timer,
            created_at: at,
            updated_at: at,
        }
    }

    fn task(created: OffsetDateTime, done: Option<OffsetDateTime>) -> crate::Task {
        crate::Task {
            id: crate::TaskId::new(),
            title: "Conferir o corrimao".into(),
            description: String::new(),
            project_id: Some(project()),
            source_capture_id: None,
            state: crate::TaskState::Backlog,
            lifecycle_state: crate::LifecycleState::Active,
            created_at: created,
            updated_at: created,
            completed_at: done,
        }
    }

    fn event(at: OffsetDateTime, kind: crate::ActivityKind) -> crate::ActivityEvent {
        crate::ActivityEvent {
            id: crate::ActivityEventId::new(),
            kind,
            process_name: "acad.exe".into(),
            detected_at: at,
            processed: false,
        }
    }

    fn input<'a>(
        entries: &'a [crate::TimeEntry],
        tasks: &'a [crate::Task],
        captures: &'a [crate::Capture],
        events: &'a [crate::ActivityEvent],
        name: &'a dyn Fn(ProjectId) -> String,
    ) -> ComposeInput<'a> {
        ComposeInput {
            since: moment(0),
            until: moment(24),
            rounding: rounding(),
            entries,
            tasks,
            captures,
            events,
            sessions: &[],
            objectives: &[],
            academic: &[],
            project_name: name,
        }
    }

    /// Os nomes atravessam a ponte para o TypeScript. Um rename silencioso aqui
    /// faria a tela deixar de reconhecer o tipo do item, sem erro de compilacao
    /// de nenhum dos dois lados.
    #[test]
    fn every_kind_round_trips_through_its_wire_name() {
        for kind in [
            CalendarKind::Session,
            CalendarKind::TaskDone,
            CalendarKind::TaskCreated,
            CalendarKind::Capture,
            CalendarKind::AppOpened,
            CalendarKind::DayStarted,
            CalendarKind::DayEnded,
            CalendarKind::ObjectiveDone,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
            assert_eq!(serde_json::from_str::<CalendarKind>(&json).unwrap(), kind);
        }
    }

    /// Sem isto, desenhar um mes traria o ano inteiro.
    #[test]
    fn a_session_outside_the_window_stays_out() {
        let name = |_: ProjectId| "Rancho Queimado".to_owned();
        let entries = [
            entry(moment(-5), 3_600),
            entry(moment(10), 3_600),
            entry(moment(30), 3_600),
        ];
        let items = compose(input(&entries, &[], &[], &[], &name));
        assert_eq!(items.len(), 1, "so a de dentro da janela");
        assert_eq!(items[0].at, moment(10));
    }

    /// `seconds` e o tempo BRUTO e `amount_cents` vem de `settle`. Se um dia
    /// alguem calcular o valor aqui a mao, este teste quebra.
    #[test]
    fn a_session_carries_its_duration_and_the_settled_value() {
        let name = |_: ProjectId| "Rancho Queimado".to_owned();
        let entries = [entry(moment(10), 7_200)];
        let items = compose(input(&entries, &[], &[], &[], &name));

        let expected = crate::settle(
            &crate::TrackedSession {
                project_id: project().to_string(),
                duration_seconds: 7_200,
                idle_seconds: 0,
                billable: true,
                hourly_rate_snapshot_cents: 3_000,
            },
            rounding(),
        );
        assert_eq!(items[0].seconds, 7_200);
        assert_eq!(items[0].amount_cents, expected.amount_cents);
        assert_eq!(items[0].title, "Rancho Queimado");
    }

    #[test]
    fn a_task_never_finished_yields_only_the_created_item() {
        let name = |_: ProjectId| String::new();
        let tasks = [task(moment(3), None)];
        let items = compose(input(&[], &tasks, &[], &[], &name));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, CalendarKind::TaskCreated);
    }

    /// A mesma Task aparece em dois dias porque aconteceram os dois.
    #[test]
    fn a_task_created_and_finished_in_the_window_yields_two() {
        let name = |_: ProjectId| String::new();
        let tasks = [task(moment(3), Some(moment(20)))];
        let items = compose(input(&[], &tasks, &[], &[], &name));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, CalendarKind::TaskCreated);
        assert_eq!(items[1].kind, CalendarKind::TaskDone);
    }

    #[test]
    fn only_app_opened_becomes_an_item() {
        let name = |_: ProjectId| String::new();
        let events = [
            event(moment(4), crate::ActivityKind::AppOpened),
            event(moment(5), crate::ActivityKind::AppClosed),
            event(moment(6), crate::ActivityKind::IdleStarted),
            event(moment(7), crate::ActivityKind::TimerStopped),
        ];
        let items = compose(input(&[], &[], &[], &events, &name));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].at, moment(4));
    }

    /// A ordem e do dia, e nao da ordem em que as fontes foram lidas.
    #[test]
    fn items_come_out_in_chronological_order() {
        let name = |_: ProjectId| String::new();
        let entries = [entry(moment(18), 600)];
        let tasks = [task(moment(2), Some(moment(9)))];
        let events = [event(moment(6), crate::ActivityKind::AppOpened)];
        let items = compose(input(&entries, &tasks, &[], &events, &name));

        let hours: Vec<_> = items.iter().map(|item| item.at).collect();
        assert_eq!(hours, [moment(2), moment(6), moment(9), moment(18)]);
    }
}

//! O que o sistema observa: programas abertos e periodos parados.
//!
//! Separado de `tracking` de proposito. Rastrear tempo e uma decisao do
//! usuario; observar o sistema e algo que acontece por conta, e o dominio que
//! descreve isso tem regras proprias — a principal delas sendo que **observacao
//! nao vira hora sozinha**. O CronoCAD ja tinha essa disciplina: o evento fica
//! guardado, a Linha do Tempo mostra o vao, e quem decide se aquilo foi
//! trabalho e a pessoa.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CoreError, ErrorCode};

macro_rules! monitoring_id {
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

monitoring_id!(ActivityEventId, "Activity event ID");

/// Um programa cuja abertura sugere trabalho.
///
/// O `id` e TEXT livre e nao UUID porque as sugestoes vem semeadas pela
/// migration com ids legiveis (`app-acad`), e o CronoCAD as trouxe assim. Trocar
/// para UUID quebraria o `ON CONFLICT` que impede duplicar a semente.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredApp {
    pub id: String,
    pub display_name: String,
    /// Nome do executavel. Unico: o monitoramento casa por ele, e dois
    /// cadastros do mesmo processo gerariam dois lembretes para uma abertura.
    pub process_name: String,
    pub enabled: bool,
    pub remind_on_open: bool,
    pub remind_on_close: bool,
}

/// O que o sistema observou.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    AppOpened,
    AppClosed,
    IdleStarted,
    IdleEnded,
    TimerStarted,
    TimerPaused,
    TimerResumed,
    #[default]
    TimerStopped,
}

impl ActivityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AppOpened => "app_opened",
            Self::AppClosed => "app_closed",
            Self::IdleStarted => "idle_started",
            Self::IdleEnded => "idle_ended",
            Self::TimerStarted => "timer_started",
            Self::TimerPaused => "timer_paused",
            Self::TimerResumed => "timer_resumed",
            Self::TimerStopped => "timer_stopped",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "app_opened" => Ok(Self::AppOpened),
            "app_closed" => Ok(Self::AppClosed),
            "idle_started" => Ok(Self::IdleStarted),
            "idle_ended" => Ok(Self::IdleEnded),
            "timer_started" => Ok(Self::TimerStarted),
            "timer_paused" => Ok(Self::TimerPaused),
            "timer_resumed" => Ok(Self::TimerResumed),
            "timer_stopped" => Ok(Self::TimerStopped),
            _ => Err(CoreError::new(
                ErrorCode::InvalidInput,
                format!("`{value}` nao e um tipo de evento."),
                false,
            )),
        }
    }
}

/// Uma observacao do sistema, com hora.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: ActivityEventId,
    pub kind: ActivityKind,
    /// Qual programa, quando o evento e sobre programa.
    pub process_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub detected_at: OffsetDateTime,
    /// Marca que este evento ja virou sessao ou ja foi descartado. Existe para a
    /// Linha do Tempo nao reoferecer o mesmo periodo todo dia.
    pub processed: bool,
}

#[derive(Clone, Debug)]
pub struct NewActivityEvent {
    pub kind: ActivityKind,
    pub process_name: String,
    pub detected_at: OffsetDateTime,
}

/// Um intervalo fechado de tempo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    #[serde(with = "time::serde::rfc3339")]
    pub start: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub end: OffsetDateTime,
}

impl Period {
    pub fn seconds(&self) -> i64 {
        (self.end - self.start).whole_seconds().max(0)
    }
}

/// Os períodos em que algum programa monitorado esteve aberto.
///
/// Casa `app_opened` com o `app_closed` do MESMO processo. Uma abertura sem
/// fechamento — o programa ainda está aberto, ou o M/OS foi encerrado antes —
/// fecha em `now`, porque o trabalho que está acontecendo agora é justamente o
/// que mais interessa oferecer.
pub fn open_periods(events: &[ActivityEvent], now: OffsetDateTime) -> Vec<Period> {
    let mut open: Vec<(String, OffsetDateTime)> = Vec::new();
    let mut periods = Vec::new();

    for event in events {
        match event.kind {
            ActivityKind::AppOpened => {
                open.push((event.process_name.clone(), event.detected_at));
            }
            ActivityKind::AppClosed => {
                // Do fim para o começo: um mesmo programa aberto duas vezes
                // fecha primeiro a abertura mais recente, que é o que a pilha do
                // sistema operacional faz.
                if let Some(index) = open
                    .iter()
                    .rposition(|(name, _)| name == &event.process_name)
                {
                    let (_, start) = open.remove(index);
                    if event.detected_at > start {
                        periods.push(Period {
                            start,
                            end: event.detected_at,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    for (_, start) in open {
        if now > start {
            periods.push(Period { start, end: now });
        }
    }

    periods.sort_by_key(|period| period.start);
    periods
}

/// O que sobra de `periods` depois de tirar o que `covered` já cobre.
///
/// É isto que a Linha do Tempo oferece: o programa esteve aberto, e não há
/// sessão registrada naquele pedaço. O resto ela não mostra, porque oferecer um
/// período já registrado convidaria a contar a mesma hora duas vezes.
pub fn uncovered(periods: &[Period], covered: &[Period]) -> Vec<Period> {
    let mut blocks: Vec<Period> = covered.to_vec();
    blocks.sort_by_key(|period| period.start);

    let mut result = Vec::new();
    for period in periods {
        let mut cursor = period.start;
        for block in &blocks {
            if block.end <= cursor || block.start >= period.end {
                continue;
            }
            if block.start > cursor {
                result.push(Period {
                    start: cursor,
                    end: block.start.min(period.end),
                });
            }
            cursor = cursor.max(block.end);
            if cursor >= period.end {
                break;
            }
        }
        if cursor < period.end {
            result.push(Period {
                start: cursor,
                end: period.end,
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os nomes atravessam a ponte e vivem no banco, com CHECK constraint. Um
    /// rename silencioso quebraria a leitura de 321 eventos ja gravados.
    #[test]
    fn every_kind_round_trips_through_its_wire_name() {
        let all = [
            ActivityKind::AppOpened,
            ActivityKind::AppClosed,
            ActivityKind::IdleStarted,
            ActivityKind::IdleEnded,
            ActivityKind::TimerStarted,
            ActivityKind::TimerPaused,
            ActivityKind::TimerResumed,
            ActivityKind::TimerStopped,
        ];
        for kind in all {
            assert_eq!(ActivityKind::parse(kind.as_str()).unwrap(), kind);
        }
    }

    #[test]
    fn an_unknown_kind_is_refused_by_name() {
        let error = ActivityKind::parse("app_exploded").unwrap_err();
        assert!(error.message.contains("app_exploded"));
    }

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000 + seconds).unwrap()
    }

    fn event(kind: ActivityKind, process: &str, seconds: i64) -> ActivityEvent {
        ActivityEvent {
            id: ActivityEventId::new(),
            kind,
            process_name: process.to_owned(),
            detected_at: at(seconds),
            processed: false,
        }
    }

    fn period(start: i64, end: i64) -> Period {
        Period {
            start: at(start),
            end: at(end),
        }
    }

    #[test]
    fn an_open_and_close_pair_becomes_one_period() {
        let events = [
            event(ActivityKind::AppOpened, "acad.exe", 0),
            event(ActivityKind::AppClosed, "acad.exe", 3_600),
        ];
        assert_eq!(open_periods(&events, at(9_999)), vec![period(0, 3_600)]);
    }

    /// O programa continua aberto agora. E justamente o trabalho em curso, e
    /// deixa-lo de fora esconderia o periodo que mais interessa oferecer.
    #[test]
    fn an_unclosed_program_runs_until_now() {
        let events = [event(ActivityKind::AppOpened, "revit.exe", 0)];
        assert_eq!(open_periods(&events, at(600)), vec![period(0, 600)]);
    }

    /// Fechamento sem abertura acontece: o M/OS pode ter comecado a observar com
    /// o programa ja aberto. Ignorar e melhor que inventar um inicio.
    #[test]
    fn a_close_without_an_open_is_ignored() {
        let events = [event(ActivityKind::AppClosed, "acad.exe", 3_600)];
        assert!(open_periods(&events, at(9_999)).is_empty());
    }

    #[test]
    fn events_from_different_programs_do_not_pair_with_each_other() {
        let events = [
            event(ActivityKind::AppOpened, "acad.exe", 0),
            event(ActivityKind::AppOpened, "revit.exe", 100),
            event(ActivityKind::AppClosed, "acad.exe", 200),
            event(ActivityKind::AppClosed, "revit.exe", 300),
        ];
        assert_eq!(
            open_periods(&events, at(9_999)),
            vec![period(0, 200), period(100, 300)]
        );
    }

    /// A hora ja registrada sai da oferta. Sem isso a Linha do Tempo convidaria
    /// a contar a mesma hora duas vezes — e a segunda contagem so apareceria na
    /// fatura.
    #[test]
    fn a_recorded_session_is_carved_out_of_the_offer() {
        let open = [period(0, 3_600)];
        let recorded = [period(600, 1_200)];
        assert_eq!(
            uncovered(&open, &recorded),
            vec![period(0, 600), period(1_200, 3_600)]
        );
    }

    #[test]
    fn a_fully_covered_period_offers_nothing() {
        assert!(uncovered(&[period(600, 1_200)], &[period(0, 3_600)]).is_empty());
    }

    #[test]
    fn nothing_recorded_offers_the_whole_period() {
        assert_eq!(uncovered(&[period(0, 3_600)], &[]), vec![period(0, 3_600)]);
    }

    #[test]
    fn several_sessions_carve_several_holes() {
        let open = [period(0, 10_000)];
        let recorded = [period(1_000, 2_000), period(5_000, 6_000)];
        assert_eq!(
            uncovered(&open, &recorded),
            vec![
                period(0, 1_000),
                period(2_000, 5_000),
                period(6_000, 10_000)
            ]
        );
    }

    /// Sessoes fora de ordem e sobrepostas sao o caso real de quem corrigiu
    /// horario a mao. O resultado nao pode depender da ordem em que chegam.
    #[test]
    fn overlapping_and_unsorted_sessions_still_carve_correctly() {
        let open = [period(0, 10_000)];
        let recorded = [
            period(5_000, 6_000),
            period(1_000, 2_000),
            period(1_500, 5_500),
        ];
        assert_eq!(
            uncovered(&open, &recorded),
            vec![period(0, 1_000), period(6_000, 10_000)]
        );
    }
}

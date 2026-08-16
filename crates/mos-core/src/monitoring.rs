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
}

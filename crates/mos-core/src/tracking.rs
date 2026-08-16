//! Rastreio de tempo por projeto — o domínio que vem do CronoCAD (ADR-032).
//!
//! Etapa A da absorção: só a REGRA entra aqui. Persistência, comandos, bandeja e
//! detecção de processo continuam no `apps/cronocad` até a etapa B mover o
//! esquema. Mover a regra primeiro é o que torna a migração reversível — se
//! parar aqui, nada quebrou e o M/OS ganhou um domínio testado.
//!
//! Todas as funções são puras. Timestamps são segundos epoch (`i64`) para não
//! dependerem de fuso e para sobreviverem a mudança de relógio: diferença
//! negativa nunca reduz tempo já acumulado.
//!
//! Duas regras deste módulo não são detalhe de implementação, e sim decisões que
//! o CronoCAD tomou e que a absorção preserva:
//!
//! 1. **O banco guarda sempre o tempo real.** Arredondamento e desconto de
//!    inatividade se aplicam na visualização e na cobrança, nunca sobrescrevendo
//!    o valor original.
//! 2. **O arredondamento é por sessão, e depois somado.** Arredondar a soma daria
//!    outro número, e duas telas do mesmo dado divergiriam.
//!
//! O módulo se chama `tracking` e não `time` porque `time` já é o nome do crate
//! de data e hora usado no restante do domínio.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CoreError, ErrorCode, ProjectId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimeEntryId(Uuid);

impl TimeEntryId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::InvalidInput, "Time entry ID invalido.", false))
    }
}

impl Default for TimeEntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TimeEntryId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// O tipo de trabalho da sessão. Vocabulário de projetista de CAD, herdado do
/// CronoCAD — é o que o usuário reconhece ao rever a semana.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Drawing,
    Detailing,
    Revision,
    Meeting,
    Study,
    #[default]
    Other,
}

/// De onde a sessão veio.
///
/// `Reconstructed` não é detalhe técnico: marca a hora que o usuário reconstruiu
/// depois, e não a que o cronômetro mediu. Faturar as duas sem distinguir seria
/// cobrar estimativa como medição.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrySource {
    #[default]
    Timer,
    Manual,
    Reconstructed,
}

/// Estado do projeto no rastreio de tempo.
///
/// Existe porque `LifecycleState` do M/OS não distingue "concluído" de
/// "arquivado", e para uma obra entregue essa diferença é a resposta a "por que
/// isso parou?".
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingStatus {
    #[default]
    Active,
    Paused,
    Completed,
    Archived,
}

macro_rules! parse_enum {
    ($name:ident, $label:literal, $($text:literal => $variant:path),+ $(,)?) => {
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $($variant => $text,)+
                }
            }

            pub fn parse(value: &str) -> Result<Self, CoreError> {
                match value {
                    $($text => Ok($variant),)+
                    _ => Err(CoreError::new(
                        ErrorCode::InvalidInput,
                        format!(concat!("`{}` nao e ", $label, "."), value),
                        false,
                    )),
                }
            }
        }
    };
}

parse_enum!(
    ActivityType,
    "um tipo de atividade",
    "drawing" => ActivityType::Drawing,
    "detailing" => ActivityType::Detailing,
    "revision" => ActivityType::Revision,
    "meeting" => ActivityType::Meeting,
    "study" => ActivityType::Study,
    "other" => ActivityType::Other,
);

parse_enum!(
    EntrySource,
    "uma origem de sessao",
    "timer" => EntrySource::Timer,
    "manual" => EntrySource::Manual,
    "reconstructed" => EntrySource::Reconstructed,
);

parse_enum!(
    TrackingStatus,
    "um estado de rastreio",
    "active" => TrackingStatus::Active,
    "paused" => TrackingStatus::Paused,
    "completed" => TrackingStatus::Completed,
    "archived" => TrackingStatus::Archived,
);

parse_enum!(
    RoundingMode,
    "um modo de arredondamento",
    "nearest" => RoundingMode::Nearest,
    "up" => RoundingMode::Up,
    "down" => RoundingMode::Down,
);

/// Uma sessão de trabalho já gravada.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: TimeEntryId,
    pub project_id: ProjectId,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub description: String,
    pub activity_type: ActivityType,
    pub billable: bool,
    pub hourly_rate_snapshot_cents: i64,
    pub source: EntrySource,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// O que é preciso para gravar uma sessão.
///
/// `hourly_rate_snapshot_cents` entra aqui e não é derivado do Project na hora
/// de gravar, porque a importação precisa preservar a taxa que valia na época —
/// e não a que vale hoje.
#[derive(Clone, Debug)]
pub struct NewTimeEntry {
    pub project_id: ProjectId,
    pub started_at: OffsetDateTime,
    pub ended_at: Option<OffsetDateTime>,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub description: String,
    pub activity_type: ActivityType,
    pub billable: bool,
    pub hourly_rate_snapshot_cents: i64,
    pub source: EntrySource,
}

/// Dados de cobrança de um Project.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTracking {
    pub project_id: ProjectId,
    pub hourly_rate_cents: i64,
    pub code: String,
    pub color: String,
    pub tracking_status: TrackingStatus,
}

/// Configuração de arredondamento e inatividade, linha única.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingSettings {
    pub rounding: Rounding,
    pub idle_threshold_minutes: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerStatus {
    Running,
    Paused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    Nearest,
    Up,
    Down,
}

/// Estado mínimo para calcular a duração decorrida.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    pub status: TimerStatus,
    /// Segundos já acumulados em pausas anteriores.
    pub accumulated_seconds: i64,
    /// Epoch do último `resume`. Ignorado quando pausado.
    pub last_resumed_epoch: i64,
}

/// Segundos decorridos em `now_epoch`.
///
/// Nunca negativo, e robusto a relógio para trás — o delta negativo vira zero em
/// vez de subtrair. Um cronômetro que anda para trás porque o sistema ajustou a
/// hora apagaria trabalho que existiu.
pub fn elapsed_seconds(timer: &TimerSnapshot, now_epoch: i64) -> i64 {
    let accumulated = timer.accumulated_seconds.max(0);
    match timer.status {
        TimerStatus::Paused => accumulated,
        TimerStatus::Running => accumulated + (now_epoch - timer.last_resumed_epoch).max(0),
    }
}

/// Duração líquida: bruta menos inativa, nunca negativa.
pub fn net_duration(gross_seconds: i64, idle_seconds: i64) -> i64 {
    (gross_seconds - idle_seconds.max(0)).max(0)
}

/// Duração faturável: a líquida quando a sessão é cobrável, senão zero.
pub fn billable_duration(net_seconds: i64, billable: bool) -> i64 {
    if billable {
        net_seconds.max(0)
    } else {
        0
    }
}

/// Arredonda para o intervalo dado.
///
/// NUNCA é persistido: aplica-se só na visualização e na cobrança. Intervalo
/// menor ou igual a zero devolve a duração original, e é isso que faz
/// "arredondamento desativado" não precisar de um caminho próprio.
pub fn round_duration(seconds: i64, interval_minutes: i64, mode: RoundingMode) -> i64 {
    if interval_minutes <= 0 {
        return seconds;
    }
    let interval = interval_minutes * 60;
    let quotient = seconds as f64 / interval as f64;
    let units = match mode {
        RoundingMode::Up => quotient.ceil(),
        RoundingMode::Down => quotient.floor(),
        RoundingMode::Nearest => quotient.round(),
    };
    (units as i64) * interval
}

/// Valor em centavos para uma duração, dado o valor/hora em centavos.
pub fn amount_for_duration(seconds: i64, hourly_rate_cents: i64) -> i64 {
    let hours = seconds.max(0) as f64 / 3600.0;
    (hours * hourly_rate_cents as f64).round() as i64
}

/// Configuração de arredondamento.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rounding {
    pub enabled: bool,
    pub interval_minutes: i64,
    pub mode: RoundingMode,
}

impl Rounding {
    /// Zero quando desativado, o que faz `round_duration` devolver o original.
    fn effective_interval(&self) -> i64 {
        if self.enabled {
            self.interval_minutes
        } else {
            0
        }
    }
}

/// Uma sessão de trabalho, no mínimo que a cobrança precisa.
///
/// `TrackedSession` e não `Session` porque `sessão` já significa outra coisa no
/// M/OS — a sessão do Hermes na VPS. Dois tipos com o mesmo nome e domínios
/// diferentes é o começo de um `use` errado que compila.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedSession {
    pub project_id: String,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub billable: bool,
    /// Valor/hora preservado no momento da sessão: reajustar o projeto não
    /// reescreve o que já foi trabalhado.
    pub hourly_rate_snapshot_cents: i64,
}

/// Totais acumulados de um projeto.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    /// Tempo real registrado, sem desconto nenhum.
    pub gross_seconds: i64,
    pub idle_seconds: i64,
    /// Líquido, faturável e já arredondado — a base da cobrança.
    pub billable_seconds: i64,
    pub amount_cents: i64,
}

/// Soma as sessões por projeto.
///
/// Projeto sem sessão simplesmente não aparece no mapa: cabe a quem consome
/// tratar a ausência como zero. Devolver zero aqui obrigaria esta função a
/// conhecer a lista de projetos, que é dado de outro agregado.
pub fn aggregate_by_project(
    sessions: &[TrackedSession],
    rounding: Rounding,
) -> HashMap<String, Totals> {
    let interval = rounding.effective_interval();
    let mut totals: HashMap<String, Totals> = HashMap::new();

    for session in sessions {
        let net = net_duration(session.duration_seconds, session.idle_seconds);
        let billable = billable_duration(net, session.billable);
        let rounded = round_duration(billable, interval, rounding.mode);

        let entry = totals.entry(session.project_id.clone()).or_default();
        entry.gross_seconds += session.duration_seconds.max(0);
        entry.idle_seconds += session.idle_seconds.max(0);
        entry.billable_seconds += rounded;
        entry.amount_cents += amount_for_duration(rounded, session.hourly_rate_snapshot_cents);
    }

    totals
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUNDING_OFF: Rounding = Rounding {
        enabled: false,
        interval_minutes: 15,
        mode: RoundingMode::Nearest,
    };

    fn running(accumulated: i64, last_resumed: i64) -> TimerSnapshot {
        TimerSnapshot {
            status: TimerStatus::Running,
            accumulated_seconds: accumulated,
            last_resumed_epoch: last_resumed,
        }
    }

    fn session(
        project: &str,
        duration: i64,
        idle: i64,
        billable: bool,
        rate: i64,
    ) -> TrackedSession {
        TrackedSession {
            project_id: project.to_owned(),
            duration_seconds: duration,
            idle_seconds: idle,
            billable,
            hourly_rate_snapshot_cents: rate,
        }
    }

    #[test]
    fn a_running_timer_adds_the_time_since_the_last_resume() {
        assert_eq!(elapsed_seconds(&running(100, 1_000), 1_030), 130);
    }

    #[test]
    fn a_paused_timer_reports_only_what_it_accumulated() {
        let timer = TimerSnapshot {
            status: TimerStatus::Paused,
            accumulated_seconds: 250,
            last_resumed_epoch: 0,
        };
        assert_eq!(elapsed_seconds(&timer, 999_999), 250);
    }

    /// O relogio do sistema pode andar para tras — fuso, NTP, o usuario. Se o
    /// delta negativo entrasse na conta, trabalho que existiu seria apagado.
    #[test]
    fn a_clock_going_backwards_never_reduces_accumulated_time() {
        assert_eq!(elapsed_seconds(&running(500, 1_000), 940), 500);
    }

    #[test]
    fn idle_time_comes_off_the_net_duration() {
        assert_eq!(net_duration(3_600, 600), 3_000);
        assert_eq!(net_duration(300, 600), 0);
    }

    #[test]
    fn a_non_billable_session_bills_nothing() {
        assert_eq!(billable_duration(3_000, true), 3_000);
        assert_eq!(billable_duration(3_000, false), 0);
    }

    #[test]
    fn rounding_up_takes_an_hour_and_seven_to_an_hour_and_fifteen() {
        assert_eq!(round_duration(4_020, 15, RoundingMode::Up), 4_500);
    }

    #[test]
    fn rounding_down_and_nearest_land_where_expected() {
        assert_eq!(round_duration(4_020, 15, RoundingMode::Down), 3_600);
        assert_eq!(round_duration(4_020, 15, RoundingMode::Nearest), 3_600);
        assert_eq!(round_duration(4_080, 15, RoundingMode::Nearest), 4_500);
    }

    /// Intervalo invalido devolve o original em vez de errar: e o que permite
    /// "arredondamento desativado" ser o mesmo caminho de codigo.
    #[test]
    fn an_invalid_interval_returns_the_original_duration() {
        assert_eq!(round_duration(4_020, 0, RoundingMode::Up), 4_020);
    }

    #[test]
    fn the_amount_follows_the_hourly_rate() {
        assert_eq!(amount_for_duration(3_600, 10_000), 10_000);
        assert_eq!(amount_for_duration(5_400, 10_000), 15_000);
        assert_eq!(amount_for_duration(4_020, 9_000), 10_050);
    }

    #[test]
    fn sessions_are_summed_per_project() {
        let sessions = vec![
            session("a", 3_600, 0, true, 10_000),
            session("a", 1_800, 0, true, 10_000),
            session("b", 3_600, 0, true, 5_000),
        ];
        let totals = aggregate_by_project(&sessions, ROUNDING_OFF);

        assert_eq!(totals["a"].billable_seconds, 5_400);
        assert_eq!(totals["a"].amount_cents, 15_000);
        assert_eq!(totals["b"].amount_cents, 5_000);
    }

    #[test]
    fn a_project_without_sessions_is_absent_rather_than_zero() {
        assert!(!aggregate_by_project(&[], ROUNDING_OFF).contains_key("a"));
    }

    #[test]
    fn a_non_billable_session_still_counts_its_hours() {
        let totals = aggregate_by_project(&[session("a", 3_600, 0, false, 10_000)], ROUNDING_OFF);

        assert_eq!(totals["a"].gross_seconds, 3_600);
        assert_eq!(totals["a"].billable_seconds, 0);
        assert_eq!(totals["a"].amount_cents, 0);
    }

    /// O bruto e o tempo real e nao se mexe. O desconto vive no faturavel.
    #[test]
    fn idle_reduces_the_billable_but_never_the_gross() {
        let totals = aggregate_by_project(&[session("a", 3_600, 600, true, 10_000)], ROUNDING_OFF);

        assert_eq!(totals["a"].gross_seconds, 3_600);
        assert_eq!(totals["a"].idle_seconds, 600);
        assert_eq!(totals["a"].billable_seconds, 3_000);
        assert_eq!(totals["a"].amount_cents, 8_333);
    }

    /// Duas sessoes de 10min arredondadas para 15 dao 30. Arredondar a SOMA
    /// daria 15 — e as duas telas do mesmo dado divergiriam.
    #[test]
    fn rounding_applies_per_session_and_not_over_the_sum() {
        let rounding = Rounding {
            enabled: true,
            interval_minutes: 15,
            mode: RoundingMode::Up,
        };
        let sessions = vec![
            session("a", 600, 0, true, 6_000),
            session("a", 600, 0, true, 6_000),
        ];
        let totals = aggregate_by_project(&sessions, rounding);

        assert_eq!(totals["a"].billable_seconds, 1_800);
        assert_eq!(totals["a"].amount_cents, 3_000);
    }

    #[test]
    fn rounding_disabled_preserves_the_real_time() {
        let totals = aggregate_by_project(&[session("a", 600, 0, true, 6_000)], ROUNDING_OFF);

        assert_eq!(totals["a"].billable_seconds, 600);
        assert_eq!(totals["a"].amount_cents, 1_000);
    }

    /// O projeto teve reajuste, e cada sessao mantem o valor da sua epoca.
    #[test]
    fn each_session_keeps_its_own_hourly_rate_snapshot() {
        let sessions = vec![
            session("a", 3_600, 0, true, 5_000),
            session("a", 3_600, 0, true, 9_000),
        ];
        let totals = aggregate_by_project(&sessions, ROUNDING_OFF);

        assert_eq!(totals["a"].amount_cents, 14_000);
    }
}

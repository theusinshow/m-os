//! Calculo de duracao, arredondamento e valor — regras centrais (secoes 9-12).
//!
//! Todas as funcoes sao puras e determinísticas. Timestamps sao expressos em
//! segundos epoch (i64) para independer do fuso e serem robustos a mudancas do
//! relogio: diferencas negativas nunca reduzem tempo ja acumulado.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerStatus {
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    Nearest,
    Up,
    Down,
}

/// Estado minimo necessario para calcular a duracao decorrida.
#[derive(Debug, Clone, Copy)]
pub struct TimerSnapshot {
    pub status: TimerStatus,
    /// Segundos ja acumulados em pausas anteriores.
    pub accumulated_seconds: i64,
    /// Epoch (segundos) do ultimo `resume`. Ignorado quando pausado.
    pub last_resumed_epoch: i64,
}

/// Segundos decorridos em `now_epoch`. Nunca negativo; robusto a relogio para
/// tras (delta negativo e tratado como zero).
pub fn elapsed_seconds(timer: &TimerSnapshot, now_epoch: i64) -> i64 {
    let accumulated = timer.accumulated_seconds.max(0);
    match timer.status {
        TimerStatus::Paused => accumulated,
        TimerStatus::Running => {
            let delta = (now_epoch - timer.last_resumed_epoch).max(0);
            accumulated + delta
        }
    }
}

/// Duracao liquida = bruta - inativa (nunca negativa).
pub fn net_duration(gross_seconds: i64, idle_seconds: i64) -> i64 {
    (gross_seconds - idle_seconds.max(0)).max(0)
}

/// Duracao faturavel: liquida quando `billable`, senao 0.
pub fn billable_duration(net_seconds: i64, billable: bool) -> i64 {
    if billable {
        net_seconds.max(0)
    } else {
        0
    }
}

/// Arredonda a duracao para o intervalo dado. NUNCA e persistido: aplica-se
/// apenas na visualizacao/cobranca (secao 12). Intervalo <= 0 retorna original.
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

/// Valor em centavos para uma duracao dado o valor/hora em centavos.
/// Arredonda para o centavo inteiro mais proximo.
pub fn amount_for_duration(seconds: i64, hourly_rate_cents: i64) -> i64 {
    let hours = seconds.max(0) as f64 / 3600.0;
    (hours * hourly_rate_cents as f64).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(accumulated: i64, last_resumed: i64) -> TimerSnapshot {
        TimerSnapshot {
            status: TimerStatus::Running,
            accumulated_seconds: accumulated,
            last_resumed_epoch: last_resumed,
        }
    }

    #[test]
    fn elapsed_somando_desde_resume() {
        let t = running(100, 1_000);
        assert_eq!(elapsed_seconds(&t, 1_030), 130);
    }

    #[test]
    fn elapsed_pausado_retorna_acumulado() {
        let t = TimerSnapshot {
            status: TimerStatus::Paused,
            accumulated_seconds: 250,
            last_resumed_epoch: 0,
        };
        assert_eq!(elapsed_seconds(&t, 999_999), 250);
    }

    #[test]
    fn elapsed_relogio_para_tras_nao_reduz() {
        let t = running(500, 1_000);
        assert_eq!(elapsed_seconds(&t, 940), 500);
    }

    #[test]
    fn net_desconta_inatividade() {
        assert_eq!(net_duration(3_600, 600), 3_000);
        assert_eq!(net_duration(300, 600), 0);
    }

    #[test]
    fn billable_respeita_flag() {
        assert_eq!(billable_duration(3_000, true), 3_000);
        assert_eq!(billable_duration(3_000, false), 0);
    }

    #[test]
    fn round_up_1h07_para_1h15() {
        // 4020s = 1h07 -> 4500s = 1h15
        assert_eq!(round_duration(4_020, 15, RoundingMode::Up), 4_500);
    }

    #[test]
    fn round_down_e_nearest() {
        assert_eq!(round_duration(4_020, 15, RoundingMode::Down), 3_600);
        assert_eq!(round_duration(4_020, 15, RoundingMode::Nearest), 3_600);
        assert_eq!(round_duration(4_080, 15, RoundingMode::Nearest), 4_500);
    }

    #[test]
    fn round_intervalo_invalido_retorna_original() {
        assert_eq!(round_duration(4_020, 0, RoundingMode::Up), 4_020);
    }

    #[test]
    fn amount_calcula_valor() {
        assert_eq!(amount_for_duration(3_600, 10_000), 10_000);
        assert_eq!(amount_for_duration(5_400, 10_000), 15_000);
        assert_eq!(amount_for_duration(4_020, 9_000), 10_050);
    }
}

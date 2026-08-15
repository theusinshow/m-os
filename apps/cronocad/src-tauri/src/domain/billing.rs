//! Agregacao de horas e valores por projeto (secoes 11-12 e 20).
//!
//! Puro e deterministico: recebe as sessoes ja lidas do banco e devolve os
//! totais. Nao reimplementa arredondamento nem calculo monetario — compoe as
//! funcoes ja testadas de [`super::timer`].
//!
//! Regra critica: o arredondamento e aplicado **por sessao** e depois somado,
//! exatamente como o Relatorio faz no frontend. Arredondar a soma daria outro
//! numero e as duas telas divergiriam.

use std::collections::HashMap;

use super::timer::{
    amount_for_duration, billable_duration, net_duration, round_duration, RoundingMode,
};

/// Configuracao de arredondamento vinda da tabela `settings`.
#[derive(Debug, Clone, Copy)]
pub struct Rounding {
    pub enabled: bool,
    pub interval_minutes: i64,
    pub mode: RoundingMode,
}

impl Rounding {
    /// Intervalo efetivo: zero quando desativado, o que faz
    /// [`round_duration`] devolver a duracao original.
    fn effective_interval(&self) -> i64 {
        if self.enabled {
            self.interval_minutes
        } else {
            0
        }
    }
}

/// Dados minimos de uma sessao para o calculo de cobranca.
#[derive(Debug, Clone)]
pub struct Session {
    pub project_id: String,
    pub duration_seconds: i64,
    pub idle_seconds: i64,
    pub billable: bool,
    /// Valor/hora preservado no momento da sessao (secao 20): alterar o valor
    /// atual do projeto nao altera sessoes ja registradas.
    pub hourly_rate_snapshot_cents: i64,
}

/// Totais acumulados de um projeto.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    /// Tempo real registrado, sem qualquer desconto.
    pub gross_seconds: i64,
    pub idle_seconds: i64,
    /// Liquido, faturavel e ja arredondado — base da cobranca.
    pub billable_seconds: i64,
    pub amount_cents: i64,
}

/// Soma as sessoes por projeto. Projetos sem sessao simplesmente nao aparecem
/// no mapa; cabe a quem consome tratar a ausencia como zero.
pub fn aggregate_by_project(sessions: &[Session], rounding: Rounding) -> HashMap<String, Totals> {
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

    const OFF: Rounding = Rounding {
        enabled: false,
        interval_minutes: 15,
        mode: RoundingMode::Nearest,
    };

    fn session(project: &str, duration: i64, idle: i64, billable: bool, rate: i64) -> Session {
        Session {
            project_id: project.to_string(),
            duration_seconds: duration,
            idle_seconds: idle,
            billable,
            hourly_rate_snapshot_cents: rate,
        }
    }

    #[test]
    fn soma_por_projeto_separadamente() {
        let sessions = vec![
            session("a", 3_600, 0, true, 10_000),
            session("a", 1_800, 0, true, 10_000),
            session("b", 3_600, 0, true, 5_000),
        ];
        let totals = aggregate_by_project(&sessions, OFF);

        assert_eq!(totals["a"].billable_seconds, 5_400);
        assert_eq!(totals["a"].amount_cents, 15_000);
        assert_eq!(totals["b"].amount_cents, 5_000);
    }

    #[test]
    fn projeto_sem_sessao_fica_ausente() {
        let totals = aggregate_by_project(&[], OFF);
        assert!(!totals.contains_key("a"));
    }

    #[test]
    fn sessao_nao_faturavel_soma_horas_mas_nao_valor() {
        let sessions = vec![session("a", 3_600, 0, false, 10_000)];
        let totals = aggregate_by_project(&sessions, OFF);

        assert_eq!(totals["a"].gross_seconds, 3_600);
        assert_eq!(totals["a"].billable_seconds, 0);
        assert_eq!(totals["a"].amount_cents, 0);
    }

    #[test]
    fn inatividade_e_descontada_do_faturavel_mas_nao_do_bruto() {
        // 1h com 10min inativos, a R$ 100,00/h -> cobra 50min = R$ 83,33.
        let sessions = vec![session("a", 3_600, 600, true, 10_000)];
        let totals = aggregate_by_project(&sessions, OFF);

        assert_eq!(totals["a"].gross_seconds, 3_600);
        assert_eq!(totals["a"].idle_seconds, 600);
        assert_eq!(totals["a"].billable_seconds, 3_000);
        assert_eq!(totals["a"].amount_cents, 8_333);
    }

    #[test]
    fn arredonda_por_sessao_e_nao_sobre_a_soma() {
        // Duas sessoes de 10min. Arredondando cada uma para 15min -> 30min.
        // Arredondar a soma (20min) daria 15min. Precisa dar 30.
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
    fn arredondamento_desativado_preserva_o_tempo_real() {
        let sessions = vec![session("a", 600, 0, true, 6_000)];
        let totals = aggregate_by_project(&sessions, OFF);

        assert_eq!(totals["a"].billable_seconds, 600);
        assert_eq!(totals["a"].amount_cents, 1_000);
    }

    #[test]
    fn cada_sessao_usa_seu_proprio_snapshot_de_valor_hora() {
        // Mesmo projeto, valor/hora diferente por sessao (o projeto teve
        // reajuste): cada sessao mantem o valor da sua epoca.
        let sessions = vec![
            session("a", 3_600, 0, true, 5_000),
            session("a", 3_600, 0, true, 9_000),
        ];
        let totals = aggregate_by_project(&sessions, OFF);

        assert_eq!(totals["a"].amount_cents, 14_000);
    }
}

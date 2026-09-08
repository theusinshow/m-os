//! Repetição: a regra que diz quando um lembrete acontece de novo.
//!
//! Vive em módulo próprio, e não dentro de [`crate::attention`], por uma razão
//! só: aqui é **aritmética de calendário**, e aritmética de calendário é o tipo
//! de código que erra em silêncio. Isolá-la é o que permite testar "todo dia 31
//! num mês de 30" sem montar Reminder, repositório e relógio.
//!
//! # As duas decisões que o resto do sistema herda
//!
//! **O horário de uma repetição é LOCAL, não UTC.** "Todo dia às 08:00" quer
//! dizer oito da manhã onde a pessoa está, e não 11:00 UTC para sempre. Por isso
//! a regra guarda hora e minuto locais mais o deslocamento em que ela foi
//! criada, e converte só na saída. Guardar o instante UTC faria a repetição
//! escorregar uma hora inteira em qualquer mudança de fuso — e escorregar em
//! silêncio, que é pior.
//!
//! **O deslocamento é fixo, e isso é um limite conhecido.** O M/OS não carrega
//! base de fusos (tzdb): o `time` sem `tzdb` sabe somar deslocamentos, não sabe
//! quando um país muda de horário. O Brasil não tem horário de verão desde 2019,
//! então hoje o limite não custa nada; num país que tenha, a repetição andaria
//! uma hora nas duas viradas do ano. Está registrado em `ATTENTION-SYSTEM.md`
//! §29 e na §"Limitações" da documentação, e a saída, no dia em que doer, é
//! trocar `offset_minutes` por um identificador IANA e uma tzdb — não é remendar
//! aqui.

use serde::{Deserialize, Serialize};
use time::{Date, Duration, Month, OffsetDateTime, Time, UtcOffset, Weekday};

use crate::{CoreError, ErrorCode};

/// Até onde a busca pelo próximo dia caminha antes de desistir.
///
/// Dois anos e um pouco: cobre "todo dia 29 de fevereiro", que é o caso mais
/// esparso que o modelo aceita, com folga para um ano bissexto que ainda não
/// chegou. Um laço sem teto aqui seria um travamento à espera de uma regra
/// impossível — e regra impossível é exatamente o que uma migration mal feita
/// pode gravar.
const SEARCH_LIMIT_DAYS: u16 = 800;

/// Como a próxima ocorrência é ancorada.
///
/// A diferença é de produto e não de implementação, e ela importa:
///
/// - **Fixa**: "toda segunda às 09:00" continua sendo toda segunda, mesmo que a
///   da semana passada só tenha sido resolvida na quarta.
/// - **Por conclusão**: "limpar o computador a cada 30 dias **depois que eu
///   fizer**". Concluir no dia 8 marca o próximo para o dia 38. Adiar a
///   conclusão adia a série inteira, e é isso que se quer de manutenção.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceAnchor {
    #[default]
    Fixed,
    Completion,
}

impl RecurrenceAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Completion => "completion",
        }
    }
}

/// Qual dia do mês a regra mensal quer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MonthlyDay {
    /// "Todo dia 10". Num mês que não tem o dia pedido — dia 31 em abril —, cai
    /// no ÚLTIMO dia do mês, e não pula o mês.
    ///
    /// Pular seria a leitura literal e a resposta errada: quem pede "todo dia
    /// 31" quer dizer "no fim do mês", e um sistema que some em abril, junho,
    /// setembro e novembro é um sistema que não se pode usar para conta a pagar.
    Day { day: u8 },
    /// "Primeira segunda do mês". `ordinal` 1..=4; 5 significa a ÚLTIMA.
    Nth { weekday: u8, ordinal: u8 },
    /// O último dia útil do mês — segunda a sexta. Feriado não entra: o M/OS
    /// tem calendário de feriados (`feriados.rs`), mas ele é do Brasil e de um
    /// ano por vez, e uma repetição que dependesse dele mudaria de resposta
    /// conforme a tabela envelhecesse.
    LastBusinessDay,
}

/// A forma de repetir.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RecurrenceRule {
    /// Todo dia.
    Daily,
    /// Todo dia útil — segunda a sexta.
    Weekdays,
    /// Nos dias da semana escolhidos. `0` é segunda, `6` é domingo.
    Weekly {
        days: Vec<u8>,
    },
    Monthly {
        day: MonthlyDay,
    },
    Yearly {
        month: u8,
        day: u8,
    },
    /// A cada N dias. É a forma que "a cada 30 dias depois de concluir" usa.
    EveryDays {
        days: u16,
    },
    /// A cada N semanas, no mesmo dia da semana da ocorrência anterior.
    EveryWeeks {
        weeks: u16,
    },
}

/// A regra completa: forma, âncora e o horário local em que ela acontece.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    pub rule: RecurrenceRule,
    #[serde(default)]
    pub anchor: RecurrenceAnchor,
    /// Hora local, 0..=23.
    pub hour: u8,
    /// Minuto local, 0..=59.
    pub minute: u8,
    /// O deslocamento em relação ao UTC, em minutos, em que a regra foi escrita.
    /// `-180` é o horário de Brasília.
    pub offset_minutes: i16,
}

impl Recurrence {
    /// Monta uma regra a partir de um instante — a hora dele vira a hora dela.
    ///
    /// É o caminho que a interface usa: a pessoa escolheu "toda segunda às 9h"
    /// escolhendo uma segunda às 9h, e a regra nasce dali. Pedir hora de novo
    /// seria pedir duas vezes a mesma coisa.
    pub fn from_instant(
        rule: RecurrenceRule,
        anchor: RecurrenceAnchor,
        local: OffsetDateTime,
    ) -> Self {
        Self {
            rule,
            anchor,
            hour: local.hour(),
            minute: local.minute(),
            offset_minutes: (local.offset().whole_seconds() / 60) as i16,
        }
    }

    /// Recusa regras que nenhum calendário pode satisfazer.
    ///
    /// Chamada ao criar e ao ler do banco. Uma regra inválida gravada é um
    /// lembrete que para de repetir sem ninguém perceber — e "sem ninguém
    /// perceber" é a única falha que este sistema inteiro existe para não ter.
    pub fn validate(&self) -> Result<(), CoreError> {
        let refuse = |mensagem: &str| {
            Err(CoreError::new(
                ErrorCode::InvalidInput,
                mensagem.to_owned(),
                false,
            ))
        };

        if self.hour > 23 || self.minute > 59 {
            return refuse("Horario de repeticao invalido.");
        }
        if self.offset_minutes < -(14 * 60) || self.offset_minutes > 14 * 60 {
            return refuse("Fuso de repeticao invalido.");
        }

        match &self.rule {
            RecurrenceRule::Weekly { days } => {
                if days.is_empty() {
                    return refuse("Uma repeticao semanal precisa de ao menos um dia.");
                }
                if days.iter().any(|day| *day > 6) {
                    return refuse("Dia da semana invalido.");
                }
            }
            RecurrenceRule::Monthly { day } => match day {
                MonthlyDay::Day { day } => {
                    if *day == 0 || *day > 31 {
                        return refuse("Dia do mes invalido.");
                    }
                }
                MonthlyDay::Nth { weekday, ordinal } => {
                    if *weekday > 6 {
                        return refuse("Dia da semana invalido.");
                    }
                    if *ordinal == 0 || *ordinal > 5 {
                        return refuse("Ordinal invalido: use de 1 a 5, onde 5 e o ultimo.");
                    }
                }
                MonthlyDay::LastBusinessDay => {}
            },
            RecurrenceRule::Yearly { month, day } => {
                if *month == 0 || *month > 12 || *day == 0 || *day > 31 {
                    return refuse("Data anual invalida.");
                }
            }
            RecurrenceRule::EveryDays { days } => {
                if *days == 0 {
                    return refuse("Repetir a cada zero dias nao repete nada.");
                }
            }
            RecurrenceRule::EveryWeeks { weeks } => {
                if *weeks == 0 {
                    return refuse("Repetir a cada zero semanas nao repete nada.");
                }
            }
            RecurrenceRule::Daily | RecurrenceRule::Weekdays => {}
        }
        Ok(())
    }

    fn offset(&self) -> UtcOffset {
        UtcOffset::from_whole_seconds(i32::from(self.offset_minutes) * 60).unwrap_or(UtcOffset::UTC)
    }

    fn time_of_day(&self) -> Time {
        Time::from_hms(self.hour, self.minute, 0).unwrap_or(Time::MIDNIGHT)
    }

    /// A próxima ocorrência ESTRITAMENTE depois de `from`.
    ///
    /// Estritamente: senão uma regra diária calculada a partir da própria
    /// ocorrência devolveria ela mesma, e o agendador entraria em laço fechado
    /// disparando o mesmo lembrete para sempre.
    ///
    /// `None` quando a regra é impossível — o que só acontece com dado
    /// corrompido, já que [`Self::validate`] roda na entrada e na leitura.
    pub fn next_after(&self, from: OffsetDateTime) -> Option<OffsetDateTime> {
        let offset = self.offset();
        let local = from.to_offset(offset);
        let time = self.time_of_day();

        // Os intervalos por contagem são ancorados na ocorrência anterior, e não
        // varridos: "a cada 30 dias" a partir de hoje é hoje + 30, e procurar o
        // "próximo dia que combina" não faria sentido nenhum aqui.
        if let Some(step) = match &self.rule {
            RecurrenceRule::EveryDays { days } => Some(Duration::days(i64::from(*days))),
            RecurrenceRule::EveryWeeks { weeks } => Some(Duration::weeks(i64::from(*weeks))),
            _ => None,
        } {
            let mut candidate = local.date().with_time(time).assume_offset(offset) + step;
            // Uma passada extra cobre o caso de a hora do dia puxar o candidato
            // para trás do próprio `from` — "a cada 1 dia" às 08:00, calculado
            // a partir das 23:00, cairia às 08:00 de amanhã, que é depois; mas
            // calculado a partir das 07:00 de amanhã com passo de zero dias
            // seria antes. O laço fecha a porta sem depender do caso.
            while candidate <= from {
                candidate += step;
            }
            return Some(candidate);
        }

        let mut date = local.date();
        for _ in 0..SEARCH_LIMIT_DAYS {
            let candidate = date.with_time(time).assume_offset(offset);
            if candidate > from && self.matches(date) {
                return Some(candidate);
            }
            date = date.next_day()?;
        }
        None
    }

    /// Este dia serve à regra?
    fn matches(&self, date: Date) -> bool {
        match &self.rule {
            RecurrenceRule::Daily => true,
            RecurrenceRule::Weekdays => is_business_day(date),
            RecurrenceRule::Weekly { days } => days.contains(&weekday_index(date.weekday())),
            RecurrenceRule::Monthly { day } => matches_monthly(date, day),
            RecurrenceRule::Yearly { month, day } => {
                u8::from(date.month()) == *month && matches_month_day(date, *day)
            }
            // Tratados antes, por contagem.
            RecurrenceRule::EveryDays { .. } | RecurrenceRule::EveryWeeks { .. } => false,
        }
    }

    /// Como a regra se lê em português. A tela mostra isto, e não JSON.
    pub fn describe(&self) -> String {
        let hora = format!("{:02}:{:02}", self.hour, self.minute);
        let base = match &self.rule {
            RecurrenceRule::Daily => "Todo dia".to_owned(),
            RecurrenceRule::Weekdays => "Todo dia util".to_owned(),
            RecurrenceRule::Weekly { days } => {
                let mut ordenados = days.clone();
                ordenados.sort_unstable();
                ordenados.dedup();
                let nomes: Vec<&str> = ordenados.iter().map(|day| weekday_name(*day)).collect();
                format!("Toda {}", nomes.join(", "))
            }
            RecurrenceRule::Monthly { day } => match day {
                MonthlyDay::Day { day } => format!("Todo dia {day}"),
                MonthlyDay::Nth { weekday, ordinal } => {
                    let qual = match ordinal {
                        1 => "Primeira",
                        2 => "Segunda",
                        3 => "Terceira",
                        4 => "Quarta",
                        _ => "Ultima",
                    };
                    format!("{qual} {} do mes", weekday_name(*weekday))
                }
                MonthlyDay::LastBusinessDay => "Ultimo dia util do mes".to_owned(),
            },
            RecurrenceRule::Yearly { month, day } => format!("Todo dia {day}/{month}"),
            RecurrenceRule::EveryDays { days } => format!("A cada {days} dias"),
            RecurrenceRule::EveryWeeks { weeks } => format!("A cada {weeks} semanas"),
        };

        let sufixo = match self.anchor {
            RecurrenceAnchor::Fixed => String::new(),
            RecurrenceAnchor::Completion => ", depois de concluir".to_owned(),
        };
        format!("{base} as {hora}{sufixo}")
    }
}

/// `0` é segunda e `6` é domingo — a ordem em que a semana se lê aqui.
pub fn weekday_index(weekday: Weekday) -> u8 {
    weekday.number_days_from_monday()
}

fn weekday_name(index: u8) -> &'static str {
    match index {
        0 => "segunda",
        1 => "terca",
        2 => "quarta",
        3 => "quinta",
        4 => "sexta",
        5 => "sabado",
        _ => "domingo",
    }
}

fn is_business_day(date: Date) -> bool {
    weekday_index(date.weekday()) <= 4
}

fn days_in_month(year: i32, month: Month) -> u8 {
    time::util::days_in_month(month, year)
}

/// O dia pedido, ou o último do mês quando o mês é curto demais.
fn matches_month_day(date: Date, day: u8) -> bool {
    let last = days_in_month(date.year(), date.month());
    date.day() == day.min(last)
}

fn matches_monthly(date: Date, day: &MonthlyDay) -> bool {
    match day {
        MonthlyDay::Day { day } => matches_month_day(date, *day),
        MonthlyDay::Nth { weekday, ordinal } => {
            if weekday_index(date.weekday()) != *weekday {
                return false;
            }
            if *ordinal == 5 {
                // A última ocorrência daquele dia da semana: não existe outra
                // sete dias à frente dentro do mesmo mês.
                return date.day() + 7 > days_in_month(date.year(), date.month());
            }
            // A n-ésima: dia 1..7 é a primeira, 8..14 a segunda, e assim por diante.
            u32::from(date.day()).div_ceil(7) == u32::from(*ordinal)
        }
        MonthlyDay::LastBusinessDay => {
            if !is_business_day(date) {
                return false;
            }
            let last = days_in_month(date.year(), date.month());
            // Nenhum outro dia útil depois deste, dentro do mês.
            let mut cursor = date;
            while cursor.day() < last {
                let Some(next) = cursor.next_day() else {
                    return true;
                };
                if next.month() != date.month() {
                    return true;
                }
                if is_business_day(next) {
                    return false;
                }
                cursor = next;
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn brasilia() -> UtcOffset {
        UtcOffset::from_hms(-3, 0, 0).unwrap()
    }

    fn regra(rule: RecurrenceRule) -> Recurrence {
        Recurrence {
            rule,
            anchor: RecurrenceAnchor::Fixed,
            hour: 8,
            minute: 0,
            offset_minutes: -180,
        }
    }

    /// O caso do pedido: "todo dia às 08:00" continua às 08:00 LOCAIS.
    #[test]
    fn a_daily_rule_keeps_the_local_hour() {
        let recurrence = regra(RecurrenceRule::Daily);
        // 2026-09-08 12:00Z = 09:00 em Brasília — as 08:00 de hoje já passaram.
        let next = recurrence
            .next_after(datetime!(2026-09-08 12:00 UTC))
            .unwrap();
        let local = next.to_offset(brasilia());
        assert_eq!(local.hour(), 8);
        assert_eq!(local.minute(), 0);
        assert_eq!(local.date(), time::macros::date!(2026 - 09 - 09));
    }

    /// Estritamente depois: senão o agendador dispara o mesmo instante para sempre.
    #[test]
    fn the_next_occurrence_is_strictly_after() {
        let recurrence = regra(RecurrenceRule::Daily);
        let exactly = datetime!(2026-09-08 11:00 UTC); // 08:00 em Brasília
        let next = recurrence.next_after(exactly).unwrap();
        assert!(next > exactly);
        assert_eq!(next, datetime!(2026-09-09 11:00 UTC));
    }

    #[test]
    fn weekdays_skip_the_weekend() {
        let recurrence = regra(RecurrenceRule::Weekdays);
        // 2026-09-11 é uma sexta. A próxima é segunda, dia 14.
        let next = recurrence
            .next_after(datetime!(2026-09-11 12:00 UTC))
            .unwrap();
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2026 - 09 - 14)
        );
    }

    #[test]
    fn a_weekly_rule_takes_the_chosen_days() {
        // Segunda e quinta.
        let recurrence = regra(RecurrenceRule::Weekly { days: vec![0, 3] });
        // 2026-09-08 é terça.
        let next = recurrence
            .next_after(datetime!(2026-09-08 12:00 UTC))
            .unwrap();
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2026 - 09 - 10)
        );
    }

    /// Dia 31 num mês de 30 cai no dia 30, e não pula o mês.
    ///
    /// Pular seria a leitura literal e a resposta errada: quem pede "todo dia
    /// 31" quer dizer fim do mês, e sumir em quatro meses do ano faria disto um
    /// lembrete inútil para conta a pagar.
    #[test]
    fn a_monthly_day_falls_back_to_the_last_day_of_a_short_month() {
        let recurrence = regra(RecurrenceRule::Monthly {
            day: MonthlyDay::Day { day: 31 },
        });
        // De 31 de março para abril, que tem 30.
        let next = recurrence
            .next_after(datetime!(2026-03-31 12:00 UTC))
            .unwrap();
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2026 - 04 - 30)
        );
    }

    #[test]
    fn february_gets_the_29th_when_it_has_one() {
        let recurrence = regra(RecurrenceRule::Monthly {
            day: MonthlyDay::Day { day: 31 },
        });
        let next = recurrence
            .next_after(datetime!(2024-01-31 12:00 UTC))
            .unwrap();
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2024 - 02 - 29)
        );
    }

    #[test]
    fn the_first_monday_of_the_month() {
        let recurrence = regra(RecurrenceRule::Monthly {
            day: MonthlyDay::Nth {
                weekday: 0,
                ordinal: 1,
            },
        });
        let next = recurrence
            .next_after(datetime!(2026-09-08 12:00 UTC))
            .unwrap();
        // A primeira segunda de outubro de 2026 é dia 5.
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2026 - 10 - 05)
        );
    }

    #[test]
    fn the_last_friday_of_the_month() {
        let recurrence = regra(RecurrenceRule::Monthly {
            day: MonthlyDay::Nth {
                weekday: 4,
                ordinal: 5,
            },
        });
        let next = recurrence
            .next_after(datetime!(2026-09-01 12:00 UTC))
            .unwrap();
        // A última sexta de setembro de 2026 é dia 25.
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2026 - 09 - 25)
        );
    }

    /// Último dia útil: 31 de maio de 2026 é um domingo, então é a sexta, dia 29.
    #[test]
    fn the_last_business_day_walks_back_over_the_weekend() {
        let recurrence = regra(RecurrenceRule::Monthly {
            day: MonthlyDay::LastBusinessDay,
        });
        let next = recurrence
            .next_after(datetime!(2026-05-02 12:00 UTC))
            .unwrap();
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2026 - 05 - 29)
        );
    }

    #[test]
    fn every_n_days_counts_from_the_anchor() {
        let recurrence = Recurrence {
            anchor: RecurrenceAnchor::Completion,
            ..regra(RecurrenceRule::EveryDays { days: 30 })
        };
        // Concluído no dia 8 às 14h locais.
        let next = recurrence
            .next_after(datetime!(2026-09-08 17:00 UTC))
            .unwrap();
        let local = next.to_offset(brasilia());
        assert_eq!(local.date(), time::macros::date!(2026 - 10 - 08));
        assert_eq!(local.hour(), 8);
    }

    #[test]
    fn a_yearly_rule_returns_next_year() {
        let recurrence = regra(RecurrenceRule::Yearly { month: 3, day: 15 });
        let next = recurrence
            .next_after(datetime!(2026-09-08 12:00 UTC))
            .unwrap();
        assert_eq!(
            next.to_offset(brasilia()).date(),
            time::macros::date!(2027 - 03 - 15)
        );
    }

    #[test]
    fn an_impossible_rule_is_refused_before_it_is_stored() {
        assert!(regra(RecurrenceRule::Weekly { days: vec![] })
            .validate()
            .is_err());
        assert!(regra(RecurrenceRule::Weekly { days: vec![9] })
            .validate()
            .is_err());
        assert!(regra(RecurrenceRule::EveryDays { days: 0 })
            .validate()
            .is_err());
        assert!(regra(RecurrenceRule::Monthly {
            day: MonthlyDay::Day { day: 0 }
        })
        .validate()
        .is_err());
        assert!(regra(RecurrenceRule::Daily).validate().is_ok());
    }

    #[test]
    fn an_invalid_hour_is_refused() {
        let mut recurrence = regra(RecurrenceRule::Daily);
        recurrence.hour = 25;
        assert!(recurrence.validate().is_err());
    }

    /// A regra nasce do instante que a pessoa escolheu — não se pergunta a hora
    /// duas vezes.
    #[test]
    fn a_rule_born_from_an_instant_keeps_its_hour() {
        let local = datetime!(2026-09-08 09:30 -3);
        let recurrence =
            Recurrence::from_instant(RecurrenceRule::Daily, RecurrenceAnchor::Fixed, local);
        assert_eq!(recurrence.hour, 9);
        assert_eq!(recurrence.minute, 30);
        assert_eq!(recurrence.offset_minutes, -180);
    }

    #[test]
    fn the_rule_reads_in_portuguese() {
        assert_eq!(regra(RecurrenceRule::Daily).describe(), "Todo dia as 08:00");
        assert_eq!(
            regra(RecurrenceRule::Weekdays).describe(),
            "Todo dia util as 08:00"
        );
        assert!(Recurrence {
            anchor: RecurrenceAnchor::Completion,
            ..regra(RecurrenceRule::EveryDays { days: 30 })
        }
        .describe()
        .ends_with("depois de concluir"));
    }

    /// A ida e volta pelo JSON precisa preservar a regra: é assim que ela vive
    /// no banco, e uma regra que muda de significado ao ser lida é um lembrete
    /// que repete errado sem ninguém ver.
    #[test]
    fn a_rule_survives_the_round_trip_through_json() {
        for rule in [
            RecurrenceRule::Daily,
            RecurrenceRule::Weekdays,
            RecurrenceRule::Weekly {
                days: vec![0, 2, 4],
            },
            RecurrenceRule::Monthly {
                day: MonthlyDay::Day { day: 10 },
            },
            RecurrenceRule::Monthly {
                day: MonthlyDay::LastBusinessDay,
            },
            RecurrenceRule::Monthly {
                day: MonthlyDay::Nth {
                    weekday: 1,
                    ordinal: 3,
                },
            },
            RecurrenceRule::Yearly { month: 12, day: 25 },
            RecurrenceRule::EveryDays { days: 30 },
            RecurrenceRule::EveryWeeks { weeks: 2 },
        ] {
            let original = regra(rule);
            let json = serde_json::to_string(&original).unwrap();
            let voltou: Recurrence = serde_json::from_str(&json).unwrap();
            assert_eq!(original, voltou, "regra nao sobreviveu ao JSON: {json}");
        }
    }
}

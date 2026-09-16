//! Prazos ditos em voz alta, virando data.
//!
//! "Mando amanhã", "até sexta", "dia 20". A V1 guardava so a palavra
//! (`due_hint`) e deixava a interpretacao para a tela, que sugeria sempre
//! "amanha as 9h" — qualquer que fosse a palavra. Com `Task.due_at` de volta
//! (ADR-066), o prazo precisa virar instante, e a interpretacao precisa ser a
//! mesma em todo lugar.
//!
//! # As regras
//!
//! - **deterministico.** Nada de modelo: "sexta" numa reuniao de quarta e a
//!   sexta desta semana, sempre;
//! - **a referencia e o inicio da reuniao**, e nao o momento da analise. Uma
//!   reuniao de sexta analisada no sabado nao pode transformar "amanha" em
//!   domingo;
//! - **no fuso de quem gravou.** O instante chega com o offset certo;
//! - **o original nunca some.** O que nao resolve continua como texto, e o que
//!   resolve guarda a expressao ao lado.
//! - **18:00 por padrao.** Prazo sem hora vence no fim do expediente, e nao a
//!   meia-noite — "manda amanha" as 00:00 de amanha ja estaria atrasado.

use serde::{Deserialize, Serialize};
use time::{Date, Duration, Month, OffsetDateTime, Time, Weekday};

use crate::Confidence;

/// Um prazo interpretado.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDue {
    pub original_expression: String,
    #[serde(with = "time::serde::rfc3339")]
    pub resolved_at: OffsetDateTime,
    pub confidence: Confidence,
}

/// A hora padrao de um prazo sem hora.
pub const DEFAULT_DUE_HOUR: u8 = 18;

/// Sem acento e em minuscula. Portugues falado chega do whisper com e sem
/// acento, e "amanhã" e "amanha" sao a mesma palavra.
pub fn fold(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn weekday_of(word: &str) -> Option<Weekday> {
    let word = word.trim_end_matches("-feira").trim_end_matches(" feira");
    match word {
        "segunda" => Some(Weekday::Monday),
        "terca" => Some(Weekday::Tuesday),
        "quarta" => Some(Weekday::Wednesday),
        "quinta" => Some(Weekday::Thursday),
        "sexta" => Some(Weekday::Friday),
        "sabado" => Some(Weekday::Saturday),
        "domingo" => Some(Weekday::Sunday),
        _ => None,
    }
}

fn number_of(word: &str) -> Option<i64> {
    if let Ok(value) = word.parse::<i64>() {
        return Some(value);
    }
    Some(match word {
        "um" | "uma" => 1,
        "dois" | "duas" => 2,
        "tres" => 3,
        "quatro" => 4,
        "cinco" => 5,
        "seis" => 6,
        "sete" => 7,
        "oito" => 8,
        "nove" => 9,
        "dez" => 10,
        "quinze" => 15,
        "vinte" => 20,
        "trinta" => 30,
        _ => return None,
    })
}

fn days_until(from: Weekday, to: Weekday) -> i64 {
    let from = from.number_days_from_monday() as i64;
    let to = to.number_days_from_monday() as i64;
    (to - from).rem_euclid(7)
}

fn last_day_of_month(date: Date) -> Date {
    let days = date.month().length(date.year());
    Date::from_calendar_date(date.year(), date.month(), days).unwrap_or(date)
}

fn at_hour(date: Date, reference: OffsetDateTime, hour: u8, minute: u8) -> OffsetDateTime {
    let time = Time::from_hms(hour.min(23), minute.min(59), 0).unwrap_or(Time::MIDNIGHT);
    date.with_time(time).assume_offset(reference.offset())
}

/// Hora dita junto, se houver: "as 14h", "14:30", "10 horas".
fn spoken_hour(tokens: &[&str]) -> Option<(u8, u8)> {
    for (index, token) in tokens.iter().enumerate() {
        let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != ':');
        if let Some((h, m)) = token.split_once(':') {
            if let (Ok(h), Ok(m)) = (h.parse::<u8>(), m.parse::<u8>()) {
                if h < 24 && m < 60 {
                    return Some((h, m));
                }
            }
        }
        if let Some(h) = token.strip_suffix('h') {
            let (h, m) = h.split_once('h').unwrap_or((h, "0"));
            if let (Ok(h), Ok(m)) = (h.parse::<u8>(), m.parse::<u8>()) {
                if h < 24 && m < 60 {
                    return Some((h, m));
                }
            }
        }
        if let Ok(h) = token.parse::<u8>() {
            let next = tokens.get(index + 1).copied();
            if h < 24 && matches!(next, Some("horas" | "hora")) {
                return Some((h, 0));
            }
        }
    }
    None
}

/// Interpreta uma expressao de prazo.
///
/// `reference` e o inicio da reuniao, JA no offset local de quem gravou.
pub fn resolve_due(expression: &str, reference: OffsetDateTime) -> Option<ResolvedDue> {
    let folded = fold(expression);
    let cleaned: String = folded
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '/' || c == ':' || c == '-' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let today = reference.date();
    let phrase = format!(" {} ", tokens.join(" "));
    let has = |needle: &str| phrase.contains(&format!(" {needle} "));

    let (date, confidence) = resolve_date(&tokens, &phrase, today, &has)?;
    let (hour, minute) = spoken_hour(&tokens).unwrap_or((DEFAULT_DUE_HOUR, 0));
    Some(ResolvedDue {
        original_expression: expression.trim().to_owned(),
        resolved_at: at_hour(date, reference, hour, minute),
        confidence,
    })
}

fn resolve_date(
    tokens: &[&str],
    phrase: &str,
    today: Date,
    has: &dyn Fn(&str) -> bool,
) -> Option<(Date, Confidence)> {
    // A ordem importa: "depois de amanha" contem "amanha".
    if has("depois de amanha") {
        return Some((today + Duration::days(2), Confidence::High));
    }
    if has("amanha") {
        return Some((today + Duration::days(1), Confidence::High));
    }
    if has("hoje") || has("ainda hoje") || has("agora a tarde") {
        return Some((today, Confidence::High));
    }

    // dd/mm ou dd/mm/aaaa
    for token in tokens {
        let parts: Vec<&str> = token.split('/').collect();
        if parts.len() == 2 || parts.len() == 3 {
            let Ok(day) = parts[0].parse::<u8>() else {
                continue;
            };
            let month = parts[1]
                .parse::<u8>()
                .ok()
                .and_then(|m| Month::try_from(m).ok());
            let Some(month) = month else { continue };
            let year = match parts.get(2) {
                Some(y) => {
                    let Ok(y) = y.parse::<i32>() else { continue };
                    if y < 100 {
                        2000 + y
                    } else {
                        y
                    }
                }
                None => today.year(),
            };
            let Ok(mut date) = Date::from_calendar_date(year, month, day) else {
                continue;
            };
            // Sem ano e ja passado ha mais de um mes: e o ano que vem. "20/01"
            // dito em dezembro nao e janeiro passado.
            if parts.len() == 2 && date < today - Duration::days(31) {
                if let Ok(next) = Date::from_calendar_date(year + 1, month, day) {
                    date = next;
                }
            }
            return Some((date, Confidence::High));
        }
    }

    // "dia 20"
    for window in tokens.windows(2) {
        if window[0] == "dia" {
            if let Ok(day) = window[1].parse::<u8>() {
                if (1..=31).contains(&day) {
                    let this_month =
                        Date::from_calendar_date(today.year(), today.month(), day).ok();
                    return match this_month {
                        Some(date) if date >= today => Some((date, Confidence::High)),
                        _ => {
                            let next = last_day_of_month(today) + Duration::days(1);
                            Date::from_calendar_date(next.year(), next.month(), day)
                                .ok()
                                .map(|date| (date, Confidence::Medium))
                        }
                    };
                }
            }
        }
    }

    // "em 3 dias", "daqui a duas semanas"
    for (index, token) in tokens.iter().enumerate() {
        let Some(unit) = tokens.get(index + 1) else {
            continue;
        };
        let Some(amount) = number_of(token) else {
            continue;
        };
        let previous = index.checked_sub(1).map(|i| tokens[i]);
        let introduced = matches!(previous, Some("em" | "a" | "daqui" | "proximos" | "uns"));
        if !introduced || amount <= 0 || amount > 365 {
            continue;
        }
        match *unit {
            "dia" | "dias" => {
                return Some((today + Duration::days(amount), Confidence::High));
            }
            "semana" | "semanas" => {
                return Some((today + Duration::days(7 * amount), Confidence::Medium));
            }
            _ => {}
        }
    }

    if has("fim do mes") || has("final do mes") {
        return Some((last_day_of_month(today), Confidence::Medium));
    }
    if has("fim de semana") || has("final de semana") {
        let days = days_until(today.weekday(), Weekday::Saturday);
        let days = if days == 0 { 7 } else { days };
        return Some((today + Duration::days(days), Confidence::Medium));
    }
    if has("semana que vem") || has("proxima semana") {
        let days = days_until(today.weekday(), Weekday::Monday);
        let days = if days == 0 { 7 } else { days };
        return Some((today + Duration::days(days), Confidence::Medium));
    }
    if has("essa semana") || has("esta semana") || has("ate o fim da semana") {
        let days = days_until(today.weekday(), Weekday::Friday);
        return Some((today + Duration::days(days), Confidence::Medium));
    }

    // Dias da semana: "sexta", "ate segunda", "quinta que vem", "proxima terca".
    for (index, token) in tokens.iter().enumerate() {
        let Some(weekday) = weekday_of(token) else {
            continue;
        };
        let next_week = phrase.contains(&format!(" {token} que vem "))
            || tokens.get(index + 1).is_some_and(|t| *t == "que")
                && tokens.get(index + 2).is_some_and(|t| *t == "vem")
            || index
                .checked_sub(1)
                .is_some_and(|i| matches!(tokens[i], "proxima" | "proximo"));
        let mut days = days_until(today.weekday(), weekday);
        let mut confidence = Confidence::High;
        if days == 0 {
            // "Sexta" dito numa sexta: quase sempre a proxima. Mas pode ser hoje
            // — e e isso que a confianca media diz.
            days = 7;
            confidence = Confidence::Medium;
        }
        if next_week && days < 7 {
            days += 7;
            // "Quinta que vem" dito na segunda e ambiguo em portugues falado:
            // uns entendem a desta semana, outros a da seguinte.
            confidence = Confidence::Medium;
        }
        return Some((today + Duration::days(days), confidence));
    }

    None
}

/// Procura uma expressao de prazo dentro de um texto maior.
///
/// Usado nos marcadores das notas: `!task enviar bases sexta @Victor`.
pub fn find_due_expression(text: &str) -> Option<String> {
    const FRASES: &[&str] = &[
        "depois de amanha",
        "amanha",
        "hoje",
        "fim do mes",
        "final do mes",
        "fim de semana",
        "semana que vem",
        "proxima semana",
        "essa semana",
        "esta semana",
    ];
    let folded = fold(text);
    let words: Vec<&str> = folded
        .split(|c: char| !(c.is_alphanumeric() || c == '/'))
        .filter(|w| !w.is_empty())
        .collect();
    let joined = format!(" {} ", words.join(" "));

    for frase in FRASES {
        if joined.contains(&format!(" {frase} ")) {
            let preposicao = ["ate ", "para ", "pra "]
                .iter()
                .find(|p| joined.contains(&format!(" {p}{frase} ")))
                .map(|p| p.to_string())
                .unwrap_or_default();
            return Some(format!("{preposicao}{frase}"));
        }
    }
    for (index, word) in words.iter().enumerate() {
        if weekday_of(word).is_some() {
            let que_vem =
                words.get(index + 1) == Some(&"que") && words.get(index + 2) == Some(&"vem");
            return Some(if que_vem {
                format!("{word} que vem")
            } else {
                (*word).to_owned()
            });
        }
        if word.contains('/') && word.split('/').all(|p| p.parse::<u16>().is_ok()) {
            return Some((*word).to_owned());
        }
        if *word == "dia" {
            if let Some(next) = words.get(index + 1) {
                if next.parse::<u8>().is_ok() {
                    return Some(format!("dia {next}"));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    /// Quarta-feira, 16/09/2026, 14:32 em Brasilia.
    fn quarta() -> OffsetDateTime {
        datetime!(2026-09-16 14:32 -3)
    }

    fn data(expr: &str) -> (String, Confidence) {
        let due = resolve_due(expr, quarta()).unwrap_or_else(|| panic!("nao resolveu {expr}"));
        (due.resolved_at.date().to_string(), due.confidence)
    }

    #[test]
    fn amanha_hoje_e_depois_de_amanha() {
        assert_eq!(data("amanhã"), ("2026-09-17".into(), Confidence::High));
        assert_eq!(
            data("Amanha de manhã"),
            ("2026-09-17".into(), Confidence::High)
        );
        assert_eq!(
            data("depois de amanhã"),
            ("2026-09-18".into(), Confidence::High)
        );
        assert_eq!(data("ainda hoje"), ("2026-09-16".into(), Confidence::High));
    }

    #[test]
    fn dia_da_semana_e_o_proximo() {
        assert_eq!(data("sexta"), ("2026-09-18".into(), Confidence::High));
        assert_eq!(
            data("até segunda-feira"),
            ("2026-09-21".into(), Confidence::High)
        );
        // Quarta dita numa quarta: a proxima, com duvida.
        assert_eq!(data("quarta"), ("2026-09-23".into(), Confidence::Medium));
        assert_eq!(
            data("sexta que vem"),
            ("2026-09-25".into(), Confidence::Medium)
        );
    }

    #[test]
    fn datas_numericas() {
        assert_eq!(data("dia 20"), ("2026-09-20".into(), Confidence::High));
        assert_eq!(data("dia 3"), ("2026-10-03".into(), Confidence::Medium));
        assert_eq!(data("20/09"), ("2026-09-20".into(), Confidence::High));
        assert_eq!(data("05/01"), ("2027-01-05".into(), Confidence::High));
        assert_eq!(data("30/10/2026"), ("2026-10-30".into(), Confidence::High));
    }

    #[test]
    fn intervalos() {
        assert_eq!(data("em 3 dias"), ("2026-09-19".into(), Confidence::High));
        assert_eq!(
            data("daqui a duas semanas"),
            ("2026-09-30".into(), Confidence::Medium)
        );
        assert_eq!(
            data("semana que vem"),
            ("2026-09-21".into(), Confidence::Medium)
        );
        assert_eq!(
            data("fim do mês"),
            ("2026-09-30".into(), Confidence::Medium)
        );
        assert_eq!(
            data("essa semana"),
            ("2026-09-18".into(), Confidence::Medium)
        );
    }

    #[test]
    fn a_hora_padrao_e_o_fim_do_expediente_e_a_dita_vence() {
        let due = resolve_due("amanhã", quarta()).unwrap();
        assert_eq!(due.resolved_at.hour(), 18);
        assert_eq!(due.resolved_at.offset(), quarta().offset());

        let due = resolve_due("amanhã às 10h", quarta()).unwrap();
        assert_eq!((due.resolved_at.hour(), due.resolved_at.minute()), (10, 0));
        let due = resolve_due("sexta 14:30", quarta()).unwrap();
        assert_eq!((due.resolved_at.hour(), due.resolved_at.minute()), (14, 30));
    }

    #[test]
    fn o_que_nao_e_prazo_nao_vira_data() {
        assert!(resolve_due("quando der", quarta()).is_none());
        assert!(resolve_due("logo", quarta()).is_none());
        assert!(resolve_due("", quarta()).is_none());
    }

    #[test]
    fn a_expressao_original_e_preservada() {
        let due = resolve_due("  até sexta  ", quarta()).unwrap();
        assert_eq!(due.original_expression, "até sexta");
    }

    #[test]
    fn acha_a_expressao_dentro_de_uma_frase() {
        assert_eq!(
            find_due_expression("enviar as bases até sexta @Victor").as_deref(),
            Some("sexta")
        );
        assert_eq!(
            find_due_expression("revisar prancha 04 amanhã").as_deref(),
            Some("amanha")
        );
        assert_eq!(find_due_expression("conferir IFC").as_deref(), None);
    }
}

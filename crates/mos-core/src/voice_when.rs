//! Datas naturais faladas em portugues, resolvidas para um instante.
//!
//! # Por que aqui, e nao no renderer
//!
//! `CORE-FOUNDATION.md` §5 e normativa: *"a interpretacao de datas naturais e a
//! apresentacao devem respeitar timezone e locale do usuario"*. Quem conhece o
//! fuso e a tela — o `ReminderComposer` calcula "amanha 9h" no renderer
//! justamente por isso, e o `calendar_window` recebe a janela ja como instante.
//!
//! A regra aqui obedece a mesma lei por outro caminho: **o fuso entra como
//! parametro**. `now_local` chega do renderer ja carregando o offset de quem
//! falou, e tudo o que este modulo produz herda esse offset. O banco continua
//! guardando UTC, e o core continua sem adivinhar onde a pessoa esta.
//!
//! # E por que o texto original tambem e devolvido
//!
//! "Amanha" so significa alguma coisa no dia em que foi dito. Guardar apenas o
//! instante perderia a frase; guardar apenas a frase obrigaria quem le depois a
//! reinterpretar "amanha" contra o relogio errado. Os dois, entao —
//! `ResolvedWhen` carrega o instante E o trecho falado que o produziu.

use time::{Date, Duration, OffsetDateTime, Time, Weekday};

/// A hora padrao de um dia sem hora dita.
///
/// Nove da manha, e nao meia-noite: e o mesmo default que o
/// `ReminderComposer` ja oferece em "Amanha 9h", e um lembrete que toca a
/// meia-noite e um lembrete perdido.
const DEFAULT_HOUR: u8 = 9;

const MORNING_HOUR: u8 = 9;
const AFTERNOON_HOUR: u8 = 14;
const EVENING_HOUR: u8 = 20;

/// Um instante falado, e a frase que o produziu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedWhen {
    /// O trecho COMO FOI DITO, recortado do texto original com acentos e tudo.
    pub raw: String,
    /// O instante, no mesmo offset de `now_local`.
    pub instant: OffsetDateTime,
    /// Se a hora foi dita, ou se veio do padrao.
    pub explicit_time: bool,
    /// Ancoras que nao apontam para um dia especifico — "semana que vem".
    ///
    /// Existe para a confianca: resolver e possivel, mas afirmar que a pessoa
    /// quis segunda-feira as nove seria inventar precisao que ela nao deu.
    pub vague: bool,
}

/// Uma palavra do texto, e onde ela estava.
///
/// `norm` e minuscula e sem acento; `start`/`end` sao indices de CARACTERE no
/// texto original, e nao bytes — e o que permite devolver o trecho falado com a
/// grafia que a pessoa usou.
#[derive(Clone, Debug)]
struct Token {
    norm: String,
    start: usize,
    end: usize,
}

/// O texto quebrado em palavras, com o original preservado ao lado.
#[derive(Clone, Debug)]
pub(crate) struct Spoken {
    original: Vec<char>,
    tokens: Vec<Token>,
}

impl Spoken {
    pub(crate) fn new(text: &str) -> Self {
        let original: Vec<char> = text.chars().collect();
        let mut tokens = Vec::new();
        let mut start = None;
        for (index, character) in original.iter().enumerate() {
            // Digito e letra formam palavra; o resto separa. O hifen separa de
            // proposito: "segunda-feira" vira duas palavras e "segunda" continua
            // casando, que e o que interessa.
            //
            // Os dois pontos sao a excecao, e so entre digitos: o whisper
            // escreve "9:30" para uma hora dita, e quebrar ali produziria um
            // "9" e um "30" que a leitura de hora nao consegue juntar de volta.
            let inside_clock = *character == ':'
                && index > 0
                && original[index - 1].is_ascii_digit()
                && original.get(index + 1).is_some_and(char::is_ascii_digit);
            if character.is_alphanumeric() || inside_clock {
                start.get_or_insert(index);
            } else if let Some(begin) = start.take() {
                tokens.push(Self::token(&original, begin, index));
            }
        }
        if let Some(begin) = start {
            tokens.push(Self::token(&original, begin, original.len()));
        }
        Self { original, tokens }
    }

    fn token(original: &[char], start: usize, end: usize) -> Token {
        let norm = original[start..end].iter().copied().map(fold).collect();
        Token { norm, start, end }
    }

    pub(crate) fn words(&self) -> Vec<&str> {
        self.tokens
            .iter()
            .map(|token| token.norm.as_str())
            .collect()
    }

    /// O texto inteiro normalizado, com uma palavra por espaco.
    ///
    /// E o que permite procurar expressao de varias palavras sem repetir o laco
    /// de janela em cada regra.
    pub(crate) fn normalized(&self) -> String {
        self.words().join(" ")
    }

    /// O trecho original entre duas palavras, inclusive.
    fn slice(&self, first: usize, last: usize) -> String {
        let start = self.tokens[first].start;
        let end = self.tokens[last].end;
        self.original[start..end].iter().collect()
    }

    /// Onde a sequencia de palavras comeca, se ela existir.
    fn find(&self, needle: &[&str]) -> Option<usize> {
        if needle.is_empty() || needle.len() > self.tokens.len() {
            return None;
        }
        (0..=self.tokens.len() - needle.len()).find(|&index| {
            (0..needle.len()).all(|step| self.tokens[index + step].norm == needle[step])
        })
    }
}

/// Minuscula e sem acento, um caractere por caractere.
///
/// A troca e 1:1 de proposito: `á` vira `a`, e nao some. Manter o comprimento
/// e o que deixa os indices do texto normalizado servirem para recortar o
/// original.
pub(crate) fn fold(character: char) -> char {
    let lower = character.to_lowercase().next().unwrap_or(character);
    match lower {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        outro => outro,
    }
}

/// Normaliza um texto inteiro. Atalho para quem so quer comparar.
pub(crate) fn fold_text(text: &str) -> String {
    text.chars().map(fold).collect()
}

/// Numeros por extenso, ate trinta e um — o suficiente para hora e dia do mes.
fn spelled_number(word: &str) -> Option<u8> {
    let value = match word {
        "zero" => 0,
        "uma" | "um" => 1,
        "duas" | "dois" => 2,
        "tres" => 3,
        "quatro" => 4,
        "cinco" => 5,
        "seis" => 6,
        "sete" => 7,
        "oito" => 8,
        "nove" => 9,
        "dez" => 10,
        "onze" => 11,
        "doze" => 12,
        "treze" => 13,
        "quatorze" | "catorze" => 14,
        "quinze" => 15,
        "dezesseis" => 16,
        "dezessete" => 17,
        "dezoito" => 18,
        "dezenove" => 19,
        "vinte" => 20,
        "trinta" => 30,
        _ => return None,
    };
    Some(value)
}

/// Um numero dito de qualquer jeito: `9`, `nove`, `09`.
fn number_at(spoken: &Spoken, index: usize) -> Option<u8> {
    let token = spoken.tokens.get(index)?;
    if let Ok(value) = token.norm.parse::<u8>() {
        return Some(value);
    }
    spelled_number(&token.norm)
}

fn weekday_of(word: &str) -> Option<Weekday> {
    let day = match word {
        "segunda" => Weekday::Monday,
        "terca" => Weekday::Tuesday,
        "quarta" => Weekday::Wednesday,
        "quinta" => Weekday::Thursday,
        "sexta" => Weekday::Friday,
        "sabado" => Weekday::Saturday,
        "domingo" => Weekday::Sunday,
        _ => return None,
    };
    Some(day)
}

/// A ancora de dia que a frase deu, se deu alguma.
struct DayAnchor {
    date: Date,
    first: usize,
    last: usize,
    vague: bool,
}

/// A hora que a frase deu, se deu alguma.
struct TimeAnchor {
    time: Time,
    first: usize,
    last: usize,
}

/// Le a frase e devolve o instante que ela pede.
///
/// `None` quando nao ha tempo nenhum na frase — que e o caso comum e nao e
/// falha: "comprar cafe" nao tem quando, e forcar um seria inventar prazo.
pub fn resolve_when(text: &str, now_local: OffsetDateTime) -> Option<ResolvedWhen> {
    let spoken = Spoken::new(text);
    if spoken.tokens.is_empty() {
        return None;
    }

    // A duracao relativa e resolvida primeiro e sozinha: "daqui a duas horas"
    // ja carrega dia e hora juntos, e deixa-la disputar com as outras ancoras
    // faria "duas" virar dia do mes.
    if let Some(resolved) = relative_duration(&spoken, now_local) {
        return Some(resolved);
    }

    let day = day_anchor(&spoken, now_local);
    let time = time_anchor(&spoken);

    let (date, explicit_time, vague) = match (&day, &time) {
        (None, None) => return None,
        (Some(day), _) => (day.date, time.is_some(), day.vague),
        // Hora sem dia: hoje, se ainda nao passou; amanha, se passou. Pedir
        // "as nove" as dez da noite quer dizer amanha, e nao daqui a onze horas
        // atras.
        (None, Some(anchor)) => {
            let today = now_local.date();
            let candidate = OffsetDateTime::new_in_offset(today, anchor.time, now_local.offset());
            let date = if candidate > now_local {
                today
            } else {
                today.next_day()?
            };
            (date, true, false)
        }
    };

    let clock = time
        .as_ref()
        .map(|anchor| anchor.time)
        .unwrap_or(Time::from_hms(DEFAULT_HOUR, 0, 0).ok()?);
    let instant = OffsetDateTime::new_in_offset(date, clock, now_local.offset());

    let mut first = usize::MAX;
    let mut last = 0usize;
    for (begin, end) in [
        day.as_ref().map(|anchor| (anchor.first, anchor.last)),
        time.as_ref().map(|anchor| (anchor.first, anchor.last)),
    ]
    .into_iter()
    .flatten()
    {
        first = first.min(begin);
        last = last.max(end);
    }

    Some(ResolvedWhen {
        raw: spoken.slice(first, last),
        instant,
        explicit_time,
        vague,
    })
}

/// `daqui a duas horas`, `em 30 minutos`, `daqui a tres dias`.
fn relative_duration(spoken: &Spoken, now_local: OffsetDateTime) -> Option<ResolvedWhen> {
    let words = spoken.words();
    for index in 0..words.len() {
        // Duas aberturas: "daqui a X" e "em X". A segunda e ambigua sozinha —
        // "em casa" nao e tempo —, e por isso ela exige a unidade logo adiante.
        let (number_at_index, first) = if words[index] == "daqui" {
            let mut cursor = index + 1;
            if words.get(cursor) == Some(&"a") || words.get(cursor) == Some(&"ha") {
                cursor += 1;
            }
            (cursor, index)
        } else if words[index] == "em" {
            (index + 1, index)
        } else {
            continue;
        };

        let Some(amount) = number_at(spoken, number_at_index) else {
            continue;
        };
        let unit_index = number_at_index + 1;
        let Some(unit) = words.get(unit_index) else {
            continue;
        };
        let step = match *unit {
            "minuto" | "minutos" | "min" => Duration::minutes(amount as i64),
            "hora" | "horas" => Duration::hours(amount as i64),
            "dia" | "dias" => Duration::days(amount as i64),
            "semana" | "semanas" => Duration::weeks(amount as i64),
            _ => continue,
        };
        return Some(ResolvedWhen {
            raw: spoken.slice(first, unit_index),
            instant: now_local + step,
            explicit_time: true,
            vague: false,
        });
    }
    None
}

fn day_anchor(spoken: &Spoken, now_local: OffsetDateTime) -> Option<DayAnchor> {
    let today = now_local.date();
    let words = spoken.words();

    if let Some(index) = spoken.find(&["depois", "de", "amanha"]) {
        return Some(DayAnchor {
            date: today.next_day()?.next_day()?,
            first: index,
            last: index + 2,
            vague: false,
        });
    }
    if let Some(index) = spoken.find(&["semana", "que", "vem"]) {
        // A proxima segunda. Resolver e possivel; afirmar que a pessoa quis
        // segunda as nove e que nao seria — dai o `vague`.
        return Some(DayAnchor {
            date: next_weekday(today, Weekday::Monday),
            first: index,
            last: index + 2,
            vague: true,
        });
    }
    for (index, word) in words.iter().enumerate() {
        match *word {
            "hoje" => {
                return Some(DayAnchor {
                    date: today,
                    first: index,
                    last: index,
                    vague: false,
                })
            }
            "amanha" => {
                return Some(DayAnchor {
                    date: today.next_day()?,
                    first: index,
                    last: index,
                    vague: false,
                })
            }
            _ => {}
        }
        if let Some(weekday) = weekday_of(word) {
            // "sexta-feira" e "sexta" apontam para o mesmo dia; o sufixo entra
            // no trecho falado quando existe, para o recibo ler como a pessoa
            // disse.
            let last = if words.get(index + 1) == Some(&"feira") {
                index + 1
            } else {
                index
            };
            return Some(DayAnchor {
                date: next_weekday(today, weekday),
                first: index,
                last,
                vague: false,
            });
        }
        // "dia 25". Exige o marcador: um numero solto e hora, quantidade ou
        // codigo de Project com muito mais frequencia do que dia do mes.
        if *word == "dia" {
            if let Some(number) = number_at(spoken, index + 1) {
                if let Some(date) = day_of_month(today, number) {
                    return Some(DayAnchor {
                        date,
                        first: index,
                        last: index + 1,
                        vague: false,
                    });
                }
            }
        }
    }
    None
}

fn time_anchor(spoken: &Spoken) -> Option<TimeAnchor> {
    let words = spoken.words();

    if let Some(index) = spoken.find(&["meio", "dia"]) {
        return Some(TimeAnchor {
            time: Time::from_hms(12, 0, 0).ok()?,
            first: index,
            last: index + 1,
        });
    }

    for (index, word) in words.iter().enumerate() {
        // `9h`, `9h30`, `21h`, `09:00`.
        if let Some(anchor) = compact_hour(word, index) {
            return Some(anchor);
        }

        if *word != "as" && *word != "a" && *word != "ate" {
            continue;
        }
        let Some(hour) = number_at(spoken, index + 1) else {
            continue;
        };
        if hour > 23 {
            continue;
        }
        let mut last = index + 1;
        let mut minute = 0u8;
        // "e meia", "e trinta", "e quinze".
        if words.get(last + 1) == Some(&"e") {
            if words.get(last + 2) == Some(&"meia") {
                minute = 30;
                last += 2;
            } else if let Some(value) = number_at(spoken, last + 2) {
                if value < 60 {
                    minute = value;
                    last += 2;
                }
            }
        }
        let hour = disambiguate_hour(hour, &words, last);
        return Some(TimeAnchor {
            time: Time::from_hms(hour, minute, 0).ok()?,
            first: index,
            last,
        });
    }

    // Periodo do dia sem hora: "de manha", "a tarde", "de noite".
    for (index, word) in words.iter().enumerate() {
        let hour = match *word {
            "manha" => MORNING_HOUR,
            "tarde" => AFTERNOON_HOUR,
            "noite" => EVENING_HOUR,
            _ => continue,
        };
        return Some(TimeAnchor {
            time: Time::from_hms(hour, 0, 0).ok()?,
            first: index,
            last: index,
        });
    }
    None
}

/// `9h`, `9h30`, `21h00`, `09:00` — as formas escritas que a transcricao produz.
fn compact_hour(word: &str, index: usize) -> Option<TimeAnchor> {
    let (hour_text, minute_text) = word.split_once('h').or_else(|| word.split_once(':'))?;
    let hour: u8 = hour_text.parse().ok()?;
    if hour > 23 {
        return None;
    }
    let minute: u8 = if minute_text.is_empty() {
        0
    } else {
        minute_text.parse().ok()?
    };
    if minute > 59 {
        return None;
    }
    Some(TimeAnchor {
        time: Time::from_hms(hour, minute, 0).ok()?,
        first: index,
        last: index,
    })
}

/// "as nove da noite" sao vinte e uma horas.
///
/// Sem isto, um lembrete pedido para as nove da noite tocaria pela manha — e a
/// pessoa descobriria doze horas tarde demais.
fn disambiguate_hour(hour: u8, words: &[&str], after: usize) -> u8 {
    if hour == 0 || hour > 12 {
        return hour;
    }
    let periodo = words
        .iter()
        .skip(after + 1)
        .take(3)
        .find(|word| matches!(**word, "manha" | "tarde" | "noite"));
    match periodo.copied() {
        Some("tarde") if hour < 12 => hour + 12,
        Some("noite") if hour < 12 => hour + 12,
        _ => hour,
    }
}

/// O dia `number` mais proximo que ainda nao passou.
fn day_of_month(today: Date, number: u8) -> Option<Date> {
    if number == 0 || number > 31 {
        return None;
    }
    let this_month = Date::from_calendar_date(today.year(), today.month(), number).ok();
    match this_month {
        Some(date) if date >= today => Some(date),
        // Ja passou neste mes: e o do mes que vem. Percorrer ate doze meses
        // resolve o 31 que nao existe em fevereiro sem laco infinito.
        _ => {
            let mut year = today.year();
            let mut month = today.month();
            for _ in 0..12 {
                month = month.next();
                if month == time::Month::January {
                    year += 1;
                }
                if let Ok(date) = Date::from_calendar_date(year, month, number) {
                    return Some(date);
                }
            }
            None
        }
    }
}

/// A PROXIMA ocorrencia daquele dia da semana.
///
/// Pedir "segunda" numa segunda quer dizer a proxima, e nao agora — o mesmo
/// calculo que o `ReminderComposer` ja faz no renderer.
fn next_weekday(today: Date, weekday: Weekday) -> Date {
    let current = today.weekday().number_days_from_monday() as i64;
    let target = weekday.number_days_from_monday() as i64;
    let ahead = match (target - current).rem_euclid(7) {
        0 => 7,
        other => other,
    };
    today + Duration::days(ahead)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{macros::datetime, UtcOffset};

    /// Quarta-feira, 19 de agosto de 2026, 14h32, em Brasilia.
    fn agora() -> OffsetDateTime {
        datetime!(2026-08-19 14:32:00 -03:00)
    }

    fn resolve(frase: &str) -> ResolvedWhen {
        resolve_when(frase, agora()).expect("a frase tem tempo")
    }

    #[test]
    fn frase_sem_tempo_nao_inventa_prazo() {
        assert!(resolve_when("comprar cafe", agora()).is_none());
    }

    #[test]
    fn amanha_as_nove() {
        let quando = resolve("me lembra amanha as nove de revisar o memorial");
        assert_eq!(quando.instant, datetime!(2026-08-20 09:00:00 -03:00));
        assert!(quando.explicit_time);
        assert_eq!(quando.raw, "amanha as nove");
    }

    #[test]
    fn o_offset_de_quem_falou_e_preservado() {
        // Lisboa. O mesmo texto, outro fuso, outro instante em UTC.
        let lisboa = datetime!(2026-08-19 14:32:00 +01:00);
        let quando = resolve_when("amanha as nove", lisboa).unwrap();
        assert_eq!(
            quando.instant.offset(),
            UtcOffset::from_hms(1, 0, 0).unwrap()
        );
        assert_eq!(quando.instant, datetime!(2026-08-20 09:00:00 +01:00));
    }

    #[test]
    fn dia_sem_hora_cai_nas_nove() {
        let quando = resolve("amanha reunir com o cliente");
        assert_eq!(quando.instant, datetime!(2026-08-20 09:00:00 -03:00));
        assert!(!quando.explicit_time);
    }

    #[test]
    fn hora_sem_dia_que_ainda_nao_passou_e_hoje() {
        let quando = resolve("as 18h ligar para o engenheiro");
        assert_eq!(quando.instant, datetime!(2026-08-19 18:00:00 -03:00));
    }

    #[test]
    fn hora_sem_dia_que_ja_passou_e_amanha() {
        // Sao 14h32. "As nove" nao pode ser hoje de manha.
        let quando = resolve("as nove ligar para o engenheiro");
        assert_eq!(quando.instant, datetime!(2026-08-20 09:00:00 -03:00));
    }

    #[test]
    fn depois_de_amanha() {
        let quando = resolve("depois de amanha as 10h");
        assert_eq!(quando.instant, datetime!(2026-08-21 10:00:00 -03:00));
    }

    #[test]
    fn dia_da_semana_e_sempre_o_proximo() {
        // Hoje e quarta. "Sexta" e depois de amanha.
        let quando = resolve("sexta revisar a apresentacao");
        assert_eq!(quando.instant, datetime!(2026-08-21 09:00:00 -03:00));
    }

    #[test]
    fn o_dia_da_semana_de_hoje_pede_a_semana_que_vem() {
        // Hoje e quarta. Pedir "quarta" e pedir a proxima, nao agora.
        let quando = resolve("quarta revisar a apresentacao");
        assert_eq!(quando.instant, datetime!(2026-08-26 09:00:00 -03:00));
    }

    #[test]
    fn sexta_feira_com_sufixo_entra_inteira_no_trecho() {
        let quando = resolve("me lembra sexta-feira de revisar");
        assert_eq!(quando.raw, "sexta-feira");
    }

    #[test]
    fn semana_que_vem_resolve_mas_admite_ser_vaga() {
        let quando = resolve("semana que vem falar com o Joao");
        assert_eq!(quando.instant, datetime!(2026-08-24 09:00:00 -03:00));
        assert!(quando.vague);
    }

    #[test]
    fn dia_do_mes_que_ainda_nao_chegou() {
        let quando = resolve("dia 25 entregar o memorial");
        assert_eq!(quando.instant, datetime!(2026-08-25 09:00:00 -03:00));
    }

    #[test]
    fn dia_do_mes_que_ja_passou_vira_o_mes_seguinte() {
        let quando = resolve("dia 5 entregar o memorial");
        assert_eq!(quando.instant, datetime!(2026-09-05 09:00:00 -03:00));
    }

    #[test]
    fn dia_trinta_e_um_pula_os_meses_que_nao_o_tem() {
        // 31 de agosto ja teria passado se hoje fosse setembro; a partir de
        // 19/09 o proximo 31 e o de outubro, e nao um 31 de setembro que nao
        // existe.
        let setembro = datetime!(2026-09-19 14:32:00 -03:00);
        let quando = resolve_when("dia 31 fechar o mes", setembro).unwrap();
        assert_eq!(quando.instant, datetime!(2026-10-31 09:00:00 -03:00));
    }

    #[test]
    fn daqui_a_duas_horas() {
        let quando = resolve("me lembra daqui a duas horas de ligar");
        assert_eq!(quando.instant, datetime!(2026-08-19 16:32:00 -03:00));
        assert_eq!(quando.raw, "daqui a duas horas");
    }

    #[test]
    fn em_trinta_minutos() {
        let quando = resolve("em 30 minutos sair");
        assert_eq!(quando.instant, datetime!(2026-08-19 15:02:00 -03:00));
    }

    #[test]
    fn em_sozinho_sem_unidade_nao_e_tempo() {
        // "em casa" nao pode virar prazo.
        assert!(resolve_when("guardar isso em casa", agora()).is_none());
    }

    #[test]
    fn nove_da_noite_sao_vinte_e_uma_horas() {
        let quando = resolve("me lembra hoje as nove da noite");
        assert_eq!(quando.instant, datetime!(2026-08-19 21:00:00 -03:00));
    }

    #[test]
    fn as_tres_da_tarde() {
        let quando = resolve("amanha as tres da tarde");
        assert_eq!(quando.instant, datetime!(2026-08-20 15:00:00 -03:00));
    }

    #[test]
    fn e_meia() {
        let quando = resolve("amanha as nove e meia");
        assert_eq!(quando.instant, datetime!(2026-08-20 09:30:00 -03:00));
    }

    #[test]
    fn hora_compacta_com_agulha() {
        let quando = resolve("amanha 9h30 revisar");
        assert_eq!(quando.instant, datetime!(2026-08-20 09:30:00 -03:00));
    }

    #[test]
    fn sexta_a_tarde() {
        let quando = resolve("sexta a tarde revisar a apresentacao");
        assert_eq!(quando.instant, datetime!(2026-08-21 14:00:00 -03:00));
    }

    #[test]
    fn meio_dia() {
        let quando = resolve("amanha meio dia almocar");
        assert_eq!(quando.instant, datetime!(2026-08-20 12:00:00 -03:00));
    }

    #[test]
    fn o_trecho_falado_preserva_acento_e_caixa() {
        let quando = resolve_when("me lembra Amanhã às nove", agora()).unwrap();
        assert_eq!(quando.raw, "Amanhã às nove");
    }
}

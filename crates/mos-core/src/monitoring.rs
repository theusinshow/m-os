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

/// Como o sistema observa.
///
/// Separada de [`crate::TrackingSettings`] porque responde outra pergunta: uma
/// diz como o tempo VIRA dinheiro, esta diz o quanto o aplicativo olha por cima
/// do ombro. Quem desliga o monitoramento nao quer, com isso, mexer no
/// arredondamento da fatura.
///
/// Estava gravada no banco desde a migration 0013 e nao tinha como ser lida:
/// a importacao trouxe a configuracao do CronoCAD para colunas que nenhum tipo
/// alcancava.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringSettings {
    pub process_monitoring_enabled: bool,
    /// De quantos em quantos segundos a lista de processos e relida.
    pub check_interval_seconds: i64,
    pub idle_detection_enabled: bool,
    /// Sem teclado nem mouse por tantos minutos, o periodo vira inatividade.
    pub idle_threshold_minutes: i64,
    pub remind_on_open: bool,
    pub remind_on_close: bool,
    /// Oferecer gravacao quando um programa abre o microfone (ADR-047).
    ///
    /// LIGADA de fabrica. O custo esta admitido na ADR: a fronteira da ADR-037 e
    /// atravessada com aviso e nao com pedido, e este campo e a mitigacao.
    pub meeting_detection_enabled: bool,
}

/// Quais processos abriram e quais fecharam entre dois instantes.
///
/// Pura, e por isso testavel sem Windows: o que decide se houve transicao e a
/// diferenca entre dois conjuntos, e nao o sistema operacional. O loop que a
/// chama e a unica parte que precisa de uma maquina de verdade.
pub fn diff_transitions(
    previous: &std::collections::BTreeSet<String>,
    current: &std::collections::BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    (
        current.difference(previous).cloned().collect(),
        previous.difference(current).cloned().collect(),
    )
}

/// Um processo com o microfone aberto, e ha quanto tempo.
///
/// So isto atravessa a fronteira: **quem** e **desde quando**. Nao ha titulo de
/// janela, nao ha conteudo de aba, nao ha audio — e a ausencia deles E a
/// feature, nao uma limitacao dela (ADR-047).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrofoneAberto {
    pub processo: String,
    pub segundos_aberto: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisaoDeOferta {
    Oferecer(String),
    Nada,
}

/// O que o laco sabe no momento de perguntar.
#[derive(Clone, Debug)]
pub struct ContextoDaOferta {
    pub gravando: bool,
    /// Em minuscula, como o `suppress` do monitor ja guarda.
    pub silenciados: std::collections::BTreeSet<String>,
    pub ligado: bool,
    pub espera_segundos: i64,
}

/// O processo do proprio M/OS, que nunca dispara a oferta.
const EU_MESMO: &str = "mos-desktop.exe";

/// Decide se ha oferta a fazer, e para qual processo.
///
/// Pura, e por isso testavel sem Windows — mesma divisao de `diff_transitions`:
/// o que decide e um conjunto de fatos, e nao o sistema operacional.
pub fn decidir_oferta(abertos: &[MicrofoneAberto], contexto: &ContextoDaOferta) -> DecisaoDeOferta {
    if !contexto.ligado || contexto.gravando {
        return DecisaoDeOferta::Nada;
    }

    let alvo = abertos
        .iter()
        // O M/OS abre o microfone quando grava. Sem esta linha ele se veria
        // gravando e ofereceria gravar de novo.
        .filter(|entrada| !entrada.processo.eq_ignore_ascii_case(EU_MESMO))
        .filter(|entrada| {
            !contexto
                .silenciados
                .contains(&entrada.processo.to_lowercase())
        })
        .filter(|entrada| entrada.segundos_aberto >= contexto.espera_segundos)
        // Ganha o aberto ha MAIS tempo: com Discord ao lado do Meet, quem abriu
        // primeiro provavelmente e a reuniao, e quem abriu depois e o acessorio.
        .max_by_key(|entrada| entrada.segundos_aberto);

    match alvo {
        Some(entrada) => DecisaoDeOferta::Oferecer(entrada.processo.clone()),
        None => DecisaoDeOferta::Nada,
    }
}

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
    fn contexto() -> ContextoDaOferta {
        ContextoDaOferta {
            gravando: false,
            silenciados: std::collections::BTreeSet::new(),
            ligado: true,
            espera_segundos: 20,
        }
    }

    fn aberto(processo: &str, segundos: i64) -> MicrofoneAberto {
        MicrofoneAberto {
            processo: processo.to_string(),
            segundos_aberto: segundos,
        }
    }

    #[test]
    fn oferece_quando_um_processo_passa_da_espera() {
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 25)], &contexto()),
            DecisaoDeOferta::Oferecer("chrome.exe".into())
        );
    }

    #[test]
    fn nao_oferece_antes_da_espera() {
        // Microfone que abre por dois segundos e teste de som, push-to-talk,
        // notificacao. Reuniao mantem aberto.
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 5)], &contexto()),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn o_proprio_mos_nao_conta() {
        // `mos-desktop.exe` esta no ConsentStore, e gravar abre o microfone.
        assert_eq!(
            decidir_oferta(&[aberto("mos-desktop.exe", 300)], &contexto()),
            DecisaoDeOferta::Nada
        );
        assert_eq!(
            decidir_oferta(&[aberto("MOS-Desktop.EXE", 300)], &contexto()),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn nao_oferece_durante_gravacao() {
        let mut ctx = contexto();
        ctx.gravando = true;
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300)], &ctx),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn nao_oferece_para_processo_silenciado() {
        let mut ctx = contexto();
        ctx.silenciados.insert("chrome.exe".into());
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300)], &ctx),
            DecisaoDeOferta::Nada
        );
        // O silencio de um nao cala o outro.
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300), aberto("zoom.exe", 100)], &ctx),
            DecisaoDeOferta::Oferecer("zoom.exe".into())
        );
    }

    #[test]
    fn desligado_nao_oferece_nada() {
        let mut ctx = contexto();
        ctx.ligado = false;
        assert_eq!(
            decidir_oferta(&[aberto("chrome.exe", 300)], &ctx),
            DecisaoDeOferta::Nada
        );
    }

    #[test]
    fn com_varios_ganha_o_aberto_ha_mais_tempo() {
        assert_eq!(
            decidir_oferta(
                &[aberto("discord.exe", 40), aberto("chrome.exe", 120)],
                &contexto()
            ),
            DecisaoDeOferta::Oferecer("chrome.exe".into())
        );
    }

    use super::*;

    fn processes(names: &[&str]) -> std::collections::BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    /// A abertura e o fechamento sao a materia-prima da Linha do Tempo. Detectar
    /// uma abertura que nao houve inventaria trabalho; perder uma real apagaria
    /// o unico rastro de que o CAD esteve aberto naquela tarde.
    #[test]
    fn opening_and_closing_are_detected_once_each() {
        let before = processes(&["acad.exe"]);
        let after = processes(&["acad.exe", "revit.exe"]);

        let (opened, closed) = diff_transitions(&before, &after);
        assert_eq!(opened, ["revit.exe"]);
        assert!(closed.is_empty());

        let (opened, closed) = diff_transitions(&after, &before);
        assert!(opened.is_empty());
        assert_eq!(closed, ["revit.exe"]);
    }

    /// O laco roda a cada poucos segundos. Sem isto, um AutoCAD aberto a tarde
    /// inteira geraria centenas de eventos identicos.
    #[test]
    fn a_process_that_stayed_open_generates_nothing() {
        let same = processes(&["acad.exe", "revit.exe"]);
        let (opened, closed) = diff_transitions(&same, &same);
        assert!(opened.is_empty());
        assert!(closed.is_empty());
    }

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

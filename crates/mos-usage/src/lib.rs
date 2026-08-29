//! Quanto de IA foi consumido, lido do que o Claude Code já grava no disco.
//!
//! O crate não fala com rede e não guarda nada. Ele lê os transcripts em
//! `~/.claude/projects/**/*.jsonl`, devolve [`Evento`]s e sabe agrupá-los nas
//! janelas de cinco horas que são a unidade real de cota.
//!
//! # Três fatos da máquina que desenharam este código
//!
//! Os transcripts desta máquina foram contados antes de qualquer linha ser
//! escrita, e cada um dos três achados vira uma regra aqui:
//!
//! * **507 MB, 18 projetos.** Reler tudo a cada tique está fora de questão, e é
//!   por isso que [`leitura`] guarda offset por arquivo;
//! * **3277 linhas com `usage` para 2108 `requestId` únicos.** Um terço das
//!   linhas repete um request já visto — somar linha a linha inflaria o consumo
//!   em cerca de 55%. O [`Evento`] carrega o `request_id` justamente para que
//!   quem persiste possa recusar o repetido;
//! * **`cache_read: 73243` contra `output: 496`** num request qualquer. Somar
//!   token cru faria o anel medir tamanho de contexto em vez de consumo, e é
//!   por isso que [`peso`] pondera.
//!
//! # O que o arquivo NÃO tem
//!
//! Teto de cota e hora de reset. O Claude Code não grava nenhum dos dois. Este
//! crate portanto não devolve porcentagem: devolve peso absoluto e a janela em
//! que ele caiu. Quem transforma isso em proporção é quem conhece o histórico —
//! e a régua é o próprio pico observado, nunca um teto inventado.

pub mod leitura;

pub use leitura::{varrer, Fonte, Ponteiro, Varredura};

use serde::Deserialize;
use time::{Duration, OffsetDateTime};

/// A duração da janela de cota. Cinco horas é o bloco que a Anthropic usa, e o
/// único número deste crate que vem de fora da máquina.
pub const JANELA: Duration = Duration::hours(5);

/// Um request de assistente, com o que ele custou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evento {
    /// A chave da deduplicação. Ver o segundo achado no topo do módulo.
    pub request_id: String,
    pub em: OffsetDateTime,
    pub modelo: String,
    pub input: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
    pub output: u64,
}

impl Evento {
    /// O peso em milésimos de token-equivalente-de-input.
    ///
    /// Milésimos e não unidades porque o cache lido pesa `0,1`, e arredondar
    /// isso para zero em aritmética inteira apagaria a parcela que mais aparece
    /// nos transcripts.
    pub fn peso(&self) -> u64 {
        self.input * 1_000
            + self.cache_creation * 1_250
            + self.cache_read * 100
            + self.output * 5_000
    }
}

/// O formato que interessa dentro da linha. Tudo o mais é ignorado de propósito:
/// campos que o Claude Code acrescentar amanhã não podem quebrar a leitura.
#[derive(Deserialize)]
struct Linha {
    #[serde(default, rename = "requestId")]
    request_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: Option<Mensagem>,
}

#[derive(Deserialize)]
struct Mensagem {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Uso>,
}

#[derive(Deserialize)]
struct Uso {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

/// Um evento, se a linha for um request de assistente.
///
/// Devolver `None` é o caso COMUM, e não a exceção: a maioria das linhas de um
/// transcript é entrada do usuário, resultado de ferramenta ou resumo, e nenhuma
/// delas custou cota. Linha corrompida — um transcript cortado no meio por um
/// crash é normal — também cai aqui, sem erro e sem parar a varredura.
pub fn parse_linha(linha: &str) -> Option<Evento> {
    let bruto: Linha = serde_json::from_str(linha).ok()?;
    let mensagem = bruto.message?;
    let uso = mensagem.usage?;
    // Sem `requestId` não há como deduplicar, e contar um evento que pode ser
    // repetido é pior que perdê-lo: o erro do primeiro é permanente e cresce.
    let request_id = bruto.request_id?;
    let em = OffsetDateTime::parse(
        &bruto.timestamp?,
        &time::format_description::well_known::Rfc3339,
    )
    .ok()?;
    Some(Evento {
        request_id,
        em,
        modelo: mensagem.model.unwrap_or_else(|| "desconhecido".to_string()),
        input: uso.input_tokens,
        cache_creation: uso.cache_creation_input_tokens,
        cache_read: uso.cache_read_input_tokens,
        output: uso.output_tokens,
    })
}

/// Um bloco de cinco horas, com o que foi gasto dentro dele.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bloco {
    pub inicio: OffsetDateTime,
    pub fim: OffsetDateTime,
    pub peso: u64,
    pub requisicoes: u64,
}

/// O início da janela em que um instante cai: a hora cheia.
///
/// Arredondar para baixo, e não usar o instante do primeiro evento, é o que faz
/// duas máquinas — ou duas varreduras em ordens diferentes — chegarem ao mesmo
/// `inicio` para o mesmo evento. Sem isso o `inicio` seria função da ordem de
/// leitura, e ele é chave primária no banco.
pub fn inicio_da_janela(em: OffsetDateTime) -> OffsetDateTime {
    em.replace_minute(0)
        .and_then(|t| t.replace_second(0))
        .and_then(|t| t.replace_nanosecond(0))
        .unwrap_or(em)
}

/// Agrupa eventos em blocos de cinco horas.
///
/// Duas condições abrem bloco novo, e as duas são necessárias:
///
/// * o evento caiu depois do fim do bloco corrente — a janela expirou pelo
///   relógio;
/// * passaram-se mais de cinco horas desde o último evento — a janela expirou
///   por silêncio, mesmo que o relógio ainda coubesse.
///
/// Os eventos não precisam chegar ordenados: a função ordena, porque a
/// varredura percorre arquivos em ordem de diretório e não de tempo.
pub fn blocos(mut eventos: Vec<Evento>) -> Vec<Bloco> {
    eventos.sort_by_key(|evento| evento.em);
    let mut blocos: Vec<Bloco> = Vec::new();
    let mut ultimo: Option<OffsetDateTime> = None;

    for evento in eventos {
        let novo = match (blocos.last(), ultimo) {
            (Some(bloco), Some(anterior)) => {
                evento.em >= bloco.fim || evento.em - anterior > JANELA
            }
            _ => true,
        };
        if novo {
            let inicio = inicio_da_janela(evento.em);
            blocos.push(Bloco {
                inicio,
                fim: inicio + JANELA,
                peso: 0,
                requisicoes: 0,
            });
        }
        let bloco = blocos.last_mut().expect("acabou de ser empurrado");
        bloco.peso += evento.peso();
        bloco.requisicoes += 1;
        ultimo = Some(evento.em);
    }

    blocos
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn linha_de(request_id: &str, timestamp: &str, output: u64) -> String {
        format!(
            r#"{{"requestId":"{request_id}","timestamp":"{timestamp}","type":"assistant","message":{{"model":"claude-opus-5","usage":{{"input_tokens":2,"cache_creation_input_tokens":1000,"cache_read_input_tokens":70000,"output_tokens":{output}}}}}}}"#
        )
    }

    #[test]
    fn le_uma_linha_de_assistente() {
        let evento = parse_linha(&linha_de("req_1", "2026-08-29T03:13:13.697Z", 496))
            .expect("linha de assistente vira evento");
        assert_eq!(evento.request_id, "req_1");
        assert_eq!(evento.modelo, "claude-opus-5");
        assert_eq!(evento.output, 496);
        assert_eq!(evento.em, datetime!(2026-08-29 03:13:13.697 UTC));
    }

    #[test]
    fn linha_sem_usage_nao_e_evento() {
        // O caso comum: entrada do usuário não custou cota.
        let linha = r#"{"type":"user","timestamp":"2026-08-29T03:13:00Z","message":{"role":"user","content":"oi"}}"#;
        assert!(parse_linha(linha).is_none());
    }

    #[test]
    fn linha_corrompida_nao_derruba_a_leitura() {
        // Transcript cortado no meio por um crash. Acontece, e não é erro.
        assert!(parse_linha(r#"{"requestId":"req_1","timesta"#).is_none());
        assert!(parse_linha("").is_none());
    }

    #[test]
    fn linha_sem_request_id_e_recusada() {
        // Sem chave não há dedupe, e um evento que pode ser contado duas vezes
        // é pior que um evento perdido.
        let linha = r#"{"timestamp":"2026-08-29T03:13:00Z","message":{"model":"claude-opus-5","usage":{"output_tokens":10}}}"#;
        assert!(parse_linha(linha).is_none());
    }

    #[test]
    fn o_peso_pondera_pelo_preco_publicado() {
        let evento = parse_linha(&linha_de("req_1", "2026-08-29T03:13:13.697Z", 496)).unwrap();
        // 2×1000 + 1000×1250 + 70000×100 + 496×5000
        assert_eq!(evento.peso(), 2_000 + 1_250_000 + 7_000_000 + 2_480_000);
    }

    #[test]
    fn a_ponderacao_tira_o_cache_lido_do_volante() {
        // O achado do módulo, com os números reais de um request desta máquina:
        // 73243 de cache lido contra 496 de saída.
        //
        // A ponderação NÃO inverte a ordem — o cache lido continua sendo a maior
        // parcela, e deve mesmo, porque ele existiu. O que ela faz é derrubar a
        // desproporção de 148× para cerca de 3×, e é essa diferença que decide
        // se o anel mede consumo ou mede tamanho de contexto.
        let evento = Evento {
            request_id: "req".into(),
            em: datetime!(2026-08-29 03:00:00 UTC),
            modelo: "claude-opus-5".into(),
            input: 2,
            cache_creation: 1_054,
            cache_read: 73_243,
            output: 496,
        };
        let cru = evento.cache_read as f64 / evento.output as f64;
        let ponderado = (evento.cache_read * 100) as f64 / (evento.output * 5_000) as f64;
        assert!(cru > 140.0, "cru: {cru:.0}×");
        assert!(ponderado < 4.0, "ponderado: {ponderado:.1}×");
    }

    #[test]
    fn a_janela_comeca_na_hora_cheia() {
        assert_eq!(
            inicio_da_janela(datetime!(2026-08-29 03:13:13.697 UTC)),
            datetime!(2026-08-29 03:00:00 UTC)
        );
    }

    fn evento_em(request_id: &str, em: OffsetDateTime) -> Evento {
        Evento {
            request_id: request_id.into(),
            em,
            modelo: "claude-opus-5".into(),
            input: 1,
            cache_creation: 0,
            cache_read: 0,
            output: 0,
        }
    }

    #[test]
    fn eventos_da_mesma_janela_ficam_no_mesmo_bloco() {
        let blocos = blocos(vec![
            evento_em("a", datetime!(2026-08-29 03:13:00 UTC)),
            evento_em("b", datetime!(2026-08-29 06:59:00 UTC)),
        ]);
        assert_eq!(blocos.len(), 1);
        assert_eq!(blocos[0].inicio, datetime!(2026-08-29 03:00:00 UTC));
        assert_eq!(blocos[0].fim, datetime!(2026-08-29 08:00:00 UTC));
        assert_eq!(blocos[0].requisicoes, 2);
    }

    #[test]
    fn o_relogio_fecha_a_janela_mesmo_sem_silencio() {
        // 03:13 e 08:30 têm menos de cinco horas de intervalo entre si? Não —
        // mas o ponto aqui é o FIM do bloco, que é 08:00.
        let blocos = blocos(vec![
            evento_em("a", datetime!(2026-08-29 03:13:00 UTC)),
            evento_em("b", datetime!(2026-08-29 07:59:00 UTC)),
            evento_em("c", datetime!(2026-08-29 08:01:00 UTC)),
        ]);
        assert_eq!(blocos.len(), 2);
        assert_eq!(blocos[1].inicio, datetime!(2026-08-29 08:00:00 UTC));
    }

    #[test]
    fn eventos_fora_de_ordem_caem_no_bloco_certo() {
        // A varredura percorre diretórios, não a linha do tempo.
        let blocos = blocos(vec![
            evento_em("b", datetime!(2026-08-29 20:00:00 UTC)),
            evento_em("a", datetime!(2026-08-29 03:13:00 UTC)),
        ]);
        assert_eq!(blocos.len(), 2);
        assert_eq!(blocos[0].inicio, datetime!(2026-08-29 03:00:00 UTC));
        assert_eq!(blocos[1].inicio, datetime!(2026-08-29 20:00:00 UTC));
    }
}

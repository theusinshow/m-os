//! O contrato `mos-meeting`, e a validacao que o sustenta.
//!
//! O modelo responde em texto, e dentro dele um bloco cercado. E o mesmo
//! mecanismo que `SPEC-ACOES-ENTRE-APPS.md` ja usa para acoes, e pelo mesmo
//! motivo verificado: o protocolo do gateway **nao tem registro de ferramenta do
//! lado do cliente** (ADR-028).
//!
//! A regra que governa o arquivo inteiro vem da spec de acoes, §3 passo 4:
//!
//! > **Argumento fora do esquema = proposta recusada, nao corrigida.**
//!
//! Aqui ela ganha uma forma mais dura, porque o que esta em jogo e proveniencia:
//! **evidencia que aponta para um segmento inexistente e descartada**, e o
//! descarte e CONTADO. Um item que perde toda a evidencia sobrevive marcado, e
//! deixa de ser elegivel a criacao em lote (§12.3) — o Meeting Agent nao
//! apresenta inferencia como fato sem proveniencia.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    Confidence, InsightId, InsightKind, InsightStatus, MeetingEvidence, MeetingId, MeetingInsight,
    SegmentId, TranscriptSegment,
};

/// O que impede uma resposta de virar analise.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AnalysisError {
    /// A resposta veio sem o bloco. Costuma significar que o modelo respondeu em
    /// prosa — e reprompt resolve, o que e por que esta variante e separada.
    NoBlock,
    /// O bloco existe e nao e JSON valido.
    Malformed { detail: String },
    /// O JSON e valido e nao tem a forma do contrato.
    OffContract { detail: String },
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoBlock => write!(f, "A resposta veio sem o bloco `mos-meeting`."),
            Self::Malformed { detail } => {
                write!(f, "O bloco `mos-meeting` nao e JSON valido: {detail}")
            }
            Self::OffContract { detail } => {
                write!(f, "O bloco `mos-meeting` nao segue o contrato: {detail}")
            }
        }
    }
}

impl std::error::Error for AnalysisError {}

/// O que foi recusado, e por que.
///
/// **Contado, e nao descartado em silencio.** Um corte que nao aparece em lugar
/// nenhum le-se como "o modelo nao encontrou nada", quando na verdade encontrou
/// e nos recusamos. A interface mostra isto quando e diferente de zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejections {
    /// `kind` que o contrato nao conhece.
    pub unknown_kind: u32,
    /// Itens sem texto util.
    pub empty_text: u32,
    /// Evidencia apontando para segmento que nao existe nesta reuniao.
    ///
    /// E o numero mais importante daqui: ele mede citacao inventada.
    pub invented_evidence: u32,
    /// Recorte fora dos limites do texto do segmento.
    pub bad_range: u32,
    /// Itens que ficaram sem nenhuma evidencia valida.
    pub without_evidence: u32,
}

impl Rejections {
    pub fn any(&self) -> bool {
        self.unknown_kind > 0
            || self.empty_text > 0
            || self.invented_evidence > 0
            || self.bad_range > 0
            || self.without_evidence > 0
    }
}

/// O resultado de ler uma resposta do Hermes.
#[derive(Clone, Debug)]
pub struct AnalysisOutcome {
    pub summary: String,
    pub topics: Vec<String>,
    pub insights: Vec<MeetingInsight>,
    pub rejections: Rejections,
}

// ---------------------------------------------------------------------------
// O JSON, como ele chega
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Payload {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default, rename = "dueHint")]
    due_hint: Option<String>,
    #[serde(default)]
    confidence: String,
    #[serde(default)]
    evidence: Vec<Evidence>,
}

#[derive(Deserialize)]
struct Evidence {
    #[serde(default)]
    segment: String,
    #[serde(default, rename = "charStart")]
    char_start: Option<u32>,
    #[serde(default, rename = "charEnd")]
    char_end: Option<u32>,
}

/// Extrai o bloco `mos-meeting` da resposta.
///
/// Aceita a cerca com ou sem o rotulo depois das crases, porque modelos variam —
/// mas exige que o rotulo exista em algum lugar da linha de abertura. Uma cerca
/// generica poderia ser um bloco de codigo que o modelo escreveu por outro
/// motivo, e le-lo como analise seria pior que nao achar nada.
fn extract_block(response: &str) -> Option<&str> {
    let mut rest = response;
    while let Some(start) = rest.find("```") {
        let after = &rest[start + 3..];
        let line_end = after.find('\n')?;
        let label = after[..line_end].trim();
        let body_start = line_end + 1;
        let body = &after[body_start..];
        let Some(end) = body.find("```") else {
            // Cerca aberta e nao fechada: a resposta foi cortada no meio.
            return None;
        };
        if label.eq_ignore_ascii_case("mos-meeting") {
            return Some(&body[..end]);
        }
        rest = &body[end + 3..];
    }
    None
}

/// Le a resposta do Hermes e devolve uma analise validada.
///
/// `segments` sao os segmentos REAIS desta reuniao. Eles sao a unica autoridade
/// sobre o que existe: qualquer evidencia fora deles e citacao inventada.
pub fn parse_analysis(
    meeting_id: MeetingId,
    response: &str,
    segments: &[TranscriptSegment],
) -> Result<AnalysisOutcome, AnalysisError> {
    let block = extract_block(response).ok_or(AnalysisError::NoBlock)?;
    let payload: Payload =
        serde_json::from_str(block).map_err(|error| AnalysisError::Malformed {
            detail: error.to_string(),
        })?;

    let summary = payload.summary.trim().to_owned();
    if summary.is_empty() && payload.items.is_empty() {
        return Err(AnalysisError::OffContract {
            detail: "o bloco nao trouxe nem resumo nem itens".into(),
        });
    }

    // O indice dos segmentos reais. `HashMap` e nao `HashSet` porque a validacao
    // do recorte precisa do TEXTO, e nao so da existencia do id.
    let known: HashMap<SegmentId, &str> = segments
        .iter()
        .map(|segment| (segment.id, segment.text.as_str()))
        .collect();

    let mut rejections = Rejections::default();
    let mut insights = Vec::new();

    for (index, item) in payload.items.into_iter().enumerate() {
        let Ok(kind) = InsightKind::parse(&item.kind) else {
            rejections.unknown_kind += 1;
            continue;
        };
        let text = item.text.trim().to_owned();
        if text.is_empty() {
            rejections.empty_text += 1;
            continue;
        }

        let mut evidence = Vec::new();
        for reference in item.evidence {
            let Some(segment_id) = read_segment_id(&reference.segment) else {
                rejections.invented_evidence += 1;
                continue;
            };
            let Some(segment_text) = known.get(&segment_id) else {
                // A defesa contra citacao inventada, e ela e barata: um mapa dos
                // ids reais. Sem ela, um `WHY?` levaria a pessoa a um trecho que
                // nunca existiu — e ela confiaria no que visse.
                rejections.invented_evidence += 1;
                continue;
            };

            let (char_start, char_end) =
                match validate_range(segment_text, reference.char_start, reference.char_end) {
                    Range::Valid(start, end) => (start, end),
                    Range::None => (None, None),
                    Range::Invalid => {
                        // O recorte cai; o segmento fica. A evidencia continua
                        // apontando para a fala certa, so sem o grifo.
                        rejections.bad_range += 1;
                        (None, None)
                    }
                };

            evidence.push(MeetingEvidence {
                segment_id,
                seq: evidence.len() as i64,
                char_start,
                char_end,
            });
        }

        if evidence.is_empty() {
            rejections.without_evidence += 1;
        }

        insights.push(MeetingInsight {
            id: InsightId::new(),
            meeting_id,
            kind,
            seq: index as i64,
            text,
            owner: item.owner.map(|owner| owner.trim().to_owned()).filter(|owner| !owner.is_empty()),
            due_hint: item
                .due_hint
                .map(|hint| hint.trim().to_owned())
                .filter(|hint| !hint.is_empty()),
            confidence: Confidence::parse(&item.confidence).unwrap_or(
                // Um `confidence` que o modelo nao mandou, ou mandou errado, e
                // uma confianca que ninguem mediu. `Medium` e o valor honesto:
                // `High` afirmaria certeza inexistente, e `Low` excluiria do
                // lote um item que talvez fosse bom.
                Confidence::Medium,
            ),
            status: InsightStatus::Proposed,
            created_task_id: None,
            created_reminder_id: None,
            evidence,
        });
    }

    Ok(AnalysisOutcome {
        summary,
        topics: payload
            .topics
            .into_iter()
            .map(|topic| topic.trim().to_owned())
            .filter(|topic| !topic.is_empty())
            .collect(),
        insights,
        rejections,
    })
}

/// Le o id de um segmento a partir do que o modelo escreveu.
///
/// O caminho normal e o campo trazer so o UUID. Mas modelos copiam a linha
/// inteira — `[id] 00:00:11 VOCE — texto` — e isso foi observado na primeira
/// analise real: **8 de 8 evidencias vieram assim**.
///
/// Extrair o UUID de dentro do texto **nao viola** a regra de "recusado, nao
/// corrigido". A regra existe para impedir que um argumento invalido seja
/// adivinhado; aqui nada e adivinhado. O id extraido continua sendo conferido
/// contra os segmentos REAIS logo em seguida, e um id que nao exista continua
/// sendo descartado e contado. O que muda e so a forma de ler, e nao o que e
/// aceito: uma linha copiada que cite um segmento inexistente cai exatamente
/// como caia antes.
///
/// A defesa contra citacao inventada nunca foi o formato do campo. E o mapa dos
/// ids reais.
fn read_segment_id(raw: &str) -> Option<SegmentId> {
    let trimmed = raw.trim();
    if let Ok(id) = SegmentId::parse(trimmed) {
        return Some(id);
    }
    // Um UUID tem 36 caracteres. Varre as janelas desse tamanho e devolve a
    // primeira que for um id valido.
    let bytes = trimmed.as_bytes();
    if bytes.len() < 36 {
        return None;
    }
    for start in 0..=bytes.len() - 36 {
        if !trimmed.is_char_boundary(start) || !trimmed.is_char_boundary(start + 36) {
            continue;
        }
        if let Ok(id) = SegmentId::parse(&trimmed[start..start + 36]) {
            return Some(id);
        }
    }
    None
}

enum Range {
    Valid(Option<u32>, Option<u32>),
    None,
    Invalid,
}

/// Valida um recorte dentro do texto de um segmento.
///
/// **Os limites sao checados em BYTES e em fronteira de caractere.** Um indice
/// no meio de um caractere multibyte — e portugues tem muitos — faria o
/// fatiamento entrar em panico, e um panic dentro de um comando derruba o turno
/// inteiro (§18).
fn validate_range(text: &str, start: Option<u32>, end: Option<u32>) -> Range {
    match (start, end) {
        (None, None) => Range::None,
        (Some(start), Some(end)) => {
            let start = start as usize;
            let end = end as usize;
            if start >= end
                || end > text.len()
                || !text.is_char_boundary(start)
                || !text.is_char_boundary(end)
            {
                return Range::Invalid;
            }
            Range::Valid(Some(start as u32), Some(end as u32))
        }
        // Meio recorte nao recorta nada.
        _ => Range::Invalid,
    }
}

// ---------------------------------------------------------------------------
// O prompt
// ---------------------------------------------------------------------------

/// Uma janela de transcricao pronta para ir ao Hermes.
#[derive(Clone, Debug)]
pub struct PromptWindow {
    pub text: String,
    /// Quantos segmentos entraram nesta janela.
    pub segments: usize,
}

/// Quanto de transcricao cabe num envio.
///
/// Nao e o limite do modelo: e o ORCAMENTO que decidimos gastar. A ADR-028
/// registra por que ele precisa existir — *"o contexto e fixo no envio, e nao ha
/// segunda chance"*.
pub const WINDOW_BUDGET_CHARS: usize = 48_000;

/// Quantos segmentos de sobreposicao entre janelas.
///
/// Sem sobreposicao, uma decisao dita exatamente na fronteira apareceria pela
/// metade nas duas janelas e inteira em nenhuma.
const OVERLAP_SEGMENTS: usize = 6;

/// Formata a transcricao em janelas orcadas.
///
/// Cada linha carrega **o id do segmento**, e e isso que torna a evidencia
/// possivel: o modelo so consegue citar um trecho se souber o nome dele.
pub fn build_windows(segments: &[TranscriptSegment], budget: usize) -> Vec<PromptWindow> {
    if segments.is_empty() {
        return Vec::new();
    }
    let budget = budget.max(1_000);
    let mut windows = Vec::new();
    let mut index = 0usize;

    while index < segments.len() {
        let mut text = String::new();
        let start = index;
        while index < segments.len() {
            let line = format_segment(&segments[index]);
            if !text.is_empty() && text.len() + line.len() > budget {
                break;
            }
            text.push_str(&line);
            index += 1;
        }
        // Um unico segmento maior que o orcamento inteiro: ele entra sozinho, em
        // vez de travar o laco para sempre.
        if index == start {
            text.push_str(&format_segment(&segments[index]));
            index += 1;
        }
        windows.push(PromptWindow {
            text,
            segments: index - start,
        });
        if index < segments.len() {
            index = index.saturating_sub(OVERLAP_SEGMENTS).max(start + 1);
        }
    }
    windows
}

fn format_segment(segment: &TranscriptSegment) -> String {
    let quem = match segment.channel {
        crate::MeetingChannel::Mic => "VOCE",
        crate::MeetingChannel::System => "REMOTO",
    };
    format!(
        "[{}] {} {} — {}\n",
        segment.id,
        clock(segment.start_ms),
        quem,
        segment.text
    )
}

fn clock(ms: i64) -> String {
    let total = ms / 1000;
    format!("{:02}:{:02}:{:02}", total / 3600, (total / 60) % 60, total % 60)
}

/// As instrucoes que acompanham a transcricao.
///
/// Elas nao pedem gentileza nem tom: pedem FORMA. O que garante qualidade e a
/// validacao do lado de ca, e nao a educacao do prompt — um modelo que ignore
/// isto produz um bloco que `parse_analysis` recusa.
pub fn instructions(title: &str, notes: &str) -> String {
    // Sem notas, o bloco NAO existe. Um cabecalho vazio ensinaria o modelo a
    // procurar conteudo que nao esta la, e modelo que procura o que nao existe
    // acaba inventando.
    let bloco = if notes.trim().is_empty() {
        String::new()
    } else {
        format!(
            "NOTAS DE QUEM GRAVOU (contexto, nao transcricao):\n\
             {}\n\
             \n\
             Elas dizem o que importou para quem estava na reuniao. Use para o\n\
             resumo e para desambiguar o que foi dito. Elas NAO foram ditas em\n\
             voz alta, entao nao servem de evidencia.\n\
             \n",
            notes.trim()
        )
    };
    format!(
        "{bloco}Voce esta analisando a transcricao da reuniao \"{title}\".\n\
         \n\
         Cada linha tem a forma `[id] hh:mm:ss QUEM — texto`.\n\
         O `id` e SO o que esta dentro dos colchetes, por exemplo\n\
         `0198c4a1-2b3d-7e4f-8a9b-0c1d2e3f4a5b`.\n\
         `VOCE` e a pessoa que gravou. `REMOTO` sao os outros participantes.\n\
         \n\
         Responda com um bloco cercado ```mos-meeting contendo JSON:\n\
         \n\
         {{\n\
         \x20 \"summary\": \"resumo curto, em portugues\",\n\
         \x20 \"topics\": [\"assunto\"],\n\
         \x20 \"items\": [\n\
         \x20   {{\n\
         \x20     \"kind\": \"decision|my_action|other_action|deadline|follow_up|open_question|risk\",\n\
         \x20     \"text\": \"o item, em portugues\",\n\
         \x20     \"owner\": \"quem, se foi dito\",\n\
         \x20     \"dueHint\": \"o prazo COMO FOI DITO, ex: amanha, sexta\",\n\
         \x20     \"confidence\": \"high|medium|low\",\n\
         \x20     \"evidence\": [{{ \"segment\": \"o id da linha\" }}]\n\
         \x20   }}\n\
         \x20 ]\n\
         }}\n\
         \n\
         Regras:\n\
         - `my_action` e o que VOCE se comprometeu a fazer; `other_action` e dos outros.\n\
         - todo item precisa de pelo menos um `segment`, e ele e o id entre\n\
         \x20 colchetes da linha que sustenta o item — **so o id**, sem os\n\
         \x20 colchetes, sem o horario e sem o texto.\n\
         - nao invente id: um id que nao esta acima faz a evidencia ser\n\
         \x20 descartada, e o item perde o direito de virar Task num clique.\n\
         - `dueHint` guarda a palavra dita, nunca uma data calculada.\n\
         - use `low` quando a frase for hipotetica (\"talvez\", \"quem sabe\").\n\
         - as notas acima, quando existirem, sao CONTEXTO: nenhum item pode\n\
         \x20 ter como unica base uma nota, porque nota nao tem `segment`.\n\
         - nao repita a transcricao no resumo."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RawSegment;

    fn segmentos() -> Vec<TranscriptSegment> {
        crate::interleave(
            MeetingId::new(),
            vec![RawSegment {
                start_ms: 4_000,
                end_ms: 8_000,
                text: "Eu termino os slides amanha de manha.".into(),
                confidence: None,
            }],
            vec![RawSegment {
                start_ms: 9_000,
                end_ms: 12_000,
                text: "Combinado, eu reviso na sexta.".into(),
                confidence: None,
            }],
        )
    }

    fn resposta(items: &str) -> String {
        format!(
            "Analisei a reuniao.\n\n```mos-meeting\n{{\"summary\":\"Alinhamento.\",\
             \"topics\":[\"NexoDoc\"],\"items\":[{items}]}}\n```\n"
        )
    }

    #[test]
    fn le_o_bloco_e_valida_a_evidencia() {
        let segments = segmentos();
        let id = segments[0].id;
        let response = resposta(&format!(
            "{{\"kind\":\"my_action\",\"text\":\"Finalizar os slides\",\
             \"owner\":\"Matheus\",\"dueHint\":\"amanha\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":\"{id}\"}}]}}"
        ));

        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.summary, "Alinhamento.");
        assert_eq!(outcome.topics, vec!["NexoDoc"]);
        assert_eq!(outcome.insights.len(), 1);
        let item = &outcome.insights[0];
        assert_eq!(item.kind, InsightKind::MyAction);
        assert_eq!(item.owner.as_deref(), Some("Matheus"));
        assert_eq!(item.due_hint.as_deref(), Some("amanha"));
        assert_eq!(item.evidence.len(), 1);
        assert_eq!(item.evidence[0].segment_id, id);
        assert!(!outcome.rejections.any());
    }

    #[test]
    fn evidencia_inventada_e_descartada_e_contada() {
        let segments = segmentos();
        let inventado = SegmentId::new();
        let response = resposta(&format!(
            "{{\"kind\":\"my_action\",\"text\":\"Acao inventada\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":\"{inventado}\"}}]}}"
        ));

        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.invented_evidence, 1);
        assert_eq!(outcome.rejections.without_evidence, 1);
        // O item SOBREVIVE, sem evidencia — e por isso deixa de ser elegivel ao
        // lote. Descarta-lo esconderia do usuario que o modelo o propos.
        assert_eq!(outcome.insights.len(), 1);
        assert!(outcome.insights[0].evidence.is_empty());
        assert!(!outcome.insights[0].eligible_for_bulk());
    }

    #[test]
    fn a_linha_inteira_copiada_ainda_encontra_o_segmento() {
        // Observado na primeira analise real: o modelo copiou
        // `[id] hh:mm:ss QUEM — texto` no lugar do id. A identidade continua
        // verificavel, entao a evidencia vale.
        let segments = segmentos();
        let id = segments[0].id;
        let linha_inteira = format!(
            "[{id}] 00:00:04 VOCE — {}",
            segments[0].text
        );
        let response = resposta(&format!(
            "{{\"kind\":\"my_action\",\"text\":\"Slides\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":{}}}]}}",
            serde_json::to_string(&linha_inteira).unwrap()
        ));

        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.invented_evidence, 0);
        assert_eq!(outcome.insights[0].evidence[0].segment_id, id);
    }

    #[test]
    fn a_linha_copiada_de_um_segmento_inexistente_continua_caindo() {
        // A tolerancia e de FORMA, e nao de identidade: um id que nao existe
        // nesta reuniao cai exatamente como caia antes.
        let segments = segmentos();
        let inventado = SegmentId::new();
        let linha = format!("[{inventado}] 00:00:04 VOCE — uma fala que nunca houve");
        let response = resposta(&format!(
            "{{\"kind\":\"my_action\",\"text\":\"x\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":{}}}]}}",
            serde_json::to_string(&linha).unwrap()
        ));
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.invented_evidence, 1);
        assert_eq!(outcome.rejections.without_evidence, 1);
    }

    #[test]
    fn id_que_nem_uuid_e_e_recusado_como_invencao() {
        let segments = segmentos();
        let response = resposta(
            "{\"kind\":\"decision\",\"text\":\"Uma decisao\",\"confidence\":\"high\",\
             \"evidence\":[{\"segment\":\"linha 3\"}]}",
        );
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.invented_evidence, 1);
    }

    #[test]
    fn kind_desconhecido_derruba_o_item_e_e_contado() {
        let segments = segmentos();
        let response = resposta(
            "{\"kind\":\"acao_urgente\",\"text\":\"x\",\"confidence\":\"high\",\"evidence\":[]}",
        );
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.unknown_kind, 1);
        assert!(outcome.insights.is_empty());
    }

    #[test]
    fn confianca_ausente_ou_errada_vira_media_e_nao_alta() {
        let segments = segmentos();
        let id = segments[0].id;
        for confidence in ["", "altissima", "HIGH "] {
            let response = resposta(&format!(
                "{{\"kind\":\"decision\",\"text\":\"Uma decisao\",\"confidence\":\"{confidence}\",\
                 \"evidence\":[{{\"segment\":\"{id}\"}}]}}"
            ));
            let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
            assert_eq!(
                outcome.insights[0].confidence,
                Confidence::Medium,
                "confidence {confidence:?} deveria virar Medium"
            );
        }
    }

    #[test]
    fn recorte_fora_do_texto_cai_mas_o_segmento_fica() {
        let segments = segmentos();
        let id = segments[0].id;
        let response = resposta(&format!(
            "{{\"kind\":\"my_action\",\"text\":\"Slides\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":\"{id}\",\"charStart\":0,\"charEnd\":9999}}]}}"
        ));
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.bad_range, 1);
        assert_eq!(outcome.insights[0].evidence.len(), 1, "a evidencia fica");
        assert!(outcome.insights[0].evidence[0].char_start.is_none());
    }

    #[test]
    fn recorte_no_meio_de_caractere_multibyte_e_recusado_sem_panico() {
        // "manhã" tem um caractere de dois bytes. Um indice no meio dele faria
        // o fatiamento entrar em panico, e panico dentro de um comando derruba o
        // turno inteiro.
        let mut segments = segmentos();
        segments[0].text = "amanhã".into();
        let id = segments[0].id;
        let posicao_ruim = "amanh".len() + 1; // dentro do "ã"
        let response = resposta(&format!(
            "{{\"kind\":\"my_action\",\"text\":\"x\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":\"{id}\",\"charStart\":0,\"charEnd\":{posicao_ruim}}}]}}"
        ));
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.bad_range, 1);
        assert!(outcome.insights[0].evidence[0].char_end.is_none());
    }

    #[test]
    fn meio_recorte_nao_recorta() {
        let segments = segmentos();
        let id = segments[0].id;
        let response = resposta(&format!(
            "{{\"kind\":\"decision\",\"text\":\"x\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":\"{id}\",\"charStart\":3}}]}}"
        ));
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.bad_range, 1);
    }

    #[test]
    fn resposta_sem_bloco_e_um_erro_proprio() {
        let segments = segmentos();
        assert_eq!(
            parse_analysis(segments[0].meeting_id, "So prosa, sem bloco.", &segments)
                .unwrap_err(),
            AnalysisError::NoBlock
        );
    }

    #[test]
    fn cerca_generica_nao_e_confundida_com_o_contrato() {
        let segments = segmentos();
        let response = "```json\n{\"summary\":\"nao e nosso\"}\n```";
        assert_eq!(
            parse_analysis(segments[0].meeting_id, response, &segments).unwrap_err(),
            AnalysisError::NoBlock
        );
    }

    #[test]
    fn cerca_generica_antes_do_bloco_certo_nao_atrapalha() {
        let segments = segmentos();
        let id = segments[0].id;
        let response = format!(
            "Primeiro um exemplo:\n```json\n{{\"x\":1}}\n```\n\
             Agora a analise:\n```mos-meeting\n{{\"summary\":\"Achou.\",\"items\":[\
             {{\"kind\":\"decision\",\"text\":\"Uma decisao\",\"confidence\":\"high\",\
             \"evidence\":[{{\"segment\":\"{id}\"}}]}}]}}\n```"
        );
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.summary, "Achou.");
        assert_eq!(outcome.insights.len(), 1);
    }

    #[test]
    fn cerca_aberta_e_nao_fechada_e_resposta_cortada() {
        let segments = segmentos();
        let response = "```mos-meeting\n{\"summary\":\"cortou aqui";
        assert_eq!(
            parse_analysis(segments[0].meeting_id, response, &segments).unwrap_err(),
            AnalysisError::NoBlock
        );
    }

    #[test]
    fn json_quebrado_diz_que_e_json_quebrado() {
        let segments = segmentos();
        let response = "```mos-meeting\n{isto nao e json}\n```";
        assert!(matches!(
            parse_analysis(segments[0].meeting_id, response, &segments),
            Err(AnalysisError::Malformed { .. })
        ));
    }

    #[test]
    fn bloco_vazio_e_fora_do_contrato_e_nao_analise_vazia() {
        let segments = segmentos();
        let response = "```mos-meeting\n{}\n```";
        assert!(matches!(
            parse_analysis(segments[0].meeting_id, response, &segments),
            Err(AnalysisError::OffContract { .. })
        ));
    }

    #[test]
    fn item_sem_texto_cai() {
        let segments = segmentos();
        let response = resposta("{\"kind\":\"decision\",\"text\":\"   \",\"evidence\":[]}");
        let outcome = parse_analysis(segments[0].meeting_id, &response, &segments).unwrap();
        assert_eq!(outcome.rejections.empty_text, 1);
        assert!(outcome.insights.is_empty());
    }

    // -----------------------------------------------------------------------
    // Janelas
    // -----------------------------------------------------------------------

    fn muitos(n: usize) -> Vec<TranscriptSegment> {
        crate::interleave(
            MeetingId::new(),
            (0..n)
                .map(|i| RawSegment {
                    start_ms: i as i64 * 1000,
                    end_ms: i as i64 * 1000 + 900,
                    text: format!("Fala numero {i} com algum conteudo para ocupar espaco."),
                    confidence: None,
                })
                .collect(),
            Vec::new(),
        )
    }

    #[test]
    fn uma_transcricao_pequena_cabe_numa_janela() {
        let janelas = build_windows(&muitos(20), WINDOW_BUDGET_CHARS);
        assert_eq!(janelas.len(), 1);
        assert_eq!(janelas[0].segments, 20);
    }

    #[test]
    fn a_janela_carrega_o_id_o_relogio_e_quem_falou() {
        let segments = segmentos();
        let janelas = build_windows(&segments, WINDOW_BUDGET_CHARS);
        let texto = &janelas[0].text;
        assert!(texto.contains(&segments[0].id.to_string()), "o id precisa estar la");
        assert!(texto.contains("00:00:04"));
        assert!(texto.contains("VOCE"));
        assert!(texto.contains("REMOTO"));
    }

    #[test]
    fn uma_transcricao_grande_e_dividida_com_sobreposicao() {
        let segments = muitos(400);
        let janelas = build_windows(&segments, 4_000);
        assert!(janelas.len() > 1, "precisa dividir");

        // A soma das janelas passa do total justamente por causa da
        // sobreposicao: sem ela, uma decisao dita na fronteira apareceria pela
        // metade nas duas e inteira em nenhuma.
        let somados: usize = janelas.iter().map(|janela| janela.segments).sum();
        assert!(
            somados > segments.len(),
            "sem sobreposicao: {somados} contra {}",
            segments.len()
        );
    }

    #[test]
    fn o_orcamento_tem_piso_e_ele_e_deliberado() {
        // `build_windows` clampa o orcamento em 1.000: uma janela menor que uma
        // fala nao ajudaria ninguem, e produziria uma janela por segmento.
        let segments = muitos(5);
        assert_eq!(build_windows(&segments, 1).len(), 1);
    }

    #[test]
    fn um_segmento_maior_que_a_janela_entra_sozinho_e_o_laco_termina() {
        // A guarda que impede o laco infinito: sem ela, um segmento que nao cabe
        // no orcamento faria `index` nunca avancar.
        let mut segments = muitos(3);
        for segment in &mut segments {
            segment.text = "palavra ".repeat(300);
        }
        let janelas = build_windows(&segments, 1_000);

        assert_eq!(janelas.len(), 3, "um por segmento");
        assert!(janelas.iter().all(|janela| janela.segments == 1));
        assert!(
            janelas.iter().all(|janela| janela.text.len() > 1_000),
            "a janela estoura o orcamento porque a alternativa e nao gravar a fala"
        );
    }

    #[test]
    fn transcricao_vazia_nao_gera_janela() {
        assert!(build_windows(&[], WINDOW_BUDGET_CHARS).is_empty());
    }

    #[test]
    fn as_instrucoes_carregam_o_titulo_e_o_nome_do_bloco() {
        let texto = instructions("NexoDoc — Comercial", "");
        assert!(texto.contains("NexoDoc — Comercial"));
        assert!(texto.contains("mos-meeting"));
        assert!(texto.contains("my_action"));
        assert!(texto.contains("nao invente id"));
    }

    #[test]
    fn as_notas_entram_como_contexto_e_a_regra_de_evidencia_fica() {
        let com = instructions("Obra X", "cliente quer o orcamento ate sexta");
        assert!(com.contains("NOTAS DE QUEM GRAVOU"));
        assert!(com.contains("cliente quer o orcamento ate sexta"));
        // A regra que sustenta o "aceitar num clique" nao pode afrouxar.
        assert!(com.contains("pelo menos um `segment`"));
        assert!(
            com.contains("nao servem de evidencia"),
            "o prompt precisa dizer que a nota NAO ancora item"
        );

        // Sem notas, o bloco nao existe: um cabecalho vazio ensinaria o modelo a
        // procurar conteudo que nao esta la.
        let sem = instructions("Obra X", "   ");
        assert!(!sem.contains("NOTAS DE QUEM GRAVOU"));
        // E o resto do prompt continua inteiro.
        assert!(sem.contains("mos-meeting"));
        assert!(sem.contains("pelo menos um `segment`"));
    }
}

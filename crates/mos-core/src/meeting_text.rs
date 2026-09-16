//! As regras deterministicas sobre o texto de uma reuniao.
//!
//! **O modelo sugere; o dominio valida.** Tudo aqui e o lado de ca dessa frase:
//! o que o Hermes devolve passa por regras que nao dependem de prompt — e que
//! continuam valendo quando o modelo ignora a instrucao.
//!
//! - marcadores escritos nas notas (`!task`, `!decision`, `!question`);
//! - deduplicacao de itens repetidos entre janelas;
//! - decisao com linguagem hipotetica cai para pergunta ("talvez", "acho que");
//! - responsavel conferido contra o canal da evidencia;
//! - Project inferido por codigo, `#tag`, cronometro e nome citado;
//! - vocabulario pessoal aplicado a uma copia normalizada da transcricao, sem
//!   nunca tocar no texto cru;
//! - o texto do follow-up, montado sem IA.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::meeting_dates::{find_due_expression, fold};
use crate::{
    Confidence, InsightId, InsightKind, InsightOrigin, InsightStatus, MeetingChannel, MeetingId,
    MeetingInsight, ProjectId, TranscriptSegment,
};

// ---------------------------------------------------------------------------
// Marcadores nas notas
// ---------------------------------------------------------------------------

/// Um item que a pessoa escreveu com marcador.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrittenItem {
    pub kind: InsightKind,
    pub text: String,
    pub owner: Option<String>,
    pub project_tag: Option<String>,
    pub due_expression: Option<String>,
}

/// As tags soltas das notas: `#projeto` e `@pessoa`, em qualquer linha.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteTags {
    pub projects: Vec<String>,
    pub people: Vec<String>,
}

fn marker_kind(word: &str) -> Option<InsightKind> {
    match fold(word).as_str() {
        "!task" | "!tarefa" | "!acao" | "!todo" => Some(InsightKind::MyAction),
        "!decision" | "!decisao" | "!decidido" => Some(InsightKind::Decision),
        "!question" | "!pergunta" | "!duvida" => Some(InsightKind::OpenQuestion),
        "!risk" | "!risco" => Some(InsightKind::Risk),
        _ => None,
    }
}

fn clean_tag(token: &str) -> String {
    token
        .trim_start_matches(['#', '@'])
        .trim_end_matches(|c: char| ",.;:!?)".contains(c))
        .to_owned()
}

/// Le os marcadores das notas.
///
/// Uma linha e um item. O marcador pode vir no inicio (`!task revisar`) ou
/// depois de um marcador de lista (`- !task revisar`). `@pessoa` numa tarefa
/// transforma a tarefa em compromisso DELA; `#projeto` vira dica de Project.
pub fn parse_note_markers(notes: &str) -> Vec<WrittenItem> {
    let mut items = Vec::new();
    for line in notes.lines() {
        let line = line.trim().trim_start_matches(['-', '*', '•']).trim();
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else { continue };
        let Some(mut kind) = marker_kind(first) else {
            continue;
        };

        let mut owner = None;
        let mut project_tag = None;
        let mut text_words = Vec::new();
        for word in words {
            if word.starts_with('@') && word.len() > 1 {
                owner.get_or_insert_with(|| clean_tag(word));
            } else if word.starts_with('#') && word.len() > 1 {
                project_tag.get_or_insert_with(|| clean_tag(word));
            } else {
                text_words.push(word);
            }
        }
        let text = text_words.join(" ");
        if text.trim().is_empty() {
            continue;
        }
        if kind == InsightKind::MyAction && owner.is_some() {
            kind = InsightKind::Commitment;
        }
        items.push(WrittenItem {
            kind,
            due_expression: find_due_expression(&text),
            text,
            owner,
            project_tag,
        });
    }
    items
}

/// Todas as `#tags` e `@pessoas` das notas, sem repeticao.
pub fn parse_note_tags(notes: &str) -> NoteTags {
    let mut tags = NoteTags::default();
    for word in notes.split_whitespace() {
        let clean = clean_tag(word);
        if clean.is_empty() {
            continue;
        }
        if word.starts_with('#') && !tags.projects.contains(&clean) {
            tags.projects.push(clean);
        } else if word.starts_with('@') && !tags.people.contains(&clean) {
            tags.people.push(clean);
        }
    }
    tags
}

/// Os itens escritos, prontos para o banco.
///
/// **Confianca alta e sem evidencia**, e as duas coisas sao deliberadas: foi a
/// pessoa que escreveu, entao nao ha interpretacao a duvidar, e nao ha fala a
/// citar. `eligible_for_bulk` conhece a origem e nao exige evidencia de nota.
pub fn written_insights(meeting_id: MeetingId, notes: &str) -> Vec<MeetingInsight> {
    parse_note_markers(notes)
        .into_iter()
        .enumerate()
        .map(|(index, item)| MeetingInsight {
            id: InsightId::new(),
            meeting_id,
            kind: item.kind,
            seq: index as i64,
            text: item.text,
            owner: item.owner,
            due_hint: item.due_expression,
            confidence: Confidence::High,
            status: InsightStatus::Proposed,
            created_task_id: None,
            created_reminder_id: None,
            evidence: Vec::new(),
            origin: InsightOrigin::Written,
            due_at: None,
            due_confidence: None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Deduplicacao
// ---------------------------------------------------------------------------

const STOPWORDS: &[&str] = &[
    "a", "o", "as", "os", "de", "da", "do", "das", "dos", "e", "em", "no", "na", "nos", "nas",
    "um", "uma", "para", "pra", "pro", "com", "que", "se", "ao", "aos", "por", "vai", "vou",
];

/// A chave que decide se dois textos dizem a mesma coisa.
///
/// Sem acento, caixa, pontuacao e palavra vazia. "Enviar as bases." e "enviar
/// bases" sao o mesmo item; "enviar bases" e "revisar bases" nao sao.
pub fn normalized_key(text: &str) -> String {
    fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty() && !STOPWORDS.contains(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Junta itens repetidos: mesmo tipo e mesma chave viram um so, com as
/// evidencias somadas.
///
/// Acontece de verdade: a analise em janelas sobrepostas ve a mesma frase duas
/// vezes, e sem isto a mesma acao viraria duas Tasks.
pub fn dedupe_insights(insights: Vec<MeetingInsight>) -> Vec<MeetingInsight> {
    let mut kept: Vec<MeetingInsight> = Vec::with_capacity(insights.len());
    let mut index: HashMap<(String, String), usize> = HashMap::new();

    for insight in insights {
        let key = (
            insight.kind.as_str().to_owned(),
            normalized_key(&insight.text),
        );
        match index.get(&key) {
            Some(&position) => {
                let target = &mut kept[position];
                for evidence in insight.evidence {
                    if !target
                        .evidence
                        .iter()
                        .any(|e| e.segment_id == evidence.segment_id)
                    {
                        target.evidence.push(evidence);
                    }
                }
                for (seq, evidence) in target.evidence.iter_mut().enumerate() {
                    evidence.seq = seq as i64;
                }
                if target.owner.is_none() {
                    target.owner = insight.owner;
                }
                if target.due_hint.is_none() {
                    target.due_hint = insight.due_hint;
                }
                target.confidence = stronger(target.confidence, insight.confidence);
            }
            None => {
                index.insert(key, kept.len());
                kept.push(insight);
            }
        }
    }
    for (seq, insight) in kept.iter_mut().enumerate() {
        insight.seq = seq as i64;
    }
    kept
}

fn stronger(a: Confidence, b: Confidence) -> Confidence {
    let rank = |c: Confidence| match c {
        Confidence::High => 2,
        Confidence::Medium => 1,
        Confidence::Low => 0,
    };
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

// ---------------------------------------------------------------------------
// Decisao vs. brainstorm, e responsavel
// ---------------------------------------------------------------------------

const HYPOTHETICAL: &[&str] = &[
    "talvez",
    "acho que",
    "quem sabe",
    "poderia",
    "poderiamos",
    "podiamos",
    "seria bom",
    "seria legal",
    "vamos ver",
    "sera que",
    "pensar em",
    "de repente",
    "se der",
    "a gente podia",
    "nao sei se",
    "possivelmente",
    "eventualmente",
];

/// A frase e hipotetica?
pub fn is_hypothetical(text: &str) -> bool {
    let folded = format!(" {} ", normalized_spaces(&fold(text)));
    HYPOTHETICAL
        .iter()
        .any(|needle| folded.contains(&format!(" {needle} ")))
}

fn normalized_spaces(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

const FIRST_PERSON: &[&str] = &[
    "eu",
    "vou",
    "faco",
    "mando",
    "envio",
    "reviso",
    "termino",
    "fico",
    "consigo",
    "deixa comigo",
    "pode deixar",
    "me",
    "comigo",
];

/// Aplica as regras deterministicas sobre o que o Hermes devolveu.
///
/// 1. **Decisao hipotetica vira pergunta em aberto, com confianca baixa.** "Acho
///    que vamos usar o perfil W" nao foi decidido; foi cogitado.
/// 2. **Acao minha cuja evidencia e toda do canal REMOTO, e sem primeira pessoa
///    na fala, perde a confianca alta.** O canal e o unico sinal que a V1 tem
///    com certeza sobre quem falou; ele nao condena o item, mas pede revisao.
/// 3. **Qualquer item com evidencia hipotetica perde a confianca alta.**
pub fn apply_domain_rules(
    insights: Vec<MeetingInsight>,
    segments: &[TranscriptSegment],
) -> Vec<MeetingInsight> {
    let by_id: HashMap<_, _> = segments.iter().map(|s| (s.id, s)).collect();
    insights
        .into_iter()
        .map(|mut insight| {
            let evidence_texts: Vec<&TranscriptSegment> = insight
                .evidence
                .iter()
                .filter_map(|e| by_id.get(&e.segment_id).copied())
                .collect();
            let hypothetical = is_hypothetical(&insight.text)
                || evidence_texts.iter().any(|s| is_hypothetical(&s.text));

            if insight.kind == InsightKind::Decision && hypothetical {
                insight.kind = InsightKind::OpenQuestion;
                insight.confidence = Confidence::Low;
            } else if hypothetical && insight.confidence == Confidence::High {
                insight.confidence = Confidence::Medium;
            }

            if insight.kind == InsightKind::MyAction
                && insight.confidence == Confidence::High
                && !evidence_texts.is_empty()
                && evidence_texts
                    .iter()
                    .all(|s| s.channel == MeetingChannel::System)
            {
                let folded = format!(
                    " {} ",
                    normalized_spaces(&fold(
                        &evidence_texts
                            .iter()
                            .map(|s| s.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    ))
                );
                let speaks_as_me = FIRST_PERSON
                    .iter()
                    .any(|word| folded.contains(&format!(" {word} ")));
                if !speaks_as_me {
                    insight.confidence = Confidence::Medium;
                }
            }
            if insight.kind == InsightKind::OtherAction {
                insight.kind = InsightKind::Commitment;
            }
            insight
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

/// Um Project candidato, visto do ponto de vista da reuniao.
#[derive(Clone, Debug)]
pub struct ProjectCandidate {
    pub id: ProjectId,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectInferenceInput<'a> {
    pub projects: &'a [ProjectCandidate],
    pub notes: &'a str,
    pub segments: &'a [TranscriptSegment],
    /// O Project do cronometro do CronoCAD que corria durante a gravacao.
    pub active_timer_project: Option<ProjectId>,
    /// O que o Hermes sugeriu, cru.
    pub hermes_hint: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInference {
    pub project_id: ProjectId,
    pub confidence: Confidence,
    /// `code | tag | timer | mentions | hermes`.
    pub reason: String,
}

/// Codigos de projeto no formato da casa: `167-25`, `0042/26`.
fn codes_in(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '-' || c == '/'))
        .filter(|token| {
            let parts: Vec<&str> = token.split(['-', '/']).collect();
            parts.len() == 2
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                && parts[0].len() >= 2
        })
        .map(str::to_owned)
        .collect()
}

/// Quantas vezes `name` aparece como palavra inteira em `haystack` (ja dobrados).
fn mentions(haystack: &str, name: &str) -> usize {
    if name.len() < 3 {
        return 0;
    }
    let padded = format!(" {haystack} ");
    padded.matches(&format!(" {name} ")).count()
}

/// Infere o Project da reuniao.
///
/// **So confianca alta associa sozinha.** Media aparece como sugestao. A ordem
/// dos sinais e a da forca: codigo citado que existe, `#tag` nas notas,
/// cronometro rodando no Project, nome citado tres vezes sem empate, e por
/// ultimo o palpite do Hermes conferido contra a lista real.
pub fn infer_project(input: &ProjectInferenceInput<'_>) -> Option<ProjectInference> {
    if input.projects.is_empty() {
        return None;
    }
    let transcript = normalized_spaces(&fold(
        &input
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    ));
    let folded_names: Vec<(ProjectId, String)> = input
        .projects
        .iter()
        .map(|p| (p.id, normalized_spaces(&fold(&p.name))))
        .collect();

    // 1. Codigo citado (na fala ou nas notas) que e parte do nome de UM Project.
    let mut codes = codes_in(input.notes);
    for segment in input.segments {
        codes.extend(codes_in(&segment.text));
    }
    for code in &codes {
        let owners: Vec<&(ProjectId, String)> = input
            .projects
            .iter()
            .zip(folded_names.iter())
            .filter(|(p, _)| p.name.contains(code.as_str()))
            .map(|(_, f)| f)
            .collect();
        if owners.len() == 1 {
            return Some(ProjectInference {
                project_id: owners[0].0,
                confidence: Confidence::High,
                reason: "code".into(),
            });
        }
    }

    // 2. `#tag` nas notas.
    for tag in parse_note_tags(input.notes).projects {
        let tag = normalized_spaces(&fold(&tag.replace(['-', '_'], " ")));
        let matches: Vec<_> = folded_names
            .iter()
            .filter(|(_, name)| {
                *name == tag || name.split(' ').next() == Some(tag.as_str()) || name.contains(&tag)
            })
            .collect();
        if matches.len() == 1 {
            return Some(ProjectInference {
                project_id: matches[0].0,
                confidence: Confidence::High,
                reason: "tag".into(),
            });
        }
    }

    // 3. Cronometro do CronoCAD rodando no Project durante a gravacao.
    if let Some(project) = input.active_timer_project {
        if input.projects.iter().any(|p| p.id == project) {
            return Some(ProjectInference {
                project_id: project,
                confidence: Confidence::High,
                reason: "timer".into(),
            });
        }
    }

    // 4. Nome citado na fala: tres vezes e sem empate e sugestao forte.
    let mut counted: Vec<(ProjectId, usize)> = folded_names
        .iter()
        .map(|(id, name)| (*id, mentions(&transcript, name)))
        .filter(|(_, count)| *count > 0)
        .collect();
    counted.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    if let Some((id, count)) = counted.first() {
        let tie = counted.get(1).is_some_and(|(_, other)| other == count);
        if *count >= 3 && !tie {
            return Some(ProjectInference {
                project_id: *id,
                confidence: Confidence::Medium,
                reason: "mentions".into(),
            });
        }
    }

    // 5. O palpite do Hermes, so se ele existir de verdade.
    if let Some(hint) = input.hermes_hint {
        let hint = normalized_spaces(&fold(hint));
        if !hint.is_empty() {
            let matches: Vec<_> = folded_names
                .iter()
                .filter(|(_, name)| {
                    *name == hint || name.contains(&hint) || hint.contains(name.as_str())
                })
                .collect();
            if matches.len() == 1 {
                return Some(ProjectInference {
                    project_id: matches[0].0,
                    confidence: Confidence::Medium,
                    reason: "hermes".into(),
                });
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Titulo
// ---------------------------------------------------------------------------

/// O titulo ainda e o do relogio?
///
/// Enquanto for, o M/OS pode troca-lo pelo titulo que a analise sugere. Quando
/// a pessoa renomeia, ele deixa de casar com este padrao — e nunca mais e
/// tocado.
pub fn is_default_title(title: &str) -> bool {
    let rest = match title.trim().strip_prefix("Reuniao de ") {
        Some(rest) => rest,
        None => match title.trim().strip_prefix("Reunião de ") {
            Some(rest) => rest,
            None => return false,
        },
    };
    let bytes = rest.as_bytes();
    bytes.len() == 11
        && bytes[2] == b'/'
        && bytes[5] == b' '
        && bytes[8] == b':'
        && [0, 1, 3, 4, 6, 7, 9, 10]
            .iter()
            .all(|&i| bytes[i].is_ascii_digit())
}

/// Aceita um titulo sugerido, ou recusa.
pub fn validate_title(suggested: &str) -> Option<String> {
    let trimmed = suggested.trim().trim_matches('"').trim();
    let chars = trimmed.chars().count();
    if !(4..=80).contains(&chars) || is_default_title(trimmed) {
        return None;
    }
    Some(trimmed.to_owned())
}

// ---------------------------------------------------------------------------
// Vocabulario
// ---------------------------------------------------------------------------

/// Uma troca feita na transcricao normalizada.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    /// O trecho como o whisper escreveu.
    pub original: String,
    /// O termo do vocabulario.
    pub term: String,
    /// Posicao em bytes dentro do texto NORMALIZADO.
    pub start: usize,
    pub end: usize,
    /// A troca foi por semelhanca, e nao so por acento ou caixa.
    pub uncertain: bool,
}

/// Palavras comuns que nunca sao "corrigidas" para um termo parecido.
const COMMON_WORDS: &[&str] = &[
    "projeto",
    "projetos",
    "reuniao",
    "prancha",
    "pranchas",
    "estrutura",
    "estrutural",
    "revisao",
    "revisar",
    "enviar",
    "mandar",
    "pessoal",
    "semana",
    "amanha",
    "segunda",
    "terca",
    "quarta",
    "quinta",
    "sexta",
    "sabado",
    "domingo",
    "cliente",
    "arquivo",
    "arquivos",
    "detalhe",
    "detalhes",
    "planta",
    "plantas",
    "fundacao",
    "fundacoes",
    "pilares",
    "vigas",
    "lajes",
    "armadura",
    "nivel",
    "niveis",
    "documento",
    "proposta",
    "orcamento",
    "entrega",
    "entregar",
    "obrigado",
    "combinado",
    "certeza",
    "questao",
    "problema",
    "exemplo",
    "momento",
    "depois",
    "sempre",
    "ninguem",
    "alguem",
    "alguma",
    "algum",
    "porque",
    "quando",
    "entao",
    "tambem",
    "aquele",
    "aquela",
];

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current.push(
                (previous[j + 1] + 1)
                    .min(current[j] + 1)
                    .min(previous[j] + cost),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

fn allowed_distance(term_len: usize) -> usize {
    match term_len {
        0..=5 => 0,
        6..=9 => 1,
        _ => 2,
    }
}

/// Aplica o vocabulario a um texto, sem tocar no original.
///
/// Trocas de acento e caixa sao certas ("criciuma" → "Criciúma"). Trocas por
/// semelhanca sao INCERTAS e marcadas, e so acontecem com termo longo e palavra
/// que nao e do portugues comum — "reuniao" nunca vira "Reunião Vila Nova".
/// Frases de ate tres palavras sao comparadas inteiras.
pub fn normalize_with_vocabulary(text: &str, vocabulary: &[String]) -> (String, Vec<Correction>) {
    struct Term {
        display: String,
        folded: String,
        words: usize,
    }
    let terms: Vec<Term> = vocabulary
        .iter()
        .map(|t| t.trim())
        .filter(|t| t.chars().count() >= 3)
        .map(|t| Term {
            display: t.to_owned(),
            folded: normalized_spaces(&fold(t)),
            words: t.split_whitespace().count().clamp(1, 3),
        })
        .collect();
    if terms.is_empty() {
        return (text.to_owned(), Vec::new());
    }

    // Palavras com as posicoes no texto original.
    let mut words: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() || c == '-' {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            words.push((s, i));
        }
    }
    if let Some(s) = start {
        words.push((s, text.len()));
    }

    let mut out = String::with_capacity(text.len());
    let mut corrections = Vec::new();
    let mut cursor = 0usize;
    let mut index = 0usize;

    while index < words.len() {
        let mut replaced = false;
        for size in (1..=3).rev() {
            if index + size > words.len() {
                continue;
            }
            let span_start = words[index].0;
            let span_end = words[index + size - 1].1;
            let original = &text[span_start..span_end];
            let folded = normalized_spaces(&fold(original));
            if size == 1 && COMMON_WORDS.contains(&folded.as_str()) {
                continue;
            }
            let best = terms
                .iter()
                .filter(|term| term.words == size)
                .map(|term| (term, levenshtein(&folded, &term.folded)))
                .filter(|(term, distance)| {
                    *distance <= allowed_distance(term.folded.chars().count())
                })
                .min_by_key(|(_, distance)| *distance);
            let Some((term, distance)) = best else {
                continue;
            };
            if original == term.display {
                // Ja esta certo: avanca sem registrar troca.
                out.push_str(&text[cursor..span_end]);
                cursor = span_end;
                index += size;
                replaced = true;
                break;
            }
            out.push_str(&text[cursor..span_start]);
            let at = out.len();
            out.push_str(&term.display);
            corrections.push(Correction {
                original: original.to_owned(),
                term: term.display.clone(),
                start: at,
                end: out.len(),
                uncertain: distance > 0,
            });
            cursor = span_end;
            index += size;
            replaced = true;
            break;
        }
        if !replaced {
            index += 1;
        }
    }
    out.push_str(&text[cursor..]);
    (out, corrections)
}

// ---------------------------------------------------------------------------
// Follow-up
// ---------------------------------------------------------------------------

/// O texto do follow-up, pronto para copiar.
///
/// **Sem IA, e sem envio.** Ele e montado do que ja foi validado — resumo,
/// decisoes, acoes com dono e prazo —, e a pessoa decide para onde cola.
pub fn build_follow_up(
    title: &str,
    date_label: &str,
    summary: &str,
    insights: &[MeetingInsight],
    due_label: &dyn Fn(&MeetingInsight) -> Option<String>,
) -> String {
    let mut out = format!("{title} — {date_label}\n");
    if !summary.trim().is_empty() {
        out.push_str(&format!("\n{}\n", summary.trim()));
    }
    let live: Vec<&MeetingInsight> = insights
        .iter()
        .filter(|i| i.status != InsightStatus::Dismissed)
        .collect();

    let section = |out: &mut String, label: &str, kinds: &[InsightKind]| {
        let items: Vec<&&MeetingInsight> =
            live.iter().filter(|i| kinds.contains(&i.kind)).collect();
        if items.is_empty() {
            return;
        }
        out.push_str(&format!("\n{label}\n"));
        for item in items {
            let mut line = format!("- {}", item.text.trim());
            if let Some(owner) = item.owner.as_deref().filter(|o| !o.trim().is_empty()) {
                line.push_str(&format!(" ({owner})"));
            }
            if let Some(due) = due_label(item) {
                line.push_str(&format!(" — {due}"));
            }
            out.push_str(&line);
            out.push('\n');
        }
    };

    section(&mut out, "Decisões", &[InsightKind::Decision]);
    section(
        &mut out,
        "Próximos passos",
        &[
            InsightKind::MyAction,
            InsightKind::FollowUp,
            InsightKind::Deadline,
        ],
    );
    section(
        &mut out,
        "Com outras pessoas",
        &[
            InsightKind::OtherAction,
            InsightKind::Commitment,
            InsightKind::Dependency,
        ],
    );
    section(
        &mut out,
        "Em aberto",
        &[InsightKind::OpenQuestion, InsightKind::Risk],
    );
    out.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interleave, MeetingEvidence, RawSegment, SegmentId};

    fn raw(start: i64, text: &str) -> RawSegment {
        RawSegment {
            start_ms: start,
            end_ms: start + 2000,
            text: text.into(),
            confidence: None,
        }
    }

    fn insight(
        kind: InsightKind,
        text: &str,
        confidence: Confidence,
        evidence: &[SegmentId],
    ) -> MeetingInsight {
        MeetingInsight {
            id: InsightId::new(),
            meeting_id: MeetingId::new(),
            kind,
            seq: 0,
            text: text.into(),
            owner: None,
            due_hint: None,
            confidence,
            status: InsightStatus::Proposed,
            created_task_id: None,
            created_reminder_id: None,
            evidence: evidence
                .iter()
                .enumerate()
                .map(|(seq, id)| MeetingEvidence {
                    segment_id: *id,
                    seq: seq as i64,
                    char_start: None,
                    char_end: None,
                })
                .collect(),
            origin: InsightOrigin::Spoken,
            due_at: None,
            due_confidence: None,
        }
    }

    #[test]
    fn marcadores_viram_itens_escritos() {
        let notas = "anotei umas coisas\n\
                     - !task revisar prancha 04 sexta #167-25\n\
                     !decision usar perfil W 200\n\
                     !pergunta confirmar nível EE-04\n\
                     !task enviar bases @Victor amanhã\n\
                     !task\n";
        let itens = parse_note_markers(notas);
        assert_eq!(itens.len(), 4);
        assert_eq!(itens[0].kind, InsightKind::MyAction);
        assert_eq!(itens[0].text, "revisar prancha 04 sexta");
        assert_eq!(itens[0].project_tag.as_deref(), Some("167-25"));
        assert_eq!(itens[0].due_expression.as_deref(), Some("sexta"));
        assert_eq!(itens[1].kind, InsightKind::Decision);
        assert_eq!(itens[2].kind, InsightKind::OpenQuestion);
        // Tarefa com @pessoa e compromisso DELA.
        assert_eq!(itens[3].kind, InsightKind::Commitment);
        assert_eq!(itens[3].owner.as_deref(), Some("Victor"));
    }

    #[test]
    fn item_escrito_entra_no_lote_sem_evidencia() {
        let itens = written_insights(MeetingId::new(), "!task revisar prancha");
        assert_eq!(itens.len(), 1);
        assert!(itens[0].eligible_for_bulk());
        assert!(itens[0].preselected());
    }

    #[test]
    fn deduplicacao_junta_o_mesmo_item_e_soma_evidencias() {
        let a = SegmentId::new();
        let b = SegmentId::new();
        let itens = dedupe_insights(vec![
            insight(
                InsightKind::MyAction,
                "Enviar as bases.",
                Confidence::Medium,
                &[a],
            ),
            insight(
                InsightKind::MyAction,
                "enviar bases",
                Confidence::High,
                &[b, a],
            ),
            insight(
                InsightKind::MyAction,
                "revisar bases",
                Confidence::High,
                &[b],
            ),
            insight(
                InsightKind::Decision,
                "enviar bases",
                Confidence::High,
                &[b],
            ),
        ]);
        assert_eq!(itens.len(), 3);
        assert_eq!(itens[0].evidence.len(), 2);
        assert_eq!(itens[0].confidence, Confidence::High);
        assert_eq!(
            itens.iter().map(|i| i.seq).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn decisao_hipotetica_vira_pergunta() {
        let segmentos = interleave(
            MeetingId::new(),
            vec![raw(0, "Acho que a gente usa o perfil W, vamos ver.")],
            vec![],
        );
        let itens = apply_domain_rules(
            vec![insight(
                InsightKind::Decision,
                "Usar perfil W",
                Confidence::High,
                &[segmentos[0].id],
            )],
            &segmentos,
        );
        assert_eq!(itens[0].kind, InsightKind::OpenQuestion);
        assert_eq!(itens[0].confidence, Confidence::Low);
    }

    #[test]
    fn decisao_explicita_continua_decisao() {
        let segmentos = interleave(
            MeetingId::new(),
            vec![raw(0, "Fechado: vamos usar o perfil W 200.")],
            vec![],
        );
        let itens = apply_domain_rules(
            vec![insight(
                InsightKind::Decision,
                "Usar perfil W 200",
                Confidence::High,
                &[segmentos[0].id],
            )],
            &segmentos,
        );
        assert_eq!(itens[0].kind, InsightKind::Decision);
        assert_eq!(itens[0].confidence, Confidence::High);
    }

    #[test]
    fn minha_acao_so_dita_pelo_remoto_pede_revisao() {
        let segmentos = interleave(
            MeetingId::new(),
            vec![],
            vec![raw(0, "O Matheus precisa mandar as bases até sexta.")],
        );
        let itens = apply_domain_rules(
            vec![insight(
                InsightKind::MyAction,
                "Mandar as bases",
                Confidence::High,
                &[segmentos[0].id],
            )],
            &segmentos,
        );
        assert_eq!(itens[0].confidence, Confidence::Medium);

        // Dita pelo VOCE, fica alta.
        let meus = interleave(
            MeetingId::new(),
            vec![raw(0, "Eu mando as bases até sexta.")],
            vec![],
        );
        let itens = apply_domain_rules(
            vec![insight(
                InsightKind::MyAction,
                "Mandar as bases",
                Confidence::High,
                &[meus[0].id],
            )],
            &meus,
        );
        assert_eq!(itens[0].confidence, Confidence::High);
    }

    #[test]
    fn acao_de_outros_legado_vira_compromisso() {
        let itens = apply_domain_rules(
            vec![insight(
                InsightKind::OtherAction,
                "Victor revisa",
                Confidence::High,
                &[],
            )],
            &[],
        );
        assert_eq!(itens[0].kind, InsightKind::Commitment);
    }

    fn projects() -> Vec<ProjectCandidate> {
        vec![
            ProjectCandidate {
                id: ProjectId::new(),
                name: "167-25 Residencial Vila Nova".into(),
            },
            ProjectCandidate {
                id: ProjectId::new(),
                name: "NexoDoc".into(),
            },
            ProjectCandidate {
                id: ProjectId::new(),
                name: "Escadas Minarum".into(),
            },
        ]
    }

    #[test]
    fn codigo_citado_associa_com_confianca_alta() {
        let lista = projects();
        let segmentos = interleave(
            MeetingId::new(),
            vec![raw(0, "Sobre a obra 167-25, a prancha 04...")],
            vec![],
        );
        let inferido = infer_project(&ProjectInferenceInput {
            projects: &lista,
            segments: &segmentos,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(inferido.project_id, lista[0].id);
        assert_eq!(inferido.confidence, Confidence::High);
    }

    #[test]
    fn nome_citado_tres_vezes_e_so_sugestao() {
        let lista = projects();
        let segmentos = interleave(
            MeetingId::new(),
            vec![
                raw(0, "No NexoDoc a gente"),
                raw(3000, "o NexoDoc precisa"),
                raw(6000, "e o nexodoc"),
            ],
            vec![],
        );
        let inferido = infer_project(&ProjectInferenceInput {
            projects: &lista,
            segments: &segmentos,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(inferido.project_id, lista[1].id);
        assert_eq!(inferido.confidence, Confidence::Medium);

        // Duas mencoes nao bastam.
        let pouco = interleave(MeetingId::new(), vec![raw(0, "NexoDoc e NexoDoc")], vec![]);
        assert!(infer_project(&ProjectInferenceInput {
            projects: &lista,
            segments: &pouco,
            ..Default::default()
        })
        .is_none());
    }

    #[test]
    fn tag_e_cronometro_associam() {
        let lista = projects();
        let por_tag = infer_project(&ProjectInferenceInput {
            projects: &lista,
            notes: "lembrar #escadas",
            ..Default::default()
        })
        .unwrap();
        assert_eq!(por_tag.project_id, lista[2].id);
        assert_eq!(por_tag.reason, "tag");

        let por_timer = infer_project(&ProjectInferenceInput {
            projects: &lista,
            active_timer_project: Some(lista[1].id),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(por_timer.confidence, Confidence::High);
    }

    #[test]
    fn palpite_do_hermes_so_vale_se_o_project_existe() {
        let lista = projects();
        assert!(infer_project(&ProjectInferenceInput {
            projects: &lista,
            hermes_hint: Some("Projeto Inventado"),
            ..Default::default()
        })
        .is_none());
        let valido = infer_project(&ProjectInferenceInput {
            projects: &lista,
            hermes_hint: Some("nexodoc"),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(valido.reason, "hermes");
    }

    #[test]
    fn titulo_padrao_e_reconhecido() {
        assert!(is_default_title("Reuniao de 16/09 14:32"));
        assert!(!is_default_title("Revisão estrutural — 167-25"));
        assert!(!is_default_title("Reuniao de sexta"));
        assert_eq!(
            validate_title("  \"Revisão estrutural\" "),
            Some("Revisão estrutural".into())
        );
        assert_eq!(validate_title("ok"), None);
        assert_eq!(validate_title("Reuniao de 16/09 14:32"), None);
    }

    #[test]
    fn vocabulario_corrige_acento_com_certeza_e_semelhanca_com_duvida() {
        let vocab = vec![
            "Criciúma".to_owned(),
            "NexoDoc".to_owned(),
            "Vila Nova".to_owned(),
        ];
        let (texto, trocas) = normalize_with_vocabulary(
            "Precisamos enviar para criciuma e para Cricuma sexta.",
            &vocab,
        );
        assert_eq!(
            texto,
            "Precisamos enviar para Criciúma e para Criciúma sexta."
        );
        assert_eq!(trocas.len(), 2);
        assert!(!trocas[0].uncertain);
        assert!(trocas[1].uncertain);
        assert_eq!(&texto[trocas[1].start..trocas[1].end], "Criciúma");

        let (texto, trocas) = normalize_with_vocabulary("a obra da vila nova e o nexo doc", &vocab);
        assert_eq!(texto, "a obra da Vila Nova e o nexo doc");
        assert_eq!(trocas.len(), 1);
    }

    #[test]
    fn vocabulario_nunca_troca_palavra_comum_nem_termo_curto() {
        let vocab = vec!["Revisão".to_owned(), "SAP".to_owned()];
        let (texto, trocas) = normalize_with_vocabulary("vamos revisao do sap e da sop", &vocab);
        assert_eq!(texto, "vamos revisao do SAP e da sop");
        assert_eq!(trocas.len(), 1);
    }

    #[test]
    fn follow_up_sai_organizado_e_sem_itens_descartados() {
        let mut descartado = insight(
            InsightKind::MyAction,
            "Coisa descartada",
            Confidence::High,
            &[],
        );
        descartado.status = InsightStatus::Dismissed;
        let mut compromisso = insight(
            InsightKind::Commitment,
            "Revisar IFC",
            Confidence::High,
            &[],
        );
        compromisso.owner = Some("Victor".into());
        let texto = build_follow_up(
            "Revisão estrutural",
            "16/09",
            "Alinhamos a prancha 04.",
            &[
                insight(
                    InsightKind::Decision,
                    "Usar perfil W 200",
                    Confidence::High,
                    &[],
                ),
                insight(InsightKind::MyAction, "Enviar bases", Confidence::High, &[]),
                compromisso,
                descartado,
            ],
            &|i| (i.text == "Enviar bases").then(|| "amanhã".to_owned()),
        );
        assert!(texto.starts_with("Revisão estrutural — 16/09"));
        assert!(texto.contains("Decisões\n- Usar perfil W 200"));
        assert!(texto.contains("- Enviar bases — amanhã"));
        assert!(texto.contains("- Revisar IFC (Victor)"));
        assert!(!texto.contains("descartada"));
    }
}

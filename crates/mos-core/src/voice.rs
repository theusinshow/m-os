//! Voice Inbox: o registro durável do áudio, e a leitura do que foi dito.
//!
//! # As duas metades
//!
//! **`VoiceNote` é o áudio.** Ele existe entre "parei de falar" e "existe
//! texto", e existe porque `Capture.content` é `NOT NULL` com
//! `CHECK (length(trim(content)) > 0)` e o domínio não tem operação de
//! reescrever conteúdo. Uma Capture não pode, portanto, nascer antes da
//! transcrição sem inventar um conteúdo falso e uma mutação que hoje não
//! existe — e reescrever conteúdo depois destruiria a garantia de que a
//! transcrição original é preservada. É o mesmo desenho que `Meeting` usa, e
//! pela mesma razão.
//!
//! **`understand` é a leitura.** Determinística, sem rede e sem IA. O Hermes
//! não participa desta feature, e a ausência é a decisão: `MEETING-AGENT.md`
//! §15.3 já registra que onde a regra determinística serve, ela ganha da IA — e
//! aqui ela serve, porque o vocabulário de "me lembra amanhã às nove" é
//! pequeno e fechado. O que ela não entende vira Capture, que é o
//! comportamento correto e não uma falha.
//!
//! # A guarda que não é refinamento
//!
//! O whisper preenche silêncio com texto inventado. Medido em 19/08 no Meeting
//! Agent: um canal quase mudo transcreveu `"Legenda por Sônia Ruberti"`, e o
//! nome inventado chegou até o resumo. Numa reunião isso é ruído; **aqui isso
//! seria uma Task nascida de silêncio.** Por isso `heard` recusa antes de
//! transcrever, e `is_hallucination` recusa depois.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    voice_when::{fold_text, resolve_when, Spoken},
    CaptureId, Confidence, CoreError, ErrorCode, ProjectId, TaskId,
};

// ------------------------------------------------------------------ guardas

/// Abaixo disto foi tecla encostada, e não fala.
pub const MIN_DURATION_MS: i64 = 400;

/// Pico de RMS, na escala `0..1000` que a thread de captura já produz.
///
/// O número vem da medição de 19/08: o canal que alucinou tinha picos de
/// 1639–4969 contra 27763 do microfone, numa escala de 32767 — ou seja, ~50 de
/// 1000 contra ~847. O piso fica acima do primeiro e muito abaixo do segundo.
pub const MIN_PEAK_LEVEL: u64 = 120;

/// Teto de uma gravação, em milissegundos.
///
/// Não é limite de pensamento: é a rede de segurança do microfone. Se o evento
/// de soltar a tecla se perder — janela trocada, sessão bloqueada —, isto é o
/// que fecha o stream sozinho.
pub const MAX_DURATION_MS: i64 = 120_000;

/// O que a gravação produziu, antes de qualquer transcrição.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Heard {
    Speech,
    TooShort,
    TooQuiet,
}

/// Vale a pena mandar isto ao transcritor?
///
/// As duas recusas não gravam nada — nem linha, nem arquivo. Uma gravação
/// recusada aqui é uma gravação que nunca aconteceu, e é assim que o §23 do
/// brief é cumprido: cancelamento não deixa lixo no banco.
pub fn heard(duration_ms: i64, peak_level: u64) -> Heard {
    if duration_ms < MIN_DURATION_MS {
        return Heard::TooShort;
    }
    if peak_level < MIN_PEAK_LEVEL {
        return Heard::TooQuiet;
    }
    Heard::Speech
}

/// O preenchimento que os modelos de fala inventam quando não há fala.
///
/// Em português a família é sempre a mesma: créditos de legenda. Ela chega
/// inteira, bem pontuada e indistinguível de uma frase real para qualquer
/// verificação de forma — só o conteúdo a denuncia.
///
/// Complementa `is_speech`, que já descarta `[Música]` e `(inaudible)`: aquilo
/// se anuncia com delimitadores, isto não.
pub fn is_hallucination(text: &str) -> bool {
    let folded = fold_text(text.trim()).to_lowercase();
    if folded.is_empty() {
        return true;
    }
    const MARCAS: [&str; 8] = [
        "legenda por",
        "legendas por",
        "legendado por",
        "legendas pela comunidade",
        "amara.org",
        "subtitles by",
        "subtitulos por",
        "transcricao por",
    ];
    MARCAS.iter().any(|marca| folded.contains(marca))
}

// ------------------------------------------------------------------ entidade

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VoiceNoteId(Uuid);

impl VoiceNoteId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::InvalidInput, "Voice note ID invalido.", false))
    }
}

impl Default for VoiceNoteId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VoiceNoteId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Uma máquina de estados, e não campos de etapa soltos.
///
/// `recorded` + `transcribed` + `captured` como booleanos independentes
/// permitiriam representar "capturado sem ter gravado", que é impossível. É a
/// mesma regra que `Meeting.status` aplica.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceNoteStatus {
    Recording,
    /// Áudio em disco, esperando transcrição.
    Recorded,
    Transcribing,
    /// Existe Capture. **É o único estado em que o áudio pode ser apagado.**
    Captured,
    /// A transcrição não aconteceu. O áudio continua em disco, e é isso que
    /// torna `voice_retry` honesto.
    Failed,
    Cancelled,
}

impl VoiceNoteStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Recorded => "recorded",
            Self::Transcribing => "transcribing",
            Self::Captured => "captured",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "recording" => Ok(Self::Recording),
            "recorded" => Ok(Self::Recorded),
            "transcribing" => Ok(Self::Transcribing),
            "captured" => Ok(Self::Captured),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de voice note desconhecido.",
                false,
            )),
        }
    }

    /// Se o áudio desta nota ainda guarda informação que o banco não tem.
    ///
    /// É a pergunta que decide apagar bytes, e por isso ela mora no domínio e
    /// não em quem chama o filesystem.
    pub fn audio_still_needed(self) -> bool {
        matches!(self, Self::Recording | Self::Recorded | Self::Transcribing | Self::Failed)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceNote {
    pub id: VoiceNoteId,
    pub status: VoiceNoteStatus,
    /// Relativo ao diretório de dados, e derivado do id. **Nunca vem do
    /// renderer** — mesma regra do `Meeting.audio_dir`.
    pub audio_dir: String,
    pub duration_ms: i64,
    pub peak_level: u64,
    /// A transcrição ORIGINAL, como o modelo devolveu. Ela não é reescrita.
    pub transcript: String,
    pub provider: String,
    pub capture_id: Option<CaptureId>,
    pub context_project_id: Option<ProjectId>,
    pub context_task_id: Option<TaskId>,
    /// Só existe em `failed`, e existe sempre que ele é failed.
    pub failure_message: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub audio_deleted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewVoiceNote {
    pub id: VoiceNoteId,
    pub audio_dir: String,
    pub started_at: OffsetDateTime,
    pub context_project_id: Option<ProjectId>,
    pub context_task_id: Option<TaskId>,
}

impl NewVoiceNote {
    /// O diretório é DERIVADO do id, aqui dentro. Quem chama não escolhe onde
    /// os bytes caem — é o que impede um caminho vindo da tela de virar
    /// escrita em disco.
    pub fn create(
        started_at: OffsetDateTime,
        context_project_id: Option<ProjectId>,
        context_task_id: Option<TaskId>,
    ) -> Self {
        let id = VoiceNoteId::new();
        Self {
            audio_dir: format!("voice/{id}"),
            id,
            started_at,
            context_project_id,
            context_task_id,
        }
    }
}

/// O que pode acontecer com uma nota.
#[derive(Clone, Debug)]
pub enum VoiceTransition {
    Recorded { duration_ms: i64, peak_level: u64 },
    Transcribing,
    Captured { capture_id: CaptureId, transcript: String, provider: String },
    Failed { message: String },
    Cancelled,
}

/// Aplica uma transição, ou recusa.
///
/// Devolve a nota nova em vez de mutar: quem grava recebe um valor pronto, e
/// não há caminho em que metade da transição chegue ao banco.
pub fn apply(
    note: &VoiceNote,
    transition: VoiceTransition,
    now: OffsetDateTime,
) -> Result<VoiceNote, CoreError> {
    use VoiceNoteStatus::*;

    let recusa = |esperado: &str| {
        Err(CoreError::new(
            ErrorCode::InvalidTransition,
            format!(
                "Uma nota de voz em '{}' nao pode {esperado}.",
                note.status.as_str()
            ),
            false,
        ))
    };

    let mut next = note.clone();
    next.updated_at = now;

    match transition {
        VoiceTransition::Recorded {
            duration_ms,
            peak_level,
        } => {
            if note.status != Recording {
                return recusa("terminar de gravar");
            }
            next.status = Recorded;
            next.duration_ms = duration_ms.max(0);
            next.peak_level = peak_level;
        }
        VoiceTransition::Transcribing => {
            // `Failed` reentra de propósito: é o retry, e ele é o motivo de o
            // áudio continuar em disco.
            if !matches!(note.status, Recorded | Failed) {
                return recusa("comecar a transcrever");
            }
            next.status = Transcribing;
            next.failure_message = String::new();
        }
        VoiceTransition::Captured {
            capture_id,
            transcript,
            provider,
        } => {
            if note.status != Transcribing {
                return recusa("virar Capture");
            }
            if transcript.trim().is_empty() {
                return Err(CoreError::new(
                    ErrorCode::InvalidInput,
                    "Uma nota de voz nao vira Capture sem transcricao.",
                    false,
                ));
            }
            next.status = Captured;
            next.capture_id = Some(capture_id);
            next.transcript = transcript;
            next.provider = provider;
            next.failure_message = String::new();
        }
        VoiceTransition::Failed { message } => {
            if matches!(note.status, Captured | Cancelled) {
                return recusa("falhar");
            }
            next.status = Failed;
            next.failure_message = if message.trim().is_empty() {
                "A transcricao nao foi concluida.".to_owned()
            } else {
                message
            };
        }
        VoiceTransition::Cancelled => {
            if note.status == Captured {
                return recusa("ser cancelada");
            }
            next.status = Cancelled;
        }
    }

    Ok(next)
}

// ------------------------------------------------------------- entendimento

/// O que a tela sabia quando o atalho tocou.
///
/// **Sinal, e não verdade** (brief §13). O contexto só entra quando a fala não
/// disse nada, e quando entra, entra com um degrau a menos de confiança.
#[derive(Clone, Copy, Debug, Default)]
pub struct VoiceContext {
    pub project_id: Option<ProjectId>,
    pub task_id: Option<TaskId>,
}

/// O mínimo de um Project para reconhecê-lo numa frase.
#[derive(Clone, Debug)]
pub struct ProjectHint {
    pub id: ProjectId,
    pub name: String,
}

/// O que fazer com o que foi entendido.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceAction {
    /// Só a Capture. É o destino da maioria, e não é falha.
    Keep,
    CreateTask,
    CreateTaskWithReminder,
}

/// De onde veio o Project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSource {
    None,
    Spoken,
    Context,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Understanding {
    /// O título sugerido para a Task. A Capture guarda a fala inteira.
    pub title: String,
    pub action: VoiceAction,
    pub confidence: Confidence,
    pub project_id: Option<ProjectId>,
    pub project_source: ProjectSource,
    /// O prazo COMO FOI DITO. Vazio quando não houve.
    pub when_raw: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub when: Option<OffsetDateTime>,
    /// Se a frase hesitou. Visível porque é o motivo de não ter agido.
    pub hedged: bool,
}

impl Understanding {
    /// Confiança alta é o único grau que age sozinho.
    pub fn should_execute(&self) -> bool {
        self.action != VoiceAction::Keep && self.confidence == Confidence::High
    }
}

/// Marcadores de hesitação.
///
/// Presentes, a frase vira Capture **mesmo tendo verbo e data**. É o brief §17
/// em forma de código: *não confunda linguagem natural com autorização*.
/// "Talvez eu devesse olhar aquele memorial" tem verbo e tem objeto, e não é
/// um pedido.
const HEDGES: [&str; 10] = [
    "talvez",
    "acho que",
    "quem sabe",
    "seria bom",
    "seria legal",
    "eu devia",
    "eu deveria",
    "nao sei se",
    "qualquer hora",
    "um dia desses",
];

/// Verbos que pedem lembrete, explicitamente.
const REMINDER_VERBS: [&str; 6] = [
    "me lembra",
    "me lembre",
    "me lembrar",
    "lembra me",
    "lembrar de",
    "nao me deixa esquecer",
];

/// Verbos que pedem trabalho registrado.
const TASK_VERBS: [&str; 10] = [
    "coloca",
    "colocar",
    "adiciona",
    "adicionar",
    "cria uma task",
    "criar uma task",
    "cria uma tarefa",
    "criar uma tarefa",
    "anota",
    "anotar",
];

/// Relato sobre terceiros. Não é pedido, e por isso não vira ação.
const REPORTED_SPEECH: [&str; 5] = ["disse que", "falou que", "avisou que", "mandou dizer", "comentou que"];

const STOPWORDS: [&str; 22] = [
    "de", "da", "do", "das", "dos", "o", "a", "os", "as", "e", "em", "no", "na", "nos", "nas",
    "para", "pra", "com", "que", "um", "uma", "ao",
];

/// Lê a fala e decide o que ela pede.
///
/// `now_local` chega do renderer já com o offset de quem falou — é a regra
/// normativa de `CORE-FOUNDATION.md` §5, e o mesmo padrão do
/// `ReminderComposer`.
pub fn understand(
    transcript: &str,
    now_local: OffsetDateTime,
    context: VoiceContext,
    projects: &[ProjectHint],
) -> Understanding {
    let spoken = Spoken::new(transcript);
    let normalized = spoken.normalized();

    let hedged = HEDGES.iter().any(|marca| contains_phrase(&normalized, marca));
    let reminder_verb = REMINDER_VERBS
        .iter()
        .any(|verbo| contains_phrase(&normalized, verbo));
    let task_verb = TASK_VERBS.iter().any(|verbo| contains_phrase(&normalized, verbo));
    let reported = REPORTED_SPEECH
        .iter()
        .any(|marca| contains_phrase(&normalized, marca));

    let when = resolve_when(transcript, now_local);
    let spoken_project = match_project(&spoken, projects);

    let (project_id, project_source) = match (spoken_project, context.project_id) {
        (Some(id), _) => (Some(id), ProjectSource::Spoken),
        (None, Some(id)) => (Some(id), ProjectSource::Context),
        (None, None) => (None, ProjectSource::None),
    };

    // Um instante ja passado nao autoriza lembrete.
    //
    // "Me lembra hoje as nove", dito as duas da tarde, resolve para as nove da
    // MANHA de hoje. `NewReminder::at` recusa o passado, e sem esta leitura a
    // acao inteira falharia — a Capture ficaria salva e a tela mostraria um
    // erro, para uma frase perfeitamente normal.
    //
    // A saida NAO e empurrar para amanha: ninguem disse amanha. E nao criar
    // lembrete, e deixar a Task.
    let when_passou = when
        .as_ref()
        .map(|resolved| resolved.instant <= now_local)
        .unwrap_or(false);

    let (action, confidence) = classify(Signals {
        hedged,
        reminder_verb,
        task_verb,
        reported,
        when_passou,
        when: when.as_ref(),
        project_spoken: matches!(project_source, ProjectSource::Spoken),
        infinitive_opening: opens_with_infinitive(&spoken),
    });

    Understanding {
        title: title_from(transcript, when.as_ref().map(|resolved| resolved.raw.as_str())),
        action,
        confidence,
        project_id,
        project_source,
        when_raw: when
            .as_ref()
            .map(|resolved| resolved.raw.clone())
            .unwrap_or_default(),
        when: when.as_ref().map(|resolved| resolved.instant),
        hedged,
    }
}

struct Signals<'a> {
    hedged: bool,
    reminder_verb: bool,
    task_verb: bool,
    reported: bool,
    /// O instante resolvido ja passou. Ele continua legivel; so nao agenda.
    when_passou: bool,
    when: Option<&'a crate::ResolvedWhen>,
    project_spoken: bool,
    infinitive_opening: bool,
}

fn classify(signals: Signals<'_>) -> (VoiceAction, Confidence) {
    // A hesitação vence tudo, inclusive verbo e data juntos.
    if signals.hedged {
        return (VoiceAction::Keep, Confidence::Low);
    }

    let firm_when = signals
        .when
        .map(|resolved| !resolved.vague)
        .unwrap_or(false)
        && !signals.when_passou;

    if signals.reminder_verb {
        return match signals.when {
            // Pedido explícito com instante explícito: é o caso em que agir
            // sozinho é o certo.
            Some(_) if firm_when => (VoiceAction::CreateTaskWithReminder, Confidence::High),
            // Instante já passado: Task sim, lembrete não. Agendar para trás é
            // impossível, e adivinhar o dia certo seria inventar.
            Some(_) if signals.when_passou => (VoiceAction::CreateTask, Confidence::Medium),
            Some(_) => (VoiceAction::CreateTaskWithReminder, Confidence::Medium),
            // Pediu lembrete e não disse quando. Inventar prazo seria pior que
            // não agir — a Capture guarda o pedido e a oferta fica visível.
            None => (VoiceAction::CreateTask, Confidence::Medium),
        };
    }

    if signals.task_verb {
        let confidence = if signals.project_spoken {
            Confidence::High
        } else {
            Confidence::Medium
        };
        return if firm_when {
            (VoiceAction::CreateTaskWithReminder, confidence)
        } else {
            (VoiceAction::CreateTask, confidence)
        };
    }

    // "João disse que vai mandar o orçamento sexta." Não é meu pedido, e a
    // ausência de Waiting For no M/OS não autoriza inventar uma Task no lugar.
    if signals.reported {
        return (VoiceAction::Keep, Confidence::Low);
    }

    if firm_when {
        return (VoiceAction::CreateTaskWithReminder, Confidence::Medium);
    }

    // "Comprar café." Frase curta abrindo em infinitivo é Task com boa
    // probabilidade — e média nunca age sozinha, então o erro custa uma oferta
    // ignorada, e não uma Task indesejada.
    if signals.infinitive_opening {
        return (VoiceAction::CreateTask, Confidence::Medium);
    }

    (VoiceAction::Keep, Confidence::Low)
}

/// Frase curta que abre em infinitivo: "comprar café", "revisar as fundações".
///
/// O teto de palavras é o que separa um pedido de um pensamento longo. Sem
/// ele, qualquer parágrafo que começasse com verbo viraria oferta de Task.
fn opens_with_infinitive(spoken: &Spoken) -> bool {
    let words = spoken.words();
    if words.is_empty() || words.len() > 6 {
        return false;
    }
    let first = words[0];
    first.len() >= 5
        && (first.ends_with("ar") || first.ends_with("er") || first.ends_with("ir"))
}

/// A expressão aparece no texto, respeitando fronteira de palavra.
///
/// `contains` cru casaria "uma" dentro de "alguma", e um marcador de hesitação
/// falso-positivo silenciaria a feature inteira.
fn contains_phrase(normalized: &str, phrase: &str) -> bool {
    let padded = format!(" {normalized} ");
    padded.contains(&format!(" {phrase} "))
}

/// Qual Project a frase citou, se citou algum.
///
/// Dois caminhos, e **o empate não escolhe**: dois Projects candidatos para a
/// mesma frase significam que ninguém foi citado com clareza, e chutar um
/// deles seria o pior desfecho possível — trabalho arquivado no lugar errado
/// sem ninguém ter escolhido.
fn match_project(spoken: &Spoken, projects: &[ProjectHint]) -> Option<ProjectId> {
    let words = spoken.words();
    let candidates: Vec<(ProjectId, Vec<String>)> = projects
        .iter()
        .map(|project| (project.id, significant_tokens(&project.name)))
        .filter(|(_, tokens)| !tokens.is_empty())
        .collect();

    // 1. O nome inteiro, dito inteiro.
    let full: Vec<ProjectId> = candidates
        .iter()
        .filter(|(_, tokens)| contains_sequence(&words, tokens))
        .map(|(id, _)| *id)
        .collect();
    if full.len() == 1 {
        return Some(full[0]);
    }
    if full.len() > 1 {
        return None;
    }

    // 2. "no projeto X", com X sendo o começo do nome — o caso do código falado
    //    ("063-26") de um Project chamado "063-26 Residência Souza".
    let marker = words
        .iter()
        .position(|word| *word == "projeto" || *word == "project")?;
    let rest: Vec<String> = words
        .iter()
        .skip(marker + 1)
        .take(4)
        .map(|word| word.to_string())
        .collect();

    // Do trecho mais longo para o mais curto: "063 26" antes de "063".
    for length in (1..=rest.len()).rev() {
        let needle = &rest[..length];
        if needle.iter().all(|token| is_stopword(token)) {
            continue;
        }
        let hits: Vec<ProjectId> = candidates
            .iter()
            .filter(|(_, tokens)| tokens.starts_with(needle) || contains_sequence_owned(tokens, needle))
            .map(|(id, _)| *id)
            .collect();
        if hits.len() == 1 {
            return Some(hits[0]);
        }
        if hits.len() > 1 {
            return None;
        }
    }
    None
}

fn is_stopword(token: &str) -> bool {
    STOPWORDS.contains(&token)
}

/// As palavras que identificam um Project, sem as que não identificam nada.
fn significant_tokens(name: &str) -> Vec<String> {
    Spoken::new(name)
        .words()
        .into_iter()
        .filter(|word| !is_stopword(word))
        .map(|word| word.to_string())
        .collect()
}

fn contains_sequence(haystack: &[&str], needle: &[String]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    (0..=haystack.len() - needle.len())
        .any(|start| (0..needle.len()).all(|step| haystack[start + step] == needle[step]))
}

fn contains_sequence_owned(haystack: &[String], needle: &[String]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    (0..=haystack.len() - needle.len())
        .any(|start| (0..needle.len()).all(|step| haystack[start + step] == needle[step]))
}

/// O título da Task: a fala sem o andaime.
///
/// Sem o verbo de pedido, sem o prazo e sem o trecho de Project — eles viram
/// campos, e repeti-los no título faria a Task ler como a transcrição em vez de
/// como trabalho. **A Capture guarda a fala inteira**, então nada se perde
/// aqui.
pub fn title_from(transcript: &str, when_raw: Option<&str>) -> String {
    let mut text = transcript.trim().to_owned();

    if let Some(raw) = when_raw {
        if !raw.is_empty() {
            if let Some(at) = text.find(raw) {
                text.replace_range(at..at + raw.len(), " ");
            }
        }
    }

    text = strip_leading_phrase(&text, &REMINDER_VERBS);
    text = strip_leading_phrase(&text, &TASK_VERBS);
    text = strip_project_phrase(&text);

    let mut cleaned = collapse(&text);
    cleaned = strip_leading_connectors(&cleaned);
    // Ponto final de uma frase falada não pertence a um título.
    cleaned = cleaned.trim_end_matches(['.', ',', ';', ':']).trim().to_owned();

    if cleaned.is_empty() {
        cleaned = collapse(transcript);
    }
    let cleaned = capitalize(&cleaned);
    if cleaned.chars().count() > 120 {
        let short: String = cleaned.chars().take(119).collect();
        format!("{}…", short.trim_end())
    } else {
        cleaned
    }
}

/// Remove uma das expressões, onde quer que ela esteja.
///
/// Onde quer que esteja, e não só no início: "coloca revisar fundações" abre
/// com o verbo, mas "amanhã me lembra de revisar" o traz no meio depois de o
/// prazo ter saído.
fn strip_leading_phrase(text: &str, phrases: &[&str]) -> String {
    let folded = fold_text(text).to_lowercase();
    for phrase in phrases {
        if let Some(at) = folded.find(phrase) {
            // Índices do texto dobrado servem para o original: `fold` troca um
            // caractere por um caractere, nunca muda o comprimento em chars.
            let start = char_to_byte(text, folded[..at].chars().count());
            let end = char_to_byte(text, folded[..at + phrase.len()].chars().count());
            let mut out = String::with_capacity(text.len());
            out.push_str(&text[..start]);
            out.push(' ');
            out.push_str(&text[end..]);
            return out;
        }
    }
    text.to_owned()
}

/// Tira "no projeto X" / "do projeto X" do título.
fn strip_project_phrase(text: &str) -> String {
    let folded = fold_text(text).to_lowercase();
    let Some(at) = folded.find("projeto ").or_else(|| folded.find("project ")) else {
        return text.to_owned();
    };
    // Volta a preposição que vier colada antes: "no projeto", "do projeto".
    let mut start_chars = folded[..at].chars().count();
    let before: Vec<char> = folded[..at].chars().collect();
    for preposicao in ["no ", "do ", "na ", "da ", "em ", "para o ", "pro "] {
        let tamanho = preposicao.chars().count();
        if before.len() >= tamanho
            && before[before.len() - tamanho..].iter().collect::<String>() == preposicao
        {
            start_chars -= tamanho;
            break;
        }
    }
    let start = char_to_byte(text, start_chars);
    String::from(&text[..start])
}

fn strip_leading_connectors(text: &str) -> String {
    let mut current = text.trim().to_owned();
    loop {
        let folded = fold_text(&current).to_lowercase();
        let Some(first) = folded.split_whitespace().next() else {
            return current;
        };
        if !matches!(first, "de" | "que" | "a" | "o" | "para" | "pra" | "e" | "ao") {
            return current;
        }
        let cut = char_to_byte(&current, first.chars().count());
        current = current[cut..].trim_start().to_owned();
        if current.is_empty() {
            return current;
        }
    }
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn char_to_byte(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn agora() -> OffsetDateTime {
        datetime!(2026-08-19 14:32:00 -03:00)
    }

    fn projeto(nome: &str) -> ProjectHint {
        ProjectHint {
            id: ProjectId::new(),
            name: nome.to_owned(),
        }
    }

    fn ler(frase: &str) -> Understanding {
        understand(frase, agora(), VoiceContext::default(), &[])
    }

    // -------------------------------------------------------------- guardas

    #[test]
    fn tecla_encostada_nao_vira_gravacao() {
        assert_eq!(heard(200, 900), Heard::TooShort);
    }

    #[test]
    fn silencio_nao_vai_ao_transcritor() {
        // O caso medido em 19/08: energia de sobra para o arquivo existir, e de
        // menos para haver fala.
        assert_eq!(heard(8_000, 50), Heard::TooQuiet);
    }

    #[test]
    fn fala_de_verdade_passa() {
        assert_eq!(heard(8_000, 847), Heard::Speech);
    }

    #[test]
    fn o_credito_de_legenda_inventado_e_reconhecido() {
        assert!(is_hallucination("Legenda por Sônia Ruberti"));
        assert!(is_hallucination("  legendas pela comunidade Amara.org  "));
        assert!(is_hallucination(""));
    }

    #[test]
    fn fala_de_verdade_nao_e_confundida_com_alucinacao() {
        assert!(!is_hallucination("me lembra amanha de revisar o memorial"));
        // A palavra sozinha não denuncia nada: legendar um vídeo é trabalho.
        assert!(!is_hallucination("preciso revisar a legenda do grafico"));
    }

    // ------------------------------------------------------------- exemplos

    #[test]
    fn exemplo_a_comprar_cafe_oferece_task_sem_criar_sozinho() {
        let lido = ler("Comprar café.");
        assert_eq!(lido.action, VoiceAction::CreateTask);
        assert_eq!(lido.confidence, Confidence::Medium);
        assert!(!lido.should_execute());
        assert_eq!(lido.title, "Comprar café");
    }

    #[test]
    fn exemplo_b_lembrete_explicito_age_sozinho() {
        let lido = ler("Me lembra amanhã às nove de revisar o memorial.");
        assert_eq!(lido.action, VoiceAction::CreateTaskWithReminder);
        assert_eq!(lido.confidence, Confidence::High);
        assert!(lido.should_execute());
        assert_eq!(lido.when, Some(datetime!(2026-08-20 09:00:00 -03:00)));
        assert_eq!(lido.when_raw, "amanhã às nove");
        assert_eq!(lido.title, "Revisar o memorial");
    }

    #[test]
    fn exemplo_c_task_no_projeto_falado() {
        let projetos = [projeto("063-26 Residência Souza"), projeto("NexoDoc")];
        let lido = understand(
            "Coloca revisar fundações no projeto 063-26.",
            agora(),
            VoiceContext::default(),
            &projetos,
        );
        assert_eq!(lido.action, VoiceAction::CreateTask);
        assert_eq!(lido.confidence, Confidence::High);
        assert_eq!(lido.project_id, Some(projetos[0].id));
        assert_eq!(lido.project_source, ProjectSource::Spoken);
        assert_eq!(lido.title, "Revisar fundações");
    }

    #[test]
    fn exemplo_d_relato_sobre_terceiro_fica_em_capture() {
        // Não há Waiting For no M/OS, e a ausência não autoriza inventar Task.
        let lido = ler("João disse que vai mandar o orçamento sexta.");
        assert_eq!(lido.action, VoiceAction::Keep);
        assert_eq!(lido.confidence, Confidence::Low);
    }

    // ------------------------------------------------------------ hesitacao

    #[test]
    fn hesitacao_vence_verbo_e_data_juntos() {
        let lido = ler("Talvez me lembra amanhã às nove de olhar aquele memorial.");
        assert!(lido.hedged);
        assert_eq!(lido.action, VoiceAction::Keep);
        assert_eq!(lido.confidence, Confidence::Low);
        // O prazo continua legível — só não autoriza nada.
        assert!(lido.when.is_some());
    }

    #[test]
    fn linguagem_natural_nao_e_autorizacao() {
        let lido = ler("Talvez eu devesse olhar aquele memorial.");
        assert_eq!(lido.action, VoiceAction::Keep);
        assert!(!lido.should_execute());
    }

    #[test]
    fn hesitacao_nao_casa_dentro_de_outra_palavra() {
        // "alguma" contém "uma"; "atalho" contém "talho". Fronteira de palavra.
        let lido = ler("Me lembra amanhã às nove de revisar alguma coisa.");
        assert!(!lido.hedged);
    }

    // -------------------------------------------------------------- prazo

    #[test]
    fn lembrete_sem_quando_nao_inventa_prazo() {
        let lido = ler("Me lembra de ligar para o engenheiro.");
        assert_eq!(lido.action, VoiceAction::CreateTask);
        assert_eq!(lido.confidence, Confidence::Medium);
        assert!(lido.when.is_none());
    }

    #[test]
    fn um_instante_ja_passado_nao_agenda_lembrete() {
        // São 14h32. "Hoje às nove" resolve para as nove da MANHÃ, que já foi.
        // `NewReminder::at` recusaria o passado, e sem esta leitura a ação
        // inteira falharia numa frase perfeitamente normal.
        let lido = ler("Me lembra hoje às nove de revisar o memorial.");
        assert_eq!(lido.action, VoiceAction::CreateTask);
        assert!(!lido.should_execute());
        // O prazo continua legível — ele só não agenda.
        assert!(lido.when.is_some());
        assert_eq!(lido.when_raw, "hoje às nove");
    }

    #[test]
    fn prazo_vago_nao_autoriza_acao_automatica() {
        let lido = ler("Me lembra semana que vem de falar com o cliente.");
        assert_eq!(lido.confidence, Confidence::Medium);
        assert!(!lido.should_execute());
    }

    // ------------------------------------------------------------- project

    #[test]
    fn o_contexto_entra_quando_a_fala_nao_disse_nada() {
        let nexo = projeto("NexoDoc");
        let lido = understand(
            "Me lembra sexta de revisar a apresentação.",
            agora(),
            VoiceContext {
                project_id: Some(nexo.id),
                task_id: None,
            },
            std::slice::from_ref(&nexo),
        );
        assert_eq!(lido.project_id, Some(nexo.id));
        assert_eq!(lido.project_source, ProjectSource::Context);
    }

    #[test]
    fn a_fala_vence_o_contexto() {
        let nexo = projeto("NexoDoc");
        let obra = projeto("063-26 Residência Souza");
        let lido = understand(
            "Coloca revisar fundações no projeto 063-26.",
            agora(),
            VoiceContext {
                project_id: Some(nexo.id),
                task_id: None,
            },
            &[nexo, obra.clone()],
        );
        assert_eq!(lido.project_id, Some(obra.id));
    }

    #[test]
    fn nome_inteiro_dito_inteiro_casa_sem_marcador() {
        let nexo = projeto("NexoDoc");
        let lido = understand(
            "Anota melhorar o import do NexoDoc.",
            agora(),
            VoiceContext::default(),
            &[nexo.clone(), projeto("063-26 Residência Souza")],
        );
        assert_eq!(lido.project_id, Some(nexo.id));
    }

    #[test]
    fn o_empate_nao_escolhe_nenhum() {
        // Dois Projects começando por "063": citar "063" não identifica um.
        let lido = understand(
            "Coloca revisar fundações no projeto 063.",
            agora(),
            VoiceContext::default(),
            &[projeto("063-26 Residência Souza"), projeto("063-27 Anexo")],
        );
        assert_eq!(lido.project_id, None);
        assert_eq!(lido.project_source, ProjectSource::None);
        // E sem Project falado, o verbo de trabalho não basta para agir.
        assert_eq!(lido.confidence, Confidence::Medium);
    }

    #[test]
    fn project_que_ninguem_citou_nao_e_inventado() {
        let lido = understand(
            "Comprar café.",
            agora(),
            VoiceContext::default(),
            &[projeto("NexoDoc")],
        );
        assert_eq!(lido.project_id, None);
    }

    // --------------------------------------------------------------- titulo

    #[test]
    fn o_titulo_perde_o_andaime_e_a_capture_guarda_tudo() {
        assert_eq!(
            title_from("Me lembra amanhã às nove de revisar o memorial.", Some("amanhã às nove")),
            "Revisar o memorial"
        );
    }

    #[test]
    fn titulo_sem_nada_para_tirar_sobrevive_inteiro() {
        assert_eq!(title_from("Comprar café", None), "Comprar café");
    }

    #[test]
    fn titulo_que_ficaria_vazio_volta_a_fala() {
        // Só o andaime foi dito. Um título vazio recusaria a Task; a fala
        // inteira é pior título e melhor comportamento.
        assert_eq!(title_from("Me lembra amanhã", Some("amanhã")), "Me lembra amanhã");
    }

    #[test]
    fn titulo_longo_e_cortado_com_reticencia() {
        let longo = "revisar ".repeat(40);
        let titulo = title_from(&longo, None);
        assert!(titulo.chars().count() <= 120);
        assert!(titulo.ends_with('…'));
    }

    // ------------------------------------------------------- maquina de estados

    fn nota() -> VoiceNote {
        let nova = NewVoiceNote::create(agora(), None, None);
        VoiceNote {
            id: nova.id,
            status: VoiceNoteStatus::Recording,
            audio_dir: nova.audio_dir,
            duration_ms: 0,
            peak_level: 0,
            transcript: String::new(),
            provider: String::new(),
            capture_id: None,
            context_project_id: None,
            context_task_id: None,
            failure_message: String::new(),
            audio_deleted_at: None,
            started_at: agora(),
            updated_at: agora(),
        }
    }

    #[test]
    fn o_diretorio_de_audio_e_derivado_do_id() {
        let nova = NewVoiceNote::create(agora(), None, None);
        assert_eq!(nova.audio_dir, format!("voice/{}", nova.id));
    }

    #[test]
    fn a_nota_percorre_o_caminho_feliz() {
        let nota = nota();
        let gravada = apply(
            &nota,
            VoiceTransition::Recorded {
                duration_ms: 4_200,
                peak_level: 800,
            },
            agora(),
        )
        .unwrap();
        assert_eq!(gravada.status, VoiceNoteStatus::Recorded);

        let transcrevendo = apply(&gravada, VoiceTransition::Transcribing, agora()).unwrap();
        let capturada = apply(
            &transcrevendo,
            VoiceTransition::Captured {
                capture_id: CaptureId::new(),
                transcript: "comprar cafe".into(),
                provider: "whisper.cpp".into(),
            },
            agora(),
        )
        .unwrap();
        assert_eq!(capturada.status, VoiceNoteStatus::Captured);
        assert!(capturada.capture_id.is_some());
        assert!(!capturada.status.audio_still_needed());
    }

    #[test]
    fn uma_nota_nao_vira_capture_sem_ter_gravado() {
        let erro = apply(
            &nota(),
            VoiceTransition::Captured {
                capture_id: CaptureId::new(),
                transcript: "qualquer coisa".into(),
                provider: String::new(),
            },
            agora(),
        )
        .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidTransition);
    }

    #[test]
    fn uma_capture_nao_nasce_de_transcricao_vazia() {
        let gravada = apply(
            &nota(),
            VoiceTransition::Recorded {
                duration_ms: 4_200,
                peak_level: 800,
            },
            agora(),
        )
        .unwrap();
        let transcrevendo = apply(&gravada, VoiceTransition::Transcribing, agora()).unwrap();
        let erro = apply(
            &transcrevendo,
            VoiceTransition::Captured {
                capture_id: CaptureId::new(),
                transcript: "   ".into(),
                provider: String::new(),
            },
            agora(),
        )
        .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn falhar_preserva_o_audio_e_deixa_o_retry_entrar() {
        let gravada = apply(
            &nota(),
            VoiceTransition::Recorded {
                duration_ms: 4_200,
                peak_level: 800,
            },
            agora(),
        )
        .unwrap();
        let falhou = apply(
            &gravada,
            VoiceTransition::Failed {
                message: "o transcritor nao esta configurado".into(),
            },
            agora(),
        )
        .unwrap();
        assert_eq!(falhou.status, VoiceNoteStatus::Failed);
        assert!(falhou.status.audio_still_needed());
        // E o retry é uma transição legítima a partir daqui.
        assert!(apply(&falhou, VoiceTransition::Transcribing, agora()).is_ok());
    }

    #[test]
    fn uma_nota_ja_capturada_nao_regride() {
        let capturada = VoiceNote {
            status: VoiceNoteStatus::Captured,
            ..nota()
        };
        assert!(apply(&capturada, VoiceTransition::Cancelled, agora()).is_err());
        assert!(apply(
            &capturada,
            VoiceTransition::Failed {
                message: "tarde demais".into()
            },
            agora()
        )
        .is_err());
    }

    #[test]
    fn falha_sem_mensagem_ainda_diz_alguma_coisa() {
        let falhou = apply(
            &nota(),
            VoiceTransition::Failed {
                message: "  ".into(),
            },
            agora(),
        )
        .unwrap();
        assert!(!falhou.failure_message.trim().is_empty());
    }
}

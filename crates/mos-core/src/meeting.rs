//! O dominio de Meeting.
//!
//! Puro: sem janela, sem SQLite, sem WASAPI. E obrigatorio, e nao estetico —
//! `SETUP-MAQUINA.md` §4 registra que `cargo test -p mos-desktop` nao roda na
//! maquina principal, e a conclusao dele e que "a logica precisa morar em
//! `mos-core` ou `mos-storage-sqlite`, onde os testes rodam".
//!
//! Ver `docs/MEETING-AGENT.md` §6 e §7.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CoreError, ErrorCode, LifecycleState, ProjectId, ReminderId, TaskId};

macro_rules! meeting_id {
    ($name:ident, $erro:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn parse(value: &str) -> Result<Self, CoreError> {
                Uuid::parse_str(value)
                    .map(Self)
                    .map_err(|_| CoreError::new(ErrorCode::InvalidInput, $erro, false))
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

meeting_id!(MeetingId, "Meeting ID invalido.");
meeting_id!(SegmentId, "ID de segmento invalido.");
meeting_id!(InsightId, "ID de item de reuniao invalido.");

/// De onde a reuniao nasceu.
///
/// So `Manual` existe na V1. Os outros dois estao aqui porque a V2 os exige e
/// porque um `match` exaustivo e o que garante que ninguem esqueca um caso ao
/// implementa-los — mas **nenhum caminho de codigo os produz hoje**, e a §17.2
/// depende disso: nao existe gravacao que comece sem clique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingSource {
    Manual,
    /// V2: sugerida por evento de calendario. Nunca inicia sozinha.
    Calendar,
    /// V2: sugerida por deteccao de uso de microfone. Nunca inicia sozinha.
    Detected,
}

impl MeetingSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Calendar => "calendar",
            Self::Detected => "detected",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "manual" => Ok(Self::Manual),
            "calendar" => Ok(Self::Calendar),
            "detected" => Ok(Self::Detected),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Origem de Meeting desconhecida.",
                false,
            )),
        }
    }
}

/// Em que etapa do processamento a falha aconteceu.
///
/// Faz parte de `Failed` porque "a gravacao esta segura e a transcricao falhou"
/// e "a gravacao se perdeu" pedem respostas opostas (§20), e sem o estagio a
/// interface nao consegue dizer qual das duas aconteceu.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailedStage {
    Audio,
    Transcription,
    Analysis,
}

impl FailedStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Transcription => "transcription",
            Self::Analysis => "analysis",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "audio" => Ok(Self::Audio),
            "transcription" => Ok(Self::Transcription),
            "analysis" => Ok(Self::Analysis),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estagio de falha desconhecido.",
                false,
            )),
        }
    }

    /// O estado de repouso ao qual o retry devolve a reuniao.
    ///
    /// **Falha nunca e terminal.** O retry volta ao repouso ANTERIOR, e nao ao
    /// inicio: uma analise que falhou nao manda transcrever de novo, porque a
    /// transcricao continua boa e refaze-la custaria minutos por nada.
    fn resting_state(self) -> MeetingStatus {
        match self {
            // Falha de audio nao tem repouso anterior: nao ha insumo a
            // preservar. Ela volta para `Interrupted` para que a pessoa decida,
            // porque descartar por conta seria apagar gravacao (§9.2).
            Self::Audio => MeetingStatus::Interrupted,
            Self::Transcription => MeetingStatus::Recorded,
            Self::Analysis => MeetingStatus::Transcribed,
        }
    }
}

/// A maquina de estados da reuniao.
///
/// **Um enum, e nao tres campos.** `audioState`, `transcriptionState` e
/// `analysisState` separados permitiriam representar estados impossiveis —
/// analisando antes de transcrever, transcrevendo e analisando ao mesmo tempo.
/// O que E ortogonal, e por isso continua separado, e `lifecycle_state`
/// (ADR-015).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStatus {
    Recording,
    /// Gravacao suspensa pela pessoa.
    ///
    /// Os dois canais param de escrever JUNTOS, e o tempo pausado nao vira
    /// frame — entao nao vira duracao, porque `duration_ms` e medida em frames
    /// gravados e nunca por diferenca de relogio. Nao ha vao para reconstruir,
    /// e e por isso que este estado custou tao pouco.
    Paused,
    Stopping,
    /// Queda detectada na abertura. **Estado real, nao ausencia**: ele existe no
    /// banco com a duracao recuperada medida em disco.
    Interrupted,
    Recorded,
    Transcribing,
    /// Estado de REPOUSO, e nao de passagem. Com o Hermes offline a reuniao fica
    /// aqui, com transcricao completa e utilizavel — isso nao e falha (§20).
    Transcribed,
    Analyzing,
    Ready,
    Failed(FailedStage),
    Cancelled,
}

impl MeetingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Paused => "paused",
            Self::Stopping => "stopping",
            Self::Interrupted => "interrupted",
            Self::Recorded => "recorded",
            Self::Transcribing => "transcribing",
            Self::Transcribed => "transcribed",
            Self::Analyzing => "analyzing",
            Self::Ready => "ready",
            Self::Failed(_) => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// O par que vai para o banco: `status` e `failed_stage`.
    pub fn as_columns(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Failed(stage) => ("failed", Some(stage.as_str())),
            other => (other.as_str(), None),
        }
    }

    pub fn from_columns(status: &str, stage: Option<&str>) -> Result<Self, CoreError> {
        match status {
            "recording" => Ok(Self::Recording),
            "paused" => Ok(Self::Paused),
            "stopping" => Ok(Self::Stopping),
            "interrupted" => Ok(Self::Interrupted),
            "recorded" => Ok(Self::Recorded),
            "transcribing" => Ok(Self::Transcribing),
            "transcribed" => Ok(Self::Transcribed),
            "analyzing" => Ok(Self::Analyzing),
            "ready" => Ok(Self::Ready),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => {
                let stage = stage.ok_or_else(|| {
                    CoreError::new(
                        ErrorCode::DataIntegrity,
                        "Meeting em falha sem o estagio da falha.",
                        false,
                    )
                })?;
                Ok(Self::Failed(FailedStage::parse(stage)?))
            }
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de Meeting desconhecido.",
                false,
            )),
        }
    }

    /// A captura esta viva. E o que a reconciliacao da abertura procura (§9.1) e
    /// o que impede uma segunda gravacao de comecar.
    pub fn is_capturing(self) -> bool {
        matches!(self, Self::Recording | Self::Stopping)
    }

    /// Nao ha mais trabalho a fazer nesta reuniao sem o usuario pedir.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Ready | Self::Cancelled)
    }

    /// A reuniao tem audio em disco que pode ser processado.
    pub fn has_audio_to_process(self) -> bool {
        matches!(self, Self::Recorded | Self::Interrupted)
    }
}

/// O que pode acontecer com uma reuniao.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Transition {
    /// O usuario clicou em Parar.
    Stop,
    /// O usuario clicou em Pausar.
    Pause,
    /// O usuario clicou em Retomar.
    Resume,
    /// A captura terminou de fechar os arquivos.
    AudioSettled,
    /// A abertura encontrou a reuniao em captura e o processo anterior morreu.
    DetectInterrupted,
    /// O usuario escolheu [Processar] na tela de recuperacao.
    ProcessRecovered,
    /// O usuario escolheu [Descartar].
    Cancel,
    StartTranscription,
    TranscriptionDone,
    StartAnalysis,
    AnalysisDone,
    Fail(FailedStage),
    /// Tentar de novo depois de uma falha.
    Retry,
}

impl Transition {
    fn name(self) -> &'static str {
        match self {
            Self::Stop => "parar",
            Self::Pause => "pausar",
            Self::Resume => "retomar",
            Self::AudioSettled => "encerrar a captura de",
            Self::DetectInterrupted => "recuperar",
            Self::ProcessRecovered => "processar",
            Self::Cancel => "descartar",
            Self::StartTranscription => "transcrever",
            Self::TranscriptionDone => "concluir a transcricao de",
            Self::StartAnalysis => "analisar",
            Self::AnalysisDone => "concluir a analise de",
            Self::Fail(_) => "falhar",
            Self::Retry => "tentar de novo",
        }
    }
}

/// A unica funcao que muda `MeetingStatus`.
///
/// Espelha `attention::apply`: pura, com `now` injetado, e recusando o que nao
/// e transicao valida em vez de deixar passar. Um estado alcancado por caminho
/// nao previsto e um estado que nenhum teste cobre.
pub fn apply(
    meeting: &Meeting,
    transition: Transition,
    now: OffsetDateTime,
) -> Result<Meeting, CoreError> {
    use MeetingStatus::*;

    let refused = || {
        Err(CoreError::new(
            ErrorCode::InvalidTransition,
            format!(
                "Nao da para {} uma reuniao em {}.",
                transition.name(),
                meeting.status.as_str()
            ),
            false,
        ))
    };

    let mut next = meeting.clone();
    next.updated_at = now;

    match (meeting.status, transition) {
        (Recording, Transition::Stop) => {
            next.status = Stopping;
        }

        (Recording, Transition::Pause) => {
            next.status = Paused;
        }

        (Paused, Transition::Resume) => {
            next.status = Recording;
        }

        // Parar a partir de Paused vai para Stopping, igual a Recording: os
        // arquivos ainda precisam ser fechados, e `ended_at` continua sendo
        // carimbado no AudioSettled e nao aqui.
        (Paused, Transition::Stop) => {
            next.status = Stopping;
        }

        // A captura fechou os arquivos. `ended_at` e carimbado aqui, e nao no
        // clique: entre o clique e o fechamento ainda entrou audio, e a reuniao
        // dura ate o ultimo frame gravado.
        (Stopping, Transition::AudioSettled) => {
            next.status = Recorded;
            next.ended_at = Some(now);
        }

        // A abertura encontrou uma reuniao em captura. Nao existe outro caminho
        // para isso: o processo anterior morreu sem terminar (§9.1).
        (Recording | Stopping, Transition::DetectInterrupted) => {
            next.status = Interrupted;
            next.ended_at = Some(now);
        }

        (Interrupted, Transition::ProcessRecovered) => {
            next.status = Recorded;
        }

        // Descartar e permitido de qualquer estado que ainda tenha audio, e o
        // audio some junto. A LINHA fica, com `cancelled_at`, para que
        // "descartei uma reuniao de 1h18" seja um fato consultavel e nao um
        // buraco.
        (state, Transition::Cancel) if !state.is_terminal() => {
            next.status = Cancelled;
            next.cancelled_at = Some(now);
            if next.ended_at.is_none() {
                next.ended_at = Some(now);
            }
        }

        (Recorded, Transition::StartTranscription) => {
            next.status = Transcribing;
            next.failure = None;
        }

        (Transcribing, Transition::TranscriptionDone) => {
            next.status = Transcribed;
        }

        (Transcribed, Transition::StartAnalysis) => {
            next.status = Analyzing;
            next.failure = None;
        }

        (Analyzing, Transition::AnalysisDone) => {
            next.status = Ready;
        }

        // Reanalisar uma reuniao pronta e legitimo: o contrato pode ter mudado,
        // ou a primeira analise pode ter saido pobre.
        (Ready, Transition::StartAnalysis) => {
            next.status = Analyzing;
            next.failure = None;
        }

        // Falhar e permitido do estagio correspondente, e so dele. Uma falha de
        // transcricao numa reuniao que esta analisando seria um bug de
        // orquestracao, e recusa-la aqui e o que o transforma em erro visivel.
        (Transcribing, Transition::Fail(FailedStage::Transcription)) => {
            next.status = Failed(FailedStage::Transcription);
        }
        (Analyzing, Transition::Fail(FailedStage::Analysis)) => {
            next.status = Failed(FailedStage::Analysis);
        }
        (Recording | Stopping | Interrupted, Transition::Fail(FailedStage::Audio)) => {
            next.status = Failed(FailedStage::Audio);
            if next.ended_at.is_none() {
                next.ended_at = Some(now);
            }
        }

        (Failed(stage), Transition::Retry) => {
            next.status = stage.resting_state();
            next.failure = None;
        }

        _ => return refused(),
    }

    Ok(next)
}

/// O destino de um canal de audio.
///
/// Tres variantes, e nao um booleano, porque "nunca abriu" e "abriu e caiu aos
/// 32:10" pedem frases diferentes na tela (§20) e preservam quantidades
/// diferentes de audio.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ChannelOutcome {
    Capturing,
    Captured,
    Unavailable {
        reason: String,
    },
    /// Tudo ate `at_ms` esta gravado. O numero e o que a interface mostra, e por
    /// isso ele nunca pode ser estimado.
    Lost {
        at_ms: i64,
        reason: String,
    },
}

impl ChannelOutcome {
    pub fn as_columns(&self) -> (&'static str, Option<i64>, Option<String>) {
        match self {
            Self::Capturing => ("capturing", None, None),
            Self::Captured => ("captured", None, None),
            Self::Unavailable { reason } => ("unavailable", None, Some(reason.clone())),
            Self::Lost { at_ms, reason } => ("lost", Some(*at_ms), Some(reason.clone())),
        }
    }

    pub fn from_columns(
        state: &str,
        at_ms: Option<i64>,
        reason: Option<String>,
    ) -> Result<Self, CoreError> {
        match state {
            "capturing" => Ok(Self::Capturing),
            "captured" => Ok(Self::Captured),
            "unavailable" => Ok(Self::Unavailable {
                reason: reason.unwrap_or_default(),
            }),
            "lost" => Ok(Self::Lost {
                at_ms: at_ms.unwrap_or(0),
                reason: reason.unwrap_or_default(),
            }),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de canal de audio desconhecido.",
                false,
            )),
        }
    }

    /// O canal produziu audio utilizavel.
    pub fn has_audio(&self) -> bool {
        matches!(self, Self::Capturing | Self::Captured | Self::Lost { .. })
    }
}

/// Quanto tempo o audio temporario fica.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRetention {
    /// O padrao, e a preferencia declarada do proprietario.
    #[default]
    DeleteAfterProcessing,
    Keep24h,
    Keep,
}

impl AudioRetention {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeleteAfterProcessing => "delete_after_processing",
            Self::Keep24h => "keep_24h",
            Self::Keep => "keep",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "delete_after_processing" => Ok(Self::DeleteAfterProcessing),
            "keep_24h" => Ok(Self::Keep24h),
            "keep" => Ok(Self::Keep),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Politica de retencao de audio desconhecida.",
                false,
            )),
        }
    }
}

/// A falha que parou a reuniao, em forma legivel.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingFailure {
    pub stage: FailedStage,
    /// Mensagem para a PESSOA. Nunca contem texto de transcricao (§16.3).
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: MeetingId,
    pub title: String,
    pub status: MeetingStatus,
    pub lifecycle_state: LifecycleState,
    pub source: MeetingSource,

    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    /// Medido em FRAMES GRAVADOS, nunca por diferenca de relogio. Se um canal
    /// perdeu quatro segundos, a duracao precisa refletir o que existe.
    pub duration_ms: i64,

    pub project_id: Option<ProjectId>,

    /// Relativo ao diretorio de dados. Nunca vem do renderer (§18).
    pub audio_dir: String,
    pub retention: AudioRetention,
    #[serde(with = "time::serde::rfc3339::option")]
    pub audio_deleted_at: Option<OffsetDateTime>,

    pub mic: ChannelOutcome,
    pub system: ChannelOutcome,

    pub failure: Option<MeetingFailure>,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub cancelled_at: Option<OffsetDateTime>,
    /// O que quem gravou escreveu durante a reuniao.
    ///
    /// Vazio significa "ninguem escreveu", e nao "falta dado" — por isso e
    /// String e nao Option. Sobe ao Hermes como CONTEXTO e nao gera item: o
    /// prompt exige `segment` por item, e uma nota nao foi dita, foi escrita.
    #[serde(default)]
    pub notes: String,
}

impl Meeting {
    /// O audio pode ser apagado agora?
    ///
    /// A regra e CONSERVADORA de proposito. Ela responde `false` em toda duvida,
    /// porque o custo de errar para um lado e um diretorio ocupando disco e o
    /// custo de errar para o outro e uma reuniao perdida (§16.1).
    pub fn audio_may_be_deleted(&self, now: OffsetDateTime) -> bool {
        if self.audio_deleted_at.is_some() {
            return false;
        }
        match self.status {
            // Descartada pelo usuario: ele pediu.
            MeetingStatus::Cancelled => true,
            // Processada com sucesso. `Transcribed` conta porque a analise pode
            // ter sido recusada por escolha, e nesse caso o processamento
            // terminou (§16.1).
            MeetingStatus::Ready | MeetingStatus::Transcribed => match self.retention {
                AudioRetention::DeleteAfterProcessing => true,
                AudioRetention::Keep24h => self
                    .ended_at
                    .is_some_and(|ended| now - ended >= time::Duration::hours(24)),
                AudioRetention::Keep => false,
            },
            // Todo o resto, incluindo `Failed`: o audio e o insumo do retry, e
            // apaga-lo transformaria uma falha recuperavel em perda.
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewMeeting {
    pub id: MeetingId,
    pub title: String,
    pub source: MeetingSource,
    pub started_at: OffsetDateTime,
    pub project_id: Option<ProjectId>,
    pub audio_dir: String,
    pub retention: AudioRetention,
}

impl NewMeeting {
    /// O titulo nasce do relogio, e nao de um formulario.
    ///
    /// `VISION.md` §14: se a funcionalidade exigir que o usuario pare para
    /// alimentar o sistema, o sistema deveria fazer esse trabalho. Pedir um
    /// titulo antes de gravar seria exatamente isso — e o titulo e editavel
    /// depois, quando ela ja sabe do que a reuniao tratou.
    pub fn start(
        title: &str,
        source: MeetingSource,
        project_id: Option<ProjectId>,
        started_at: OffsetDateTime,
    ) -> Self {
        let id = MeetingId::new();
        let title = title.trim();
        let title = if title.is_empty() {
            format!(
                "Reuniao de {:02}/{:02} {:02}:{:02}",
                started_at.day(),
                started_at.month() as u8,
                started_at.hour(),
                started_at.minute()
            )
        } else {
            title.to_owned()
        };
        Self {
            id,
            title,
            source,
            started_at,
            project_id,
            // Derivado do id, e nunca recebido de fora: e o que impede um path
            // vindo do renderer de escapar do diretorio de dados (§18).
            audio_dir: format!("meetings/{id}"),
            retention: AudioRetention::default(),
        }
    }
}

/// De qual canal um trecho de transcricao veio.
///
/// **Esta e a informacao que a V1 protege acima de qualquer outra.** MIC e o
/// usuario local, SYSTEM sao os remotos, e e essa distincao que separa "o que EU
/// prometi" de "o que outros disseram" com certeza em vez de probabilidade.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Mic,
    System,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::System => "system",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "mic" => Ok(Self::Mic),
            "system" => Ok(Self::System),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Canal de transcricao desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: SegmentId,
    pub meeting_id: MeetingId,
    pub seq: i64,
    /// Relativo ao inicio da reuniao, e COMUM aos dois canais. Isso so e
    /// verdade por causa do keep-alive de silencio provado na Fase 1 — sem ele
    /// o canal SYSTEM para no silencio e as duas linhas do tempo divergem.
    pub start_ms: i64,
    pub end_ms: i64,
    pub channel: Channel,
    pub text: String,
    /// `None` na V1. Reservado para diarizacao (V2), que nunca pode alterar a
    /// atribuicao de canal.
    pub speaker: Option<String>,
    pub confidence: Option<f32>,
}

/// Um segmento antes de entrar no banco, ainda sem `seq` global.
///
/// Existe separado porque `seq` so pode ser atribuido depois que os DOIS canais
/// foram transcritos — e um campo que fica errado entre a criacao e a
/// intercalacao e um campo que alguem vai ler no meio.
#[derive(Clone, Debug)]
pub struct RawSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f32>,
}

/// Intercala os segmentos dos dois canais numa unica ordem de leitura.
///
/// Empate de `start_ms` resolve com MIC primeiro. E arbitrario, e isso esta
/// certo: o que uma ordenacao precisa ser e DETERMINISTICA, para que a mesma
/// transcricao nao mude de ordem entre duas aberturas.
pub fn interleave(
    meeting_id: MeetingId,
    mic: Vec<RawSegment>,
    system: Vec<RawSegment>,
) -> Vec<TranscriptSegment> {
    let mut all: Vec<(Channel, RawSegment)> = Vec::with_capacity(mic.len() + system.len());
    all.extend(mic.into_iter().map(|segment| (Channel::Mic, segment)));
    all.extend(system.into_iter().map(|segment| (Channel::System, segment)));

    // `sort_by` estavel + chave (start_ms, canal): o canal entra na chave para
    // que o empate nao dependa da ordem em que os dois vetores foram
    // concatenados.
    all.sort_by(|left, right| {
        left.1
            .start_ms
            .cmp(&right.1.start_ms)
            .then_with(|| match (left.0, right.0) {
                (Channel::Mic, Channel::System) => std::cmp::Ordering::Less,
                (Channel::System, Channel::Mic) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            })
    });

    all.into_iter()
        .enumerate()
        .map(|(index, (channel, raw))| TranscriptSegment {
            id: SegmentId::new(),
            meeting_id,
            seq: index as i64,
            start_ms: raw.start_ms,
            end_ms: raw.end_ms,
            channel,
            text: raw.text,
            speaker: None,
            confidence: raw.confidence,
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingAnalysis {
    pub meeting_id: MeetingId,
    pub summary: String,
    pub model: String,
    #[serde(with = "time::serde::rfc3339")]
    pub produced_at: OffsetDateTime,
    /// Quantas janelas de transcricao foram enviadas. **Aparece na interface**:
    /// um corte de cobertura que nao aparece na tela le-se como "cobriu tudo"
    /// quando nao cobriu (§11.4).
    pub windows: u32,
}

/// O tipo de um item extraido da reuniao.
///
/// **Uma tabela com `kind`, e nao oito tabelas.** E o argumento textual da
/// ADR-025, que enfrentou a mesma escolha: parte vira tabela propria quando
/// precisar de lifecycle ou consulta propria, e nenhum destes precisa.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightKind {
    Decision,
    MyAction,
    OtherAction,
    Deadline,
    FollowUp,
    OpenQuestion,
    Risk,
    Topic,
}

impl InsightKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::MyAction => "my_action",
            Self::OtherAction => "other_action",
            Self::Deadline => "deadline",
            Self::FollowUp => "follow_up",
            Self::OpenQuestion => "open_question",
            Self::Risk => "risk",
            Self::Topic => "topic",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "decision" => Ok(Self::Decision),
            "my_action" => Ok(Self::MyAction),
            "other_action" => Ok(Self::OtherAction),
            "deadline" => Ok(Self::Deadline),
            "follow_up" => Ok(Self::FollowUp),
            "open_question" => Ok(Self::OpenQuestion),
            "risk" => Ok(Self::Risk),
            "topic" => Ok(Self::Topic),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de item de reuniao desconhecido.",
                false,
            )),
        }
    }

    /// Este tipo pode virar Task?
    ///
    /// Decisao e topico nao viram: eles registram o que foi resolvido, nao o que
    /// falta fazer. Oferecer "criar Task" numa decisao ensinaria a tratar
    /// registro como pendencia.
    pub fn is_actionable(self) -> bool {
        matches!(
            self,
            Self::MyAction | Self::OtherAction | Self::Deadline | Self::FollowUp
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Nivel de confianca desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightStatus {
    Proposed,
    Accepted,
    Dismissed,
}

impl InsightStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "dismissed" => Ok(Self::Dismissed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de item de reuniao desconhecido.",
                false,
            )),
        }
    }
}

/// Referencia a um trecho da transcricao.
///
/// **O texto da citacao nao e guardado.** Ele E o texto do segmento. Isso atende
/// ao pedido de nao duplicar texto e compra algo melhor: a evidencia nao pode
/// divergir da transcricao, porque ela e a transcricao.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingEvidence {
    pub segment_id: SegmentId,
    pub seq: i64,
    pub char_start: Option<u32>,
    pub char_end: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingInsight {
    pub id: InsightId,
    pub meeting_id: MeetingId,
    pub kind: InsightKind,
    pub seq: i64,
    pub text: String,
    /// Como foi dito. **Nao e chave estrangeira para pessoa**: nao existe
    /// entidade Pessoa no M/OS, e inventar uma a partir de um nome falado
    /// criaria um cadastro que ninguem pediu.
    pub owner: Option<String>,
    /// O texto natural do prazo — "amanha", "sexta". **Nunca um instante.**
    /// Resolver na analise congelaria uma interpretacao; resolver na confirmacao
    /// poe a interpretacao na tela, que e o que o `UX-PRINCIPLES` §19 pede.
    pub due_hint: Option<String>,
    pub confidence: Confidence,
    pub status: InsightStatus,
    pub created_task_id: Option<TaskId>,
    pub created_reminder_id: Option<ReminderId>,
    pub evidence: Vec<MeetingEvidence>,
}

impl MeetingInsight {
    /// Pode virar Task com UM clique, dentro de uma criacao em lote?
    ///
    /// Duas regras deterministicas, e elas sobrepoem o modelo (§12.3):
    ///
    /// 1. **sem evidencia valida, nao entra.** O Meeting Agent nao apresenta
    ///    inferencia como fato sem proveniencia;
    /// 2. **`confidence: low` nao entra.** "Talvez a gente possa revisar isso
    ///    amanha" pode virar Task; nao vira Task junto com outras seis num
    ///    unico clique.
    ///
    /// Nenhuma das duas impede a criacao MANUAL, que abre o item e exige
    /// edicao. Elas governam o lote.
    pub fn eligible_for_bulk(&self) -> bool {
        self.status == InsightStatus::Proposed
            && self.kind.is_actionable()
            && self.confidence != Confidence::Low
            && !self.evidence.is_empty()
    }
}

/// O que pode dar errado ao transcrever.
///
/// Cada variante existe porque pede uma FRASE diferente na tela: "instale o
/// modelo" e "o áudio sumiu" nao sao o mesmo problema, e um erro unico obrigaria
/// a interface a adivinhar qual dos dois aconteceu.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum TranscriptionError {
    /// Nenhum provider foi configurado ainda.
    NotConfigured,
    /// O binario ou o modelo nao estao onde a configuracao diz.
    MissingRuntime { detail: String },
    /// O audio que deveria ser transcrito nao existe ou esta vazio.
    NoAudio,
    /// O provider rodou e falhou.
    Failed { detail: String },
    /// O provider rodou e devolveu algo que nao da para ler.
    Unreadable { detail: String },
    Cancelled,
}

impl std::fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "A transcricao local ainda nao foi configurada em Settings."
            ),
            Self::MissingRuntime { detail } => {
                write!(f, "O transcritor local nao foi encontrado: {detail}")
            }
            Self::NoAudio => write!(f, "Nao ha audio para transcrever."),
            Self::Failed { detail } => write!(f, "A transcricao falhou: {detail}"),
            Self::Unreadable { detail } => {
                write!(f, "O transcritor devolveu um resultado ilegivel: {detail}")
            }
            Self::Cancelled => write!(f, "A transcricao foi interrompida."),
        }
    }
}

impl std::error::Error for TranscriptionError {}

/// Um pedido de transcricao de UM canal.
///
/// Um canal por chamada, e nao os dois: eles sao transcritos independentemente e
/// so depois intercalados por `interleave`. Passar os dois juntos convidaria um
/// provider a misturá-los — e a separacao MIC/SYSTEM e a informacao que a V1
/// protege acima de qualquer outra.
#[derive(Clone, Debug)]
pub struct TranscriptionRequest<'a> {
    /// WAV de 16 kHz mono. Quem o monta e o adapter de audio.
    pub audio: &'a std::path::Path,
    pub channel: Channel,
    /// `None` deixa o provider detectar. Em portugues, declarar ajuda.
    pub language: Option<&'a str>,
}

/// A porta da transcricao.
///
/// **Meeting nao conhece provider. Provider nao conhece Meeting.** E a mesma
/// fronteira que `ports.rs` desenha para a persistencia, e ela e o que permite
/// trocar whisper local por outra coisa sem tocar no dominio.
pub trait TranscriptionProvider: Send + Sync {
    /// Nome legivel, para aparecer em `MeetingAnalysis.model` e em Settings.
    fn name(&self) -> String;

    /// O provider consegue rodar agora?
    ///
    /// Existe separado de `transcribe` para que Settings possa dizer "pronto" ou
    /// "falta o modelo" ANTES de uma reuniao de uma hora depender disso.
    fn ready(&self) -> Result<(), TranscriptionError>;

    /// `progress` recebe `0.0..=1.0`. Ele e chamado de uma thread de trabalho, e
    /// nunca do fio da interface.
    fn transcribe(
        &self,
        request: TranscriptionRequest<'_>,
        progress: &dyn Fn(f32),
    ) -> Result<Vec<RawSegment>, TranscriptionError>;
}

/// Limpa o que os modelos de fala produzem e que nao e fala.
///
/// Whisper marca musica, silencio e ruido com colchetes e parenteses —
/// `[Música]`, `(inaudible)`, `[BLANK_AUDIO]`. Numa reuniao isso e ruido puro:
/// vira segmento, entra na busca, e pode virar evidencia de um item de acao.
/// Descartar aqui, no dominio, garante que todo provider herde a regra.
pub fn is_speech(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Um segmento que e SO um marcador entre delimitadores nao e fala. Um que
    // contem um marcador no meio de uma frase continua sendo.
    let inteiro_entre_delimitadores = (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('*') && trimmed.ends_with('*'));
    if inteiro_entre_delimitadores {
        return false;
    }
    // Reticencias e pontuacao solta tambem nao sao fala.
    trimmed.chars().any(char::is_alphanumeric)
}

/// A partir de quantas repeticoes seguidas deixa de ser fala.
///
/// **Tres, e nao duas.** "Uhum, uhum" numa ligacao e resposta de gente; vinte e
/// quatro "Tchau" seguidos e o decodificador girando no silencio depois de a
/// ligacao ja ter acabado. Duas e o limite do que uma pessoa faz sem querer.
const REPETICOES_ATE_VIRAR_LACO: usize = 3;

/// Duas falas sao a mesma para efeito de laco.
///
/// Ignora caixa e espaco em volta porque o decodificador varia os dois no meio
/// do proprio laco — "Beleza." e " beleza. " sao a mesma volta da roda.
fn mesma_fala(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Junta laco de repeticao num segmento so.
///
/// Existe porque NENHUMA configuracao do whisper matou o laco: as onze rodadas
/// que originaram esta regra deixaram de 3 a 10 repeticoes, com e sem VAD, com e
/// sem supressao de nao-fala. Sobrando no provider, a regra tem que estar aqui —
/// no dominio, junto do `is_speech`, para nao depender de quem escreveu o
/// adapter.
///
/// O segmento que sobra cobre o intervalo INTEIRO do laco: um salto na
/// transcricao precisa pousar no trecho que a evidencia aponta.
fn colapsar_lacos(segments: Vec<RawSegment>) -> Vec<RawSegment> {
    fn descarregar(grupo: &mut Vec<RawSegment>, fora: &mut Vec<RawSegment>) {
        if grupo.len() >= REPETICOES_ATE_VIRAR_LACO {
            let fim = grupo.iter().map(|s| s.end_ms).max().unwrap_or_default();
            let mut primeiro = grupo.remove(0);
            primeiro.end_ms = fim;
            fora.push(primeiro);
            grupo.clear();
        } else {
            fora.append(grupo);
        }
    }

    let mut fora: Vec<RawSegment> = Vec::with_capacity(segments.len());
    let mut grupo: Vec<RawSegment> = Vec::new();

    for segmento in segments {
        let continua = grupo
            .last()
            .is_some_and(|anterior| mesma_fala(&anterior.text, &segmento.text));
        if !continua {
            descarregar(&mut grupo, &mut fora);
        }
        grupo.push(segmento);
    }
    descarregar(&mut grupo, &mut fora);

    fora
}

/// Prepara os segmentos crus de um canal para virarem transcricao.
///
/// Descarta o que nao e fala, junta o que ficou colado e ordena. Roda no
/// dominio, e nao em cada provider, para que a regra nao dependa de quem
/// implementou o adapter.
pub fn clean_segments(mut segments: Vec<RawSegment>) -> Vec<RawSegment> {
    segments.retain(|segment| is_speech(&segment.text));
    segments.sort_by_key(|segment| segment.start_ms);
    for segment in &mut segments {
        segment.text = segment.text.trim().to_owned();
        // Um segmento com fim antes do inicio quebra a evidencia: o salto na
        // transcricao pousaria antes do trecho. Corrigir aqui e melhor que
        // confiar em todo provider acertar.
        if segment.end_ms < segment.start_ms {
            segment.end_ms = segment.start_ms;
        }
    }
    // O colapso vem DEPOIS da ordenacao: laco e fenomeno de vizinhanca, e
    // vizinhanca so existe depois de ordenar.
    colapsar_lacos(segments)
}

/// O que o usuario confirmou ao aceitar um item de reuniao.
///
/// Ela chega **do preview**, e nao do modelo: titulo, Project e prazo sao
/// editaveis antes de confirmar. E o §13.2 em forma de tipo — quem clicou em
/// "Criar Task" escolheu aquilo, e quem falou uma frase pode ter sido mal
/// entendido.
#[derive(Clone, Debug)]
pub struct AcceptInsight {
    pub insight_id: InsightId,
    pub title: String,
    pub description: String,
    pub project_id: Option<ProjectId>,
    /// Quando criar um Reminder junto. `None` cria so a Task.
    ///
    /// **Instante, e nao `due_hint`.** O texto natural ficou no item; a
    /// interpretacao dele acontece na tela, onde a pessoa ve e corrige. Aceitar
    /// "amanha" aqui obrigaria o dominio a adivinhar que horas.
    pub remind_at: Option<OffsetDateTime>,
}

/// O que a aceitacao produziu.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedInsight {
    pub insight: MeetingInsight,
    pub task_id: TaskId,
    pub reminder_id: Option<ReminderId>,
}

/// O preview de um item, antes de ele virar Task.
///
/// **Todo item mostra preview, inclusive os de confianca alta.** O risco
/// classifica a consequencia da acao; o preview responde a outra coisa — a
/// incerteza da interpretacao. Numa reuniao isso e extremo: ninguem escolheu
/// nada, alguem so falou.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightPreview {
    pub insight_id: InsightId,
    pub kind: InsightKind,
    /// O titulo sugerido para a Task. Editavel.
    pub title: String,
    pub owner: Option<String>,
    /// O prazo COMO FOI DITO. A tela resolve; o dominio nao.
    pub due_hint: Option<String>,
    pub confidence: Confidence,
    /// Quantas evidencias sustentam o item. Zero significa sem proveniencia.
    pub evidence_count: usize,
    /// O item pode entrar numa criacao em lote?
    pub eligible_for_bulk: bool,
    /// Por que ele NAO pode, quando nao pode. Vazio quando pode.
    pub blocked_reason: String,
}

impl MeetingInsight {
    /// Monta o preview deste item.
    pub fn preview(&self) -> InsightPreview {
        let blocked_reason = if self.status != InsightStatus::Proposed {
            "Este item ja foi resolvido.".to_owned()
        } else if !self.kind.is_actionable() {
            "Decisoes e topicos registram o que ficou resolvido, e nao o que falta fazer."
                .to_owned()
        } else if self.evidence.is_empty() {
            // A frase diz o que fazer, e nao so o que falta: sem ela, o botao
            // desabilitado seria um beco sem saida (`UX-PRINCIPLES` §64).
            "Sem evidencia na transcricao. Confira o item antes de criar a Task.".to_owned()
        } else if self.confidence == Confidence::Low {
            "Confianca baixa: a frase original era hipotetica. Confira antes de criar."
                .to_owned()
        } else {
            String::new()
        };

        InsightPreview {
            insight_id: self.id,
            kind: self.kind,
            title: self.text.clone(),
            owner: self.owner.clone(),
            due_hint: self.due_hint.clone(),
            confidence: self.confidence,
            evidence_count: self.evidence.len(),
            eligible_for_bulk: self.eligible_for_bulk(),
            blocked_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn now() -> OffsetDateTime {
        datetime!(2026-08-18 14:00:00 UTC)
    }

    fn meeting(status: MeetingStatus) -> Meeting {
        Meeting {
            id: MeetingId::new(),
            title: "NexoDoc".into(),
            status,
            lifecycle_state: LifecycleState::Active,
            source: MeetingSource::Manual,
            started_at: now(),
            ended_at: None,
            duration_ms: 0,
            project_id: None,
            audio_dir: "meetings/x".into(),
            retention: AudioRetention::default(),
            audio_deleted_at: None,
            mic: ChannelOutcome::Capturing,
            system: ChannelOutcome::Capturing,
            failure: None,
            created_at: now(),
            updated_at: now(),
            cancelled_at: None,
            notes: String::new(),
        }
    }

    #[test]
    fn pausar_e_retomar_andam_entre_recording_e_paused() {
        let gravando = meeting(MeetingStatus::Recording);

        let pausada = apply(&gravando, Transition::Pause, now()).unwrap();
        assert_eq!(pausada.status, MeetingStatus::Paused);
        // Pausar NAO carimba fim: a reuniao nao acabou, ela esta esperando.
        assert!(pausada.ended_at.is_none());

        let retomada = apply(&pausada, Transition::Resume, now()).unwrap();
        assert_eq!(retomada.status, MeetingStatus::Recording);
    }

    #[test]
    fn parar_funciona_a_partir_de_paused() {
        let pausada = meeting(MeetingStatus::Paused);
        let parando = apply(&pausada, Transition::Stop, now()).unwrap();
        assert_eq!(parando.status, MeetingStatus::Stopping);
    }

    #[test]
    fn pausa_recusada_fora_de_recording() {
        for estado in [
            MeetingStatus::Recorded,
            MeetingStatus::Transcribed,
            MeetingStatus::Ready,
            MeetingStatus::Paused,
        ] {
            assert!(
                apply(&meeting(estado), Transition::Pause, now()).is_err(),
                "Pause deveria ser recusado em {}",
                estado.as_str()
            );
        }
        // E retomar so faz sentido a partir de Paused.
        assert!(apply(&meeting(MeetingStatus::Recording), Transition::Resume, now()).is_err());
    }

    #[test]
    fn o_caminho_feliz_inteiro() {
        let mut current = meeting(MeetingStatus::Recording);
        for (transition, expected) in [
            (Transition::Stop, MeetingStatus::Stopping),
            (Transition::AudioSettled, MeetingStatus::Recorded),
            (Transition::StartTranscription, MeetingStatus::Transcribing),
            (Transition::TranscriptionDone, MeetingStatus::Transcribed),
            (Transition::StartAnalysis, MeetingStatus::Analyzing),
            (Transition::AnalysisDone, MeetingStatus::Ready),
        ] {
            current = apply(&current, transition, now()).unwrap();
            assert_eq!(current.status, expected);
        }
    }

    #[test]
    fn parar_carimba_o_fim_no_fechamento_e_nao_no_clique() {
        let recording = meeting(MeetingStatus::Recording);
        let stopping = apply(&recording, Transition::Stop, now()).unwrap();
        assert!(stopping.ended_at.is_none(), "o clique nao encerra");

        let recorded = apply(&stopping, Transition::AudioSettled, now()).unwrap();
        assert_eq!(recorded.ended_at, Some(now()));
    }

    #[test]
    fn queda_vira_interrupted_e_nao_recorded() {
        let interrupted =
            apply(&meeting(MeetingStatus::Recording), Transition::DetectInterrupted, now()).unwrap();
        assert_eq!(interrupted.status, MeetingStatus::Interrupted);

        let processed =
            apply(&interrupted, Transition::ProcessRecovered, now()).unwrap();
        assert_eq!(processed.status, MeetingStatus::Recorded);
    }

    #[test]
    fn descartar_carimba_e_preserva_a_linha() {
        let cancelled =
            apply(&meeting(MeetingStatus::Interrupted), Transition::Cancel, now()).unwrap();
        assert_eq!(cancelled.status, MeetingStatus::Cancelled);
        assert_eq!(cancelled.cancelled_at, Some(now()));
    }

    #[test]
    fn nao_da_para_descartar_o_que_ja_terminou() {
        let error = apply(&meeting(MeetingStatus::Ready), Transition::Cancel, now()).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidTransition);
    }

    #[test]
    fn transcribed_e_repouso_e_analise_nao_e_obrigatoria() {
        let transcribed = meeting(MeetingStatus::Transcribed);
        // Hermes offline: a reuniao simplesmente fica aqui, e isso nao e falha.
        assert!(!transcribed.status.is_terminal());
        assert_eq!(
            apply(&transcribed, Transition::StartAnalysis, now())
                .unwrap()
                .status,
            MeetingStatus::Analyzing
        );
    }

    #[test]
    fn retry_volta_ao_repouso_anterior_e_nao_ao_inicio() {
        let failed_analysis = {
            let mut m = meeting(MeetingStatus::Failed(FailedStage::Analysis));
            m.failure = Some(MeetingFailure {
                stage: FailedStage::Analysis,
                message: "o formato nao deu para ler".into(),
            });
            m
        };
        let retried = apply(&failed_analysis, Transition::Retry, now()).unwrap();
        assert_eq!(
            retried.status,
            MeetingStatus::Transcribed,
            "analise que falha nao manda transcrever de novo"
        );
        assert!(retried.failure.is_none());

        let failed_transcription = meeting(MeetingStatus::Failed(FailedStage::Transcription));
        assert_eq!(
            apply(&failed_transcription, Transition::Retry, now())
                .unwrap()
                .status,
            MeetingStatus::Recorded
        );
    }

    #[test]
    fn falha_de_estagio_errado_e_recusada() {
        // Falha de transcricao numa reuniao analisando e bug de orquestracao.
        let error = apply(
            &meeting(MeetingStatus::Analyzing),
            Transition::Fail(FailedStage::Transcription),
            now(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidTransition);
    }

    #[test]
    fn transicoes_invalidas_sao_recusadas() {
        for (status, transition) in [
            (MeetingStatus::Recording, Transition::StartTranscription),
            (MeetingStatus::Recorded, Transition::StartAnalysis),
            (MeetingStatus::Transcribing, Transition::AnalysisDone),
            (MeetingStatus::Ready, Transition::Stop),
            (MeetingStatus::Cancelled, Transition::ProcessRecovered),
            (MeetingStatus::Transcribed, Transition::TranscriptionDone),
        ] {
            let error = apply(&meeting(status), transition, now()).unwrap_err();
            assert_eq!(
                error.code,
                ErrorCode::InvalidTransition,
                "{status:?} + {transition:?} deveria ser recusada"
            );
        }
    }

    #[test]
    fn status_atravessa_o_banco_com_o_estagio_da_falha() {
        let status = MeetingStatus::Failed(FailedStage::Analysis);
        let (text, stage) = status.as_columns();
        assert_eq!((text, stage), ("failed", Some("analysis")));
        assert_eq!(MeetingStatus::from_columns(text, stage).unwrap(), status);

        // `failed` sem estagio e integridade quebrada, e nao um default.
        assert_eq!(
            MeetingStatus::from_columns("failed", None).unwrap_err().code,
            ErrorCode::DataIntegrity
        );
    }

    #[test]
    fn audio_so_e_apagavel_quando_o_processamento_terminou() {
        let mut ready = meeting(MeetingStatus::Ready);
        ready.ended_at = Some(now());
        assert!(ready.audio_may_be_deleted(now()));

        // Falha NUNCA autoriza: o audio e o insumo do retry.
        for status in [
            MeetingStatus::Failed(FailedStage::Transcription),
            MeetingStatus::Failed(FailedStage::Analysis),
            MeetingStatus::Recorded,
            MeetingStatus::Interrupted,
            MeetingStatus::Recording,
        ] {
            let mut m = meeting(status);
            m.ended_at = Some(now());
            assert!(
                !m.audio_may_be_deleted(now()),
                "{status:?} nao pode autorizar apagar audio"
            );
        }
    }

    #[test]
    fn retencao_de_24h_espera_as_24h() {
        let mut m = meeting(MeetingStatus::Ready);
        m.retention = AudioRetention::Keep24h;
        m.ended_at = Some(now());
        assert!(!m.audio_may_be_deleted(now() + time::Duration::hours(23)));
        assert!(m.audio_may_be_deleted(now() + time::Duration::hours(24)));
    }

    #[test]
    fn retencao_keep_nunca_autoriza() {
        let mut m = meeting(MeetingStatus::Ready);
        m.retention = AudioRetention::Keep;
        m.ended_at = Some(now());
        assert!(!m.audio_may_be_deleted(now() + time::Duration::days(3650)));
    }

    #[test]
    fn audio_ja_apagado_nao_e_apagado_de_novo() {
        let mut m = meeting(MeetingStatus::Ready);
        m.ended_at = Some(now());
        m.audio_deleted_at = Some(now());
        assert!(!m.audio_may_be_deleted(now()));
    }

    #[test]
    fn o_titulo_nasce_do_relogio_quando_vazio() {
        let started = datetime!(2026-08-18 14:02:00 UTC);
        let meeting = NewMeeting::start("   ", MeetingSource::Manual, None, started);
        assert_eq!(meeting.title, "Reuniao de 18/08 14:02");
    }

    #[test]
    fn o_diretorio_de_audio_deriva_do_id() {
        let meeting = NewMeeting::start("x", MeetingSource::Manual, None, now());
        assert_eq!(meeting.audio_dir, format!("meetings/{}", meeting.id));
    }

    #[test]
    fn intercalar_ordena_por_tempo_com_mic_no_empate() {
        let id = MeetingId::new();
        let raw = |start: i64, text: &str| RawSegment {
            start_ms: start,
            end_ms: start + 1000,
            text: text.into(),
            confidence: None,
        };
        let segments = interleave(
            id,
            vec![raw(0, "eu primeiro"), raw(2000, "eu depois")],
            vec![raw(1000, "remoto no meio"), raw(0, "remoto no empate")],
        );

        let ordem: Vec<(&str, i64)> = segments
            .iter()
            .map(|s| (s.channel.as_str(), s.start_ms))
            .collect();
        assert_eq!(
            ordem,
            vec![("mic", 0), ("system", 0), ("system", 1000), ("mic", 2000)]
        );
        assert_eq!(
            segments.iter().map(|s| s.seq).collect::<Vec<_>>(),
            vec![0, 1, 2, 3],
            "seq e denso e comeca em zero"
        );
    }

    #[test]
    fn intercalar_com_um_canal_vazio_funciona() {
        let id = MeetingId::new();
        let segments = interleave(
            id,
            vec![RawSegment {
                start_ms: 0,
                end_ms: 10,
                text: "so eu".into(),
                confidence: None,
            }],
            Vec::new(),
        );
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].channel, Channel::Mic);
    }

    fn insight(kind: InsightKind, confidence: Confidence, evidence: usize) -> MeetingInsight {
        MeetingInsight {
            id: InsightId::new(),
            meeting_id: MeetingId::new(),
            kind,
            seq: 0,
            text: "Finalizar a apresentacao".into(),
            owner: None,
            due_hint: None,
            confidence,
            status: InsightStatus::Proposed,
            created_task_id: None,
            created_reminder_id: None,
            evidence: (0..evidence)
                .map(|seq| MeetingEvidence {
                    segment_id: SegmentId::new(),
                    seq: seq as i64,
                    char_start: None,
                    char_end: None,
                })
                .collect(),
        }
    }

    #[test]
    fn sem_evidencia_nao_entra_no_lote() {
        let orphan = insight(InsightKind::MyAction, Confidence::High, 0);
        assert!(!orphan.eligible_for_bulk());
        assert!(insight(InsightKind::MyAction, Confidence::High, 1).eligible_for_bulk());
    }

    #[test]
    fn confianca_baixa_nao_entra_no_lote() {
        assert!(!insight(InsightKind::MyAction, Confidence::Low, 2).eligible_for_bulk());
        assert!(insight(InsightKind::MyAction, Confidence::Medium, 2).eligible_for_bulk());
    }

    #[test]
    fn decisao_e_topico_nao_viram_task() {
        for kind in [InsightKind::Decision, InsightKind::Topic, InsightKind::Risk] {
            assert!(!kind.is_actionable(), "{kind:?} nao e acionavel");
            assert!(!insight(kind, Confidence::High, 2).eligible_for_bulk());
        }
        for kind in [
            InsightKind::MyAction,
            InsightKind::OtherAction,
            InsightKind::Deadline,
            InsightKind::FollowUp,
        ] {
            assert!(kind.is_actionable(), "{kind:?} deveria ser acionavel");
        }
    }

    #[test]
    fn item_ja_resolvido_nao_entra_no_lote() {
        let mut accepted = insight(InsightKind::MyAction, Confidence::High, 1);
        accepted.status = InsightStatus::Accepted;
        assert!(!accepted.eligible_for_bulk());

        let mut dismissed = insight(InsightKind::MyAction, Confidence::High, 1);
        dismissed.status = InsightStatus::Dismissed;
        assert!(!dismissed.eligible_for_bulk());
    }

    #[test]
    fn marcador_de_ruido_nao_e_fala() {
        for ruido in [
            "[Música]",
            "[BLANK_AUDIO]",
            "(inaudible)",
            "*risos*",
            "   ",
            "...",
            "---",
        ] {
            assert!(!is_speech(ruido), "{ruido:?} nao deveria ser fala");
        }
    }

    #[test]
    fn frase_com_marcador_no_meio_continua_sendo_fala() {
        assert!(is_speech("Eu termino [pausa] os slides amanha."));
        assert!(is_speech("Combinado."));
        assert!(is_speech("2026"));
    }

    #[test]
    fn a_limpeza_descarta_ruido_ordena_e_conserta_intervalo() {
        let raw = |start: i64, end: i64, text: &str| RawSegment {
            start_ms: start,
            end_ms: end,
            text: text.into(),
            confidence: None,
        };
        let limpos = clean_segments(vec![
            raw(3000, 4000, "  Depois  "),
            raw(1000, 2000, "[Música]"),
            // Fim antes do inicio: a evidencia pousaria antes do trecho.
            raw(500, 100, "Primeiro"),
        ]);

        assert_eq!(limpos.len(), 2, "o marcador de ruido sai");
        assert_eq!(limpos[0].text, "Primeiro");
        assert_eq!(limpos[0].end_ms, 500, "o intervalo invertido e corrigido");
        assert_eq!(limpos[1].text, "Depois", "e o texto vem aparado");
    }

    #[test]
    fn a_limpeza_de_um_canal_so_de_ruido_devolve_vazio() {
        let limpos = clean_segments(vec![RawSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "[BLANK_AUDIO]".into(),
            confidence: None,
        }]);
        assert!(limpos.is_empty());
    }

    #[test]
    fn o_preview_explica_por_que_um_item_nao_entra_no_lote() {
        let sem_evidencia = insight(InsightKind::MyAction, Confidence::High, 0);
        let preview = sem_evidencia.preview();
        assert!(!preview.eligible_for_bulk);
        assert!(
            preview.blocked_reason.contains("evidencia"),
            "a razao precisa nomear o que falta: {}",
            preview.blocked_reason
        );
        assert_eq!(preview.evidence_count, 0);

        let baixa = insight(InsightKind::MyAction, Confidence::Low, 2);
        assert!(baixa.preview().blocked_reason.contains("hipotetica"));

        let decisao = insight(InsightKind::Decision, Confidence::High, 2);
        assert!(decisao.preview().blocked_reason.contains("resolvido"));
    }

    #[test]
    fn o_preview_de_um_item_bom_nao_tem_bloqueio() {
        let bom = insight(InsightKind::MyAction, Confidence::High, 1);
        let preview = bom.preview();
        assert!(preview.eligible_for_bulk);
        assert!(preview.blocked_reason.is_empty());
        assert_eq!(preview.title, "Finalizar a apresentacao");
    }

    #[test]
    fn o_preview_carrega_o_prazo_como_foi_dito() {
        let mut item = insight(InsightKind::MyAction, Confidence::High, 1);
        item.due_hint = Some("sexta".into());
        assert_eq!(item.preview().due_hint.as_deref(), Some("sexta"));
    }

    #[test]
    fn canal_perdido_ainda_tem_audio() {
        assert!(ChannelOutcome::Lost {
            at_ms: 32_000,
            reason: "headset desconectado".into()
        }
        .has_audio());
        assert!(!ChannelOutcome::Unavailable {
            reason: "sem dispositivo".into()
        }
        .has_audio());
    }

    #[test]
    fn canal_atravessa_o_banco() {
        for outcome in [
            ChannelOutcome::Capturing,
            ChannelOutcome::Captured,
            ChannelOutcome::Unavailable {
                reason: "sem dispositivo".into(),
            },
            ChannelOutcome::Lost {
                at_ms: 32_000,
                reason: "headset desconectado".into(),
            },
        ] {
            let (state, at_ms, reason) = outcome.as_columns();
            assert_eq!(
                ChannelOutcome::from_columns(state, at_ms, reason).unwrap(),
                outcome
            );
        }
    }

    fn seg(inicio: i64, fim: i64, texto: &str) -> RawSegment {
        RawSegment {
            start_ms: inicio,
            end_ms: fim,
            text: texto.into(),
            confidence: None,
        }
    }

    #[test]
    fn laco_tres_repeticoes_viram_uma() {
        // O laco real da reuniao de 20/08: 24 "Tchau" seguidos no rabo mudo.
        let limpos = clean_segments(vec![
            seg(0, 1000, "Bom dia"),
            seg(1000, 2000, "Tchau."),
            seg(2000, 3000, "Tchau."),
            seg(3000, 4000, "Tchau."),
            seg(4000, 5000, "Ate mais"),
        ]);
        let textos: Vec<&str> = limpos.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(textos, vec!["Bom dia", "Tchau.", "Ate mais"]);

        // O colapsado cobre o intervalo INTEIRO do laco: do inicio do primeiro
        // ao fim do ultimo. Um salto na transcricao tem que pousar no trecho.
        assert_eq!(limpos[1].start_ms, 1000);
        assert_eq!(limpos[1].end_ms, 4000);
    }

    #[test]
    fn laco_duas_repeticoes_sao_fala() {
        // "Uhum, uhum" numa ligacao acontece. Vinte e quatro "Tchau", nao.
        let limpos = clean_segments(vec![seg(0, 500, "Uhum."), seg(500, 1000, "Uhum.")]);
        assert_eq!(limpos.len(), 2);
    }

    #[test]
    fn laco_so_junta_o_que_e_consecutivo() {
        let limpos = clean_segments(vec![
            seg(0, 100, "Sim."),
            seg(100, 200, "Sim."),
            seg(200, 300, "E ai?"),
            seg(300, 400, "Sim."),
        ]);
        assert_eq!(limpos.len(), 4);
    }

    #[test]
    fn laco_ignora_caixa_e_espaco_em_volta() {
        let limpos = clean_segments(vec![
            seg(0, 100, "Beleza."),
            seg(100, 200, " beleza. "),
            seg(200, 300, "BELEZA."),
        ]);
        assert_eq!(limpos.len(), 1);
        assert_eq!(limpos[0].text, "Beleza.");
        assert_eq!(limpos[0].end_ms, 300);
    }
}

//! O pipeline da reuniao: o que anda sozinho depois de parar de gravar.
//!
//! # Por que existe
//!
//! Ate a V2, transcrever e analisar eram BOTOES. A cadeia funcionava, e a
//! pessoa precisava lembrar de empurra-la: clicar em Transcrever, esperar,
//! clicar em Analisar, esperar. Quando o Hermes estava fora, a reuniao ia para
//! `failed` e ficava la ate alguem voltar. Quando o app caia no meio, a reuniao
//! ficava em `transcribing` para sempre — o §9.3 do `MEETING-AGENT.md`
//! prometia o conserto, e ele nunca foi escrito.
//!
//! Aqui mora a POLITICA, pura: quando tentar de novo, quando desistir, o que
//! conta como falha passageira, e o que a pessoa ve. Quem executa e o laco do
//! desktop, que so obedece.
//!
//! Spec: `docs/superpowers/specs/2026-09-16-meeting-agent-v2-design.md` §2.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{CoreError, ErrorCode, FailedStage, Meeting, MeetingId, MeetingStatus};

/// O estagio que o job representa.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStage {
    Transcription,
    Analysis,
}

impl JobStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcription => "transcription",
            Self::Analysis => "analysis",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "transcription" => Ok(Self::Transcription),
            "analysis" => Ok(Self::Analysis),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estagio de job desconhecido.",
                false,
            )),
        }
    }

    /// Quantas tentativas antes de pedir a pessoa.
    ///
    /// **A analise aguenta mais.** O Hermes mora numa VPS atras de um tunel, e
    /// ele ficar fora por uma tarde e normal. O whisper e local: falhar quatro
    /// vezes seguidas nao e azar, e defeito — insistir so esquenta a GPU.
    pub fn max_attempts(self) -> u32 {
        match self {
            Self::Transcription => 4,
            Self::Analysis => 6,
        }
    }

    pub fn failed_stage(self) -> FailedStage {
        match self {
            Self::Transcription => FailedStage::Transcription,
            Self::Analysis => FailedStage::Analysis,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    WaitingRetry,
    /// Nao anda sozinho. Ou esgotou as tentativas, ou falta configuracao — o
    /// que muda e se ele volta sozinho quando a configuracao aparecer.
    NeedsAttention,
    Done,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingRetry => "waiting_retry",
            Self::NeedsAttention => "needs_attention",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "waiting_retry" => Ok(Self::WaitingRetry),
            "needs_attention" => Ok(Self::NeedsAttention),
            "done" => Ok(Self::Done),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de job desconhecido.",
                false,
            )),
        }
    }

    /// Ainda ha algo a fazer.
    pub fn is_open(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Running | Self::WaitingRetry | Self::NeedsAttention
        )
    }
}

/// Que tipo de falha foi.
///
/// A classe decide o que acontece depois, e e por isso que ela existe separada
/// da mensagem: "o tunel caiu" e "o transcritor nao esta instalado" sao ambos
/// erros, e pedem respostas opostas — um espera e tenta, o outro nao adianta
/// tentar ate alguem configurar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// Pode dar certo sozinho: rede, Hermes fora, whisper que morreu por
    /// memoria, disco temporariamente cheio.
    Transient,
    /// So da certo depois de a pessoa configurar algo. Nao gasta tentativa.
    Configuration,
    /// Nao adianta tentar de novo: nao ha audio, o consentimento foi revogado.
    Permanent,
}

/// Uma falha de estagio, ja classificada.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageFailure {
    /// Identificador estavel, para metrica e log: `hermes_unreachable`,
    /// `transcriber_missing`, `no_audio`...
    pub code: String,
    /// A frase para a PESSOA. Nunca contem fala (§16.3).
    pub message: String,
    pub class: FailureClass,
}

impl StageFailure {
    pub fn new(code: &str, message: impl Into<String>, class: FailureClass) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            class,
        }
    }
}

/// A escada de espera entre tentativas, em segundos.
///
/// 30 s pega o solavanco (o tunel reconectando); 2 e 10 min pegam a queda
/// curta; 30 min e 2 h pegam a tarde sem Hermes. Nenhuma espera e menor que o
/// intervalo do laco, entao nenhuma tentativa e "imediata" por acidente.
pub const RETRY_LADDER_SECS: [i64; 5] = [30, 120, 600, 1_800, 7_200];

/// O pipeline persistente de uma reuniao.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingJob {
    pub meeting_id: MeetingId,
    pub stage: JobStage,
    pub status: JobStatus,
    pub attempt_count: u32,
    /// `0..=1` DENTRO do estagio. Medido — o whisper diz; a analise conta
    /// janelas. Nunca estimado.
    pub progress: f32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub finished_at: Option<OffsetDateTime>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_retry_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// O que fazer depois de uma falha.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AfterFailure {
    /// Espera e tenta de novo. A reuniao volta ao repouso, e NAO vira `failed`.
    RetryAt(OffsetDateTime),
    /// Falta configuracao. Nao gasta tentativa, e volta quando ela aparecer.
    WaitForConfiguration,
    /// Acabou o que se tenta sozinho. A reuniao vira `failed(stage)` — com o
    /// insumo do estagio anterior intacto.
    GiveUp,
}

impl MeetingJob {
    /// Um job novo, pronto para o laco pegar.
    pub fn queue(meeting_id: MeetingId, stage: JobStage, now: OffsetDateTime) -> Self {
        Self {
            meeting_id,
            stage,
            status: JobStatus::Queued,
            attempt_count: 0,
            progress: 0.0,
            started_at: None,
            finished_at: None,
            last_error_code: None,
            last_error_message: None,
            next_retry_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Passa para o proximo estagio, reaproveitando a linha.
    ///
    /// Uma linha por reuniao: o historico de tentativas do estagio anterior nao
    /// e contexto do seguinte, e carregar `attempt_count` da transcricao para a
    /// analise faria a analise desistir antes da hora.
    pub fn advance(&mut self, stage: JobStage, now: OffsetDateTime) {
        *self = Self {
            created_at: self.created_at,
            ..Self::queue(self.meeting_id, stage, now)
        };
    }

    /// Este job deve rodar agora?
    pub fn is_due(&self, now: OffsetDateTime) -> bool {
        match self.status {
            JobStatus::Queued => true,
            JobStatus::WaitingRetry => self.next_retry_at.is_none_or(|at| at <= now),
            _ => false,
        }
    }

    pub fn start(&mut self, now: OffsetDateTime) {
        self.status = JobStatus::Running;
        self.progress = 0.0;
        self.started_at = Some(now);
        self.finished_at = None;
        self.next_retry_at = None;
        self.updated_at = now;
    }

    pub fn set_progress(&mut self, fraction: f32, now: OffsetDateTime) {
        // Progresso so anda para a frente. Duas passadas do whisper reportam de
        // zero cada uma; quem soma e quem chama, e aqui a barra nunca volta.
        let clamped = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.progress = self.progress.max(clamped);
        self.updated_at = now;
    }

    pub fn succeed(&mut self, now: OffsetDateTime) {
        self.status = JobStatus::Done;
        self.progress = 1.0;
        self.finished_at = Some(now);
        self.last_error_code = None;
        self.last_error_message = None;
        self.next_retry_at = None;
        self.updated_at = now;
    }

    pub fn cancel(&mut self, now: OffsetDateTime) {
        self.status = JobStatus::Cancelled;
        self.next_retry_at = None;
        self.updated_at = now;
    }

    /// Registra uma falha e decide o que vem depois.
    ///
    /// A regra toda esta aqui, e nao no laco, para o laco nao ter onde errar:
    /// ele grava o job e obedece o `AfterFailure`.
    pub fn fail(&mut self, failure: &StageFailure, now: OffsetDateTime) -> AfterFailure {
        self.last_error_code = Some(failure.code.clone());
        self.last_error_message = Some(failure.message.clone());
        self.finished_at = Some(now);
        self.updated_at = now;

        match failure.class {
            FailureClass::Configuration => {
                self.status = JobStatus::NeedsAttention;
                self.next_retry_at = None;
                AfterFailure::WaitForConfiguration
            }
            FailureClass::Permanent => {
                self.attempt_count += 1;
                self.status = JobStatus::NeedsAttention;
                self.next_retry_at = None;
                AfterFailure::GiveUp
            }
            FailureClass::Transient => {
                self.attempt_count += 1;
                if self.attempt_count >= self.stage.max_attempts() {
                    self.status = JobStatus::NeedsAttention;
                    self.next_retry_at = None;
                    return AfterFailure::GiveUp;
                }
                let index = (self.attempt_count as usize - 1).min(RETRY_LADDER_SECS.len() - 1);
                let at = now + time::Duration::seconds(RETRY_LADDER_SECS[index]);
                self.status = JobStatus::WaitingRetry;
                self.next_retry_at = Some(at);
                AfterFailure::RetryAt(at)
            }
        }
    }

    /// A configuracao que faltava apareceu: volta para a fila sem custo.
    pub fn configuration_ready(&mut self, now: OffsetDateTime) -> bool {
        let waiting = self.status == JobStatus::NeedsAttention
            && self.attempt_count < self.stage.max_attempts()
            && self
                .last_error_code
                .as_deref()
                .is_some_and(is_configuration_code);
        if waiting {
            self.status = JobStatus::Queued;
            self.updated_at = now;
        }
        waiting
    }

    /// Um job `running` encontrado na abertura. O processo que o rodava morreu.
    ///
    /// **Gasta uma tentativa.** Se foi o proprio estagio que derrubou o app, sem
    /// isto ele derrubaria de novo a cada abertura, para sempre.
    pub fn recover_orphan(&mut self, now: OffsetDateTime) {
        if self.status != JobStatus::Running {
            return;
        }
        self.attempt_count += 1;
        self.last_error_code = Some("interrupted".into());
        self.last_error_message = Some("O M/OS fechou no meio do processamento. Retomando.".into());
        self.status = if self.attempt_count >= self.stage.max_attempts() {
            JobStatus::NeedsAttention
        } else {
            JobStatus::Queued
        };
        self.updated_at = now;
    }

    /// Retry manual: zera a contagem, porque a pessoa pediu.
    pub fn manual_retry(&mut self, now: OffsetDateTime) {
        self.status = JobStatus::Queued;
        self.attempt_count = 0;
        self.next_retry_at = None;
        self.updated_at = now;
    }
}

fn is_configuration_code(code: &str) -> bool {
    matches!(code, "transcriber_missing" | "consent_missing")
}

/// Classifica um erro do whisper pela forma dele.
pub fn classify_transcription_error(error: &crate::TranscriptionError) -> StageFailure {
    use crate::TranscriptionError::*;
    match error {
        NotConfigured => StageFailure::new(
            "transcriber_missing",
            "A transcrição local ainda não foi configurada. Assim que ela estiver pronta, a reunião segue sozinha.",
            FailureClass::Configuration,
        ),
        MissingRuntime { .. } => StageFailure::new(
            "transcriber_missing",
            "O transcritor local não foi encontrado. Assim que ele voltar, a reunião segue sozinha.",
            FailureClass::Configuration,
        ),
        NoAudio => StageFailure::new(
            "no_audio",
            "Não há áudio para transcrever nesta reunião.",
            FailureClass::Permanent,
        ),
        Failed { .. } => StageFailure::new(
            "transcriber_failed",
            "A transcrição falhou. O áudio está seguro e o M/OS vai tentar de novo.",
            FailureClass::Transient,
        ),
        Unreadable { .. } => StageFailure::new(
            "transcriber_unreadable",
            "O transcritor devolveu um resultado ilegível. O áudio está seguro e o M/OS vai tentar de novo.",
            FailureClass::Transient,
        ),
        Cancelled => StageFailure::new(
            "cancelled",
            "A transcrição foi interrompida.",
            FailureClass::Transient,
        ),
    }
}

// ---------------------------------------------------------------------------
// A fase que a pessoa ve
// ---------------------------------------------------------------------------

/// Reuniao → Processando → Pronta, e as excecoes.
///
/// O enum de dez estados continua sendo a verdade TECNICA. Este e o que a tela
/// mostra, e ele existe para a pessoa nunca precisar saber o que e
/// "transcribed" — ela precisa saber se a reuniao esta pronta.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingPhase {
    Recording,
    Finalizing,
    Processing,
    Ready,
    /// A transcricao esta pronta e a organizacao inteligente ficou pendente.
    PartiallyReady,
    /// Caiu gravando, e ainda nao comecou a processar.
    Recovered,
    /// Nao anda sozinha: falta configuracao, ou nao ha audio.
    NeedsAttention,
    /// A transcricao esgotou as tentativas. O audio esta seguro.
    FailedRecoverable,
    Discarded,
}

/// A fase de uma reuniao, dado o job dela.
pub fn meeting_phase(meeting: &Meeting, job: Option<&MeetingJob>) -> MeetingPhase {
    use MeetingStatus::*;

    let job_open = job.filter(|job| job.status.is_open());
    let job_stuck = job_open.is_some_and(|job| job.status == JobStatus::NeedsAttention);

    match meeting.status {
        Recording | Paused => MeetingPhase::Recording,
        Stopping => MeetingPhase::Finalizing,
        Cancelled => MeetingPhase::Discarded,
        Interrupted => {
            if meeting.duration_ms == 0 {
                MeetingPhase::NeedsAttention
            } else if job_open.is_some() && !job_stuck {
                MeetingPhase::Processing
            } else {
                MeetingPhase::Recovered
            }
        }
        Recorded | Transcribing => {
            if job_stuck {
                MeetingPhase::NeedsAttention
            } else {
                MeetingPhase::Processing
            }
        }
        Transcribed => match job_open {
            Some(job) if job.stage == JobStage::Analysis && !job_stuck => MeetingPhase::Processing,
            _ => MeetingPhase::PartiallyReady,
        },
        Analyzing => MeetingPhase::Processing,
        Ready => MeetingPhase::Ready,
        Failed(FailedStage::Audio) => MeetingPhase::NeedsAttention,
        Failed(FailedStage::Transcription) => {
            if job_open.is_some() && !job_stuck {
                MeetingPhase::Processing
            } else {
                MeetingPhase::FailedRecoverable
            }
        }
        Failed(FailedStage::Analysis) => {
            if job_open.is_some() && !job_stuck {
                MeetingPhase::Processing
            } else {
                MeetingPhase::PartiallyReady
            }
        }
    }
}

/// Um passo da lista de progresso.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Done,
    Active,
    Pending,
    Waiting,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressStep {
    /// `saved | audio | transcription | analysis | ready`. A tela nomeia.
    pub key: String,
    pub state: StepState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgress {
    pub steps: Vec<ProgressStep>,
    /// `0..=1`, ponderada. `None` quando nao ha o que medir.
    pub fraction: Option<f32>,
}

/// Quanto do trabalho total a transcricao representa.
///
/// **Medido, e nao arbitrario.** Uma reuniao de uma hora leva ~21 min de
/// whisper em CPU (D-7) e ~1–2 min de Hermes; na GPU a razao cai, mas a
/// transcricao continua sendo a maior parte. 70/30 erra para o lado de a barra
/// andar devagar na analise, e nunca para o lado de chegar a 90% e parar.
pub const TRANSCRIPTION_WEIGHT: f32 = 0.7;

pub fn pipeline_progress(meeting: &Meeting, job: Option<&MeetingJob>) -> PipelineProgress {
    use MeetingStatus::*;
    use StepState::{Active, Done, Pending, Waiting};
    const FAILED: StepState = StepState::Failed;

    let job_progress = job.map(|job| job.progress).unwrap_or(0.0);
    let waiting = job.is_some_and(|job| job.status == JobStatus::WaitingRetry);
    let stuck = job.is_some_and(|job| job.status == JobStatus::NeedsAttention);

    let (saved, audio, transcription, analysis, ready, fraction) = match meeting.status {
        Recording | Paused | Stopping => (Active, Pending, Pending, Pending, Pending, None),
        Interrupted | Recorded => (
            Done,
            Pending,
            if stuck {
                FAILED
            } else if waiting {
                Waiting
            } else {
                Pending
            },
            Pending,
            Pending,
            Some(0.02),
        ),
        Transcribing => {
            let audio_state = if job_progress > 0.0 { Done } else { Active };
            (
                Done,
                audio_state,
                if job_progress > 0.0 { Active } else { Pending },
                Pending,
                Pending,
                Some(0.05 + job_progress * (TRANSCRIPTION_WEIGHT - 0.05)),
            )
        }
        Transcribed => {
            let analysis_state = match job {
                Some(job) if job.stage == JobStage::Analysis && job.status.is_open() => {
                    if stuck {
                        FAILED
                    } else if waiting {
                        Waiting
                    } else {
                        Pending
                    }
                }
                _ => FAILED,
            };
            (
                Done,
                Done,
                Done,
                analysis_state,
                Pending,
                Some(TRANSCRIPTION_WEIGHT),
            )
        }
        Analyzing => (
            Done,
            Done,
            Done,
            Active,
            Pending,
            Some(TRANSCRIPTION_WEIGHT + job_progress * (1.0 - TRANSCRIPTION_WEIGHT)),
        ),
        Ready => (Done, Done, Done, Done, Done, Some(1.0)),
        Failed(FailedStage::Audio) => (FAILED, Pending, Pending, Pending, Pending, None),
        Failed(FailedStage::Transcription) => (
            Done,
            Done,
            if waiting { Waiting } else { FAILED },
            Pending,
            Pending,
            Some(0.05),
        ),
        Failed(FailedStage::Analysis) => (
            Done,
            Done,
            Done,
            if waiting { Waiting } else { FAILED },
            Pending,
            Some(TRANSCRIPTION_WEIGHT),
        ),
        Cancelled => (FAILED, Pending, Pending, Pending, Pending, None),
    };

    let steps = [
        ("saved", saved),
        ("audio", audio),
        ("transcription", transcription),
        ("analysis", analysis),
        ("ready", ready),
    ]
    .into_iter()
    .map(|(key, state)| ProgressStep {
        key: key.to_owned(),
        state,
    })
    .collect();

    PipelineProgress {
        steps,
        fraction: fraction.map(|value: f32| value.clamp(0.0, 1.0)),
    }
}

/// A partir de quando uma reuniao na lixeira vira exclusao definitiva.
pub const TRASH_RETENTION_DAYS: i64 = 30;

/// A reuniao na lixeira ja pode ser apagada de vez?
pub fn trash_expired(meeting: &Meeting, now: OffsetDateTime) -> bool {
    meeting.lifecycle_state == crate::LifecycleState::Trashed
        && meeting
            .trashed_at
            .is_some_and(|at| now - at >= time::Duration::days(TRASH_RETENTION_DAYS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioRetention, ChannelOutcome, LifecycleState, MeetingSource};
    use time::macros::datetime;

    fn now() -> OffsetDateTime {
        datetime!(2026-09-16 14:00 UTC)
    }

    fn meeting(status: MeetingStatus) -> Meeting {
        Meeting {
            id: MeetingId::new(),
            title: "Revisao estrutural".into(),
            status,
            lifecycle_state: LifecycleState::Active,
            source: MeetingSource::Manual,
            started_at: now(),
            ended_at: None,
            duration_ms: 60_000,
            project_id: None,
            audio_dir: "meetings/x".into(),
            retention: AudioRetention::default(),
            audio_deleted_at: None,
            mic: ChannelOutcome::Captured,
            system: ChannelOutcome::Captured,
            failure: None,
            created_at: now(),
            updated_at: now(),
            cancelled_at: None,
            notes: String::new(),
            stop_reason: None,
            trim_start_ms: None,
            trim_end_ms: None,
            trim_origin: None,
            trashed_at: None,
            associated_app: None,
            suggested_end_ms: None,
        }
    }

    fn transient() -> StageFailure {
        StageFailure::new("hermes_unreachable", "fora", FailureClass::Transient)
    }

    #[test]
    fn falha_passageira_espera_na_escada_e_nao_desiste_cedo() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Analysis, now());
        job.start(now());
        assert_eq!(
            job.fail(&transient(), now()),
            AfterFailure::RetryAt(now() + time::Duration::seconds(30))
        );
        assert_eq!(job.status, JobStatus::WaitingRetry);
        assert!(!job.is_due(now()));
        assert!(job.is_due(now() + time::Duration::seconds(30)));

        job.start(now());
        assert_eq!(
            job.fail(&transient(), now()),
            AfterFailure::RetryAt(now() + time::Duration::seconds(120))
        );
    }

    #[test]
    fn a_analise_aguenta_mais_que_a_transcricao() {
        let mut transcricao = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        let mut desistiu = 0;
        for tentativa in 1..=10 {
            transcricao.start(now());
            if transcricao.fail(&transient(), now()) == AfterFailure::GiveUp {
                desistiu = tentativa;
                break;
            }
        }
        assert_eq!(desistiu, 4);

        let mut analise = MeetingJob::queue(MeetingId::new(), JobStage::Analysis, now());
        for tentativa in 1..=10 {
            analise.start(now());
            if analise.fail(&transient(), now()) == AfterFailure::GiveUp {
                desistiu = tentativa;
                break;
            }
        }
        assert_eq!(desistiu, 6);
        assert_eq!(analise.status, JobStatus::NeedsAttention);
    }

    #[test]
    fn falta_de_configuracao_nao_gasta_tentativa_e_volta_sozinha() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.start(now());
        let falha = classify_transcription_error(&crate::TranscriptionError::NotConfigured);
        assert_eq!(job.fail(&falha, now()), AfterFailure::WaitForConfiguration);
        assert_eq!(job.attempt_count, 0);
        assert_eq!(job.status, JobStatus::NeedsAttention);

        assert!(job.configuration_ready(now()));
        assert_eq!(job.status, JobStatus::Queued);
    }

    #[test]
    fn falha_permanente_pede_a_pessoa_na_hora() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.start(now());
        let falha = classify_transcription_error(&crate::TranscriptionError::NoAudio);
        assert_eq!(job.fail(&falha, now()), AfterFailure::GiveUp);
        // Configuracao aparecer nao ressuscita uma falha que nao e de configuracao.
        assert!(!job.configuration_ready(now()));
    }

    #[test]
    fn job_orfao_volta_para_a_fila_gastando_uma_tentativa() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.start(now());
        job.recover_orphan(now());
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempt_count, 1);

        // O estagio que derruba o app nao derruba para sempre.
        for _ in 0..5 {
            job.start(now());
            job.recover_orphan(now());
        }
        assert_eq!(job.status, JobStatus::NeedsAttention);
    }

    #[test]
    fn avancar_de_estagio_zera_as_tentativas() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.start(now());
        job.fail(&transient(), now());
        job.start(now());
        job.succeed(now());
        job.advance(JobStage::Analysis, now());
        assert_eq!(job.stage, JobStage::Analysis);
        assert_eq!(job.attempt_count, 0);
        assert_eq!(job.status, JobStatus::Queued);
    }

    #[test]
    fn progresso_nunca_volta() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.set_progress(0.6, now());
        job.set_progress(0.2, now());
        assert!((job.progress - 0.6).abs() < f32::EPSILON);
        job.set_progress(f32::NAN, now());
        assert!((job.progress - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn a_fase_esconde_o_estado_tecnico() {
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Paused), None),
            MeetingPhase::Recording
        );
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Recorded), None),
            MeetingPhase::Processing
        );
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Ready), None),
            MeetingPhase::Ready
        );

        // Transcrita com a analise na fila: ainda processando.
        let analise = MeetingJob::queue(MeetingId::new(), JobStage::Analysis, now());
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Transcribed), Some(&analise)),
            MeetingPhase::Processing
        );
        // Transcrita sem analise possivel: parcialmente pronta, e nunca inutil.
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Transcribed), None),
            MeetingPhase::PartiallyReady
        );
    }

    #[test]
    fn transcritor_ausente_pede_atencao_sem_parecer_falha() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.fail(
            &classify_transcription_error(&crate::TranscriptionError::NotConfigured),
            now(),
        );
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Recorded), Some(&job)),
            MeetingPhase::NeedsAttention
        );
    }

    #[test]
    fn interrompida_sem_audio_pede_atencao_e_com_audio_e_recuperada() {
        let mut vazia = meeting(MeetingStatus::Interrupted);
        vazia.duration_ms = 0;
        assert_eq!(meeting_phase(&vazia, None), MeetingPhase::NeedsAttention);
        assert_eq!(
            meeting_phase(&meeting(MeetingStatus::Interrupted), None),
            MeetingPhase::Recovered
        );
    }

    #[test]
    fn o_progresso_pondera_os_estagios_e_so_anda_para_a_frente() {
        let mut job = MeetingJob::queue(MeetingId::new(), JobStage::Transcription, now());
        job.set_progress(0.5, now());
        let meio = pipeline_progress(&meeting(MeetingStatus::Transcribing), Some(&job));
        let fracao = meio.fraction.unwrap();
        assert!(fracao > 0.3 && fracao < 0.4, "{fracao}");

        let analise = pipeline_progress(&meeting(MeetingStatus::Analyzing), None);
        assert!((analise.fraction.unwrap() - TRANSCRIPTION_WEIGHT).abs() < 1e-6);

        let pronta = pipeline_progress(&meeting(MeetingStatus::Ready), None);
        assert_eq!(pronta.fraction, Some(1.0));
        assert!(pronta
            .steps
            .iter()
            .all(|step| step.state == StepState::Done));
    }

    #[test]
    fn a_lixeira_expira_em_trinta_dias() {
        let mut lixo = meeting(MeetingStatus::Ready);
        lixo.lifecycle_state = LifecycleState::Trashed;
        lixo.trashed_at = Some(now());
        assert!(!trash_expired(&lixo, now() + time::Duration::days(29)));
        assert!(trash_expired(&lixo, now() + time::Duration::days(30)));
    }
}

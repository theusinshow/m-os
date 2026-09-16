//! Comandos e lacos do Meeting Agent.
//!
//! **E o unico lugar onde `mos-audio` e `mos-core` se encontram.** O crate de
//! audio nao conhece Meeting e nao alcanca o banco; o dominio nao sabe que
//! WASAPI existe. A traducao entre os dois mora aqui, pelo mesmo desenho que
//! `jarvis.rs` usa para a ponte do Hermes (ADR-024).
//!
//! Casca fina de proposito: `SETUP-MAQUINA.md` §4 registra que
//! `cargo test -p mos-desktop` nao roda na maquina principal, e teste que nao
//! roda nao protege nada. Toda REGRA vive em `mos-core` ou
//! `mos-storage-sqlite` — a politica do pipeline em `meeting_pipeline`, a
//! heuristica do Guardian em `meeting_guardian` —, e o que sobra aqui e
//! coletar sinais, obedecer vereditos e mover bytes.
//!
//! # A V2, em uma frase
//!
//! **Parar e o unico gesto.** Transcrever, analisar, tentar de novo e perceber
//! que a reuniao acabou deixaram de ser botoes e viraram dois lacos: `run`, que
//! acompanha a gravacao e o Recording Guardian, e `run_pipeline`, que processa.
//!
//! Spec: `docs/superpowers/specs/2026-09-16-meeting-agent-v2-design.md`.

use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};

use mos_audio::{AudioError, ChannelState, Recording};
use mos_core::meeting_guardian::{
    self, GuardianConfig, GuardianEvent, GuardianState, GuardianView, MicUser, Observation,
};
use mos_core::{
    AudioOutcome, AudioRetention, ChannelOutcome, CoreError, ErrorCode, FailedStage, FailureClass,
    GuardianSettle, JobStage, Meeting, MeetingAnalysis, MeetingInsight, MeetingOverview,
    MeetingSource, MeetingStatus, StageFailure, StopReason, TranscriptSegment,
    TranscriptionProvider, TrimOrigin,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

// ============================================================================
// Preferencias
// ============================================================================

/// O que a pessoa escolhe sobre reunioes. Mora no `settings.json`: e deste
/// aparelho — o Guardian observa o microfone DESTE computador.
///
/// **Sem limiar tecnico.** Tolerancia, cooldown e pesos sao constantes do
/// dominio; uma tela com "segundos de tolerancia" transformaria a pessoa em quem
/// calibra a heuristica.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPreferences {
    /// Transcrever e organizar sozinho ao parar.
    #[serde(default = "sim")]
    pub auto_process: bool,
    #[serde(default)]
    pub guardian: GuardianConfig,
    /// A retencao com que cada reuniao nova nasce.
    #[serde(default)]
    pub retention: AudioRetention,
    /// Termos proprios: nomes, siglas, codigos.
    #[serde(default)]
    pub vocabulary: Vec<String>,
}

fn sim() -> bool {
    true
}

impl Default for MeetingPreferences {
    fn default() -> Self {
        Self {
            auto_process: true,
            guardian: GuardianConfig::default(),
            retention: AudioRetention::default(),
            vocabulary: Vec::new(),
        }
    }
}

fn preferences(app: &AppHandle) -> MeetingPreferences {
    let state = app.state::<AppState>();
    crate::load_settings(&state.settings_path).meetings
}

#[tauri::command]
pub fn meeting_preferences(app: AppHandle) -> MeetingPreferences {
    preferences(&app)
}

#[tauri::command]
pub fn meeting_set_preferences(
    app: AppHandle,
    preferences: MeetingPreferences,
) -> Result<MeetingPreferences, CoreError> {
    let state = app.state::<AppState>();
    let mut settings = crate::load_settings(&state.settings_path);
    let mut cleaned = preferences;
    cleaned.vocabulary = cleaned
        .vocabulary
        .into_iter()
        .map(|term| term.trim().to_owned())
        .filter(|term| term.chars().count() >= 2)
        .collect();
    cleaned.vocabulary.sort();
    cleaned.vocabulary.dedup();
    settings.meetings = cleaned.clone();
    crate::save_settings(&state.settings_path, &settings)?;
    Ok(cleaned)
}

// ============================================================================
// A gravacao viva
// ============================================================================

/// A gravacao viva do processo.
///
/// `Option` e nao uma fila: **uma gravacao por vez**. O dominio ja recusa a
/// segunda, e este `Mutex` e a mesma regra do lado do adapter — dois gravadores
/// disputariam o mesmo dispositivo.
#[derive(Default)]
pub struct RecordingState {
    active: Mutex<Option<Active>>,
}

impl RecordingState {
    /// Se ha gravacao em curso agora.
    ///
    /// Trava envenenada conta como "nao esta gravando": e o lado seguro, porque
    /// o erro vira uma oferta a mais e nunca uma gravacao perdida.
    pub fn gravando(&self) -> bool {
        self.active
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }
}

struct Active {
    meeting_id: String,
    recording: Recording,
    guardian: GuardianState,
    view: GuardianView,
}

/// O que o renderer recebe, uma vez por segundo.
///
/// **Nao existe PCM aqui.** `micLevel` e `systemLevel` sao RMS ja reduzidos a
/// `0..1000` dentro da thread de captura (`MEETING-AGENT.md` §4.3).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTick {
    pub meeting_id: String,
    pub duration_ms: i64,
    pub mic: ChannelState,
    pub system: ChannelState,
    pub mic_level: u64,
    pub system_level: u64,
    /// Lido do atomico da sessao, e nao do banco: a barra precisa parar de
    /// pulsar no MESMO instante em que o audio para.
    pub paused: bool,
    /// O que o Recording Guardian pede que a tela mostre.
    pub guardian: GuardianView,
}

fn audio_error(error: AudioError) -> CoreError {
    let code = match error {
        AudioError::Unsupported => ErrorCode::InvalidTransition,
        AudioError::AlreadyRecording => ErrorCode::InvalidTransition,
        AudioError::Device(_) => ErrorCode::InvalidInput,
        AudioError::Storage { .. } | AudioError::Misaligned { .. } => ErrorCode::Io,
    };
    CoreError::new(code, error.to_string(), matches!(code, ErrorCode::Io))
}

fn to_domain(state: ChannelState) -> ChannelOutcome {
    match state {
        ChannelState::Capturing => ChannelOutcome::Capturing,
        ChannelState::Captured => ChannelOutcome::Captured,
        ChannelState::Unavailable { reason } => ChannelOutcome::Unavailable { reason },
        ChannelState::Lost { at_ms, reason } => ChannelOutcome::Lost { at_ms, reason },
    }
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, CoreError> {
    app.path().app_data_dir().map_err(|error| {
        CoreError::new(
            ErrorCode::Io,
            format!("Nao foi possivel localizar o diretorio de dados: {error}"),
            false,
        )
    })
}

/// O caminho absoluto do audio de uma reuniao.
///
/// Derivado de `audio_dir`, que por sua vez e derivado do `MeetingId` — **nenhum
/// path vem do renderer** (§18).
fn audio_root(app: &AppHandle, meeting: &Meeting) -> Result<PathBuf, CoreError> {
    let base = data_dir(app)?;
    let candidate = base.join(&meeting.audio_dir);
    if !candidate.starts_with(&base) {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Caminho de audio fora do diretorio de dados.",
            false,
        ));
    }
    Ok(candidate)
}

fn epoch_now() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

/// O proprio M/OS, que nunca e "o app da reuniao".
const EU_MESMO: &str = "mos-desktop.exe";

/// Quem tem o microfone agora, sem o M/OS.
fn mic_users() -> Vec<MicUser> {
    crate::microfone::abertos_agora()
        .into_iter()
        .filter(|entry| !entry.processo.eq_ignore_ascii_case(EU_MESMO))
        .map(|entry| MicUser {
            process: entry.processo,
            seconds_open: entry.segundos_aberto,
        })
        .collect()
}

fn log(nivel: crate::diagnostico::Nivel, mensagem: &str) {
    // Nunca conteudo: so id, estagio, codigo, duracao e motivo (§16.3).
    crate::diagnostico::escrever(nivel, "reuniao", mensagem);
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn meeting_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    recorder: tauri::State<'_, RecordingState>,
    title: &str,
    project_id: Option<&str>,
    source: Option<&str>,
    associated_app: Option<String>,
) -> Result<Meeting, CoreError> {
    let mut active = recorder.active.lock().map_err(lock_error)?;
    if active.is_some() {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "Ja existe uma gravacao em curso.",
            false,
        ));
    }

    // O titulo automatico nasce AQUI, e nao no dominio, porque so aqui existe
    // fuso. A analise troca por um titulo util depois — enquanto ninguem
    // renomear.
    let escolhido = title.trim().to_owned();
    let titulo = if escolhido.is_empty() {
        let agora = crate::surface::now_local(&app);
        format!(
            "Reuniao de {:02}/{:02} {:02}:{:02}",
            agora.day(),
            agora.month() as u8,
            agora.hour(),
            agora.minute()
        )
    } else {
        escolhido
    };

    // O app da reuniao: o que a oferta apontou, ou o que ja tem o microfone ha
    // mais tempo. Nome de executavel — o dado da ADR-047, e nada alem.
    let associado = associated_app
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            mic_users()
                .into_iter()
                .max_by_key(|user| user.seconds_open)
                .map(|user| user.process)
        });
    let origem = match source {
        Some("detected") => MeetingSource::Detected,
        _ => MeetingSource::Manual,
    };
    let prefs = preferences(&app);

    let meeting = state.meetings.start_with(
        &titulo,
        project_id,
        origem,
        associado.clone(),
        prefs.retention,
    )?;
    let root = audio_root(&app, &meeting)?;
    let started_at = meeting
        .started_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    match Recording::start(&root, &started_at) {
        Ok(recording) => {
            *active = Some(Active {
                meeting_id: meeting.id.to_string(),
                recording,
                guardian: GuardianState::start(epoch_now(), associado),
                view: GuardianView::Idle,
            });
            drop(active);
            update_tray(&app, Some(0), false, &GuardianView::Idle);
            let _ = app.emit("meeting-started", &meeting);
            let _ = app.emit("data-changed", "meeting");
            log(
                crate::diagnostico::Nivel::Info,
                &format!(
                    "gravacao iniciada id={} origem={}",
                    meeting.id,
                    origem.as_str()
                ),
            );
            Ok(meeting)
        }
        Err(error) => {
            let detail = error.to_string();
            let _ = state
                .meetings
                .fail(&meeting.id.to_string(), FailedStage::Audio, &detail);
            Err(audio_error(error))
        }
    }
}

/// Encerra a gravacao em curso — o caminho UNICO de parar.
///
/// Clique na barra, tray, atalho, Guardian, os dois canais caindo e o app
/// saindo passam todos por aqui, e so o `reason` muda. Antes da V2 eram quatro
/// copias, e uma delas emitia menos eventos que as outras.
pub fn finish_recording(app: &AppHandle, reason: StopReason) -> Result<Meeting, CoreError> {
    let recorder = app.state::<RecordingState>();
    let active = recorder
        .active
        .lock()
        .map_err(lock_error)?
        .take()
        .ok_or_else(sem_gravacao)?;
    let state = app.state::<AppState>();

    // `Stopping` antes de fechar os arquivos: entre o clique e o fechamento
    // ainda entra audio, e a reuniao dura ate o ultimo frame gravado.
    state.meetings.stop(&active.meeting_id)?;
    let outcome = active.recording.stop().map_err(audio_error)?;
    let duration_ms = outcome.duration_ms;

    let settle = GuardianSettle {
        associated_app: active.guardian.associated_app.clone(),
        suggested_end_ms: active.guardian.suggested_end_ms(),
        confident_trim_end_ms: active.guardian.confident_trim_end(duration_ms),
    };
    let settled = state.meetings.settle_recording(
        &active.meeting_id,
        AudioOutcome {
            duration_ms,
            mic: to_domain(outcome.mic),
            system: to_domain(outcome.system),
        },
        reason,
        settle.clone(),
    );

    update_tray(app, None, false, &GuardianView::Idle);
    hide_overlay(app);

    if let Ok(meeting) = &settled {
        if let Some(end) = meeting
            .trim_end_ms
            .filter(|_| meeting.trim_origin == Some(TrimOrigin::Auto))
        {
            let _ = state.meetings.record_guardian_event(
                &active.meeting_id,
                "trim_applied",
                None,
                "auto",
                Some(duration_ms - end),
            );
        }
        if !preferences(app).auto_process {
            let _ = state.meetings.hold_for_manual_start(&active.meeting_id);
        }
        log(
            crate::diagnostico::Nivel::Info,
            &format!(
                "gravacao encerrada id={} stop_reason={} duracao_ms={} excesso_ms={}",
                meeting.id,
                reason.as_str(),
                duration_ms,
                settle
                    .suggested_end_ms
                    .map(|end| (duration_ms - end).max(0))
                    .unwrap_or(0)
            ),
        );
        let _ = app.emit("meeting-stopped", meeting);
    }
    let _ = app.emit("data-changed", "meeting");
    wake_pipeline(app);
    settled
}

#[tauri::command]
pub fn meeting_stop(app: AppHandle) -> Result<Meeting, CoreError> {
    finish_recording(&app, StopReason::Manual)
}

/// Encerra e ignora o que veio depois do fim provavel.
///
/// O gesto da pergunta "Parece que sua reuniao terminou — encerrar e ignorar o
/// que veio depois de 15:42?". O corte e nao destrutivo: os chunks ficam.
#[tauri::command]
pub fn meeting_stop_and_trim(app: AppHandle, end_ms: i64) -> Result<Meeting, CoreError> {
    let meeting = finish_recording(&app, StopReason::Manual)?;
    let state = app.state::<AppState>();
    let id = meeting.id.to_string();
    if end_ms > 0 && end_ms < meeting.duration_ms && meeting.trim_end_ms.is_none() {
        let outcome = state
            .meetings
            .set_trim(&id, 0, end_ms, TrimOrigin::Suggested)?;
        let _ = state.meetings.record_guardian_event(
            &id,
            "trim_applied",
            None,
            "stop_prompt",
            Some(meeting.duration_ms - end_ms),
        );
        return Ok(outcome.meeting);
    }
    Ok(meeting)
}

/// Suspende a gravacao em curso.
///
/// PAUSAR: o audio para PRIMEIRO, o estado muda depois. RETOMAR: o estado muda
/// primeiro, o audio volta depois. Frame gravado com a tela dizendo o contrario
/// e a mentira que a §17.2 proibe, nos dois sentidos.
#[tauri::command]
pub fn meeting_pause(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    recorder: tauri::State<'_, RecordingState>,
) -> Result<Meeting, CoreError> {
    let active = recorder.active.lock().map_err(lock_error)?;
    let atual = active.as_ref().ok_or_else(sem_gravacao)?;
    atual.recording.set_paused(true);
    let id = atual.meeting_id.clone();
    let mut frame = tick(atual);
    frame.paused = true;
    drop(active);

    let pausada = state.meetings.pause(&id)?;
    let _ = app.emit("meeting-tick", &frame);
    update_tray(&app, Some(frame.duration_ms), true, &frame.guardian);
    Ok(pausada)
}

#[tauri::command]
pub fn meeting_resume(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    recorder: tauri::State<'_, RecordingState>,
) -> Result<Meeting, CoreError> {
    let id = {
        let active = recorder.active.lock().map_err(lock_error)?;
        active.as_ref().ok_or_else(sem_gravacao)?.meeting_id.clone()
    };
    let retomada = state.meetings.resume(&id)?;

    let active = recorder.active.lock().map_err(lock_error)?;
    let atual = active.as_ref().ok_or_else(sem_gravacao)?;
    atual.recording.set_paused(false);
    let frame = tick(atual);
    drop(active);

    let _ = app.emit("meeting-tick", &frame);
    update_tray(&app, Some(frame.duration_ms), false, &frame.guardian);
    Ok(retomada)
}

/// ⭐ Marcar momento. Sinal de relevancia para o Hermes, e atalho na
/// transcricao — nunca fato.
#[tauri::command]
pub fn meeting_mark_moment(app: AppHandle) -> Result<mos_core::MeetingBookmark, CoreError> {
    let (id, at_ms) = {
        let recorder = app.state::<RecordingState>();
        let active = recorder.active.lock().map_err(lock_error)?;
        let atual = active.as_ref().ok_or_else(sem_gravacao)?;
        (
            atual.meeting_id.clone(),
            atual.recording.state().duration_ms,
        )
    };
    let bookmark = app.state::<AppState>().meetings.add_bookmark(&id, at_ms)?;
    let _ = app.emit("meeting-bookmarked", &bookmark);
    Ok(bookmark)
}

/// "Continuar gravando": desfaz a suspeita e poe o Guardian em cooldown.
#[tauri::command]
pub fn meeting_guardian_continue(app: AppHandle) -> Result<(), CoreError> {
    let (id, events) = {
        let recorder = app.state::<RecordingState>();
        let mut active = recorder.active.lock().map_err(lock_error)?;
        let atual = active.as_mut().ok_or_else(sem_gravacao)?;
        let events = atual.guardian.keep_recording(epoch_now());
        atual.view = GuardianView::Idle;
        (atual.meeting_id.clone(), events)
    };
    let state = app.state::<AppState>();
    let _ = state
        .meetings
        .record_guardian_event(&id, "continued", None, "", None);
    for event in events {
        if matches!(event, GuardianEvent::CountdownCancelled) {
            let _ = state.meetings.record_guardian_event(
                &id,
                "countdown_cancelled",
                None,
                "user",
                None,
            );
        }
    }
    hide_overlay(&app);
    let _ = app.emit("meeting-guardian", GuardianView::Idle);
    Ok(())
}

fn sem_gravacao() -> CoreError {
    CoreError::new(
        ErrorCode::InvalidTransition,
        "Nao ha gravacao em curso.",
        false,
    )
}

/// O estado da gravacao para a interface. Barato: le atomicos.
#[tauri::command]
pub fn meeting_recording(
    recorder: tauri::State<'_, RecordingState>,
) -> Result<Option<MeetingTick>, CoreError> {
    let active = recorder.active.lock().map_err(lock_error)?;
    Ok(active.as_ref().map(tick))
}

fn tick(active: &Active) -> MeetingTick {
    let state = active.recording.state();
    MeetingTick {
        meeting_id: active.meeting_id.clone(),
        duration_ms: state.duration_ms,
        mic: state.mic,
        system: state.system,
        mic_level: state.mic_level,
        system_level: state.system_level,
        paused: active.recording.is_paused(),
        guardian: active.view.clone(),
    }
}

// ============================================================================
// Leitura
// ============================================================================

#[tauri::command]
pub fn meeting_list(
    state: tauri::State<'_, AppState>,
    include_archived: bool,
) -> Result<Vec<Meeting>, CoreError> {
    state.meetings.meetings(include_archived)
}

/// A lista com fase, progresso e contagens — o que a pagina e a Home leem.
#[tauri::command]
pub fn meeting_overview(
    state: tauri::State<'_, AppState>,
    include_archived: bool,
) -> Result<Vec<MeetingOverview>, CoreError> {
    state.meetings.overview(include_archived)
}

#[tauri::command]
pub fn meeting_overview_one(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<MeetingOverview, CoreError> {
    state.meetings.overview_one(id)
}

#[tauri::command]
pub fn meeting_get(state: tauri::State<'_, AppState>, id: &str) -> Result<Meeting, CoreError> {
    state.meetings.meeting(id)
}

#[tauri::command]
pub fn meeting_transcript(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Vec<TranscriptSegment>, CoreError> {
    state.meetings.transcript(id)
}

#[tauri::command]
pub fn meeting_analysis(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Option<MeetingAnalysis>, CoreError> {
    state.meetings.analysis(id)
}

#[tauri::command]
pub fn meeting_insights(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Vec<MeetingInsight>, CoreError> {
    state.meetings.insights(id)
}

#[tauri::command]
pub fn meeting_bookmarks(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Vec<mos_core::MeetingBookmark>, CoreError> {
    state.meetings.bookmarks(id)
}

#[tauri::command]
pub fn meeting_add_bookmark(
    state: tauri::State<'_, AppState>,
    id: &str,
    at_ms: i64,
) -> Result<mos_core::MeetingBookmark, CoreError> {
    state.meetings.add_bookmark(id, at_ms)
}

#[tauri::command]
pub fn meeting_delete_bookmark(
    state: tauri::State<'_, AppState>,
    bookmark_id: &str,
) -> Result<(), CoreError> {
    state.meetings.delete_bookmark(bookmark_id)
}

#[tauri::command]
pub fn meeting_set_project(
    state: tauri::State<'_, AppState>,
    id: &str,
    project_id: Option<&str>,
) -> Result<Meeting, CoreError> {
    state.meetings.set_project(id, project_id)
}

#[tauri::command]
pub fn meeting_set_title(
    state: tauri::State<'_, AppState>,
    id: &str,
    title: &str,
) -> Result<Meeting, CoreError> {
    state.meetings.set_title(id, title)
}

/// Grava as anotacoes, com debounce. Os marcadores (`!task`) viram itens quando
/// a gravacao ja parou — nao precisam do Hermes.
#[tauri::command]
pub fn meeting_set_notes(
    state: tauri::State<'_, AppState>,
    id: &str,
    notes: &str,
) -> Result<Meeting, CoreError> {
    let meeting = state.meetings.set_notes(id, notes)?;
    if !meeting.status.is_capturing() {
        let _ = state.meetings.apply_written_insights(id);
    }
    Ok(meeting)
}

#[tauri::command]
pub fn meeting_set_archived(
    state: tauri::State<'_, AppState>,
    id: &str,
    archived: bool,
) -> Result<Meeting, CoreError> {
    state.meetings.set_lifecycle(
        id,
        if archived {
            mos_core::LifecycleState::Archived
        } else {
            mos_core::LifecycleState::Active
        },
    )
}

/// Compatibilidade: "Processar" numa reuniao interrompida. A V2 ja processa
/// sozinha, e o comando so poe na fila.
#[tauri::command]
pub fn meeting_process_recovered(app: AppHandle, id: &str) -> Result<Meeting, CoreError> {
    let state = app.state::<AppState>();
    let meeting = state.meetings.meeting(id)?;
    if meeting.duration_ms > 0 {
        state.meetings.retry_job(id)?;
        wake_pipeline(&app);
    }
    state.meetings.meeting(id)
}

/// "Descartar" uma gravacao interrompida: apaga o audio, e a linha fica.
#[tauri::command]
pub fn meeting_discard(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Meeting, CoreError> {
    let cancelled = state.meetings.cancel(id)?;
    let root = audio_root(&app, &cancelled)?;
    mos_audio::delete_session_audio(&root).map_err(audio_error)?;
    state.meetings.mark_audio_deleted(id)
}

#[tauri::command]
pub fn meeting_open_commitments(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MeetingInsight>, CoreError> {
    state.meetings.open_commitments()
}

/// Reunioes interrompidas que ainda esperam alguem — na V2, so as sem audio.
#[tauri::command]
pub fn meeting_interrupted(state: tauri::State<'_, AppState>) -> Result<Vec<Meeting>, CoreError> {
    Ok(state
        .meetings
        .meetings(false)?
        .into_iter()
        .filter(|meeting| meeting.status == MeetingStatus::Interrupted)
        .collect())
}

// ============================================================================
// Lixeira e exclusao
// ============================================================================

/// Apagar reuniao: vai para a lixeira, com desfazer.
///
/// `stop_first` e o "Encerrar e apagar" da pergunta que a tela faz quando a
/// reuniao ainda grava — sem ele, a recusa do dominio chega como erro.
#[tauri::command]
pub fn meeting_trash(app: AppHandle, id: &str, stop_first: bool) -> Result<Meeting, CoreError> {
    let state = app.state::<AppState>();
    if stop_first {
        let gravando_esta = app
            .state::<RecordingState>()
            .active
            .lock()
            .map(|guard| guard.as_ref().is_some_and(|active| active.meeting_id == id))
            .unwrap_or(false);
        if gravando_esta {
            finish_recording(&app, StopReason::Manual)?;
        }
    }
    let trashed = state.meetings.trash(id)?;
    let _ = app.emit("meeting-trashed", id);
    let _ = app.emit("data-changed", "meeting");
    Ok(trashed)
}

#[tauri::command]
pub fn meeting_restore(app: AppHandle, id: &str) -> Result<Meeting, CoreError> {
    let restored = app.state::<AppState>().meetings.restore(id)?;
    let _ = app.emit("data-changed", "meeting");
    wake_pipeline(&app);
    Ok(restored)
}

#[tauri::command]
pub fn meeting_trashed(state: tauri::State<'_, AppState>) -> Result<Vec<Meeting>, CoreError> {
    state.meetings.trashed()
}

/// O arquivo que marca uma pasta como "mandada apagar".
///
/// Escrito ANTES de apagar a linha do banco. Se a remocao do disco falhar
/// depois, a abertura seguinte encontra a pasta sem linha e COM a marca, e
/// termina o servico. Pasta sem linha e sem marca continua sendo so relatada
/// (§9.2): nada apaga o que ninguem mandou apagar.
const TOMBSTONE: &str = ".apagar";

/// Exclusao definitiva: banco, indices, jobs, momentos, eventos e arquivos.
/// **As Tasks e os lembretes criados ficam** — ja fazem parte do trabalho.
#[tauri::command]
pub fn meeting_delete(app: AppHandle, id: &str) -> Result<(), CoreError> {
    delete_permanently(&app, id)?;
    let _ = app.emit("meeting-deleted", id);
    let _ = app.emit("data-changed", "meeting");
    Ok(())
}

fn delete_permanently(app: &AppHandle, id: &str) -> Result<(), CoreError> {
    let state = app.state::<AppState>();
    let meeting = state.meetings.meeting(id)?;
    let raiz = audio_root(app, &meeting);
    if let Ok(caminho) = &raiz {
        if caminho.exists() {
            let _ = std::fs::write(caminho.join(TOMBSTONE), b"");
        }
    }
    state.meetings.delete(id)?;
    if let Ok(caminho) = raiz {
        if caminho.exists() {
            if let Err(causa) = std::fs::remove_dir_all(&caminho) {
                log(
                    crate::diagnostico::Nivel::Aviso,
                    &format!(
                        "reuniao {id} apagada do banco; a pasta fica marcada para a proxima abertura: {causa}"
                    ),
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(temp_dir_for(id));
    log(
        crate::diagnostico::Nivel::Info,
        &format!("reuniao apagada definitivamente id={id}"),
    );
    Ok(())
}

#[tauri::command]
pub fn meeting_empty_trash(app: AppHandle) -> Result<usize, CoreError> {
    let trashed = app.state::<AppState>().meetings.trashed()?;
    let mut apagadas = 0;
    for meeting in trashed {
        if delete_permanently(&app, &meeting.id.to_string()).is_ok() {
            apagadas += 1;
        }
    }
    let _ = app.emit("data-changed", "meeting");
    Ok(apagadas)
}

// ============================================================================
// Corte
// ============================================================================

#[tauri::command]
pub fn meeting_set_trim(
    app: AppHandle,
    id: &str,
    start_ms: i64,
    end_ms: i64,
    origin: Option<&str>,
) -> Result<mos_core::TrimOutcome, CoreError> {
    let state = app.state::<AppState>();
    let origin = match origin {
        Some("suggested") => TrimOrigin::Suggested,
        _ => TrimOrigin::Manual,
    };
    let before = state.meetings.meeting(id)?;
    let outcome = state.meetings.set_trim(id, start_ms, end_ms, origin)?;
    let reverted = outcome.meeting.trim_end_ms.is_none() && outcome.meeting.trim_start_ms.is_none();
    let _ = state.meetings.record_guardian_event(
        id,
        if reverted {
            "trim_reverted"
        } else {
            "trim_applied"
        },
        None,
        origin.as_str(),
        Some(outcome.meeting.trimmed_ms() - before.trimmed_ms()),
    );
    if outcome.requeued.is_some() {
        wake_pipeline(&app);
    }
    let _ = app.emit("data-changed", "meeting");
    Ok(outcome)
}

#[tauri::command]
pub fn meeting_clear_trim(app: AppHandle, id: &str) -> Result<mos_core::TrimOutcome, CoreError> {
    let state = app.state::<AppState>();
    let outcome = state.meetings.clear_trim(id)?;
    let _ = state
        .meetings
        .record_guardian_event(id, "trim_reverted", None, "manual", None);
    if outcome.requeued.is_some() {
        wake_pipeline(&app);
    }
    let _ = app.emit("data-changed", "meeting");
    Ok(outcome)
}

/// Onde a conversa provavelmente acabou, quando vale sugerir o corte.
#[tauri::command]
pub fn meeting_trim_suggestion(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Option<i64>, CoreError> {
    state.meetings.transcript_trim_suggestion(id)
}

// ============================================================================
// Itens
// ============================================================================

#[tauri::command]
pub fn meeting_previews(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Vec<mos_core::InsightPreview>, CoreError> {
    state.meetings.previews(id)
}

/// O recibo de uma aceitacao. Carrega o `UndoStep` porque e assim que o resto
/// do M/OS oferece volta: na janela do recibo (ADR-035).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptReceipt {
    pub insight: MeetingInsight,
    pub task_id: String,
    pub reminder_id: Option<String>,
    pub undo: mos_core::UndoStep,
}

/// Um item confirmado na revisao, como a tela o manda.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptRequest {
    pub insight_id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub remind_at: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
}

fn parse_instant(
    value: Option<&str>,
    what: &str,
) -> Result<Option<time::OffsetDateTime>, CoreError> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
                .map_err(|error| {
                    CoreError::new(
                        ErrorCode::InvalidInput,
                        format!("{what} invalido: {error}"),
                        false,
                    )
                })
        })
        .transpose()
}

fn to_accept(request: &AcceptRequest) -> Result<mos_core::AcceptInsight, CoreError> {
    Ok(mos_core::AcceptInsight {
        insight_id: mos_core::InsightId::parse(&request.insight_id)?,
        title: request.title.clone(),
        description: request.description.clone().unwrap_or_default(),
        project_id: request
            .project_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(mos_core::ProjectId::parse)
            .transpose()?,
        remind_at: parse_instant(request.remind_at.as_deref(), "Instante do lembrete")?,
        due_at: parse_instant(request.due_at.as_deref(), "Prazo")?,
    })
}

fn receipt(accepted: mos_core::AcceptedInsight) -> AcceptReceipt {
    AcceptReceipt {
        task_id: accepted.task_id.to_string(),
        reminder_id: accepted.reminder_id.map(|id| id.to_string()),
        undo: mos_core::UndoStep::UndoMeetingInsight {
            insight_id: accepted.insight.id.to_string(),
            task_id: accepted.task_id.to_string(),
            reminder_id: accepted.reminder_id.map(|id| id.to_string()),
        },
        insight: accepted.insight,
    }
}

/// Aceita um item — o caminho da V1, que agora e um lote de um.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn meeting_accept_insight(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    insight_id: &str,
    title: &str,
    description: Option<&str>,
    project_id: Option<&str>,
    remind_at: Option<&str>,
    due_at: Option<&str>,
) -> Result<AcceptReceipt, CoreError> {
    let accepted = state.meetings.accept_insight(to_accept(&AcceptRequest {
        insight_id: insight_id.to_owned(),
        title: title.to_owned(),
        description: description.map(str::to_owned),
        project_id: project_id.map(str::to_owned),
        remind_at: remind_at.map(str::to_owned),
        due_at: due_at.map(str::to_owned),
    })?)?;
    let _ = app.emit("data-changed", "meeting-insight");
    Ok(receipt(accepted))
}

/// A revisao em lote. Tudo numa transacao; o recibo desfaz todas juntas.
#[tauri::command]
pub fn meeting_accept_batch(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    items: Vec<AcceptRequest>,
) -> Result<Vec<AcceptReceipt>, CoreError> {
    let accepts = items.iter().map(to_accept).collect::<Result<Vec<_>, _>>()?;
    let accepted = state.meetings.accept_batch(accepts)?;
    let _ = app.emit("data-changed", "meeting-insight");
    Ok(accepted.into_iter().map(receipt).collect())
}

#[tauri::command]
pub fn meeting_dismiss_insight(
    state: tauri::State<'_, AppState>,
    insight_id: &str,
) -> Result<MeetingInsight, CoreError> {
    state.meetings.dismiss_insight(insight_id)
}

/// Um item criado a partir de um trecho da transcricao: "criar Task",
/// "marcar decisao".
#[tauri::command]
pub fn meeting_add_insight(
    app: AppHandle,
    id: &str,
    segment_id: &str,
    kind: &str,
    text: Option<&str>,
) -> Result<MeetingInsight, CoreError> {
    let state = app.state::<AppState>();
    let kind = mos_core::InsightKind::parse(kind)?;
    let insight = state
        .meetings
        .add_manual_insight(id, segment_id, kind, text)?;
    let offset = crate::surface::now_local(&app).offset();
    let _ = state.meetings.resolve_dues(id, offset);
    let _ = app.emit("data-changed", "meeting-insight");
    Ok(insight)
}

/// O texto do follow-up, pronto para copiar. Nunca e enviado.
#[tauri::command]
pub fn meeting_follow_up(app: AppHandle, id: &str) -> Result<String, CoreError> {
    let state = app.state::<AppState>();
    let meeting = state.meetings.meeting(id)?;
    let offset = crate::surface::now_local(&app).offset();
    let local = meeting.started_at.to_offset(offset);
    let summary = state
        .meetings
        .analysis(id)?
        .map(|analysis| analysis.summary)
        .unwrap_or_default();
    let insights = state.meetings.insights(id)?;
    let date = format!(
        "{:02}/{:02}/{}",
        local.day(),
        local.month() as u8,
        local.year()
    );
    Ok(mos_core::meeting_text::build_follow_up(
        &meeting.title,
        &date,
        &summary,
        &insights,
        &|insight| {
            insight.due_at.map(|due| {
                let due = due.to_offset(offset);
                format!("até {:02}/{:02}", due.day(), due.month() as u8)
            })
        },
    ))
}

/// O Project que a reuniao parece ser, quando ainda nao ha um.
#[tauri::command]
pub fn meeting_project_suggestion(
    app: AppHandle,
    id: &str,
) -> Result<Option<mos_core::meeting_text::ProjectInference>, CoreError> {
    let state = app.state::<AppState>();
    let meeting = state.meetings.meeting(id)?;
    if meeting.project_id.is_some() {
        return Ok(None);
    }
    let projects = project_candidates(&app);
    let segments = state.meetings.transcript(id)?;
    Ok(mos_core::meeting_text::infer_project(
        &mos_core::meeting_text::ProjectInferenceInput {
            projects: &projects,
            notes: &meeting.notes,
            segments: &segments,
            active_timer_project: None,
            hermes_hint: None,
        },
    ))
}

fn project_candidates(app: &AppHandle) -> Vec<mos_core::meeting_text::ProjectCandidate> {
    app.state::<AppState>()
        .work
        .projects(false)
        .unwrap_or_default()
        .into_iter()
        .map(|project| mos_core::meeting_text::ProjectCandidate {
            id: project.id,
            name: project.name,
        })
        .collect()
}

// ============================================================================
// Ouvir um trecho
// ============================================================================

/// Um trecho de ate 60 s como WAV em base64, para o `<audio>` da tela.
///
/// **O renderer nunca recebe caminho** (§18): recebe os bytes. `mode` escolhe
/// `both`, `mic` ("Voce") ou `system` ("Remoto").
#[tauri::command]
pub async fn meeting_clip(
    app: AppHandle,
    id: String,
    start_ms: i64,
    duration_ms: Option<i64>,
    mode: Option<String>,
) -> Result<String, CoreError> {
    let meeting = app.state::<AppState>().meetings.meeting(&id)?;
    if meeting.audio_deleted_at.is_some() {
        return Err(CoreError::new(
            ErrorCode::NotFound,
            "O áudio desta reunião já foi apagado pela política de retenção.",
            false,
        ));
    }
    let root = audio_root(&app, &meeting)?;
    let mode = match mode.as_deref() {
        Some("mic") => mos_audio::ClipMode::Mic,
        Some("system") => mos_audio::ClipMode::System,
        _ => mos_audio::ClipMode::Both,
    };
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        mos_audio::clip_wav_bytes(&root, mode, start_ms, duration_ms.unwrap_or(30_000))
    })
    .await
    .map_err(|error| CoreError::new(ErrorCode::Io, error.to_string(), true))?
    .map_err(audio_error)?;
    Ok(base64(&bytes))
}

fn base64(bytes: &[u8]) -> String {
    const TABELA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABELA[(n >> 18) as usize & 63] as char);
        out.push(TABELA[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABELA[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABELA[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

// ============================================================================
// Teste de audio
// ============================================================================

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTest {
    pub mic_ok: bool,
    pub system_ok: bool,
    /// O loopback abriu, mas nada tocou nos segundos do teste. Normal quando
    /// nada esta tocando — e diferente de "nao abriu".
    pub system_silent: bool,
    pub mic_level: u64,
    pub system_level: u64,
}

/// Grava tres segundos numa pasta temporaria e diz o que ouviu.
///
/// Nao roda durante uma gravacao: os dois disputariam o dispositivo.
#[tauri::command]
pub async fn meeting_audio_test(app: AppHandle) -> Result<AudioTest, CoreError> {
    if app.state::<RecordingState>().gravando() {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "Há uma gravação em curso. Teste o áudio depois de encerrar.",
            false,
        ));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let root = std::env::temp_dir().join(format!("mos-audio-test-{}", epoch_now()));
        let result = (|| {
            let recording = Recording::start(&root, "").map_err(audio_error)?;
            std::thread::sleep(Duration::from_secs(3));
            let state = recording.state();
            let outcome = recording.stop().map_err(audio_error)?;
            let mic_ok = outcome.mic.has_audio() && state.mic_peak > 0;
            let system_open = outcome.system.has_audio();
            Ok(AudioTest {
                mic_ok,
                system_ok: system_open,
                system_silent: system_open && state.system_peak == 0,
                mic_level: state.mic_peak,
                system_level: state.system_peak,
            })
        })();
        let _ = std::fs::remove_dir_all(&root);
        result
    })
    .await
    .map_err(|error| CoreError::new(ErrorCode::Io, error.to_string(), true))?
}

// ============================================================================
// Recuperacao e limpeza na abertura
// ============================================================================

/// A reconciliacao de abertura.
///
/// Uma reuniao em captura num processo recem-nascido significa que o anterior
/// morreu sem terminar. **Nada e apagado**: ela vira `interrupted` com a duracao
/// que o disco sustenta — e, com audio, entra no pipeline sozinha.
pub fn reconcile_on_open(app: &AppHandle) -> Result<Vec<Meeting>, CoreError> {
    let state = app.state::<AppState>();
    let handle = app.clone();
    let interrupted = state.meetings.reconcile_on_open(&move |meeting| {
        audio_root(&handle, meeting)
            .ok()
            .and_then(|root| mos_audio::recover_session(&root).ok())
            .map(|recovered| recovered.duration_ms)
            .unwrap_or(0)
    })?;
    state.meetings.auto_process_recovered()?;
    let consent = !crate::analysis_consent(&state.settings_path).is_empty();
    let _ = state.meetings.recover_pipeline_on_open(consent);
    Ok(interrupted)
}

/// Faxina da abertura: lixeira vencida, pastas marcadas, temporarios, retencao.
pub fn housekeeping_on_open(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(expired) = state.meetings.expired_trash() {
        for meeting in expired {
            if let Err(error) = delete_permanently(app, &meeting.id.to_string()) {
                log(
                    crate::diagnostico::Nivel::Aviso,
                    &format!(
                        "lixeira: nao foi possivel apagar {}: {}",
                        meeting.id, error.message
                    ),
                );
            }
        }
    }

    // Pastas com marca de exclusao e sem linha: termina o que a exclusao
    // comecou. Sem marca, so relata.
    if let Ok(base) = data_dir(app) {
        if let Ok(entries) = std::fs::read_dir(base.join("meetings")) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let Ok(id) = mos_core::MeetingId::parse(name) else {
                    continue;
                };
                if state.meetings.meeting(&id.to_string()).is_ok() {
                    continue;
                }
                if path.join(TOMBSTONE).exists() {
                    let _ = std::fs::remove_dir_all(&path);
                } else {
                    log(
                        crate::diagnostico::Nivel::Aviso,
                        &format!("pasta de reuniao sem linha no banco (mantida): {name}"),
                    );
                }
            }
        }
    }

    // WAVs temporarios de transcricao: derivados, e sobram quando o processo
    // morre no meio. Mais de uma hora parados nao pertencem a ninguem.
    if let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_ours = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("mos-meeting-") || n.starts_with("mos-audio-test-"));
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| m.elapsed().ok())
                .is_some_and(|age| age > Duration::from_secs(3600));
            if is_ours && old {
                let _ = std::fs::remove_dir_all(&path);
            }
        }
    }

    if let Err(error) = clean_expired_audio(app) {
        log(
            crate::diagnostico::Nivel::Aviso,
            &format!("limpeza de audio falhou: {}", error.message),
        );
    }
}

/// Apaga o audio que a politica de retencao ja liberou. Apaga primeiro, marca
/// depois.
pub fn clean_expired_audio(app: &AppHandle) -> Result<usize, CoreError> {
    let state = app.state::<AppState>();
    let mut cleaned = 0usize;
    for meeting in state.meetings.audio_to_clean()? {
        // Na fila para transcrever de novo (corte que cresceu, retry manual): o
        // audio e o insumo. Nao apaga.
        if state
            .meetings
            .job(&meeting.id.to_string())?
            .is_some_and(|job| job.status.is_open() && job.stage == JobStage::Transcription)
        {
            continue;
        }
        let root = audio_root(app, &meeting)?;
        if mos_audio::delete_session_audio(&root).is_ok() {
            state.meetings.mark_audio_deleted(&meeting.id.to_string())?;
            cleaned += 1;
        }
    }
    Ok(cleaned)
}

// ============================================================================
// O laco da gravacao: tick, nivel e Recording Guardian
// ============================================================================

/// O nivel, quinze vezes por segundo. Dois numeros, nunca PCM.
pub async fn run_levels(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_millis(66)).await;
        let recorder = app.state::<RecordingState>();
        let Ok(active) = recorder.active.lock() else {
            continue;
        };
        let Some(current) = active.as_ref() else {
            continue;
        };
        let state = current.recording.state();
        drop(active);
        let _ = app.emit(
            "meeting-level",
            MeetingLevel {
                mic: state.mic_level,
                system: state.system_level,
            },
        );
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingLevel {
    pub mic: u64,
    pub system: u64,
}

/// Os sinais caros, lidos em ritmo proprio.
struct Sinais {
    users: Vec<MicUser>,
    users_at: Option<Instant>,
    app_running: Option<bool>,
    locked: bool,
    processes_at: Option<Instant>,
    system: sysinfo::System,
}

impl Sinais {
    fn new() -> Self {
        Self {
            users: Vec::new(),
            users_at: None,
            app_running: None,
            locked: false,
            processes_at: None,
            system: sysinfo::System::new(),
        }
    }

    /// Microfone a cada 2 s, processos a cada 5 s. O registro e barato; a
    /// varredura de processos custa dezenas de milissegundos e nao precisa de
    /// mais que isso para um sinal que so conta depois de 90 s.
    fn refresh(&mut self, associated: Option<&str>) {
        if self
            .users_at
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(2))
        {
            self.users = mic_users();
            self.users_at = Some(Instant::now());
        }
        if self
            .processes_at
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(5))
        {
            self.system
                .refresh_processes(sysinfo::ProcessesToUpdate::All, false);
            let names: Vec<String> = self
                .system
                .processes()
                .values()
                .map(|process| process.name().to_string_lossy().to_lowercase())
                .collect();
            // A tela de bloqueio do Windows e um processo com nome. Nome de
            // executavel, e nada alem (ADR-037).
            self.locked = names.iter().any(|name| name == "logonui.exe");
            self.app_running = associated
                .filter(|app| app.to_lowercase().ends_with(".exe"))
                .map(|app| names.iter().any(|name| *name == app.to_lowercase()));
            self.processes_at = Some(Instant::now());
        }
    }
}

/// O laco que alimenta a interface enquanto grava, e que roda o Guardian.
///
/// Uma emissao por segundo, e SO enquanto existe gravacao. Ele tambem e quem
/// percebe que os dois canais morreram: **nunca fingir que continua gravando**
/// (§20).
pub async fn run(app: AppHandle) {
    let mut sinais = Sinais::new();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 1. Quem e o app associado — sem segurar a trava enquanto se le o
        //    registro e se varre processos.
        let associated = {
            let recorder = app.state::<RecordingState>();
            let Ok(active) = recorder.active.lock() else {
                continue;
            };
            match active.as_ref() {
                Some(current) => current.guardian.associated_app.clone(),
                None => continue,
            }
        };
        sinais.refresh(associated.as_deref());
        let config = preferences(&app).guardian;

        // 2. A volta do Guardian, com a trava.
        let (frame, events, meeting_id, changed) = {
            let recorder = app.state::<RecordingState>();
            let Ok(mut active) = recorder.active.lock() else {
                continue;
            };
            let Some(current) = active.as_mut() else {
                continue;
            };
            let state = current.recording.state();
            let (mic_level, system_level) = current.recording.take_window_levels();
            let observation = Observation {
                now: epoch_now(),
                duration_ms: state.duration_ms,
                paused: current.recording.is_paused(),
                mic_level,
                system_level,
                mic_alive: matches!(state.mic, ChannelState::Capturing),
                system_alive: matches!(state.system, ChannelState::Capturing),
                mic_users: sinais.users.clone(),
                app_running: sinais.app_running,
                locked: sinais.locked,
                idle_secs: crate::monitor::idle_seconds(),
            };
            let (view, events) =
                meeting_guardian::observe(&mut current.guardian, &observation, &config);
            let changed = view != current.view || matches!(view, GuardianView::Countdown { .. });
            current.view = view;
            (tick(current), events, current.meeting_id.clone(), changed)
        };

        let both_gone = !frame.mic.has_audio() && !frame.system.has_audio();
        let _ = app.emit("meeting-tick", &frame);
        update_tray(&app, Some(frame.duration_ms), frame.paused, &frame.guardian);
        if changed {
            let _ = app.emit("meeting-guardian", &frame.guardian);
            match &frame.guardian {
                GuardianView::Idle => hide_overlay(&app),
                view => show_overlay(&app, view, frame.duration_ms),
            }
        }

        let state = app.state::<AppState>();
        let mut auto_stop = None;
        for event in events {
            match event {
                GuardianEvent::Suggested {
                    confidence,
                    trigger,
                } => {
                    let _ = state.meetings.record_guardian_event(
                        &meeting_id,
                        "suggested",
                        Some(confidence),
                        &trigger,
                        None,
                    );
                    log(
                        crate::diagnostico::Nivel::Info,
                        &format!(
                            "guardian sugeriu encerrar id={meeting_id} guardian_trigger={trigger}"
                        ),
                    );
                }
                GuardianEvent::LongPrompted { confidence } => {
                    let _ = state.meetings.record_guardian_event(
                        &meeting_id,
                        "long_prompted",
                        Some(confidence),
                        "long_meeting",
                        None,
                    );
                }
                GuardianEvent::CountdownStarted {
                    confidence,
                    trigger,
                } => {
                    let _ = state.meetings.record_guardian_event(
                        &meeting_id,
                        "countdown_started",
                        Some(confidence),
                        &trigger,
                        None,
                    );
                }
                GuardianEvent::CountdownCancelled => {
                    let _ = state.meetings.record_guardian_event(
                        &meeting_id,
                        "countdown_cancelled",
                        None,
                        "signal",
                        None,
                    );
                }
                GuardianEvent::Cleared => {}
                GuardianEvent::AutoStop {
                    reason,
                    confidence,
                    trigger,
                } => {
                    let _ = state.meetings.record_guardian_event(
                        &meeting_id,
                        "auto_stopped",
                        Some(confidence),
                        &trigger,
                        None,
                    );
                    auto_stop = Some((reason, trigger));
                }
                GuardianEvent::MicSilent => {
                    let _ = state.meetings.record_guardian_event(
                        &meeting_id,
                        "health_warning",
                        None,
                        "mic_silent",
                        None,
                    );
                    notify(
                        &app,
                        "O microfone parece mudo",
                        "A chamada está com o microfone, mas nenhum som chega à gravação. Confira se ele não está silenciado.",
                    );
                }
            }
        }

        if let Some((reason, trigger)) = auto_stop {
            log(
                crate::diagnostico::Nivel::Info,
                &format!(
                    "guardian encerrou id={meeting_id} stop_reason={} guardian_trigger={trigger}",
                    reason.as_str()
                ),
            );
            if finish_recording(&app, reason).is_ok() {
                notify(
                    &app,
                    "Gravação encerrada",
                    "A reunião parecia ter terminado. O M/OS está organizando o que foi dito.",
                );
            }
            continue;
        }

        if both_gone {
            // Encerra pelo mesmo caminho do clique, para que o estado e os
            // arquivos terminem exatamente como terminariam num Stop normal.
            let _ = finish_recording(&app, StopReason::DeviceFailure);
        }
    }
}

/// Encerra a gravacao quando o processo esta saindo.
pub fn shutdown(app: &AppHandle) {
    if app.state::<RecordingState>().gravando() {
        let _ = finish_recording(app, StopReason::AppExit);
    }
}

/// A pergunta do Guardian na janelinha de sobreposicao.
///
/// **A mesma janela da oferta de gravar**: as duas nunca coexistem, porque a
/// oferta so aparece sem gravacao em curso e o Guardian so existe com uma.
/// Nao rouba foco.
fn show_overlay(app: &AppHandle, view: &GuardianView, duration_ms: i64) {
    let Some(window) = app.get_webview_window("reuniao-detectada") else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        if let Ok(Some(tela)) = window.current_monitor() {
            let screen = tela.size();
            let scale = tela.scale_factor();
            if let Ok(size) = window.outer_size() {
                let margin = (24.0 * scale) as u32;
                let x = screen.width.saturating_sub(size.width + margin);
                let y = screen.height.saturating_sub(size.height + margin * 3);
                let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
            }
        }
        let _ = window.show();
        let _ = window.set_always_on_top(true);
    }
    let _ = window.emit(
        "reuniao-terminou",
        serde_json::json!({ "view": view, "durationMs": duration_ms }),
    );
}

fn hide_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("reuniao-detectada") {
        let _ = window.emit(
            "reuniao-terminou",
            serde_json::json!({ "view": { "kind": "idle" } }),
        );
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        }
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CoreError {
    CoreError::new(
        ErrorCode::StorageUnavailable,
        format!("O estado da gravacao foi interrompido: {error}"),
        false,
    )
}

/// Mostra, esconde e atualiza o estado de gravacao no tray.
///
/// `None` volta ao menu de repouso. A troca de menu so acontece na TRANSICAO;
/// o relogio de cada segundo e `set_text`, que nao reconstroi nada.
pub fn update_tray(
    app: &AppHandle,
    duration_ms: Option<i64>,
    paused: bool,
    guardian: &GuardianView,
) {
    use std::sync::atomic::Ordering;

    let Some(handles) = app.try_state::<crate::TrayHandles>() else {
        return;
    };
    match duration_ms {
        Some(ms) => {
            let total = ms / 1000;
            let clock = if total >= 3600 {
                format!(
                    "{}:{:02}:{:02}",
                    total / 3600,
                    (total / 60) % 60,
                    total % 60
                )
            } else {
                format!("{:02}:{:02}", total / 60, total % 60)
            };
            let suffix = match guardian {
                GuardianView::Ask { .. } => " · terminou?",
                GuardianView::Countdown { .. } => " · encerrando",
                GuardianView::Idle if paused => " · pausada",
                GuardianView::Idle => "",
            };
            let label = format!("● Reunião · {clock}{suffix}");
            let _ = handles.clock.set_text(&label);
            let _ = handles
                .pause
                .set_text(if paused { "Retomar" } else { "Pausar" });
            let _ = handles.tray.set_tooltip(Some(label.as_str()));
            if !handles.live_shown.swap(true, Ordering::Relaxed) {
                let _ = handles.tray.set_menu(Some(handles.live.clone()));
            }
        }
        None => {
            if handles.live_shown.swap(false, Ordering::Relaxed) {
                let _ = handles.tray.set_menu(Some(handles.idle.clone()));
            }
            let _ = handles.tray.set_tooltip(Some("M/OS"));
        }
    }
}

/// Parar pelo tray. O caminho e o MESMO do clique na barra.
pub fn stop_from_tray(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = finish_recording(&handle, StopReason::Manual);
    });
}

/// Pausar ou retomar pelo tray.
pub fn toggle_pause_from_tray(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let paused = handle
            .state::<RecordingState>()
            .active
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|active| active.recording.is_paused()));
        let state = handle.state::<AppState>();
        let recorder = handle.state::<RecordingState>();
        let _ = match paused {
            Some(true) => meeting_resume(handle.clone(), state, recorder).map(|_| ()),
            Some(false) => meeting_pause(handle.clone(), state, recorder).map(|_| ()),
            None => Ok(()),
        };
    });
}

/// Marcar momento pelo tray ou pelo atalho.
pub fn mark_from_shortcut(app: &AppHandle) {
    let _ = meeting_mark_moment(app.clone());
}

/// `Ctrl+Alt+M`: inicia quando nada grava, marca momento quando grava.
///
/// **Iniciar pelo atalho e um clique.** A pessoa apertou; a barra aparece no
/// mesmo segundo. O que a §17.2 proibe e gravar sem indicacao, e nao gravar por
/// teclado.
pub fn primary_shortcut(app: &AppHandle) {
    if app.state::<RecordingState>().gravando() {
        mark_from_shortcut(app);
        return;
    }
    let consent = {
        let state = app.state::<AppState>();
        !crate::analysis_consent(&state.settings_path).is_empty()
    };
    if !consent {
        // A primeira gravacao da vida passa pela tela de consentimento.
        crate::reveal_window(app, "main");
        let _ = app.emit("meeting-consent-needed", ());
        return;
    }
    let state = app.state::<AppState>();
    let recorder = app.state::<RecordingState>();
    let _ = meeting_start(app.clone(), state, recorder, "", None, None, None);
}

/// `Ctrl+Alt+Shift+M`: encerra.
pub fn stop_shortcut(app: &AppHandle) {
    if app.state::<RecordingState>().gravando() {
        stop_from_tray(app);
    }
}

// ============================================================================
// Transcritor e consentimento
// ============================================================================

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriberStatus {
    pub configured: bool,
    pub ready: bool,
    pub problem: String,
    pub name: String,
    pub binary: String,
    pub model: String,
    pub vad_model: String,
    pub threads: u32,
}

fn provider(app: &AppHandle) -> mos_transcribe::WhisperCliProvider {
    let state = app.state::<AppState>();
    mos_transcribe::WhisperCliProvider::new(crate::whisper_config(&state.settings_path))
}

#[tauri::command]
pub fn meeting_transcriber_status(app: AppHandle) -> TranscriberStatus {
    let config = {
        let state = app.state::<AppState>();
        crate::whisper_config(&state.settings_path)
    };
    let provider = mos_transcribe::WhisperCliProvider::new(config.clone());
    let ready = provider.ready();
    TranscriberStatus {
        configured: config.is_set(),
        ready: ready.is_ok(),
        problem: ready
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default(),
        name: provider.name(),
        binary: config.binary,
        model: config.model,
        vad_model: config.vad_model,
        threads: config.threads,
    }
}

#[tauri::command]
pub fn meeting_set_transcriber(
    app: AppHandle,
    binary: &str,
    model: &str,
    threads: u32,
    vad_model: &str,
) -> Result<TranscriberStatus, CoreError> {
    {
        let state = app.state::<AppState>();
        crate::set_whisper_config(
            &state.settings_path,
            mos_transcribe::WhisperConfig {
                binary: binary.trim().to_owned(),
                model: model.trim().to_owned(),
                threads,
                vad_model: vad_model.trim().to_owned(),
            },
        )?;
    }
    wake_pipeline(&app);
    Ok(meeting_transcriber_status(app))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisConsent {
    pub granted: bool,
    pub granted_at: String,
}

#[tauri::command]
pub fn meeting_analysis_consent(state: tauri::State<'_, AppState>) -> AnalysisConsent {
    let at = crate::analysis_consent(&state.settings_path);
    AnalysisConsent {
        granted: !at.is_empty(),
        granted_at: at,
    }
}

/// Concede ou revoga o consentimento de enviar transcricao ao Hermes — uma vez,
/// e nao a cada reuniao (D-A).
#[tauri::command]
pub fn meeting_set_analysis_consent(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    granted: bool,
) -> Result<AnalysisConsent, CoreError> {
    let at = if granted {
        state
            .clock
            .now()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    } else {
        String::new()
    };
    crate::set_analysis_consent(&state.settings_path, &at)?;
    if granted {
        let _ = state.meetings.release_waiting("consent_missing");
        wake_pipeline(&app);
    }
    Ok(AnalysisConsent {
        granted: !at.is_empty(),
        granted_at: at,
    })
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardianStats {
    pub counts: Vec<(String, i64)>,
}

/// Metrica local do Guardian: quantas vezes sugeriu, quantas a pessoa quis
/// continuar, quanto excesso foi cortado.
#[tauri::command]
pub fn meeting_guardian_stats(
    state: tauri::State<'_, AppState>,
) -> Result<GuardianStats, CoreError> {
    Ok(GuardianStats {
        counts: state.meetings.guardian_counts()?,
    })
}

/// O estado tecnico de uma reuniao, para o painel de desenvolvimento.
#[tauri::command]
pub fn meeting_debug(app: AppHandle, id: &str) -> Result<serde_json::Value, CoreError> {
    let state = app.state::<AppState>();
    let job = state.meetings.job(id)?;
    let meeting = state.meetings.meeting(id)?;
    let guardian = app
        .state::<RecordingState>()
        .active
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .filter(|active| active.meeting_id == id)
                .map(|active| active.guardian.clone())
        });
    Ok(serde_json::json!({
        "status": meeting.status.as_str(),
        "stopReason": meeting.stop_reason.map(StopReason::as_str),
        "associatedApp": meeting.associated_app,
        "suggestedEndMs": meeting.suggested_end_ms,
        "trim": [meeting.trim_start_ms, meeting.trim_end_ms],
        "trimOrigin": meeting.trim_origin.map(TrimOrigin::as_str),
        "retention": meeting.retention.as_str(),
        "audioDeletedAt": meeting.audio_deleted_at.map(|at| at.to_string()),
        "job": job,
        "guardian": guardian,
    }))
}

// ============================================================================
// O pipeline
// ============================================================================

/// O sino do pipeline: parar, tentar de novo e configurar tocam aqui, e o laco
/// acorda na hora em vez de esperar a proxima volta.
#[derive(Default)]
pub struct PipelineRuntime {
    wake: tokio::sync::Notify,
}

pub fn wake_pipeline(app: &AppHandle) {
    if let Some(runtime) = app.try_state::<PipelineRuntime>() {
        runtime.wake.notify_one();
    }
}

/// Compatibilidade com a V1: "Transcrever" poe na fila.
#[tauri::command]
pub fn meeting_transcribe(app: AppHandle, id: String) -> Result<Meeting, CoreError> {
    let state = app.state::<AppState>();
    state.meetings.queue(&id, JobStage::Transcription)?;
    wake_pipeline(&app);
    state.meetings.meeting(&id)
}

/// Compatibilidade com a V1: "Analisar" poe na fila.
#[tauri::command]
pub fn meeting_analyze(app: AppHandle, id: String) -> Result<Meeting, CoreError> {
    let state = app.state::<AppState>();
    if crate::analysis_consent(&state.settings_path).is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "A analise envia a transcricao ao Hermes, e isso ainda nao foi autorizado.",
            false,
        ));
    }
    state.meetings.queue(&id, JobStage::Analysis)?;
    wake_pipeline(&app);
    state.meetings.meeting(&id)
}

/// "Tentar de novo" / "Processar agora": zera a contagem e acorda o laco.
#[tauri::command]
pub fn meeting_retry(app: AppHandle, id: String) -> Result<Meeting, CoreError> {
    let state = app.state::<AppState>();
    state.meetings.retry_job(&id)?;
    wake_pipeline(&app);
    let _ = app.emit("data-changed", "meeting");
    state.meetings.meeting(&id)
}

/// O progresso que a tela ve.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobProgress {
    meeting_id: String,
    stage: JobStage,
    /// Dentro do estagio, `0..=1`.
    progress: f32,
    /// A fracao global ponderada, `0..=1`.
    overall: f32,
    /// Detalhe do estagio: `mic`/`system` na transcricao; `2/3` na analise.
    detail: String,
}

fn emit_progress(app: &AppHandle, id: &str, stage: JobStage, progress: f32, detail: &str) {
    let weight = mos_core::meeting_pipeline::TRANSCRIPTION_WEIGHT;
    let overall = match stage {
        JobStage::Transcription => 0.05 + progress.clamp(0.0, 1.0) * (weight - 0.05),
        JobStage::Analysis => weight + progress.clamp(0.0, 1.0) * (1.0 - weight),
    };
    let _ = app.emit(
        "meeting-progress",
        JobProgress {
            meeting_id: id.to_owned(),
            stage,
            progress,
            overall,
            detail: detail.to_owned(),
        },
    );
}

/// O laco que processa: um job por vez, sem a tela participar.
pub async fn run_pipeline(app: AppHandle) {
    // Deixa a abertura respirar: a reconciliacao e a primeira tela vem antes.
    tokio::time::sleep(Duration::from_secs(8)).await;
    loop {
        process_due(&app).await;
        let runtime = app.state::<PipelineRuntime>();
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(15)) => {}
            _ = runtime.wake.notified() => {}
        }
    }
}

async fn process_due(app: &AppHandle) {
    let (consent, transcriber_ready) = {
        let state = app.state::<AppState>();
        (
            !crate::analysis_consent(&state.settings_path).is_empty(),
            provider(app).ready().is_ok(),
        )
    };
    {
        let state = app.state::<AppState>();
        if transcriber_ready {
            let _ = state.meetings.release_waiting("transcriber_missing");
        }
        if consent {
            let _ = state.meetings.release_waiting("consent_missing");
        }
    }

    // Um por vez, ate a fila esvaziar: o whisper usa todos os nucleos, e duas
    // transcricoes simultaneas nao terminam antes de uma.
    for _ in 0..16 {
        let next = {
            let state = app.state::<AppState>();
            match state.meetings.due_jobs() {
                Ok(due) => due.into_iter().next(),
                Err(_) => None,
            }
        };
        let Some(job) = next else {
            return;
        };
        let id = job.meeting_id.to_string();
        match job.stage {
            JobStage::Transcription => run_transcription_job(app, &id, consent).await,
            JobStage::Analysis => run_analysis_job(app, &id, consent).await,
        }
        let _ = app.emit("data-changed", "meeting");
    }
}

fn record_failure(app: &AppHandle, id: &str, stage: JobStage, failure: StageFailure) {
    let state = app.state::<AppState>();
    let attempts = state
        .meetings
        .job(id)
        .ok()
        .flatten()
        .map(|job| job.attempt_count)
        .unwrap_or(0);
    match state.meetings.fail_job(id, &failure) {
        Ok((meeting, after)) => {
            log(
                crate::diagnostico::Nivel::Aviso,
                &format!(
                    "pipeline falhou id={id} stage={} error_code={} retry_count={} depois={after:?}",
                    stage.as_str(),
                    failure.code,
                    attempts + 1,
                ),
            );
            if matches!(after, mos_core::AfterFailure::GiveUp) {
                let _ = app.emit("meeting-failed", id);
                notify(
                    app,
                    "Uma reunião precisa de você",
                    &format!("“{}”: {}", meeting.title, failure.message),
                );
            } else {
                let _ = app.emit("meeting-waiting", id);
            }
        }
        Err(error) => log(
            crate::diagnostico::Nivel::Erro,
            &format!("pipeline nao registrou a falha id={id}: {}", error.message),
        ),
    }
}

/// Uma pasta temporaria por reuniao, apagada sempre — sucesso ou falha.
fn temp_dir_for(id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("mos-meeting-{id}"))
}

struct TempGuard(PathBuf);

impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Abaixo disto (amplitude RMS de i16, em janelas de 1 s), o canal e silencio
/// ou ruido de fundo e nao vale uma passada do whisper. ≈ −60 dBFS.
const SILENT_CHANNEL_RMS: f64 = 33.0;

async fn run_transcription_job(app: &AppHandle, id: &str, consent: bool) {
    let started = Instant::now();
    if let Err(error) = provider(app).ready() {
        record_failure(
            app,
            id,
            JobStage::Transcription,
            mos_core::meeting_pipeline::classify_transcription_error(&error),
        );
        return;
    }
    let meeting = {
        let state = app.state::<AppState>();
        match state.meetings.begin_job(id) {
            Ok((meeting, _)) => meeting,
            Err(error) => {
                log(
                    crate::diagnostico::Nivel::Aviso,
                    &format!("pipeline nao comecou id={id}: {}", error.message),
                );
                return;
            }
        }
    };
    emit_progress(app, id, JobStage::Transcription, 0.0, "audio");

    let handle = app.clone();
    let owned = id.to_owned();
    let outcome =
        tauri::async_runtime::spawn_blocking(move || transcribe_channels(&handle, &owned, meeting))
            .await
            .unwrap_or_else(|_| {
                Err(StageFailure::new(
            "worker_panicked",
            "A transcrição foi interrompida. O áudio está seguro e o M/OS vai tentar de novo.",
            FailureClass::Transient,
        ))
            });

    match outcome {
        Ok(segments) => {
            let segments = normalize_transcript(app, segments);
            let state = app.state::<AppState>();
            let count = segments.len();
            let has_speech = count > 0;
            match state
                .meetings
                .complete_transcription_job(id, segments, consent && has_speech)
            {
                Ok(meeting) => {
                    log(
                        crate::diagnostico::Nivel::Info,
                        &format!(
                            "transcricao concluida id={id} stage=transcription duration_ms={} segmentos={count}",
                            started.elapsed().as_millis(),
                        ),
                    );
                    let _ = app.emit("meeting-transcribed", &meeting);
                    if !has_speech || !consent {
                        let _ = clean_expired_audio(app);
                        notify_ready(app, id);
                    }
                }
                Err(error) => record_failure(
                    app,
                    id,
                    JobStage::Transcription,
                    StageFailure::new(
                        "transcript_write_failed",
                        format!(
                            "Não foi possível guardar a transcrição ({}). O áudio está seguro.",
                            error.message
                        ),
                        FailureClass::Transient,
                    ),
                ),
            }
        }
        Err(failure) => record_failure(app, id, JobStage::Transcription, failure),
    }
}

/// Transcreve os dois canais da faixa efetiva e intercala.
///
/// **Um canal por chamada, e nunca os dois juntos.** MIC e o usuario local,
/// SYSTEM sao os remotos — a distincao que a V1 protege acima de qualquer outra.
fn transcribe_channels(
    app: &AppHandle,
    id: &str,
    meeting: Meeting,
) -> Result<Vec<TranscriptSegment>, StageFailure> {
    use mos_audio::Channel as AudioChannel;
    use mos_core::{MeetingChannel, TranscriptionRequest};

    if meeting.audio_deleted_at.is_some() {
        return Err(StageFailure::new(
            "no_audio",
            "O áudio desta reunião já foi apagado, então não dá para transcrever de novo.",
            FailureClass::Permanent,
        ));
    }
    let root = audio_root(app, &meeting)
        .map_err(|error| StageFailure::new("audio_path", error.message, FailureClass::Permanent))?;
    let (start_ms, end_ms) = meeting.effective_range();
    if end_ms <= start_ms {
        return Err(StageFailure::new(
            "no_audio",
            "Não há áudio gravado nesta reunião.",
            FailureClass::Permanent,
        ));
    }

    let provider = provider(app);
    let work = temp_dir_for(id);
    std::fs::create_dir_all(&work).map_err(|error| {
        StageFailure::new(
            "temp_unavailable",
            format!("Não foi possível preparar o áudio ({error}). O M/OS vai tentar de novo."),
            FailureClass::Transient,
        )
    })?;
    let _guard = TempGuard(work.clone());

    let state = app.state::<AppState>();
    let mut por_canal = Vec::new();
    for (index, (audio_channel, domain_channel)) in [
        (AudioChannel::Mic, MeetingChannel::Mic),
        (AudioChannel::System, MeetingChannel::System),
    ]
    .into_iter()
    .enumerate()
    {
        let base = index as f32 * 0.5;
        let _ = state.meetings.job_progress(id, base);
        let wav = work.join(format!("{}.wav", audio_channel.folder()));
        let (frames, _gain) = mos_audio::export_channel_range_normalized(
            &root,
            audio_channel,
            &wav,
            start_ms,
            end_ms,
        )
        .map_err(|error| {
            StageFailure::new(
                "audio_export_failed",
                format!("Não foi possível preparar o áudio ({error}). A gravação está segura."),
                FailureClass::Transient,
            )
        })?;
        if frames == 0 || channel_is_silent(&root, audio_channel, start_ms, end_ms) {
            // Canal sem audio util nao e falha: um dos dois pode ter caido, e o
            // outro continua sendo a reuniao. E silencio nao vale meia hora de
            // whisper — nem o laco de "Legenda por..." que ele inventa no vazio.
            por_canal.push(Vec::new());
            continue;
        }

        let channel_name = audio_channel.folder().to_owned();
        let segments = provider
            .transcribe(
                TranscriptionRequest {
                    audio: &wav,
                    channel: domain_channel,
                    language: Some("pt"),
                },
                &|fraction| {
                    emit_progress(
                        app,
                        id,
                        JobStage::Transcription,
                        base + fraction * 0.5,
                        &channel_name,
                    );
                },
            )
            .map_err(|error| mos_core::meeting_pipeline::classify_transcription_error(&error))?;
        // Os timestamps do whisper sao do ARQUIVO, que comeca no inicio da
        // faixa. Somar `start_ms` devolve a regua da reuniao — e a evidencia
        // `14:04` continua significando 14:04 da gravacao.
        let shifted = segments
            .into_iter()
            .map(|mut segment| {
                segment.start_ms += start_ms;
                segment.end_ms += start_ms;
                segment
            })
            .collect::<Vec<_>>();
        por_canal.push(shifted);
    }

    let system = por_canal.pop().unwrap_or_default();
    let mic = por_canal.pop().unwrap_or_default();
    Ok(mos_core::interleave(meeting.id, mic, system))
}

fn channel_is_silent(
    root: &std::path::Path,
    channel: mos_audio::Channel,
    start_ms: i64,
    end_ms: i64,
) -> bool {
    let Ok(samples) = mos_audio::read_channel_range(root, channel, start_ms, end_ms) else {
        return false;
    };
    if samples.is_empty() {
        return true;
    }
    // O maior RMS em janelas de 1 s: uma frase curta numa hora de silencio
    // ainda e fala, e a media da hora inteira a apagaria.
    samples.chunks(16_000).all(|window| {
        let sum: f64 = window.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
        (sum / window.len() as f64).sqrt() < SILENT_CHANNEL_RMS
    })
}

/// Aplica o vocabulario pessoal numa copia ao lado do texto cru.
fn normalize_transcript(
    app: &AppHandle,
    segments: Vec<TranscriptSegment>,
) -> Vec<TranscriptSegment> {
    let mut vocabulary = preferences(app).vocabulary;
    for project in project_candidates(app) {
        vocabulary.push(project.name);
    }
    if vocabulary.is_empty() {
        return segments;
    }
    segments
        .into_iter()
        .map(|mut segment| {
            let (normalized, corrections) =
                mos_core::meeting_text::normalize_with_vocabulary(&segment.text, &vocabulary);
            if !corrections.is_empty() {
                segment.text_normalized = Some(normalized);
                segment.corrections = corrections;
            }
            segment
        })
        .collect()
}

const REPROMPTS: usize = 1;

async fn run_analysis_job(app: &AppHandle, id: &str, consent: bool) {
    let started = Instant::now();
    if !consent {
        record_failure(
            app,
            id,
            JobStage::Analysis,
            StageFailure::new(
                "consent_missing",
                "A transcrição está pronta. A organização com o Hermes espera a autorização em Settings.",
                FailureClass::Configuration,
            ),
        );
        return;
    }
    let meeting = {
        let state = app.state::<AppState>();
        match state.meetings.begin_job(id) {
            Ok((meeting, _)) => meeting,
            Err(error) => {
                log(
                    crate::diagnostico::Nivel::Aviso,
                    &format!("analise nao comecou id={id}: {}", error.message),
                );
                return;
            }
        }
    };
    emit_progress(app, id, JobStage::Analysis, 0.0, "");

    match analyze(app, &meeting).await {
        Ok(()) => {
            let state = app.state::<AppState>();
            let offset = crate::surface::now_local(app).offset();
            let _ = state.meetings.resolve_dues(id, offset);
            log(
                crate::diagnostico::Nivel::Info,
                &format!(
                    "analise concluida id={id} stage=analysis duration_ms={}",
                    started.elapsed().as_millis()
                ),
            );
            if let Ok(meeting) = state.meetings.meeting(id) {
                let _ = app.emit("meeting-analyzed", &meeting);
            }
            let _ = clean_expired_audio(app);
            notify_ready(app, id);
        }
        Err(failure) => record_failure(app, id, JobStage::Analysis, failure),
    }
}

/// "Reuniao pronta — 3 acoes · 2 decisoes", quando a pessoa nao esta olhando.
fn notify_ready(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let Ok(overview) = state.meetings.overview_one(id) else {
        return;
    };
    let _ = app.emit("meeting-ready", &overview);
    let focused = app
        .get_webview_window("main")
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false);
    if focused {
        return;
    }
    let mut partes = Vec::new();
    match overview.pending_actions {
        0 => {}
        1 => partes.push("1 ação identificada".to_owned()),
        n => partes.push(format!("{n} ações identificadas")),
    }
    match overview.decisions {
        0 => {}
        1 => partes.push("1 decisão registrada".to_owned()),
        n => partes.push(format!("{n} decisões registradas")),
    }
    let corpo = if partes.is_empty() {
        "Resumo e transcrição prontos.".to_owned()
    } else {
        partes.join(" · ")
    };
    notify(
        app,
        &format!("Reunião pronta — {}", overview.meeting.title),
        &corpo,
    );
}

/// Onde a analise esta, em janelas. Janela, e nao porcentagem inventada.
fn anuncia_janela(app: &AppHandle, meeting_id: &str, window: u32, windows: u32) {
    let fraction = if windows == 0 {
        0.0
    } else if window == 0 {
        windows as f32 / (windows as f32 + 1.0)
    } else {
        (window - 1) as f32 / (windows as f32 + 1.0)
    };
    let state = app.state::<AppState>();
    let _ = state.meetings.job_progress(meeting_id, fraction);
    emit_progress(
        app,
        meeting_id,
        JobStage::Analysis,
        fraction,
        &if window == 0 {
            "juntando".to_owned()
        } else {
            format!("{window}/{windows}")
        },
    );
}

fn hermes_failure(_error: impl std::fmt::Display) -> StageFailure {
    // O texto do erro do Hermes NAO vai para a frase da pessoa nem para o log:
    // ele pode ecoar trecho do prompt, e o prompt e a reuniao (§16.3).
    StageFailure::new(
        "hermes_unreachable",
        "A transcrição está pronta. O Hermes não respondeu, e a organização tenta de novo sozinha.",
        FailureClass::Transient,
    )
}

async fn analyze(app: &AppHandle, meeting: &Meeting) -> Result<(), StageFailure> {
    let id = meeting.id.to_string();
    let (segments, base_url, bookmarks, active_timer) = {
        let state = app.state::<AppState>();
        let segments = state.meetings.transcript(&id).unwrap_or_default();
        let bookmarks = state
            .meetings
            .bookmarks(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|bookmark| bookmark.at_ms)
            .collect::<Vec<_>>();
        let active_timer = state
            .tracking
            .active_timer()
            .ok()
            .flatten()
            .map(|timer| timer.project_id);
        (
            segments,
            crate::hermes::current_base_url(app),
            bookmarks,
            active_timer,
        )
    };

    if segments.is_empty() {
        return Err(StageFailure::new(
            "no_transcript",
            "Esta reunião não tem fala transcrita para organizar.",
            FailureClass::Permanent,
        ));
    }

    let projects = project_candidates(app);
    let project_names: Vec<String> = projects.iter().map(|p| p.name.clone()).collect();
    let vocabulary = preferences(app).vocabulary;
    let windows = mos_core::build_windows(&segments, mos_core::WINDOW_BUDGET_CHARS);
    let instructions = mos_core::instructions_v2(
        &meeting.title,
        &meeting.notes,
        &mos_core::AnalysisContext {
            projects: &project_names,
            vocabulary: &vocabulary,
            bookmarks_ms: &bookmarks,
        },
    );

    // O REGISTRO do que sai (ADR-027): mede, e nao copia.
    let characters: usize = windows.iter().map(|window| window.text.len()).sum();
    let _ = app.emit(
        "meeting-sending",
        serde_json::json!({
            "meetingId": id,
            "segments": segments.len(),
            "characters": characters,
            "windows": windows.len(),
            "firstMs": segments.first().map(|s| s.start_ms).unwrap_or(0),
            "lastMs": segments.last().map(|s| s.end_ms).unwrap_or(0),
        }),
    );

    let outcome = if windows.len() == 1 {
        anuncia_janela(app, &id, 1, 1);
        ask_with_retry(
            &base_url,
            &format!("{instructions}\n\n---\n\n{}", windows[0].text),
            meeting.id,
            &segments,
        )
        .await?
    } else {
        consolidate(app, &base_url, &windows, &instructions, meeting, &segments).await?
    };

    let inference =
        mos_core::meeting_text::infer_project(&mos_core::meeting_text::ProjectInferenceInput {
            projects: &projects,
            notes: &meeting.notes,
            segments: &segments,
            active_timer_project: active_timer,
            hermes_hint: outcome.project_hint.as_deref(),
        });

    let state = app.state::<AppState>();
    let analysis = mos_core::MeetingAnalysis {
        meeting_id: meeting.id,
        summary: outcome.summary,
        model: "hermes".into(),
        produced_at: state.clock.now(),
        windows: windows.len().max(1) as u32,
    };
    state
        .meetings
        .complete_analysis_job(analysis, outcome.insights, outcome.title, inference)
        .map_err(|error| {
            StageFailure::new(
                "analysis_write_failed",
                format!(
                    "Não foi possível guardar a organização ({}). A transcrição está segura.",
                    error.message
                ),
                FailureClass::Transient,
            )
        })?;
    Ok(())
}

async fn ask_with_retry(
    base_url: &str,
    prompt: &str,
    meeting_id: mos_core::MeetingId,
    segments: &[TranscriptSegment],
) -> Result<mos_core::AnalysisOutcome, StageFailure> {
    let mut ultimo = String::new();
    for tentativa in 0..=REPROMPTS {
        let pedido = if tentativa == 0 {
            prompt.to_owned()
        } else {
            format!(
                "{prompt}\n\nA resposta anterior nao pode ser lida: {ultimo}\n\
                 Responda APENAS com o bloco cercado ```mos-meeting e o JSON dentro dele."
            )
        };
        let resposta = crate::hermes::ask_once(base_url, &pedido)
            .await
            .map_err(hermes_failure)?;
        match mos_core::parse_analysis(meeting_id, &resposta, segments) {
            Ok(outcome) => return Ok(outcome),
            Err(error) => ultimo = error.to_string(),
        }
    }
    Err(StageFailure::new(
        "analysis_unreadable",
        "O Hermes respondeu num formato que não deu para ler. A transcrição está segura, e a organização tenta de novo.",
        FailureClass::Transient,
    ))
}

async fn consolidate(
    app: &AppHandle,
    base_url: &str,
    windows: &[mos_core::PromptWindow],
    instructions: &str,
    meeting: &Meeting,
    segments: &[TranscriptSegment],
) -> Result<mos_core::AnalysisOutcome, StageFailure> {
    let id = meeting.id.to_string();
    let mut resumos = Vec::new();
    let mut insights = Vec::new();
    let mut topics: Vec<String> = Vec::new();
    let mut title = None;
    let mut project_hint = None;

    for (index, window) in windows.iter().enumerate() {
        anuncia_janela(app, &id, index as u32 + 1, windows.len() as u32);
        let prompt = format!(
            "{instructions}\n\n\
             Esta e a parte {} de {} da transcricao.\n\n---\n\n{}",
            index + 1,
            windows.len(),
            window.text
        );
        let parcial = ask_with_retry(base_url, &prompt, meeting.id, segments).await?;
        if !parcial.summary.is_empty() {
            resumos.push(parcial.summary);
        }
        topics.extend(parcial.topics);
        insights.extend(parcial.insights);
        title = title.or(parcial.title);
        project_hint = project_hint.or(parcial.project_hint);
    }

    topics.sort();
    topics.dedup();
    anuncia_janela(app, &id, 0, windows.len() as u32);

    let pedido = format!(
        "Estes sao os resumos parciais de uma reuniao dividida em {} partes.\n\
         Escreva UM resumo unico, curto, em portugues, sem repetir, e um titulo util.\n\
         Responda apenas com o bloco cercado ```mos-meeting contendo \
         {{ \"summary\": \"...\", \"title\": \"...\" }}.\n\n{}",
        windows.len(),
        resumos.join("\n\n---\n\n")
    );
    let (summary, final_title) = match crate::hermes::ask_once(base_url, &pedido).await {
        Ok(resposta) => match mos_core::parse_analysis(meeting.id, &resposta, segments) {
            Ok(outcome) => (outcome.summary, outcome.title),
            Err(_) => (resumos.join(" "), None),
        },
        Err(_) => (resumos.join(" "), None),
    };

    // As janelas se sobrepoem: a mesma acao pode ter vindo duas vezes.
    let insights = mos_core::meeting_text::dedupe_insights(insights);
    Ok(mos_core::AnalysisOutcome {
        summary,
        topics,
        insights,
        rejections: mos_core::Rejections::default(),
        title: final_title.or(title),
        project_hint,
    })
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_confere_com_a_referencia() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"RIFF"), "UklGRg==");
    }
}

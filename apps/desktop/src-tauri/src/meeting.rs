//! Comandos do Meeting Agent.
//!
//! **E o unico lugar onde `mos-audio` e `mos-core` se encontram.** O crate de
//! audio nao conhece Meeting e nao alcanca o banco; o dominio nao sabe que
//! WASAPI existe. A traducao entre os dois mora aqui, pelo mesmo desenho que
//! `jarvis.rs` usa para a ponte do Hermes (ADR-024).
//!
//! Casca fina de proposito: `SETUP-MAQUINA.md` §4 registra que
//! `cargo test -p mos-desktop` nao roda na maquina principal, e teste que nao
//! roda nao protege nada. Toda regra vive em `mos-core` ou
//! `mos-storage-sqlite`, e o que sobra aqui e adaptacao e laco.

use std::{path::PathBuf, sync::Mutex, time::Duration};

use mos_audio::{AudioError, ChannelState, Recording};
use mos_core::{
    AudioOutcome, ChannelOutcome, CoreError, ErrorCode, FailedStage, Meeting, MeetingAnalysis,
    MeetingInsight, MeetingStatus, TranscriptSegment, TranscriptionProvider,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

/// A gravacao viva do processo.
///
/// `Option` e nao uma fila: **uma gravacao por vez**. O dominio ja recusa a
/// segunda, e este `Mutex` e a mesma regra do lado do adapter — dois gravadores
/// disputariam o mesmo dispositivo.
#[derive(Default)]
pub struct RecordingState {
    active: Mutex<Option<Active>>,
}

struct Active {
    meeting_id: String,
    recording: Recording,
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
}

fn audio_error(error: AudioError) -> CoreError {
    let code = match error {
        AudioError::Unsupported => ErrorCode::InvalidTransition,
        AudioError::AlreadyRecording => ErrorCode::InvalidTransition,
        AudioError::Device(_) => ErrorCode::InvalidInput,
        AudioError::Storage { .. } | AudioError::Misaligned { .. } => ErrorCode::Io,
    };
    // `retryable: true` para falha de dispositivo e de disco: as duas mudam
    // sozinhas — o headset volta, o disco esvazia — e a interface pode oferecer
    // "tentar de novo" sem mentir.
    CoreError::new(code, error.to_string(), matches!(code, ErrorCode::Io))
}

/// Traduz o destino de um canal, do vocabulario do adapter para o do dominio.
///
/// Os dois enums existem separados de proposito: sem essa duplicacao, `mos-audio`
/// precisaria depender de `mos-core` e a fronteira da §4.2 deixaria de ser
/// mantida pelo compilador.
fn to_domain(state: ChannelState) -> ChannelOutcome {
    match state {
        ChannelState::Capturing => ChannelOutcome::Capturing,
        ChannelState::Captured => ChannelOutcome::Captured,
        ChannelState::Unavailable { reason } => ChannelOutcome::Unavailable { reason },
        ChannelState::Lost { at_ms, reason } => ChannelOutcome::Lost { at_ms, reason },
    }
}

/// O caminho absoluto do audio de uma reuniao.
///
/// Derivado de `audio_dir`, que por sua vez e derivado do `MeetingId` — **nenhum
/// path vem do renderer** (§18). E validado como filho do diretorio de dados
/// antes de qualquer escrita ou remocao.
fn audio_root(app: &AppHandle, meeting: &Meeting) -> Result<PathBuf, CoreError> {
    let base = app.path().app_data_dir().map_err(|error| {
        CoreError::new(
            ErrorCode::Io,
            format!("Nao foi possivel localizar o diretorio de dados: {error}"),
            false,
        )
    })?;
    let candidate = base.join(&meeting.audio_dir);
    // A guarda existe mesmo com o path sendo derivado: se um dia `audio_dir`
    // passar a vir de outro lugar, a escapatoria falha aqui e nao no filesystem.
    if !candidate.starts_with(&base) {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Caminho de audio fora do diretorio de dados.",
            false,
        ));
    }
    Ok(candidate)
}

#[tauri::command]
pub fn meeting_start(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    recorder: tauri::State<'_, RecordingState>,
    title: &str,
    project_id: Option<&str>,
) -> Result<Meeting, CoreError> {
    let mut active = recorder.active.lock().map_err(lock_error)?;
    if active.is_some() {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "Ja existe uma gravacao em curso.",
            false,
        ));
    }

    // O dominio cria a linha PRIMEIRO. Se a captura falhar, existe uma reuniao
    // em `recording` que a proxima abertura recupera — e "uma reuniao que nao
    // gravou nada" e um fato visivel, enquanto uma captura sem linha no banco
    // seria audio que ninguem encontraria.
    let meeting = state.meetings.start(title, project_id)?;
    let root = audio_root(&app, &meeting)?;

    // O instante vem de fora: `mos-audio` nao tem relogio, e nao deve ter. E a
    // mesma fronteira que mantem o crate sem `mos-core`.
    let started_at = meeting
        .started_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    match Recording::start(&root, &started_at) {
        Ok(recording) => {
            *active = Some(Active {
                meeting_id: meeting.id.to_string(),
                recording,
            });
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

#[tauri::command]
pub fn meeting_stop(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    recorder: tauri::State<'_, RecordingState>,
) -> Result<Meeting, CoreError> {
    let active = recorder
        .active
        .lock()
        .map_err(lock_error)?
        .take()
        .ok_or_else(|| {
            CoreError::new(
                ErrorCode::InvalidTransition,
                "Nao ha gravacao em curso.",
                false,
            )
        })?;

    // `Stopping` antes de fechar os arquivos: entre o clique e o fechamento
    // ainda entra audio, e a reuniao dura ate o ultimo frame gravado.
    state.meetings.stop(&active.meeting_id)?;
    let outcome = active.recording.stop().map_err(audio_error)?;

    let settled = state.meetings.settle_audio(
        &active.meeting_id,
        AudioOutcome {
            duration_ms: outcome.duration_ms,
            mic: to_domain(outcome.mic),
            system: to_domain(outcome.system),
        },
    );
    update_tray(&app, None);
    settled
}

/// Suspende a gravacao em curso.
///
/// A ORDEM importa, e ela e diferente nos dois sentidos.
///
/// PAUSAR: o audio para PRIMEIRO, o estado muda depois. Se fosse ao contrario, o
/// intervalo entre as duas linhas gravaria frames numa reuniao que a tela ja
/// mostra como pausada.
///
/// RETOMAR: o estado muda primeiro, o audio volta depois — pelo mesmo motivo
/// invertido. Frame gravado antes de a tela dizer "gravando" e a mesma mentira,
/// so que pior, porque ninguem esta olhando.
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
    let frame = tick(atual);
    drop(active);

    let pausada = state.meetings.pause(&id)?;
    // Tick imediato: sem ele a barra levaria ate um segundo para dizer PAUSADO,
    // e um segundo de ponto vermelho pulsando depois do clique e exatamente a
    // mentira que a §17.2 proibe.
    let _ = app.emit("meeting-tick", &frame);
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
    Ok(retomada)
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
    }
}

#[tauri::command]
pub fn meeting_list(
    state: tauri::State<'_, AppState>,
    include_archived: bool,
) -> Result<Vec<Meeting>, CoreError> {
    state.meetings.meetings(include_archived)
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

/// Grava as anotacoes. Chamado com debounce pela tela — nao ha botao de salvar,
/// porque um botao de salvar numa nota de reuniao e uma chance de perder o que
/// se escreveu.
#[tauri::command]
pub fn meeting_set_notes(
    state: tauri::State<'_, AppState>,
    id: &str,
    notes: &str,
) -> Result<Meeting, CoreError> {
    state.meetings.set_notes(id, notes)
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

/// O usuario escolheu [Processar] na tela de recuperacao.
#[tauri::command]
pub fn meeting_process_recovered(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Meeting, CoreError> {
    state.meetings.process_recovered(id)
}

/// O usuario escolheu [Descartar]. Apaga o audio DEPOIS de mudar o estado.
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

/// A reconciliacao de abertura.
///
/// Uma reuniao em captura num processo recem-nascido significa, necessariamente,
/// que o anterior morreu sem terminar. **Nada e apagado**: ela vira `interrupted`
/// com a duracao que o disco sustenta, e quem decide entre processar e descartar
/// e a pessoa (§9.2).
pub fn reconcile_on_open(app: &AppHandle) -> Result<Vec<Meeting>, CoreError> {
    let state = app.state::<AppState>();
    let handle = app.clone();
    state.meetings.reconcile_on_open(&move |meeting| {
        audio_root(&handle, meeting)
            .ok()
            .and_then(|root| mos_audio::recover_session(&root).ok())
            .map(|recovered| recovered.duration_ms)
            .unwrap_or(0)
    })
}

/// Apaga o audio que a politica de retencao ja liberou.
///
/// A ORDEM importa: apaga primeiro, marca depois. Marcar antes deixaria uma
/// reuniao dizendo que o audio sumiu com o audio ainda no disco, ocupando espaco
/// para sempre porque ninguem mais o procuraria.
pub fn clean_expired_audio(app: &AppHandle) -> Result<usize, CoreError> {
    let state = app.state::<AppState>();
    let mut cleaned = 0usize;
    for meeting in state.meetings.audio_to_clean()? {
        let root = audio_root(app, &meeting)?;
        if mos_audio::delete_session_audio(&root).is_ok() {
            state.meetings.mark_audio_deleted(&meeting.id.to_string())?;
            cleaned += 1;
        }
    }
    Ok(cleaned)
}

/// O nivel, quinze vezes por segundo.
///
/// Laco SEPARADO do `run` porque as duas coisas mudam em ritmos diferentes:
/// estado, duracao e saude dos canais mudam uma vez por segundo, e mandar o
/// `MeetingTick` inteiro a 15 Hz seria repetir quinze vezes por segundo um
/// objeto que mudou zero.
///
/// Aqui vao DOIS numeros, e nada mais. Nunca PCM: o RMS ja sai reduzido a
/// `0..1000` dentro da thread de captura, e e so isso que atravessa.
pub async fn run_levels(app: AppHandle) {
    loop {
        // 66 ms: quinze quadros por segundo. Acima disso a onda deixa de mostrar
        // a cadencia da fala; abaixo, o custo cresce sem o olho ganhar nada.
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

/// O nivel cru, a 15 Hz. Dois numeros e nada mais.
#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingLevel {
    pub mic: u64,
    pub system: u64,
}

/// O laco que alimenta a interface enquanto grava.
///
/// Uma emissao por segundo, e SO enquanto existe gravacao. Ele tambem e quem
/// percebe que os dois canais morreram: uma gravacao que perdeu os dois nao
/// continua "gravando" na tela ate alguem clicar em Parar — **nunca fingir que
/// continua gravando** (§20).
pub async fn run(app: AppHandle) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let recorder = app.state::<RecordingState>();
        let Ok(active) = recorder.active.lock() else {
            continue;
        };
        let Some(current) = active.as_ref() else {
            continue;
        };
        let frame = tick(current);
        let both_gone = !frame.mic.has_audio() && !frame.system.has_audio();
        drop(active);

        let _ = app.emit("meeting-tick", &frame);
        // O tray e a segunda promessa de "nunca gravar escondido": com a janela
        // fechada, ele e o UNICO lugar onde o fato aparece.
        update_tray(&app, Some(frame.duration_ms));

        if both_gone {
            // Encerra pelo mesmo caminho do clique, para que o estado e os
            // arquivos terminem exatamente como terminariam num Stop normal.
            let state = app.state::<AppState>();
            if let Ok(mut active) = recorder.active.lock() {
                if let Some(current) = active.take() {
                    let _ = state.meetings.stop(&current.meeting_id);
                    if let Ok(outcome) = current.recording.stop() {
                        let _ = state.meetings.settle_audio(
                            &current.meeting_id,
                            AudioOutcome {
                                duration_ms: outcome.duration_ms,
                                mic: to_domain(outcome.mic),
                                system: to_domain(outcome.system),
                            },
                        );
                    }
                }
            }
        }
    }
}

/// Encerra a gravacao quando o processo esta saindo.
///
/// Sem isto, `Quit` com uma gravacao em curso deixaria a reuniao em `recording`
/// e o ultimo chunk sem `sync_all` — a proxima abertura a recuperaria como
/// interrompida, o que e verdade, mas seria uma interrupcao que nos causamos.
pub fn shutdown(app: &AppHandle) {
    let recorder = app.state::<RecordingState>();
    let Ok(mut active) = recorder.active.lock() else {
        return;
    };
    let Some(current) = active.take() else {
        return;
    };
    let state = app.state::<AppState>();
    let _ = state.meetings.stop(&current.meeting_id);
    if let Ok(outcome) = current.recording.stop() {
        let _ = state.meetings.settle_audio(
            &current.meeting_id,
            AudioOutcome {
                duration_ms: outcome.duration_ms,
                mic: to_domain(outcome.mic),
                system: to_domain(outcome.system),
            },
        );
    }
}

/// Quantas reunioes esperam decisao de recuperacao. Alimenta o aviso da abertura.
#[tauri::command]
pub fn meeting_interrupted(state: tauri::State<'_, AppState>) -> Result<Vec<Meeting>, CoreError> {
    Ok(state
        .meetings
        .meetings(false)?
        .into_iter()
        .filter(|meeting| meeting.status == MeetingStatus::Interrupted)
        .collect())
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
pub fn update_tray(app: &AppHandle, duration_ms: Option<i64>) {
    use std::sync::atomic::Ordering;

    let Some(handles) = app.try_state::<crate::TrayHandles>() else {
        return;
    };
    match duration_ms {
        Some(ms) => {
            let total = ms / 1000;
            let label = format!(
                "Meeting Notes · {:02}:{:02}:{:02}",
                total / 3600,
                (total / 60) % 60,
                total % 60
            );
            let _ = handles.clock.set_text(&label);
            // A tooltip tambem muda: quem passa o mouse no icone precisa ver que
            // esta gravando sem abrir o menu.
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

/// Parar pela entrada do tray.
///
/// Ela existe porque a janela pode estar escondida, e e justamente ai que a
/// pessoa precisa parar sem procurar o aplicativo atras do Meet. O caminho e o
/// MESMO do clique na barra — nao ha um segundo jeito de encerrar.
pub fn stop_from_tray(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let recorder = handle.state::<RecordingState>();
        let state = handle.state::<AppState>();
        let active = {
            let Ok(mut guard) = recorder.active.lock() else { return };
            guard.take()
        };
        let Some(active) = active else { return };

        let _ = state.meetings.stop(&active.meeting_id);
        if let Ok(outcome) = active.recording.stop() {
            let _ = state.meetings.settle_audio(
                &active.meeting_id,
                AudioOutcome {
                    duration_ms: outcome.duration_ms,
                    mic: to_domain(outcome.mic),
                    system: to_domain(outcome.system),
                },
            );
        }
        update_tray(&handle, None);
        let _ = handle.emit("data-changed", "meeting");
    });
}

// ============================================================================
// Transcricao
// ============================================================================

/// O que Settings mostra sobre o transcritor local.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriberStatus {
    pub configured: bool,
    pub ready: bool,
    /// A frase que a pessoa le. Vazia quando esta pronto.
    pub problem: String,
    pub name: String,
    pub binary: String,
    pub model: String,
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
        problem: ready.err().map(|error| error.to_string()).unwrap_or_default(),
        name: provider.name(),
        binary: config.binary,
        model: config.model,
        threads: config.threads,
    }
}

#[tauri::command]
pub fn meeting_set_transcriber(
    app: AppHandle,
    binary: &str,
    model: &str,
    threads: u32,
) -> Result<TranscriberStatus, CoreError> {
    {
        let state = app.state::<AppState>();
        crate::set_whisper_config(
            &state.settings_path,
            mos_transcribe::WhisperConfig {
                binary: binary.trim().to_owned(),
                model: model.trim().to_owned(),
                threads,
            },
        )?;
    }
    Ok(meeting_transcriber_status(app))
}

/// Transcreve uma reuniao, em segundo plano.
///
/// Devolve assim que o estado vira `transcribing`. O trabalho continua numa
/// thread propria e o resultado chega por evento — uma reuniao de uma hora leva
/// minutos, e segurar o comando ate o fim congelaria a interface inteira.
#[tauri::command]
pub fn meeting_transcribe(app: AppHandle, id: String) -> Result<Meeting, CoreError> {
    let started = {
        let state = app.state::<AppState>();
        // Falhar ANTES de mudar o estado: uma reuniao que virasse `transcribing`
        // sem transcritor configurado ficaria presa num estagio que nada faz
        // avancar, e o retry tentaria de novo a mesma coisa.
        provider(&app).ready().map_err(transcription_error)?;
        state.meetings.start_transcription(&id)?
    };

    let handle = app.clone();
    std::thread::Builder::new()
        .name("mos-transcribe".into())
        .spawn(move || run_transcription(handle, id))
        .map_err(|error| {
            CoreError::new(
                ErrorCode::Io,
                format!("Nao foi possivel iniciar a transcricao: {error}"),
                true,
            )
        })?;

    Ok(started)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptionProgress {
    meeting_id: String,
    /// `0.0..=1.0`, somando os dois canais.
    progress: f32,
    channel: String,
}

fn run_transcription(app: AppHandle, id: String) {
    let outcome = transcribe_channels(&app, &id);
    let state = app.state::<AppState>();

    match outcome {
        Ok(segments) => {
            // A transcricao entra ANTES da transicao de estado. Se a escrita
            // falhar, a reuniao continua `transcribing` e o retry a encontra; se
            // o estado mudasse primeiro, uma falha deixaria uma reuniao
            // `transcribed` sem transcricao.
            match state.meetings.finish_transcription(&id, segments) {
                Ok(meeting) => {
                    let _ = app.emit("meeting-transcribed", &meeting);
                }
                Err(error) => {
                    let _ = state.meetings.fail(
                        &id,
                        FailedStage::Transcription,
                        &format!("Nao foi possivel gravar a transcricao: {error}"),
                    );
                    let _ = app.emit("meeting-failed", &id);
                }
            }
        }
        Err(problem) => {
            let _ = state
                .meetings
                .fail(&id, FailedStage::Transcription, &problem);
            let _ = app.emit("meeting-failed", &id);
        }
    }
}

/// Transcreve os dois canais e intercala.
///
/// **Um canal por chamada, e nunca os dois juntos.** MIC e o usuario local,
/// SYSTEM sao os remotos, e e essa separacao que sustenta "o que EU prometi"
/// versus "o que outros disseram". Um provider que recebesse os dois poderia
/// misturá-los.
fn transcribe_channels(app: &AppHandle, id: &str) -> Result<Vec<TranscriptSegment>, String> {
    use mos_audio::Channel as AudioChannel;
    use mos_core::{MeetingChannel, TranscriptionRequest};

    let (meeting, root) = {
        let state = app.state::<AppState>();
        let meeting = state.meetings.meeting(id).map_err(|error| error.to_string())?;
        let root = audio_root(app, &meeting).map_err(|error| error.to_string())?;
        (meeting, root)
    };

    if meeting.audio_deleted_at.is_some() {
        return Err("O audio desta reuniao ja foi apagado.".into());
    }

    let provider = provider(app);
    let work = std::env::temp_dir().join(format!("mos-meeting-{}", meeting.id));
    std::fs::create_dir_all(&work).map_err(|error| error.to_string())?;

    let mut por_canal = Vec::new();
    for (index, (audio_channel, domain_channel)) in [
        (AudioChannel::Mic, MeetingChannel::Mic),
        (AudioChannel::System, MeetingChannel::System),
    ]
    .into_iter()
    .enumerate()
    {
        let wav = work.join(format!("{}.wav", audio_channel.folder()));
        let frames = mos_audio::export_channel(&root, audio_channel, &wav)
            .map_err(|error| error.to_string())?;
        if frames == 0 {
            // Canal sem audio nao e falha: um dos dois pode ter caido, e o outro
            // continua sendo a reuniao.
            por_canal.push(Vec::new());
            continue;
        }

        let base = index as f32 * 0.5;
        let segments = provider
            .transcribe(
                TranscriptionRequest {
                    audio: &wav,
                    channel: domain_channel,
                    // Declarado, e nao detectado: as reunioes sao em portugues, e
                    // deixar o modelo adivinhar custa qualidade num audio que
                    // comeca com "alo, ta me ouvindo?".
                    language: Some("pt"),
                },
                &|fraction| {
                    let _ = app.emit(
                        "meeting-transcribing",
                        TranscriptionProgress {
                            meeting_id: meeting.id.to_string(),
                            progress: base + fraction * 0.5,
                            channel: audio_channel.folder().to_owned(),
                        },
                    );
                },
            )
            .map_err(|error| error.to_string())?;
        por_canal.push(segments);
    }

    // O WAV e temporario e grande. Deixa-lo para tras encheria o disco a cada
    // reuniao — e ele e derivado, entao apagar nao perde nada.
    let _ = std::fs::remove_dir_all(&work);

    let system = por_canal.pop().unwrap_or_default();
    let mic = por_canal.pop().unwrap_or_default();
    if mic.is_empty() && system.is_empty() {
        return Err("Nenhum dos dois canais produziu fala transcritivel.".into());
    }

    Ok(mos_core::interleave(meeting.id, mic, system))
}

fn transcription_error(error: mos_core::TranscriptionError) -> CoreError {
    use mos_core::TranscriptionError::*;
    let code = match error {
        NotConfigured | MissingRuntime { .. } => ErrorCode::InvalidTransition,
        NoAudio => ErrorCode::NotFound,
        Failed { .. } | Unreadable { .. } => ErrorCode::Io,
        Cancelled => ErrorCode::InvalidTransition,
    };
    // Falha de execucao e retentavel; falta de configuracao nao e — insistir
    // sem configurar daria o mesmo erro para sempre.
    CoreError::new(code, error.to_string(), matches!(code, ErrorCode::Io))
}

/// Tenta de novo depois de uma falha.
#[tauri::command]
pub fn meeting_retry(app: AppHandle, id: String) -> Result<Meeting, CoreError> {
    let retried = {
        let state = app.state::<AppState>();
        state.meetings.retry(&id)?
    };
    // Voltou para `recorded`: o estagio que falhou foi a transcricao, e ela pode
    // recomecar sozinha. Voltar para `transcribed` significa que a analise
    // falhou, e essa e a Fase 4.
    if retried.status == MeetingStatus::Recorded {
        return meeting_transcribe(app, id);
    }
    Ok(retried)
}

// ============================================================================
// Analise com o Hermes
// ============================================================================

/// O que Settings mostra sobre o consentimento.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisConsent {
    pub granted: bool,
    /// Quando foi dado, em RFC3339. Vazio quando nunca foi.
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

/// Concede ou revoga o consentimento de enviar transcricao ao Hermes.
///
/// **Uma vez, e nao a cada reuniao** (decisao D-A). `UX-PRINCIPLES` §21 e
/// explicito: confirmacoes constantes ensinam a clicar sem ler, e uma tela
/// juridica repetida seria pior que nenhuma porque ninguem a leria na decima
/// vez. Revogar existe e e simetrico: revogado, reunioes param em `transcribed`.
#[tauri::command]
pub fn meeting_set_analysis_consent(
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
    Ok(AnalysisConsent {
        granted: !at.is_empty(),
        granted_at: at,
    })
}

/// O registro do que efetivamente saiu da maquina.
///
/// A ADR-027 exige que a pergunta *"o que exatamente foi para a VPS?"* tenha
/// resposta **depois** do envio, e nao so antes. Ele nao guarda o texto — guarda
/// a medida dele.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SentRecord {
    meeting_id: String,
    segments: usize,
    characters: usize,
    windows: u32,
    first_ms: i64,
    last_ms: i64,
}

#[tauri::command]
pub fn meeting_analyze(app: AppHandle, id: String) -> Result<Meeting, CoreError> {
    let started = {
        let state = app.state::<AppState>();
        if crate::analysis_consent(&state.settings_path).is_empty() {
            // Recusa ANTES de mudar o estado: uma reuniao que virasse
            // `analyzing` sem consentimento ficaria presa num estagio que nada
            // faz avancar.
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "A analise envia a transcricao ao Hermes, e isso ainda nao foi autorizado.",
                false,
            ));
        }
        state.meetings.start_analysis(&id)?
    };

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_analysis(handle, id).await;
    });

    Ok(started)
}

async fn run_analysis(app: AppHandle, id: String) {
    match analyze(&app, &id).await {
        Ok(meeting) => {
            let _ = app.emit("meeting-analyzed", &meeting);
        }
        Err(problem) => {
            let state = app.state::<AppState>();
            let _ = state.meetings.fail(&id, FailedStage::Analysis, &problem);
            let _ = app.emit("meeting-failed", &id);
        }
    }
}

/// Quantas vezes reperguntar quando a resposta nao traz o bloco.
///
/// Uma so. `SPEC-ACOES-ENTRE-APPS.md` §3 fixou que argumento fora do esquema e
/// **recusado, nao corrigido**; insistir mais que isso seria tentar consertar.
const REPROMPTS: usize = 1;

async fn analyze(app: &AppHandle, id: &str) -> Result<Meeting, String> {
    let (meeting, segments, base_url) = {
        let state = app.state::<AppState>();
        let meeting = state.meetings.meeting(id).map_err(|e| e.to_string())?;
        let segments = state.meetings.transcript(id).map_err(|e| e.to_string())?;
        let base_url = crate::hermes::current_base_url(app);
        (meeting, segments, base_url)
    };

    if segments.is_empty() {
        return Err("Esta reuniao nao tem transcricao para analisar.".into());
    }

    let windows = mos_core::build_windows(&segments, mos_core::WINDOW_BUDGET_CHARS);
    // As notas sobem JUNTO, como contexto. Elas nao geram item: o prompt
    // exige `segment` por item, e uma nota nao foi dita, foi escrita.
    let instructions = mos_core::instructions(&meeting.title, &meeting.notes);

    // O REGISTRO do que sai, emitido antes do envio e guardado na reuniao. Ele
    // mede, e nao copia: contagem de segmentos, caracteres, janelas e o
    // intervalo coberto. Nenhum texto de fala atravessa este caminho.
    let characters: usize = windows.iter().map(|window| window.text.len()).sum();
    let _ = app.emit(
        "meeting-sending",
        SentRecord {
            meeting_id: meeting.id.to_string(),
            segments: segments.len(),
            characters,
            windows: windows.len() as u32,
            first_ms: segments.first().map(|s| s.start_ms).unwrap_or(0),
            last_ms: segments.last().map(|s| s.end_ms).unwrap_or(0),
        },
    );

    let outcome = if windows.len() == 1 {
        ask_with_retry(
            &base_url,
            &format!("{instructions}\n\n---\n\n{}", windows[0].text),
            meeting.id,
            &segments,
        )
        .await?
    } else {
        consolidate(&base_url, &windows, &instructions, &meeting, &segments).await?
    };

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
        .finish_analysis(analysis, outcome.insights)
        .map_err(|error| error.to_string())
}

/// Pergunta, e repergunta uma vez se o bloco nao vier.
async fn ask_with_retry(
    base_url: &str,
    prompt: &str,
    meeting_id: mos_core::MeetingId,
    segments: &[TranscriptSegment],
) -> Result<mos_core::AnalysisOutcome, String> {
    let mut ultimo = String::new();
    for tentativa in 0..=REPROMPTS {
        let pedido = if tentativa == 0 {
            prompt.to_owned()
        } else {
            // A repergunta diz o que faltou, e nao repete o pedido inteiro: o
            // erro concreto e a unica informacao nova que temos.
            format!(
                "{prompt}\n\nA resposta anterior nao pode ser lida: {ultimo}\n\
                 Responda APENAS com o bloco cercado ```mos-meeting e o JSON dentro dele."
            )
        };

        let resposta = crate::hermes::ask_once(base_url, &pedido)
            .await
            .map_err(|error| error.to_string())?;

        match mos_core::parse_analysis(meeting_id, &resposta, segments) {
            Ok(outcome) => return Ok(outcome),
            Err(error) => {
                // A resposta crua NAO vai para o log nem para o erro do usuario:
                // ela contem a analise da reuniao, e §16.3 proibe registrar
                // conteudo. So a forma do problema atravessa.
                ultimo = error.to_string();
            }
        }
    }
    Err(ultimo)
}

/// Analisa janela a janela e consolida.
///
/// A passada final recebe **os resultados das janelas**, e nao a transcricao: ela
/// existe para juntar, e mandar tudo de novo estouraria o mesmo orcamento que
/// obrigou a dividir.
async fn consolidate(
    base_url: &str,
    windows: &[mos_core::PromptWindow],
    instructions: &str,
    meeting: &Meeting,
    segments: &[TranscriptSegment],
) -> Result<mos_core::AnalysisOutcome, String> {
    let mut resumos = Vec::new();
    let mut insights = Vec::new();
    let mut topics: Vec<String> = Vec::new();

    for (index, window) in windows.iter().enumerate() {
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
    }

    topics.sort();
    topics.dedup();

    // O resumo final e a unica coisa que o modelo reescreve. Os ITENS vem das
    // janelas e nao passam por aqui — mandar itens ja validados de volta ao
    // modelo daria a ele a chance de trocar um id de evidencia por outro.
    let pedido = format!(
        "Estes sao os resumos parciais de uma reuniao dividida em {} partes.\n\
         Escreva UM resumo unico, curto, em portugues, sem repetir.\n\
         Responda apenas com o bloco cercado ```mos-meeting contendo \
         {{ \"summary\": \"...\" }}.\n\n{}",
        windows.len(),
        resumos.join("\n\n---\n\n")
    );
    let summary = match crate::hermes::ask_once(base_url, &pedido).await {
        Ok(resposta) => mos_core::parse_analysis(meeting.id, &resposta, segments)
            .map(|outcome| outcome.summary)
            // A consolidacao falhando NAO perde a analise: os itens ja estao
            // validados. Junta-se os parciais e segue.
            .unwrap_or_else(|_| resumos.join(" ")),
        Err(_) => resumos.join(" "),
    };

    let rejections = mos_core::Rejections::default();
    Ok(mos_core::AnalysisOutcome {
        summary,
        topics,
        insights,
        rejections,
    })
}

// ============================================================================
// Item de reuniao -> Task / Reminder
// ============================================================================

/// O recibo de uma aceitacao.
///
/// Ele carrega o `UndoStep` porque e assim que o resto do M/OS oferece volta:
/// na janela de cinco segundos do recibo, e nao num botao permanente (ADR-035).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptReceipt {
    pub insight: MeetingInsight,
    pub task_id: String,
    pub reminder_id: Option<String>,
    pub undo: mos_core::UndoStep,
}

#[tauri::command]
pub fn meeting_previews(
    state: tauri::State<'_, AppState>,
    id: &str,
) -> Result<Vec<mos_core::InsightPreview>, CoreError> {
    state.meetings.previews(id)
}

/// Aceita um item, criando Task e — quando pedido — Reminder.
///
/// `remindAt` chega da TELA, resolvido a partir do `dueHint`. O domínio nunca
/// interpreta "amanha": ele recebe um instante que a pessoa viu e pode ter
/// corrigido (`UX-PRINCIPLES` §19).
#[tauri::command]
pub fn meeting_accept_insight(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    insight_id: &str,
    title: &str,
    description: Option<&str>,
    project_id: Option<&str>,
    remind_at: Option<&str>,
) -> Result<AcceptReceipt, CoreError> {
    let remind_at = remind_at
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
                .map_err(|error| {
                    CoreError::new(
                        ErrorCode::InvalidInput,
                        format!("Instante do lembrete invalido: {error}"),
                        false,
                    )
                })
        })
        .transpose()?;

    let accepted = state.meetings.accept_insight(mos_core::AcceptInsight {
        insight_id: mos_core::InsightId::parse(insight_id)?,
        title: title.to_owned(),
        description: description.unwrap_or_default().to_owned(),
        project_id: project_id
            .filter(|value| !value.trim().is_empty())
            .map(mos_core::ProjectId::parse)
            .transpose()?,
        remind_at,
    })?;

    let _ = app.emit("data-changed", "meeting-insight");
    Ok(AcceptReceipt {
        task_id: accepted.task_id.to_string(),
        reminder_id: accepted.reminder_id.map(|id| id.to_string()),
        undo: mos_core::UndoStep::UndoMeetingInsight {
            insight_id: accepted.insight.id.to_string(),
            task_id: accepted.task_id.to_string(),
            reminder_id: accepted.reminder_id.map(|id| id.to_string()),
        },
        insight: accepted.insight,
    })
}

#[tauri::command]
pub fn meeting_dismiss_insight(
    state: tauri::State<'_, AppState>,
    insight_id: &str,
) -> Result<MeetingInsight, CoreError> {
    state.meetings.dismiss_insight(insight_id)
}

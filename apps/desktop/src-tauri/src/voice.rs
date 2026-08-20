//! Voice Inbox: o laco entre o microfone, o transcritor e a Inbox.
//!
//! **Casca fina, como `meeting.rs`.** Toda regra que pode estar errada — o piso
//! de energia, a leitura da data falada, a decisao de agir — vive em
//! `mos_core::voice` e e testada la. `SETUP-MAQUINA.md` §4 registra que
//! `cargo test -p mos-desktop` nao roda na maquina principal, e teste que nao
//! roda nao protege nada.
//!
//! O que sobra aqui e adaptacao e laco: traduzir `AudioError` para `CoreError`,
//! derivar caminhos, mandar a thread de transcricao e emitir evento.
//!
//! # A ordem que importa
//!
//! ```text
//! nota no banco  →  microfone abre  →  fala  →  microfone fecha
//!                                                     │
//!                          piso de energia  ←─────────┘
//!                                │
//!                    recusa ─────┴───── passa
//!                       │                 │
//!            nada persistido        transcreve
//!                                         │
//!                                      Capture
//!                                         │
//!                              audio apagado  +  leitura
//! ```
//!
//! A nota entra no banco ANTES de o microfone abrir, pela mesma razao do
//! `meeting_start`: se a captura falhar, existe uma linha que a proxima
//! abertura reconcilia. Audio sem linha no banco seria audio que ninguem acha.

use std::{
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

use mos_audio::{AudioError, Recording};
use mos_core::{
    heard, is_hallucination, understand, CoreError, ErrorCode, Heard, ProjectHint, TaskState,
    TranscriptionProvider, UndoStep, VoiceAction, VoiceContext, VoiceNote,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};


/// Quanto tempo o recibo fica na tela antes de o HUD sumir.
///
/// Ele nao apaga nada: e so o tempo em que "Desfazer" esta ao alcance da mao.
const RECEIPT_MS: u64 = 6_000;

/// O estado vivo do Voice Inbox neste processo.
///
/// `Option` e nao fila: **uma gravacao por vez**. Dois gravadores disputariam o
/// mesmo microfone, e o dominio ja recusaria a segunda de qualquer jeito.
/// O que a gravacao em curso precisa, e nada alem disso.
///
/// **O contexto da tela e o fuso saem daqui e passam a morar em `surface.rs`.**
/// Estavam neste registro, e o motivo era bom: o atalho global dispara do lado
/// do Rust, e naquele caminho nao ha chamada do renderer para carregar contexto
/// junto — a tela publica o que esta olhando, e o backend guarda.
///
/// O motivo nunca foi exclusivo da voz, porem. O Hermes precisa exatamente das
/// mesmas duas coisas para entender "me lembra disso sexta as 15h": "disso" so
/// a tela sabe preencher, e "sexta as 15h" so o fuso resolve. Duas copias
/// dariam dois lugares para a tela publicar, e um dia para elas divergirem.
#[derive(Default)]
pub struct VoiceRuntime {
    active: Mutex<Option<Active>>,
}

struct Active {
    note_id: String,
    recording: Recording,
}

/// O que o HUD recebe enquanto a fala acontece.
///
/// **Nao existe PCM aqui.** `level` e `peak` sao RMS ja reduzidos a `0..1000`
/// dentro da thread de captura — a mesma fronteira do Meeting Agent.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTick {
    pub note_id: String,
    pub duration_ms: i64,
    pub level: u64,
    pub peak: u64,
    /// Vazio quando o microfone esta bem. A frase do sistema, quando nao esta.
    pub problem: String,
}

/// O desfecho de soltar a tecla.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum VoiceStopped {
    /// Nao havia gravacao. **Nao e erro, e a resposta certa.**
    ///
    /// O atalho global e `Ctrl+Alt+Space`, e ele CONTEM `Alt` — o mesmo `Alt`
    /// que o HUD escuta. Soltar a tecla dispara os dois caminhos de parada, e
    /// eles correm um contra o outro: o Rust para pelo `Released` do atalho, e
    /// o renderer para pelo `keyup`. Quem chega em segundo encontra o
    /// microfone ja fechado.
    ///
    /// Devolver erro ali fazia o HUD anunciar uma falha no exato instante em
    /// que tudo tinha dado certo. Isto e idempotencia, e nao tolerancia a bug:
    /// "pare de gravar" pedido duas vezes tem o mesmo desfecho das duas.
    NotRecording,
    /// Nada foi persistido, e nada precisa ser desfeito.
    TooShort,
    TooQuiet,
    /// O audio esta em disco e a transcricao comecou em outra thread.
    ///
    /// So o id: a nota inteira ja chegou ao HUD por `voice_start` ou pelo
    /// evento `voice-started`, e repeti-la aqui seria mandar 232 bytes de
    /// estado que o outro lado ja tem para dizer "comecou".
    Transcribing { note_id: String },
}

/// O que a leitura produziu, ja com o recibo pronto para a tela.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceResult {
    pub note_id: String,
    pub capture_id: String,
    /// A transcricao INTEIRA. E o que a Capture guarda.
    pub transcript: String,
    /// O titulo que uma Task teria. Existe mesmo quando nenhuma foi criada:
    /// e o que a oferta de confianca media mostra.
    pub title: String,
    pub action: VoiceAction,
    pub confidence: String,
    pub executed: bool,
    pub task_id: Option<String>,
    pub reminder_id: Option<String>,
    pub project_id: Option<String>,
    pub project_name: String,
    pub project_from_context: bool,
    /// O prazo COMO FOI DITO, e o instante para o qual ele foi resolvido.
    pub when_raw: String,
    pub when: Option<String>,
    pub hedged: bool,
    pub undo: Option<UndoStep>,
    /// Quanto tempo o recibo fica ao alcance da mao.
    pub receipt_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceFailed {
    pub note_id: String,
    pub message: String,
    /// Se ha audio em disco esperando um retry.
    pub retryable: bool,
}

// --------------------------------------------------------------- utilitarios

fn audio_error(error: AudioError) -> CoreError {
    let code = match error {
        AudioError::Unsupported | AudioError::AlreadyRecording => ErrorCode::InvalidTransition,
        AudioError::Device(_) => ErrorCode::InvalidInput,
        AudioError::Storage { .. } | AudioError::Misaligned { .. } => ErrorCode::Io,
    };
    CoreError::new(code, error.to_string(), matches!(code, ErrorCode::Io))
}

/// O mesmo para o estado vivo da voz.
fn runtime(app: &AppHandle) -> Result<tauri::State<'_, VoiceRuntime>, CoreError> {
    app.try_state::<VoiceRuntime>().ok_or_else(|| {
        CoreError::new(
            ErrorCode::StorageUnavailable,
            "O M/OS ainda esta abrindo.",
            true,
        )
    })
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> CoreError {
    CoreError::new(
        ErrorCode::StorageUnavailable,
        format!("Estado de voz indisponivel: {error}"),
        false,
    )
}

/// O caminho absoluto do audio de uma nota.
///
/// Derivado de `audio_dir`, que por sua vez e derivado do `VoiceNoteId` —
/// **nenhum path vem do renderer**. A guarda existe mesmo assim: se um dia
/// `audio_dir` passar a vir de outro lugar, a escapatoria falha aqui e nao no
/// filesystem.
fn audio_root(app: &AppHandle, note: &VoiceNote) -> Result<PathBuf, CoreError> {
    let base = app.path().app_data_dir().map_err(|error| {
        CoreError::new(
            ErrorCode::Io,
            format!("Nao foi possivel localizar o diretorio de dados: {error}"),
            false,
        )
    })?;
    let candidate = base.join(&note.audio_dir);
    if !candidate.starts_with(&base) {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Caminho de audio fora do diretorio de dados.",
            false,
        ));
    }
    Ok(candidate)
}

/// O agora de quem falou, no fuso de quem falou.
fn now_local(app: &AppHandle) -> time::OffsetDateTime {
    crate::surface::now_local(app)
}

fn provider(app: &AppHandle) -> Result<mos_transcribe::WhisperCliProvider, CoreError> {
    let state = crate::services(app)?;
    Ok(mos_transcribe::WhisperCliProvider::new(
        crate::whisper_config(&state.settings_path),
    ))
}

// ------------------------------------------------------------------ comandos

/// Abre o microfone.
///
/// Devolve a nota assim que a captura sobe. Nada de transcricao acontece aqui:
/// o HUD precisa aparecer imediatamente, e o transcritor so entra quando a
/// pessoa parar de falar.
#[tauri::command]
pub fn voice_start(app: AppHandle) -> Result<VoiceNote, CoreError> {
    let live = runtime(&app)?;
    let mut active = live.active.lock().map_err(lock_error)?;
    // Ja gravando: devolve a nota que esta em curso, e nao um erro.
    //
    // **O gesto e UM, e as portas para ele sao duas.** O atalho global e
    // `Ctrl+Alt+G`, e ele CONTEM `Alt` — o mesmo `Alt` que o HUD escuta para
    // falar com a janela ja aberta. Afundar a tecla dispara os dois caminhos, e
    // o segundo chegava aqui e pintava "ja existe uma gravacao em curso" por
    // cima do "Ouvindo" — um erro na tela no exato instante em que tudo tinha
    // funcionado. Foi assim que apareceu ao rodar o app.
    //
    // Isto e idempotencia, e nao tolerancia a bug: "comece a gravar" pedido
    // duas vezes no mesmo gesto tem o mesmo desfecho das duas.
    if let Some(em_curso) = active.as_ref() {
        let id = em_curso.note_id.clone();
        drop(active);
        return crate::services(&app)?.voice.note(&id);
    }

    let context = crate::surface::voice_context(&app);
    let state = crate::services(&app)?;
    let note = state.voice.start(
        context.project_id.map(|id| id.to_string()).as_deref(),
        context.task_id.map(|id| id.to_string()).as_deref(),
    )?;

    let root = audio_root(&app, &note)?;
    let started_at = note
        .started_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    match Recording::start_mic(&root, &started_at) {
        Ok(recording) => {
            *active = Some(Active {
                note_id: note.id.to_string(),
                recording,
            });
            drop(active);
            spawn_watchdog(app.clone(), note.id.to_string());
            let _ = app.emit("voice-started", &note);
            Ok(note)
        }
        Err(error) => {
            // O microfone nao abriu. A nota vira `failed` em vez de sumir: ela
            // e o registro de que a tentativa aconteceu, e a frase do sistema
            // ("dispositivo indisponivel") precisa ter onde morar.
            let problema = error.to_string();
            let _ = state.voice.failed(&note.id.to_string(), &problema);
            Err(audio_error(error))
        }
    }
}

/// A rede de segurança do microfone.
///
/// Se o evento de soltar a tecla se perder — troca de janela, sessao bloqueada,
/// plugin engasgado —, o stream ficaria aberto sem ninguem sabendo. Esta thread
/// e a terceira guarda (as outras duas sao `Esc` e a perda de foco), e ela
/// existe porque as duas primeiras dependem da interface estar viva.
fn spawn_watchdog(app: AppHandle, note_id: String) {
    std::thread::Builder::new()
        .name("mos-voice-watchdog".into())
        .spawn(move || {
            // Em passos, e nao num sono unico de dois minutos: a fala tipica
            // dura segundos, e uma thread parada ate o teto por gravacao
            // acumularia dezenas delas numa sessao de trabalho normal. Assim
            // ela sai junto com a gravacao que veio vigiar.
            let passos = mos_core::MAX_DURATION_MS / 500;
            for _ in 0..passos {
                std::thread::sleep(Duration::from_millis(500));
                let ainda_gravando = app
                    .try_state::<VoiceRuntime>()
                    .and_then(|runtime| {
                        runtime.active.lock().ok().map(|guard| {
                            guard
                                .as_ref()
                                .is_some_and(|active| active.note_id == note_id)
                        })
                    })
                    .unwrap_or(false);
                if !ainda_gravando {
                    return;
                }
            }
            let _ = app.emit("voice-capped", ());
            let _ = voice_stop(app.clone());
        })
        .ok();
}

/// Fecha o microfone e decide se aquilo foi fala.
#[tauri::command]
pub fn voice_stop(app: AppHandle) -> Result<VoiceStopped, CoreError> {
    let Some(active) = runtime(&app)?.active.lock().map_err(lock_error)?.take() else {
        return Ok(VoiceStopped::NotRecording);
    };

    let note_id = active.note_id.clone();
    let outcome = active.recording.stop().map_err(audio_error)?;
    let state = crate::services(&app)?;

    // As duas recusas NAO GRAVAM NADA: nem linha, nem arquivo. Uma gravacao
    // recusada aqui e uma gravacao que nunca aconteceu.
    match heard(outcome.duration_ms, outcome.mic_peak) {
        Heard::TooShort => {
            discard(&app, &note_id);
            Ok(VoiceStopped::TooShort)
        }
        Heard::TooQuiet => {
            discard(&app, &note_id);
            Ok(VoiceStopped::TooQuiet)
        }
        Heard::Speech => {
            state
                .voice
                .recorded(&note_id, outcome.duration_ms, outcome.mic_peak)?;
            spawn_transcription(app.clone(), note_id.clone());
            Ok(VoiceStopped::Transcribing { note_id })
        }
    }
}

/// `Esc`: para, joga o audio fora e apaga a linha.
#[tauri::command]
pub fn voice_cancel(app: AppHandle) -> Result<(), CoreError> {
    let taken = runtime(&app)?.active.lock().map_err(lock_error)?.take();
    let Some(active) = taken else {
        return Ok(());
    };
    let note_id = active.note_id.clone();
    // O resultado da parada nao interessa: cancelar descarta de qualquer jeito,
    // e um erro ao medir o que vai ser apagado nao pode impedir o descarte.
    let _ = active.recording.stop();
    discard(&app, &note_id);
    let _ = app.emit("voice-cancelled", ());
    Ok(())
}

/// Apaga tudo o que aquela tentativa produziu.
///
/// Best-effort de proposito: um arquivo que resiste ao apagamento nao pode
/// impedir a linha de sair do banco, e uma linha que resiste nao pode manter os
/// bytes em disco. As duas falhas sao silenciosas porque nenhuma delas tem o
/// que pedir ao usuario — e ambas descrevem uma gravacao que ele ja descartou.
fn discard(app: &AppHandle, note_id: &str) {
    let Ok(state) = crate::services(app) else {
        return;
    };
    if let Ok(note) = state.voice.note(note_id) {
        if let Ok(root) = audio_root(app, &note) {
            let _ = mos_audio::delete_session_audio(&root);
        }
    }
    let _ = state.voice.discard(note_id);
}

/// O que o HUD le a cada quadro. Barato: le atomicos.
#[tauri::command]
pub fn voice_recording(app: AppHandle) -> Result<Option<VoiceTick>, CoreError> {
    let live = runtime(&app)?;
    let active = live.active.lock().map_err(lock_error)?;
    let Some(active) = active.as_ref() else {
        return Ok(None);
    };
    let state = active.recording.state();
    let problem = match &state.mic {
        mos_audio::ChannelState::Unavailable { reason } => reason.clone(),
        mos_audio::ChannelState::Lost { reason, .. } => reason.clone(),
        _ => String::new(),
    };
    Ok(Some(VoiceTick {
        note_id: active.note_id.clone(),
        duration_ms: state.duration_ms,
        level: state.mic_level,
        peak: state.mic_peak,
        problem,
    }))
}

/// As notas que ainda guardam audio que o banco nao tem em texto.
#[tauri::command]
pub fn voice_pending(app: AppHandle) -> Result<Vec<VoiceNote>, CoreError> {
    crate::services(&app)?.voice.unfinished()
}

/// Tenta de novo transcrever uma nota cujo audio continua em disco.
#[tauri::command]
pub fn voice_retry(app: AppHandle, id: String) -> Result<VoiceNote, CoreError> {
    let state = crate::services(&app)?;
    let note = state.voice.note(&id)?;
    if note.audio_deleted_at.is_some() {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "O audio desta nota ja foi apagado.",
            false,
        ));
    }
    // Falhar ANTES de mudar o estado: uma nota que virasse `transcribing` sem
    // transcritor configurado ficaria presa num estagio que nada faz avancar.
    provider(&app)?
        .ready()
        .map_err(|error| CoreError::new(ErrorCode::InvalidTransition, error.to_string(), true))?;
    let started = state.voice.transcribing(&id)?;
    spawn_transcription(app.clone(), id);
    Ok(started)
}

/// Descarta uma nota pendente e o audio dela.
#[tauri::command]
pub fn voice_discard(app: AppHandle, id: String) -> Result<(), CoreError> {
    discard(&app, &id);
    Ok(())
}

/// Executa a acao que a leitura ofereceu, quando a pessoa aceita a oferta.
///
/// **Existe separado de `spawn_transcription` de proposito.** A confianca alta
/// age sozinha e passa por dentro; a media chega ate aqui pela mao de quem
/// apertou ⏎. Sao dois caminhos com a mesma implementacao e autorizacoes
/// diferentes, e junta-los apagaria a diferenca.
#[tauri::command]
pub fn voice_act(app: AppHandle, note_id: String) -> Result<VoiceResult, CoreError> {
    let state = crate::services(&app)?;
    let note = state.voice.note(&note_id)?;
    let capture_id = note.capture_id.ok_or_else(|| {
        CoreError::new(
            ErrorCode::InvalidTransition,
            "Esta nota ainda nao virou Capture.",
            false,
        )
    })?;
    let reading = read_note(&app, &note)?;
    execute(&app, &note, capture_id.to_string(), reading, true)
}

// -------------------------------------------------------------- transcricao

/// Transcreve, entende e — quando a confianca autoriza — age.
///
/// Tudo em thread propria. Um `large-v3-turbo` numa GPU leva segundos para
/// quatro palavras, e segurar o comando ate o fim congelaria a interface no
/// exato momento em que ela precisa dizer "ouvindo".
fn spawn_transcription(app: AppHandle, note_id: String) {
    std::thread::Builder::new()
        .name("mos-voice-transcribe".into())
        .spawn(move || run_transcription(app, note_id))
        .ok();
}

fn run_transcription(app: AppHandle, note_id: String) {
    let Ok(state) = crate::services(&app) else {
        return;
    };
    if state.voice.transcribing(&note_id).is_err() {
        return;
    }
    let _ = app.emit("voice-transcribing", &note_id);

    let nome_do_provider = provider(&app)
        .map(|provider| provider.name())
        .unwrap_or_default();
    match transcribe(&app, &note_id) {
        Ok(transcript) => match state.voice.captured(&note_id, &transcript, &nome_do_provider) {
            Ok((note, capture)) => {
                // O audio sai AGORA, e nao antes: o texto ja esta no banco e
                // indexado, entao os bytes deixaram de carregar informacao que
                // so eles tinham. Guardar mais tempo so aumentaria a superficie
                // de privacidade sem comprar nada.
                delete_audio(&app, &note);

                let reading = match read_note(&app, &note) {
                    Ok(reading) => reading,
                    Err(error) => {
                        // A leitura falhou, e a Capture continua la. E o §19 do
                        // brief acontecendo: intencao desconhecida termina na
                        // Inbox, e isso e o comportamento correto.
                        let _ = app.emit("voice-failed", VoiceFailed {
                            note_id: note_id.clone(),
                            message: error.message,
                            retryable: false,
                        });
                        return;
                    }
                };
                let executar = reading.understanding.should_execute();
                match execute(&app, &note, capture.id.to_string(), reading, executar) {
                    Ok(resultado) => {
                        let _ = app.emit("voice-captured", &resultado);
                        let _ = app.emit_to("main", "capture-changed", capture.id.to_string());
                        let _ = app.emit_to("main", "data-changed", "voice");
                    }
                    Err(error) => {
                        let _ = app.emit(
                            "voice-failed",
                            VoiceFailed {
                                note_id: note_id.clone(),
                                message: error.message,
                                retryable: false,
                            },
                        );
                    }
                }
            }
            Err(error) => fail(&app, &note_id, &error.message),
        },
        Err(problema) => fail(&app, &note_id, &problema),
    }
}

fn fail(app: &AppHandle, note_id: &str, message: &str) {
    let Ok(state) = crate::services(app) else {
        return;
    };
    let retryable = state
        .voice
        .failed(note_id, message)
        .map(|note| note.audio_deleted_at.is_none())
        .unwrap_or(false);
    let _ = app.emit(
        "voice-failed",
        VoiceFailed {
            note_id: note_id.to_owned(),
            message: message.to_owned(),
            retryable,
        },
    );
}

/// Exporta o canal do microfone e manda ao provider.
///
/// **Um canal, e nao dois.** A gravacao de voz nunca abriu o loopback, entao
/// nao ha o que intercalar — e a ausencia do segundo canal e a diferenca entre
/// ditar um lembrete e gravar o ambiente.
fn transcribe(app: &AppHandle, note_id: &str) -> Result<String, String> {
    use mos_core::{MeetingChannel, TranscriptionRequest};

    let (note, root) = {
        let state = crate::services(app).map_err(|error| error.message)?;
        let note = state.voice.note(note_id).map_err(|error| error.message)?;
        let root = audio_root(app, &note).map_err(|error| error.message)?;
        (note, root)
    };

    let provider = provider(app).map_err(|error| error.message)?;
    provider.ready().map_err(|error| error.to_string())?;

    let work = std::env::temp_dir().join(format!("mos-voice-{}", note.id));
    std::fs::create_dir_all(&work).map_err(|error| error.to_string())?;
    let wav = work.join("mic.wav");
    let frames = mos_audio::export_channel(&root, mos_audio::Channel::Mic, &wav)
        .map_err(|error| error.to_string())?;
    if frames == 0 {
        let _ = std::fs::remove_dir_all(&work);
        return Err("A gravacao nao produziu audio.".into());
    }

    let segments = provider
        .transcribe(
            TranscriptionRequest {
                audio: &wav,
                channel: MeetingChannel::Mic,
                // Declarado, e nao detectado. Quatro palavras nao dao ao modelo
                // material para adivinhar o idioma, e adivinhar errado devolve
                // uma frase em espanhol que parece uma transcricao ruim.
                language: Some("pt"),
            },
            &|_| {},
        )
        .map_err(|error| error.to_string())?;
    // O WAV e derivado e grande. Apagar nao perde nada, e deixa-lo encheria o
    // disco a cada frase dita.
    let _ = std::fs::remove_dir_all(&work);

    let transcript = segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if transcript.trim().is_empty() {
        return Err("Nao consegui entender o audio.".into());
    }
    // A terceira guarda, e a que so pode existir DEPOIS do modelo: o
    // preenchimento de credito de legenda chega bem pontuado e indistinguivel
    // de fala real para qualquer verificacao de forma.
    if is_hallucination(&transcript) {
        return Err("Nao consegui entender o audio.".into());
    }
    Ok(transcript)
}

// ---------------------------------------------------------------- entendimento

struct Reading {
    understanding: mos_core::Understanding,
    project_name: String,
}

fn read_note(app: &AppHandle, note: &VoiceNote) -> Result<Reading, CoreError> {
    let state = crate::services(app)?;
    let projects = state.work.projects(false)?;
    let hints: Vec<ProjectHint> = projects
        .iter()
        .map(|project| ProjectHint {
            id: project.id,
            name: project.name.clone(),
        })
        .collect();

    let understanding = understand(
        &note.transcript,
        now_local(app),
        VoiceContext {
            project_id: note.context_project_id,
            task_id: note.context_task_id,
        },
        &hints,
    );
    let project_name = understanding
        .project_id
        .and_then(|id| projects.iter().find(|project| project.id == id))
        .map(|project| project.name.clone())
        .unwrap_or_default();

    Ok(Reading {
        understanding,
        project_name,
    })
}

/// Cria (ou nao) a Task e o Reminder, e monta o recibo.
fn execute(
    app: &AppHandle,
    note: &VoiceNote,
    capture_id: String,
    reading: Reading,
    autorizado: bool,
) -> Result<VoiceResult, CoreError> {
    let state = crate::services(app)?;
    let understanding = reading.understanding;
    let quando = understanding
        .when
        .map(|instant| {
            instant
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        })
        .filter(|value| !value.is_empty());

    let mut resultado = VoiceResult {
        note_id: note.id.to_string(),
        capture_id: capture_id.clone(),
        transcript: note.transcript.clone(),
        title: understanding.title.clone(),
        action: understanding.action,
        confidence: understanding.confidence.as_str().to_owned(),
        executed: false,
        task_id: None,
        reminder_id: None,
        project_id: understanding.project_id.map(|id| id.to_string()),
        project_name: reading.project_name,
        project_from_context: understanding.project_source == mos_core::ProjectSource::Context,
        when_raw: understanding.when_raw.clone(),
        when: quando,
        hedged: understanding.hedged,
        undo: None,
        receipt_ms: RECEIPT_MS,
    };

    if !autorizado || understanding.action == VoiceAction::Keep {
        return Ok(resultado);
    }

    let reminder = match (understanding.action, understanding.when) {
        (VoiceAction::CreateTaskWithReminder, Some(instant)) => Some(
            state
                .attention
                .draft_at(&understanding.title, "", instant, mos_core::ReminderSource::Capture)?,
        ),
        _ => None,
    };

    let (task, reminder) = state.work.create_task_from_capture_with_reminder(
        &capture_id,
        &understanding.title,
        // A DESCRICAO fica vazia, e a ausencia e deliberada: a fala inteira ja
        // esta na Capture, que e a origem da Task. Copia-la aqui criaria uma
        // segunda copia do mesmo texto, e as duas divergiriam na primeira
        // edicao.
        "",
        understanding.project_id,
        reminder,
    )?;

    resultado.executed = true;
    resultado.task_id = Some(task.id.to_string());
    resultado.reminder_id = reminder.as_ref().map(|reminder| reminder.id.to_string());
    resultado.undo = Some(UndoStep::UndoVoiceAction {
        capture_id,
        task_id: task.id.to_string(),
        reminder_id: resultado.reminder_id.clone(),
    });
    // Uma Task criada por voz nasce em BACKLOG, como qualquer outra. A ausencia
    // de tratamento especial e a decisao: voz e uma forma de digitar, e nao um
    // canal com privilegios proprios.
    debug_assert_eq!(task.state, TaskState::Backlog);
    Ok(resultado)
}

// ------------------------------------------------------------------- atalho

/// O atalho global foi afundado.
///
/// O auto-repeat do Windows dispara isto varias vezes enquanto a tecla esta
/// pressionada. A guarda e o proprio estado: ja gravando, nao acontece nada.
pub fn shortcut_pressed(app: &AppHandle) {
    // `try_state` e nao `state`: o atalho dispara do lado do Rust, sem passar
    // por tela nenhuma, e pode chegar antes de o `setup` terminar. `state()`
    // ali seria panico no `main`, que e o app inteiro perdido.
    let ja_gravando = app
        .try_state::<VoiceRuntime>()
        .and_then(|runtime| runtime.active.lock().ok().map(|guard| guard.is_some()))
        .unwrap_or(false);
    if ja_gravando {
        return;
    }
    // O HUD aparece PRIMEIRO. Ele e a unica indicacao de que o microfone
    // abriu, e ele nao pode chegar depois do microfone.
    crate::reveal_window(app, "quick-capture");
    let _ = app.emit_to("quick-capture", "voice-armed", ());
    if let Err(error) = voice_start(app.clone()) {
        let _ = app.emit_to("quick-capture", "voice-refused", error.message);
    }
}

/// A tecla foi solta.
pub fn shortcut_released(app: &AppHandle) {
    let gravando = app
        .try_state::<VoiceRuntime>()
        .and_then(|runtime| runtime.active.lock().ok().map(|guard| guard.is_some()))
        .unwrap_or(false);
    if !gravando {
        return;
    }
    match voice_stop(app.clone()) {
        Ok(stopped) => {
            let _ = app.emit_to("quick-capture", "voice-stopped", stopped);
        }
        Err(error) => {
            let _ = app.emit_to("quick-capture", "voice-refused", error.message);
        }
    }
}

// -------------------------------------------------------------- reconciliacao

/// O que o processo anterior deixou pelo caminho.
///
/// Roda na abertura. Uma nota em `recording` num processo recem-nascido
/// significa, necessariamente, que o anterior morreu sem terminar — e o audio
/// dela pode estar inteiro em disco. Ela vira `failed`, que e o estado em que o
/// retry a encontra.
pub fn reconcile(app: &AppHandle) {
    let Ok(state) = crate::services(app) else {
        return;
    };
    let Ok(pendentes) = state.voice.unfinished() else {
        return;
    };
    for note in pendentes {
        let id = note.id.to_string();
        match note.status {
            mos_core::VoiceNoteStatus::Recording | mos_core::VoiceNoteStatus::Transcribing => {
                // Sem audio em disco, a nota nao guarda nada e sai. COM audio,
                // ela fica: e a diferenca entre limpar e perder.
                let tem_audio = audio_root(app, &note)
                    .map(|root| mos_audio::recover_session(&root).map(|s| s.has_audio()).unwrap_or(false))
                    .unwrap_or(false);
                if tem_audio {
                    let _ = state
                        .voice
                        .failed(&id, "O M/OS fechou no meio desta gravacao.");
                } else {
                    discard(app, &id);
                }
            }
            _ => {}
        }
    }
}

/// Apaga os bytes e registra que apagou.
///
/// A ordem e deliberada: **marcar primeiro**. Se os bytes sumissem e a marca
/// nao, a nota diria que o audio existe e o retry falharia dizendo que o
/// arquivo sumiu — um beco sem saida. Marcada antes, o pior caso e uma marca
/// para um arquivo que continua em disco, que a proxima limpeza resolve.
fn delete_audio(app: &AppHandle, note: &VoiceNote) {
    let Ok(state) = crate::services(app) else {
        return;
    };
    if state.voice.mark_audio_deleted(&note.id.to_string()).is_err() {
        return;
    }
    if let Ok(root) = audio_root(app, note) {
        let _ = mos_audio::delete_session_audio(&root);
    }
}

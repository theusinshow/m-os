//! Captura de audio do Meeting Agent.
//!
//! Este crate **nao depende de `mos-core` nem de `mos-storage-sqlite`**, e a
//! ausencia e a decisao (`MEETING-AGENT.md` §4.2). Ele produz bytes e FATOS —
//! frames gravados, canal perdido em t, dispositivo indisponivel. O que isso
//! significa para uma Meeting e decidido no dominio, e a traducao acontece no
//! crate do desktop, que e o unico lugar onde adapter e dominio se encontram.
//!
//! Fora do Windows ele compila e recusa: `AudioError::Unsupported`. Devolver
//! silencio seria pior que recusar, porque uma gravacao vazia se parece com uma
//! gravacao que funcionou.

mod chunks;
mod session;
mod wav;

#[cfg(windows)]
mod capture;

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};

pub use chunks::{recover, Format, Recovered, CHUNK_MS};
pub use session::{Channel, ChannelInfo, SessionDir, SessionFile, Timing};
pub use wav::{export_channel, export_channel_normalized};

/// O que pode dar errado na captura.
///
/// Erro proprio, e nao `CoreError`: e o que mantem este crate sem `mos-core`.
/// Mesmo padrao do `mos-hermes`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AudioError {
    /// O sistema operacional nao oferece esta capacidade.
    Unsupported,
    /// Ja existe uma gravacao em curso neste processo.
    AlreadyRecording,
    Device(String),
    Storage {
        path: String,
        detail: String,
    },
    /// Bytes que nao completam um frame. Meio frame desalinha tudo o que vier
    /// depois, sem sintoma ate alguem ouvir ruido branco.
    Misaligned {
        bytes: usize,
        frame: usize,
    },
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => {
                write!(
                    f,
                    "A captura de audio existe apenas no Windows nesta versao."
                )
            }
            Self::AlreadyRecording => write!(f, "Ja existe uma gravacao em curso."),
            Self::Device(detail) => write!(f, "Dispositivo de audio indisponivel: {detail}"),
            Self::Storage { path, detail } => write!(f, "Falha ao gravar em {path}: {detail}"),
            Self::Misaligned { bytes, frame } => {
                write!(
                    f,
                    "Escrita de {bytes} bytes nao e multipla do frame de {frame}."
                )
            }
        }
    }
}

impl std::error::Error for AudioError {}

/// O destino de um canal.
///
/// Tres variantes, e nao um booleano, porque "nunca abriu" e "abriu e caiu aos
/// 32:10" pedem frases diferentes na tela e preservam quantidades diferentes de
/// audio.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum ChannelState {
    Capturing,
    Captured,
    Unavailable { reason: String },
    Lost { at_ms: i64, reason: String },
}

impl ChannelState {
    pub fn has_audio(&self) -> bool {
        matches!(self, Self::Capturing | Self::Captured | Self::Lost { .. })
    }
}

/// O que a interface ve, uma vez por segundo.
///
/// **Nao existe PCM aqui.** `level` e um RMS ja reduzido a `0..1000` dentro da
/// thread de captura.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingState {
    pub duration_ms: i64,
    pub mic: ChannelState,
    pub system: ChannelState,
    pub mic_level: u64,
    pub system_level: u64,
    /// O maior RMS visto desde o inicio, por canal.
    ///
    /// Existe para o Voice Inbox: o instantaneo responde "esta entrando som
    /// agora?", e uma gravacao de tres segundos lida por amostragem pode cair
    /// inteira nas pausas entre palavras. O pico e o que separa "falei baixo"
    /// de "nao falei".
    #[serde(default)]
    pub mic_peak: u64,
    #[serde(default)]
    pub system_peak: u64,
}

/// O que a captura mediu quando os arquivos fecharam.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOutcome {
    /// Medida em FRAMES GRAVADOS, nunca por diferenca de relogio. Se um canal
    /// perdeu quatro segundos, a duracao precisa refletir o que existe.
    pub duration_ms: i64,
    pub mic: ChannelState,
    pub system: ChannelState,
    /// O pico de cada canal ao longo da gravacao inteira.
    #[serde(default)]
    pub mic_peak: u64,
    #[serde(default)]
    pub system_peak: u64,
}

/// O que a recuperacao de abertura encontrou em disco.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveredSession {
    pub duration_ms: i64,
    pub mic: Recovered,
    pub system: Recovered,
}

impl RecoveredSession {
    pub fn has_audio(&self) -> bool {
        self.mic.frames > 0 || self.system.frames > 0
    }
}

/// Mede o que existe em disco para uma sessao, sem apagar nada.
///
/// A duracao e o MAIOR dos dois canais. Nao a media nem o menor: um canal que
/// caiu aos 32 minutos numa reuniao de 78 nao encurta a reuniao — ele encurta
/// aquele canal, e o outro continua sendo o que aconteceu.
pub fn recover_session(root: &Path) -> Result<RecoveredSession, AudioError> {
    let session = SessionDir::new(root);
    let format = session
        .read_manifest()?
        .map(|manifest| manifest.format)
        .unwrap_or(Format::CAPTURE);

    let mic = recover(&session.channel(Channel::Mic), format)?;
    let system = recover(&session.channel(Channel::System), format)?;
    Ok(RecoveredSession {
        duration_ms: mic.duration_ms(format).max(system.duration_ms(format)),
        mic,
        system,
    })
}

/// Apaga o audio de uma sessao.
///
/// **Nada neste crate chama isto sozinho.** Nao existe rotina de limpeza
/// automatica de "arquivos orfaos" — ela e exatamente o comportamento que
/// transformaria 1h18 de reuniao em zero sem ninguem perceber (§9.2).
pub fn delete_session_audio(root: &Path) -> Result<(), AudioError> {
    SessionDir::new(root).delete_audio()
}

/// Uma gravacao em curso.
///
/// Enquanto ele existir, tres threads estao rodando: microfone, audio do sistema
/// e o keep-alive de silencio. `stop` as encerra e devolve o que foi medido.
pub struct Recording {
    root: PathBuf,
    stop: Arc<AtomicBool>,
    /// Compartilhado com os DOIS canais e com o keep-alive.
    ///
    /// Um atomico so, e nao um por canal: pausar apenas o MIC deixaria o SYSTEM
    /// acumulando frames que o outro nao tem, e a linha do tempo torceria — a
    /// mesma falha de 4710 ms que o spike mediu, chegando pelo outro lado.
    paused: Arc<AtomicBool>,
    #[cfg(windows)]
    mic: Option<capture::ChannelThread>,
    #[cfg(windows)]
    system: Option<capture::ChannelThread>,
    #[cfg(windows)]
    keep_alive: Option<std::thread::JoinHandle<bool>>,
}

impl Recording {
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Suspende ou retoma a escrita nos dois canais.
    ///
    /// Nao fecha o stream do WASAPI: reabrir ao retomar poderia devolver outro
    /// formato efetivo, que e uma troca silenciosa. Pausado, o pacote e lido e
    /// descartado.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

#[cfg(windows)]
impl Recording {
    /// Comeca a gravar os DOIS canais em `root`. E a gravacao de reuniao.
    ///
    /// O keep-alive sobe ANTES dos canais: se o motor de audio so acordar depois
    /// que o loopback ja esta lendo, os primeiros segundos do canal SYSTEM saem
    /// vazios e a linha do tempo nasce torta.
    pub fn start(root: &Path, started_at: &str) -> Result<Self, AudioError> {
        Self::start_with(root, started_at, true)
    }

    /// Grava SO o microfone. E a gravacao do Voice Inbox.
    ///
    /// Tres ausencias, e as tres sao a decisao:
    ///
    /// - **sem loopback.** Capturar o que sai pelos alto-falantes enquanto
    ///   alguem dita um lembrete gravaria a musica, a reuniao aberta atras e a
    ///   voz de quem estivesse do outro lado dela. Numa reuniao isso e o
    ///   produto; aqui seria vigilancia acidental;
    /// - **sem keep-alive.** Ele existe para o loopback nao emudecer, e sem
    ///   canal SYSTEM ele so manteria o motor de audio do Windows acordado sem
    ///   ninguem ouvindo;
    /// - **sem o segundo arquivo.** `recover_session` continua funcionando: o
    ///   canal ausente le como zero frames, que e a verdade.
    pub fn start_mic(root: &Path, started_at: &str) -> Result<Self, AudioError> {
        Self::start_with(root, started_at, false)
    }

    fn start_with(root: &Path, started_at: &str, with_system: bool) -> Result<Self, AudioError> {
        let session = SessionDir::new(root);
        session.create()?;

        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        // O keep-alive so existe para o loopback nao emudecer. Sem canal SYSTEM
        // ele manteria o motor de audio do Windows acordado sem ninguem ouvindo.
        let keep_alive =
            with_system.then(|| capture::spawn_keep_alive(stop.clone(), paused.clone()));

        let mic = capture::spawn(
            Channel::Mic,
            &session.channel(Channel::Mic),
            stop.clone(),
            paused.clone(),
        );
        let system = with_system.then(|| {
            capture::spawn(
                Channel::System,
                &session.channel(Channel::System),
                stop.clone(),
                paused.clone(),
            )
        });

        let (mic_device, system_device) = capture::default_devices().unwrap_or((None, None));
        let system_device = if with_system { system_device } else { None };
        // O manifesto e escrito no inicio, com o que ja se sabe. Ele e reescrito
        // no fim com o que aconteceu — e ate la, se o processo morrer, este
        // arquivo e o que diz a recuperacao como ler os bytes.
        session.write_manifest(&SessionFile {
            version: SessionFile::VERSION,
            started_at: started_at.to_owned(),
            format: Format::CAPTURE,
            chunk_ms: CHUNK_MS,
            mic: mic_device.map(|device| ChannelInfo {
                device,
                opened: false,
                timing: Timing::Events,
                effective_format: Format::CAPTURE,
                keep_alive: false,
            }),
            system: system_device.map(|device| ChannelInfo {
                device,
                opened: false,
                timing: Timing::Events,
                effective_format: Format::CAPTURE,
                keep_alive: true,
            }),
        })?;

        Ok(Self {
            root: root.to_path_buf(),
            stop,
            paused,
            mic: Some(mic),
            system,
            keep_alive,
        })
    }

    /// O estado atual, para a interface. Barato: le atomicos.
    pub fn state(&self) -> RecordingState {
        let read = |thread: &Option<capture::ChannelThread>| match thread {
            Some(thread) => {
                let live = &thread.live;
                let lost = live.lost_at_ms.load(Ordering::Relaxed);
                let state = if lost >= 0 {
                    ChannelState::Lost {
                        at_ms: lost,
                        reason: "o dispositivo parou de responder".into(),
                    }
                } else if live.opened.load(Ordering::Relaxed) {
                    ChannelState::Capturing
                } else {
                    ChannelState::Unavailable {
                        reason: "o dispositivo ainda nao abriu".into(),
                    }
                };
                (
                    live.frames.load(Ordering::Relaxed),
                    live.level_milli.load(Ordering::Relaxed),
                    live.peak_milli.load(Ordering::Relaxed),
                    state,
                )
            }
            None => (
                0,
                0,
                0,
                ChannelState::Unavailable {
                    reason: String::new(),
                },
            ),
        };

        let (mic_frames, mic_level, mic_peak, mic_state) = read(&self.mic);
        let (system_frames, system_level, system_peak, system_state) = read(&self.system);
        RecordingState {
            duration_ms: Format::CAPTURE.frames_to_ms(mic_frames.max(system_frames)),
            mic: mic_state,
            system: system_state,
            mic_level,
            system_level,
            mic_peak,
            system_peak,
        }
    }

    /// Encerra e devolve o que foi medido.
    ///
    /// A duracao vem do disco, e nao dos contadores em memoria: e a mesma
    /// medicao que a recuperacao faria, e usar as duas fontes garantiria que
    /// elas divergissem um dia.
    pub fn stop(mut self) -> Result<SessionOutcome, AudioError> {
        self.stop.store(true, Ordering::Relaxed);

        // Os picos sao lidos ANTES do join: `ChannelThread::join` consome a
        // thread e leva o `Live` junto, e depois dele nao ha de onde tira-los.
        let peak = |thread: &Option<capture::ChannelThread>| {
            thread
                .as_ref()
                .map(|thread| thread.live.peak_milli.load(Ordering::Relaxed))
                .unwrap_or(0)
        };
        let mic_peak = peak(&self.mic);
        let system_peak = peak(&self.system);

        let mic = self.mic.take().map(capture::ChannelThread::join);
        let system = self.system.take().map(capture::ChannelThread::join);
        if let Some(handle) = self.keep_alive.take() {
            let _ = handle.join();
        }

        let session = SessionDir::new(&self.root);
        if let Ok(Some(mut manifest)) = session.read_manifest() {
            // A thread de captura sabe o dispositivo, o timing e o formato
            // efetivo. Ela NAO sabe do keep-alive — ele roda numa thread
            // propria, e ela nem precisa saber que ele existe.
            //
            // Sem esta preservacao, o merge do fim sobrescrevia `keep_alive:
            // true` por `false`, e o manifesto passava a MENTIR sobre a unica
            // coisa que diz se a linha do tempo do canal remoto e confiavel. O
            // teste de hardware pegou isso.
            manifest.mic = merge(manifest.mic, mic.as_ref().and_then(|r| r.info.clone()));
            manifest.system = merge(
                manifest.system,
                system.as_ref().and_then(|r| r.info.clone()),
            );
            let _ = session.write_manifest(&manifest);
        }

        let recovered = recover_session(&self.root)?;
        Ok(SessionOutcome {
            duration_ms: recovered.duration_ms,
            mic: settle(mic, recovered.mic),
            system: settle(system, recovered.system),
            mic_peak,
            system_peak,
        })
    }
}

#[cfg(not(windows))]
impl Recording {
    pub fn start(_root: &Path, _started_at: &str) -> Result<Self, AudioError> {
        Err(AudioError::Unsupported)
    }

    pub fn start_mic(_root: &Path, _started_at: &str) -> Result<Self, AudioError> {
        Err(AudioError::Unsupported)
    }

    pub fn state(&self) -> RecordingState {
        RecordingState {
            duration_ms: 0,
            mic: ChannelState::Unavailable {
                reason: "sem suporte nesta plataforma".into(),
            },
            system: ChannelState::Unavailable {
                reason: "sem suporte nesta plataforma".into(),
            },
            mic_level: 0,
            system_level: 0,
            mic_peak: 0,
            system_peak: 0,
        }
    }

    pub fn stop(self) -> Result<SessionOutcome, AudioError> {
        Err(AudioError::Unsupported)
    }
}

/// Junta o que a thread descobriu ao que ja se sabia, sem perder nem um nem outro.
///
/// A thread e a autoridade sobre dispositivo, timing e formato efetivo — ela os
/// observou. Quem abriu a gravacao e a autoridade sobre o keep-alive. Um merge
/// que deixasse um lado vencer inteiro apagaria metade da verdade.
#[cfg(windows)]
fn merge(previous: Option<ChannelInfo>, observed: Option<ChannelInfo>) -> Option<ChannelInfo> {
    match (previous, observed) {
        (Some(previous), Some(observed)) => Some(ChannelInfo {
            keep_alive: previous.keep_alive,
            ..observed
        }),
        (previous, observed) => observed.or(previous),
    }
}

/// Concilia o que a thread reportou com o que existe em disco.
///
/// O DISCO manda sobre "houve audio?". Uma thread que reportou `Captured` mas
/// nao deixou frame nenhum nao capturou nada — e chamar isso de capturado seria
/// a mentira mais cara desta feature.
#[cfg(windows)]
fn settle(report: Option<capture::ChannelReport>, recovered: Recovered) -> ChannelState {
    let state = report
        .map(|report| report.state)
        .unwrap_or(ChannelState::Unavailable {
            reason: "o canal nao foi iniciado".into(),
        });
    match state {
        ChannelState::Captured | ChannelState::Capturing if recovered.frames == 0 => {
            ChannelState::Unavailable {
                reason: "o canal abriu mas nao produziu audio".into(),
            }
        }
        ChannelState::Capturing => ChannelState::Captured,
        outro => outro,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duracao_recuperada_e_a_do_canal_mais_longo() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        session.create().unwrap();

        // MIC gravou 2 s, SYSTEM caiu com 1 s. A reuniao durou 2 s.
        let mut mic =
            chunks::ChunkWriter::create(&session.channel(Channel::Mic), Format::CAPTURE, CHUNK_MS)
                .unwrap();
        mic.write(&vec![0u8; 16_000 * 2 * 2]).unwrap();
        mic.finish().unwrap();

        let mut system = chunks::ChunkWriter::create(
            &session.channel(Channel::System),
            Format::CAPTURE,
            CHUNK_MS,
        )
        .unwrap();
        system.write(&vec![0u8; 16_000 * 2]).unwrap();
        system.finish().unwrap();

        let recovered = recover_session(session.path()).unwrap();
        assert_eq!(recovered.duration_ms, 2000);
        assert_eq!(recovered.mic.frames, 32_000);
        assert_eq!(recovered.system.frames, 16_000);
        assert!(recovered.has_audio());
    }

    #[test]
    fn sessao_sem_nada_em_disco_e_zero_e_nao_erro() {
        let dir = tempfile::tempdir().unwrap();
        let recovered = recover_session(&dir.path().join("nunca-gravou")).unwrap();
        assert_eq!(recovered.duration_ms, 0);
        assert!(!recovered.has_audio());
    }

    #[test]
    fn a_recuperacao_usa_o_formato_do_manifesto() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        // Um formato diferente do padrao: 8 kHz mono i16.
        let format = Format {
            sample_rate: 8_000,
            channels: 1,
            bytes_per_sample: 2,
        };
        session
            .write_manifest(&SessionFile {
                version: SessionFile::VERSION,
                started_at: String::new(),
                format,
                chunk_ms: CHUNK_MS,
                mic: None,
                system: None,
            })
            .unwrap();

        let mut mic =
            chunks::ChunkWriter::create(&session.channel(Channel::Mic), format, CHUNK_MS).unwrap();
        mic.write(&vec![0u8; 8_000 * 2]).unwrap();
        mic.finish().unwrap();

        // Um segundo a 8 kHz. Se a recuperacao usasse 16 kHz por padrao, ela
        // reportaria meio segundo — e a evidencia apontaria para o lugar errado.
        assert_eq!(recover_session(session.path()).unwrap().duration_ms, 1000);
    }

    #[test]
    fn apagar_o_audio_nao_reclama_de_sessao_que_nao_existe() {
        let dir = tempfile::tempdir().unwrap();
        delete_session_audio(&dir.path().join("nunca-existiu")).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn fora_do_windows_a_gravacao_recusa_em_vez_de_devolver_silencio() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            Recording::start_mic(dir.path(), ""),
            Err(AudioError::Unsupported)
        ));
    }

    /// A invariante da pausa, testada onde ela realmente mora.
    ///
    /// Nao ha como instanciar `Recording` sem WASAPI, entao testar
    /// `set_paused`/`is_paused` exigiria hardware — e um teste de
    /// `if pausado { 0 } else { frames }` seria tautologico, provando so que o
    /// `if` foi digitado.
    ///
    /// O que IMPORTA e outra coisa, e ela e verificavel aqui: a duracao sai dos
    /// frames em disco. Se a pausa nao escreve, o tempo pausado nao existe para
    /// quem le depois — sem nenhum campo novo, sem nenhuma subtracao.
    #[test]
    fn o_tempo_pausado_nao_entra_na_duracao_porque_nao_vira_frame() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0199"));
        session.create().unwrap();

        let um_segundo = vec![0u8; 16_000 * 2];

        for canal in Channel::BOTH {
            let mut escritor =
                chunks::ChunkWriter::create(&session.channel(canal), Format::CAPTURE, CHUNK_MS)
                    .unwrap();
            // 1 s gravado, "pausa" de qualquer duracao — que aqui e simplesmente
            // nao escrever nada —, e mais 1 s depois de retomar.
            escritor.write(&um_segundo).unwrap();
            escritor.write(&um_segundo).unwrap();
            escritor.finish().unwrap();
        }

        let recuperada = recover_session(&dir.path().join("0199")).unwrap();
        assert_eq!(
            recuperada.duration_ms, 2_000,
            "a duracao precisa ser o audio gravado, e nao o relogio de parede"
        );
    }
}

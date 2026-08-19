//! A captura WASAPI de producao.
//!
//! Recria os padroes aprovados pelo spike da Fase 1 — **nao e copia dele**. O
//! que a Fase 1 provou, e que este arquivo assume como decidido:
//!
//! - o loopback abre o dispositivo de SAIDA pedindo `Direction::Capture`, o que
//!   o crate traduz em `AUDCLNT_STREAMFLAGS_LOOPBACK` e recusa com exclusive;
//! - `EventsShared` funciona no Windows 11 26200 (11 ms de intervalo maximo) e
//!   custa 2,4x menos CPU que polling;
//! - `autoconvert` funciona junto com loopback, e sem ele o disco cresce 24x;
//! - **o keep-alive de silencio e obrigatorio**: num endpoint ocioso, 25 s de
//!   silencio dao 2.498 pacotes com ele e ZERO sem ele;
//! - `BufferInfo.index` conta em frames do DISPOSITIVO, e nao nos convertidos.

#![cfg(windows)]

use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use wasapi::{
    initialize_mta, AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat,
};

use crate::{
    chunks::{ChunkWriter, Format, CHUNK_MS},
    session::{Channel, ChannelInfo, Timing},
    AudioError, ChannelState,
};

/// O que a thread de captura publica para quem estiver olhando.
///
/// Atomicos, e nao um `Mutex`: a interface le uma vez por segundo e a captura
/// escreve dezenas de vezes por segundo. Um lock aqui poria o fio da gravacao
/// esperando o fio da janela.
#[derive(Default)]
pub struct Live {
    pub frames: AtomicU64,
    /// RMS em milesimos. **E o unico dado de audio que sai desta thread** — nao
    /// existe caminho de PCM para fora do crate.
    pub level_milli: AtomicU64,
    /// O MAIOR RMS visto desde o inicio, na mesma escala de `level_milli`.
    ///
    /// Existe para o Voice Inbox, e nao para a barra de nivel: o instantaneo
    /// responde "esta entrando som agora?", e essa pergunta nao serve para
    /// decidir se HOUVE fala numa gravacao de tres segundos — quem le por
    /// amostragem pode cair inteiro nas pausas entre palavras.
    ///
    /// O pico e o que separa "falei baixo" de "nao falei". O whisper preenche
    /// silencio com credito de legenda inventado, entao essa distincao decide
    /// se o audio chega a ele.
    pub peak_milli: AtomicU64,
    /// `-1` enquanto o canal esta vivo; o instante da perda, em ms, depois.
    pub lost_at_ms: AtomicI64,
    pub opened: AtomicBool,
}

impl Live {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            lost_at_ms: AtomicI64::new(-1),
            ..Default::default()
        })
    }
}

pub struct ChannelThread {
    join: JoinHandle<ChannelReport>,
    pub live: Arc<Live>,
}

impl ChannelThread {
    pub fn join(self) -> ChannelReport {
        self.join.join().unwrap_or(ChannelReport {
            state: ChannelState::Unavailable {
                reason: "a thread de captura terminou de forma inesperada".into(),
            },
            info: None,
        })
    }
}

/// O que a thread reporta quando termina.
///
/// **Nao carrega contagem de frames de proposito.** Quem responde "quanto foi
/// gravado?" e o disco, em `recover_session` — manter a contagem tambem aqui
/// criaria duas fontes para a mesma pergunta, e elas divergiriam no primeiro
/// caso em que a escrita falhasse depois do contador ter subido.
pub struct ChannelReport {
    pub state: ChannelState,
    pub info: Option<ChannelInfo>,
}

pub fn spawn(
    channel: Channel,
    directory: &Path,
    stop: Arc<AtomicBool>,
) -> ChannelThread {
    let live = Live::new();
    let live_for_thread = live.clone();
    let directory = directory.to_path_buf();

    let join = thread::Builder::new()
        .name(format!("mos-audio-{}", channel.folder()))
        .spawn(move || run(channel, &directory, stop, live_for_thread))
        .expect("criar thread de captura");

    ChannelThread { join, live }
}

/// Sobe a prioridade da thread para a classe de audio profissional do Windows.
///
/// Sem isso, uma varredura de disco ou um build em paralelo atrasa a leitura do
/// buffer o suficiente para o dispositivo descartar frames — e o sintoma seria
/// uma descontinuidade que pareceria culpa do WASAPI.
fn raise_priority() {
    use windows_sys::Win32::System::Threading::AvSetMmThreadCharacteristicsW;
    let task: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
    let mut index = 0u32;
    unsafe {
        AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index);
    }
}

fn desired_format() -> WaveFormat {
    WaveFormat::new(
        16,
        16,
        &SampleType::Int,
        Format::CAPTURE.sample_rate as usize,
        Format::CAPTURE.channels as usize,
        None,
    )
}

fn describe(format: &WaveFormat) -> Format {
    Format {
        sample_rate: format.get_samplespersec(),
        channels: format.get_nchannels(),
        bytes_per_sample: format.get_bitspersample() / 8,
    }
}

struct Opened {
    client: AudioClient,
    format: WaveFormat,
    device: String,
    period_ms: u32,
}

// Nota sobre o que NAO esta aqui: o spike contava frames perdidos comparando
// `BufferInfo.index` com o esperado, e mediu 31 frames fixos em 30 s e em 900 s
// — deslocamento unico do resampler, e nao perda. A producao nao carrega esse
// contador porque ele nao alimenta nenhuma decisao: o que a interface precisa
// saber e se o canal CAIU, e isso chega por erro de leitura, nao por aritmetica
// de posicao. Se um dia a deriva precisar ser observada em producao, o caminho
// esta medido em `TECHNICAL-SPIKE-MEETING-AUDIO.md` §5.4.

fn open(channel: Channel) -> Result<Opened, String> {
    let direction = match channel {
        // O audio do sistema vem do dispositivo de RENDERIZACAO. E essa escolha,
        // e nao uma flag que passamos, que liga o loopback.
        Channel::Mic => Direction::Capture,
        Channel::System => Direction::Render,
    };
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let device = enumerator
        .get_default_device(&direction)
        .map_err(|error| format!("nao ha dispositivo padrao: {error}"))?;
    let name = device
        .get_friendlyname()
        .unwrap_or_else(|_| "dispositivo sem nome".into());

    let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
    let (default_period, _) = client.get_device_period().map_err(|e| e.to_string())?;
    // Meio segundo de buffer: grande o bastante para uma pausa de escalonamento
    // nao virar descontinuidade, pequeno o bastante para o Stop continuar
    // imperceptivel.
    const BUFFER_HNS: i64 = 5_000_000;
    let wanted = desired_format();
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: BUFFER_HNS,
    };

    if client
        .initialize_client(&wanted, &Direction::Capture, &mode)
        .is_ok()
    {
        return Ok(Opened {
            client,
            format: wanted,
            device: name,
            period_ms: ((default_period / 10_000).max(1)) as u32,
        });
    }

    // O motor recusou o formato pedido. Um cliente que ja falhou em `Initialize`
    // nao pode ser reinicializado, entao pedimos outro ao dispositivo — insistir
    // no antigo daria `AUDCLNT_E_ALREADY_INITIALIZED`, que pareceria bug nosso.
    let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
    let mix = client.get_mixformat().map_err(|e| e.to_string())?;
    client
        .initialize_client(
            &mix,
            &Direction::Capture,
            &StreamMode::EventsShared {
                autoconvert: false,
                buffer_duration_hns: BUFFER_HNS,
            },
        )
        .map_err(|error| format!("o motor de audio recusou tambem o formato nativo: {error}"))?;

    Ok(Opened {
        client,
        format: mix,
        device: name,
        period_ms: ((default_period / 10_000).max(1)) as u32,
    })
}

fn run(
    channel: Channel,
    directory: &Path,
    stop: Arc<AtomicBool>,
    live: Arc<Live>,
) -> ChannelReport {
    if initialize_mta().ok().is_err() {
        return unavailable("nao foi possivel inicializar o COM nesta thread");
    }
    raise_priority();

    let opened = match open(channel) {
        Ok(opened) => opened,
        // O canal nunca abriu. Isso e `Unavailable` COM MOTIVO, e nao uma
        // gravacao silenciosamente vazia.
        Err(reason) => return unavailable(&reason),
    };

    let format = describe(&opened.format);
    let bytes_per_frame = format.bytes_per_frame();
    let mut writer = match ChunkWriter::create(directory, format, CHUNK_MS) {
        Ok(writer) => writer,
        Err(error) => return unavailable(&format!("{error}")),
    };

    let handle = match opened.client.set_get_eventhandle() {
        Ok(handle) => handle,
        Err(error) => return unavailable(&format!("SetEventHandle falhou: {error}")),
    };
    let capture = match opened.client.get_audiocaptureclient() {
        Ok(capture) => capture,
        Err(error) => return unavailable(&format!("GetService falhou: {error}")),
    };
    if let Err(error) = opened.client.start_stream() {
        return unavailable(&format!("Start falhou: {error}"));
    }
    live.opened.store(true, Ordering::Relaxed);

    let mut info = ChannelInfo {
        device: opened.device.clone(),
        opened: true,
        timing: Timing::Events,
        effective_format: format,
        keep_alive: false,
    };

    let started = Instant::now();
    let event_timeout_ms = (opened.period_ms * 3).max(50);

    let mut scratch = vec![0u8; 256 * 1024];
    let mut consecutive_timeouts = 0u32;
    let mut using_events = true;
    let mut state = ChannelState::Captured;

    'capture: while !stop.load(Ordering::Relaxed) {
        if using_events {
            if handle.wait_for_event(event_timeout_ms).is_err() {
                consecutive_timeouts += 1;
                if consecutive_timeouts >= 3 {
                    // Troca de modo REGISTRADA. Degradar em silencio seria
                    // prometer no manifesto um modo que nao foi usado.
                    info.timing = Timing::EventsThenPolling;
                    using_events = false;
                }
                continue;
            }
            consecutive_timeouts = 0;
        } else {
            thread::sleep(Duration::from_millis((opened.period_ms / 2).max(1) as u64));
        }

        loop {
            let available = match capture.get_next_packet_size() {
                Ok(Some(frames)) if frames > 0 => frames,
                Ok(_) => break,
                Err(error) => {
                    state = lost(started, &format!("a leitura do dispositivo falhou: {error}"));
                    break 'capture;
                }
            };

            let wanted_bytes = available as usize * bytes_per_frame;
            if scratch.len() < wanted_bytes {
                scratch.resize(wanted_bytes, 0);
            }

            let (frames_read, _buffer) = match capture.read_from_device(&mut scratch[..wanted_bytes])
            {
                Ok(result) => result,
                Err(error) => {
                    // O dispositivo sumiu no meio. **Nunca fingir que continua
                    // gravando**: o canal cai aqui, com o instante exato, e o
                    // outro segue sozinho.
                    state = lost(started, &format!("o dispositivo parou de responder: {error}"));
                    break 'capture;
                }
            };
            if frames_read == 0 {
                break;
            }

            let payload = &scratch[..frames_read as usize * bytes_per_frame];
            let level = rms_milli(payload, format);
            live.level_milli.store(level, Ordering::Relaxed);
            live.peak_milli.fetch_max(level, Ordering::Relaxed);

            if let Err(error) = writer.write(payload) {
                state = lost(started, &format!("a escrita em disco falhou: {error}"));
                break 'capture;
            }
            live.frames.store(writer.total_frames(), Ordering::Relaxed);
        }
    }

    let _ = opened.client.stop_stream();
    if let Err(error) = writer.finish() {
        // Falhar ao FECHAR e diferente de falhar ao gravar: o que ja esta em
        // disco continua la, e a recuperacao o encontra.
        state = lost(started, &format!("o fechamento do arquivo falhou: {error}"));
    }

    if let ChannelState::Lost { at_ms, .. } = &state {
        live.lost_at_ms.store(*at_ms, Ordering::Relaxed);
    }

    ChannelReport {
        state,
        info: Some(info),
    }
}

fn unavailable(reason: &str) -> ChannelReport {
    ChannelReport {
        state: ChannelState::Unavailable {
            reason: reason.to_owned(),
        },
        info: None,
    }
}

fn lost(started: Instant, reason: &str) -> ChannelState {
    ChannelState::Lost {
        at_ms: started.elapsed().as_millis() as i64,
        reason: reason.to_owned(),
    }
}

/// RMS reduzido a milesimos.
///
/// E o unico numero que sai do audio para o resto do programa. A interface
/// precisa de "esta me ouvindo?", e nao de amostras.
fn rms_milli(bytes: &[u8], format: Format) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let soma: f64 = match format.bytes_per_sample {
        2 => bytes
            .chunks_exact(2)
            .map(|pair| {
                let sample = i16::from_le_bytes([pair[0], pair[1]]) as f64 / i16::MAX as f64;
                sample * sample
            })
            .sum(),
        4 => bytes
            .chunks_exact(4)
            .map(|quad| {
                let sample = f32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]) as f64;
                sample * sample
            })
            .sum(),
        _ => return 0,
    };
    let n = (bytes.len() / format.bytes_per_sample.max(1) as usize).max(1);
    ((soma / n as f64).sqrt() * 1000.0).min(1000.0) as u64
}

/// Mantem o motor de audio acordado escrevendo silencio na saida.
///
/// **Obrigatorio, e nao uma otimizacao.** O WASAPI so empurra dados para o
/// endpoint de renderizacao quando existe algum stream ativo; quando nada toca,
/// nao ha nada para capturar. Numa reuniao isso acontece o tempo todo, e sem
/// este stream o canal SYSTEM PARA no silencio — desalinhando a linha do tempo
/// contra o microfone e fazendo `14:04` apontar para o lugar errado pelo resto
/// da reuniao.
///
/// Medido na Fase 1, endpoint ocioso, 25 s: 2.498 pacotes com ele, zero sem ele.
pub fn spawn_keep_alive(stop: Arc<AtomicBool>) -> JoinHandle<bool> {
    thread::Builder::new()
        .name("mos-audio-keepalive".into())
        .spawn(move || {
            if initialize_mta().ok().is_err() {
                return false;
            }
            let Ok(enumerator) = DeviceEnumerator::new() else {
                return false;
            };
            let Ok(device) = enumerator.get_default_device(&Direction::Render) else {
                return false;
            };
            let Ok(mut client) = device.get_iaudioclient() else {
                return false;
            };
            let Ok(format) = client.get_mixformat() else {
                return false;
            };
            if client
                .initialize_client(
                    &format,
                    &Direction::Render,
                    &StreamMode::PollingShared {
                        autoconvert: false,
                        buffer_duration_hns: 5_000_000,
                    },
                )
                .is_err()
            {
                return false;
            }
            let Ok(render) = client.get_audiorenderclient() else {
                return false;
            };
            if client.start_stream().is_err() {
                return false;
            }

            let bytes_per_frame = format.get_blockalign() as usize;
            let silence = vec![0u8; 4096 * bytes_per_frame];
            while !stop.load(Ordering::Relaxed) {
                let room = client.get_available_space_in_frames().unwrap_or(0) as usize;
                let frames = room.min(4096);
                if frames > 0 {
                    let _ = render.write_to_device(
                        frames,
                        &silence[..frames * bytes_per_frame],
                        Some(wasapi::BufferFlags {
                            data_discontinuity: false,
                            silent: true,
                            timestamp_error: false,
                        }),
                    );
                }
                thread::sleep(Duration::from_millis(50));
            }
            let _ = client.stop_stream();
            true
        })
        .expect("criar thread de keep-alive")
}

/// O nome do dispositivo padrao de cada direcao, para a interface mostrar.
pub fn default_devices() -> Result<(Option<String>, Option<String>), AudioError> {
    initialize_mta()
        .ok()
        .map_err(|error| AudioError::Device(error.to_string()))?;
    let enumerator =
        DeviceEnumerator::new().map_err(|error| AudioError::Device(error.to_string()))?;
    let name = |direction: Direction| {
        enumerator
            .get_default_device(&direction)
            .ok()
            .and_then(|device| device.get_friendlyname().ok())
    };
    Ok((name(Direction::Capture), name(Direction::Render)))
}

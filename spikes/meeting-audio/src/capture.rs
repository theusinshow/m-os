//! Captura WASAPI: microfone e loopback do dispositivo de saida.
//!
//! Tudo que toca o Windows mora aqui. O resto do spike nao sabe que WASAPI
//! existe, que e a mesma fronteira que `docs/MEETING-AGENT.md` §4.2 desenha para
//! o `mos-audio` de producao.
//!
//! O loopback funciona pela combinacao de direcoes: pegamos o dispositivo de
//! SAIDA e pedimos `Direction::Capture` nele. O crate traduz isso em
//! `AUDCLNT_STREAMFLAGS_LOOPBACK` e recusa a combinacao com exclusive mode com
//! erro tipado — que e o comportamento que a documentacao da Microsoft descreve.

#![cfg(windows)]

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use wasapi::{
    initialize_mta, AudioClient, Device, DeviceEnumerator, Direction, SampleType, StreamMode,
    WaveFormat,
};

use crate::{
    chunks::{ChunkWriter, Format},
    report::{ChannelReport, ChannelStats},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Mic,
    System,
}

impl Channel {
    pub fn folder(self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::System => "system",
        }
    }

    /// De qual dispositivo o canal nasce.
    ///
    /// O microfone vem de um dispositivo de captura. O audio do sistema vem do
    /// dispositivo de RENDERIZACAO — e e essa escolha, e nao uma flag que
    /// passamos, que liga o loopback.
    fn device_direction(self) -> Direction {
        match self {
            Self::Mic => Direction::Capture,
            Self::System => Direction::Render,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timing {
    Events,
    Polling,
}

impl Timing {
    fn label(self) -> &'static str {
        match self {
            Self::Events => "events",
            Self::Polling => "polling",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaptureOptions {
    pub channel: Channel,
    pub timing: Timing,
    pub autoconvert: bool,
    pub chunk_ms: u64,
    pub out_dir: PathBuf,
    pub sample_rate: usize,
    /// Nome do dispositivo, ou vazio para o padrao do sistema.
    ///
    /// Existe para o teste da D-2: a saida padrao desta maquina e um dispositivo
    /// VIRTUAL, cujo driver roda continuamente e por isso nunca revelaria o
    /// buraco do loopback em silencio. So um endpoint fisico responde a
    /// pergunta.
    pub device_name: String,
}

/// O formato que pedimos ao motor de audio: 16 kHz, mono, i16.
///
/// E o que o Whisper consome. Guardar 48 kHz estereo float custaria 12x o disco
/// para informacao que nenhum consumidor le, e o audio e apagado depois do
/// processamento de qualquer forma.
fn desired_format(sample_rate: usize) -> WaveFormat {
    WaveFormat::new(16, 16, &SampleType::Int, sample_rate, 1, None)
}

fn format_label(format: &WaveFormat) -> String {
    let sample = match format.get_subformat() {
        Ok(SampleType::Float) => "f",
        Ok(SampleType::Int) => "i",
        Err(_) => "?",
    };
    format!(
        "{}/{}/{sample}{}",
        format.get_samplespersec(),
        format.get_nchannels(),
        format.get_bitspersample()
    )
}

/// Handle de um canal em captura.
pub struct ChannelHandle {
    join: JoinHandle<ChannelReport>,
    /// Nivel RMS mais recente, em milesimos. E o unico dado que sai da thread de
    /// captura em tempo real — nunca PCM.
    pub level_milli: Arc<AtomicU64>,
    pub frames: Arc<AtomicU64>,
}

impl ChannelHandle {
    pub fn join(self) -> ChannelReport {
        self.join.join().unwrap_or_else(|_| ChannelReport {
            device: "desconhecido".into(),
            requested_format: String::new(),
            effective_format: String::new(),
            timing: String::new(),
            autoconvert_accepted: false,
            sample_rate: 0,
            stats: ChannelStats::default(),
            recorded_ms: 0,
            device_span_ms: 0,
            drift_ms: 0,
            chunks: 0,
            trailing_bytes: 0,
            lost_at_ms: Some(0),
            lost_reason: Some("a thread de captura entrou em panico".into()),
        })
    }
}

pub fn spawn(options: CaptureOptions, stop: Arc<AtomicBool>) -> ChannelHandle {
    let level_milli = Arc::new(AtomicU64::new(0));
    let frames = Arc::new(AtomicU64::new(0));
    let level_for_thread = level_milli.clone();
    let frames_for_thread = frames.clone();

    let join = thread::Builder::new()
        .name(format!("capture-{}", options.channel.folder()))
        .spawn(move || run(options, stop, level_for_thread, frames_for_thread))
        .expect("criar thread de captura");

    ChannelHandle {
        join,
        level_milli,
        frames,
    }
}

/// Sobe a prioridade da thread para a classe de audio profissional do Windows.
///
/// Sem isso, uma varredura de disco ou um build em paralelo atrasa a leitura do
/// buffer o suficiente para o dispositivo descartar frames — e o sintoma seria
/// uma descontinuidade que pareceria culpa do WASAPI.
fn raise_thread_priority() -> Option<isize> {
    use windows_sys::Win32::System::Threading::AvSetMmThreadCharacteristicsW;

    let task: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
    let mut index = 0u32;
    let handle = unsafe { AvSetMmThreadCharacteristicsW(task.as_ptr(), &mut index) };
    if handle.is_null() {
        None
    } else {
        Some(handle as isize)
    }
}

fn run(
    options: CaptureOptions,
    stop: Arc<AtomicBool>,
    level_milli: Arc<AtomicU64>,
    frames_out: Arc<AtomicU64>,
) -> ChannelReport {
    initialize_mta().ok().expect("COM em MTA");
    let _priority = raise_thread_priority();

    let mut report = ChannelReport {
        device: String::new(),
        requested_format: format_label(&desired_format(options.sample_rate)),
        effective_format: String::new(),
        timing: options.timing.label().into(),
        autoconvert_accepted: false,
        sample_rate: options.sample_rate as u32,
        stats: ChannelStats::default(),
        recorded_ms: 0,
        device_span_ms: 0,
        drift_ms: 0,
        chunks: 0,
        trailing_bytes: 0,
        lost_at_ms: None,
        lost_reason: None,
    };

    let started = Instant::now();
    match open(&options) {
        Ok(open) => {
            report.device = open.device_name.clone();
            report.effective_format = format_label(&open.format);
            report.autoconvert_accepted = open.autoconvert_accepted;
            report.sample_rate = open.format.get_samplespersec();
            capture_loop(
                open,
                &options,
                &stop,
                &mut report,
                &level_milli,
                &frames_out,
                started,
            );
        }
        Err(error) => {
            // O canal nunca abriu. Isso e `Unavailable`, e nao uma gravacao
            // silenciosamente vazia: o relatorio diz o motivo.
            report.lost_at_ms = Some(0);
            report.lost_reason = Some(error);
        }
    }

    let format = Format {
        sample_rate: report.sample_rate.max(1),
        channels: 1,
        bytes_per_sample: 2,
    };
    if let Ok(recovered) = crate::chunks::recover(&options.out_dir, format) {
        report.chunks = recovered.chunks;
        report.trailing_bytes = recovered.trailing_bytes;
    }
    report.recorded_ms = report.stats.recorded_ms(report.sample_rate);
    report.device_span_ms = report.stats.device_span_ms();
    report.drift_ms = report.stats.drift_ms(report.sample_rate);
    report
}

struct OpenChannel {
    client: AudioClient,
    format: WaveFormat,
    device_name: String,
    autoconvert_accepted: bool,
    period_hns: i64,
    /// Taxa NATIVA do dispositivo, que nao e a nossa quando `autoconvert` esta
    /// ligado. Ela e necessaria porque `BufferInfo.index` conta em frames do
    /// DISPOSITIVO, e nao nos frames convertidos que recebemos — comparar os
    /// dois sem converter inventa frames perdidos que nunca existiram.
    device_sample_rate: u32,
}

fn open(options: &CaptureOptions) -> Result<OpenChannel, String> {
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let direction = options.channel.device_direction();
    let device: Device = if options.device_name.trim().is_empty() {
        enumerator
            .get_default_device(&direction)
            .map_err(|error| format!("nao ha dispositivo padrao de {direction:?}: {error}"))?
    } else {
        enumerator
            .get_device_collection(&direction)
            .map_err(|error| error.to_string())?
            .get_device_with_name(&options.device_name)
            .map_err(|error| {
                format!("dispositivo \"{}\" nao encontrado: {error}", options.device_name)
            })?
    };
    let device_name = device.get_friendlyname().unwrap_or_else(|_| "sem nome".into());
    let mut client = device.get_iaudioclient().map_err(|error| error.to_string())?;
    let (default_period, _min_period) = client
        .get_device_period()
        .map_err(|error| error.to_string())?;
    // Lido ANTES de `Initialize`: depois dele o cliente ja esta comprometido com
    // um formato, e a taxa nativa some da vista.
    let device_sample_rate = client
        .get_mixformat()
        .map(|mix| mix.get_samplespersec())
        .unwrap_or(options.sample_rate as u32);

    // Buffer de meio segundo. Grande o bastante para uma pausa de escalonamento
    // nao virar descontinuidade; pequeno o bastante para a latencia do Stop
    // continuar imperceptivel.
    let buffer_hns = 5_000_000i64;

    // Tentativa 1: o formato que queremos, com o motor convertendo (D-3).
    let wanted = desired_format(options.sample_rate);
    let mode = match options.timing {
        Timing::Events => StreamMode::EventsShared {
            autoconvert: options.autoconvert,
            buffer_duration_hns: buffer_hns,
        },
        Timing::Polling => StreamMode::PollingShared {
            autoconvert: options.autoconvert,
            buffer_duration_hns: buffer_hns,
        },
    };

    if options.autoconvert
        && client
            .initialize_client(&wanted, &Direction::Capture, &mode)
            .is_ok()
    {
        return Ok(OpenChannel {
            client,
            format: wanted,
            device_name,
            autoconvert_accepted: true,
            period_hns: default_period,
            device_sample_rate,
        });
    }

    // Tentativa 2: o formato do motor. Um cliente que ja falhou em Initialize
    // nao pode ser reinicializado, entao pedimos outro ao dispositivo — o COM
    // devolve uma instancia nova, e insistir na antiga daria
    // AUDCLNT_E_ALREADY_INITIALIZED, que pareceria um bug nosso.
    let mut client = device.get_iaudioclient().map_err(|error| error.to_string())?;
    let mix = client.get_mixformat().map_err(|error| error.to_string())?;
    let fallback_mode = match options.timing {
        Timing::Events => StreamMode::EventsShared {
            autoconvert: false,
            buffer_duration_hns: buffer_hns,
        },
        Timing::Polling => StreamMode::PollingShared {
            autoconvert: false,
            buffer_duration_hns: buffer_hns,
        },
    };
    client
        .initialize_client(&mix, &Direction::Capture, &fallback_mode)
        .map_err(|error| format!("Initialize recusou tambem o mix format: {error}"))?;

    Ok(OpenChannel {
        client,
        format: mix,
        device_name,
        autoconvert_accepted: false,
        period_hns: default_period,
        device_sample_rate,
    })
}

#[allow(clippy::too_many_arguments)]
fn capture_loop(
    open: OpenChannel,
    options: &CaptureOptions,
    stop: &AtomicBool,
    report: &mut ChannelReport,
    level_milli: &AtomicU64,
    frames_out: &AtomicU64,
    started: Instant,
) {
    let OpenChannel {
        client,
        format,
        period_hns,
        device_sample_rate,
        ..
    } = open;

    let bytes_per_frame = format.get_blockalign() as usize;
    let writer_format = Format {
        sample_rate: format.get_samplespersec(),
        channels: format.get_nchannels(),
        bytes_per_sample: format.get_bitspersample() / 8,
    };

    let mut writer = match ChunkWriter::create(&options.out_dir, writer_format, options.chunk_ms) {
        Ok(writer) => writer,
        Err(error) => {
            report.lost_at_ms = Some(0);
            report.lost_reason = Some(format!("nao foi possivel abrir o diretorio: {error}"));
            return;
        }
    };

    let event_handle = match options.timing {
        Timing::Events => match client.set_get_eventhandle() {
            Ok(handle) => Some(handle),
            Err(error) => {
                report.lost_at_ms = Some(0);
                report.lost_reason = Some(format!("SetEventHandle falhou: {error}"));
                return;
            }
        },
        Timing::Polling => None,
    };

    let capture = match client.get_audiocaptureclient() {
        Ok(capture) => capture,
        Err(error) => {
            report.lost_at_ms = Some(0);
            report.lost_reason = Some(format!("GetService(IAudioCaptureClient) falhou: {error}"));
            return;
        }
    };

    if let Err(error) = client.start_stream() {
        report.lost_at_ms = Some(0);
        report.lost_reason = Some(format!("Start falhou: {error}"));
        return;
    }

    // O periodo do dispositivo em ms, usado como cadencia do polling e como base
    // do timeout do evento.
    let period_ms = ((period_hns / 10_000).max(1)) as u32;
    let event_timeout_ms = (period_ms * 3).max(50);

    let mut scratch = vec![0u8; 1024 * 1024];
    let mut last_packet = Instant::now();
    // Quantos frames do DISPOSITIVO cabem em um frame nosso. Vale 1 quando nao
    // ha conversao e 3 quando pedimos 16 kHz a um dispositivo de 48 kHz.
    let index_ratio = device_sample_rate.max(1) as f64 / writer_format.sample_rate.max(1) as f64;
    let mut expected_index: Option<f64> = None;
    // Depois de tres timeouts seguidos o modo por evento e considerado morto.
    // Um timeout isolado e escalonamento; tres seguidos e a D-1 respondida.
    let mut consecutive_timeouts = 0u32;
    let mut using_events = event_handle.is_some();

    while !stop.load(Ordering::Relaxed) {
        if using_events {
            if let Some(handle) = &event_handle {
                if handle.wait_for_event(event_timeout_ms).is_err() {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts >= 3 {
                        // Troca de modo, e ela e REGISTRADA. Degradar em silencio
                        // seria prometer no relatorio um modo que nao foi usado.
                        report.stats.timing_fallbacks += 1;
                        report.timing = "events->polling".into();
                        using_events = false;
                    }
                    continue;
                }
                consecutive_timeouts = 0;
            }
        } else {
            thread::sleep(Duration::from_millis((period_ms / 2).max(1) as u64));
        }

        loop {
            let available = match capture.get_next_packet_size() {
                Ok(Some(frames)) => frames,
                Ok(None) => break,
                Err(error) => {
                    report.stats.read_errors += 1;
                    if report.lost_reason.is_none() {
                        report.lost_reason = Some(format!("GetNextPacketSize: {error}"));
                    }
                    break;
                }
            };
            if available == 0 {
                break;
            }

            let wanted_bytes = available as usize * bytes_per_frame;
            if scratch.len() < wanted_bytes {
                scratch.resize(wanted_bytes, 0);
            }

            let (frames_read, info) = match capture.read_from_device(&mut scratch[..wanted_bytes]) {
                Ok(result) => result,
                Err(error) => {
                    report.stats.read_errors += 1;
                    report.lost_at_ms = Some(started.elapsed().as_millis() as u64);
                    report.lost_reason = Some(format!("GetBuffer: {error}"));
                    stop_and_finish(&client, &mut writer);
                    return;
                }
            };
            if frames_read == 0 {
                break;
            }

            let now = Instant::now();
            let gap = now.duration_since(last_packet).as_millis() as u64;
            if report.stats.packets > 0 && gap > report.stats.max_gap_ms {
                report.stats.max_gap_ms = gap;
            }
            last_packet = now;

            report.stats.packets += 1;
            report.stats.frames += frames_read as u64;
            if info.flags.silent {
                report.stats.silent_packets += 1;
            }
            // O WASAPI marca DATA_DISCONTINUITY no PRIMEIRO pacote depois do
            // Start, sempre. Conta-la como defeito faria toda gravacao nascer
            // com uma falha que nao aconteceu.
            if info.flags.data_discontinuity && report.stats.packets > 1 {
                report.stats.discontinuities += 1;
            }
            if info.flags.timestamp_error {
                report.stats.timestamp_errors += 1;
            }

            // A posicao do dispositivo e o unico jeito honesto de saber que
            // frames sumiram: se o indice do primeiro frame deste pacote esta
            // alem de onde o anterior terminou, a diferenca nao chegou.
            //
            // O acumulador e f64 porque `index_ratio` pode nao ser inteiro. Com
            // u64 o arredondamento de cada pacote somaria alguns milhares de
            // frames fantasmas ao longo de uma hora.
            if let Some(expected) = expected_index {
                let gap_device_frames = info.index as f64 - expected;
                if gap_device_frames > index_ratio {
                    report.stats.dropped_frames += (gap_device_frames / index_ratio) as u64;
                }
            }
            expected_index = Some(info.index as f64 + frames_read as f64 * index_ratio);

            if report.stats.first_timestamp_hns == 0 {
                report.stats.first_timestamp_hns = info.timestamp;
            }
            report.stats.last_timestamp_hns = info.timestamp
                + (frames_read as u64 * 10_000_000 / writer_format.sample_rate.max(1) as u64);

            let payload = &scratch[..frames_read as usize * bytes_per_frame];
            level_milli.store(rms_milli(payload, &writer_format), Ordering::Relaxed);

            if let Err(error) = writer.write(payload) {
                report.lost_at_ms = Some(started.elapsed().as_millis() as u64);
                report.lost_reason = Some(format!("escrita em disco falhou: {error}"));
                stop_and_finish(&client, &mut writer);
                return;
            }
            frames_out.store(writer.total_frames(), Ordering::Relaxed);
        }
    }

    stop_and_finish(&client, &mut writer);
}

fn stop_and_finish(client: &AudioClient, writer: &mut ChunkWriter) {
    let _ = client.stop_stream();
    let _ = writer.finish();
}

/// RMS reduzido a milesimos. E o unico numero que sai do audio para o resto do
/// programa — a §4.3 do documento proibe PCM atravessando a fronteira.
fn rms_milli(bytes: &[u8], format: &Format) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let sum_squares: f64 = match format.bytes_per_sample {
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
    let count = (bytes.len() / format.bytes_per_sample.max(1) as usize).max(1);
    ((sum_squares / count as f64).sqrt() * 1000.0).min(1000.0) as u64
}

/// Mantem o motor de audio acordado escrevendo silencio no dispositivo de saida.
///
/// O WASAPI so empurra dados para o endpoint de renderizacao quando existe algum
/// stream ativo — quando nada toca, nao ha nada para capturar. Numa reuniao isso
/// acontece o tempo todo, e sem este stream o canal SYSTEM simplesmente PARA,
/// desalinhando a linha do tempo contra o microfone.
///
/// Ele nao produz som: escreve zeros e marca o buffer como silencioso.
pub fn spawn_keep_alive(stop: Arc<AtomicBool>, device_name: String) -> JoinHandle<Option<String>> {
    thread::Builder::new()
        .name("keep-alive".into())
        .spawn(move || {
            if let Err(error) = initialize_mta().ok() {
                return Some(format!("keep-alive nao inicializou COM: {error}"));
            }
            let enumerator = DeviceEnumerator::new().ok()?;
            let device = if device_name.trim().is_empty() {
                enumerator.get_default_device(&Direction::Render).ok()?
            } else {
                enumerator
                    .get_device_collection(&Direction::Render)
                    .ok()?
                    .get_device_with_name(&device_name)
                    .ok()?
            };
            let mut client = device.get_iaudioclient().ok()?;
            let format = client.get_mixformat().ok()?;
            let mode = StreamMode::PollingShared {
                autoconvert: false,
                buffer_duration_hns: 5_000_000,
            };
            if let Err(error) = client.initialize_client(&format, &Direction::Render, &mode) {
                return Some(format!("keep-alive nao abriu: {error}"));
            }
            let render = client.get_audiorenderclient().ok()?;
            if client.start_stream().is_err() {
                return Some("keep-alive nao iniciou".into());
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
            None
        })
        .expect("criar thread de keep-alive")
}

/// Lista os dispositivos das duas direcoes. Serve para o operador do spike saber
/// contra o que esta testando antes de gravar 15 minutos.
pub fn list_devices() -> Result<Vec<(String, String)>, String> {
    initialize_mta().ok().map_err(|error| error.to_string())?;
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let mut found = Vec::new();
    for direction in [Direction::Capture, Direction::Render] {
        let label = match direction {
            Direction::Capture => "entrada",
            Direction::Render => "saida",
        };
        let default_id = enumerator
            .get_default_device(&direction)
            .ok()
            .and_then(|device| device.get_id().ok());
        let collection = enumerator
            .get_device_collection(&direction)
            .map_err(|error| error.to_string())?;
        let count = collection.get_nbr_devices().unwrap_or(0);
        for index in 0..count {
            let Ok(device) = collection.get_device_at_index(index) else {
                continue;
            };
            let name = device.get_friendlyname().unwrap_or_else(|_| "sem nome".into());
            let is_default = device.get_id().ok() == default_id;
            found.push((
                format!("{label}{}", if is_default { " (padrao)" } else { "" }),
                name,
            ));
        }
    }
    Ok(found)
}

pub fn channel_dir(root: &Path, channel: Channel) -> PathBuf {
    root.join(channel.folder())
}

//! Spike descartavel da Fase 1 do Meeting Agent. NAO e codigo de produto.
//!
//! Ele existe para responder as perguntas D-1 a D-5 de `docs/MEETING-AGENT.md`
//! §24 com numeros medidos, e para provar o Gate A antes de qualquer linha de
//! interface ser escrita.
//!
//! Uso:
//!
//! ```text
//! meeting-audio-spike devices
//! meeting-audio-spike record --secs 900 --out .\sessao
//! meeting-audio-spike record --secs 600 --no-keepalive     # o teste da D-2
//! meeting-audio-spike record --secs 60  --timing polling   # o teste da D-1
//! meeting-audio-spike record --secs 60  --no-autoconvert   # o teste da D-3
//! meeting-audio-spike inspect .\sessao
//! ```

mod chunks;
mod report;

#[cfg(windows)]
mod capture;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime},
};

use report::Report;

/// Pedido de parada, compartilhado entre as threads e o handler de Ctrl+C.
static STOP: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");

    let code = match command {
        "devices" => cmd_devices(),
        "record" => cmd_record(&args[1..]),
        "inspect" => cmd_inspect(&args[1..]),
        _ => {
            print_help();
            0
        }
    };
    std::process::exit(code);
}

fn print_help() {
    println!(
        "M/OS Meeting Agent — spike de audio (Fase 1)\n\
         \n\
         Comandos:\n\
         \x20 devices                      lista dispositivos de entrada e saida\n\
         \x20 record [opcoes]              grava mic + audio do sistema\n\
         \x20 inspect <dir>                mede o que existe em disco\n\
         \n\
         Opcoes de record:\n\
         \x20 --secs N          duracao; sem isso grava ate Ctrl+C\n\
         \x20 --out DIR         diretorio da sessao (padrao: .\\meeting-spike)\n\
         \x20 --timing MODO     events | polling            (padrao: events)   D-1\n\
         \x20 --no-keepalive    nao mantem o motor acordado                    D-2\n\
         \x20 --no-autoconvert  nao pede 16 kHz mono ao motor                  D-3\n\
         \x20 --mic-only        so o microfone\n\
         \x20 --system-only     so o audio do sistema\n\
         \x20 --chunk-ms N      duracao do chunk (padrao: 10000)
\n           --mic-device N    nome exato do dispositivo de entrada
\n           --system-device N nome exato do dispositivo de saida            D-2"
    );
}

#[cfg(not(windows))]
fn cmd_devices() -> i32 {
    eprintln!("Este spike so roda no Windows: WASAPI nao existe em outro lugar.");
    1
}

#[cfg(windows)]
fn cmd_devices() -> i32 {
    match capture::list_devices() {
        Ok(devices) => {
            for (kind, name) in devices {
                println!("{kind:<18} {name}");
            }
            0
        }
        Err(error) => {
            eprintln!("Nao foi possivel listar dispositivos: {error}");
            1
        }
    }
}

struct Options {
    secs: Option<u64>,
    out: PathBuf,
    timing: &'static str,
    keep_alive: bool,
    autoconvert: bool,
    mic: bool,
    system: bool,
    chunk_ms: u64,
    mic_device: String,
    system_device: String,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        secs: None,
        out: PathBuf::from("meeting-spike"),
        timing: "events",
        keep_alive: true,
        autoconvert: true,
        mic: true,
        system: true,
        chunk_ms: 10_000,
        mic_device: String::new(),
        system_device: String::new(),
    };
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--secs" => {
                index += 1;
                options.secs = Some(
                    args.get(index)
                        .ok_or("--secs sem valor")?
                        .parse()
                        .map_err(|_| "--secs precisa de um numero")?,
                );
            }
            "--out" => {
                index += 1;
                options.out = PathBuf::from(args.get(index).ok_or("--out sem valor")?);
            }
            "--timing" => {
                index += 1;
                options.timing = match args.get(index).map(String::as_str) {
                    Some("events") => "events",
                    Some("polling") => "polling",
                    _ => return Err("--timing aceita events ou polling".into()),
                };
            }
            "--chunk-ms" => {
                index += 1;
                options.chunk_ms = args
                    .get(index)
                    .ok_or("--chunk-ms sem valor")?
                    .parse()
                    .map_err(|_| "--chunk-ms precisa de um numero")?;
            }
            "--mic-device" => {
                index += 1;
                options.mic_device = args.get(index).ok_or("--mic-device sem valor")?.clone();
            }
            "--system-device" => {
                index += 1;
                options.system_device =
                    args.get(index).ok_or("--system-device sem valor")?.clone();
            }
            "--no-keepalive" => options.keep_alive = false,
            "--no-autoconvert" => options.autoconvert = false,
            "--mic-only" => options.system = false,
            "--system-only" => options.mic = false,
            other => return Err(format!("opcao desconhecida: {other}")),
        }
        index += 1;
    }
    if !options.mic && !options.system {
        return Err("--mic-only e --system-only juntos nao gravam nada".into());
    }
    Ok(options)
}

#[cfg(not(windows))]
fn cmd_record(_args: &[String]) -> i32 {
    eprintln!("Este spike so roda no Windows: WASAPI nao existe em outro lugar.");
    1
}

#[cfg(windows)]
fn cmd_record(args: &[String]) -> i32 {
    use capture::{Channel, CaptureOptions, Timing};

    let options = match parse(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return 2;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&options.out) {
        eprintln!("Nao foi possivel criar {}: {error}", options.out.display());
        return 1;
    }

    install_ctrl_c();

    let timing = if options.timing == "polling" {
        Timing::Polling
    } else {
        Timing::Events
    };
    let stop = Arc::new(AtomicBool::new(false));

    let keep_alive = if options.keep_alive && options.system {
        Some(capture::spawn_keep_alive(
            stop.clone(),
            options.system_device.clone(),
        ))
    } else {
        None
    };

    let mic = options.mic.then(|| {
        capture::spawn(
            CaptureOptions {
                channel: Channel::Mic,
                timing,
                autoconvert: options.autoconvert,
                chunk_ms: options.chunk_ms,
                out_dir: capture::channel_dir(&options.out, Channel::Mic),
                sample_rate: 16_000,
                device_name: options.mic_device.clone(),
            },
            stop.clone(),
        )
    });
    let system = options.system.then(|| {
        capture::spawn(
            CaptureOptions {
                channel: Channel::System,
                timing,
                autoconvert: options.autoconvert,
                chunk_ms: options.chunk_ms,
                out_dir: capture::channel_dir(&options.out, Channel::System),
                sample_rate: 16_000,
                device_name: options.system_device.clone(),
            },
            stop.clone(),
        )
    });

    println!(
        "Gravando em {} · timing {} · keep-alive {} · autoconvert {}",
        options.out.display(),
        options.timing,
        if options.keep_alive { "ligado" } else { "DESLIGADO" },
        if options.autoconvert { "ligado" } else { "DESLIGADO" }
    );
    println!("Ctrl+C para parar.\n");

    let started = Instant::now();
    let started_unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // O laco da interface. Uma linha por segundo, e nada de PCM: e exatamente o
    // orcamento que a §4.3 do documento reserva para o renderer.
    loop {
        std::thread::sleep(Duration::from_millis(1000));
        let elapsed = started.elapsed();
        let mic_frames = mic.as_ref().map(|h| h.frames.load(Ordering::Relaxed)).unwrap_or(0);
        let system_frames = system
            .as_ref()
            .map(|h| h.frames.load(Ordering::Relaxed))
            .unwrap_or(0);
        let mic_level = mic.as_ref().map(|h| h.level_milli.load(Ordering::Relaxed)).unwrap_or(0);
        let system_level = system
            .as_ref()
            .map(|h| h.level_milli.load(Ordering::Relaxed))
            .unwrap_or(0);

        print!(
            "\r  {}   mic {:>6}s {}   system {:>6}s {}   ",
            clock(elapsed.as_secs()),
            mic_frames / 16_000,
            meter(mic_level),
            system_frames / 16_000,
            meter(system_level),
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();

        if STOP.load(Ordering::Relaxed) {
            break;
        }
        if let Some(secs) = options.secs {
            if elapsed.as_secs() >= secs {
                break;
            }
        }
    }
    println!("\n\nParando...");

    stop.store(true, Ordering::Relaxed);
    let wall_duration_ms = started.elapsed().as_millis() as u64;

    let mic_report = mic.map(|handle| handle.join());
    let system_report = system.map(|handle| handle.join());
    if let Some(handle) = keep_alive {
        if let Ok(Some(problem)) = handle.join() {
            eprintln!("keep-alive: {problem}");
        }
    }

    let cross = match (&mic_report, &system_report) {
        (Some(mic), Some(system)) => Some(mic.recorded_ms as i64 - system.recorded_ms as i64),
        _ => None,
    };
    let (cpu_ms, peak_bytes) = process_metrics();

    let mut report = Report {
        started_at_unix_ms: started_unix,
        wall_duration_ms,
        mic: mic_report,
        system: system_report,
        cross_channel_drift_ms: cross,
        keep_alive: options.keep_alive,
        peak_working_set_bytes: peak_bytes,
        process_cpu_ms: cpu_ms,
        cpu_percent_of_one_core: if wall_duration_ms > 0 {
            cpu_ms as f64 * 100.0 / wall_duration_ms as f64
        } else {
            0.0
        },
        bytes_on_disk: directory_bytes(&options.out),
        verdict: Vec::new(),
    };
    report.verdict = report::verdict(&report);

    let path = options.out.join("report.json");
    match serde_json::to_vec_pretty(&report) {
        Ok(json) => {
            if let Err(error) = std::fs::write(&path, json) {
                eprintln!("Nao foi possivel escrever {}: {error}", path.display());
            }
        }
        Err(error) => eprintln!("Nao foi possivel serializar o relatorio: {error}"),
    }

    print_report(&report);
    println!("\nRelatorio completo em {}", path.display());

    if report.verdict.iter().any(|line| line.starts_with("[FALHA]")) {
        1
    } else {
        0
    }
}

fn print_report(report: &Report) {
    println!("─────────────────────────────────────────────");
    for (name, channel) in [("MICROFONE", &report.mic), ("SISTEMA", &report.system)] {
        let Some(channel) = channel else { continue };
        println!("\n{name}  {}", channel.device);
        println!("  formato pedido   {}", channel.requested_format);
        println!(
            "  formato efetivo  {}{}",
            channel.effective_format,
            if channel.autoconvert_accepted {
                "  (autoconvert aceito)"
            } else {
                "  (autoconvert RECUSADO)"
            }
        );
        println!("  timing           {}", channel.timing);
        println!("  gravado          {} ms", channel.recorded_ms);
        println!("  relogio do device {} ms", channel.device_span_ms);
        println!("  deriva           {} ms", channel.drift_ms);
        println!("  pacotes          {}", channel.stats.packets);
        println!("  pacotes silenciosos {}", channel.stats.silent_packets);
        println!("  descontinuidades {}", channel.stats.discontinuities);
        println!("  frames perdidos  {}", channel.stats.dropped_frames);
        println!("  maior intervalo  {} ms", channel.stats.max_gap_ms);
        println!("  chunks           {}", channel.chunks);
        if let Some(at) = channel.lost_at_ms {
            println!(
                "  PERDIDO aos {at} ms: {}",
                channel.lost_reason.as_deref().unwrap_or("sem motivo")
            );
        }
    }
    println!("\nVEREDITO");
    for line in &report.verdict {
        println!("  {line}");
    }
}

fn clock(secs: u64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
}

/// Oito blocos de nivel. E o "pequeno indicador de nivel de mic" que o brief
/// permite — e o teto do que a interface de producao vai mostrar.
fn meter(level_milli: u64) -> String {
    let filled = (level_milli * 8 / 1000).min(8) as usize;
    format!("[{}{}]", "#".repeat(filled), "·".repeat(8 - filled))
}

fn directory_bytes(root: &std::path::Path) -> u64 {
    let mut total = 0;
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    for entry in entries.filter_map(Result::ok) {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            total += directory_bytes(&entry.path());
        } else {
            total += metadata.len();
        }
    }
    total
}

fn cmd_inspect(args: &[String]) -> i32 {
    let Some(root) = args.first().map(PathBuf::from) else {
        eprintln!("inspect precisa do diretorio da sessao");
        return 2;
    };
    let format = chunks::Format {
        sample_rate: 16_000,
        channels: 1,
        bytes_per_sample: 2,
    };
    println!("Sessao em {}", root.display());
    let mut worst: Option<u64> = None;
    for channel in ["mic", "system"] {
        match chunks::recover(&root.join(channel), format) {
            Ok(recovered) => {
                let ms = format.frames_to_ms(recovered.frames);
                println!(
                    "  {channel:<8} {} chunks · {} frames · {} · {} bytes soltos",
                    recovered.chunks,
                    recovered.frames,
                    clock(ms / 1000),
                    recovered.trailing_bytes
                );
                if recovered.frames > 0 {
                    worst = Some(worst.map_or(ms, |current: u64| current.min(ms)));
                }
            }
            Err(error) => println!("  {channel:<8} erro: {error}"),
        }
    }
    match worst {
        // Isto e o que a tela de recuperacao mostraria: a duracao que o disco
        // sustenta, medida em frames, e nao a que o relogio sugeriria.
        Some(ms) => println!("\n  Recuperavel: {}", clock(ms / 1000)),
        None => println!("\n  Nada recuperavel."),
    }
    0
}

#[cfg(windows)]
fn install_ctrl_c() {
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

    unsafe extern "system" fn handler(_ctrl_type: u32) -> i32 {
        // Nao encerra o processo: apenas PEDE a parada. O encerramento abrupto
        // deixaria o ultimo chunk sem `sync_all`, que e exatamente a perda que a
        // gravacao incremental existe para evitar.
        STOP.store(true, Ordering::Relaxed);
        1
    }

    unsafe {
        SetConsoleCtrlHandler(Some(handler), 1);
    }
}

#[cfg(not(windows))]
fn install_ctrl_c() {}

#[cfg(windows)]
fn process_metrics() -> (u64, u64) {
    use windows_sys::Win32::{
        Foundation::FILETIME,
        System::{
            ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            Threading::{GetCurrentProcess, GetProcessTimes},
        },
    };

    unsafe {
        let process = GetCurrentProcess();

        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let cpu_ms = if GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user)
            != 0
        {
            (filetime_to_hns(kernel) + filetime_to_hns(user)) / 10_000
        } else {
            0
        };

        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        let peak = if K32GetProcessMemoryInfo(
            process,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ) != 0
        {
            counters.PeakWorkingSetSize as u64
        } else {
            0
        };

        (cpu_ms, peak)
    }
}

#[cfg(windows)]
fn filetime_to_hns(value: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

#[cfg(not(windows))]
fn process_metrics() -> (u64, u64) {
    (0, 0)
}

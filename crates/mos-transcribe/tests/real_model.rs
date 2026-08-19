//! Exercicio contra um whisper.cpp de verdade.
//!
//! **`#[ignore]` de proposito**, e configurado por ambiente em vez de caminho
//! fixo: nem toda maquina tem o binario e o modelo, e um teste que exigisse
//! 550 MB em disco para `cargo test` passar seria um teste que todo mundo
//! desliga.
//!
//! ```powershell
//! $env:MOS_WHISPER_BIN  = "...\whisper-cli.exe"
//! $env:MOS_WHISPER_MODEL= "...\ggml-large-v3-turbo-q5_0.bin"
//! $env:MOS_WHISPER_WAV  = "...\reuniao-16k.wav"
//! cargo test -p mos-transcribe --test real_model -- --ignored --nocapture
//! ```
//!
//! Ele prova o que o teste de unidade nao pode: que o comando montado por este
//! crate roda, que a saida que o binario escreve e a que o parser espera, e que
//! o resultado sai limpo pela regra do dominio.

use std::{cell::RefCell, path::PathBuf, time::Instant};

use mos_core::{MeetingChannel, TranscriptionProvider, TranscriptionRequest};
use mos_transcribe::{WhisperCliProvider, WhisperConfig};

fn from_env() -> Option<(WhisperConfig, PathBuf)> {
    let binary = std::env::var("MOS_WHISPER_BIN").ok()?;
    let model = std::env::var("MOS_WHISPER_MODEL").ok()?;
    let wav = std::env::var("MOS_WHISPER_WAV").ok()?;
    Some((
        WhisperConfig {
            binary,
            model,
            threads: std::env::var("MOS_WHISPER_THREADS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
        },
        PathBuf::from(wav),
    ))
}

#[test]
#[ignore = "precisa de whisper.cpp e de um modelo; rode com --ignored"]
fn transcreve_um_wav_real_e_devolve_segmentos_limpos() {
    let Some((config, wav)) = from_env() else {
        panic!("defina MOS_WHISPER_BIN, MOS_WHISPER_MODEL e MOS_WHISPER_WAV");
    };

    let provider = WhisperCliProvider::new(config);
    provider.ready().expect("o provider precisa estar pronto");
    println!("provider: {}", provider.name());

    // `RefCell` porque a porta recebe `&dyn Fn`, e nao `FnMut`: o provider
    // chama o progresso de uma thread de trabalho, e um `FnMut` obrigaria o
    // chamador a pensar em exclusividade onde ele so quer contar.
    let visto = RefCell::new(Vec::new());
    let inicio = Instant::now();
    let segments = provider
        .transcribe(
            TranscriptionRequest {
                audio: &wav,
                channel: MeetingChannel::Mic,
                language: Some("pt"),
            },
            &|fraction| visto.borrow_mut().push(fraction),
        )
        .expect("a transcricao precisa funcionar");
    let levou = inicio.elapsed();

    println!("\n{} segmentos em {:.1?}", segments.len(), levou);
    for segment in &segments {
        println!(
            "  [{:>7}-{:>7} ms] {}",
            segment.start_ms, segment.end_ms, segment.text
        );
    }

    assert!(!segments.is_empty(), "o audio precisa produzir fala");

    // O progresso precisa chegar ao fim: uma barra que para em 90% e uma barra
    // que ensina a pessoa a nao confiar nela.
    let visto = visto.into_inner();
    assert_eq!(
        visto.last().copied(),
        Some(1.0),
        "o progresso precisa terminar em 1.0: {visto:?}"
    );

    // Ordem e intervalos precisam ser utilizaveis como evidencia: um segmento
    // fora de ordem faria o salto na transcricao pousar no lugar errado.
    for par in segments.windows(2) {
        assert!(
            par[0].start_ms <= par[1].start_ms,
            "segmentos fora de ordem: {} depois de {}",
            par[1].start_ms,
            par[0].start_ms
        );
    }
    for segment in &segments {
        assert!(segment.start_ms >= 0);
        assert!(segment.end_ms >= segment.start_ms);
        // A limpeza do dominio ja rodou: nada de marcador de ruido nem de texto
        // com espaco sobrando.
        assert_eq!(segment.text, segment.text.trim());
        assert!(mos_core::is_speech(&segment.text), "{:?}", segment.text);
    }
}

#[test]
#[ignore = "precisa de whisper.cpp e de um modelo; rode com --ignored"]
fn os_dois_canais_intercalam_preservando_a_origem() {
    let Some((config, wav)) = from_env() else {
        panic!("defina as variaveis de ambiente");
    };
    let outro = std::env::var("MOS_WHISPER_WAV2")
        .map(PathBuf::from)
        .expect("defina MOS_WHISPER_WAV2 com o segundo canal");

    let provider = WhisperCliProvider::new(config);
    let transcreve = |path: &PathBuf, channel| {
        provider
            .transcribe(
                TranscriptionRequest {
                    audio: path,
                    channel,
                    language: Some("pt"),
                },
                &|_| {},
            )
            .expect("transcricao")
    };

    let mic = transcreve(&wav, MeetingChannel::Mic);
    let system = transcreve(&outro, MeetingChannel::System);
    let meeting_id = mos_core::MeetingId::new();
    let intercalados = mos_core::interleave(meeting_id, mic, system);

    println!("\n{} segmentos intercalados", intercalados.len());
    for segment in &intercalados {
        let quem = match segment.channel {
            MeetingChannel::Mic => "VOCE  ",
            MeetingChannel::System => "REMOTO",
        };
        println!(
            "  {:>7} ms  {quem}  {}",
            segment.start_ms, segment.text
        );
    }

    assert!(intercalados.len() >= 4);
    // A ORIGEM e o que a V1 protege acima de tudo: sem os dois canais
    // representados, "o que EU prometi" deixa de existir como distincao.
    assert!(
        intercalados
            .iter()
            .any(|s| s.channel == MeetingChannel::Mic),
        "o canal do microfone precisa aparecer"
    );
    assert!(
        intercalados
            .iter()
            .any(|s| s.channel == MeetingChannel::System),
        "o canal do sistema precisa aparecer"
    );
    // `seq` denso, comecando em zero: e por ele que a transcricao e lida.
    assert_eq!(
        intercalados.iter().map(|s| s.seq).collect::<Vec<_>>(),
        (0..intercalados.len() as i64).collect::<Vec<_>>()
    );
}

//! Exercicio contra o hardware real.
//!
//! **`#[ignore]` de proposito.** Ele abre dispositivos de audio de verdade e
//! grava por segundos; num `cargo test` normal isso seria lentidao e falha em
//! qualquer maquina sem placa de som. Ele existe para ser rodado a mao:
//!
//! ```text
//! cargo test -p mos-audio --test hardware -- --ignored --nocapture
//! ```
//!
//! O spike da Fase 1 provou que o WASAPI faz o que a documentacao diz. Este
//! prova outra coisa, e ela nao e a mesma: que o crate de PRODUCAO — com o
//! keep-alive, os chunks, o manifesto e a conciliacao com o disco — grava.

#![cfg(windows)]

use std::{thread, time::Duration};

use mos_audio::{recover_session, Channel, Format, Recording, SessionDir};

/// Grava por tempo suficiente para cruzar uma fronteira de chunk.
///
/// 12 segundos e o numero minimo que exercita a rotacao: com `CHUNK_MS` de 10 s,
/// uma gravacao de 5 s so provaria que o primeiro arquivo abre.
const SEGUNDOS: u64 = 12;

#[test]
#[ignore = "abre dispositivos de audio reais; rode com --ignored"]
fn grava_os_dois_canais_e_concilia_com_o_disco() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("meetings/0198-teste");

    let recording =
        Recording::start(&root, "2026-08-19T14:00:00Z").expect("a gravacao precisa comecar");

    // O estado precisa ficar vivo enquanto grava, e a duracao precisa CRESCER.
    // Uma duracao parada seria a assinatura de um canal que abriu e nao entrega
    // — exatamente o que o keep-alive existe para impedir no canal do sistema.
    let mut leituras = Vec::new();
    for _ in 0..SEGUNDOS {
        thread::sleep(Duration::from_secs(1));
        let state = recording.state();
        leituras.push(state.duration_ms);
        println!(
            "  {:>6} ms   mic {:?} ({:>4})   system {:?} ({:>4})",
            state.duration_ms, state.mic, state.mic_level, state.system, state.system_level
        );
    }

    let outcome = recording
        .stop()
        .expect("parar precisa devolver o que gravou");
    println!("\noutcome: {outcome:#?}");

    assert!(
        leituras.last().unwrap() > &leituras[0],
        "a duracao precisa crescer durante a gravacao: {leituras:?}"
    );
    assert!(
        outcome.duration_ms >= (SEGUNDOS as i64 - 2) * 1000,
        "gravou {} ms, esperava perto de {} ms",
        outcome.duration_ms,
        SEGUNDOS * 1000
    );
    assert!(
        outcome.mic.has_audio(),
        "o microfone nao produziu audio: {:?}",
        outcome.mic
    );
    // O canal do sistema so tem audio continuo por causa do keep-alive. Se este
    // assert cair, e a §5.4 do documento que precisa ser reaberta.
    assert!(
        outcome.system.has_audio(),
        "o canal do sistema nao produziu audio — o keep-alive falhou: {:?}",
        outcome.system
    );

    // O que a recuperacao encontraria, se o processo tivesse morrido aqui.
    let recovered = recover_session(&root).unwrap();
    println!("recuperavel: {recovered:#?}");
    assert!(recovered.has_audio());
    assert_eq!(
        recovered.mic.trailing_bytes, 0,
        "nenhum arquivo pode terminar no meio de um frame"
    );
    assert_eq!(recovered.system.trailing_bytes, 0);
    assert!(
        recovered.mic.chunks >= 2,
        "12 s com chunk de 10 s precisa ter rotacionado: {} chunks",
        recovered.mic.chunks
    );

    // E os dois canais precisam ter gravado quase o mesmo tempo. A divergencia e
    // o numero que diz se a linha do tempo compartilhada se sustenta — e a
    // evidencia `14:04` depende dela.
    let mic_ms = recovered.mic.duration_ms(Format::CAPTURE);
    let system_ms = recovered.system.duration_ms(Format::CAPTURE);
    println!(
        "divergencia entre canais: {} ms",
        (mic_ms - system_ms).abs()
    );
    assert!(
        (mic_ms - system_ms).abs() < 500,
        "os canais divergiram {} ms (mic {mic_ms}, system {system_ms})",
        (mic_ms - system_ms).abs()
    );

    // O manifesto precisa dizer o que aconteceu, incluindo o keep-alive.
    let manifest = SessionDir::new(&root)
        .read_manifest()
        .unwrap()
        .expect("o manifesto precisa existir");
    println!("manifesto: {manifest:#?}");
    assert!(
        manifest.system.as_ref().is_some_and(|info| info.keep_alive),
        "o manifesto precisa registrar que o keep-alive estava ligado"
    );
    assert!(
        !manifest.mic.as_ref().unwrap().keep_alive,
        "e precisa registrar que o microfone NAO usa keep-alive"
    );
    assert_eq!(
        manifest.started_at, "2026-08-19T14:00:00Z",
        "o instante do inicio precisa sobreviver ao merge do fim"
    );

    // E os arquivos existem onde o desenho diz.
    let session = SessionDir::new(&root);
    assert!(session.channel(Channel::Mic).join("000000.pcm").exists());
    assert!(session.channel(Channel::System).join("000000.pcm").exists());
}

#[test]
#[ignore = "abre dispositivos de audio reais; rode com --ignored"]
fn uma_gravacao_curta_e_apagavel() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("meetings/0198-curta");

    let recording = Recording::start(&root, "2026-08-19T14:00:00Z").unwrap();
    thread::sleep(Duration::from_secs(2));
    let outcome = recording.stop().unwrap();
    assert!(outcome.duration_ms > 0);

    mos_audio::delete_session_audio(&root).unwrap();
    assert!(!root.exists(), "o diretorio inteiro sai");
    // E medir de novo devolve zero, sem erro: e o que a recuperacao veria depois
    // de uma limpeza.
    assert_eq!(recover_session(&root).unwrap().duration_ms, 0);
}

/// A gravacao do Voice Inbox: SO o microfone, e o pico que decide se houve fala.
///
/// Fale enquanto ele roda. As duas assercoes que importam sao opostas entre si,
/// e e por isso que as duas precisam existir:
///
/// - o **microfone** precisa ter passado do piso de energia, senao a fala nao
///   chegaria ao transcritor;
/// - o canal do **sistema** precisa nao existir. Um Voice Inbox que gravasse o
///   loopback capturaria a musica, a reuniao aberta atras e a voz de quem
///   estivesse do outro lado dela — e ninguem pediu isso ao apertar um atalho
///   para ditar um lembrete.
#[test]
#[ignore = "abre dispositivos de audio reais; rode com --ignored"]
fn a_gravacao_de_voz_pega_o_microfone_e_nada_alem_dele() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("voice/0198-teste");

    let recording =
        Recording::start_mic(&root, "2026-08-19T14:00:00Z").expect("a gravacao precisa comecar");
    println!("FALE AGORA — 4 segundos");
    for _ in 0..4 {
        thread::sleep(Duration::from_secs(1));
        let state = recording.state();
        println!(
            "  {:>6} ms   mic {:>4} (pico {:>4})   system {:?}",
            state.duration_ms, state.mic_level, state.mic_peak, state.system
        );
    }
    let outcome = recording
        .stop()
        .expect("parar precisa devolver o que gravou");
    println!("\noutcome: {outcome:#?}");

    assert!(
        outcome.mic.has_audio(),
        "o microfone nao gravou: {:?}",
        outcome.mic
    );
    assert!(
        outcome.mic_peak >= mos_core_piso(),
        "o pico foi {} — abaixo do piso, a fala nao chegaria ao transcritor",
        outcome.mic_peak
    );
    assert!(
        !outcome.system.has_audio(),
        "o Voice Inbox nao pode gravar o audio do sistema: {:?}",
        outcome.system
    );

    // E nao ha sequer diretorio para o canal do sistema.
    let session = SessionDir::new(&root);
    assert!(session.channel(Channel::Mic).join("000000.pcm").exists());
    assert!(
        !session.channel(Channel::System).join("000000.pcm").exists(),
        "nenhum byte do sistema pode ter sido escrito"
    );
}

/// O piso de energia do dominio, repetido aqui como literal.
///
/// `mos-audio` nao depende de `mos-core` (§4.2), e essa ausencia e o que
/// mantem a fronteira. O numero e o mesmo de `mos_core::voice::MIN_PEAK_LEVEL`,
/// e o custo de duplicar e um literal — menor que o de furar a fronteira por
/// causa de um teste.
fn mos_core_piso() -> u64 {
    120
}

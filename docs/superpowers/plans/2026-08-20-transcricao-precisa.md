# Transcrição Precisa — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer a transcrição de reuniões pegar o que foi de fato dito — corrigindo o nível do áudio antes do whisper, calando o laço em silêncio, e tornando o número do progresso verdadeiro.

**Architecture:** Três camadas, cada uma no crate onde a regra pertence e onde os testes rodam nesta máquina. O ganho é adaptativo e vive no adapter de áudio (`mos-audio`), aplicado só ao WAV temporário que o whisper vê — os chunks no disco não mudam. Os freios (`-sns`, VAD) viram argumentos montados por função pura no `mos-transcribe`. O colapso de laço é regra de domínio, no `mos-core`, ao lado do `is_speech`.

**Tech Stack:** Rust (crates `mos-audio`, `mos-transcribe`, `mos-core`, `mos-desktop`), React + TypeScript (renderer), `whisper-cli.exe` do whisper.cpp como sidecar, modelo Silero para VAD.

## Global Constraints

- **Spec de origem:** `docs/superpowers/specs/2026-08-20-transcricao-precisa-design.md`. Toda decisão numerada (D-1 a D-6) tem tarefa correspondente aqui.
- **Ganho:** aplica só se RMS < **−32 dBFS**; mira **−25 dBFS**; teto de **20 dB**.
- **VAD:** `-vt 0.25`, `-vp 250`. Caminho do modelo **vazio significa VAD desligado, nunca erro**.
- **Colapso:** **3 ou mais** idênticos consecutivos viram um. Dois sobrevivem.
- **Limiares finos** (`-nth`, `-et`, `-lpt`) ficam no default. Não mexer.
- **Nada de vocabulário / `--prompt`.** Reprovou na medição: 82 repetições da mesma frase.
- **Comentário em código é sem acento** (convenção do repositório); documento e teste podem ter.
- **Testes do `mos-desktop` não rodam nesta máquina** (`SETUP-MAQUINA.md` §4). Nenhuma regra nova pode nascer lá.
- **Commit por tarefa**, e `git add` **por caminho** — nunca `git add -A`. Outra sessão pode estar escrevendo no mesmo repositório.

---

### Task 1: O ganho adaptativo, do cálculo até os dois chamadores

**Files:**
- Modify: `crates/mos-audio/src/wav.rs` (duas funções puras, uma pública e testes)
- Modify: `crates/mos-audio/src/lib.rs:32` (exportar a nova função)
- Modify: `apps/desktop/src-tauri/src/meeting.rs:837`
- Modify: `apps/desktop/src-tauri/src/voice.rs:582`
- Test: `crates/mos-audio/src/wav.rs` (módulo `tests` no fim do arquivo)

**Interfaces:**
- Consumes: nada de tarefas anteriores.
- Produces: `ganho_para(rms: f32) -> f32`; `com_ganho(amostra: i16, ganho: f32) -> i16`; `export_channel_normalized(session_root: &Path, channel: Channel, destination: &Path) -> Result<(u64, f32), AudioError>` — devolve frames e o ganho aplicado, sendo `1.0` "arquivo intocado".

- [ ] **Step 1: Escrever os testes das duas funções puras**

No fim de `crates/mos-audio/src/wav.rs`, dentro de `mod tests`:

```rust
    #[test]
    fn ganho_o_canal_baixo_sobe_e_o_alto_passa_intocado() {
        // O mic da reuniao medida: -44 dBFS.
        assert!((ganho_para(206.0) - 8.94).abs() < 0.05);
        // O canal do sistema da mesma reuniao: -22 dBFS. Acima do piso.
        assert_eq!(ganho_para(2500.0), 1.0);
        // Exatamente no piso nao mexe: o piso e o limite de quem ja esta bom.
        assert_eq!(ganho_para(823.2), 1.0);
        // Um canal quase mudo nao vira chiado amplificado: o teto e 20 dB.
        assert_eq!(ganho_para(1.0), 10.0);
        // Silencio absoluto nao divide por zero.
        assert_eq!(ganho_para(0.0), 1.0);
    }

    #[test]
    fn ganho_o_joelho_suave_nao_estoura_o_inteiro() {
        // Sem ganho, a amostra atravessa igual.
        assert_eq!(com_ganho(1234, 1.0), 1234);
        // Um pico que multiplicado passaria de i16 e curvado, e nao cortado.
        let alto = com_ganho(20_000, 8.94);
        assert!(alto < 32_767, "deveria curvar antes do teto, veio {alto}");
        assert!(alto > 25_000, "curvou cedo demais, veio {alto}");
        // A curva preserva o sinal.
        assert_eq!(com_ganho(-20_000, 8.94), -alto);
        // Fala baixa cresce quase linearmente: e o ponto do ganho.
        assert!((com_ganho(500, 8.94) as f32 - 4470.0).abs() < 60.0);
    }
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-audio ganho`
Expected: FAIL com `cannot find function 'ganho_para' in this scope`

- [ ] **Step 3: Implementar as duas funções puras**

Em `crates/mos-audio/src/wav.rs`, antes de `fn storage`:

```rust
/// O maior valor de um i16, como f32. E o teto do joelho suave.
const LIMITE: f32 = 32_767.0;
/// Abaixo disto o canal e baixo demais para o transcritor. -32 dBFS.
const PISO_RMS: f32 = 823.2;
/// Para onde o ganho mira. -25 dBFS.
const ALVO_RMS: f32 = 1842.6;
/// Teto de 20 dB. Um canal quase mudo nao vira um canal de chiado amplificado.
const GANHO_MAXIMO: f32 = 10.0;

/// Quanto amplificar um canal com este RMS.
///
/// **Adaptativo, e nao fixo.** Na reuniao que originou esta regra o microfone
/// estava em -44 dBFS e o audio do sistema em -22 dBFS. Ganho fixo estragaria o
/// segundo para salvar o primeiro; o piso existe para que quem ja esta bom passe
/// intocado.
pub fn ganho_para(rms: f32) -> f32 {
    if rms <= 0.0 || rms >= PISO_RMS {
        return 1.0;
    }
    (ALVO_RMS / rms).min(GANHO_MAXIMO)
}

/// Aplica o ganho com joelho suave.
///
/// `tanh` e nao corte: cortar um pico gera harmonico que o mel do whisper le
/// como consoante que ninguem falou. A curva comprime o pico e deixa a fala
/// baixa — que e o que interessa — crescer quase linearmente.
pub fn com_ganho(amostra: i16, ganho: f32) -> i16 {
    if ganho == 1.0 {
        return amostra;
    }
    let ampliada = amostra as f32 * ganho;
    (LIMITE * (ampliada / LIMITE).tanh()).round() as i16
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test -p mos-audio ganho`
Expected: PASS, 2 testes.

- [ ] **Step 5: Escrever os testes da função pública**

```rust
    /// Um segundo de senoide com a amplitude pedida, em chunks de 100 ms.
    fn grava_onda(session: &SessionDir, channel: Channel, amplitude: f32) {
        let mut bytes = Vec::new();
        for i in 0..16_000i32 {
            let amostra = ((i as f32 * 0.05).sin() * amplitude) as i16;
            bytes.extend_from_slice(&amostra.to_le_bytes());
        }
        let mut writer =
            ChunkWriter::create(&session.channel(channel), Format::CAPTURE, 100).unwrap();
        writer.write(&bytes).unwrap();
        writer.finish().unwrap();
    }

    fn pico(bytes: &[u8]) -> u16 {
        bytes
            .chunks_exact(2)
            .map(|par| i16::from_le_bytes([par[0], par[1]]).unsigned_abs())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn normalizado_levanta_o_canal_baixo_e_nao_toca_nos_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let session = sessao(&dir.path().join("0199"));
        grava_onda(&session, Channel::Mic, 300.0);

        let destino = dir.path().join("saida/mic.wav");
        let (frames, ganho) =
            export_channel_normalized(session.path(), Channel::Mic, &destino).unwrap();

        assert_eq!(frames, 16_000);
        assert!(ganho > 1.0, "canal baixo deveria receber ganho, veio {ganho}");

        let bytes = fs::read(&destino).unwrap();
        assert_eq!(bytes.len(), 44 + 32_000);
        assert!(pico(&bytes[44..]) > 1_500, "o ganho nao chegou no arquivo");

        // E os chunks no disco continuam sendo o que o microfone captou.
        let chunk = fs::read_dir(session.channel(Channel::Mic))
            .unwrap()
            .filter_map(Result::ok)
            .map(|entrada| entrada.path())
            .find(|caminho| caminho.extension().is_some_and(|ext| ext == "pcm"))
            .unwrap();
        assert!(pico(&fs::read(chunk).unwrap()) <= 301, "o chunk foi alterado");
    }

    #[test]
    fn normalizado_nao_mexe_num_canal_que_ja_esta_alto() {
        let dir = tempfile::tempdir().unwrap();
        let session = sessao(&dir.path().join("0200"));
        grava_onda(&session, Channel::System, 8_000.0);

        let destino = dir.path().join("saida/system.wav");
        let (_, ganho) =
            export_channel_normalized(session.path(), Channel::System, &destino).unwrap();
        assert_eq!(ganho, 1.0);
    }

    #[test]
    fn normalizado_nao_quebra_com_canal_vazio() {
        let dir = tempfile::tempdir().unwrap();
        let session = sessao(&dir.path().join("0201"));
        let destino = dir.path().join("saida/mic.wav");
        let (frames, ganho) =
            export_channel_normalized(session.path(), Channel::Mic, &destino).unwrap();
        assert_eq!(frames, 0);
        assert_eq!(ganho, 1.0);
    }
```

- [ ] **Step 6: Rodar e ver falhar**

Run: `cargo test -p mos-audio normalizado`
Expected: FAIL com `cannot find function 'export_channel_normalized'`

- [ ] **Step 7: Implementar a função pública**

Em `crates/mos-audio/src/wav.rs`, logo depois de `export_channel`:

```rust
/// O mesmo canal, com o nivel corrigido para o transcritor.
///
/// **Duas passadas, e nao uma.** O ganho depende do RMS do canal INTEIRO, e o
/// RMS so existe depois de ler tudo — comecar a amplificar no primeiro chunk
/// seria escolher o ganho pelos primeiros dez segundos.
///
/// A primeira passada e o `export_channel` que ja existe, com seus testes. A
/// segunda reescreve as amostras, e so acontece quando ha ganho a aplicar.
///
/// Devolve os frames e o ganho aplicado; `1.0` significa arquivo intocado.
pub fn export_channel_normalized(
    session_root: &Path,
    channel: Channel,
    destination: &Path,
) -> Result<(u64, f32), AudioError> {
    let frames = export_channel(session_root, channel, destination)?;
    if frames == 0 {
        return Ok((0, 1.0));
    }

    let mut bytes = fs::read(destination).map_err(|error| storage(destination, error))?;
    let dados = &mut bytes[HEADER_BYTES as usize..];
    let total = dados.len() / 2;
    if total == 0 {
        return Ok((frames, 1.0));
    }

    let mut soma = 0f64;
    for par in dados.chunks_exact(2) {
        let amostra = i16::from_le_bytes([par[0], par[1]]) as f64;
        soma += amostra * amostra;
    }
    let rms = (soma / total as f64).sqrt() as f32;

    let ganho = ganho_para(rms);
    if ganho == 1.0 {
        return Ok((frames, 1.0));
    }

    for par in dados.chunks_exact_mut(2) {
        let amostra = i16::from_le_bytes([par[0], par[1]]);
        par.copy_from_slice(&com_ganho(amostra, ganho).to_le_bytes());
    }
    fs::write(destination, &bytes).map_err(|error| storage(destination, error))?;
    Ok((frames, ganho))
}
```

Em `crates/mos-audio/src/lib.rs:32`, trocar a linha do `pub use`:

```rust
pub use wav::{export_channel, export_channel_normalized};
```

- [ ] **Step 8: Rodar a suíte inteira do crate**

Run: `cargo test -p mos-audio`
Expected: PASS — inclusive os testes antigos do `export_channel`, cujo comportamento não pode ter mudado.

- [ ] **Step 9: Ligar os dois chamadores**

Em `apps/desktop/src-tauri/src/meeting.rs:837`, trocar:

```rust
        let frames = mos_audio::export_channel(&root, audio_channel, &wav)
            .map_err(|error| error.to_string())?;
```

por:

```rust
        // Normalizado, e nao cru: o microfone desta casa grava a -44 dBFS, e o
        // whisper responde a audio baixo entrando em laco de repeticao. O ganho
        // vive so neste WAV temporario; os chunks no disco seguem intocados.
        let (frames, _ganho) = mos_audio::export_channel_normalized(&root, audio_channel, &wav)
            .map_err(|error| error.to_string())?;
```

Em `apps/desktop/src-tauri/src/voice.rs:582`, trocar:

```rust
    let frames = mos_audio::export_channel(&root, mos_audio::Channel::Mic, &wav)
```

por:

```rust
    // Nota de voz e o MESMO microfone baixo da reuniao, e merece o mesmo ganho.
    let (frames, _ganho) =
        mos_audio::export_channel_normalized(&root, mos_audio::Channel::Mic, &wav)
```

- [ ] **Step 10: Conferir que o desktop ainda compila**

Run: `cargo check -p mos-desktop`
Expected: `Finished` sem erro.

- [ ] **Step 11: Commit**

```bash
git add crates/mos-audio/src/wav.rs crates/mos-audio/src/lib.rs apps/desktop/src-tauri/src/meeting.rs apps/desktop/src-tauri/src/voice.rs
git commit -m "feat(audio): o canal baixo chega ao whisper com nivel, e o alto passa intocado"
```

---

### Task 2: Os freios — `args()` puro, `-sns` e o VAD

**Files:**
- Modify: `crates/mos-transcribe/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/meeting.rs` (`meeting_set_transcriber`, `TranscriberStatus`, `meeting_transcriber_status`)
- Test: `crates/mos-transcribe/src/lib.rs` (módulo `tests` no fim do arquivo)

**Interfaces:**
- Consumes: nada da Task 1.
- Produces: `WhisperConfig { binary: String, model: String, threads: u32, vad_model: String }` (serializa `vadModel`); `pub fn args(config: &WhisperConfig, audio: &Path, saida: &Path, language: Option<&str>) -> Vec<String>`.

- [ ] **Step 1: Escrever os testes de `args`**

No fim de `crates/mos-transcribe/src/lib.rs`, dentro de `mod tests`:

```rust
    fn config_de_teste(vad: &str) -> WhisperConfig {
        WhisperConfig {
            binary: r"C:\w\whisper-cli.exe".into(),
            model: r"C:\w\ggml-large-v3.bin".into(),
            threads: 0,
            vad_model: vad.into(),
        }
    }

    #[test]
    fn os_freios_entram_e_o_vad_traz_os_limiares_medidos() {
        let saida = std::path::PathBuf::from(r"C:\tmp\mic.whisper");
        let lista = args(
            &config_de_teste(r"C:\w\ggml-silero-v5.1.2.bin"),
            std::path::Path::new(r"C:\tmp\mic.wav"),
            &saida,
            Some("pt"),
        );

        // Suprimir token de nao-fala e o freio que nao depende de modelo nenhum.
        assert!(lista.iter().any(|a| a == "-sns"));
        assert!(lista.iter().any(|a| a == "--vad"));
        assert!(lista.iter().any(|a| a == r"C:\w\ggml-silero-v5.1.2.bin"));

        // 0.25 e 250 sao MEDIDOS: no padrao 0.5 o VAD apagou fala de verdade.
        let vt = lista.iter().position(|a| a == "-vt").unwrap();
        assert_eq!(lista[vt + 1], "0.25");
        let vp = lista.iter().position(|a| a == "-vp").unwrap();
        assert_eq!(lista[vp + 1], "250");

        // O que ja existia continua: idioma declarado e JSON de saida.
        let l = lista.iter().position(|a| a == "-l").unwrap();
        assert_eq!(lista[l + 1], "pt");
        assert!(lista.iter().any(|a| a == "-oj"));
        assert!(lista.iter().any(|a| a == "-np"));

        // E o que a medicao REPROVOU nao pode aparecer nunca.
        assert!(!lista.iter().any(|a| a == "--prompt"));
    }

    #[test]
    fn sem_modelo_de_vad_o_vad_simplesmente_nao_entra() {
        let saida = std::path::PathBuf::from(r"C:\tmp\mic.whisper");
        let lista = args(
            &config_de_teste("   "),
            std::path::Path::new(r"C:\tmp\mic.wav"),
            &saida,
            Some("pt"),
        );
        // Degrada, e nao quebra: quem nao baixou o Silero transcreve como antes.
        assert!(!lista.iter().any(|a| a == "--vad"));
        assert!(!lista.iter().any(|a| a == "-vm"));
        // Mas o freio que nao depende de arquivo nenhum continua de pe.
        assert!(lista.iter().any(|a| a == "-sns"));
    }

    #[test]
    fn zero_threads_e_sem_idioma_deixam_o_binario_decidir() {
        let saida = std::path::PathBuf::from(r"C:\tmp\mic.whisper");
        let sem = args(
            &config_de_teste(""),
            std::path::Path::new(r"C:\tmp\mic.wav"),
            &saida,
            None,
        );
        assert!(!sem.iter().any(|a| a == "-t"));
        assert!(!sem.iter().any(|a| a == "-l"));

        let mut config = config_de_teste("");
        config.threads = 8;
        let com = args(&config, std::path::Path::new(r"C:\tmp\mic.wav"), &saida, None);
        let t = com.iter().position(|a| a == "-t").unwrap();
        assert_eq!(com[t + 1], "8");
    }
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-transcribe`
Expected: FAIL com `cannot find function 'args'` e `struct 'WhisperConfig' has no field named 'vad_model'`

- [ ] **Step 3: Acrescentar o campo à configuração**

Em `crates/mos-transcribe/src/lib.rs`, dentro de `pub struct WhisperConfig`, depois de `threads`:

```rust
    /// Caminho do modelo Silero, para o VAD.
    ///
    /// **Vazio significa VAD desligado, e nao erro.** Uma maquina que nao baixou
    /// o Silero continua transcrevendo como antes — a mesma disciplina que o
    /// `binary` e o `model` seguem, e a razao da D-7 do MEETING-AGENT: a escolha
    /// de runtime e do usuario.
    #[serde(default)]
    pub vad_model: String,
```

- [ ] **Step 4: Escrever a função `args`**

Em `crates/mos-transcribe/src/lib.rs`, antes de `impl TranscriptionProvider for WhisperCliProvider`:

```rust
/// Limiar do VAD. **Medido, e nao herdado.**
///
/// No padrao 0.5 o VAD tratou um microfone de -44 dBFS como silencio e apagou
/// uma conversa tecnica inteira. 0.25 com folga de 250 ms nas bordas recuperou o
/// mesmo trecho.
const VAD_LIMIAR: &str = "0.25";
/// Folga nas bordas do trecho de fala, em ms. Sem ela o VAD corta a primeira
/// silaba, que e onde mora a diferenca entre "laje" e "aje".
const VAD_FOLGA_MS: &str = "250";

/// A linha de comando inteira, como dado.
///
/// Funcao pura, e nao um `Command` montado no meio do `transcribe`, porque a
/// escolha dos freios e a parte desta casa que MAIS precisa de teste — e um
/// `Command` nao se inspeciona.
pub fn args(
    config: &WhisperConfig,
    audio: &Path,
    saida: &Path,
    language: Option<&str>,
) -> Vec<String> {
    let mut lista: Vec<String> = vec![
        "-m".into(),
        config.model.trim().into(),
        "-f".into(),
        audio.display().to_string(),
        // JSON, e nao o texto da tela: o texto perde os offsets, e sem offset
        // nao existe evidencia clicavel.
        "-oj".into(),
        "-of".into(),
        saida.display().to_string(),
        // Sem impressao progressiva do TEXTO: ela seria descartada.
        "-np".into(),
        // Suprime token de nao-fala. E o freio que nao depende de arquivo
        // nenhum, entao ele vale ate em maquina sem Silero.
        "-sns".into(),
    ];

    if let Some(language) = language {
        lista.push("-l".into());
        lista.push(language.into());
    }
    if config.threads > 0 {
        lista.push("-t".into());
        lista.push(config.threads.to_string());
    }

    let vad = config.vad_model.trim();
    if !vad.is_empty() {
        lista.push("--vad".into());
        lista.push("-vm".into());
        lista.push(vad.into());
        lista.push("-vt".into());
        lista.push(VAD_LIMIAR.into());
        lista.push("-vp".into());
        lista.push(VAD_FOLGA_MS.into());
    }

    lista
}
```

- [ ] **Step 5: Usar `args` dentro do `transcribe`**

Em `crates/mos-transcribe/src/lib.rs`, no `fn transcribe`, trocar todo o bloco que vai de `let mut command = Command::new(self.binary());` até o fecho do `if self.config.threads > 0 { ... }` por:

```rust
        let mut command = Command::new(self.binary());
        command
            .args(args(&self.config, request.audio, &saida, request.language))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
```

- [ ] **Step 6: Rodar e ver passar**

Run: `cargo test -p mos-transcribe`
Expected: PASS — os 3 testes novos e os antigos de parsing.

- [ ] **Step 7: Fazer o desktop compilar de novo**

O `WhisperConfig` ganhou campo, e o `mos-desktop` constrói um.

Em `apps/desktop/src-tauri/src/meeting.rs`, em `pub struct TranscriberStatus`, acrescentar depois de `pub model: String,`:

```rust
    pub vad_model: String,
```

Em `pub fn meeting_transcriber_status`, dentro do literal `TranscriberStatus { ... }`, acrescentar **logo depois de `model: config.model,`** (a ordem importa: `config` é movido campo a campo):

```rust
        vad_model: config.vad_model,
```

E trocar `meeting_set_transcriber` inteira por:

```rust
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
    Ok(meeting_transcriber_status(app))
}
```

- [ ] **Step 8: Conferir a compilação**

Run: `cargo check -p mos-desktop`
Expected: `Finished` sem erro.

- [ ] **Step 9: Commit**

```bash
git add crates/mos-transcribe/src/lib.rs apps/desktop/src-tauri/src/meeting.rs
git commit -m "feat(transcricao): a linha de comando vira dado, e ganha os freios medidos"
```

---

### Task 3: O progresso que diz a verdade

**Files:**
- Modify: `crates/mos-transcribe/src/lib.rs`
- Test: `crates/mos-transcribe/src/lib.rs`

**Interfaces:**
- Consumes: `args(...)` da Task 2.
- Produces: `pub fn parse_progress(linha: &str) -> Option<f32>`, devolvendo `0.0..=1.0`.

- [ ] **Step 1: Escrever o teste do parser**

```rust
    #[test]
    fn progresso_a_linha_do_binario_vira_fracao() {
        assert_eq!(
            parse_progress("whisper_print_progress_callback: progress =   7%"),
            Some(0.07)
        );
        assert_eq!(
            parse_progress("whisper_print_progress_callback: progress = 100%"),
            Some(1.0)
        );
        // Linha de outra natureza nao e progresso.
        assert_eq!(parse_progress("ggml_cuda_init: found 1 CUDA devices"), None);
        assert_eq!(parse_progress(""), None);
        // Numero fora da faixa nao vira fracao maluca.
        assert_eq!(parse_progress("progress = 900%"), Some(1.0));
    }
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-transcribe progresso`
Expected: FAIL com `cannot find function 'parse_progress'`

- [ ] **Step 3: Implementar o parser**

Em `crates/mos-transcribe/src/lib.rs`, ao lado de `args`:

```rust
/// Le `progress = NN%` do stderr do binario.
///
/// O formato vem do `whisper_print_progress_callback` do whisper.cpp, que so
/// aparece com `-pp`. Ele CONVIVE com o `-np` — verificado neste build antes de
/// a decisao ser tomada.
pub fn parse_progress(linha: &str) -> Option<f32> {
    let depois = linha.split("progress =").nth(1)?;
    let numero = depois.trim().strip_suffix('%')?.trim();
    let porcento: f32 = numero.parse().ok()?;
    Some((porcento / 100.0).clamp(0.0, 1.0))
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cargo test -p mos-transcribe progresso`
Expected: PASS

- [ ] **Step 5: Pedir o progresso ao binário**

Em `args`, logo depois da linha `"-sns".into(),`, acrescentar:

```rust
        // Progresso no stderr. Sem ele o unico progresso possivel seria 0, 0.9 e
        // 1.0 — uma barra que pula de nada para quase tudo.
        "-pp".into(),
```

E acrescentar esta asserção ao teste `os_freios_entram_e_o_vad_traz_os_limiares_medidos`:

```rust
        assert!(lista.iter().any(|a| a == "-pp"));
```

- [ ] **Step 6: Trocar `output()` por leitura em fluxo**

Em `crates/mos-transcribe/src/lib.rs`, trocar o bloco que hoje começa em `let output = command.output()` e termina no fecho do `if !output.status.success() { ... }` por:

```rust
        use std::io::{BufRead, BufReader};

        let mut filho = command
            .spawn()
            .map_err(|error| TranscriptionError::MissingRuntime {
                detail: format!("nao foi possivel executar {}: {error}", self.config.binary),
            })?;

        // As ultimas linhas ficam guardadas porque a mensagem de erro sai daqui.
        // Ler o stderr so no fim — como o `output()` fazia — e o que impedia o
        // progresso de existir: ele chegava depois de o trabalho ter acabado.
        let mut ultimas: Vec<String> = Vec::new();
        if let Some(stderr) = filho.stderr.take() {
            for linha in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(fracao) = parse_progress(&linha) {
                    progress(fracao);
                    continue;
                }
                if !linha.trim().is_empty() {
                    ultimas.push(linha);
                    if ultimas.len() > 20 {
                        ultimas.remove(0);
                    }
                }
            }
        }

        let status = filho
            .wait()
            .map_err(|error| TranscriptionError::MissingRuntime {
                detail: format!("{} nao terminou: {error}", self.config.binary),
            })?;

        if !status.success() {
            // O stderr do binario e tecnico, e vai para o erro — mas ele nunca
            // contem transcricao, entao nao viola a regra de nao registrar
            // conteudo de reuniao (§16.3).
            let detail = ultimas
                .last()
                .cloned()
                .unwrap_or_else(|| "sem detalhe".to_owned());
            return Err(TranscriptionError::Failed { detail });
        }
```

- [ ] **Step 7: Rodar a suíte do crate**

Run: `cargo test -p mos-transcribe`
Expected: PASS

- [ ] **Step 8: Conferir a compilação do desktop**

Run: `cargo check -p mos-desktop`
Expected: `Finished` sem erro.

- [ ] **Step 9: Commit**

```bash
git add crates/mos-transcribe/src/lib.rs
git commit -m "feat(transcricao): o progresso vem do binario enquanto ele trabalha"
```

---

### Task 4: O colapso de laço, no domínio

**Files:**
- Modify: `crates/mos-core/src/meeting.rs`
- Test: `crates/mos-core/src/meeting.rs` (módulo `tests` do arquivo)

**Interfaces:**
- Consumes: nada das tarefas anteriores.
- Produces: `fn colapsar_lacos(segments: Vec<RawSegment>) -> Vec<RawSegment>`, privada, chamada no fim de `clean_segments`. `RawSegment { start_ms: i64, end_ms: i64, text: String, confidence: Option<f32> }`.

- [ ] **Step 1: Escrever os testes**

No `mod tests` de `crates/mos-core/src/meeting.rs`:

```rust
    fn seg(inicio: i64, fim: i64, texto: &str) -> RawSegment {
        RawSegment {
            start_ms: inicio,
            end_ms: fim,
            text: texto.into(),
            confidence: None,
        }
    }

    #[test]
    fn laco_tres_repeticoes_viram_uma() {
        // O laco real da reuniao de 20/08: 24 "Tchau" seguidos no rabo mudo.
        let limpos = clean_segments(vec![
            seg(0, 1000, "Bom dia"),
            seg(1000, 2000, "Tchau."),
            seg(2000, 3000, "Tchau."),
            seg(3000, 4000, "Tchau."),
            seg(4000, 5000, "Ate mais"),
        ]);
        let textos: Vec<&str> = limpos.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(textos, vec!["Bom dia", "Tchau.", "Ate mais"]);

        // O colapsado cobre o intervalo INTEIRO do laco: do inicio do primeiro
        // ao fim do ultimo. Um salto na transcricao tem que pousar no trecho.
        assert_eq!(limpos[1].start_ms, 1000);
        assert_eq!(limpos[1].end_ms, 4000);
    }

    #[test]
    fn laco_duas_repeticoes_sao_fala() {
        // "Uhum, uhum" numa ligacao acontece. Vinte e quatro "Tchau", nao.
        let limpos = clean_segments(vec![seg(0, 500, "Uhum."), seg(500, 1000, "Uhum.")]);
        assert_eq!(limpos.len(), 2);
    }

    #[test]
    fn laco_so_junta_o_que_e_consecutivo() {
        let limpos = clean_segments(vec![
            seg(0, 100, "Sim."),
            seg(100, 200, "Sim."),
            seg(200, 300, "E ai?"),
            seg(300, 400, "Sim."),
        ]);
        assert_eq!(limpos.len(), 4);
    }

    #[test]
    fn laco_ignora_caixa_e_espaco_em_volta() {
        let limpos = clean_segments(vec![
            seg(0, 100, "Beleza."),
            seg(100, 200, " beleza. "),
            seg(200, 300, "BELEZA."),
        ]);
        assert_eq!(limpos.len(), 1);
        assert_eq!(limpos[0].text, "Beleza.");
        assert_eq!(limpos[0].end_ms, 300);
    }
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cargo test -p mos-core laco`
Expected: FAIL — `laco_tres_repeticoes_viram_uma` recebe 5 segmentos onde espera 3.

- [ ] **Step 3: Implementar o colapso**

Em `crates/mos-core/src/meeting.rs`, antes de `pub fn clean_segments`:

```rust
/// A partir de quantas repeticoes seguidas deixa de ser fala.
///
/// **Tres, e nao duas.** "Uhum, uhum" numa ligacao e resposta de gente; vinte e
/// quatro "Tchau" seguidos e o decodificador girando no silencio depois de a
/// ligacao ja ter acabado. Duas e o limite do que uma pessoa faz sem querer.
const REPETICOES_ATE_VIRAR_LACO: usize = 3;

/// Duas falas sao a mesma para efeito de laco.
///
/// Ignora caixa e espaco em volta porque o decodificador varia os dois no meio
/// do proprio laco — "Beleza." e " beleza. " sao a mesma volta da roda.
fn mesma_fala(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Junta laco de repeticao num segmento so.
///
/// Existe porque NENHUMA configuracao do whisper matou o laco: as onze rodadas
/// que originaram esta regra deixaram de 3 a 10 repeticoes, com e sem VAD, com e
/// sem supressao de nao-fala. Sobrando no provider, a regra tem que estar aqui —
/// no dominio, junto do `is_speech`, para nao depender de quem escreveu o
/// adapter.
///
/// O segmento que sobra cobre o intervalo INTEIRO do laco: um salto na
/// transcricao precisa pousar no trecho que a evidencia aponta.
fn colapsar_lacos(segments: Vec<RawSegment>) -> Vec<RawSegment> {
    fn descarregar(grupo: &mut Vec<RawSegment>, fora: &mut Vec<RawSegment>) {
        if grupo.len() >= REPETICOES_ATE_VIRAR_LACO {
            let fim = grupo.iter().map(|s| s.end_ms).max().unwrap_or_default();
            let mut primeiro = grupo.remove(0);
            primeiro.end_ms = fim;
            fora.push(primeiro);
            grupo.clear();
        } else {
            fora.append(grupo);
        }
    }

    let mut fora: Vec<RawSegment> = Vec::with_capacity(segments.len());
    let mut grupo: Vec<RawSegment> = Vec::new();

    for segmento in segments {
        let continua = grupo
            .last()
            .is_some_and(|anterior| mesma_fala(&anterior.text, &segmento.text));
        if !continua {
            descarregar(&mut grupo, &mut fora);
        }
        grupo.push(segmento);
    }
    descarregar(&mut grupo, &mut fora);

    fora
}
```

- [ ] **Step 4: Chamar do `clean_segments`**

Em `crates/mos-core/src/meeting.rs`, trocar o fim de `pub fn clean_segments` — a linha `segments` que fecha a função — por:

```rust
    // O colapso vem DEPOIS da ordenacao: laco e fenomeno de vizinhanca, e
    // vizinhanca so existe depois de ordenar.
    colapsar_lacos(segments)
```

- [ ] **Step 5: Rodar e ver passar**

Run: `cargo test -p mos-core`
Expected: PASS — os 4 novos e os 298 que já existiam.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src/meeting.rs
git commit -m "feat(reuniao): vinte e quatro tchaus viram um, e dois uhum sobrevivem"
```

---

### Task 5: O campo do VAD na tela

**Files:**
- Modify: `apps/desktop/src/types.ts` (tipo `TranscriberStatus`)
- Modify: `apps/desktop/src/api.ts:230`
- Modify: `apps/desktop/src/MeetingSettings.tsx`
- Test: nenhum arquivo de teste novo — é ligação de campo, e não há teste de DOM neste repo (a razão está escrita no topo de `lequePetalas.ts`).

**Interfaces:**
- Consumes: `TranscriberStatus.vadModel` e o comando `meeting_set_transcriber(binary, model, threads, vadModel)`, ambos da Task 2.
- Produces: nada para tarefas seguintes.

- [ ] **Step 1: Acrescentar o campo ao tipo**

Em `apps/desktop/src/types.ts`, no tipo `TranscriberStatus`, depois de `threads`:

```ts
  vadModel: string;
```

- [ ] **Step 2: Passar o campo na chamada**

Em `apps/desktop/src/api.ts`, trocar `meetingSetTranscriber` por:

```ts
  meetingSetTranscriber(binary: string, model: string, threads: number, vadModel: string) {
    return invoke<TranscriberStatus>("meeting_set_transcriber", { binary, model, threads, vadModel });
  },
```

- [ ] **Step 3: Ligar o campo na tela**

Em `apps/desktop/src/MeetingSettings.tsx`, junto dos outros `useState` (perto da linha 22):

```tsx
  const [vadModel, setVadModel] = useState("");
```

No carregamento, junto de `setThreads(String(status.threads));`:

```tsx
      setVadModel(status.vadModel);
```

Na gravação (linha 50), trocar a chamada por:

```tsx
      const status = await api.meetingSetTranscriber(binary, model, Number(threads) || 0, vadModel);
```

E, depois do campo de `threads`, acrescentar:

```tsx
        <label className="meeting-field">
          <span className="micro-label">MODELO DE VAD (OPCIONAL)</span>
          <input
            value={vadModel}
            onChange={(event) => setVadModel(event.target.value)}
            placeholder="C:\Dev\whisper\ggml-silero-v5.1.2.bin"
          />
        </label>
        <p className="meeting-hint">
          O VAD faz o transcritor <b>não ver o silêncio</b>, que é onde nascem as repetições em
          laço. Vazio, a transcrição funciona como antes — sem VAD, e não com erro.
        </p>
```

- [ ] **Step 4: Conferir tipo e testes do renderer**

Run: `cd apps/desktop && npx tsc --noEmit && npm test`
Expected: `tsc` sem saída e 156 testes passando.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/types.ts apps/desktop/src/api.ts apps/desktop/src/MeetingSettings.tsx
git commit -m "feat(reuniao): o caminho do VAD entra em Settings, e vazio e uma resposta valida"
```

---

### Task 6: A verificação de ponta a ponta, na reunião real

**Files:**
- Modify: `%APPDATA%\com.codedbym.mos\settings.json` (não é arquivo do repositório)
- Nenhum arquivo de código.

**Interfaces:**
- Consumes: tudo das Tasks 1 a 5.
- Produces: o veredito. Sem ele, nada aqui pode ser chamado de pronto.

- [ ] **Step 1: Rodar as suítes que rodam nesta máquina**

Run: `cargo test -p mos-core -p mos-storage-sqlite -p mos-audio -p mos-transcribe`
Expected: todas PASS. **`mos-desktop` fica fora de propósito** — os testes dele não rodam aqui (`SETUP-MAQUINA.md` §4), e é por isso que nenhuma regra nova mora lá.

- [ ] **Step 2: Apontar a configuração para o modelo grande e para o Silero**

**Feche o app antes de editar**: ele reescreve o arquivo inteiro ao salvar Settings, e uma edição feita com ele aberto se perde no próximo clique em Aplicar.

Editar `%APPDATA%\com.codedbym.mos\settings.json`, bloco `whisper`:

```json
  "whisper": {
    "binary": "C:\\Dev\\whisper\\Release\\whisper-cli.exe",
    "model": "C:\\Dev\\whisper\\ggml-large-v3.bin",
    "threads": 0,
    "vadModel": "C:\\Dev\\whisper\\ggml-silero-v5.1.2.bin"
  },
```

- [ ] **Step 3: Compilar e instalar**

```bash
cd apps/desktop && npm run tauri build
```

Depois, com o app fechado, rodar `target\release\bundle\nsis\M-OS_0.3.0_x64-setup.exe /S` e **copiar `target\release\WebView2Loader.dll` para `%LOCALAPPDATA%\M-OS\`** — o instalador não a empacota no toolchain GNU, e sem ela o app morre com `0xC0000135`.

O erro `TAURI_SIGNING_PRIVATE_KEY` no fim do build é da assinatura do updater e não afeta o pacote local.

- [ ] **Step 4: Transcrever a reunião de 6 minutos pelo app**

Abrir Reuniões, escolher *"Reunião de 20/08 14:30"*, clicar **Transcrever**.

Expected: termina sem erro, em torno de 1 minuto para os dois canais somados.

- [ ] **Step 5: Conferir contra a rodada aprovada**

O corpus e as onze rodadas estão em `C:\WINDOWS\TEMP\claude\C--Dev-pessoal-m-os\ec5e445c-7722-48aa-930b-da739eeefd95\scratchpad\bancada`. A rodada que o dono validou é a `r7`.

Três coisas têm que ser verdade na transcrição que o app produziu:

1. o canal MIC tem **por volta de 23 frases** de 4 ou mais palavras (contra 17 de antes);
2. **nenhuma sequência de 3 repetições idênticas** sobrevive;
3. o trecho dos 330 s diz **"armadura da laje"** — não "lágrima", não "Live".

Se as três valerem, a spec foi cumprida. Se a 3 falhar, **não mexa nos limiares por conta própria**: a spec §3 registra que esse termo é instável, e perseguir isso é decisão do dono.

- [ ] **Step 6: Fechar**

Não há código para commitar neste passo. Se algum ajuste tiver sido necessário, commite-o com o caminho explícito e uma mensagem que diga o que a verificação encontrou.

---

## O que este plano deliberadamente NÃO faz

- **Não põe barra de progresso na tela.** Ele entrega o número verdadeiro (Task 3); desenhar a barra é da spec do redesenho de Reuniões.
- **Não toca em `--prompt` nem em vocabulário.** Reprovado na medição.
- **Não mexe em `-nth`, `-et` nem `-lpt`.** Ficam no default até que exista medição pedindo outra coisa.
- **Não muda a captura.** Gravar o microfone com nível decente na origem é o remédio de raiz, e é outro projeto: mexe no caminho da gravação, não no da transcrição.

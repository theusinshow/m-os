//! Transcricao local, por binario externo.
//!
//! # A decisao D-6, e por que sidecar
//!
//! As duas rotas eram reais. `whisper-rs` daria um processo so, sem IPC e com
//! progresso nativo — e traria `cmake` e a compilacao do ggml para dentro do
//! `cargo build` de todos os crates que dependessem dele, incluindo o do
//! desktop.
//!
//! O que decidiu foi o terreno, e nao a elegancia. O `SETUP-MAQUINA.md` §2
//! documenta que a maquina principal ja perdeu uma tarde porque faltava
//! `windres`, e §4 registra que `cargo test -p mos-desktop` **nao roda la** por
//! incompatibilidade de DLL entre mingws. Acrescentar um build de C++ a essa
//! cadeia e a proxima tarde perdida — e ela cairia sobre o `cargo build` de
//! quem so quisesse mexer numa tela.
//!
//! O sidecar custa IPC, parsing e um binario a assinar. Em troca:
//!
//! - `cargo build` continua puro Rust;
//! - o binario e TROCAVEL sem recompilar nada, o que faz a decisao D-7
//!   (CPU, Vulkan ou CUDA) virar uma escolha do usuario em vez de uma escolha
//!   de build;
//! - a maquina ja tem precedente: o `ffmpeg` mora fora do processo.
//!
//! # O que ele espera
//!
//! `whisper-cli.exe` do `whisper.cpp`, com `-oj` (saida JSON). O contrato lido e
//! o campo `transcription[]`, com `offsets.from`/`offsets.to` em milissegundos.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use mos_core::{RawSegment, TranscriptionError, TranscriptionProvider, TranscriptionRequest};
use serde::Deserialize;

/// Onde o binario e o modelo estao.
///
/// Os dois sao caminhos, e nao um "modelo" nomeado, porque a escolha entre
/// `base`, `small` e `large-v3-turbo`, e entre uma build de CPU e uma de Vulkan,
/// e do usuario. Nomear o modelo aqui congelaria a decisao D-7 num literal.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperConfig {
    /// Caminho de `whisper-cli.exe`.
    pub binary: String,
    /// Caminho do `.bin` do modelo. Precisa ser MULTILINGUE: as reunioes sao em
    /// portugues, e as variantes `.en` estao fora por construcao.
    pub model: String,
    /// Quantas threads. Zero deixa o binario decidir.
    #[serde(default)]
    pub threads: u32,
}

impl WhisperConfig {
    pub fn is_set(&self) -> bool {
        !self.binary.trim().is_empty() && !self.model.trim().is_empty()
    }
}

pub struct WhisperCliProvider {
    config: WhisperConfig,
}

impl WhisperCliProvider {
    pub fn new(config: WhisperConfig) -> Self {
        Self { config }
    }

    fn binary(&self) -> PathBuf {
        PathBuf::from(self.config.binary.trim())
    }

    fn model(&self) -> PathBuf {
        PathBuf::from(self.config.model.trim())
    }
}

impl TranscriptionProvider for WhisperCliProvider {
    fn name(&self) -> String {
        // O NOME DO MODELO, e nao "whisper": ele vai para
        // `MeetingAnalysis.model` e para a tela, e "whisper" nao distingue uma
        // transcricao feita com `base` de uma feita com `large-v3`.
        let modelo = self
            .model()
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "desconhecido".into());
        format!("whisper.cpp · {modelo}")
    }

    fn ready(&self) -> Result<(), TranscriptionError> {
        if !self.config.is_set() {
            return Err(TranscriptionError::NotConfigured);
        }
        // Os dois sao checados separadamente porque as frases sao diferentes:
        // "instale o transcritor" e "baixe o modelo" mandam a pessoa a lugares
        // diferentes.
        if !self.binary().exists() {
            return Err(TranscriptionError::MissingRuntime {
                detail: format!("o executavel nao esta em {}", self.config.binary),
            });
        }
        if !self.model().exists() {
            return Err(TranscriptionError::MissingRuntime {
                detail: format!("o modelo nao esta em {}", self.config.model),
            });
        }
        Ok(())
    }

    fn transcribe(
        &self,
        request: TranscriptionRequest<'_>,
        progress: &dyn Fn(f32),
    ) -> Result<Vec<RawSegment>, TranscriptionError> {
        self.ready()?;

        // Um WAV so com cabecalho e um canal que nao gravou nada. Chamar o
        // binario com ele gastaria segundos para devolver vazio — e um "vazio"
        // vindo do modelo e indistinguivel de um "vazio" vindo da falta de
        // audio, que sao coisas diferentes.
        let bytes = std::fs::metadata(request.audio)
            .map_err(|error| TranscriptionError::MissingRuntime {
                detail: format!("o audio nao esta em {}: {error}", request.audio.display()),
            })?
            .len();
        if bytes <= 44 {
            return Ok(Vec::new());
        }

        progress(0.0);

        let saida = request.audio.with_extension("whisper");
        let mut command = Command::new(self.binary());
        command
            .arg("-m")
            .arg(self.model())
            .arg("-f")
            .arg(request.audio)
            // JSON, e nao o texto da tela: o texto perde os offsets, e sem
            // offset nao existe evidencia clicavel.
            .arg("-oj")
            .arg("-of")
            .arg(&saida)
            // Sem impressao progressiva no stdout: ela seria descartada, e o
            // binario gasta tempo formatando o que ninguem le.
            .arg("-np")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        if let Some(language) = request.language {
            command.arg("-l").arg(language);
        }
        if self.config.threads > 0 {
            command.arg("-t").arg(self.config.threads.to_string());
        }

        let output = command
            .output()
            .map_err(|error| TranscriptionError::MissingRuntime {
                detail: format!("nao foi possivel executar {}: {error}", self.config.binary),
            })?;

        if !output.status.success() {
            // O stderr do binario e tecnico, e vai INTEIRO para o erro — mas ele
            // nunca contem transcricao, entao nao viola a regra de nao registrar
            // conteudo de reuniao (§16.3).
            let detail = String::from_utf8_lossy(&output.stderr)
                .lines()
                .rfind(|line| !line.trim().is_empty())
                .unwrap_or("sem detalhe")
                .to_owned();
            return Err(TranscriptionError::Failed { detail });
        }

        progress(0.9);

        // O `-of` do whisper.cpp acrescenta a extensao ao caminho dado.
        let json_path = with_suffix(&saida, ".json");
        let json = std::fs::read_to_string(&json_path).map_err(|error| {
            TranscriptionError::Unreadable {
                detail: format!("saida nao encontrada em {}: {error}", json_path.display()),
            }
        })?;
        // Melhor esforco: o arquivo e temporario, e deixa-lo para tras so ocupa
        // disco. Falhar aqui seria falhar uma transcricao que deu certo.
        let _ = std::fs::remove_file(&json_path);

        let segments = parse(&json)?;
        progress(1.0);
        Ok(mos_core::clean_segments(segments))
    }
}

/// `Path::with_extension` TROCA a extensao; aqui a extensao e acrescentada.
///
/// `saida.whisper` vira `saida.whisper.json`, que e o que o `-of` produz. Usar
/// `with_extension("json")` daria `saida.json`, e o arquivo nunca seria achado.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_os_string();
    text.push(suffix);
    PathBuf::from(text)
}

#[derive(Deserialize)]
struct WhisperOutput {
    #[serde(default)]
    transcription: Vec<WhisperSegment>,
}

#[derive(Deserialize)]
struct WhisperSegment {
    #[serde(default)]
    text: String,
    #[serde(default)]
    offsets: Option<WhisperOffsets>,
}

#[derive(Deserialize)]
struct WhisperOffsets {
    #[serde(default)]
    from: i64,
    #[serde(default)]
    to: i64,
}

/// Le a saida do binario.
///
/// Segmento sem `offsets` e DESCARTADO, e nao colocado em zero. Um segmento em
/// `00:00` que na verdade aconteceu aos 40 minutos e pior que um segmento
/// ausente: a evidencia clicavel levaria a pessoa ao lugar errado, e ela
/// confiaria no que visse.
fn parse(json: &str) -> Result<Vec<RawSegment>, TranscriptionError> {
    let output: WhisperOutput =
        serde_json::from_str(json).map_err(|error| TranscriptionError::Unreadable {
            detail: error.to_string(),
        })?;

    Ok(output
        .transcription
        .into_iter()
        .filter_map(|segment| {
            let offsets = segment.offsets?;
            Some(RawSegment {
                start_ms: offsets.from.max(0),
                end_ms: offsets.to.max(offsets.from.max(0)),
                text: segment.text,
                // O whisper.cpp nao expoe confianca por segmento no `-oj`.
                // Inventar um numero aqui seria pior que a ausencia: ele
                // apareceria na tela como se tivesse sido medido.
                confidence: None,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAIDA: &str = r#"{
      "systeminfo": "AVX2",
      "model": { "type": "large" },
      "transcription": [
        {
          "timestamps": { "from": "00:00:04,000", "to": "00:00:08,000" },
          "offsets": { "from": 4000, "to": 8000 },
          "text": " Eu termino os slides amanha de manha."
        },
        {
          "timestamps": { "from": "00:00:09,000", "to": "00:00:12,000" },
          "offsets": { "from": 9000, "to": 12000 },
          "text": " [Música]"
        },
        {
          "timestamps": { "from": "00:00:13,000", "to": "00:00:15,000" },
          "offsets": { "from": 13000, "to": 15000 },
          "text": " Combinado."
        }
      ]
    }"#;

    #[test]
    fn le_a_saida_do_whisper_com_offsets() {
        let segments = parse(SAIDA).unwrap();
        assert_eq!(segments.len(), 3, "o parse nao filtra; quem filtra e o dominio");
        assert_eq!(segments[0].start_ms, 4000);
        assert_eq!(segments[0].end_ms, 8000);
        assert!(segments[0].text.contains("slides"));
        assert!(segments[0].confidence.is_none(), "confianca nao e inventada");
    }

    #[test]
    fn a_limpeza_do_dominio_tira_o_ruido() {
        let limpos = mos_core::clean_segments(parse(SAIDA).unwrap());
        assert_eq!(limpos.len(), 2, "[Música] sai");
        assert_eq!(limpos[0].text, "Eu termino os slides amanha de manha.");
        assert_eq!(limpos[1].text, "Combinado.");
    }

    #[test]
    fn segmento_sem_offset_e_descartado_e_nao_zerado() {
        let json = r#"{"transcription":[
            {"text":"sem tempo"},
            {"offsets":{"from":1000,"to":2000},"text":"com tempo"}
        ]}"#;
        let segments = parse(json).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "com tempo");
    }

    #[test]
    fn offset_invertido_e_corrigido_em_vez_de_gerar_intervalo_negativo() {
        let json = r#"{"transcription":[{"offsets":{"from":5000,"to":1000},"text":"invertido"}]}"#;
        let segments = parse(json).unwrap();
        assert_eq!(segments[0].start_ms, 5000);
        assert_eq!(segments[0].end_ms, 5000);
    }

    #[test]
    fn saida_ilegivel_vira_erro_nomeado() {
        assert!(matches!(
            parse("isto nao e json"),
            Err(TranscriptionError::Unreadable { .. })
        ));
    }

    #[test]
    fn saida_sem_transcricao_e_vazia_e_nao_erro() {
        // Um WAV de silencio absoluto produz isto, e nao e falha.
        assert!(parse(r#"{"systeminfo":"AVX2"}"#).unwrap().is_empty());
    }

    #[test]
    fn sem_configuracao_o_provider_diz_isso_e_nao_falha_generica() {
        let provider = WhisperCliProvider::new(WhisperConfig::default());
        assert_eq!(provider.ready(), Err(TranscriptionError::NotConfigured));
    }

    #[test]
    fn binario_e_modelo_ausentes_produzem_frases_diferentes() {
        let dir = tempfile::tempdir().unwrap();
        let binario = dir.path().join("whisper-cli.exe");
        std::fs::write(&binario, b"nao importa").unwrap();

        // Binario existe, modelo nao.
        let provider = WhisperCliProvider::new(WhisperConfig {
            binary: binario.display().to_string(),
            model: dir.path().join("ggml.bin").display().to_string(),
            threads: 0,
        });
        let error = provider.ready().unwrap_err();
        assert!(
            format!("{error}").contains("modelo"),
            "o erro precisa apontar para o modelo: {error}"
        );

        // Nenhum dos dois existe.
        let provider = WhisperCliProvider::new(WhisperConfig {
            binary: dir.path().join("nao-existe.exe").display().to_string(),
            model: dir.path().join("ggml.bin").display().to_string(),
            threads: 0,
        });
        let error = provider.ready().unwrap_err();
        assert!(
            format!("{error}").contains("executavel"),
            "o erro precisa apontar para o executavel: {error}"
        );
    }

    #[test]
    fn o_nome_carrega_o_modelo_e_nao_so_whisper() {
        let provider = WhisperCliProvider::new(WhisperConfig {
            binary: "whisper-cli.exe".into(),
            model: r"C:\modelos\ggml-large-v3-turbo-q5_0.bin".into(),
            threads: 0,
        });
        assert_eq!(provider.name(), "whisper.cpp · ggml-large-v3-turbo-q5_0");
    }

    #[test]
    fn a_extensao_da_saida_e_acrescentada_e_nao_trocada() {
        // `with_extension` daria `saida.json` e o arquivo nunca seria achado.
        assert_eq!(
            with_suffix(Path::new("a/b/saida.whisper"), ".json"),
            PathBuf::from("a/b/saida.whisper.json")
        );
    }

    #[test]
    fn um_wav_so_com_cabecalho_devolve_vazio_sem_chamar_o_binario() {
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("vazio.wav");
        std::fs::write(&audio, [0u8; 44]).unwrap();
        let binario = dir.path().join("whisper-cli.exe");
        std::fs::write(&binario, b"x").unwrap();
        let modelo = dir.path().join("ggml.bin");
        std::fs::write(&modelo, b"x").unwrap();

        let provider = WhisperCliProvider::new(WhisperConfig {
            binary: binario.display().to_string(),
            model: modelo.display().to_string(),
            threads: 0,
        });
        let segments = provider
            .transcribe(
                TranscriptionRequest {
                    audio: &audio,
                    channel: mos_core::MeetingChannel::System,
                    language: Some("pt"),
                },
                &|_| {},
            )
            .unwrap();
        assert!(segments.is_empty());
    }
}

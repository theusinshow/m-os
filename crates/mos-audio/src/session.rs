//! O diretorio de uma sessao de gravacao, e o `session.json` que o descreve.
//!
//! Duas regras que valem mais que o formato.
//!
//! **O diretorio e a fonte de verdade da duracao.** O `session.json` diz como
//! ler os bytes; quantos bytes existem e uma pergunta para o filesystem. Nenhum
//! contador e mantido em disco durante a gravacao, porque um contador e mais uma
//! coisa que pode divergir do que ele conta.
//!
//! **O `session.json` e escrito de forma atomica.** Ele e o unico arquivo cuja
//! corrupcao impediria a recuperacao de tudo o mais — e um arquivo escrito pela
//! metade e exatamente o que uma queda no meio de uma escrita produz.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{chunks::Format, AudioError};

/// Os dois canais, sempre separados.
///
/// MIC e o usuario local, SYSTEM sao os participantes remotos. **Eles nunca sao
/// misturados na gravacao**: a separacao e a unica fonte de "eu prometi" versus
/// "outra pessoa disse", e a regra de decisao do produto e explicita — entre
/// identificar todos os speakers e preservar YOU vs SYSTEM, preserva-se o
/// segundo.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    pub const BOTH: [Channel; 2] = [Channel::Mic, Channel::System];
}

/// Como o canal foi temporizado.
///
/// Gravado porque a Fase 1 mediu que o modo por evento funciona no Windows 11
/// 26200 **mas o relato de campo diz que nem sempre**. Se a captura precisar
/// trocar para polling no meio, o relatorio precisa dizer que trocou — degradar
/// em silencio seria prometer um modo que nao foi usado.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Timing {
    Events,
    Polling,
    /// Comecou por evento e caiu para polling.
    EventsThenPolling,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    pub device: String,
    pub opened: bool,
    pub timing: Timing,
    /// O formato que o motor entregou, que pode nao ser o pedido.
    pub effective_format: Format,
    /// Se o canal SYSTEM esta sendo alimentado pelo keep-alive de silencio.
    ///
    /// A Fase 1 mediu: num endpoint ocioso, 25 s de silencio dao 2.498 pacotes
    /// com o keep-alive e **zero** sem ele. Sem esta linha no arquivo, uma
    /// gravacao feita sem keep-alive seria indistinguivel de uma feita com ele.
    #[serde(default)]
    pub keep_alive: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFile {
    /// Versao do formato deste arquivo. Existe para que uma sessao gravada por
    /// uma versao futura seja RECUSADA com clareza em vez de lida errado.
    pub version: u32,
    pub started_at: String,
    pub format: Format,
    pub chunk_ms: u64,
    pub mic: Option<ChannelInfo>,
    pub system: Option<ChannelInfo>,
}

impl SessionFile {
    pub const VERSION: u32 = 1;
}

/// O diretorio de uma sessao.
#[derive(Clone, Debug)]
pub struct SessionDir {
    root: PathBuf,
}

impl SessionDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn channel(&self, channel: Channel) -> PathBuf {
        self.root.join(channel.folder())
    }

    fn manifest(&self) -> PathBuf {
        self.root.join("session.json")
    }

    pub fn create(&self) -> Result<(), AudioError> {
        fs::create_dir_all(&self.root).map_err(|error| AudioError::Storage {
            path: self.root.display().to_string(),
            detail: error.to_string(),
        })
    }

    /// Grava o manifesto sem nunca deixar um arquivo pela metade.
    ///
    /// Escreve num temporario, sincroniza e so entao renomeia. `rename` no mesmo
    /// volume e atomico: ou o arquivo antigo continua inteiro, ou o novo esta
    /// inteiro. Nunca uma mistura dos dois.
    pub fn write_manifest(&self, session: &SessionFile) -> Result<(), AudioError> {
        self.create()?;
        let final_path = self.manifest();
        let temporary = self.root.join("session.json.tmp");

        let json = serde_json::to_vec_pretty(session).map_err(|error| AudioError::Storage {
            path: final_path.display().to_string(),
            detail: error.to_string(),
        })?;

        {
            use std::io::Write;
            let mut file = fs::File::create(&temporary).map_err(|error| AudioError::Storage {
                path: temporary.display().to_string(),
                detail: error.to_string(),
            })?;
            file.write_all(&json).map_err(|error| AudioError::Storage {
                path: temporary.display().to_string(),
                detail: error.to_string(),
            })?;
            file.sync_all().map_err(|error| AudioError::Storage {
                path: temporary.display().to_string(),
                detail: error.to_string(),
            })?;
        }

        fs::rename(&temporary, &final_path).map_err(|error| AudioError::Storage {
            path: final_path.display().to_string(),
            detail: error.to_string(),
        })
    }

    pub fn read_manifest(&self) -> Result<Option<SessionFile>, AudioError> {
        let path = self.manifest();
        let json = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(AudioError::Storage {
                    path: path.display().to_string(),
                    detail: error.to_string(),
                })
            }
        };
        let session: SessionFile =
            serde_json::from_slice(&json).map_err(|error| AudioError::Storage {
                path: path.display().to_string(),
                detail: format!("manifesto ilegivel: {error}"),
            })?;
        if session.version > SessionFile::VERSION {
            return Err(AudioError::Storage {
                path: path.display().to_string(),
                detail: "esta sessao foi gravada por uma versao mais nova do M/OS.".into(),
            });
        }
        Ok(Some(session))
    }

    /// Apaga o audio desta sessao.
    ///
    /// **So o chamador decide quando.** Nao existe rotina neste crate que apague
    /// audio por conta — foi a instrucao explicita do produto, e a razao e que
    /// uma limpeza automatica de "arquivos orfaos" e exatamente o comportamento
    /// que transformaria 1h18 de reuniao em zero sem ninguem perceber.
    pub fn delete_audio(&self) -> Result<(), AudioError> {
        match fs::remove_dir_all(&self.root) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AudioError::Storage {
                path: self.root.display().to_string(),
                detail: error.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SessionFile {
        SessionFile {
            version: SessionFile::VERSION,
            started_at: "2026-08-18T17:02:11Z".into(),
            format: Format::CAPTURE,
            chunk_ms: crate::chunks::CHUNK_MS,
            mic: Some(ChannelInfo {
                device: "Microfone (Yeti GX)".into(),
                opened: true,
                timing: Timing::Events,
                effective_format: Format::CAPTURE,
                keep_alive: false,
            }),
            system: Some(ChannelInfo {
                device: "Alto-falantes".into(),
                opened: true,
                timing: Timing::Events,
                effective_format: Format::CAPTURE,
                keep_alive: true,
            }),
        }
    }

    #[test]
    fn o_manifesto_vai_e_volta() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        session.write_manifest(&sample()).unwrap();

        let read = session.read_manifest().unwrap().unwrap();
        assert_eq!(read.format, Format::CAPTURE);
        assert!(read.system.unwrap().keep_alive, "o keep-alive fica gravado");
        assert_eq!(read.mic.unwrap().timing, Timing::Events);
    }

    #[test]
    fn escrever_de_novo_nao_deixa_temporario_para_tras() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        session.write_manifest(&sample()).unwrap();
        session.write_manifest(&sample()).unwrap();

        assert!(!session.path().join("session.json.tmp").exists());
        assert!(session.read_manifest().unwrap().is_some());
    }

    #[test]
    fn sessao_sem_manifesto_devolve_nada_e_nao_erro() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("nunca-existiu"));
        assert!(session.read_manifest().unwrap().is_none());
    }

    #[test]
    fn manifesto_de_versao_futura_e_recusado_com_clareza() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        let mut futura = sample();
        futura.version = SessionFile::VERSION + 1;
        session.write_manifest(&futura).unwrap();

        let error = session.read_manifest().unwrap_err();
        assert!(
            format!("{error}").contains("versao mais nova"),
            "o erro precisa dizer o motivo: {error}"
        );
    }

    #[test]
    fn manifesto_corrompido_falha_em_vez_de_devolver_vazio() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        session.create().unwrap();
        fs::write(session.path().join("session.json"), b"{ isto nao e json").unwrap();

        // Devolver `None` aqui faria uma sessao corrompida parecer uma sessao
        // que nunca existiu — e a recuperacao passaria direto por 1h de audio.
        assert!(session.read_manifest().is_err());
    }

    #[test]
    fn apagar_o_audio_e_idempotente() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        session.write_manifest(&sample()).unwrap();

        session.delete_audio().unwrap();
        assert!(!session.path().exists());
        session.delete_audio().unwrap();
    }
}

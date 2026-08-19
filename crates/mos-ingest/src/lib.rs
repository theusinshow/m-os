//! O lado fisico da ingestao: onde o original fica e o que da para ler dele.
//!
//! Duas responsabilidades, e nenhuma outra:
//!
//! * [`FileStore`] recebe bytes, calcula o hash enquanto recebe e coloca o
//!   arquivo no lugar definitivo de forma atomica;
//! * [`extract`] tenta ler texto do que foi guardado, e nunca deixa uma falha
//!   sua atravessar para quem preservou.
//!
//! O crate nao conhece o banco. Ele devolve fatos — hash, tamanho, caminho
//! relativo, texto — e quem os grava e o comando.

mod extract;

pub use extract::{extract, Extraction};

use std::{
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
};

use mos_core::{CoreError, ErrorCode, IngestionId, MAX_INGEST_BYTES};
use sha2::{Digest, Sha256};

/// A pasta, dentro do diretorio de dados, onde os originais vivem.
const STORE_DIR: &str = "drops";
/// Onde os bytes ficam ENQUANTO chegam. Separada de proposito: nada que esteja
/// aqui e considerado preservado, e a abertura seguinte pode limpar sem duvida.
const STAGING_DIR: &str = "drops/.recebendo";

/// O que sobrou de uma transferencia terminada.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preserved {
    pub sha256: String,
    pub byte_size: u64,
    /// Caminho RELATIVO ao diretorio de dados. E ele que vai para o banco: um
    /// caminho absoluto persistido quebraria no dia em que o perfil mudasse de
    /// lugar.
    pub stored_path: String,
}

/// O armazenamento dos originais.
///
/// Uma unica regra atravessa a estrutura inteira: nenhum caminho vem de fora.
/// O que entra e um `IngestionId` (para o staging) ou um hash (para o destino),
/// e os dois sao gerados aqui dentro.
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    /// Ancora o armazenamento no diretorio de dados da aplicacao.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join(STORE_DIR)).map_err(io_error)?;
        fs::create_dir_all(root.join(STAGING_DIR)).map_err(io_error)?;
        Ok(Self { root })
    }

    /// Abre a recepcao dos bytes de uma ingestao.
    pub fn receive(&self, id: IngestionId) -> Result<Transfer, CoreError> {
        let path = self.root.join(STAGING_DIR).join(format!("{id}.parte"));
        let file = File::create(&path).map_err(io_error)?;
        Ok(Transfer {
            file,
            path,
            hasher: Sha256::new(),
            written: 0,
        })
    }

    /// Move o que foi recebido para o lugar definitivo.
    ///
    /// `rename` dentro do mesmo volume e atomico: ou o arquivo esta inteiro no
    /// destino, ou ele nao esta la. Nao existe instante em que o caminho gravado
    /// no banco aponta para meio arquivo.
    ///
    /// Quando o destino JA existe, o arquivo recebido e descartado em vez de
    /// sobrescrever. O nome vem do hash do conteudo: um destino ocupado significa
    /// que aqueles bytes exatos ja estao guardados, e reescrever seria trocar um
    /// arquivo bom por outro identico com uma janela de risco no meio.
    pub fn commit(&self, transfer: Finished, extension: &str) -> Result<Preserved, CoreError> {
        let relative = mos_core::stored_path(&transfer.sha256, extension)?;
        let destination = self.resolve(&relative)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        if destination.exists() {
            let _ = fs::remove_file(&transfer.path);
        } else {
            fs::rename(&transfer.path, &destination).map_err(io_error)?;
        }
        Ok(Preserved {
            sha256: transfer.sha256,
            byte_size: transfer.byte_size,
            stored_path: relative,
        })
    }

    /// O caminho absoluto de um original, validado.
    ///
    /// A guarda existe mesmo com o caminho sendo derivado do hash: se um dia ele
    /// passar a vir de outro lugar — de um import, de um backup, de uma linha
    /// editada a mao —, a fuga falha aqui e nao no filesystem.
    pub fn resolve(&self, relative: &str) -> Result<PathBuf, CoreError> {
        let candidate = Path::new(relative);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Caminho de original invalido.",
                false,
            ));
        }
        if !relative.starts_with(STORE_DIR) {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Caminho de original fora da area de drops.",
                false,
            ));
        }
        Ok(self.root.join(candidate))
    }

    pub fn read(&self, relative: &str) -> Result<Vec<u8>, CoreError> {
        fs::read(self.resolve(relative)?).map_err(io_error)
    }

    pub fn exists(&self, relative: &str) -> bool {
        self.resolve(relative)
            .map(|path| path.exists())
            .unwrap_or(false)
    }

    /// Apaga o que ficou pela metade.
    ///
    /// Chamado na abertura, sobre o staging inteiro. Um arquivo truncado NAO e o
    /// original: mante-lo seria guardar uma mentira sob o nome de uma promessa.
    /// O que sobrevive a interrupcao e a Capture, que continua na Inbox dizendo
    /// o nome do que a pessoa tentou trazer.
    pub fn clear_staging(&self) -> Result<usize, CoreError> {
        let staging = self.root.join(STAGING_DIR);
        if !staging.exists() {
            return Ok(0);
        }
        let mut removed = 0;
        for entry in fs::read_dir(&staging).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            if entry.path().is_file() && fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }
}

/// A identidade de um conteudo que nao passa pelo disco.
///
/// Existe para a URL. O endereco E o conteudo de um Resource de link, entao
/// hashear o endereco normalizado da a ele a mesma identidade que um arquivo
/// tem — e a deduplicacao passa a valer para os dois sem uma segunda tabela,
/// uma segunda consulta ou um segundo conceito.
pub fn hash_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Uma transferencia em curso.
pub struct Transfer {
    file: File,
    path: PathBuf,
    hasher: Sha256,
    written: u64,
}

/// Uma transferencia terminada, ainda no staging.
pub struct Finished {
    path: PathBuf,
    pub sha256: String,
    pub byte_size: u64,
}

impl Transfer {
    /// Escreve mais um pedaco, somando ao hash no caminho.
    ///
    /// O teto e verificado A CADA pedaco, e nao apenas no tamanho declarado pelo
    /// renderer: o tamanho declarado e dado do usuario, e um cliente que mentisse
    /// nele encheria o disco enquanto o M/OS confiava no numero.
    pub fn write(&mut self, chunk: &[u8]) -> Result<(), CoreError> {
        let next = self.written + chunk.len() as u64;
        if next > MAX_INGEST_BYTES {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                format!(
                    "Arquivo maior que o limite de {} MB.",
                    MAX_INGEST_BYTES / (1024 * 1024)
                ),
                false,
            ));
        }
        self.file.write_all(chunk).map_err(io_error)?;
        self.hasher.update(chunk);
        self.written = next;
        Ok(())
    }

    /// Fecha o arquivo garantindo que os bytes chegaram ao disco.
    ///
    /// O `sync_all` e a diferenca entre "o sistema operacional aceitou meus
    /// bytes" e "os bytes existem depois de uma queda de energia" — a mesma
    /// promessa que o `synchronous=FULL` faz do lado do banco (ADR-017).
    pub fn finish(self) -> Result<Finished, CoreError> {
        self.file.sync_all().map_err(io_error)?;
        drop(self.file);
        Ok(Finished {
            path: self.path,
            sha256: format!("{:x}", self.hasher.finalize()),
            byte_size: self.written,
        })
    }

    /// Desiste, apagando o que ja tinha chegado.
    pub fn abort(self) {
        drop(self.file);
        let _ = fs::remove_file(&self.path);
    }
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::new(
        ErrorCode::Io,
        format!("Falha ao gravar o original: {error}"),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, FileStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = FileStore::new(directory.path()).unwrap();
        (directory, store)
    }

    #[test]
    fn o_original_chega_inteiro_e_enderecado_pelo_conteudo() {
        let (directory, store) = store();
        let mut transfer = store.receive(IngestionId::new()).unwrap();
        transfer.write(b"memorial ").unwrap();
        transfer.write(b"estrutural").unwrap();
        let preserved = store.commit(transfer.finish().unwrap(), "pdf").unwrap();

        assert_eq!(preserved.byte_size, 19);
        // sha256("memorial estrutural")
        assert_eq!(preserved.sha256.len(), 64);
        assert!(preserved.stored_path.starts_with("drops/"));
        assert!(preserved.stored_path.ends_with(".pdf"));
        assert_eq!(
            fs::read(directory.path().join(&preserved.stored_path)).unwrap(),
            b"memorial estrutural"
        );
        assert_eq!(store.read(&preserved.stored_path).unwrap().len(), 19);
    }

    #[test]
    fn os_mesmos_bytes_caem_no_mesmo_lugar_uma_vez_so() {
        let (_directory, store) = store();
        let primeiro = guardar(&store, b"identico", "txt");
        let segundo = guardar(&store, b"identico", "txt");

        assert_eq!(primeiro.sha256, segundo.sha256);
        assert_eq!(primeiro.stored_path, segundo.stored_path);
        assert_eq!(store.read(&primeiro.stored_path).unwrap(), b"identico");
    }

    /// O hash avulso e o MESMO que sai de uma transferencia com os mesmos bytes.
    /// Se os dois divergissem, um link e um arquivo com o mesmo conteudo teriam
    /// identidades diferentes e a deduplicacao valeria so para metade do mundo.
    #[test]
    fn o_hash_avulso_bate_com_o_da_transferencia() {
        let (_directory, store) = store();
        assert_eq!(hash_of(b"identico"), guardar(&store, b"identico", "txt").sha256);
    }

    #[test]
    fn nada_escapa_da_area_de_drops() {
        let (_directory, store) = store();
        for caminho in [
            "../segredo.txt",
            "drops/../../segredo.txt",
            "C:/Windows/System32/config",
            "/etc/passwd",
            "outra-pasta/arquivo.pdf",
        ] {
            assert!(
                store.resolve(caminho).is_err(),
                "deixou passar: {caminho}"
            );
        }
        assert!(store.resolve("drops/ab/cd/hash.pdf").is_ok());
    }

    #[test]
    fn o_teto_de_tamanho_e_conferido_pedaco_a_pedaco() {
        let (_directory, store) = store();
        let mut transfer = store.receive(IngestionId::new()).unwrap();
        // Escrever mais que o teto tem que parar mesmo sem ninguem ter declarado
        // tamanho nenhum.
        let pedaco = vec![0u8; 1024 * 1024];
        let mut erro = None;
        for _ in 0..(MAX_INGEST_BYTES / pedaco.len() as u64 + 2) {
            if let Err(falha) = transfer.write(&pedaco) {
                erro = Some(falha);
                break;
            }
        }
        assert!(erro.is_some(), "o teto nao foi aplicado");
        transfer.abort();
    }

    #[test]
    fn o_que_ficou_pela_metade_e_limpo_na_abertura() {
        let (_directory, store) = store();
        let mut transfer = store.receive(IngestionId::new()).unwrap();
        transfer.write(b"metade").unwrap();
        drop(transfer);

        assert_eq!(store.clear_staging().unwrap(), 1);
        assert_eq!(store.clear_staging().unwrap(), 0);
    }

    #[test]
    fn arquivo_sem_extensao_continua_sendo_guardado() {
        let (_directory, store) = store();
        let preserved = guardar(&store, b"bytes quaisquer", "");
        assert!(!preserved.stored_path.ends_with('.'));
        assert!(store.exists(&preserved.stored_path));
    }

    fn guardar(store: &FileStore, bytes: &[u8], extension: &str) -> Preserved {
        let mut transfer = store.receive(IngestionId::new()).unwrap();
        transfer.write(bytes).unwrap();
        store.commit(transfer.finish().unwrap(), extension).unwrap()
    }
}

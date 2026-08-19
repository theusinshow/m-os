//! Gravacao incremental em chunks de PCM cru.
//!
//! Este modulo nao conhece WASAPI e nao chama nada do Windows. E de proposito:
//! ele e a parte que o `cargo test` consegue exercitar em qualquer maquina, e a
//! parte cuja falha custa a gravacao inteira.
//!
//! O formato e PCM cru, sem cabecalho, pela razao registrada na §8.2 do
//! `docs/MEETING-AGENT.md`: um cabecalho RIFF carrega o tamanho dos dados, e ele
//! so e conhecido no fechamento. Um processo que morre no meio deixa um arquivo
//! que MENTE sobre o proprio tamanho, exatamente no cenario em que precisamos
//! confiar nele. PCM cru nao tem o que mentir.

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

/// Quantos bytes um frame ocupa, dado o formato da sessao.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format {
    pub sample_rate: u32,
    pub channels: u16,
    pub bytes_per_sample: u16,
}

impl Format {
    pub fn bytes_per_frame(&self) -> usize {
        self.channels as usize * self.bytes_per_sample as usize
    }

    pub fn frames_to_ms(&self, frames: u64) -> u64 {
        frames * 1000 / self.sample_rate.max(1) as u64
    }
}

/// Escreve um canal em arquivos numerados de duracao fixa.
///
/// Cada rotacao FECHA o arquivo anterior. Fechar e o que devolve os dados ao
/// sistema de arquivos: um `BufWriter` vivo por uma hora guardaria a ultima
/// escrita na memoria do processo, que e exatamente a memoria que some quando o
/// processo morre.
pub struct ChunkWriter {
    directory: PathBuf,
    format: Format,
    frames_per_chunk: u64,
    current: Option<BufWriter<File>>,
    index: u32,
    frames_in_chunk: u64,
    total_frames: u64,
}

impl ChunkWriter {
    pub fn create(directory: &Path, format: Format, chunk_ms: u64) -> std::io::Result<Self> {
        fs::create_dir_all(directory)?;
        let frames_per_chunk = format.sample_rate as u64 * chunk_ms / 1000;
        Ok(Self {
            directory: directory.to_path_buf(),
            format,
            frames_per_chunk: frames_per_chunk.max(1),
            current: None,
            index: 0,
            frames_in_chunk: 0,
            total_frames: 0,
        })
    }

    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    /// Grava bytes ja alinhados a frame.
    ///
    /// Bytes que nao completam um frame sao RECUSADOS em vez de gravados. Meio
    /// frame no arquivo desalinha tudo o que vier depois, e o desalinhamento nao
    /// tem sintoma ate alguem ouvir ruido branco no lugar da reuniao.
    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let frame = self.format.bytes_per_frame();
        if frame == 0 || !bytes.len().is_multiple_of(frame) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("escrita de {} bytes nao e multipla do frame de {frame}", bytes.len()),
            ));
        }

        let mut offset = 0usize;
        while offset < bytes.len() {
            if self.current.is_none() {
                self.open_next()?;
            }
            let room_frames = self.frames_per_chunk - self.frames_in_chunk;
            let available_frames = (bytes.len() - offset) / frame;
            let take_frames = room_frames.min(available_frames as u64) as usize;
            let take_bytes = take_frames * frame;

            if take_bytes > 0 {
                let writer = self.current.as_mut().expect("chunk aberto");
                writer.write_all(&bytes[offset..offset + take_bytes])?;
                offset += take_bytes;
                self.frames_in_chunk += take_frames as u64;
                self.total_frames += take_frames as u64;
            }

            if self.frames_in_chunk >= self.frames_per_chunk {
                self.rotate()?;
            }
        }
        Ok(())
    }

    /// Fecha o chunk em curso. Chamado na rotacao e no fim da gravacao.
    pub fn finish(&mut self) -> std::io::Result<()> {
        if let Some(mut writer) = self.current.take() {
            writer.flush()?;
            // `sync_all` e o que transforma "escrevi" em "esta no disco". Sem
            // ele o dado vive no cache do SO, e uma queda de energia leva junto
            // o ultimo chunk — que e justamente o que a gravacao incremental
            // existe para nao perder.
            writer.get_ref().sync_all()?;
        }
        self.frames_in_chunk = 0;
        Ok(())
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        self.finish()?;
        self.index += 1;
        Ok(())
    }

    fn open_next(&mut self) -> std::io::Result<()> {
        let path = self.directory.join(format!("{:06}.pcm", self.index));
        let file = File::create(path)?;
        self.current = Some(BufWriter::with_capacity(64 * 1024, file));
        self.frames_in_chunk = 0;
        Ok(())
    }
}

impl Drop for ChunkWriter {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// O que a recuperacao encontra em disco.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovered {
    pub chunks: u32,
    /// Frames INTEIROS. Um chunk cortado no meio de um frame conta so ate o
    /// ultimo frame completo — a `Recovered` nunca devolve um numero que o
    /// arquivo nao sustenta.
    pub frames: u64,
    pub bytes: u64,
    /// Bytes que sobraram alem do ultimo frame inteiro. Sempre 0 numa gravacao
    /// encerrada normalmente; diferente de 0 significa queda no meio de uma
    /// escrita, e e um fato a relatar, nao a esconder.
    pub trailing_bytes: u64,
}

/// Varre um diretorio de canal e mede o que existe.
///
/// O DIRETORIO e a fonte de verdade da duracao. Nenhum contador e mantido em
/// disco durante a gravacao, porque um contador e mais uma coisa que pode
/// divergir do que ele conta.
pub fn recover(directory: &Path, format: Format) -> std::io::Result<Recovered> {
    let frame = format.bytes_per_frame().max(1) as u64;
    let mut names: Vec<PathBuf> = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "pcm"))
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    names.sort();

    let mut recovered = Recovered {
        chunks: 0,
        frames: 0,
        bytes: 0,
        trailing_bytes: 0,
    };
    for path in names {
        let size = fs::metadata(&path)?.len();
        recovered.chunks += 1;
        recovered.bytes += size;
        recovered.frames += size / frame;
        recovered.trailing_bytes += size % frame;
    }
    Ok(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format() -> Format {
        Format {
            sample_rate: 16_000,
            channels: 1,
            bytes_per_sample: 2,
        }
    }

    #[test]
    fn rotaciona_no_limite_do_chunk() {
        let dir = tempfile::tempdir().unwrap();
        // 100 ms a 16 kHz mono i16 = 1600 frames = 3200 bytes por chunk.
        let mut writer = ChunkWriter::create(dir.path(), format(), 100).unwrap();
        writer.write(&vec![0u8; 3200 * 3]).unwrap();
        writer.finish().unwrap();

        let recovered = recover(dir.path(), format()).unwrap();
        assert_eq!(recovered.chunks, 3);
        assert_eq!(recovered.frames, 1600 * 3);
        assert_eq!(recovered.trailing_bytes, 0);
    }

    #[test]
    fn escrita_menor_que_o_chunk_nao_rotaciona() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = ChunkWriter::create(dir.path(), format(), 100).unwrap();
        writer.write(&vec![0u8; 800]).unwrap();
        writer.finish().unwrap();

        assert_eq!(recover(dir.path(), format()).unwrap().chunks, 1);
    }

    #[test]
    fn escrita_desalinhada_e_recusada() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = ChunkWriter::create(dir.path(), format(), 100).unwrap();
        // 3 bytes num formato de 2 bytes por frame: meio frame.
        assert!(writer.write(&[0, 0, 0]).is_err());
    }

    #[test]
    fn chunk_truncado_conta_ate_o_ultimo_frame_inteiro() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = ChunkWriter::create(dir.path(), format(), 100).unwrap();
        writer.write(&vec![0u8; 3200]).unwrap();
        writer.finish().unwrap();
        drop(writer);

        // Simula a queda: um byte a mais no arquivo, como se a escrita tivesse
        // sido cortada no meio de um frame.
        let path = dir.path().join("000000.pcm");
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[0]).unwrap();
        drop(file);

        let recovered = recover(dir.path(), format()).unwrap();
        assert_eq!(recovered.frames, 1600, "o frame incompleto nao conta");
        assert_eq!(recovered.trailing_bytes, 1, "e o resto e relatado");
    }

    #[test]
    fn diretorio_inexistente_devolve_vazio_e_nao_erro() {
        let dir = tempfile::tempdir().unwrap();
        let recovered = recover(&dir.path().join("nao-existe"), format()).unwrap();
        assert_eq!(recovered.frames, 0);
        assert_eq!(recovered.chunks, 0);
    }

    #[test]
    fn duracao_vem_dos_frames() {
        assert_eq!(format().frames_to_ms(16_000), 1000);
        assert_eq!(format().frames_to_ms(8_000), 500);
    }
}

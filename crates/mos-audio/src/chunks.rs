//! Gravacao incremental em chunks de PCM cru.
//!
//! Nao conhece WASAPI e nao chama nada do Windows — e a parte que o `cargo test`
//! exercita em qualquer maquina, e a parte cuja falha custa a gravacao inteira.
//!
//! **PCM cru, e nao WAV** (`MEETING-AGENT.md` §8.2). O cabecalho RIFF carrega o
//! tamanho dos dados, e ele so e conhecido no fechamento: um processo que morre
//! no meio deixa um arquivo que MENTE sobre o proprio tamanho, exatamente no
//! cenario em que precisamos confiar nele. PCM cru nao tem o que mentir.

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::AudioError;

/// O formato dos bytes gravados. Vive uma vez no `session.json`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub sample_rate: u32,
    pub channels: u16,
    pub bytes_per_sample: u16,
}

impl Format {
    /// O que a captura pede ao motor de audio: 16 kHz, mono, i16.
    ///
    /// E o que o Whisper consome. Guardar 48 kHz estereo float custaria 24x o
    /// disco — medido na Fase 1 — para informacao que nenhum consumidor le, e o
    /// audio e apagado depois do processamento de qualquer forma.
    pub const CAPTURE: Self = Self {
        sample_rate: 16_000,
        channels: 1,
        bytes_per_sample: 2,
    };

    pub fn bytes_per_frame(&self) -> usize {
        self.channels as usize * self.bytes_per_sample as usize
    }

    pub fn frames_to_ms(&self, frames: u64) -> i64 {
        (frames * 1000 / self.sample_rate.max(1) as u64) as i64
    }
}

/// Quanto tempo cabe num arquivo.
///
/// O chunk define a JANELA DE PERDA. 10 s a 16 kHz mono i16 = 320 kB, e o
/// arquivo e fechado e sincronizado a cada rotacao. Uma reuniao de 1h20 da 480
/// chunks por canal — numero que o filesystem lida sem cerimonia e que a
/// recuperacao varre em milissegundos.
///
/// Chunks de 1 s dariam 4.800 arquivos por canal para ganhar 9 segundos de
/// janela; chunks de 60 s trocariam um minuto de gravacao por menos arquivos.
pub const CHUNK_MS: u64 = 10_000;

/// Escreve um canal em arquivos numerados de duracao fixa.
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
    pub fn create(directory: &Path, format: Format, chunk_ms: u64) -> Result<Self, AudioError> {
        fs::create_dir_all(directory).map_err(|error| AudioError::Storage {
            path: directory.display().to_string(),
            detail: error.to_string(),
        })?;
        Ok(Self {
            directory: directory.to_path_buf(),
            format,
            frames_per_chunk: (format.sample_rate as u64 * chunk_ms / 1000).max(1),
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
    /// frame desalinha tudo o que vier depois, e o desalinhamento nao tem
    /// sintoma ate alguem ouvir ruido branco no lugar da reuniao.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), AudioError> {
        let frame = self.format.bytes_per_frame();
        // `% != 0` e nao `is_multiple_of`: este ultimo so estabilizou no Rust
        // 1.87, e o workspace declara `rust-version = 1.85`. Usa-lo aqui
        // quebraria o build de quem respeitasse o MSRV declarado.
        if frame == 0 || bytes.len() % frame != 0 {
            return Err(AudioError::Misaligned {
                bytes: bytes.len(),
                frame,
            });
        }

        let mut offset = 0usize;
        while offset < bytes.len() {
            if self.current.is_none() {
                self.open_next()?;
            }
            let room = self.frames_per_chunk - self.frames_in_chunk;
            let available = ((bytes.len() - offset) / frame) as u64;
            let take_frames = room.min(available);
            let take_bytes = take_frames as usize * frame;

            if take_bytes > 0 {
                let writer = self.current.as_mut().expect("chunk aberto");
                writer
                    .write_all(&bytes[offset..offset + take_bytes])
                    .map_err(|error| self.storage_error(error))?;
                offset += take_bytes;
                self.frames_in_chunk += take_frames;
                self.total_frames += take_frames;
            }

            if self.frames_in_chunk >= self.frames_per_chunk {
                self.rotate()?;
            }
        }
        Ok(())
    }

    /// Fecha o chunk em curso, garantindo que ele chegou ao disco.
    pub fn finish(&mut self) -> Result<(), AudioError> {
        if let Some(mut writer) = self.current.take() {
            writer.flush().map_err(|error| self.storage_error(error))?;
            // `sync_all` e o que transforma "escrevi" em "esta no disco". Sem
            // ele o dado vive no cache do SO, e uma queda de energia leva junto
            // o ultimo chunk — que e justamente o que a gravacao incremental
            // existe para nao perder.
            writer
                .get_ref()
                .sync_all()
                .map_err(|error| self.storage_error(error))?;
        }
        self.frames_in_chunk = 0;
        Ok(())
    }

    fn rotate(&mut self) -> Result<(), AudioError> {
        self.finish()?;
        self.index += 1;
        Ok(())
    }

    fn open_next(&mut self) -> Result<(), AudioError> {
        let path = self.directory.join(format!("{:06}.pcm", self.index));
        let file = File::create(&path).map_err(|error| AudioError::Storage {
            path: path.display().to_string(),
            detail: error.to_string(),
        })?;
        self.current = Some(BufWriter::with_capacity(64 * 1024, file));
        self.frames_in_chunk = 0;
        Ok(())
    }

    fn storage_error(&self, error: std::io::Error) -> AudioError {
        AudioError::Storage {
            path: self.directory.display().to_string(),
            detail: error.to_string(),
        }
    }
}

impl Drop for ChunkWriter {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// O que a recuperacao encontra em disco.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovered {
    pub chunks: u32,
    /// Frames INTEIROS. Um chunk cortado no meio de um frame conta so ate o
    /// ultimo frame completo — nunca devolve um numero que o arquivo nao
    /// sustenta.
    pub frames: u64,
    pub bytes: u64,
    /// Bytes alem do ultimo frame inteiro. Zero numa gravacao encerrada
    /// normalmente; diferente de zero significa queda no meio de uma escrita, e
    /// e um fato a relatar, nao a esconder.
    pub trailing_bytes: u64,
}

impl Recovered {
    pub fn duration_ms(&self, format: Format) -> i64 {
        format.frames_to_ms(self.frames)
    }
}

/// Varre um diretorio de canal e mede o que existe.
///
/// **O DIRETORIO e a fonte de verdade da duracao.** Nenhum contador e mantido em
/// disco durante a gravacao, porque um contador e mais uma coisa que pode
/// divergir do que ele conta.
pub fn recover(directory: &Path, format: Format) -> Result<Recovered, AudioError> {
    let frame = format.bytes_per_frame().max(1) as u64;
    let mut paths: Vec<PathBuf> = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "pcm"))
            .collect(),
        // Diretorio ausente e "nao gravou nada", e nao um erro. Uma reuniao que
        // caiu antes do primeiro chunk precisa ser recuperavel como zero.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(AudioError::Storage {
                path: directory.display().to_string(),
                detail: error.to_string(),
            })
        }
    };
    paths.sort();

    let mut recovered = Recovered::default();
    for path in paths {
        let size = fs::metadata(&path)
            .map_err(|error| AudioError::Storage {
                path: path.display().to_string(),
                detail: error.to_string(),
            })?
            .len();
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

    #[test]
    fn rotaciona_no_limite_do_chunk() {
        let dir = tempfile::tempdir().unwrap();
        // 100 ms a 16 kHz mono i16 = 1600 frames = 3200 bytes por chunk.
        let mut writer = ChunkWriter::create(dir.path(), Format::CAPTURE, 100).unwrap();
        writer.write(&vec![0u8; 3200 * 3]).unwrap();
        writer.finish().unwrap();

        let recovered = recover(dir.path(), Format::CAPTURE).unwrap();
        assert_eq!(recovered.chunks, 3);
        assert_eq!(recovered.frames, 1600 * 3);
        assert_eq!(recovered.trailing_bytes, 0);
    }

    #[test]
    fn escrita_desalinhada_e_recusada() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = ChunkWriter::create(dir.path(), Format::CAPTURE, 100).unwrap();
        assert!(matches!(
            writer.write(&[0, 0, 0]),
            Err(AudioError::Misaligned { .. })
        ));
    }

    #[test]
    fn chunk_truncado_conta_ate_o_ultimo_frame_inteiro() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = ChunkWriter::create(dir.path(), Format::CAPTURE, 100).unwrap();
        writer.write(&vec![0u8; 3200]).unwrap();
        writer.finish().unwrap();
        drop(writer);

        // Simula a queda: um byte a mais, como se a escrita tivesse sido cortada
        // no meio de um frame.
        let path = dir.path().join("000000.pcm");
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&[0]).unwrap();
        drop(file);

        let recovered = recover(dir.path(), Format::CAPTURE).unwrap();
        assert_eq!(recovered.frames, 1600, "o frame incompleto nao conta");
        assert_eq!(recovered.trailing_bytes, 1, "e o resto e relatado");
    }

    #[test]
    fn diretorio_inexistente_e_zero_e_nao_erro() {
        let dir = tempfile::tempdir().unwrap();
        let recovered = recover(&dir.path().join("nao-existe"), Format::CAPTURE).unwrap();
        assert_eq!(recovered.frames, 0);
        assert_eq!(recovered.chunks, 0);
    }

    #[test]
    fn a_duracao_vem_dos_frames() {
        assert_eq!(Format::CAPTURE.frames_to_ms(16_000), 1000);
        let recovered = Recovered {
            frames: 16_000 * 90,
            ..Default::default()
        };
        assert_eq!(recovered.duration_ms(Format::CAPTURE), 90_000);
    }

    #[test]
    fn escrita_parcial_seguida_de_fim_fecha_um_chunk_curto() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = ChunkWriter::create(dir.path(), Format::CAPTURE, 10_000).unwrap();
        // Meio segundo num chunk de dez.
        writer.write(&vec![0u8; 16_000]).unwrap();
        writer.finish().unwrap();

        let recovered = recover(dir.path(), Format::CAPTURE).unwrap();
        assert_eq!(recovered.chunks, 1);
        assert_eq!(recovered.frames, 8_000);
        assert_eq!(recovered.duration_ms(Format::CAPTURE), 500);
    }
}

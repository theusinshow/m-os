//! Monta um WAV a partir dos chunks de um canal.
//!
//! Os chunks sao PCM cru — sem cabecalho, de proposito (§8.2). Todo consumidor
//! de audio, do Whisper ao player, quer um arquivo com cabecalho. Este modulo e
//! a ponte, e ele **nao** chama `ffmpeg`: a captura ja entrega 16 kHz mono i16,
//! que e exatamente o que um WAV PCM guarda. Um processo externo aqui seria uma
//! dependencia nova para escrever 44 bytes.
//!
//! O cabecalho e escrito por ULTIMO, depois de os dados terem ido para o disco.
//! E a mesma razao que fez os chunks serem PCM cru: o tamanho so e conhecido no
//! fim, e um cabecalho escrito antes seria um cabecalho que mente se a escrita
//! for interrompida.

use std::{
    fs::{self, File},
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{chunks::Format, session::Channel, AudioError, SessionDir};

const HEADER_BYTES: u32 = 44;

/// Escreve o canal inteiro como um WAV em `destination`.
///
/// Devolve quantos frames foram escritos. Zero significa que o canal nao gravou
/// nada — e o arquivo resultante e um WAV valido e vazio, e nao um arquivo
/// quebrado: um consumidor que receba zero segundos precisa poder abrir e ver
/// zero segundos.
pub fn export_channel(
    session_root: &Path,
    channel: Channel,
    destination: &Path,
) -> Result<u64, AudioError> {
    let session = SessionDir::new(session_root);
    let format = session
        .read_manifest()?
        .map(|manifest| manifest.format)
        .unwrap_or(Format::CAPTURE);

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| storage(parent, error))?;
    }

    let mut output = BufWriter::new(File::create(destination).map_err(|error| storage(destination, error))?);
    // Espaco reservado. O cabecalho real entra no fim, quando os tamanhos
    // existirem.
    output
        .write_all(&[0u8; HEADER_BYTES as usize])
        .map_err(|error| storage(destination, error))?;

    let mut paths: Vec<_> = match fs::read_dir(session.channel(channel)) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "pcm"))
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(storage(&session.channel(channel), error)),
    };
    paths.sort();

    let frame = format.bytes_per_frame().max(1);
    let mut data_bytes = 0u64;
    let mut buffer = vec![0u8; 256 * 1024];

    for path in paths {
        let mut file = File::open(&path).map_err(|error| storage(&path, error))?;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| storage(&path, error))?;
            if read == 0 {
                break;
            }
            // Um chunk truncado no meio de um frame perde o resto. Copiar o meio
            // frame desalinharia tudo o que viesse depois dele no arquivo final,
            // e o sintoma seria ruido branco a partir daquele ponto.
            let usable = read - (read % frame);
            if usable > 0 {
                output
                    .write_all(&buffer[..usable])
                    .map_err(|error| storage(destination, error))?;
                data_bytes += usable as u64;
            }
            if usable != read {
                break;
            }
        }
    }

    output.flush().map_err(|error| storage(destination, error))?;
    let mut file = output
        .into_inner()
        .map_err(|error| storage(destination, error.into_error()))?;

    file.seek(SeekFrom::Start(0))
        .map_err(|error| storage(destination, error))?;
    file.write_all(&header(format, data_bytes))
        .map_err(|error| storage(destination, error))?;
    file.sync_all().map_err(|error| storage(destination, error))?;

    Ok(data_bytes / frame as u64)
}

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

/// O cabecalho RIFF/WAVE de 44 bytes, para PCM inteiro.
fn header(format: Format, data_bytes: u64) -> [u8; HEADER_BYTES as usize] {
    let channels = format.channels;
    let bits = format.bytes_per_sample * 8;
    let block_align = format.bytes_per_frame() as u16;
    let byte_rate = format.sample_rate * block_align as u32;
    // `data_bytes` cabe em u32 porque o proprio formato WAV nao vai alem: uma
    // reuniao de 74 horas a 16 kHz mono i16 estouraria, e ai o problema seria
    // outro.
    let data = data_bytes.min(u32::MAX as u64) as u32;

    let mut out = [0u8; HEADER_BYTES as usize];
    out[0..4].copy_from_slice(b"RIFF");
    out[4..8].copy_from_slice(&(36 + data).to_le_bytes());
    out[8..12].copy_from_slice(b"WAVE");
    out[12..16].copy_from_slice(b"fmt ");
    out[16..20].copy_from_slice(&16u32.to_le_bytes()); // tamanho do bloco fmt
    out[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
    out[22..24].copy_from_slice(&channels.to_le_bytes());
    out[24..28].copy_from_slice(&format.sample_rate.to_le_bytes());
    out[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    out[32..34].copy_from_slice(&block_align.to_le_bytes());
    out[34..36].copy_from_slice(&bits.to_le_bytes());
    out[36..40].copy_from_slice(b"data");
    out[40..44].copy_from_slice(&data.to_le_bytes());
    out
}

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

fn storage(path: &Path, error: std::io::Error) -> AudioError {
    AudioError::Storage {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chunks::ChunkWriter, session::SessionFile, CHUNK_MS};

    fn sessao(root: &Path) -> SessionDir {
        let session = SessionDir::new(root);
        session
            .write_manifest(&SessionFile {
                version: SessionFile::VERSION,
                started_at: "2026-08-19T14:00:00Z".into(),
                format: Format::CAPTURE,
                chunk_ms: CHUNK_MS,
                mic: None,
                system: None,
            })
            .unwrap();
        session
    }

    fn grava(session: &SessionDir, channel: Channel, bytes: usize) {
        let mut writer =
            ChunkWriter::create(&session.channel(channel), Format::CAPTURE, 100).unwrap();
        writer.write(&vec![7u8; bytes]).unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn o_wav_junta_os_chunks_na_ordem_e_fecha_o_cabecalho() {
        let dir = tempfile::tempdir().unwrap();
        let session = sessao(&dir.path().join("0198"));
        // 1 s a 16 kHz mono i16 = 32.000 bytes, em chunks de 100 ms.
        grava(&session, Channel::Mic, 32_000);

        let destino = dir.path().join("saida/mic.wav");
        let frames = export_channel(session.path(), Channel::Mic, &destino).unwrap();
        assert_eq!(frames, 16_000);

        let bytes = fs::read(&destino).unwrap();
        assert_eq!(bytes.len(), 44 + 32_000);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");

        // Os campos que um decodificador le antes de tocar qualquer amostra.
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 36 + 32_000);
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), 32_000);
        assert_eq!(u16::from_le_bytes(bytes[32..34].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 32_000);

        // E os dados sao os que foram gravados, na ordem.
        assert!(bytes[44..].iter().all(|byte| *byte == 7));
    }

    #[test]
    fn um_canal_vazio_vira_um_wav_valido_de_zero_segundos() {
        let dir = tempfile::tempdir().unwrap();
        let session = sessao(&dir.path().join("0198"));

        let destino = dir.path().join("saida/system.wav");
        assert_eq!(
            export_channel(session.path(), Channel::System, &destino).unwrap(),
            0
        );

        let bytes = fs::read(&destino).unwrap();
        assert_eq!(bytes.len(), 44, "so o cabecalho");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 0);
    }

    #[test]
    fn o_frame_incompleto_de_um_chunk_truncado_nao_entra() {
        let dir = tempfile::tempdir().unwrap();
        let session = sessao(&dir.path().join("0198"));
        grava(&session, Channel::Mic, 3_200);

        // A queda deixou um byte solto no fim do unico chunk.
        let chunk = session.channel(Channel::Mic).join("000000.pcm");
        let mut file = fs::OpenOptions::new().append(true).open(&chunk).unwrap();
        file.write_all(&[9]).unwrap();
        drop(file);

        let destino = dir.path().join("saida/mic.wav");
        let frames = export_channel(session.path(), Channel::Mic, &destino).unwrap();
        assert_eq!(frames, 1_600, "o meio frame fica de fora");
        assert_eq!(fs::metadata(&destino).unwrap().len(), 44 + 3_200);
    }

    #[test]
    fn o_wav_respeita_o_formato_do_manifesto() {
        let dir = tempfile::tempdir().unwrap();
        let session = SessionDir::new(dir.path().join("0198"));
        let format = Format {
            sample_rate: 8_000,
            channels: 1,
            bytes_per_sample: 2,
        };
        session
            .write_manifest(&SessionFile {
                version: SessionFile::VERSION,
                started_at: String::new(),
                format,
                chunk_ms: CHUNK_MS,
                mic: None,
                system: None,
            })
            .unwrap();
        let mut writer =
            ChunkWriter::create(&session.channel(Channel::Mic), format, 100).unwrap();
        writer.write(&vec![0u8; 16_000]).unwrap();
        writer.finish().unwrap();

        let destino = dir.path().join("mic.wav");
        export_channel(session.path(), Channel::Mic, &destino).unwrap();
        let bytes = fs::read(&destino).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            8_000,
            "o cabecalho precisa dizer a taxa real, nao a padrao"
        );
    }

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
}

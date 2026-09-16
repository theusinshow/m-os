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

    let mut output =
        BufWriter::new(File::create(destination).map_err(|error| storage(destination, error))?);
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

    output
        .flush()
        .map_err(|error| storage(destination, error))?;
    let mut file = output
        .into_inner()
        .map_err(|error| storage(destination, error.into_error()))?;

    file.seek(SeekFrom::Start(0))
        .map_err(|error| storage(destination, error))?;
    file.write_all(&header(format, data_bytes))
        .map_err(|error| storage(destination, error))?;
    file.sync_all()
        .map_err(|error| storage(destination, error))?;

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
    if dados.len() < 2 {
        return Ok((frames, 1.0));
    }

    let amostras: Vec<i16> = dados
        .chunks_exact(2)
        .map(|par| i16::from_le_bytes([par[0], par[1]]))
        .collect();

    // `calibrar`, e nao `ganho_para`: o segundo e so o chute inicial, e ele erra
    // para baixo por causa do joelho. Ver a nota na propria funcao.
    let ganho = calibrar(&amostras);
    if ganho == 1.0 {
        return Ok((frames, 1.0));
    }

    for (par, amostra) in dados.chunks_exact_mut(2).zip(amostras) {
        par.copy_from_slice(&com_ganho(amostra, ganho).to_le_bytes());
    }
    fs::write(destination, &bytes).map_err(|error| storage(destination, error))?;
    Ok((frames, ganho))
}

/// As amostras de um canal dentro de `[start_ms, end_ms)`.
///
/// Le os chunks em ordem, pula ate o primeiro frame da faixa e para no ultimo.
/// **E o que torna o corte nao destrutivo**: os arquivos em disco continuam
/// inteiros, e so a leitura escolhe o que conta. Um chunk truncado no meio de um
/// frame perde o resto, pela mesma razao do `export_channel`.
///
/// So para PCM mono i16, que e o formato da captura — outro formato devolve
/// erro em vez de ler bytes com a regua errada.
pub fn read_channel_range(
    session_root: &Path,
    channel: Channel,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<i16>, AudioError> {
    let session = SessionDir::new(session_root);
    let format = session
        .read_manifest()?
        .map(|manifest| manifest.format)
        .unwrap_or(Format::CAPTURE);
    if format.channels != 1 || format.bytes_per_sample != 2 {
        return Err(AudioError::Storage {
            path: session_root.display().to_string(),
            detail: "o recorte so le PCM mono de 16 bits".into(),
        });
    }

    let rate = format.sample_rate as i64;
    let first = (start_ms.max(0) * rate) / 1000;
    let last = if end_ms == i64::MAX {
        i64::MAX
    } else {
        (end_ms.max(0) * rate) / 1000
    };
    if last <= first {
        return Ok(Vec::new());
    }

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

    let mut samples = Vec::new();
    let mut position: i64 = 0;
    'files: for path in paths {
        let bytes = fs::read(&path).map_err(|error| storage(&path, error))?;
        let usable = bytes.len() - bytes.len() % 2;
        let count = (usable / 2) as i64;
        if position + count <= first {
            position += count;
            continue;
        }
        let skip = (first - position).max(0) as usize;
        for pair in bytes[..usable].chunks_exact(2).skip(skip) {
            if first + samples.len() as i64 >= last {
                break 'files;
            }
            samples.push(i16::from_le_bytes([pair[0], pair[1]]));
        }
        position += count;
        if usable != bytes.len() {
            break;
        }
    }
    // O laco acima conta `samples` a partir de `first`, entao o limite certo e
    // `last - first` amostras — reforcado aqui para o caso de o primeiro arquivo
    // lido comecar exatamente em `first`.
    let wanted = if last == i64::MAX {
        samples.len()
    } else {
        ((last - first) as usize).min(samples.len())
    };
    samples.truncate(wanted);
    Ok(samples)
}

/// Um canal inteiro ou uma faixa dele, com o nivel corrigido para o
/// transcritor, num WAV.
///
/// Os timestamps que o transcritor devolve sao relativos ao inicio DESTE
/// arquivo: quem chama soma `start_ms` para voltar a regua da reuniao.
pub fn export_channel_range_normalized(
    session_root: &Path,
    channel: Channel,
    destination: &Path,
    start_ms: i64,
    end_ms: i64,
) -> Result<(u64, f32), AudioError> {
    let samples = read_channel_range(session_root, channel, start_ms, end_ms)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| storage(parent, error))?;
    }
    if samples.is_empty() {
        fs::write(destination, header(Format::CAPTURE, 0))
            .map_err(|error| storage(destination, error))?;
        return Ok((0, 1.0));
    }
    let gain = calibrar(&samples);
    let bytes = wav_bytes(&samples, gain);
    fs::write(destination, &bytes).map_err(|error| storage(destination, error))?;
    Ok((samples.len() as u64, gain))
}

/// Qual audio ouvir num trecho.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipMode {
    /// Os dois canais somados — a reuniao como ela soou.
    Both,
    /// So o microfone: "Voce".
    Mic,
    /// So o audio do sistema: "Remoto".
    System,
}

/// Um trecho curto, pronto para tocar, como bytes de WAV.
///
/// **O renderer nunca recebe caminho de arquivo** (§18): ele recebe o WAV em
/// memoria. Por isso o trecho tem teto — 60 s a 16 kHz mono sao 1,9 MB, o que
/// atravessa a ponte sem cerimonia.
pub fn clip_wav_bytes(
    session_root: &Path,
    mode: ClipMode,
    start_ms: i64,
    duration_ms: i64,
) -> Result<Vec<u8>, AudioError> {
    let duration_ms = duration_ms.clamp(1_000, 60_000);
    let end_ms = start_ms.max(0) + duration_ms;
    let read = |channel: Channel| -> Result<Vec<i16>, AudioError> {
        let samples = read_channel_range(session_root, channel, start_ms, end_ms)?;
        let gain = calibrar(&samples);
        Ok(samples.into_iter().map(|s| com_ganho(s, gain)).collect())
    };
    let samples = match mode {
        ClipMode::Mic => read(Channel::Mic)?,
        ClipMode::System => read(Channel::System)?,
        ClipMode::Both => {
            let mic = read(Channel::Mic)?;
            let system = read(Channel::System)?;
            let length = mic.len().max(system.len());
            (0..length)
                .map(|index| {
                    let a = *mic.get(index).unwrap_or(&0) as i32;
                    let b = *system.get(index).unwrap_or(&0) as i32;
                    (a + b).clamp(i16::MIN as i32, i16::MAX as i32) as i16
                })
                .collect()
        }
    };
    Ok(wav_bytes(&samples, 1.0))
}

fn wav_bytes(samples: &[i16], gain: f32) -> Vec<u8> {
    let data_bytes = samples.len() as u64 * 2;
    let mut out = Vec::with_capacity(HEADER_BYTES as usize + data_bytes as usize);
    out.extend_from_slice(&header(Format::CAPTURE, data_bytes));
    for sample in samples {
        out.extend_from_slice(&com_ganho(*sample, gain).to_le_bytes());
    }
    out
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
/// Teto de ~24 dB. Um canal quase mudo nao vira um canal de chiado amplificado.
///
/// Era 20 dB, e 20 dB nao bastava: o ponto que o dono validou na bancada exigiu
/// 10,7x, e o teto anterior cortava exatamente ele.
const GANHO_MAXIMO: f32 = 16.0;

/// Quantas vezes recalibrar o ganho olhando o resultado.
///
/// Tres converge com folga: no microfone que originou a regra, a terceira volta
/// ja chega a menos de 0,1 dB do alvo.
const CALIBRACOES: usize = 3;

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

/// O RMS que sobra depois de aplicar este ganho.
pub fn rms_com_ganho(amostras: &[i16], ganho: f32) -> f32 {
    if amostras.is_empty() {
        return 0.0;
    }
    let mut soma = 0f64;
    for amostra in amostras {
        let valor = com_ganho(*amostra, ganho) as f64;
        soma += valor * valor;
    }
    (soma / amostras.len() as f64).sqrt() as f32
}

/// O ganho que de fato leva o canal ao alvo.
///
/// **Medir o resultado, e nao confiar na regra de tres.** O `ganho_para` calcula
/// como se o ganho fosse linear, e ele nao e: o joelho `tanh` comprime os picos e
/// derruba o RMS que sai — no microfone que originou esta regra, mirar -25 dBFS
/// pela conta linear aterrissava em -26,5 dBFS.
///
/// E esse 1,3 dB nao e detalhe: na bancada, ele foi a diferenca entre 30 frases
/// reais e 12, e entre acertar "armadura da laje" e perder o trecho inteiro.
pub fn calibrar(amostras: &[i16]) -> f32 {
    let mut ganho = ganho_para(rms_com_ganho(amostras, 1.0));
    if ganho == 1.0 {
        return 1.0;
    }
    for _ in 0..CALIBRACOES {
        let obtido = rms_com_ganho(amostras, ganho);
        if obtido <= 0.0 || obtido >= ALVO_RMS {
            break;
        }
        ganho = (ganho * ALVO_RMS / obtido).min(GANHO_MAXIMO);
        if ganho >= GANHO_MAXIMO {
            break;
        }
    }
    ganho
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
mod range_tests {
    use super::*;
    use crate::{chunks::ChunkWriter, session::SessionFile, CHUNK_MS};

    /// Um canal com amostras que dizem a propria posicao: a amostra `n` vale
    /// `n % 30000`. Assim o recorte prova onde comecou e onde acabou.
    fn canal(root: &Path, channel: Channel, seconds: usize, chunk_ms: u64) {
        let session = SessionDir::new(root);
        session
            .write_manifest(&SessionFile {
                version: SessionFile::VERSION,
                started_at: "2026-09-16T14:00:00Z".into(),
                format: Format::CAPTURE,
                chunk_ms: CHUNK_MS,
                mic: None,
                system: None,
            })
            .unwrap();
        let mut writer =
            ChunkWriter::create(&session.channel(channel), Format::CAPTURE, chunk_ms).unwrap();
        let bytes: Vec<u8> = (0..seconds * 16_000)
            .flat_map(|n| ((n % 30_000) as i16).to_le_bytes())
            .collect();
        writer.write(&bytes).unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn o_recorte_comeca_e_termina_no_frame_certo_atravessando_chunks() {
        let dir = tempfile::tempdir().unwrap();
        // Chunks de 1 s: a faixa 2,5 s..4,25 s atravessa tres arquivos.
        canal(dir.path(), Channel::Mic, 6, 1_000);
        let samples = read_channel_range(dir.path(), Channel::Mic, 2_500, 4_250).unwrap();
        assert_eq!(samples.len(), 28_000);
        assert_eq!(samples[0] as i64, 40_000 % 30_000);
        assert_eq!(*samples.last().unwrap() as i64, (40_000 + 27_999) % 30_000);
    }

    #[test]
    fn faixa_alem_do_fim_devolve_o_que_existe() {
        let dir = tempfile::tempdir().unwrap();
        canal(dir.path(), Channel::System, 2, 1_000);
        let samples = read_channel_range(dir.path(), Channel::System, 1_000, i64::MAX).unwrap();
        assert_eq!(samples.len(), 16_000);
        assert!(
            read_channel_range(dir.path(), Channel::System, 5_000, 6_000)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn o_trecho_para_ouvir_e_um_wav_valido_e_curto() {
        let dir = tempfile::tempdir().unwrap();
        canal(dir.path(), Channel::Mic, 3, 1_000);
        canal(dir.path(), Channel::System, 3, 1_000);
        let bytes = clip_wav_bytes(dir.path(), ClipMode::Both, 1_000, 1_000).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), HEADER_BYTES as usize + 16_000 * 2);
    }

    #[test]
    fn exportar_a_faixa_gera_wav_com_o_tamanho_da_faixa() {
        let dir = tempfile::tempdir().unwrap();
        canal(dir.path(), Channel::Mic, 4, 1_000);
        let destino = dir.path().join("out").join("mic.wav");
        let (frames, _) =
            export_channel_range_normalized(dir.path(), Channel::Mic, &destino, 1_000, 3_000)
                .unwrap();
        assert_eq!(frames, 32_000);
        assert_eq!(fs::metadata(&destino).unwrap().len(), 44 + 64_000);
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
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            36 + 32_000
        );
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(
            u32::from_le_bytes(bytes[28..32].try_into().unwrap()),
            32_000
        );
        assert_eq!(u16::from_le_bytes(bytes[32..34].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            32_000
        );

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
        let mut writer = ChunkWriter::create(&session.channel(Channel::Mic), format, 100).unwrap();
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
        // Um canal quase mudo nao vira chiado amplificado: o teto corta.
        assert_eq!(ganho_para(1.0), GANHO_MAXIMO);
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
        assert!(
            ganho > 1.0,
            "canal baixo deveria receber ganho, veio {ganho}"
        );

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
        assert!(
            pico(&fs::read(chunk).unwrap()) <= 301,
            "o chunk foi alterado"
        );
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

    #[test]
    fn calibrar_leva_o_canal_baixo_ao_alvo_de_verdade() {
        // Uma senoide baixa, como o microfone da reuniao que originou a regra.
        let amostras: Vec<i16> = (0..16_000)
            .map(|i| ((i as f32 * 0.05).sin() * 300.0) as i16)
            .collect();

        let chute = ganho_para(rms_com_ganho(&amostras, 1.0));
        let calibrado = calibrar(&amostras);

        // O chute linear fica CURTO, e e por isso que a calibracao existe.
        assert!(
            calibrado > chute,
            "a calibracao deveria corrigir o chute para cima: {chute} -> {calibrado}"
        );

        // E o que sai chega ao alvo, dentro de 1 dB.
        let obtido = rms_com_ganho(&amostras, calibrado);
        let erro_db = 20.0 * (obtido / ALVO_RMS).log10();
        assert!(erro_db.abs() < 1.0, "ficou a {erro_db:.2} dB do alvo");
    }

    #[test]
    fn calibrar_deixa_em_paz_quem_ja_esta_alto() {
        let amostras: Vec<i16> = (0..16_000)
            .map(|i| ((i as f32 * 0.05).sin() * 8_000.0) as i16)
            .collect();
        assert_eq!(calibrar(&amostras), 1.0);
    }

    #[test]
    fn calibrar_respeita_o_teto_num_canal_quase_mudo() {
        let amostras: Vec<i16> = (0..16_000)
            .map(|i| ((i as f32 * 0.05).sin() * 3.0) as i16)
            .collect();
        assert_eq!(calibrar(&amostras), GANHO_MAXIMO);
    }
}

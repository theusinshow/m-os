//! A varredura incremental dos transcripts.
//!
//! São 507 MB em 18 projetos nesta máquina. Reler tudo a cada tique de trinta
//! segundos gastaria mais disco por hora do que o M/OS inteiro gasta por mês, e
//! por isso cada arquivo carrega um [`Ponteiro`] com onde a leitura parou.
//!
//! # O ponteiro é otimização, não corretude
//!
//! Se todo o estado de offset for perdido — banco novo, migração, arquivo
//! movido — uma varredura completa produz **exatamente o mesmo resultado**. A
//! corretude mora na chave primária do `requestId`, do lado de quem persiste, e
//! não aqui. Essa separação é deliberada: um bug de offset vira lentidão, e
//! nunca número errado.

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::{parse_linha, Evento};

/// Onde a leitura de um arquivo parou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ponteiro {
    pub caminho: String,
    /// O byte seguinte à última linha COMPLETA já lida.
    pub offset: u64,
    pub tamanho: u64,
    /// Segundos desde a época. Guardado para poder pular o arquivo sem abri-lo.
    pub mtime: i64,
}

/// Uma raiz de transcripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fonte {
    pub nome: String,
    pub raiz: PathBuf,
}

impl Fonte {
    /// A raiz do Claude Code, se ela existir nesta máquina.
    ///
    /// Devolver `None` é resposta legítima e não erro: numa máquina sem Claude
    /// Code não há fonte, e sem fonte a faixa não monta — ela não aparece vazia
    /// esperando um dado que nunca virá.
    pub fn claude_code() -> Option<Self> {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)?;
        let raiz = home.join(".claude").join("projects");
        raiz.is_dir().then(|| Self {
            nome: "Claude Code".to_string(),
            raiz,
        })
    }
}

/// O resultado de uma passada.
#[derive(Debug, Default)]
pub struct Varredura {
    pub eventos: Vec<Evento>,
    /// Só dos arquivos que MUDARAM. Quem persiste faz upsert e deixa o resto
    /// como está.
    pub ponteiros: Vec<Ponteiro>,
    /// Linhas que não viraram evento nem por serem de usuário: JSON quebrado.
    /// Contado, e não abortado — ver [`crate::parse_linha`].
    pub linhas_ilegiveis: u64,
    /// Arquivos que nem abriram. Vai para o diagnóstico, não para o número.
    pub arquivos_ilegiveis: Vec<String>,
}

/// Lê o que cresceu desde a última passada.
///
/// Recebe os ponteiros conhecidos indexados por caminho, e devolve só o delta.
pub fn varrer(raiz: &Path, conhecidos: &HashMap<String, Ponteiro>) -> Varredura {
    let mut resultado = Varredura::default();
    for caminho in transcripts(raiz) {
        let chave = caminho.to_string_lossy().to_string();
        match ler_delta(&caminho, conhecidos.get(&chave), &mut resultado) {
            Ok(Some(ponteiro)) => resultado.ponteiros.push(ponteiro),
            Ok(None) => {}
            Err(_) => resultado.arquivos_ilegiveis.push(chave),
        }
    }
    resultado
}

/// Todos os `.jsonl` sob a raiz, um nível de projeto abaixo.
///
/// Travessia própria em vez de `walkdir`: a estrutura é conhecida e rasa
/// (`projects/<projeto>/<sessao>.jsonl`), e uma dependência a menos num crate
/// que já lê arquivo de terceiro é uma superfície a menos.
fn transcripts(raiz: &Path) -> Vec<PathBuf> {
    let mut encontrados = Vec::new();
    let Ok(projetos) = std::fs::read_dir(raiz) else {
        return encontrados;
    };
    for projeto in projetos.flatten() {
        let Ok(sessoes) = std::fs::read_dir(projeto.path()) else {
            continue;
        };
        for sessao in sessoes.flatten() {
            let caminho = sessao.path();
            if caminho.extension().is_some_and(|ext| ext == "jsonl") {
                encontrados.push(caminho);
            }
        }
    }
    encontrados.sort();
    encontrados
}

fn ler_delta(
    caminho: &Path,
    conhecido: Option<&Ponteiro>,
    resultado: &mut Varredura,
) -> std::io::Result<Option<Ponteiro>> {
    let metadata = std::fs::metadata(caminho)?;
    let tamanho = metadata.len();
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|instante| instante.duration_since(UNIX_EPOCH).ok())
        .map(|desde| desde.as_secs() as i64)
        .unwrap_or_default();

    // Nada mudou: nem abre. É o caminho que percorre 500 MB em milissegundos.
    if let Some(ponteiro) = conhecido {
        if ponteiro.tamanho == tamanho && ponteiro.mtime == mtime {
            return Ok(None);
        }
    }

    // Encolheu — reescrito ou truncado. Recomeçar é seguro porque o `requestId`
    // recusa o que já foi contado.
    let inicio = match conhecido {
        Some(ponteiro) if ponteiro.offset <= tamanho => ponteiro.offset,
        _ => 0,
    };

    let mut arquivo = File::open(caminho)?;
    arquivo.seek(SeekFrom::Start(inicio))?;
    let mut leitor = BufReader::new(arquivo);

    let mut offset = inicio;
    let mut linha = Vec::new();
    loop {
        linha.clear();
        let lidos = leitor.read_until(b'\n', &mut linha)?;
        if lidos == 0 {
            break;
        }
        // Linha sem `\n` no fim é uma linha AINDA SENDO ESCRITA. Consumi-la
        // faria o offset avançar sobre metade de um JSON, e a outra metade
        // chegaria órfã na passada seguinte. Ela fica para a próxima.
        if !linha.ends_with(b"\n") {
            break;
        }
        offset += lidos as u64;
        let texto = String::from_utf8_lossy(&linha);
        let texto = texto.trim();
        if texto.is_empty() {
            continue;
        }
        match parse_linha(texto) {
            Some(evento) => resultado.eventos.push(evento),
            // Linha de usuário e linha quebrada caem no mesmo `None`. Distingui-las
            // exigiria um segundo parse do que já se sabe não ser request, e o
            // contador serve para notar uma quebra de formato, não para auditar.
            None => resultado.linhas_ilegiveis += 1,
        }
    }

    Ok(Some(Ponteiro {
        caminho: caminho.to_string_lossy().to_string(),
        offset,
        tamanho,
        mtime,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn linha(request_id: &str, timestamp: &str) -> String {
        format!(
            r#"{{"requestId":"{request_id}","timestamp":"{timestamp}","type":"assistant","message":{{"model":"claude-opus-5","usage":{{"input_tokens":10,"output_tokens":20}}}}}}"#
        )
    }

    /// Cria `raiz/projeto/sessao.jsonl` com o conteúdo dado.
    fn escrever(raiz: &Path, conteudo: &str) -> PathBuf {
        let projeto = raiz.join("um-projeto");
        std::fs::create_dir_all(&projeto).unwrap();
        let caminho = projeto.join("sessao.jsonl");
        std::fs::write(&caminho, conteudo).unwrap();
        caminho
    }

    fn indexar(varredura: &Varredura) -> HashMap<String, Ponteiro> {
        varredura
            .ponteiros
            .iter()
            .map(|ponteiro| (ponteiro.caminho.clone(), ponteiro.clone()))
            .collect()
    }

    #[test]
    fn a_primeira_passada_le_o_arquivo_inteiro() {
        let dir = tempfile::tempdir().unwrap();
        escrever(
            dir.path(),
            &format!(
                "{}\n{}\n",
                linha("a", "2026-08-29T03:00:00Z"),
                linha("b", "2026-08-29T03:01:00Z")
            ),
        );
        let varredura = varrer(dir.path(), &HashMap::new());
        assert_eq!(varredura.eventos.len(), 2);
        assert_eq!(varredura.ponteiros.len(), 1);
    }

    #[test]
    fn crescer_le_so_o_delta() {
        let dir = tempfile::tempdir().unwrap();
        let caminho = escrever(
            dir.path(),
            &format!("{}\n", linha("a", "2026-08-29T03:00:00Z")),
        );
        let primeira = varrer(dir.path(), &HashMap::new());
        assert_eq!(primeira.eventos.len(), 1);

        let mut arquivo = std::fs::OpenOptions::new()
            .append(true)
            .open(&caminho)
            .unwrap();
        writeln!(arquivo, "{}", linha("b", "2026-08-29T03:01:00Z")).unwrap();
        drop(arquivo);

        let segunda = varrer(dir.path(), &indexar(&primeira));
        assert_eq!(segunda.eventos.len(), 1, "só o que foi acrescentado");
        assert_eq!(segunda.eventos[0].request_id, "b");
    }

    #[test]
    fn truncar_rele_do_zero() {
        let dir = tempfile::tempdir().unwrap();
        escrever(
            dir.path(),
            &format!(
                "{}\n{}\n",
                linha("a", "2026-08-29T03:00:00Z"),
                linha("b", "2026-08-29T03:01:00Z")
            ),
        );
        let primeira = varrer(dir.path(), &HashMap::new());
        assert_eq!(primeira.eventos.len(), 2);

        escrever(
            dir.path(),
            &format!("{}\n", linha("c", "2026-08-29T04:00:00Z")),
        );
        let segunda = varrer(dir.path(), &indexar(&primeira));
        assert_eq!(segunda.eventos.len(), 1);
        assert_eq!(segunda.eventos[0].request_id, "c");
    }

    #[test]
    fn arquivo_parado_nao_e_reaberto() {
        let dir = tempfile::tempdir().unwrap();
        escrever(
            dir.path(),
            &format!("{}\n", linha("a", "2026-08-29T03:00:00Z")),
        );
        let primeira = varrer(dir.path(), &HashMap::new());
        let segunda = varrer(dir.path(), &indexar(&primeira));
        assert!(segunda.eventos.is_empty());
        assert!(
            segunda.ponteiros.is_empty(),
            "arquivo que não mudou não devolve ponteiro para regravar"
        );
    }

    #[test]
    fn linha_pela_metade_fica_para_a_proxima_passada() {
        // O Claude Code escreve enquanto o M/OS lê. Metade de um JSON não pode
        // avançar o offset, senão a outra metade chega órfã.
        let dir = tempfile::tempdir().unwrap();
        let completa = linha("a", "2026-08-29T03:00:00Z");
        let caminho = escrever(
            dir.path(),
            &format!("{completa}\n{{\"requestId\":\"b\",\"time"),
        );
        let primeira = varrer(dir.path(), &HashMap::new());
        assert_eq!(primeira.eventos.len(), 1);

        std::fs::write(
            &caminho,
            format!("{completa}\n{}\n", linha("b", "2026-08-29T03:01:00Z")),
        )
        .unwrap();
        let segunda = varrer(dir.path(), &indexar(&primeira));
        assert_eq!(segunda.eventos.len(), 1);
        assert_eq!(
            segunda.eventos[0].request_id, "b",
            "a linha completou e foi lida inteira"
        );
    }

    #[test]
    fn raiz_inexistente_nao_e_erro() {
        let dir = tempfile::tempdir().unwrap();
        let varredura = varrer(&dir.path().join("nao-existe"), &HashMap::new());
        assert!(varredura.eventos.is_empty());
        assert!(varredura.arquivos_ilegiveis.is_empty());
    }
}

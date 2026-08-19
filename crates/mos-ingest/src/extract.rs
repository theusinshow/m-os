//! Leitura de conteudo do que ja foi preservado.
//!
//! Tudo aqui roda DEPOIS de o original estar no disco e de a entidade existir.
//! Por isso nenhuma funcao deste modulo devolve `Err`: falhar em ler um PDF e um
//! resultado — [`ExtractionState::Failed`] —, e nao um erro que alguem la em cima
//! deva tratar. Quem preservou ja cumpriu a promessa.

use mos_core::{clamp_extracted_text, DetectedKind, ExtractionState, MAX_EXTRACTED_CHARS};

/// Teto para carregar um arquivo inteiro na memoria so para ler texto.
///
/// Acima disso a leitura e recusada em vez de tentada: o arquivo continua
/// guardado e reencontravel pelo nome, e um processo que estoura a memoria
/// tentando indexar um arquivo de 400 MB seria pior que um arquivo sem indice.
const MAX_READ_BYTES: usize = 64 * 1024 * 1024;

/// O que a leitura encontrou.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Extraction {
    pub state: ExtractionState,
    pub text: String,
    pub page_count: Option<u32>,
    pub error: String,
}

impl Extraction {
    fn unsupported(reason: &str) -> Self {
        Self {
            state: ExtractionState::Unsupported,
            text: String::new(),
            page_count: None,
            error: reason.to_owned(),
        }
    }

    fn empty() -> Self {
        Self {
            state: ExtractionState::Empty,
            text: String::new(),
            page_count: None,
            error: String::new(),
        }
    }

    fn failed(reason: String) -> Self {
        Self {
            state: ExtractionState::Failed,
            text: String::new(),
            page_count: None,
            error: reason,
        }
    }
}

/// Tenta ler texto util dos bytes de um original.
///
/// Nunca entra em panico e nunca propaga erro: o pior caso e devolver
/// `Failed` com o motivo, que e exatamente o que a fila de reprocessamento
/// futura vai precisar ler.
pub fn extract(kind: DetectedKind, bytes: &[u8]) -> Extraction {
    if bytes.len() > MAX_READ_BYTES {
        return Extraction::unsupported("Arquivo grande demais para leitura de conteudo.");
    }
    match kind {
        DetectedKind::Pdf => extract_pdf(bytes),
        DetectedKind::Text | DetectedKind::Markdown | DetectedKind::Data | DetectedKind::Code => {
            extract_plain(bytes)
        }
        DetectedKind::Image => {
            // Sem OCR nesta versao, e a ausencia e deliberada: uma dependencia de
            // visao computacional para um caso que ainda nao apareceu seria
            // exatamente a infraestrutura especulativa que o §31 do briefing
            // proibe. O estado registrado e o que da a fila ao OCR futuro.
            Extraction::unsupported("Imagem: leitura de texto exige OCR, que ainda nao existe.")
        }
        DetectedKind::Archive => {
            Extraction::unsupported("Arquivo compactado: o conteudo nao e lido nesta versao.")
        }
        DetectedKind::Url => Extraction::unsupported("Link: o conteudo remoto nao e baixado."),
        DetectedKind::Unknown => Extraction::unsupported("Formato desconhecido."),
    }
}

/// Texto de PDF, pelo mesmo `lopdf` que o gerador de faturas ja compila.
///
/// Um PDF escaneado — imagem de pagina, sem camada de texto — devolve `Empty`, e
/// nao `Failed`. A diferenca importa: `Empty` e uma resposta correta sobre o
/// arquivo, e e dela que sai a fila do OCR quando ele existir.
fn extract_pdf(bytes: &[u8]) -> Extraction {
    let document = match lopdf::Document::load_mem(bytes) {
        Ok(document) => document,
        Err(error) => return Extraction::failed(format!("PDF ilegivel: {error}")),
    };
    let pages = document.get_pages();
    let page_count = pages.len() as u32;
    let numbers: Vec<u32> = pages.keys().copied().collect();

    let mut text = String::new();
    let mut failures = 0usize;
    for number in &numbers {
        // Pagina a pagina: uma pagina corrompida no meio de um memorial de 300
        // nao pode custar as outras 299.
        match document.extract_text(&[*number]) {
            Ok(page) => {
                text.push_str(&page);
                text.push('\n');
            }
            Err(_) => failures += 1,
        }
        if text.chars().count() >= MAX_EXTRACTED_CHARS {
            break;
        }
    }

    let text = clamp_extracted_text(&text);
    if text.is_empty() {
        return Extraction {
            page_count: Some(page_count),
            ..if failures == numbers.len() && failures > 0 {
                Extraction::failed("Nenhuma pagina do PDF pode ser lida.".to_owned())
            } else {
                Extraction::empty()
            }
        };
    }
    Extraction {
        state: ExtractionState::Done,
        text,
        page_count: Some(page_count),
        error: if failures > 0 {
            format!("{failures} pagina(s) ilegivel(is).")
        } else {
            String::new()
        },
    }
}

/// Texto de arquivo textual.
///
/// Recusa o que nao e texto de verdade em vez de indexar lixo: um `.csv` que na
/// verdade e binario produziria um indice cheio de sequencias sem sentido, e uma
/// busca que devolve resultados por acidente e pior que uma que nao devolve.
fn extract_plain(bytes: &[u8]) -> Extraction {
    let bytes = strip_bom(bytes);
    if bytes.iter().take(8192).any(|byte| *byte == 0) {
        return Extraction::unsupported("O arquivo diz ser texto, mas contem bytes nulos.");
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            let text = clamp_extracted_text(text);
            if text.is_empty() {
                Extraction::empty()
            } else {
                Extraction {
                    state: ExtractionState::Done,
                    text,
                    page_count: None,
                    error: String::new(),
                }
            }
        }
        // Latin-1 ainda sai de exportadores antigos, e recusar um `.txt` por
        // causa de um "ç" seria perder texto que da para ler.
        Err(_) => {
            let text = clamp_extracted_text(&String::from_utf8_lossy(bytes));
            if text.is_empty() {
                Extraction::empty()
            } else {
                Extraction {
                    state: ExtractionState::Done,
                    text,
                    page_count: None,
                    error: "Texto com bytes fora de UTF-8; lido do jeito que deu.".to_owned(),
                }
            }
        }
    }
}

fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texto_simples_e_lido() {
        let resultado = extract(DetectedKind::Text, "Conversar com Joao".as_bytes());
        assert_eq!(resultado.state, ExtractionState::Done);
        assert_eq!(resultado.text, "Conversar com Joao");
    }

    #[test]
    fn bom_nao_entra_no_indice() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"orcamento");
        assert_eq!(extract(DetectedKind::Data, &bytes).text, "orcamento");
    }

    #[test]
    fn binario_disfarcado_de_texto_e_recusado() {
        let resultado = extract(DetectedKind::Data, &[0x50, 0x4B, 0x03, 0x04, 0x00, 0x01]);
        assert_eq!(resultado.state, ExtractionState::Unsupported);
        assert!(resultado.text.is_empty());
    }

    #[test]
    fn texto_fora_de_utf8_ainda_e_lido() {
        // "café" em Latin-1.
        let resultado = extract(DetectedKind::Text, &[b'c', b'a', b'f', 0xE9]);
        assert_eq!(resultado.state, ExtractionState::Done);
        assert!(resultado.text.starts_with("caf"));
        assert!(!resultado.error.is_empty());
    }

    #[test]
    fn arquivo_vazio_e_vazio_e_nao_falha() {
        assert_eq!(extract(DetectedKind::Text, b"").state, ExtractionState::Empty);
        assert_eq!(
            extract(DetectedKind::Markdown, b"   \n\n ").state,
            ExtractionState::Empty
        );
    }

    /// A regra que sustenta o §14: formato desconhecido nao e erro.
    #[test]
    fn formato_desconhecido_nao_falha() {
        for kind in [
            DetectedKind::Unknown,
            DetectedKind::Archive,
            DetectedKind::Image,
            DetectedKind::Url,
        ] {
            let resultado = extract(kind, b"qualquer coisa");
            assert_eq!(resultado.state, ExtractionState::Unsupported);
            assert!(!resultado.error.is_empty(), "sem motivo registrado");
        }
    }

    #[test]
    fn pdf_quebrado_falha_sem_derrubar_nada() {
        let resultado = extract(DetectedKind::Pdf, b"%PDF-1.7 mentira");
        assert_eq!(resultado.state, ExtractionState::Failed);
        assert!(!resultado.error.is_empty());
    }

    #[test]
    fn arquivo_grande_demais_nao_e_lido() {
        let gigante = vec![b'a'; MAX_READ_BYTES + 1];
        assert_eq!(
            extract(DetectedKind::Text, &gigante).state,
            ExtractionState::Unsupported
        );
    }

    /// O caminho feliz do PDF, contra um PDF gerado pelo proprio M/OS.
    #[test]
    fn pdf_de_verdade_devolve_texto_e_paginas() {
        let bytes = pdf_de_uma_pagina("Fundacao em radier");
        let resultado = extract(DetectedKind::Pdf, &bytes);
        assert_eq!(resultado.page_count, Some(1));
        assert_eq!(resultado.state, ExtractionState::Done);
        assert!(
            resultado.text.contains("radier"),
            "texto extraido: {:?}",
            resultado.text
        );
    }

    /// Um PDF minimo escrito a mao: uma pagina, uma fonte interna, um texto.
    ///
    /// Escrito aqui em vez de vir de um arquivo de fixture porque um binario no
    /// repositorio e uma coisa que ninguem revisa.
    fn pdf_de_uma_pagina(texto: &str) -> Vec<u8> {
        let conteudo = format!("BT /F1 24 Tf 72 700 Td ({texto}) Tj ET");
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        let mut push = |pdf: &mut String, offsets: &mut Vec<usize>, objeto: &str| {
            offsets.push(pdf.len());
            pdf.push_str(objeto);
        };
        push(&mut pdf, &mut offsets, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        push(
            &mut pdf,
            &mut offsets,
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
        );
        push(
            &mut pdf,
            &mut offsets,
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n",
        );
        push(
            &mut pdf,
            &mut offsets,
            "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );
        push(
            &mut pdf,
            &mut offsets,
            &format!(
                "5 0 obj\n<< /Length {} >>\nstream\n{conteudo}\nendstream\nendobj\n",
                conteudo.len()
            ),
        );
        let xref = pdf.len();
        pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1));
        for offset in &offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        ));
        pdf.into_bytes()
    }
}

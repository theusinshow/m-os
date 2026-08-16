//! Relatorio e fatura em PDF.
//!
//! Herdado do CronoCAD (ADR-032) com uma unica mudanca de fundo: a tela envia as
//! celulas JA FORMATADAS — datas, duracoes, moeda — e aqui so as posicionamos
//! numa A4. Formatar em dois lugares acabaria com a fatura dizendo um numero e a
//! tela dizendo outro, e o usuario descobriria pelo cliente.
//!
//! Layout defensivo de proposito: colunas fixas, truncamento e paginacao, com as
//! fontes internas do PDF. Nada de fonte externa — um PDF que depende de arquivo
//! instalado na maquina e um PDF que abre errado na maquina do cliente.

use printpdf::{BuiltinFont, Mm, PdfDocument};
use serde::Deserialize;

/// Dados prontos para renderizacao (todas as celulas ja formatadas).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPdfData {
    pub title: String,
    pub period: String,
    /// Pares (rotulo, valor) do resumo.
    pub totals: Vec<[String; 2]>,
    /// Cabecalhos das 4 colunas da tabela.
    pub columns: [String; 4],
    /// Linhas da tabela (4 celulas cada).
    pub rows: Vec<[String; 4]>,
}

/// Dados de uma fatura por cliente.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceData {
    pub issuer_name: String,
    pub issuer_document: String,
    pub issuer_contact: String,
    pub client_name: String,
    pub period: String,
    pub columns: [String; 4],
    pub rows: Vec<[String; 4]>,
    pub total_label: String,
    pub total_value: String,
}

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const LEFT: f32 = 18.0;
const BOTTOM: f32 = 18.0;
const ROW_H: f32 = 6.5;
/// x das 4 colunas, em mm.
const COLS: [f32; 4] = [18.0, 45.0, 120.0, 155.0];
/// Quantos caracteres cabem em cada coluna antes de o texto invadir a seguinte.
const WIDTHS: [usize; 4] = [22, 40, 16, 16];

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Gera o PDF do relatorio e devolve os bytes.
pub fn build_report(report: &ReportPdfData) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) = PdfDocument::new(&report.title, Mm(PAGE_W), Mm(PAGE_H), "Camada 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|error| error.to_string())?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|error| error.to_string())?;

    let mut layer = doc.get_page(page1).get_layer(layer1);

    let mut y = PAGE_H - 20.0;
    layer.use_text(&report.title, 16.0, Mm(LEFT), Mm(y), &bold);
    y -= 7.0;
    layer.use_text(&report.period, 10.0, Mm(LEFT), Mm(y), &font);
    y -= 9.0;

    for [label, value] in &report.totals {
        layer.use_text(format!("{label}: {value}"), 10.0, Mm(LEFT), Mm(y), &font);
        y -= 5.5;
    }
    y -= 4.0;

    let draw_header = |layer: &printpdf::PdfLayerReference, y: f32| {
        for (index, title) in report.columns.iter().enumerate() {
            layer.use_text(title, 9.0, Mm(COLS[index]), Mm(y), &bold);
        }
    };
    draw_header(&layer, y);
    y -= ROW_H;

    for row in &report.rows {
        if y < BOTTOM {
            let (page, layer_index) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Camada");
            layer = doc.get_page(page).get_layer(layer_index);
            y = PAGE_H - 20.0;
            draw_header(&layer, y);
            y -= ROW_H;
        }
        for (index, cell) in row.iter().enumerate() {
            layer.use_text(
                truncate(cell, WIDTHS[index]),
                9.0,
                Mm(COLS[index]),
                Mm(y),
                &font,
            );
        }
        y -= ROW_H;
    }

    save(doc)
}

/// Gera a fatura por cliente: cabecalho do emissor, itens e total.
pub fn build_invoice(invoice: &InvoiceData) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) = PdfDocument::new("Fatura", Mm(PAGE_W), Mm(PAGE_H), "Camada 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|error| error.to_string())?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|error| error.to_string())?;

    let mut layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PAGE_H - 20.0;

    layer.use_text("FATURA", 18.0, Mm(LEFT), Mm(y), &bold);
    y -= 9.0;

    if !invoice.issuer_name.is_empty() {
        layer.use_text(&invoice.issuer_name, 11.0, Mm(LEFT), Mm(y), &bold);
        y -= 5.0;
    }
    for line in [&invoice.issuer_document, &invoice.issuer_contact] {
        if !line.is_empty() {
            layer.use_text(line, 9.0, Mm(LEFT), Mm(y), &font);
            y -= 4.5;
        }
    }
    y -= 4.0;

    layer.use_text(
        format!("Cliente: {}", invoice.client_name),
        10.0,
        Mm(LEFT),
        Mm(y),
        &bold,
    );
    y -= 5.0;
    layer.use_text(&invoice.period, 9.0, Mm(LEFT), Mm(y), &font);
    y -= 9.0;

    let draw_header = |layer: &printpdf::PdfLayerReference, y: f32| {
        for (index, title) in invoice.columns.iter().enumerate() {
            layer.use_text(title, 9.0, Mm(COLS[index]), Mm(y), &bold);
        }
    };
    draw_header(&layer, y);
    y -= ROW_H;

    for row in &invoice.rows {
        // Folga maior que no relatorio: o total precisa caber DEPOIS da ultima
        // linha, e uma fatura cujo total abriu sozinho na pagina 2 parece erro.
        if y < BOTTOM + 14.0 {
            let (page, layer_index) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Camada");
            layer = doc.get_page(page).get_layer(layer_index);
            y = PAGE_H - 20.0;
            draw_header(&layer, y);
            y -= ROW_H;
        }
        for (index, cell) in row.iter().enumerate() {
            layer.use_text(
                truncate(cell, WIDTHS[index]),
                9.0,
                Mm(COLS[index]),
                Mm(y),
                &font,
            );
        }
        y -= ROW_H;
    }

    y -= 4.0;
    layer.use_text(
        format!("{}: {}", invoice.total_label, invoice.total_value),
        12.0,
        Mm(COLS[2]),
        Mm(y),
        &bold,
    );

    save(doc)
}

fn save(doc: printpdf::PdfDocumentReference) -> Result<Vec<u8>, String> {
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut writer = std::io::BufWriter::new(&mut buffer);
        doc.save(&mut writer).map_err(|error| error.to_string())?;
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: [&str; 4]) -> [String; 4] {
        cells.map(str::to_string)
    }

    /// Um PDF de zero byte, ou que nao comece com `%PDF`, e um arquivo que o
    /// cliente recebe e nao consegue abrir — e o erro so aparece la.
    #[test]
    fn the_invoice_is_a_real_pdf() {
        let bytes = build_invoice(&InvoiceData {
            issuer_name: "Matheus".into(),
            issuer_document: String::new(),
            issuer_contact: String::new(),
            client_name: "Cliente".into(),
            period: "agosto".into(),
            columns: row(["Data", "Project", "Horas", "Valor"]),
            rows: vec![row(["16/08", "043", "2,0 h", "R$ 60,00"])],
            total_label: "Total".into(),
            total_value: "R$ 60,00".into(),
        })
        .unwrap();

        assert!(bytes.starts_with(b"%PDF"), "nao saiu um PDF valido");
    }

    /// Uma celula comprida que nao trunca invade a coluna seguinte, e a fatura
    /// sai com o nome do projeto por cima do valor.
    #[test]
    fn long_cells_are_truncated_instead_of_overlapping() {
        let long = "Project com nome absurdamente comprido que nao cabe";
        assert!(truncate(long, WIDTHS[1]).chars().count() <= WIDTHS[1]);
        assert!(truncate(long, WIDTHS[1]).ends_with('…'));
    }

    /// Muitas linhas precisam virar pagina sem estourar; o teste falha por panico
    /// ou por PDF invalido se a paginacao quebrar.
    #[test]
    fn many_rows_paginate() {
        let rows: Vec<[String; 4]> = (0..200)
            .map(|index| row(["16/08", &format!("Project {index}"), "1,0 h", "R$ 30,00"]))
            .collect();
        let bytes = build_report(&ReportPdfData {
            title: "Relatorio".into(),
            period: "agosto".into(),
            totals: vec![["Total".into(), "R$ 6000,00".into()]],
            columns: row(["Data", "Project", "Horas", "Valor"]),
            rows,
        })
        .unwrap();

        assert!(bytes.starts_with(b"%PDF"));
    }
}

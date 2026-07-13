//! Geracao do PDF de relatorio (secao 13).
//!
//! O frontend ja formata todos os textos (datas, duracoes, moeda) e envia as
//! celulas prontas; aqui apenas as posicionamos em uma pagina A4. Layout simples
//! e defensivo (colunas fixas, truncamento e paginacao) usando fontes internas.

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
// x das 4 colunas (mm).
const COLS: [f32; 4] = [18.0, 45.0, 120.0, 155.0];

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Gera o PDF e retorna os bytes.
pub fn build_report(report: &ReportPdfData) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) =
        PdfDocument::new(&report.title, Mm(PAGE_W), Mm(PAGE_H), "Camada 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let mut layer = doc.get_page(page1).get_layer(layer1);

    // Cabecalho do documento.
    let mut y = PAGE_H - 20.0;
    layer.use_text(&report.title, 16.0, Mm(LEFT), Mm(y), &bold);
    y -= 7.0;
    layer.use_text(&report.period, 10.0, Mm(LEFT), Mm(y), &font);
    y -= 9.0;

    // Resumo.
    for [label, value] in &report.totals {
        layer.use_text(format!("{label}: {value}"), 10.0, Mm(LEFT), Mm(y), &font);
        y -= 5.5;
    }
    y -= 4.0;

    // Cabecalho da tabela.
    let draw_header = |layer: &printpdf::PdfLayerReference, y: f32| {
        for (i, title) in report.columns.iter().enumerate() {
            layer.use_text(title, 9.0, Mm(COLS[i]), Mm(y), &bold);
        }
    };
    draw_header(&layer, y);
    y -= ROW_H;

    // Linhas com paginacao.
    let widths = [22usize, 40, 16, 16];
    for row in &report.rows {
        if y < BOTTOM {
            let (page, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Camada");
            layer = doc.get_page(page).get_layer(layer_idx);
            y = PAGE_H - 20.0;
            draw_header(&layer, y);
            y -= ROW_H;
        }
        for (i, cell) in row.iter().enumerate() {
            layer.use_text(
                truncate(cell, widths[i]),
                9.0,
                Mm(COLS[i]),
                Mm(y),
                &font,
            );
        }
        y -= ROW_H;
    }

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut writer = std::io::BufWriter::new(&mut buf);
        doc.save(&mut writer).map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

/// Gera a fatura por cliente em PDF (cabecalho do emissor + itens + total).
pub fn build_invoice(inv: &InvoiceData) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) =
        PdfDocument::new("Fatura CronoCAD", Mm(PAGE_W), Mm(PAGE_H), "Camada 1");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| e.to_string())?;
    let bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| e.to_string())?;

    let mut layer = doc.get_page(page1).get_layer(layer1);
    let mut y = PAGE_H - 20.0;

    layer.use_text("FATURA", 18.0, Mm(LEFT), Mm(y), &bold);
    y -= 9.0;

    // Emissor.
    if !inv.issuer_name.is_empty() {
        layer.use_text(&inv.issuer_name, 11.0, Mm(LEFT), Mm(y), &bold);
        y -= 5.0;
    }
    for line in [&inv.issuer_document, &inv.issuer_contact] {
        if !line.is_empty() {
            layer.use_text(line, 9.0, Mm(LEFT), Mm(y), &font);
            y -= 4.5;
        }
    }
    y -= 4.0;

    layer.use_text(
        format!("Cliente: {}", inv.client_name),
        10.0,
        Mm(LEFT),
        Mm(y),
        &bold,
    );
    y -= 5.0;
    layer.use_text(&inv.period, 9.0, Mm(LEFT), Mm(y), &font);
    y -= 9.0;

    // Cabecalho da tabela.
    let draw_header = |layer: &printpdf::PdfLayerReference, y: f32| {
        for (i, title) in inv.columns.iter().enumerate() {
            layer.use_text(title, 9.0, Mm(COLS[i]), Mm(y), &bold);
        }
    };
    draw_header(&layer, y);
    y -= ROW_H;

    let widths = [22usize, 40, 16, 16];
    for row in &inv.rows {
        if y < BOTTOM + 14.0 {
            let (page, layer_idx) = doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Camada");
            layer = doc.get_page(page).get_layer(layer_idx);
            y = PAGE_H - 20.0;
            draw_header(&layer, y);
            y -= ROW_H;
        }
        for (i, cell) in row.iter().enumerate() {
            layer.use_text(truncate(cell, widths[i]), 9.0, Mm(COLS[i]), Mm(y), &font);
        }
        y -= ROW_H;
    }

    // Total.
    y -= 4.0;
    layer.use_text(
        format!("{}: {}", inv.total_label, inv.total_value),
        12.0,
        Mm(COLS[2]),
        Mm(y),
        &bold,
    );

    let mut buf: Vec<u8> = Vec::new();
    {
        let mut writer = std::io::BufWriter::new(&mut buf);
        doc.save(&mut writer).map_err(|e| e.to_string())?;
    }
    Ok(buf)
}

/**
 * Servico de nivel de aplicativo: encerramento e exportacao de relatorio em PDF.
 */

import { invokeCommand } from "./tauri";

export function quitApp(): Promise<void> {
  return invokeCommand<void>("quit_app");
}

export interface PendingReminder {
  processName: string;
  displayName: string;
}

/** Lembrete pendente para o widget flutuante recuperar ao carregar. */
export function getPendingReminder(): Promise<PendingReminder | null> {
  return invokeCommand<PendingReminder | null>("get_pending_reminder");
}

export interface ReportPdfData {
  title: string;
  period: string;
  /** Pares (rotulo, valor) do resumo. */
  totals: [string, string][];
  /** Cabecalhos das 4 colunas. */
  columns: [string, string, string, string];
  /** Linhas da tabela (4 celulas cada). */
  rows: [string, string, string, string][];
}

/** Gera e salva o PDF do relatorio. Retorna true se salvo, false se cancelado. */
export function exportReportPdf(
  report: ReportPdfData,
  suggestedName: string,
): Promise<boolean> {
  return invokeCommand<boolean>("export_report_pdf", {
    report,
    suggestedName,
  });
}

export interface InvoiceData {
  issuerName: string;
  issuerDocument: string;
  issuerContact: string;
  clientName: string;
  period: string;
  columns: [string, string, string, string];
  rows: [string, string, string, string][];
  totalLabel: string;
  totalValue: string;
}

/** Gera e salva a fatura por cliente em PDF. */
export function exportInvoicePdf(
  invoice: InvoiceData,
  suggestedName: string,
): Promise<boolean> {
  return invokeCommand<boolean>("export_invoice_pdf", {
    invoice,
    suggestedName,
  });
}

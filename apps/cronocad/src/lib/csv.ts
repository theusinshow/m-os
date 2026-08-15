/**
 * Geracao de CSV. Usa ";" como separador e BOM UTF-8 para abrir corretamente no
 * Excel em pt-BR (acentos e separador de coluna).
 */

function escapeCell(value: string): string {
  if (/[";\n\r]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

const BOM = String.fromCharCode(0xfeff);

export function buildCsv(headers: string[], rows: string[][]): string {
  const lines = [headers, ...rows].map((cols) =>
    cols.map(escapeCell).join(";"),
  );
  return BOM + lines.join("\r\n");
}

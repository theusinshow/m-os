type Cell = string | number | null | undefined;

function csvCell(value: Cell): string {
  if (value === null || value === undefined) return "";
  const str = String(value);
  // Quote when the value contains a delimiter, quote or newline; double inner quotes.
  if (/[",\r\n;]/.test(str)) {
    return `"${str.replace(/"/g, '""')}"`;
  }
  return str;
}

// Builds a CSV body (no BOM). Callers add the UTF-8 BOM in the HTTP response so
// Excel pt-BR opens accented text correctly.
export function toCsv(rows: Cell[][]): string {
  return rows.map((row) => row.map(csvCell).join(",")).join("\r\n");
}

// Plain dot-decimal so spreadsheets parse it as a number regardless of locale.
export function centsToAmount(cents: number): string {
  return (cents / 100).toFixed(2);
}

const CSV_HEADERS = {
  "content-type": "text/csv; charset=utf-8",
} as const;

export function csvResponse(body: string, filename: string): Response {
  return new Response(`﻿${body}`, {
    headers: {
      ...CSV_HEADERS,
      "content-disposition": `attachment; filename="${filename}"`,
    },
  });
}

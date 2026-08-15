export function parseCurrencyToCents(value: FormDataEntryValue | null) {
  const raw = String(value ?? "").trim();
  if (!raw) {
    return 0;
  }

  const negative = raw.startsWith("-");
  const cleaned = raw.replace(/[^\d.,]/g, "");
  const lastSepIndex = Math.max(cleaned.lastIndexOf(","), cleaned.lastIndexOf("."));

  let normalized: string;
  if (lastSepIndex === -1) {
    normalized = cleaned;
  } else {
    // The last separator is a decimal point only when 1-2 digits follow it
    // ("3500,00", "3500.00", "3.50"); otherwise it's a thousands grouping
    // ("3.500", "1.000.000"). This accepts both pt-BR comma and dot decimals.
    const decimalPart = cleaned.slice(lastSepIndex + 1);
    if (decimalPart.length >= 1 && decimalPart.length <= 2) {
      const intPart = cleaned.slice(0, lastSepIndex).replace(/[.,]/g, "");
      normalized = `${intPart}.${decimalPart}`;
    } else {
      normalized = cleaned.replace(/[.,]/g, "");
    }
  }

  const amount = Number(normalized);
  if (!Number.isFinite(amount)) {
    return 0;
  }

  const cents = Math.round(amount * 100);
  return negative ? -cents : cents;
}

// Inverse of parseCurrencyToCents: renders cents as a pt-BR decimal string
// ("350000" -> "3500,00") for prefilling form inputs.
export function centsToInput(cents: number) {
  return (cents / 100).toFixed(2).replace(".", ",");
}

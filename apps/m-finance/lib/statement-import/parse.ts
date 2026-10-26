/**
 * Lê uma fatura de cartão — o `.ofx` que o banco exporta ou o texto copiado do
 * app — e devolve as compras dela, já separadas do que não é compra.
 *
 * É código puro, sem banco: a mesma leitura serve para a prévia e para a
 * gravação, e é ela que os testes cobrem com faturas de verdade.
 */

export type StatementLine = {
  /** Identidade estável da linha: o FITID do OFX, ou um hash do texto colado. */
  externalId: string;
  description: string;
  /** Sempre positivo: o que a compra cobra. */
  amountCents: number;
  /** `yyyy-mm-dd`, quando a fatura informa. */
  purchaseDate: string | null;
  /** Parcela desta fatura (`4` em "4 de 10"); `null` à vista. */
  installmentNumber: number | null;
  installmentTotal: number | null;
};

export type ParsedStatement = {
  format: "ofx" | "text";
  lines: StatementLine[];
  /** O que ainda falta pagar, quando a fatura diz. */
  owedCents: number | null;
  /**
   * Pagamentos antecipados e créditos que abateram esta fatura. No OFX sai da
   * conta `compras − saldo devedor`, que pega também o "ajuste a crédito".
   */
  creditsCents: number;
  /** Vencimento informado no texto (`yyyy-mm-dd`), se houver. */
  dueDate: string | null;
  /** Pagamentos, estornos e créditos que não viraram compra — para mostrar, não somar. */
  ignored: { description: string; amountCents: number }[];
};

const MONTHS_PT = [
  "janeiro",
  "fevereiro",
  "marco",
  "abril",
  "maio",
  "junho",
  "julho",
  "agosto",
  "setembro",
  "outubro",
  "novembro",
  "dezembro",
];

function fold(text: string) {
  return text
    .trim()
    .toLocaleLowerCase("pt-BR")
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "");
}

function toCents(value: string) {
  return Math.round(Number(value) * 100);
}

/** "R$ 1.234,56" → 123456. Aceita sem o "R$". */
function brlToCents(value: string) {
  const digits = value.replace(/[^\d,]/g, "").replace(",", ".");
  return Math.round(Number(digits) * 100);
}

/** Tira a parcela do fim da descrição: "Airbnb - Parcela 1/6", "iFood - NuPay - 2/3". */
export function splitInstallment(raw: string) {
  const match =
    /\s*-?\s*(?:parcela\s+)?(\d{1,2})\s*(?:\/|de)\s*(\d{1,2})\s*$/i.exec(raw) ??
    /\s*-?\s*parcela\s+(\d{1,2})\s*(?:\/|de)\s*(\d{1,2})/i.exec(raw);
  if (!match) return { description: raw.trim(), number: null, total: null };
  const number = Number(match[1]);
  const total = Number(match[2]);
  if (total < 2 || number < 1 || number > total) {
    return { description: raw.trim(), number: null, total: null };
  }
  const description = (raw.slice(0, match.index) + raw.slice(match.index + match[0].length))
    .replace(/\s*-\s*parcelamento de compra\s*-\s*/i, " ")
    .replace(/^parcelamento de compra\s*-\s*/i, "")
    .replace(/\s*-\s*$/, "")
    .trim();
  return { description: description || raw.trim(), number, total };
}

function isPayment(description: string) {
  return /pagamento (recebido|efetuado|de fatura)|pagto|pagamento fatura/.test(fold(description));
}

/** Hash curto e estável (FNV-1a) para dar identidade à linha colada. */
function hash(text: string) {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(36);
}

type RawLine = { id: string; description: string; amountCents: number; date: string | null };

/**
 * Separa compras de créditos. Um crédito com uma compra do mesmo valor e da
 * mesma origem é estorno: os dois somem (o "Uber - NuPay" cobrado e devolvido
 * no mesmo dia). O resto dos créditos é abatimento.
 */
function settle(raw: RawLine[]) {
  const charges = raw.filter((line) => line.amountCents > 0);
  const credits = raw.filter((line) => line.amountCents < 0);
  const ignored: ParsedStatement["ignored"] = [];
  const dropped = new Set<string>();
  let unmatchedCredits = 0;

  for (const credit of credits) {
    if (isPayment(credit.description)) {
      ignored.push({ description: credit.description, amountCents: -credit.amountCents });
      continue;
    }
    const origin = fold(splitInstallment(credit.description).description).split(/\s|\*/)[0];
    const pair = charges.find(
      (charge) =>
        !dropped.has(charge.id) &&
        charge.amountCents === -credit.amountCents &&
        fold(charge.description).startsWith(origin),
    );
    if (pair) {
      dropped.add(pair.id);
      ignored.push({ description: `${credit.description} (estorno)`, amountCents: -credit.amountCents });
    } else {
      unmatchedCredits += -credit.amountCents;
      ignored.push({ description: credit.description, amountCents: -credit.amountCents });
    }
  }

  const lines = charges
    .filter((charge) => !dropped.has(charge.id))
    .map((charge) => {
      const split = splitInstallment(charge.description);
      return {
        externalId: charge.id,
        description: split.description,
        amountCents: charge.amountCents,
        purchaseDate: charge.date,
        installmentNumber: split.number,
        installmentTotal: split.total,
      };
    });

  return { lines, ignored, unmatchedCredits };
}

function ofxTag(block: string, tag: string) {
  const match = new RegExp(`<${tag}>([^<\\r\\n]*)`, "i").exec(block);
  return match ? match[1].trim() : null;
}

function ofxDate(value: string | null) {
  if (!value || !/^\d{8}/.test(value)) return null;
  return `${value.slice(0, 4)}-${value.slice(4, 6)}-${value.slice(6, 8)}`;
}

export function parseOfx(text: string): ParsedStatement {
  const blocks = text.split(/<STMTTRN>/i).slice(1);
  // O FITID do Nubank não é único: o IOF sai com o mesmo FITID da compra
  // internacional que o gerou. A identidade é FITID + valor, e um contador
  // separa o que ainda assim se repetir.
  const seen = new Map<string, number>();
  const raw: RawLine[] = blocks.map((block, index) => {
    const amount = Number(ofxTag(block, "TRNAMT") ?? "0");
    const date = ofxDate(ofxTag(block, "DTPOSTED"));
    const description = ofxTag(block, "MEMO") ?? ofxTag(block, "NAME") ?? "Compra";
    const base = `${ofxTag(block, "FITID") ?? `${date}-${index}-${hash(description)}`}:${toCents(String(Math.abs(amount)))}`;
    const occurrence = (seen.get(base) ?? 0) + 1;
    seen.set(base, occurrence);
    return {
      id: `ofx:${base}${occurrence > 1 ? `~${occurrence}` : ""}`,
      description,
      // No OFX do cartão, compra é negativa. Aqui compra é positiva.
      amountCents: -toCents(String(amount)),
      date,
    };
  });

  const { lines, ignored } = settle(raw);
  const balance = ofxTag(text.split(/<LEDGERBAL>/i)[1] ?? "", "BALAMT");
  const owedCents = balance === null ? null : Math.max(-toCents(balance), 0);
  const chargesCents = lines.reduce((total, line) => total + line.amountCents, 0);
  const creditsCents = owedCents === null ? 0 : Math.max(chargesCents - owedCents, 0);

  return { format: "ofx", lines, owedCents, creditsCents, dueDate: null, ignored };
}

const AMOUNT = /(-?)\s*R\$\s*([\d.]+,\d{2})/;

/**
 * Data de compra no texto colado: "17/03/2026" ou "17 de março". "4/10" sem
 * ano é parcela, nunca data — por isso o formato numérico exige o ano.
 */
function textDate(line: string, year: number) {
  const numeric = /(\d{1,2})\/(\d{1,2})\/(\d{4})/.exec(line);
  if (numeric) {
    return `${numeric[3]}-${numeric[2].padStart(2, "0")}-${numeric[1].padStart(2, "0")}`;
  }
  const written = /(\d{1,2})\s+de\s+([a-z]+)/.exec(fold(line));
  if (written) {
    const month = MONTHS_PT.indexOf(written[2]);
    if (month >= 0) {
      return `${year}-${String(month + 1).padStart(2, "0")}-${written[1].padStart(2, "0")}`;
    }
  }
  return null;
}

/**
 * O texto que se copia do app do banco. Cada linha com um valor em reais é uma
 * compra; "X de Y" ou "X/Y" é a parcela; "Única" é à vista. Linhas de resumo
 * ("fatura em aberto", "as parcelas somam") ficam de fora e alimentam o total
 * e o vencimento.
 */
export function parseStatementText(text: string, today = new Date()): ParsedStatement {
  const year = today.getFullYear();
  const raw: RawLine[] = [];
  // Linhas idênticas (duas parcelas iguais da mesma loja) precisam de
  // identidades diferentes, e a mesma colagem repetida, das mesmas.
  const seen = new Map<string, number>();
  let owedCents: number | null = null;
  let dueDate: string | null = null;

  for (const original of text.split(/\r?\n/)) {
    const line = original.replace(/ /g, " ").trim();
    if (!line) continue;
    const folded = fold(line);

    const due = /vencimento\s+(?:em|dia|:)?\s*(\d{1,2})\s+de\s+([a-z]+)/.exec(folded);
    if (due && MONTHS_PT.includes(due[2])) {
      const month = MONTHS_PT.indexOf(due[2]) + 1;
      const dueYear = month < today.getMonth() + 1 - 6 ? year + 1 : year;
      dueDate = `${dueYear}-${String(month).padStart(2, "0")}-${due[1].padStart(2, "0")}`;
    }

    const amount = AMOUNT.exec(line);
    if (!amount) continue;

    const isSummary = /fatura|total|somam|limite|fechamento|saldo|ate agora/.test(folded);
    if (isSummary) {
      if (owedCents === null && /fatura|saldo|total/.test(folded)) {
        owedCents = brlToCents(amount[2]);
      }
      continue;
    }

    // A descrição é o que vem antes do valor, sem colunas de tabela.
    const before = line.slice(0, amount.index).replace(/\s{2,}|\t/g, " ").trim();
    const withoutLabel = before.replace(/\s*(?:Única|Unica)\s*$/i, "").trim();
    const parcel = /(\d{1,2})\s*(?:de|\/)\s*(\d{1,2})\s*$/.exec(withoutLabel);
    const name = parcel
      ? `${withoutLabel.slice(0, parcel.index).trim()} - ${parcel[1]}/${parcel[2]}`
      : withoutLabel;
    const description = name.replace(/\s+—\s+compra à vista$/i, "").trim() || "Compra";
    const cents = brlToCents(amount[2]) * (amount[1] === "-" ? -1 : 1);
    const occurrence = (seen.get(line) ?? 0) + 1;
    seen.set(line, occurrence);
    raw.push({
      id: `txt:${hash(`${line}#${occurrence}`)}`,
      description,
      amountCents: cents,
      date: textDate(before, year),
    });
  }

  const { lines, ignored, unmatchedCredits } = settle(raw);
  return {
    format: "text",
    lines,
    owedCents,
    creditsCents: unmatchedCredits,
    dueDate,
    ignored,
  };
}

export function parseStatement(input: string, today = new Date()): ParsedStatement {
  return /<OFX>|<STMTTRN>/i.test(input) ? parseOfx(input) : parseStatementText(input, today);
}

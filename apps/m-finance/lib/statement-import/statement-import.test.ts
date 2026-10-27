import { describe, expect, it } from "vitest";
import { learnFrom, merchantKey, suggestFor } from "@/lib/statement-import/classify";
import { parseOfx, parseStatement, parseStatementText, splitInstallment } from "@/lib/statement-import/parse";
import { planImport, type ReviewedItem } from "@/lib/statement-import/plan";

const trn = (fitid: string, date: string, amount: string, memo: string) =>
  `<STMTTRN>\n<TRNTYPE>DEBIT\n<DTPOSTED>${date}000000[-3:BRT]\n<TRNAMT>${amount}\n<FITID>${fitid}\n<MEMO>${memo}\n</STMTTRN>`;

// Formato do OFX do Nubank: compra negativa, pagamento positivo, saldo devedor negativo.
const OFX = `OFXHEADER:100
<OFX><CREDITCARDMSGSRSV1><CCSTMTTRNRS><CCSTMTRS><BANKTRANLIST>
${trn("a1", "20261005", "200.00", "Pagamento recebido")}
${trn("a2", "20261008", "-100.00", "Loja Teste - Parcela 4/10")}
${trn("a3", "20261009", "-19.90", "Uber - NuPay")}
${trn("a4", "20261009", "19.90", "Uber - NuPay")}
${trn("a5", "20261010", "-50.00", "Openai *Chatgpt Subscr")}
${trn("a6", "20261011", "-30.00", "iFood - NuPay - 2/3")}
${trn("a7", "20261012", "40.00", "Pagamento recebido")}
${trn("a8", "20261013", "10.00", "Ajuste a crédito")}
</BANKTRANLIST>
<LEDGERBAL><BALAMT>-130.00<DTASOF>20261105</LEDGERBAL>
</CCSTMTRS></CCSTMTTRNRS></CREDITCARDMSGSRSV1></OFX>`;

describe("splitInstallment", () => {
  it.each([
    ["Airbnb * Abc - Parcela 1/6", "Airbnb * Abc", 1, 6],
    ["KaBuM! - NuPay - 11/18", "KaBuM! - NuPay", 11, 18],
    ["Parcelamento de Compra - Kfc Sh - 3/3", "Kfc Sh", 3, 3],
    ["Mercado Livre 4 de 10", "Mercado Livre", 4, 10],
  ])("%s", (raw, description, number, total) => {
    expect(splitInstallment(raw)).toEqual({ description, number, total });
  });

  it("não inventa parcela onde não há", () => {
    expect(splitInstallment("Google G1ai101m")).toEqual({
      description: "Google G1ai101m",
      number: null,
      total: null,
    });
  });
});

describe("parseOfx", () => {
  const parsed = parseOfx(OFX);

  it("fica só com as compras, e estorno com par some junto com a compra", () => {
    expect(parsed.lines.map((line) => [line.description, line.amountCents])).toEqual([
      ["Loja Teste", 100_00],
      ["Openai *Chatgpt Subscr", 50_00],
      ["iFood - NuPay", 30_00],
    ]);
  });

  it("lê parcela, data e identidade", () => {
    expect(parsed.lines[0]).toMatchObject({
      externalId: "ofx:a2:10000",
      purchaseDate: "2026-10-08",
      installmentNumber: 4,
      installmentTotal: 10,
    });
  });

  it("o IOF com o mesmo FITID da compra ganha identidade própria", () => {
    const ofx = [
      "<OFX>",
      trn("z", "20261021", "-43.67", "Nba League Pass"),
      trn("z", "20261021", "-1.52", "IOF de compra internacional"),
      trn("z", "20261021", "-1.52", "IOF de compra internacional"),
    ].join("\n");
    const ids = parseOfx(ofx).lines.map((line) => line.externalId);
    expect(new Set(ids).size).toBe(3);
  });

  it("o que falta pagar vem do saldo, e o abatido é a diferença para as compras", () => {
    // Compras 180 − saldo 130 = 50: o pagamento antecipado de 40 e o ajuste de 10.
    expect(parsed.owedCents).toBe(130_00);
    expect(parsed.creditsCents).toBe(50_00);
  });
});

describe("parseStatementText", () => {
  const text = `Sua fatura em aberto está em R$ 300,00 até agora:

Compra\tParcela\tValor
Amazon — compra à vista\tÚnica\tR$ 20,00
Mercado Livre\t4 de 10\tR$ 60,00
Mercado Livre\t4 de 10\tR$ 60,00
Loja Grande\t12 de 24\tR$ 1.160,00
As parcelas somam R$ 1.280,00, mais R$ 20,00 da compra à vista. O fechamento está previsto para 9 de novembro, com vencimento em 16 de novembro.`;
  const parsed = parseStatementText(text, new Date(2026, 9, 26));

  it("cada linha com valor é uma compra; resumo fica de fora", () => {
    expect(parsed.lines.map((line) => [line.description, line.amountCents, line.installmentNumber])).toEqual([
      ["Amazon", 20_00, null],
      ["Mercado Livre", 60_00, 4],
      ["Mercado Livre", 60_00, 4],
      ["Loja Grande", 1160_00, 12],
    ]);
  });

  it("linhas iguais ganham identidades diferentes, e a mesma colagem as mesmas", () => {
    expect(parsed.lines[1].externalId).not.toBe(parsed.lines[2].externalId);
    expect(parseStatementText(text, new Date(2026, 9, 26)).lines[1].externalId).toBe(
      parsed.lines[1].externalId,
    );
  });

  it("lê o total e o vencimento do resumo", () => {
    expect(parsed.owedCents).toBe(300_00);
    expect(parsed.dueDate).toBe("2026-11-16");
  });

  it("parseStatement escolhe o formato sozinho", () => {
    expect(parseStatement(OFX).format).toBe("ofx");
    expect(parseStatement(text).format).toBe("text");
  });
});

describe("classificação", () => {
  const none = new Map();

  it.each([
    ["Openai *Chatgpt Subscr", "assinaturas", true],
    ["Openai", "software-apis", true],
    ["Anthropic* Claude Sub", "software-apis", true],
    ["Amazon Ad Free For Pri", "assinaturas", true],
    ["Amazon Brasil - NuPay", "compras", false],
    ["IOF de compra internacional", "taxas", false],
    ["Hiper Select Supermerc", "mercado", false],
    ["Posto Sonho Meu Barra", "transporte", false],
    ["iFood - NuPay", "alimentacao", false],
    ["Registrobr", "software-apis", false],
    ["Wellhub Fulano", "saude", true],
  ])("%s → %s", (description, slug, subscription) => {
    expect(suggestFor(description, none)).toMatchObject({
      categorySlug: slug,
      isSubscription: subscription,
      source: "rule",
    });
  });

  it("o que não reconhece fica sem categoria", () => {
    expect(suggestFor("Mateusribeiro", none)).toEqual({
      categorySlug: null,
      isSubscription: false,
      source: "none",
    });
  });

  it("a escolha anterior para a mesma origem vence a tabela", () => {
    const learned = learnFrom([
      { description: "Mateusribeiro", categorySlug: "lazer", isSubscription: false },
      { description: "Posto Sonho Meu Barra", categorySlug: "moto", isSubscription: false },
    ]);
    expect(suggestFor("Mateusribeiro", learned)).toMatchObject({ categorySlug: "lazer", source: "learned" });
    expect(suggestFor("Posto Sonho Meu Centro", learned)).toMatchObject({ categorySlug: "moto" });
  });

  it("assinatura e API da mesma empresa são origens diferentes", () => {
    expect(merchantKey("Openai *Chatgpt Subscr")).not.toBe(merchantKey("Openai"));
  });
});

describe("planImport", () => {
  const item = (extra: Partial<ReviewedItem> = {}): ReviewedItem => ({
    externalId: "ofx:x",
    description: "Loja",
    amountCents: 100_00,
    purchaseDate: "2026-10-08",
    installmentNumber: null,
    installmentTotal: null,
    categoryId: null,
    isSubscription: false,
    ...extra,
  });

  it("a parcela 4/10 vira as parcelas 4 a 10, uma por mês", () => {
    const { rows } = planImport([item({ installmentNumber: 4, installmentTotal: 10 })], []);
    expect(rows.map((row) => [row.monthOffset, row.installmentNumber, row.externalId])).toEqual([
      [0, 4, "ofx:x"],
      [1, 5, "ofx:x#5"],
      [2, 6, "ofx:x#6"],
      [3, 7, "ofx:x#7"],
      [4, 8, "ofx:x#8"],
      [5, 9, "ofx:x#9"],
      [6, 10, "ofx:x#10"],
    ]);
  });

  it("não grava de novo o que a mesma fatura já trouxe", () => {
    const result = planImport([item()], [{ ...item(), externalId: "ofx:x" }]);
    expect(result.rows).toHaveLength(0);
    expect(result.skipped).toHaveLength(1);
  });

  it("a parcela que a importação anterior já criou é reconhecida pela parcela", () => {
    const existing = [
      { externalId: "ofx:velho#5", description: "loja", amountCents: 100_00, installmentNumber: 5, installmentTotal: 10 },
    ];
    const result = planImport(
      [item({ externalId: "ofx:novo", installmentNumber: 5, installmentTotal: 10 })],
      existing,
    );
    expect(result.rows).toHaveLength(0);
  });
});

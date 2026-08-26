import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { groupPayables, payableKey, payableProgress, type Payable } from "@/lib/payables";

const today = new Date(2026, 7, 20); // 20 de agosto de 2026

function payable(overrides: Partial<Payable> & Pick<Payable, "id" | "dueDate">): Payable {
  return {
    type: "bill",
    name: overrides.name ?? `Conta ${overrides.id}`,
    amountCents: 10_000,
    status: "pending",
    ...overrides,
  };
}

describe("groupPayables", () => {
  it("separa os pagaveis pela janela de vencimento", () => {
    const groups = groupPayables(
      [
        payable({ id: "vencida", dueDate: "2026-08-15" }),
        payable({ id: "hoje", dueDate: "2026-08-20" }),
        payable({ id: "semana", dueDate: "2026-08-25" }),
        payable({ id: "depois", dueDate: "2026-09-05" }),
      ],
      today,
    );

    expect(groups.map((group) => group.key)).toEqual(["overdue", "today", "week", "later"]);
    expect(groups.map((group) => group.items.map((item) => item.id))).toEqual([
      ["vencida"],
      ["hoje"],
      ["semana"],
      ["depois"],
    ]);
  });

  it("omite grupos sem nenhum pagavel", () => {
    const groups = groupPayables([payable({ id: "hoje", dueDate: "2026-08-20" })], today);

    expect(groups.map((group) => group.key)).toEqual(["today"]);
  });

  it("mistura contas e faturas de cartao na mesma pilha, ordenadas por vencimento", () => {
    const groups = groupPayables(
      [
        payable({ id: "luz", dueDate: "2026-08-24" }),
        payable({ id: "nubank", dueDate: "2026-08-22", type: "invoice" }),
      ],
      today,
    );

    expect(groups).toHaveLength(1);
    expect(groups[0].items.map((item) => item.id)).toEqual(["nubank", "luz"]);
  });

  it("desce o pagavel ja pago para o fim do proprio grupo", () => {
    const groups = groupPayables(
      [
        payable({ id: "paga", dueDate: "2026-08-21", status: "paid" }),
        payable({ id: "aberta", dueDate: "2026-08-25" }),
      ],
      today,
    );

    expect(groups[0].items.map((item) => item.id)).toEqual(["aberta", "paga"]);
  });

  it("mantem o pagavel no mesmo grupo depois de pago, para a linha nao saltar", () => {
    const antes = groupPayables([payable({ id: "luz", dueDate: "2026-08-15" })], today);
    const depois = groupPayables(
      [payable({ id: "luz", dueDate: "2026-08-15", status: "paid" })],
      today,
    );

    expect(depois[0].key).toBe(antes[0].key);
    expect(depois[0].key).toBe("overdue");
  });

  it("preserva os campos extras do item original", () => {
    const groups = groupPayables(
      [{ ...payable({ id: "luz", dueDate: "2026-08-20" }), categoryName: "Casa" }],
      today,
    );

    expect(groups[0].items[0].categoryName).toBe("Casa");
  });
});

describe("payableProgress", () => {
  it("conta quanto falta pagar e quanto ja foi", () => {
    const progress = payableProgress([
      payable({ id: "a", dueDate: "2026-08-20", amountCents: 30_000, status: "paid" }),
      payable({ id: "b", dueDate: "2026-08-21", amountCents: 20_000 }),
      payable({ id: "c", dueDate: "2026-08-10", amountCents: 50_000, status: "overdue" }),
    ]);

    expect(progress).toEqual({
      totalCount: 3,
      paidCount: 1,
      remainingCount: 2,
      totalCents: 100_000,
      paidCents: 30_000,
      remainingCents: 70_000,
    });
  });

  it("trata a lista vazia sem quebrar", () => {
    expect(payableProgress([])).toEqual({
      totalCount: 0,
      paidCount: 0,
      remainingCount: 0,
      totalCents: 0,
      paidCents: 0,
      remainingCents: 0,
    });
  });
});

describe("payableKey", () => {
  it("distingue conta de fatura com o mesmo id", () => {
    expect(payableKey({ id: "abc", type: "bill" })).toBe("bill:abc");
    expect(payableKey({ id: "abc", type: "invoice" })).toBe("invoice:abc");
  });
});

/*
 * A regressão que este bloco tranca.
 *
 * `payableKey` morava em `components/payable/payable-list.tsx`, que é
 * `"use client"`, e o Server Component das Contas a importava de lá. Do outro
 * lado da fronteira, um export de módulo cliente chega como REFERÊNCIA — chamar
 * lança no render. A página de Contas caía inteira, mas só nos meses com pelo
 * menos uma conta: a chamada vive dentro de `bills.map`, então um mês vazio
 * renderizava normal e escondia o defeito.
 *
 * O teste olha o texto do arquivo porque é a única camada que enxerga a
 * fronteira: `vitest` roda tudo no mesmo processo Node, sem cliente e sem
 * servidor, e por isso NUNCA reproduziria o erro chamando a função.
 */
describe("fronteira servidor/cliente", () => {
  const read = (relativo: string) =>
    readFileSync(fileURLToPath(new URL(relativo, import.meta.url)), "utf8");

  it("mantem payableKey fora do modulo 'use client'", () => {
    const lista = read("../components/payable/payable-list.tsx");
    expect(lista).toContain('"use client"');
    expect(lista).not.toMatch(/export\s+function\s+payableKey/);
  });

  it("faz o Server Component das Contas importar payableKey da lib", () => {
    const card = read("../components/bills/bill-form-card.tsx");
    expect(card).toMatch(/import\s*\{[^}]*payableKey[^}]*\}\s*from\s*"@\/lib\/payables"/);
    expect(card).not.toMatch(
      /import\s*\{[^}]*payableKey[^}]*\}\s*from\s*"@\/components\/payable\/payable-list"/,
    );
  });
});

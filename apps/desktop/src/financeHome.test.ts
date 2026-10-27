import { describe, expect, it } from "vitest";
import { diaCurto, perguntaDoInsight, resumoDaHome } from "./financeHome";

const pack = {
  currentMonth: {
    accountingRemainingCents: 240000,
    safeToSpend: {
      safeToSpendCents: 190000,
      status: "fair",
      deductions: [{ reason: "income_reliability", amountCents: 50000 }],
    },
  },
  next30Days: {
    overdueCents: 18000,
    items: [
      { kind: "subscription", label: "Notion", amountCents: 4000, dueDate: "2026-09-18", status: "pending" },
      { kind: "invoice", label: "Fatura Nubank", amountCents: 150000, dueDate: "2026-09-15", status: "pending" },
      { kind: "bill", label: "Conta de luz", amountCents: 18000, dueDate: "2026-09-05", status: "overdue" },
    ],
  },
  recentInsights: [
    { severity: "info", title: "Assinaturas somam R$ 510", summary: "..." },
    { severity: "critical", title: "Conta de luz está vencida", summary: "10 dias" },
  ],
};

describe("resumoDaHome", () => {
  it("Safe-to-Spend com a sobra e as deduções ao lado", () => {
    const resumo = resumoDaHome(pack, "2026-09-15T12:00:00Z");
    expect(resumo.safeToSpendCents).toBe(190000);
    expect(resumo.accountingRemainingCents).toBe(240000);
    expect(resumo.deductionsCents).toBe(50000);
  });

  it("o vencido vem antes do próximo a vencer, e assinatura não é 'próximo'", () => {
    expect(resumoDaHome(pack, "t").proximo).toMatchObject({ label: "Conta de luz", overdue: true });
    const semVencido = { ...pack, next30Days: { items: pack.next30Days.items.slice(0, 2) } };
    expect(resumoDaHome(semVencido, "t").proximo?.label).toBe("Fatura Nubank");
  });

  it("o insight mais grave é o que aparece", () => {
    expect(resumoDaHome(pack, "t").atencao?.title).toBe("Conta de luz está vencida");
  });

  it("pacote vazio ou torto não quebra", () => {
    const resumo = resumoDaHome(null, "t");
    expect(resumo.safeToSpendCents).toBeNull();
    expect(resumo.status).toBe("unknown");
    expect(resumo.proximo).toBeNull();
    expect(resumoDaHome({ currentMonth: "x", next30Days: { items: "y" } }, "t").proximo).toBeNull();
  });
});

describe("textos", () => {
  it("a pergunta leva o título do alerta", () => {
    expect(perguntaDoInsight({ title: "Outubro ficou apertado" })).toContain('"Outubro ficou apertado"');
  });

  it("dia curto", () => {
    expect(diaCurto("2026-09-20")).toBe("20/09");
  });
});

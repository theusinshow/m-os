import { describe, expect, it } from "vitest";
import { cobraAtencao, horaDoItem, porDia, tituloDoDia } from "./dias";
import type { ItemDaAgenda } from "../api";

function item(over: Partial<ItemDaAgenda> = {}): ItemDaAgenda {
  return {
    kind: "session",
    at: "2026-09-04T13:00:00-03:00",
    endsAt: null,
    title: "Rancho Queimado",
    projectId: null,
    seconds: 3_600,
    amountCents: 3_000,
    ...over,
  };
}

const HOJE = new Date("2026-09-04T09:00:00-03:00");

describe("a agenda agrupada por dia", () => {
  it("junta o que cai no mesmo dia local", () => {
    const dias = porDia(
      [
        item({ at: "2026-09-04T09:00:00-03:00" }),
        item({ at: "2026-09-04T17:00:00-03:00" }),
        item({ at: "2026-09-05T09:00:00-03:00" }),
      ],
      HOJE,
    );
    expect(dias.map((dia) => dia.titulo)).toEqual(["HOJE", "AMANHÃ"]);
    expect(dias[0].itens).toHaveLength(2);
  });

  // O dia é o de quem olha. Às 22h em UTC-3 já é o dia seguinte em UTC, e
  // agrupar por UTC jogaria a noite de trabalho para amanhã.
  it("corta o dia no fuso do aparelho, e nao em UTC", () => {
    const dias = porDia([item({ at: "2026-09-04T22:00:00-03:00" })], HOJE);
    expect(dias).toHaveLength(1);
    expect(dias[0].titulo).toBe("HOJE");
  });

  // Só a sessão soma: uma prova tem `seconds` zero e uma captura não é tempo
  // trabalhado. Somar tudo daria um total que não corresponde a nada cobrável.
  it("soma so a hora trabalhada no total do dia", () => {
    const dias = porDia(
      [
        item({ seconds: 3_600 }),
        item({ kind: "capture", seconds: 0, title: "uma ideia" }),
        item({ kind: "exam_scheduled", seconds: 0, title: "Prova de Cálculo III" }),
      ],
      HOJE,
    );
    expect(dias[0].segundos).toBe(3_600);
    expect(dias[0].itens).toHaveLength(3);
  });

  it("ordena os dias do mais antigo para o mais novo", () => {
    const dias = porDia(
      [
        item({ at: "2026-09-06T09:00:00-03:00" }),
        item({ at: "2026-09-03T09:00:00-03:00" }),
      ],
      HOJE,
    );
    expect(dias.map((dia) => dia.data)).toEqual(["2026-09-03", "2026-09-06"]);
  });
});

describe("o titulo do dia", () => {
  it("usa palavra para ontem, hoje e amanha", () => {
    expect(tituloDoDia(new Date("2026-09-03T10:00:00-03:00"), HOJE)).toBe("ONTEM");
    expect(tituloDoDia(new Date("2026-09-04T10:00:00-03:00"), HOJE)).toBe("HOJE");
    expect(tituloDoDia(new Date("2026-09-05T10:00:00-03:00"), HOJE)).toBe("AMANHÃ");
  });

  it("usa dia e mes para o resto", () => {
    expect(tituloDoDia(new Date("2026-09-11T10:00:00-03:00"), HOJE)).toBe("SEX · 11 DE SET");
  });
});

describe("o que cobra atencao", () => {
  it("marca prova, prazo e bloco planejado", () => {
    expect(cobraAtencao(item({ kind: "exam_scheduled" }))).toBe(true);
    expect(cobraAtencao(item({ kind: "assignment_due" }))).toBe(true);
    expect(cobraAtencao(item({ kind: "academic_planned" }))).toBe(true);
  });

  it("nao marca o rastro do que ja passou", () => {
    expect(cobraAtencao(item({ kind: "session" }))).toBe(false);
    expect(cobraAtencao(item({ kind: "capture" }))).toBe(false);
    expect(cobraAtencao(item({ kind: "task_done" }))).toBe(false);
  });
});

describe("a hora do item", () => {
  it("mostra a hora quando ha uma", () => {
    expect(horaDoItem(item({ at: "2026-09-04T14:30:00-03:00" }))).toBe("14:30");
  });

  // Meia-noite é como o domínio grava prazo sem hora marcada. "00:00" faria um
  // prazo do dia parecer compromisso de madrugada.
  it("diz dia quando o instante e meia-noite", () => {
    expect(horaDoItem(item({ at: "2026-09-04T00:00:00-03:00" }))).toBe("dia");
  });
});

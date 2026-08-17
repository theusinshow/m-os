import { describe, expect, it } from "vitest";
import { groupByLocalDay, monthGrid, startOfLocalDay } from "./calendarDays";
import type { CalendarItem } from "./types";

function item(at: string): CalendarItem {
  return {
    kind: "session",
    at,
    endsAt: null,
    title: "043 - Rancho Queimado",
    projectId: null,
    seconds: 3000,
    amountCents: 2500,
  };
}

describe("monthGrid", () => {
  it("devolve 42 dias e comeca numa segunda", () => {
    const grid = monthGrid(new Date(2026, 7, 16));
    expect(grid).toHaveLength(42);
    expect(grid[0].getDay()).toBe(1);
  });

  it("cobre o mes inteiro pedido", () => {
    const grid = monthGrid(new Date(2026, 7, 16));
    expect(grid.filter((day) => day.getMonth() === 7)).toHaveLength(31);
  });

  /**
   * Fevereiro de 2026 comeca num domingo — o mes que mais espicha a grade para
   * tras. Se `monthGrid` derivasse o tamanho do mes em vez de fixar 42, este
   * seria o caso que devolveria uma grade de altura diferente.
   */
  it("mantem 42 celulas mesmo no mes que comeca no domingo", () => {
    const grid = monthGrid(new Date(2026, 1, 10));
    expect(grid).toHaveLength(42);
    expect(grid[0].getDay()).toBe(1);
  });
});

describe("groupByLocalDay", () => {
  /**
   * O teste que justifica o vitest existir.
   *
   * O usuário trabalha de madrugada. Uma sessão iniciada às 23:31 do dia 30 é,
   * em UTC-3, `2026-07-31T02:31:00Z`. Agrupar pelo dia do texto UTC a colocaria
   * no dia 31 — a grade mostraria horas num dia em que ele não trabalhou, sem
   * nada quebrar, sem teste falhar e sem erro aparecer.
   */
  it("poe a sessao das 23:31 no dia em que ela comecou, e nao no seguinte", () => {
    const grouped = groupByLocalDay([item(new Date(2026, 6, 30, 23, 31).toISOString())]);

    const thirtieth = startOfLocalDay(new Date(2026, 6, 30)).getTime();
    const thirtyFirst = startOfLocalDay(new Date(2026, 6, 31)).getTime();

    expect(grouped.get(thirtieth)).toHaveLength(1);
    expect(grouped.get(thirtyFirst)).toBeUndefined();
  });

  it("junta dois itens do mesmo dia local", () => {
    const grouped = groupByLocalDay([
      item(new Date(2026, 7, 12, 10, 39).toISOString()),
      item(new Date(2026, 7, 12, 18, 51).toISOString()),
    ]);
    expect(grouped.get(startOfLocalDay(new Date(2026, 7, 12)).getTime())).toHaveLength(2);
  });

  it("separa dois itens de dias vizinhos", () => {
    const grouped = groupByLocalDay([
      item(new Date(2026, 7, 12, 23, 50).toISOString()),
      item(new Date(2026, 7, 13, 0, 10).toISOString()),
    ]);
    expect(grouped.size).toBe(2);
  });

  it("devolve um mapa vazio para lista vazia", () => {
    expect(groupByLocalDay([]).size).toBe(0);
  });
});

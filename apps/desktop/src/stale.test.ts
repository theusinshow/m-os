import { describe, expect, it } from "vitest";
import {
  atividadePorProject,
  diasPorTask,
  mexidoHoje,
  paradasVisiveis,
  projectsParados,
  rotuloDeDias,
} from "./stale";
import type { Parada } from "./types";

function parada(over: Partial<Parada> = {}): Parada {
  return {
    kind: "task",
    id: "t1",
    title: "pintar a sala",
    context: "Casa",
    state: "doing",
    days: 12,
    ...over,
  };
}

describe("rótulo de dias", () => {
  it("é curto porque mora dentro de um card estreito", () => {
    expect(rotuloDeDias(12)).toBe("12d");
  });

  it("passa de 99 dias sem virar número gigante no card", () => {
    expect(rotuloDeDias(365)).toBe("99+d");
  });

  it("dia nenhum não vira rótulo", () => {
    expect(rotuloDeDias(0)).toBe("");
    expect(rotuloDeDias(-3)).toBe("");
  });
});

describe("corte da lista", () => {
  const cinco = Array.from({ length: 5 }, (_, indice) => parada({ id: `t${indice}` }));

  it("cinco cabem inteiras, e não sobra nada", () => {
    const { visiveis, restantes } = paradasVisiveis(cinco);
    expect(visiveis).toHaveLength(5);
    expect(restantes).toBe(0);
  });

  it("a sexta vira contagem em vez de sumir em silêncio", () => {
    const oito = Array.from({ length: 8 }, (_, indice) => parada({ id: `t${indice}` }));
    const { visiveis, restantes } = paradasVisiveis(oito);
    expect(visiveis).toHaveLength(5);
    expect(restantes).toBe(3);
  });

  it("preserva a ordem que veio do domínio", () => {
    // A ordem é o excesso proporcional, e ela é decidida no Rust. Reordenar
    // aqui seria uma segunda regra, divergindo em silêncio.
    const entrada = [parada({ id: "a" }), parada({ id: "b" }), parada({ id: "c" })];
    expect(paradasVisiveis(entrada).visiveis.map((item) => item.id)).toEqual(["a", "b", "c"]);
  });
});

describe("índices para o Kanban e a Home", () => {
  it("os dias de cada Task ficam acháveis por id", () => {
    const paradas = [parada({ id: "t1", days: 9 }), parada({ id: "t2", days: 40 })];
    const indice = diasPorTask(paradas);
    expect(indice.get("t1")).toBe(9);
    expect(indice.get("t2")).toBe(40);
  });

  it("o Project parado não entra no índice de Tasks", () => {
    const paradas = [parada({ kind: "project", id: "p1", days: 30 })];
    expect(diasPorTask(paradas).size).toBe(0);
    expect(projectsParados(paradas).has("p1")).toBe(true);
  });
});

describe("atividade do Project", () => {
  it("o ponto acende pela atividade real, e não pelo campo renomeado", () => {
    const indice = atividadePorProject([
      { projectId: "p1", lastActivity: "2026-08-22T09:00:00Z" },
    ]);
    expect(indice.get("p1")).toBe("2026-08-22T09:00:00Z");
  });

  it("mexido hoje compara a data local de quem está olhando", () => {
    const agora = new Date(2026, 7, 22, 15, 0, 0);
    const cedo = new Date(2026, 7, 22, 1, 0, 0).toISOString();
    const ontem = new Date(2026, 7, 21, 23, 0, 0).toISOString();
    expect(mexidoHoje(cedo, agora)).toBe(true);
    expect(mexidoHoje(ontem, agora)).toBe(false);
  });

  it("Project sem atividade conhecida não acende", () => {
    expect(mexidoHoje(undefined)).toBe(false);
    expect(mexidoHoje("lixo")).toBe(false);
  });
});

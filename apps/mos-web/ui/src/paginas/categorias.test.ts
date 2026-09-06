import { describe, expect, it } from "vitest";
import type { ItemDaAgenda } from "../api";
import { alternar, aplicar, categoriaDe, CATEGORIAS, TUDO } from "./categorias";

function item(kind: string): ItemDaAgenda {
  return {
    kind,
    at: "2026-09-05T12:00:00-03:00",
    endsAt: null,
    title: kind,
    projectId: null,
    seconds: 0,
    amountCents: 0,
  };
}

/** Os catorze tipos que o `CalendarKind` do núcleo produz hoje. */
const TIPOS = [
  "session",
  "task_done",
  "task_created",
  "capture",
  "app_opened",
  "day_started",
  "day_ended",
  "objective_done",
  "assignment_due",
  "exam_scheduled",
  "academic_planned",
  "meeting",
  "reminder",
  "holiday",
];

describe("as categorias do calendario", () => {
  // Um `kind` sem categoria seria um item que some quando qualquer filtro é
  // ligado — e sumir em silêncio é o pior defeito possível num calendário.
  it("todo tipo do nucleo cai numa categoria conhecida", () => {
    for (const tipo of TIPOS) {
      const categoria = categoriaDe(item(tipo));
      expect(
        CATEGORIAS.some((c) => c.chave === categoria),
        `${tipo} caiu em ${categoria}, que nao existe`,
      ).toBe(true);
    }
  });

  it("agrupa a faculdade inteira numa categoria so", () => {
    expect(categoriaDe(item("assignment_due"))).toBe("faculdade");
    expect(categoriaDe(item("exam_scheduled"))).toBe("faculdade");
    expect(categoriaDe(item("academic_planned"))).toBe("faculdade");
  });

  it("hora trabalhada e cronocad, e nao 'outros'", () => {
    expect(categoriaDe(item("session"))).toBe("cronocad");
  });

  // Tipo de um servidor mais novo que esta tela. Sumir seria a tela escondendo
  // o que ela não entende.
  it("tipo desconhecido cai em outros, e nao some", () => {
    expect(categoriaDe(item("coisa_do_futuro"))).toBe("outros");
    expect(aplicar([item("coisa_do_futuro")], TUDO)).toHaveLength(1);
  });
});

describe("o filtro", () => {
  const ITENS = [item("session"), item("exam_scheduled"), item("reminder")];

  it("deixa passar so o que esta ligado", () => {
    expect(aplicar(ITENS, ["faculdade"]).map((i) => i.kind)).toEqual(["exam_scheduled"]);
  });

  it("liga e desliga pelo mesmo alvo", () => {
    const sem = alternar(TUDO, "cronocad");
    expect(sem).not.toContain("cronocad");
    expect(alternar(sem, "cronocad")).toContain("cronocad");
  });

  // Ninguém desliga as oito querendo uma tela em branco: desliga-se a última
  // por engano. Uma agenda vazia sem explicação parece defeito.
  it("filtro vazio devolve tudo, e nao nada", () => {
    expect(aplicar(ITENS, [])).toHaveLength(3);
  });
});

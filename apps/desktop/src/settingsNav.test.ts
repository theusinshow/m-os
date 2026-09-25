import { describe, expect, it } from "vitest";
import { SETTINGS_GROUPS, SETTINGS_SECTIONS, secaoInicial } from "./settingsNav";

describe("o catálogo", () => {
  it("põe Sincronização primeiro: é a única que se procura, e não se acha por acaso", () => {
    expect(SETTINGS_SECTIONS[0].id).toBe("sync");
  });

  it("todo id é único — ele vira alvo de navegação e chave salva", () => {
    const ids = SETTINGS_SECTIONS.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("o Hermes tem seção própria, e não um painel dentro de Conexões", () => {
    expect(SETTINGS_SECTIONS.find((s) => s.id === "hermes")?.grupo).toBe("conexoes");
  });

  it("toda seção pertence a um grupo que existe, e toda seção diz para que serve", () => {
    const grupos = new Set(SETTINGS_GROUPS.map((g) => g.id));
    for (const secao of SETTINGS_SECTIONS) {
      expect(grupos.has(secao.grupo)).toBe(true);
      expect(secao.descricao.length).toBeGreaterThan(0);
    }
  });
});

describe("qual seção abre", () => {
  it("o pedido explícito vence a seção lembrada", () => {
    expect(secaoInicial("hermes", "dados")).toBe("hermes");
    expect(secaoInicial("function_registry", null)).toBe("avancado");
    expect(secaoInicial("updates_check", "sync")).toBe("inicio");
  });

  it("sem pedido, volta à última seção vista", () => {
    expect(secaoInicial(undefined, "saude")).toBe("saude");
  });

  it("seção lembrada que não existe mais cai na primeira", () => {
    expect(secaoInicial(undefined, "conexoes")).toBe("sync");
    expect(secaoInicial(undefined, null)).toBe("sync");
  });
});

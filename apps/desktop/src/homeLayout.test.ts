import { describe, expect, it } from "vitest";
import { arrangeHome, HOME_SECTIONS, HOME_SPANS, HOME_WIDGETS, moveInArrangement, placementsFor, stepSpan, touchedSections } from "./homeLayout";
import type { WidgetPlacement } from "./types";

const WORKSPACE = "01J0000000000000000000000A";
const OUTRO = "01J0000000000000000000000B";

function guardado(widgetId: string, position: number, extra: Partial<WidgetPlacement> = {}): WidgetPlacement {
  return { workspaceId: WORKSPACE, widgetId, position, section: null, span: null, ...extra };
}

const idsDa = (arranjo: ReturnType<typeof arrangeHome>, section: string) =>
  arranjo.filter((slot) => slot.section === section).map((slot) => slot.id);

describe("o catalogo", () => {
  it("nao repete id, porque id repetido some com um widget na resolucao", () => {
    const ids = HOME_WIDGETS.map((widget) => widget.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  /* Um widget numa faixa que nao existe nunca seria desenhado: o `HomeBoard` so
     percorre as faixas do `HOME_SECTIONS`, e o widget sumiria em silencio. */
  it("so poe widget em faixa que existe", () => {
    const faixas = new Set(HOME_SECTIONS.map((section) => section.id));
    for (const widget of HOME_WIDGETS) expect(faixas).toContain(widget.section);
  });

  /* A largura do desenho precisa estar na escala. O `stepSpan` sobrevive a uma
     largura de fora, mas o widget nasceria num degrau que a interface nao
     oferece, e voltar a ele depois de mexer seria impossivel. */
  it("so usa largura que a escala oferece", () => {
    for (const widget of HOME_WIDGETS) expect(HOME_SPANS).toContain(widget.span);
  });
});

describe("arrangeHome", () => {
  it("sem nada guardado, entrega o desenho", () => {
    const arranjo = arrangeHome([], WORKSPACE);
    expect(arranjo.map((slot) => slot.id)).toEqual(HOME_WIDGETS.map((widget) => widget.id));
    expect(idsDa(arranjo, "now")).toEqual(["now", "timer", "today_hours"]);
  });

  /* "Todos" nao tem onde gravar, entao nao aplica arranjo de ninguem — a mesma
     regra que faz "Todos" nao ocultar widget nenhum. */
  it("ignora o arranjo de outro Workspace", () => {
    const arranjo = arrangeHome([guardado("today_hours", 0)], OUTRO);
    expect(idsDa(arranjo, "now")).toEqual(["now", "timer", "today_hours"]);
  });

  it("aplica a ordem guardada dentro da faixa", () => {
    const arranjo = arrangeHome([guardado("today_hours", 0), guardado("now", 1)], WORKSPACE);
    expect(idsDa(arranjo, "now")).toEqual(["today_hours", "now", "timer"]);
  });

  /* O caso que decide se a feature envelhece bem: alguem arrumou a Home, e
     meses depois um widget novo entra no catalogo. Ele NAO pode aparecer no
     meio do arranjo — isso seria o sistema desfazendo a escolha da pessoa. */
  it("poe quem nao tem posicao guardada no fim da faixa", () => {
    const arranjo = arrangeHome([guardado("today_hours", 0)], WORKSPACE);
    expect(idsDa(arranjo, "now")).toEqual(["today_hours", "now", "timer"]);
  });

  it("deixa a largura no que o desenho escolheu ate alguem mudar", () => {
    const semNada = arrangeHome([], WORKSPACE).find((slot) => slot.id === "timer");
    expect(semNada).toMatchObject({ span: 3, savedSpan: null });

    const redimensionado = arrangeHome([guardado("timer", 0, { span: 12 })], WORKSPACE).find((slot) => slot.id === "timer");
    expect(redimensionado).toMatchObject({ span: 12, savedSpan: 12 });
  });

  /* A inversao que a migration 0017 existe para proteger: a pessoa REORDENOU e
     nada mais, entao a largura tem de continuar sendo a do desenho. */
  it("reordenar nao congela a largura", () => {
    const arranjo = arrangeHome([guardado("timer", 0), guardado("now", 1)], WORKSPACE);
    expect(arranjo.find((slot) => slot.id === "now")).toMatchObject({ span: 6, savedSpan: null });
  });

  it("move de faixa quando a faixa guardada e outra", () => {
    const arranjo = arrangeHome([guardado("timer", 0, { section: "utilities" })], WORKSPACE);
    expect(idsDa(arranjo, "now")).toEqual(["now", "today_hours"]);
    expect(idsDa(arranjo, "utilities")).toEqual(["timer", "quick_actions", "system_health"]);
  });

  /* Banco escrito por uma versao com outras faixas nao pode custar um widget:
     sem titulo para desenhar, a faixa desconhecida nao existe na tela, e o
     widget iria junto. Ele volta para casa. */
  it("devolve para a faixa de origem quem foi guardado numa faixa extinta", () => {
    const arranjo = arrangeHome([guardado("timer", 0, { section: "faixa_extinta" })], WORKSPACE);
    expect(idsDa(arranjo, "now")).toContain("timer");
    expect(arranjo).toHaveLength(HOME_WIDGETS.length);
  });

  it("nao perde ninguem com posicao repetida ou salteada", () => {
    const arranjo = arrangeHome([guardado("now", 5), guardado("timer", 5), guardado("today_hours", 900)], WORKSPACE);
    expect(arranjo).toHaveLength(HOME_WIDGETS.length);
    expect(idsDa(arranjo, "now")).toEqual(["now", "timer", "today_hours"]);
  });

  /* A visao "Todos" arruma a propria Home desde a migration 0018. O banco a
     guarda como NULL e o seletor a carrega como string vazia — este e o unico
     ponto do front onde os dois vocabularios se encontram, e por isso e o unico
     lugar onde um `??` esquecido apagaria o arranjo de quem nunca criou
     Workspace nenhum. */
  it("aplica o arranjo de Todos, que o banco guarda como nulo", () => {
    const arranjo = arrangeHome([{ workspaceId: null, widgetId: "today_hours", position: 0, section: null, span: 12 }], "");
    expect(idsDa(arranjo, "now")).toEqual(["today_hours", "now", "timer"]);
    expect(arranjo.find((slot) => slot.id === "today_hours")?.span).toBe(12);
  });

  it("nao deixa o arranjo de Todos vazar para um Workspace", () => {
    const deTodos = [{ workspaceId: null, widgetId: "today_hours", position: 0, section: null, span: null }];
    expect(idsDa(arrangeHome(deTodos, WORKSPACE), "now")).toEqual(["now", "timer", "today_hours"]);
  });

  it("nem o de um Workspace para Todos", () => {
    expect(idsDa(arrangeHome([guardado("today_hours", 0)], ""), "now")).toEqual(["now", "timer", "today_hours"]);
  });

  /* Linha de widget que nao existe mais e inofensiva, do mesmo jeito que a
     tabela de ocultos ja trata. */
  it("ignora linha de widget que saiu do catalogo", () => {
    const arranjo = arrangeHome([{ workspaceId: WORKSPACE, widgetId: "widget_extinto", position: 0, section: "now", span: 12 }], WORKSPACE);
    expect(arranjo).toHaveLength(HOME_WIDGETS.length);
    expect(arranjo.map((slot) => slot.id)).not.toContain("widget_extinto");
  });
});

describe("moveInArrangement", () => {
  const base = () => arrangeHome([], WORKSPACE);

  it("poe antes da mira", () => {
    const next = moveInArrangement(base(), "today_hours", "now", "now");
    expect(idsDa(next, "now")).toEqual(["today_hours", "now", "timer"]);
  });

  /* O caso onde o indice erra por um: o widget sai de ANTES da mira, entao a
     lista encolhe atras dela. Com indice cru, ele cairia uma casa adiante. */
  it("nao erra por um quando o widget vem de tras da mira", () => {
    const next = moveInArrangement(base(), "now", "now", "today_hours");
    expect(idsDa(next, "now")).toEqual(["timer", "now", "today_hours"]);
  });

  it("sem mira, vai para o fim da faixa e nao para o fim da Home", () => {
    const next = moveInArrangement(base(), "now", "now", null);
    expect(idsDa(next, "now")).toEqual(["timer", "today_hours", "now"]);
    // e a Home inteira continua com as faixas na ordem delas
    expect(next.map((slot) => slot.section)).toEqual([...next.map((slot) => slot.section)].sort((left, right) =>
      HOME_SECTIONS.findIndex((s) => s.id === left) - HOME_SECTIONS.findIndex((s) => s.id === right)));
  });

  it("move entre faixas, e o widget para de contar na de origem", () => {
    const next = moveInArrangement(base(), "timer", "utilities", null);
    expect(idsDa(next, "now")).toEqual(["now", "today_hours"]);
    expect(idsDa(next, "utilities")).toEqual(["quick_actions", "system_health", "timer"]);
  });

  it("aceita mira na faixa de destino ao mudar de faixa", () => {
    const next = moveInArrangement(base(), "timer", "utilities", "system_health");
    expect(idsDa(next, "utilities")).toEqual(["quick_actions", "timer", "system_health"]);
  });

  /* Faixa que ficou sem ninguem continua existindo no modo de arrumar, e
     precisa aceitar um widget de volta — sem isso ela seria inalcancavel. */
  it("aceita widget de volta numa faixa esvaziada", () => {
    let next = base();
    for (const id of ["quick_actions", "system_health"]) next = moveInArrangement(next, id, "now", null);
    expect(idsDa(next, "utilities")).toEqual([]);

    next = moveInArrangement(next, "quick_actions", "utilities", null);
    expect(idsDa(next, "utilities")).toEqual(["quick_actions"]);
  });

  it("nao mexe em nada quando o widget nao existe", () => {
    const antes = base();
    expect(moveInArrangement(antes, "nao_existe", "now", null)).toEqual(antes);
  });

  it("nunca perde nem duplica widget", () => {
    let next = base();
    for (const [id, section, before] of [["timer", "overview", null], ["recent", "now", "now"], ["apps", "utilities", null], ["timer", "now", null]] as const) {
      next = moveInArrangement(next, id, section, before);
      expect(next).toHaveLength(HOME_WIDGETS.length);
      expect(new Set(next.map((slot) => slot.id)).size).toBe(HOME_WIDGETS.length);
    }
  });
});

describe("touchedSections", () => {
  it("dentro da faixa, so ela", () => {
    expect(touchedSections(arrangeHome([], WORKSPACE), "timer", "now")).toEqual(["now"]);
  });

  /* A origem tambem mudou: quem ficou la subiu uma posicao, e gravar so o
     destino deixaria a numeracao da origem com um buraco. */
  it("entre faixas, as duas", () => {
    expect(touchedSections(arrangeHome([], WORKSPACE), "timer", "utilities")).toEqual(["now", "utilities"]);
  });
});

describe("placementsFor", () => {
  it("renumera cada faixa a partir de zero", () => {
    const escrita = placementsFor(arrangeHome([], WORKSPACE), ["now", "utilities"]);
    expect(escrita).toEqual([
      { widgetId: "now", position: 0, section: "now", span: null },
      { widgetId: "timer", position: 1, section: "now", span: null },
      { widgetId: "today_hours", position: 2, section: "now", span: null },
      { widgetId: "quick_actions", position: 0, section: "utilities", span: null },
      { widgetId: "system_health", position: 1, section: "utilities", span: null },
    ]);
  });

  /* O contrato que o Rust fixa em `reordering_must_carry_the_stored_width_along`:
     a escrita e autoritativa, entao reordenar tem de REPASSAR a largura
     guardada. Mandar `null` aqui apagaria em silencio o que a pessoa escolheu. */
  it("repassa a largura guardada ao reordenar, em vez de apaga-la", () => {
    const arranjo = arrangeHome([guardado("timer", 0, { span: 12 })], WORKSPACE);
    const escrita = placementsFor(moveInArrangement(arranjo, "timer", "now", null), ["now"]);
    expect(escrita.find((entry) => entry.widgetId === "timer")).toEqual({ widgetId: "timer", position: 2, section: "now", span: 12 });
  });

  /* E nao pode mandar o span RESOLVIDO de quem nunca escolheu largura: isso
     congelaria o desenho de hoje na primeira arrastada. */
  it("nao inventa largura para quem nunca escolheu uma", () => {
    const escrita = placementsFor(arrangeHome([], WORKSPACE), ["now"]);
    for (const entry of escrita) expect(entry.span).toBeNull();
  });
});

describe("stepSpan", () => {
  it("anda na escala do desenho", () => {
    expect(stepSpan(6, 1)).toBe(8);
    expect(stepSpan(6, -1)).toBe(5);
  });

  it("para nas pontas em vez de dar a volta", () => {
    expect(stepSpan(3, -1)).toBeNull();
    expect(stepSpan(12, 1)).toBeNull();
  });

  /* O banco aceita 1..12 e a escala do desenho nao: uma largura de fora nao
     pode travar os dois botoes e prender a pessoa numa largura que a interface
     nem oferece. */
  it("tira do lugar uma largura que nao esta na escala", () => {
    expect(stepSpan(7, 1)).toBe(8);
    expect(stepSpan(7, -1)).toBe(6);
    expect(stepSpan(1, 1)).toBe(3);
    expect(stepSpan(1, -1)).toBeNull();
  });
});

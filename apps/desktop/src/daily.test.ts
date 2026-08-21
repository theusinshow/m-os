import { describe, expect, it } from "vitest";
import {
  avisoDeCarregado,
  carryOverEmOrdem,
  dataPorExtenso,
  destinos,
  emOrdem,
  estadoDoDia,
  horaDe,
  linhaDeHistorico,
  linhaDeObjetivo,
  moverObjetivo,
  podeIniciar,
  progresso,
  rascunho,
  rascunhoValido,
  resumoDoDia,
  saudacao,
  vagasRestantes,
  SECUNDARIOS_SUGERIDOS,
} from "./daily";
import type { CarryOver, DailyContext, DailyObjective, DailySession, DailyToday, ObjectiveStatus, ObjectivePriority } from "./types";

function sessao(day: string, status: "active" | "completed" = "active"): DailySession {
  return {
    id: `s-${day}`,
    day,
    status,
    note: "",
    startedAt: "2026-08-21T12:08:00Z",
    endedAt: status === "completed" ? "2026-08-21T20:48:00Z" : null,
    createdAt: "2026-08-21T12:08:00Z",
    updatedAt: "2026-08-21T12:08:00Z",
  };
}

function objetivo(
  id: string,
  title: string,
  priority: ObjectivePriority,
  status: ObjectiveStatus,
  position = 0,
): DailyObjective {
  return {
    id,
    sessionId: "s",
    title,
    description: "",
    link: null,
    priority,
    status,
    position,
    carriedFrom: null,
    createdAt: "2026-08-21T12:08:00Z",
    updatedAt: "2026-08-21T12:08:00Z",
    completedAt: status === "completed" ? "2026-08-21T15:32:00Z" : null,
  };
}

function dia(over: Partial<DailyToday> = {}): DailyToday {
  return {
    day: "2026-08-21",
    status: "active",
    session: sessao("2026-08-21"),
    objectives: [],
    reflection: null,
    stale: null,
    staleObjectives: [],
    ...over,
  };
}

function contexto(over: Partial<DailyContext> = {}): DailyContext {
  return {
    dueToday: 0,
    overdue: 0,
    highPriority: 0,
    meetingsToday: 0,
    inbox: 0,
    freshCaptures: 0,
    projects: 0,
    doing: 0,
    openTasks: 0,
    suggestedTasks: [],
    suggestedProjects: [],
    carryOver: [],
    carryOverDay: "",
    ...over,
  };
}

describe("estado da Home", () => {
  it("sem sessão e sem ontem em aberto, o dia não começou", () => {
    expect(estadoDoDia(null)).toEqual({ tipo: "nao_iniciado" });
    expect(estadoDoDia(dia({ status: "not_started", session: null }))).toEqual({ tipo: "nao_iniciado" });
  });

  it("ontem em aberto ganha de 'não iniciado' — ignorar a porta aberta faria o histórico mentir", () => {
    const estado = estadoDoDia(
      dia({
        status: "not_started",
        session: null,
        stale: sessao("2026-08-20"),
        staleObjectives: [
          objetivo("a", "planta", "main", "pending"),
          objetivo("b", "memorial", "secondary", "completed"),
        ],
      }),
    );
    expect(estado).toEqual({ tipo: "ontem_aberto", dia: "2026-08-20", pendentes: 1 });
  });

  it("mas não ganha de 'hoje já começou': o backend já fechou a velha", () => {
    const estado = estadoDoDia(dia({ objectives: [objetivo("a", "planta", "main", "pending")] }));
    expect(estado).toEqual({ tipo: "ativo", feitos: 0, total: 1 });
  });

  it("dia encerrado carrega o placar final", () => {
    const estado = estadoDoDia(
      dia({
        status: "completed",
        session: sessao("2026-08-21", "completed"),
        objectives: [
          objetivo("a", "planta", "main", "completed"),
          objetivo("b", "memorial", "secondary", "carried_over"),
        ],
      }),
    );
    expect(estado).toEqual({ tipo: "encerrado", feitos: 1, total: 2 });
  });
});

describe("progresso", () => {
  it("abandonar um objetivo não piora o placar — sai dos dois lados da fração", () => {
    expect(
      progresso([
        objetivo("a", "planta", "main", "completed"),
        objetivo("b", "memorial", "secondary", "pending"),
        objetivo("c", "desisti", "secondary", "dropped"),
      ]),
    ).toEqual({ feitos: 1, total: 2 });
  });

  it("um dia sem objetivos não divide por nada", () => {
    expect(progresso([])).toEqual({ feitos: 0, total: 0 });
  });
});

describe("ordem", () => {
  it("o principal vem primeiro, mesmo com posição maior", () => {
    const lista = [
      objetivo("b", "memorial", "secondary", "pending", 0),
      objetivo("a", "planta", "main", "pending", 5),
      objetivo("c", "arquivos", "secondary", "pending", 1),
    ];
    expect(emOrdem(lista).map((item) => item.id)).toEqual(["a", "b", "c"]);
  });

  it("mover mira um VIZINHO, e não um índice", () => {
    const lista = [
      objetivo("a", "a", "secondary", "pending", 0),
      objetivo("b", "b", "secondary", "pending", 1),
      objetivo("c", "c", "secondary", "pending", 2),
    ];
    // Da esquerda para a direita é onde o índice calculado antes da remoção
    // erra por um. Com vizinho, não erra.
    expect(moverObjetivo(lista, "a", "c").map((item) => item.id)).toEqual(["b", "a", "c"]);
    expect(moverObjetivo(lista, "c", "a").map((item) => item.id)).toEqual(["c", "a", "b"]);
    expect(moverObjetivo(lista, "a", null).map((item) => item.id)).toEqual(["b", "c", "a"]);
    expect(moverObjetivo(lista, "inexistente", "a")).toBe(lista);
  });
});

describe("vagas de foco", () => {
  it("conta só secundários vivos", () => {
    expect(vagasRestantes([])).toBe(SECUNDARIOS_SUGERIDOS);
    expect(
      vagasRestantes([
        objetivo("a", "principal", "main", "pending"),
        objetivo("b", "um", "secondary", "pending"),
        objetivo("c", "largado", "secondary", "dropped"),
      ]),
    ).toBe(SECUNDARIOS_SUGERIDOS - 1);
  });

  it("nunca fica negativo — o excesso é aceito, e não vira erro", () => {
    const muitos = Array.from({ length: 6 }, (_, indice) =>
      objetivo(String(indice), "x", "secondary", "pending", indice),
    );
    expect(vagasRestantes(muitos)).toBe(0);
  });
});

describe("resumo do dia", () => {
  it("zero não vira linha: um painel de zeros é ansiedade com cara de informação", () => {
    expect(resumoDoDia(contexto())).toEqual([]);
    expect(resumoDoDia(null)).toEqual([]);
  });

  it("a ordem é a da urgência, e não a da grandeza", () => {
    const linhas = resumoDoDia(contexto({ inbox: 40, overdue: 2, dueToday: 3 }));
    expect(linhas.map((linha) => linha.chave)).toEqual(["overdue", "dueToday", "inbox"]);
  });

  it("singular e plural saem certos", () => {
    const linhas = resumoDoDia(contexto({ overdue: 1, dueToday: 3 }));
    expect(linhas[0].texto).toBe("1 lembrete atrasado");
    expect(linhas[1].texto).toBe("3 lembretes para hoje");
  });
});

describe("saudação", () => {
  it("é a hora do dia, e nada além", () => {
    expect(saudacao(new Date(2026, 7, 21, 9))).toBe("Bom dia.");
    expect(saudacao(new Date(2026, 7, 21, 14))).toBe("Boa tarde.");
    expect(saudacao(new Date(2026, 7, 21, 21))).toBe("Boa noite.");
  });
});

describe("linha de objetivo", () => {
  it("o principal pendente ganha o marcador cheio", () => {
    expect(linhaDeObjetivo(objetivo("a", "planta", "main", "pending")).marcador).toBe("●");
    expect(linhaDeObjetivo(objetivo("b", "memorial", "secondary", "pending")).marcador).toBe("○");
  });

  it("concluído é ✓ nos dois pesos: o desfecho vale mais que o peso", () => {
    expect(linhaDeObjetivo(objetivo("a", "planta", "main", "completed")).marcador).toBe("✓");
    expect(linhaDeObjetivo(objetivo("b", "memorial", "secondary", "completed")).marcador).toBe("✓");
  });

  it("levado e abandonado se distinguem sem depender de cor", () => {
    const levado = linhaDeObjetivo(objetivo("a", "x", "secondary", "carried_over"));
    const largado = linhaDeObjetivo(objetivo("b", "y", "secondary", "dropped"));
    expect(levado.marcador).not.toBe(largado.marcador);
    expect(levado.estado).toBe("levado para amanhã");
    expect(largado.estado).toBe("abandonado");
  });

  it("só o pendente oferece concluir", () => {
    expect(linhaDeObjetivo(objetivo("a", "x", "main", "pending")).concluivel).toBe(true);
    expect(linhaDeObjetivo(objetivo("a", "x", "main", "completed")).concluivel).toBe(false);
    expect(linhaDeObjetivo(objetivo("a", "x", "main", "carried_over")).concluivel).toBe(false);
  });
});

describe("carry-over", () => {
  it("só fala a partir do segundo adiamento — 'veio de ontem' é ruído", () => {
    expect(avisoDeCarregado(0)).toBe("");
    expect(avisoDeCarregado(1)).toBe("");
    expect(avisoDeCarregado(4)).toBe("adiado 4 vezes");
  });

  it("o mais carregado vem primeiro: é quem mais precisa de decisão", () => {
    const itens: CarryOver[] = [
      { objectiveId: "a", title: "novo", link: null, timesCarried: 0 },
      { objectiveId: "b", title: "velho", link: null, timesCarried: 4 },
      { objectiveId: "c", title: "meio", link: null, timesCarried: 2 },
    ];
    expect(carryOverEmOrdem(itens).map((item) => item.objectiveId)).toEqual(["b", "c", "a"]);
  });
});

describe("rascunhos", () => {
  it("sem vínculo, os dois campos ficam vazios — metade preenchida é recusada pelo domínio", () => {
    expect(rascunho("Resolver pendências", null)).toEqual({
      title: "Resolver pendências",
      linkKind: "",
      linkId: "",
    });
  });

  it("com vínculo, os dois vão juntos", () => {
    expect(rascunho("  Enviar arquivos  ", { kind: "task", id: "t-1" }, "o-9")).toEqual({
      title: "Enviar arquivos",
      linkKind: "task",
      linkId: "t-1",
      carriedFrom: "o-9",
    });
  });

  it("título vazio COM vínculo é válido: o backend preenche pelo título da entidade", () => {
    expect(rascunhoValido(rascunho("", { kind: "task", id: "t-1" }))).toBe(true);
    expect(rascunhoValido(rascunho("   ", null))).toBe(false);
    expect(rascunhoValido(null)).toBe(false);
  });

  it("um objetivo basta para começar, e ele pode ser secundário", () => {
    expect(podeIniciar(null, [])).toBe(false);
    expect(podeIniciar(rascunho("planta", null), [])).toBe(true);
    // Dia sem principal é escolha legítima; travar o botão viraria obstáculo.
    expect(podeIniciar(null, [rascunho("memorial", null)])).toBe(true);
  });
});

describe("destinos", () => {
  it("viram a lista que o backend espera", () => {
    const escolhas = new Map<string, ObjectiveStatus>([
      ["a", "completed"],
      ["b", "carried_over"],
    ]);
    expect(destinos(escolhas)).toEqual([
      { objectiveId: "a", status: "completed" },
      { objectiveId: "b", status: "carried_over" },
    ]);
    expect(destinos(new Map())).toEqual([]);
  });
});

describe("datas", () => {
  it("a data civil NÃO passa por new Date(texto) — isso a jogaria um dia para trás", () => {
    // `new Date("2026-08-21")` é meia-noite UTC; em UTC-3 volta como dia 20.
    expect(dataPorExtenso("2026-08-21")).toContain("21");
    expect(dataPorExtenso("2026-01-01")).toContain("01");
    expect(dataPorExtenso("lixo")).toBe("lixo");
  });

  it("a hora sai de um instante, e instante inválido não vira texto quebrado", () => {
    expect(horaDe(null)).toBe("");
    expect(horaDe("nada disso")).toBe("");
    expect(horaDe("2026-08-21T12:08:00Z")).toMatch(/\d{2}:\d{2}/);
  });

  it("a linha do histórico junta data e placar", () => {
    const linha = linhaDeHistorico({
      session: sessao("2026-08-20", "completed"),
      done: 3,
      total: 4,
      mainTitle: "planta",
      mood: "normal",
    });
    expect(linha.placar).toBe("3/4 objetivos");
    expect(linha.data).toContain("20");
  });

  it("um objetivo no singular não diz 'objetivos'", () => {
    const linha = linhaDeHistorico({
      session: sessao("2026-08-20", "completed"),
      done: 1,
      total: 1,
      mainTitle: "planta",
      mood: null,
    });
    expect(linha.placar).toBe("1/1 objetivo");
  });
});

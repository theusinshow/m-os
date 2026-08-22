import { describe, expect, it } from "vitest";
import {
  avaliadoDe,
  campoDoInstante,
  cronometroDe,
  decorridoDe,
  duracaoDe,
  faixasDe,
  instanteDoCampo,
  mediaDe,
  planoDe,
  quandoDe,
  temHoraReal,
  situacaoDe,
} from "./academic";
import type { Compromisso, Horizonte, SubjectOverview } from "./types";

function compromisso(horizonte: Horizonte, over: Partial<Compromisso> = {}): Compromisso {
  return {
    kind: "assignment",
    id: `id-${horizonte}`,
    title: "Lista 03",
    subjectId: "s1",
    subject: "Estática",
    decision: "none",
    plannedAt: null,
    plannedMinutes: 0,
    subjectAccent: "sodio",
    at: "2026-08-22T23:59:00Z",
    horizonte,
    taskId: null,
    location: "",
    ...over,
  };
}

function disciplina(over: Partial<SubjectOverview> = {}): SubjectOverview {
  return {
    id: "s1",
    name: "Estática",
    code: "EMC5132",
    accent: "sodio",
    pending: 0,
    overdue: 0,
    upcomingExams: 0,
    media: null,
    pesoAvaliado: null,
    studySecondsWeek: 0,
    next: null,
    materials: 0,
    ...over,
  };
}

describe("faixas", () => {
  it("agrupa na ordem da urgência", () => {
    const itens = [
      compromisso("later"),
      compromisso("overdue"),
      compromisso("today"),
    ];
    expect(faixasDe(itens).map((faixa) => faixa.horizonte)).toEqual([
      "overdue",
      "today",
      "later",
    ]);
  });

  it("faixa vazia não vira rótulo", () => {
    // Um "Atrasado" com nada embaixo ensina a ignorar o título.
    const faixas = faixasDe([compromisso("today")]);
    expect(faixas).toHaveLength(1);
    expect(faixas[0].titulo).toBe("Hoje");
  });

  it("sem nada, nenhuma faixa", () => {
    expect(faixasDe([])).toEqual([]);
  });
});

describe("quando", () => {
  const agora = new Date(2026, 7, 22, 12, 0, 0);

  it("para hoje mostra só a hora", () => {
    const hoje = new Date(2026, 7, 22, 23, 59).toISOString();
    expect(quandoDe(hoje, "today", agora)).toBe("23:59");
  });

  it("para amanhã diz amanhã", () => {
    const amanha = new Date(2026, 7, 23, 14, 0).toISOString();
    expect(quandoDe(amanha, "tomorrow", agora)).toBe("amanhã, 14:00");
  });

  it("o atraso conta os dias", () => {
    const ontem = new Date(2026, 7, 21, 10, 0).toISOString();
    expect(quandoDe(ontem, "overdue", agora)).toBe("venceu ontem");
    const antes = new Date(2026, 7, 18, 10, 0).toISOString();
    expect(quandoDe(antes, "overdue", agora)).toBe("venceu há 4 dias");
  });

  it("o que venceu hoje mais cedo diz a hora, e não 'há 0 dias'", () => {
    const cedo = new Date(2026, 7, 22, 9, 0).toISOString();
    expect(quandoDe(cedo, "overdue", agora)).toBe("venceu às 09:00");
  });

  it("na semana, o dia da semana situa sem contar nos dedos", () => {
    const sexta = new Date(2026, 7, 28, 15, 0).toISOString();
    // A hora entra junto desde a camada operacional: "vence sexta 23h59" e
    // "vence sexta" sao decisoes diferentes, e o prazo do Univirtus sempre traz
    // hora de verdade.
    expect(quandoDe(sexta, "this_week", agora)).toBe("sex, 28/08 · 15:00");
  });

  it("o ano só aparece quando muda", () => {
    const esteAno = new Date(2026, 11, 1, 8, 0).toISOString();
    expect(quandoDe(esteAno, "later", agora)).toBe("01/12 · 08:00");
    const outroAno = new Date(2027, 1, 1, 8, 0).toISOString();
    expect(quandoDe(outroAno, "later", agora)).toBe("01/02/2027 · 08:00");
  });

  it("data inválida não vira texto quebrado", () => {
    expect(quandoDe("lixo", "today", agora)).toBe("");
  });
});

describe("duração", () => {
  it("nunca escreve a hora zerada", () => {
    expect(duracaoDe(45 * 60)).toBe("45min");
    expect(duracaoDe(3600)).toBe("1h");
    expect(duracaoDe(3600 + 45 * 60)).toBe("1h 45min");
  });

  it("menos de um minuto não é tempo de estudo", () => {
    expect(duracaoDe(30)).toBe("—");
    expect(duracaoDe(0)).toBe("—");
  });
});

describe("média", () => {
  it("sai com uma casa e vírgula", () => {
    expect(mediaDe(7.25)).toBe("7,3");
    expect(mediaDe(10)).toBe("10,0");
  });

  it("sem nota nenhuma fica vazio, e nunca zero", () => {
    // Zero é uma nota. Uma disciplina sem prova corrigida não tirou zero —
    // ela não tirou nada, e anunciar 0,0 em março seria alarme falso.
    expect(mediaDe(null)).toBe("");
  });

  it("o peso avaliado vira porcentagem", () => {
    expect(avaliadoDe(0.25)).toBe("25% avaliado");
    expect(avaliadoDe(null)).toBe("");
  });
});

describe("situação da disciplina", () => {
  it("o pior estado ganha a frase", () => {
    expect(situacaoDe(disciplina({ overdue: 2, pending: 5, upcomingExams: 1 }))).toBe("2 atrasadas");
    expect(situacaoDe(disciplina({ pending: 1, upcomingExams: 1 }))).toBe("1 pendente");
    expect(situacaoDe(disciplina({ upcomingExams: 1 }))).toBe("1 avaliação marcada");
  });

  it("sem nada pendente, diz que está em dia", () => {
    expect(situacaoDe(disciplina())).toBe("em dia");
  });
});

describe("cronômetro", () => {
  it("conta o decorrido em segundos", () => {
    const inicio = new Date(2026, 7, 22, 12, 0, 0).toISOString();
    const agora = new Date(2026, 7, 22, 12, 45, 12);
    expect(decorridoDe(inicio, agora)).toBe(45 * 60 + 12);
  });

  it("relógio para trás não vira tempo negativo", () => {
    const inicio = new Date(2026, 7, 22, 13, 0, 0).toISOString();
    const agora = new Date(2026, 7, 22, 12, 0, 0);
    expect(decorridoDe(inicio, agora)).toBe(0);
  });

  it("formata com dois dígitos", () => {
    expect(cronometroDe(45 * 60 + 12)).toBe("00:45:12");
    expect(cronometroDe(3661)).toBe("01:01:01");
    expect(cronometroDe(-5)).toBe("00:00:00");
  });
});

describe("o campo de data e hora", () => {
  /**
   * O `datetime-local` entrega texto sem fuso. Lê-lo como UTC jogaria uma
   * entrega das 23h59 para o dia seguinte — e o prazo é justamente onde isso
   * não pode acontecer.
   */
  it("interpreta o valor no fuso de quem digitou", () => {
    const iso = instanteDoCampo("2026-08-29T23:59");
    expect(iso).not.toBeNull();
    const voltou = new Date(iso as string);
    expect(voltou.getFullYear()).toBe(2026);
    expect(voltou.getMonth()).toBe(7);
    expect(voltou.getDate()).toBe(29);
    expect(voltou.getHours()).toBe(23);
    expect(voltou.getMinutes()).toBe(59);
  });

  it("vazio significa sem prazo, e não epoch", () => {
    expect(instanteDoCampo("")).toBeNull();
    expect(instanteDoCampo("   ")).toBeNull();
  });

  it("ida e volta preserva o que a pessoa digitou", () => {
    const digitado = "2026-08-29T23:59";
    expect(campoDoInstante(instanteDoCampo(digitado))).toBe(digitado);
  });

  it("sem instante, o campo abre vazio", () => {
    expect(campoDoInstante(null)).toBe("");
    expect(campoDoInstante("lixo")).toBe("");
  });
});

describe("a hora exata do prazo", () => {
  // 23h59 é hora de verdade, e a diferença entre "vence 23h59" e "vence hoje"
  // é toda a informação que o Univirtus manda junto do prazo.
  it("mostra a hora quando ela existe", () => {
    const iso = new Date(2026, 8, 14, 23, 59).toISOString();
    expect(temHoraReal(iso)).toBe(true);
    expect(quandoDe(iso, "later", new Date(2026, 7, 22))).toContain("23:59");
  });

  // Meia-noite é ausência de hora. Mostrar "00:00" afirmaria uma precisão que
  // ninguém informou, e inventar 23:59 no lugar seria pior ainda.
  it("omite a hora na meia-noite em vez de inventar uma", () => {
    const iso = new Date(2026, 8, 14, 0, 0).toISOString();
    expect(temHoraReal(iso)).toBe(false);
    const texto = quandoDe(iso, "later", new Date(2026, 7, 22));
    expect(texto).not.toContain("00:00");
    expect(texto).not.toContain("23:59");
    expect(texto).toContain("14/09");
  });

  it("vale também para a faixa da semana", () => {
    const comHora = new Date(2026, 7, 26, 19, 30).toISOString();
    const semHora = new Date(2026, 7, 26, 0, 0).toISOString();
    expect(quandoDe(comHora, "this_week", new Date(2026, 7, 22))).toContain("19:30");
    expect(quandoDe(semHora, "this_week", new Date(2026, 7, 22))).not.toContain("00:00");
  });
});

describe("o bloco planejado", () => {
  it("diz quando e por quanto tempo", () => {
    const iso = new Date(2026, 7, 26, 19, 30).toISOString();
    expect(planoDe(iso, 60)).toBe("26/08 · 19:30 · 1h");
    expect(planoDe(iso, 90)).toBe("26/08 · 19:30 · 1h 30min");
    expect(planoDe(iso, 30)).toBe("26/08 · 19:30 · 30min");
  });

  it("omite a duração quando ela não foi definida", () => {
    const iso = new Date(2026, 7, 26, 19, 30).toISOString();
    expect(planoDe(iso, 0)).toBe("26/08 · 19:30");
  });

  it("sem plano, não diz nada", () => {
    expect(planoDe(null, 60)).toBe("");
  });
});

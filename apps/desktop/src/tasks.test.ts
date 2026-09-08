import { describe, expect, it } from "vitest";

import {
  checklistCompleto,
  doCampoLocal,
  edicaoDe,
  estimativaCurta,
  linhasColadas,
  paraCampoLocal,
  prazoCurto,
  progressoDe,
  situacaoDoPrazo,
} from "./tasks";
import type { Task } from "./types";

function task(mudanca: Partial<Task> = {}): Task {
  return {
    id: "018f-1",
    title: "Revisar projeto estrutural",
    description: "",
    projectId: null,
    sourceCaptureId: null,
    state: "backlog",
    lifecycleState: "active",
    dueAt: null,
    priority: "normal",
    estimateMinutes: null,
    parentTaskId: null,
    blockedByTaskId: null,
    waitingFor: "",
    followUpAt: null,
    checklistTotal: 0,
    checklistDone: 0,
    createdAt: "2026-09-01T12:00:00Z",
    updatedAt: "2026-09-01T12:00:00Z",
    completedAt: null,
    ...mudanca,
  };
}

describe("edicaoDe", () => {
  /* O teste que importa mais nesta suíte. A escrita é autoritativa campo por
     campo, então uma edição que esquece um campo o APAGA — e o esquecimento
     não daria erro nenhum, daria um prazo que sumiu ao trocar a prioridade. */
  it("carrega os onze campos, para que mexer num não apague os outros", () => {
    const original = task({
      dueAt: "2026-09-08T20:00:00Z",
      priority: "high",
      estimateMinutes: 30,
      waitingFor: "Victor",
      followUpAt: "2026-09-12T12:00:00Z",
      blockedByTaskId: "018f-2",
      parentTaskId: "018f-3",
      projectId: "167-25",
      description: "cobrimento de 5 cm",
    });
    const edicao = { ...edicaoDe(original), priority: "urgent" as const };
    expect(edicao).toEqual({
      id: "018f-1",
      title: "Revisar projeto estrutural",
      description: "cobrimento de 5 cm",
      projectId: "167-25",
      dueAt: "2026-09-08T20:00:00Z",
      priority: "urgent",
      estimateMinutes: 30,
      parentTaskId: "018f-3",
      blockedByTaskId: "018f-2",
      waitingFor: "Victor",
      followUpAt: "2026-09-12T12:00:00Z",
    });
  });

  it("deixa passar o null que TIRA o prazo", () => {
    const edicao = { ...edicaoDe(task({ dueAt: "2026-09-08T20:00:00Z" })), dueAt: null };
    expect(edicao.dueAt).toBeNull();
  });
});

describe("progresso", () => {
  it("é null sem checklist, e não zero", () => {
    /* Uma barra vazia diz "começou e não andou"; uma Task sem passos não
       começou nada. É a diferença entre a tela desenhar a barra e não. */
    expect(progressoDe(task())).toBeNull();
    expect(progressoDe(task({ checklistTotal: 6, checklistDone: 0 }))).toBe(0);
    expect(progressoDe(task({ checklistTotal: 6, checklistDone: 3 }))).toBe(0.5);
  });

  it("oferece concluir só quando tudo está feito e a Task está aberta", () => {
    expect(checklistCompleto(task({ checklistTotal: 5, checklistDone: 4 }))).toBe(false);
    expect(checklistCompleto(task({ checklistTotal: 5, checklistDone: 5 }))).toBe(true);
    expect(checklistCompleto(task({ checklistTotal: 5, checklistDone: 5, state: "done" }))).toBe(false);
    expect(checklistCompleto(task())).toBe(false);
  });
});

describe("prazo", () => {
  const agora = new Date("2026-09-08T10:00:00-03:00");

  it("classifica pelo que muda a decisão", () => {
    expect(situacaoDoPrazo(task(), agora)).toBe("sem_prazo");
    expect(situacaoDoPrazo(task({ dueAt: "2026-09-07T17:00:00-03:00" }), agora)).toBe("atrasado");
    expect(situacaoDoPrazo(task({ dueAt: "2026-09-08T17:00:00-03:00" }), agora)).toBe("hoje");
    expect(situacaoDoPrazo(task({ dueAt: "2026-09-10T17:00:00-03:00" }), agora)).toBe("em_breve");
    expect(situacaoDoPrazo(task({ dueAt: "2026-10-10T17:00:00-03:00" }), agora)).toBe("distante");
  });

  /* Cobrar prazo de trabalho entregue é o comportamento que faz as pessoas
     pararem de olhar para os avisos do sistema. */
  it("nunca chama de atrasada uma Task concluída", () => {
    const feita = task({ dueAt: "2026-09-01T17:00:00-03:00", state: "done", completedAt: "2026-09-02T10:00:00Z" });
    expect(situacaoDoPrazo(feita, agora)).toBe("distante");
  });

  it("escreve curto, porque divide a linha com o Project", () => {
    expect(prazoCurto("2026-09-08T17:00:00-03:00", agora)).toBe("Hoje 17:00");
    expect(prazoCurto("2026-09-09T09:30:00-03:00", agora)).toBe("Amanhã 09:30");
    expect(prazoCurto("2026-09-07T17:00:00-03:00", agora)).toBe("Ontem 17:00");
    expect(prazoCurto("2026-09-20T00:00:00-03:00", agora)).toBe("20 set");
  });

  /* O `<input type="datetime-local">` fala em hora local; o M/OS grava UTC. As
     duas conversões vivem juntas para que a ida e a volta não ganhem um fuso a
     mais e o prazo não ande três horas por edição. */
  it("vai e volta do campo local sem escorregar", () => {
    const iso = new Date("2026-09-08T17:00:00-03:00").toISOString();
    const campo = paraCampoLocal(iso);
    expect(doCampoLocal(campo)).toBe(iso);
  });

  it("campo vazio é ausência de prazo, e não data inválida", () => {
    expect(doCampoLocal("")).toBeNull();
    expect(doCampoLocal("   ")).toBeNull();
    expect(paraCampoLocal(null)).toBe("");
  });
});

describe("estimativa", () => {
  it("escreve como se fala", () => {
    expect(estimativaCurta(null)).toBe("");
    expect(estimativaCurta(0)).toBe("");
    expect(estimativaCurta(15)).toBe("15 min");
    expect(estimativaCurta(60)).toBe("1h");
    expect(estimativaCurta(90)).toBe("1h30");
  });
});

describe("colagem", () => {
  /* Só CONTA. Quem divide é o domínio, no backend — duplicar a regra aqui daria
     duas respostas para a mesma colagem. */
  it("conta as linhas com conteúdo", () => {
    expect(linhasColadas("um")).toBe(1);
    expect(linhasColadas("um\n\ndois\ntrês\n  ")).toBe(3);
    expect(linhasColadas("")).toBe(0);
  });
});

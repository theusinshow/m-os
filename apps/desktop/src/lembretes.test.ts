import { describe, expect, it } from "vitest";

import {
  acaoPrincipal,
  alertasPendentes,
  bucketOf,
  perguntaDoCartao,
  recurrenceLabel,
  snoozeChoices,
  triggerLabel,
  whenChoices,
  whenLabel,
} from "./lembretes";
import type { Reminder, ReminderTrigger } from "./types";

function lembrete(mudanca: Partial<Reminder> = {}): Reminder {
  return {
    id: "018f-1",
    title: "Enviar as bases para o Victor",
    body: "",
    target: null,
    trigger: { kind: "at", instant: "2026-09-08T20:30:00Z" },
    priority: "normal",
    status: "scheduled",
    policy: { snoozeAllowed: true, privacy: "show_content" },
    source: "user",
    nextDueAt: "2026-09-08T20:30:00Z",
    snoozeCount: 0,
    deliveredCount: 0,
    kind: "standard",
    waitingFor: "",
    persistent: false,
    escalationStep: 0,
    lastTriggeredAt: null,
    retryAt: null,
    recurrence: null,
    createdAt: "2026-09-08T12:00:00Z",
    updatedAt: "2026-09-08T12:00:00Z",
    completedAt: null,
    lifecycleState: "active",
    ...mudanca,
  };
}

function gatilho(mudanca: Partial<ReminderTrigger> = {}): ReminderTrigger {
  return {
    id: "t1",
    reminderId: "018f-1",
    scheduledAt: "2026-09-08T20:30:00Z",
    kind: "lead",
    leadMinutes: 60,
    status: "pending",
    firedAt: null,
    lifecycleState: "active",
    createdAt: "2026-09-08T12:00:00Z",
    updatedAt: "2026-09-08T12:00:00Z",
    ...mudanca,
  };
}

describe("bucketOf", () => {
  const agora = Date.parse("2026-09-08T14:00:00Z");

  it("o que o domínio marcou como esquecido vai para o topo, sem segunda opinião", () => {
    // Um lembrete FUTURO que o domínio pôs em Needs Attention continua lá: a
    // tela não recalcula a regra, ela obedece. Duas implementações da mesma
    // regra divergem, e aí a lista discorda do badge.
    const futuro = lembrete({ nextDueAt: "2026-09-20T09:00:00Z" });
    expect(bucketOf(futuro, new Set([futuro.id]), agora)).toBe("attention");
  });

  it("separa hoje de em breve pelo fim do dia local", () => {
    const hoje = lembrete({ nextDueAt: "2026-09-08T20:30:00Z" });
    const depois = lembrete({ nextDueAt: "2026-09-12T09:00:00Z" });
    expect(bucketOf(hoje, new Set(), agora)).toBe("today");
    expect(bucketOf(depois, new Set(), agora)).toBe("upcoming");
  });

  it("adiado tem grupo próprio, mesmo com hora de hoje", () => {
    const adiado = lembrete({ status: "snoozed", nextDueAt: "2026-09-08T22:00:00Z" });
    expect(bucketOf(adiado, new Set(), agora)).toBe("snoozed");
  });

  it("sem data é algum dia, e não um lembrete quebrado", () => {
    const solto = lembrete({ trigger: { kind: "someday" }, nextDueAt: null });
    expect(bucketOf(solto, new Set(), agora)).toBe("someday");
  });
});

describe("whenLabel", () => {
  const agora = Date.parse("2026-09-08T14:00:00Z");

  it("diz se falta ou se passou, e nunca só o número", () => {
    expect(whenLabel(lembrete({ nextDueAt: "2026-09-08T16:00:00Z" }), agora)).toBe("em 2 h");
    expect(whenLabel(lembrete({ nextDueAt: "2026-09-08T12:00:00Z" }), agora)).toBe("atrasado 2 h");
  });

  it("adiado conta quando volta, e diz que foi adiado", () => {
    const adiado = lembrete({ status: "snoozed", nextDueAt: "2026-09-08T15:00:00Z" });
    expect(whenLabel(adiado, agora)).toBe("adiado · volta em 1 h");
  });

  it("sem data não inventa hora nenhuma", () => {
    expect(whenLabel(lembrete({ nextDueAt: null }), agora)).toBe("sem data");
  });
});

describe("snoozeChoices", () => {
  it("às onze da noite não oferece 'à noite'", () => {
    const rotulos = snoozeChoices(new Date(2026, 8, 8, 23, 0)).map((escolha) => escolha.label);
    expect(rotulos).not.toContain("À noite");
    expect(rotulos).not.toContain("Mais tarde");
    expect(rotulos).toContain("Amanhã 9h");
  });

  it("de manhã oferece o dia inteiro", () => {
    const rotulos = snoozeChoices(new Date(2026, 8, 8, 9, 0)).map((escolha) => escolha.label);
    expect(rotulos).toContain("Mais tarde");
    expect(rotulos).toContain("À noite");
  });

  it("todo adiamento oferecido cai no futuro", () => {
    for (const hora of [0, 6, 12, 18, 23]) {
      const agora = new Date(2026, 8, 8, hora, 30);
      for (const escolha of snoozeChoices(agora)) {
        expect(escolha.resolve().getTime()).toBeGreaterThan(agora.getTime());
      }
    }
  });
});

describe("whenChoices", () => {
  it("não oferece uma hora que já passou", () => {
    const rotulos = whenChoices(new Date(2026, 8, 8, 19, 0)).map((escolha) => escolha.label);
    expect(rotulos).not.toContain("Hoje 18h");
  });

  it("segunda numa segunda quer dizer a próxima", () => {
    // 2026-09-07 é uma segunda.
    const segunda = new Date(2026, 8, 7, 10, 0);
    const escolhida = whenChoices(segunda).find((escolha) => escolha.label === "Segunda 9h");
    expect(escolhida?.resolve().getDate()).toBe(14);
  });
});

describe("pilha de alertas", () => {
  it("conta só o que ainda vai tocar", () => {
    const pilha = [
      gatilho({ id: "a", status: "fired" }),
      gatilho({ id: "b" }),
      gatilho({ id: "c" }),
      gatilho({ id: "d", status: "cancelled" }),
    ];
    expect(alertasPendentes(pilha)).toBe(2);
  });

  it("cada alerta se lê em palavras, e não como uma segunda data", () => {
    expect(triggerLabel(gatilho({ kind: "at_due", leadMinutes: null }))).toBe("no prazo");
    expect(triggerLabel(gatilho({ leadMinutes: 1440 }))).toBe("1 dia antes");
    expect(triggerLabel(gatilho({ leadMinutes: 240 }))).toBe("4 horas antes");
    expect(triggerLabel(gatilho({ leadMinutes: 15 }))).toBe("15 min antes");
  });
});

describe("follow-up", () => {
  it("um follow-up faz outra pergunta, e o botão muda com ela", () => {
    const cobranca = lembrete({ kind: "follow_up", waitingFor: "Victor" });
    expect(perguntaDoCartao(cobranca)).toBe("Victor respondeu?");
    expect(acaoPrincipal(cobranca)).toBe("Respondeu");
  });

  it("um lembrete comum não faz pergunta nenhuma", () => {
    expect(perguntaDoCartao(lembrete())).toBeNull();
    expect(acaoPrincipal(lembrete())).toBe("Concluir");
  });
});

describe("recurrenceLabel", () => {
  it("lê a regra em português, com a hora local dela", () => {
    const base = { anchor: "fixed" as const, hour: 8, minute: 0, offsetMinutes: -180 };
    expect(recurrenceLabel(lembrete({ recurrence: { rule: { kind: "daily" }, ...base } }))).toBe(
      "Todo dia às 08:00",
    );
    expect(recurrenceLabel(lembrete({ recurrence: { rule: { kind: "weekdays" }, ...base } }))).toBe(
      "Todo dia útil às 08:00",
    );
    expect(
      recurrenceLabel(
        lembrete({ recurrence: { rule: { kind: "weekly", days: [3, 0] }, ...base } }),
      ),
    ).toBe("Toda segunda, quinta às 08:00");
    expect(
      recurrenceLabel(
        lembrete({
          recurrence: { rule: { kind: "monthly", day: { kind: "lastBusinessDay" } }, ...base },
        }),
      ),
    ).toBe("Último dia útil do mês às 08:00");
  });

  it("a repetição por conclusão diz que conta a partir de concluir", () => {
    const rotulo = recurrenceLabel(
      lembrete({
        recurrence: {
          rule: { kind: "everyDays", days: 30 },
          anchor: "completion",
          hour: 9,
          minute: 0,
          offsetMinutes: -180,
        },
      }),
    );
    expect(rotulo).toBe("A cada 30 dias às 09:00, depois de concluir");
  });

  it("sem repetição não inventa rótulo", () => {
    expect(recurrenceLabel(lembrete())).toBeNull();
  });
});

import { describe, expect, it } from "vitest";
import type { VoiceResult } from "./api";
import {
  amplitudeScale,
  formatElapsed,
  formatWhen,
  receiptOf,
  refusalLabel,
  remainingWarning,
} from "./voiceHud";

/** Quarta-feira, 19 de agosto de 2026, 14h32. */
const AGORA = new Date(2026, 7, 19, 14, 32, 0);

function resultado(partes: Partial<VoiceResult> = {}): VoiceResult {
  return {
    noteId: "nota",
    captureId: "captura",
    transcript: "me lembra amanhã às nove de revisar o memorial",
    title: "Revisar o memorial",
    action: "create_task_with_reminder",
    confidence: "high",
    executed: true,
    taskId: "task",
    reminderId: "reminder",
    projectId: null,
    projectName: "",
    projectFromContext: false,
    whenRaw: "amanhã às nove",
    when: new Date(2026, 7, 20, 9, 0, 0).toISOString(),
    hedged: false,
    undo: null,
    receiptMs: 6000,
    ...partes,
  };
}

describe("amplitude", () => {
  it("nunca some, mesmo em silêncio", () => {
    // Um traço que desaparece durante uma pausa parece um microfone que caiu.
    for (let i = 0; i < 4; i += 1) {
      expect(amplitudeScale(0, i)).toBeGreaterThan(0);
    }
  });

  it("os quatro traços não sobem juntos na mesma altura", () => {
    // Quatro barras iguais são um medidor, não uma voz.
    const alturas = [0, 1, 2, 3].map((i) => amplitudeScale(600, i));
    expect(new Set(alturas).size).toBeGreaterThan(1);
  });

  it("cresce com a voz e não passa do teto", () => {
    expect(amplitudeScale(900, 1)).toBeGreaterThan(amplitudeScale(200, 1));
    expect(amplitudeScale(100_000, 1)).toBeLessThanOrEqual(1);
  });
});

describe("relógio", () => {
  it("conta em minutos e segundos", () => {
    expect(formatElapsed(0)).toBe("0:00");
    expect(formatElapsed(4_200)).toBe("0:04");
    expect(formatElapsed(65_000)).toBe("1:05");
  });

  it("o aviso do teto só aparece nos últimos dez segundos", () => {
    expect(remainingWarning(1_000, 120_000)).toBeNull();
    expect(remainingWarning(115_000, 120_000)).toBe("5s");
  });
});

describe("prazo", () => {
  it("hoje e amanhã se chamam pelo nome", () => {
    expect(formatWhen(new Date(2026, 7, 19, 18, 0).toISOString(), AGORA)).toBe("Hoje · 18:00");
    expect(formatWhen(new Date(2026, 7, 20, 9, 0).toISOString(), AGORA)).toBe("Amanhã · 09:00");
  });

  it("dentro da semana, o dia da semana localiza melhor que a data", () => {
    expect(formatWhen(new Date(2026, 7, 21, 14, 0).toISOString(), AGORA)).toBe("sexta · 14:00");
  });

  it("fora da semana, a data", () => {
    expect(formatWhen(new Date(2026, 8, 25, 9, 0).toISOString(), AGORA)).toBe("25/09 · 09:00");
  });

  it("uma data ilegível não vira texto quebrado na tela", () => {
    expect(formatWhen("nao e uma data", AGORA)).toBe("");
  });
});

describe("recibo", () => {
  it("o que foi FEITO diz o que fez", () => {
    const recibo = receiptOf(resultado(), AGORA);
    expect(recibo.headline).toBe("LEMBRETE CRIADO");
    expect(recibo.subject).toBe("Revisar o memorial");
    expect(recibo.meta).toBe("Amanhã · 09:00");
    expect(recibo.offer).toBeNull();
  });

  it("uma Task sem lembrete não promete um lembrete", () => {
    const recibo = receiptOf(
      resultado({ action: "create_task", reminderId: null, when: null, whenRaw: "" }),
      AGORA,
    );
    expect(recibo.headline).toBe("TASK CRIADA");
    expect(recibo.meta).toBe("");
  });

  it("a confiança média OFERECE, e a Capture já está salva atrás", () => {
    const recibo = receiptOf(
      resultado({
        confidence: "medium",
        executed: false,
        taskId: null,
        reminderId: null,
        action: "create_task",
        title: "Comprar café",
        transcript: "Comprar café.",
        when: null,
        whenRaw: "",
      }),
      AGORA,
    );
    expect(recibo.headline).toBe("CAPTURADO");
    expect(recibo.subject).toBe("Comprar café");
    expect(recibo.offer).toBe("Criar Task ⏎");
  });

  it("o que não foi compreendido mostra a fala, e não um título inventado", () => {
    const recibo = receiptOf(
      resultado({
        action: "keep",
        confidence: "low",
        executed: false,
        taskId: null,
        reminderId: null,
        transcript: "Talvez eu devesse olhar aquele memorial.",
        title: "Olhar aquele memorial",
        hedged: true,
        when: null,
        whenRaw: "",
      }),
      AGORA,
    );
    expect(recibo.subject).toBe("Talvez eu devesse olhar aquele memorial.");
    expect(recibo.offer).toBeNull();
  });

  it("o Project que veio da tela se anuncia como palpite", () => {
    // Um palpite que não se anuncia é indistinguível de uma afirmação.
    const doContexto = receiptOf(
      resultado({ projectName: "NexoDoc", projectFromContext: true }),
      AGORA,
    );
    expect(doContexto.meta).toContain("NexoDoc (contexto)");

    const dito = receiptOf(resultado({ projectName: "NexoDoc", projectFromContext: false }), AGORA);
    expect(dito.meta).toContain("NexoDoc");
    expect(dito.meta).not.toContain("contexto");
  });
});

describe("recusas antes da transcrição", () => {
  it("cada uma diz o que aconteceu, e nenhuma oferece desfazer", () => {
    expect(refusalLabel("tooShort")).toBe("Curto demais");
    expect(refusalLabel("tooQuiet")).toBe("Não ouvi nada");
  });
});

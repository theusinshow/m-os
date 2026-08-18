import { describe, expect, it } from "vitest";
import { type ArgosSignals, eyesFor, poseFor, weightFor } from "./argosPose";

const CALMO: ArgosSignals = { hermes: "idle", busy: false, boot: "ready", timerRunning: false };

describe("poseFor", () => {
  it("sem nada acontecendo, fica desperto", () => {
    expect(poseFor(CALMO)).toBe("desperto");
  });

  it("aprovacao pendente ganha de tudo, inclusive de falha", () => {
    expect(poseFor({ ...CALMO, hermes: "waiting", boot: "error", timerRunning: true })).toBe("encarando");
  });

  it("falha ganha de trabalho", () => {
    expect(poseFor({ ...CALMO, hermes: "failed" })).toBe("assustado");
    expect(poseFor({ ...CALMO, boot: "error", busy: true })).toBe("assustado");
  });

  it("hermes trabalhando ganha de ocupado e de cronometro", () => {
    expect(poseFor({ ...CALMO, hermes: "working", busy: true, timerRunning: true })).toBe("trabalhando");
  });

  it("boot e busy fecham os olhos, e ganham do cronometro", () => {
    expect(poseFor({ ...CALMO, boot: "loading" })).toBe("fechado");
    expect(poseFor({ ...CALMO, busy: true, timerRunning: true })).toBe("fechado");
  });

  it("cronometro sozinho concentra", () => {
    expect(poseFor({ ...CALMO, timerRunning: true })).toBe("concentrado");
  });

  // A conexao NAO entra: hermes nunca configurado e `offline`, e um Argos
  // permanentemente aterrorizado seria o resultado de trata-la como falha.
  it("nao existe sinal de conexao em ArgosSignals", () => {
    expect(Object.keys(CALMO)).toEqual(["hermes", "busy", "boot", "timerRunning"]);
  });
});

describe("weightFor", () => {
  it("chama so quando o sistema nao continua sozinho", () => {
    expect(weightFor("encarando")).toBe("chamando");
    expect(weightFor("assustado")).toBe("chamando");
  });

  it("repousa so quando nada acontece", () => {
    expect(weightFor("desperto")).toBe("repouso");
  });

  it("as demais ficam atentas", () => {
    expect(weightFor("trabalhando")).toBe("atento");
    expect(weightFor("concentrado")).toBe("atento");
    expect(weightFor("fechado")).toBe("atento");
  });
});

describe("eyesFor", () => {
  const POSES = ["desperto", "trabalhando", "encarando", "concentrado", "fechado", "assustado"] as const;

  it("todas as poses tem os dois olhos dentro do corpo", () => {
    for (const pose of POSES) {
      const { left, right } = eyesFor(pose);
      for (const eye of [left, right]) {
        expect(eye.x - eye.rx).toBeGreaterThanOrEqual(1);
        expect(eye.x + eye.rx).toBeLessThanOrEqual(23);
        expect(eye.y - eye.ry).toBeGreaterThanOrEqual(1);
        expect(eye.y + eye.ry).toBeLessThanOrEqual(23);
      }
    }
  });

  it("o olho esquerdo esta sempre a esquerda do direito", () => {
    for (const pose of POSES) {
      const { left, right } = eyesFor(pose);
      expect(left.x).toBeLessThan(right.x);
    }
  });

  it("fechado e a pose mais fina, encarando e a mais aberta", () => {
    const alturas = POSES.map((pose) => ({ pose, ry: eyesFor(pose).left.ry }));
    const menor = alturas.reduce((a, b) => (b.ry < a.ry ? b : a));
    const maior = alturas.reduce((a, b) => (b.ry > a.ry ? b : a));
    expect(menor.pose).toBe("fechado");
    expect(maior.pose).toBe("encarando");
  });

  it("trabalhando desvia o olhar para o lado", () => {
    expect(eyesFor("trabalhando").left.x).toBeGreaterThan(eyesFor("desperto").left.x);
  });

  it("so o assustado inclina", () => {
    for (const pose of POSES) {
      const { left, right } = eyesFor(pose);
      if (pose === "assustado") {
        expect(left.tilt).not.toBe(0);
        expect(left.tilt).toBe(-right.tilt);
      } else {
        expect(left.tilt).toBe(0);
        expect(right.tilt).toBe(0);
      }
    }
  });
});

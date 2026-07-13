import { describe, expect, it } from "vitest";
import {
  billableDuration,
  elapsedSeconds,
  netDuration,
} from "./duration";
import type { ActiveTimer } from "@/types/domain";

function makeTimer(overrides: Partial<ActiveTimer>): ActiveTimer {
  return {
    id: "t1",
    projectId: "p1",
    startedAt: "2026-07-11T08:00:00Z",
    lastResumedAt: "2026-07-11T08:00:00Z",
    accumulatedSeconds: 0,
    status: "running",
    description: null,
    activityType: "drawing",
    createdAt: "2026-07-11T08:00:00Z",
    updatedAt: "2026-07-11T08:00:00Z",
    ...overrides,
  };
}

describe("elapsedSeconds", () => {
  it("soma o tempo desde lastResumedAt quando rodando", () => {
    const timer = makeTimer({
      lastResumedAt: "2026-07-11T08:00:00Z",
      accumulatedSeconds: 100,
    });
    const now = Date.parse("2026-07-11T08:00:30Z"); // +30s
    expect(elapsedSeconds(timer, now)).toBe(130);
  });

  it("retorna apenas o acumulado quando pausado", () => {
    const timer = makeTimer({ status: "paused", accumulatedSeconds: 250 });
    const now = Date.parse("2026-07-11T09:00:00Z");
    expect(elapsedSeconds(timer, now)).toBe(250);
  });

  it("nunca reduz o acumulado se o relogio voltar para tras", () => {
    const timer = makeTimer({
      lastResumedAt: "2026-07-11T08:00:00Z",
      accumulatedSeconds: 500,
    });
    const now = Date.parse("2026-07-11T07:59:00Z"); // relogio para tras
    expect(elapsedSeconds(timer, now)).toBe(500);
  });
});

describe("netDuration", () => {
  it("desconta o tempo inativo da duracao bruta", () => {
    expect(netDuration(3600, 600)).toBe(3000);
  });
  it("nunca retorna negativo", () => {
    expect(netDuration(300, 600)).toBe(0);
  });
});

describe("billableDuration", () => {
  it("retorna a liquida quando faturavel", () => {
    expect(billableDuration(3000, true)).toBe(3000);
  });
  it("retorna 0 quando nao faturavel", () => {
    expect(billableDuration(3000, false)).toBe(0);
  });
});

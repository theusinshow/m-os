import { describe, expect, it } from "vitest";
import { inspectEntry } from "./suspiciousEntry";
import type { TimeEntry } from "./types";

function entry(overrides: Partial<TimeEntry>): TimeEntry {
  return {
    id: "018f0000-0000-7000-8000-000000000001",
    projectId: "018f0000-0000-7000-8000-000000000002",
    startedAt: new Date(2026, 7, 16, 13, 0).toISOString(),
    endedAt: new Date(2026, 7, 16, 15, 0).toISOString(),
    durationSeconds: 7200,
    idleSeconds: 0,
    description: "",
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 3000,
    source: "timer",
    ...overrides,
  };
}

describe("inspectEntry", () => {
  it("nao marca uma sessao normal de duas horas", () => {
    expect(inspectEntry(entry({}))).toEqual([]);
  });

  it("marca cronometro acima de oito horas", () => {
    expect(inspectEntry(entry({ durationSeconds: 9 * 3600 }))).toContain("muito-longa");
  });

  /**
   * Sessão manual foi digitada de propósito e a reconstruída nasce de uma
   * decisão explícita. Marcar as duas seria alarme falso — e alarme falso
   * ensina a ignorar o alarme.
   */
  it("nao marca sessao manual, por mais longa que seja", () => {
    expect(inspectEntry(entry({ durationSeconds: 20 * 3600, source: "manual" }))).toEqual([]);
  });

  it("nao marca cronometro ainda em andamento", () => {
    expect(inspectEntry(entry({ durationSeconds: 20 * 3600, endedAt: null }))).toEqual([]);
  });

  /**
   * O caso que a folga de duas horas existe para separar: um cronômetro
   * PAUSADO e retomado dias depois atravessa madrugadas sem ter rodado nelas.
   * Sem a folga, toda sessão pausada durante a noite viraria suspeita.
   */
  it("nao marca cronometro pausado que atravessou a noite parado", () => {
    const started = new Date(2026, 6, 28, 14, 0);
    const ended = new Date(2026, 6, 30, 14, 0);
    const reasons = inspectEntry(
      entry({
        startedAt: started.toISOString(),
        endedAt: ended.toISOString(),
        durationSeconds: 25 * 60,
      }),
    );
    expect(reasons).not.toContain("madrugada");
  });
});

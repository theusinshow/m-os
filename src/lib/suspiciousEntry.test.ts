import { describe, expect, it } from "vitest";
import { inspectEntry, LONG_SESSION_HOURS } from "./suspiciousEntry";
import type { TimeEntry, TimeEntrySource } from "@/types/domain";

/**
 * Constroi um ISO UTC a partir de componentes de horario LOCAL. A regra e
 * definida em horario local, entao o teste precisa ser independente do fuso
 * da maquina que roda a suite.
 */
function localIso(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute = 0,
): string {
  return new Date(year, month - 1, day, hour, minute, 0, 0).toISOString();
}

interface EntryOverrides {
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number;
  source?: TimeEntrySource;
}

function entry({
  startedAt,
  endedAt,
  durationSeconds,
  source = "timer",
}: EntryOverrides): TimeEntry {
  return {
    id: "e1",
    projectId: "p1",
    startedAt,
    endedAt,
    durationSeconds,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 9000,
    source,
    createdAt: startedAt,
    updatedAt: startedAt,
    deletedAt: null,
  };
}

describe("inspectEntry", () => {
  it("nao marca uma jornada exatamente no limite de horas", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 9),
        endedAt: localIso(2026, 8, 10, 17),
        durationSeconds: LONG_SESSION_HOURS * 3600,
      }),
    );

    expect(result.suspicious).toBe(false);
    expect(result.reasons).toEqual([]);
  });

  it("marca um segundo acima do limite como muito longa", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 9),
        endedAt: localIso(2026, 8, 10, 17),
        durationSeconds: LONG_SESSION_HOURS * 3600 + 1,
      }),
    );

    expect(result.suspicious).toBe(true);
    expect(result.reasons).toEqual(["muito-longa"]);
  });

  it("marca uma sessao curta que atravessa as 04:00 locais", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 3, 30),
        endedAt: localIso(2026, 8, 10, 4, 30),
        durationSeconds: 3600,
      }),
    );

    expect(result.suspicious).toBe(true);
    expect(result.reasons).toEqual(["madrugada"]);
  });

  it("marca uma sessao que comeca exatamente as 04:00 locais", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 4),
        endedAt: localIso(2026, 8, 10, 5),
        durationSeconds: 3600,
      }),
    );

    expect(result.reasons).toEqual(["madrugada"]);
  });

  it("nao marca uma sessao curta em horario comercial", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 9),
        endedAt: localIso(2026, 8, 10, 10),
        durationSeconds: 3600,
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("ignora sessao manual, por mais longa que seja", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22),
        endedAt: localIso(2026, 8, 11, 22),
        durationSeconds: 86400,
        source: "manual",
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("ignora sessao reconstruida", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22),
        endedAt: localIso(2026, 8, 11, 22),
        durationSeconds: 86400,
        source: "reconstructed",
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("ignora sessao ainda em aberto: cronometro rodando nao e erro", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22),
        endedAt: null,
        durationSeconds: 86400,
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("acumula os dois motivos no cronometro esquecido a noite", () => {
    // Caso real que originou esta regra: 10/08 22:33 -> 11/08 22:46.
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22, 33),
        endedAt: localIso(2026, 8, 11, 22, 46),
        durationSeconds: 87160,
      }),
    );

    expect(result.suspicious).toBe(true);
    expect(result.reasons).toEqual(["muito-longa", "madrugada"]);
  });
});

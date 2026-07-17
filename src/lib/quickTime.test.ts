import { describe, expect, it } from "vitest";
import type { TimeEntry } from "@/types/domain";
import { clampQuickSeconds, resolveQuickEntryWindow } from "./quickTime";

/** Cria uma sessao minima para os testes; so importam projectId/startedAt/endedAt. */
function entry(startedAt: string, endedAt: string): TimeEntry {
  return {
    id: `e-${startedAt}`,
    projectId: "p1",
    startedAt,
    endedAt,
    durationSeconds: 0,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 0,
    source: "timer",
    createdAt: startedAt,
    updatedAt: startedAt,
    deletedAt: null,
  };
}

/** Local -> Date, sem depender do fuso da maquina que roda o teste. */
function local(day: string, time: string): Date {
  return new Date(`${day}T${time}`);
}

const HOUR = 3600;

describe("resolveQuickEntryWindow", () => {
  it("ancorado: o bloco termina onde a sessao ancora comeca", () => {
    const anchorStart = local("2026-07-16", "11:30:00").toISOString();
    const w = resolveQuickEntryWindow({
      durationSeconds: HOUR,
      day: "2026-07-16",
      anchorAtIso: anchorStart,
      dayEntries: [],
      now: local("2026-07-16", "20:00:00"),
    });

    expect(w.endedAt).toBe(anchorStart);
    expect(w.startedAt).toBe(local("2026-07-16", "10:30:00").toISOString());
  });

  it("hoje: o bloco termina agora", () => {
    const now = local("2026-07-16", "15:00:00");
    const w = resolveQuickEntryWindow({
      durationSeconds: 2 * HOUR,
      day: "2026-07-16",
      dayEntries: [],
      now,
    });

    expect(w.endedAt).toBe(now.toISOString());
    expect(w.startedAt).toBe(local("2026-07-16", "13:00:00").toISOString());
  });

  it("dia passado com sessoes: termina no fim da ultima sessao daquele dia", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: HOUR,
      day: "2026-07-14",
      dayEntries: [
        entry(
          local("2026-07-14", "08:00:00").toISOString(),
          local("2026-07-14", "10:00:00").toISOString(),
        ),
        entry(
          local("2026-07-14", "13:00:00").toISOString(),
          local("2026-07-14", "16:45:00").toISOString(),
        ),
        // Sessao de outro dia: deve ser ignorada.
        entry(
          local("2026-07-15", "09:00:00").toISOString(),
          local("2026-07-15", "23:00:00").toISOString(),
        ),
      ],
      now: local("2026-07-16", "15:00:00"),
    });

    expect(w.endedAt).toBe(local("2026-07-14", "16:45:00").toISOString());
    expect(w.startedAt).toBe(local("2026-07-14", "15:45:00").toISOString());
  });

  it("dia passado vazio: termina as 18:00 locais", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: 3 * HOUR,
      day: "2026-07-14",
      dayEntries: [],
      now: local("2026-07-16", "15:00:00"),
    });

    expect(w.endedAt).toBe(local("2026-07-14", "18:00:00").toISOString());
    expect(w.startedAt).toBe(local("2026-07-14", "15:00:00").toISOString());
  });

  it("atravessa a meia-noite: sessao ancora comeca as 00:30, 3h comecam no dia anterior", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: 3 * HOUR,
      day: "2026-07-16",
      anchorAtIso: local("2026-07-16", "00:30:00").toISOString(),
      dayEntries: [],
      now: local("2026-07-16", "09:00:00"),
    });

    expect(w.startedAt).toBe(local("2026-07-15", "21:30:00").toISOString());
    expect(new Date(w.startedAt).getDate()).toBe(15);
  });

  it("o inicio e sempre anterior ao fim, pela duracao exata", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: 90 * 60,
      day: "2026-07-16",
      dayEntries: [],
      now: local("2026-07-16", "15:00:00"),
    });

    const delta =
      (new Date(w.endedAt).getTime() - new Date(w.startedAt).getTime()) / 1000;
    expect(delta).toBe(90 * 60);
  });

  it("ignora sessoes sem fim ao procurar a ultima do dia", () => {
    const openEnded = { ...entry("x", "y"), endedAt: null };
    const w = resolveQuickEntryWindow({
      durationSeconds: HOUR,
      day: "2026-07-14",
      dayEntries: [
        {
          ...openEnded,
          startedAt: local("2026-07-14", "20:00:00").toISOString(),
        },
      ],
      now: local("2026-07-16", "15:00:00"),
    });

    // Sem sessao com fim, cai na regra do dia vazio.
    expect(w.endedAt).toBe(local("2026-07-14", "18:00:00").toISOString());
  });
});

describe("clampQuickSeconds", () => {
  it("tem piso em zero", () => {
    expect(clampQuickSeconds(-900)).toBe(0);
  });

  it("tem teto de 24h", () => {
    expect(clampQuickSeconds(30 * HOUR)).toBe(24 * HOUR);
  });

  it("mantem valores validos", () => {
    expect(clampQuickSeconds(2 * HOUR)).toBe(2 * HOUR);
  });
});

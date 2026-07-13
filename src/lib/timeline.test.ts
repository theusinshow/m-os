import { describe, expect, it } from "vitest";
import { findGaps, pairAppSessions, type AppInterval } from "./timeline";
import type { ActivityEvent, TimeEntry } from "@/types/domain";

function ev(
  eventType: ActivityEvent["eventType"],
  processName: string | null,
  detectedAt: string,
): ActivityEvent {
  return {
    id: `${eventType}-${detectedAt}`,
    eventType,
    processName,
    detectedAt,
    metadataJson: null,
    processed: false,
    createdAt: detectedAt,
  };
}

describe("pairAppSessions", () => {
  it("pareia aberturas e fechamentos por processo", () => {
    const intervals = pairAppSessions([
      ev("app_opened", "acad.exe", "2026-07-11T08:12:00Z"),
      ev("app_closed", "acad.exe", "2026-07-11T11:46:00Z"),
      ev("app_opened", "acad.exe", "2026-07-11T13:34:00Z"),
    ]);
    expect(intervals).toHaveLength(2);
    expect(intervals[0]).toMatchObject({
      start: "2026-07-11T08:12:00Z",
      end: "2026-07-11T11:46:00Z",
    });
    // Abertura sem fechamento fica com end null.
    expect(intervals[1]?.end).toBeNull();
  });
});

describe("findGaps", () => {
  const now = Date.parse("2026-07-11T18:00:00Z");

  const interval: AppInterval = {
    processName: "acad.exe",
    start: "2026-07-11T08:00:00Z",
    end: "2026-07-11T10:00:00Z",
  };

  function entry(start: string, end: string): TimeEntry {
    return {
      id: "e",
      projectId: "p",
      startedAt: start,
      endedAt: end,
      durationSeconds: 3600,
      idleSeconds: 0,
      description: null,
      activityType: "drawing",
      billable: true,
      hourlyRateSnapshotCents: 9000,
      source: "timer",
      createdAt: start,
      updatedAt: start,
      deletedAt: null,
    };
  }

  it("retorna a lacuna quando nao ha sessao sobreposta", () => {
    expect(findGaps([interval], [], now)).toHaveLength(1);
  });

  it("nao retorna lacuna quando ha sessao sobreposta", () => {
    const overlapping = entry(
      "2026-07-11T08:30:00Z",
      "2026-07-11T09:30:00Z",
    );
    expect(findGaps([interval], [overlapping], now)).toHaveLength(0);
  });

  it("ignora sessoes excluidas ao detectar lacunas", () => {
    const deleted = {
      ...entry("2026-07-11T08:30:00Z", "2026-07-11T09:30:00Z"),
      deletedAt: "2026-07-11T12:00:00Z",
    };
    expect(findGaps([interval], [deleted], now)).toHaveLength(1);
  });
});

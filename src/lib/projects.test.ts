import { describe, expect, it } from "vitest";
import type { Project, TimeEntry } from "@/types/domain";
import { recentProjectIds } from "./projects";

function project(id: string): Project {
  return {
    id,
    clientId: null,
    name: id,
    code: null,
    description: null,
    hourlyRateCents: 0,
    budgetMinutes: 0,
    status: "active",
    color: null,
    notes: null,
    createdAt: "2026-07-11T08:00:00Z",
    updatedAt: "2026-07-11T08:00:00Z",
    archivedAt: null,
  };
}

function entry(projectId: string): TimeEntry {
  return {
    id: `e-${projectId}-${Math.random()}`,
    projectId,
    startedAt: "2026-07-11T08:00:00Z",
    endedAt: "2026-07-11T09:00:00Z",
    durationSeconds: 3600,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 0,
    source: "timer",
    createdAt: "2026-07-11T08:00:00Z",
    updatedAt: "2026-07-11T08:00:00Z",
    deletedAt: null,
  };
}

describe("recentProjectIds", () => {
  it("mantem a ordem de aparicao, sem repetir", () => {
    const entries = [entry("p2"), entry("p1"), entry("p2"), entry("p3")];
    const projects = [project("p1"), project("p2"), project("p3")];

    expect(recentProjectIds(entries, projects)).toEqual(["p2", "p1", "p3"]);
  });

  it("ignora sessoes de projetos que nao existem mais", () => {
    const entries = [entry("deleted"), entry("p1")];
    const projects = [project("p1")];

    expect(recentProjectIds(entries, projects)).toEqual(["p1"]);
  });

  it("respeita o limite", () => {
    const entries = [entry("p1"), entry("p2"), entry("p3"), entry("p4")];
    const projects = [
      project("p1"),
      project("p2"),
      project("p3"),
      project("p4"),
    ];

    expect(recentProjectIds(entries, projects, 2)).toEqual(["p1", "p2"]);
  });

  it("vazio quando nao ha sessoes", () => {
    expect(recentProjectIds([], [project("p1")])).toEqual([]);
  });
});

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { TimerCard } from "./TimerCard";
import type { ActiveTimer, Project } from "@/types/domain";

const project: Project = {
  id: "p1",
  clientId: null,
  name: "Residencial Aurora",
  code: "083-22",
  description: null,
  hourlyRateCents: 9000,
  budgetMinutes: 0,
  status: "active",
  color: null,
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
  archivedAt: null,
};

const timer: ActiveTimer = {
  id: "t1",
  projectId: "p1",
  startedAt: "2026-07-11T08:00:00Z",
  lastResumedAt: "2026-07-11T08:00:00Z",
  accumulatedSeconds: 0,
  status: "running",
  description: "Detalhamento",
  activityType: "detailing",
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
};

describe("TimerCard", () => {
  it("exibe o nome do projeto e o cronometro quando ha timer ativo", () => {
    render(<TimerCard timer={timer} project={project} />);
    expect(screen.getByText(project.name)).toBeInTheDocument();
    expect(screen.getByText(/^\d{2}:\d{2}:\d{2}$/)).toBeInTheDocument();
  });

  it("mostra estado vazio quando nao ha timer", () => {
    render(<TimerCard timer={null} project={null} />);
    expect(screen.getByText("Nenhum projeto selecionado")).toBeInTheDocument();
    expect(screen.getByText("00:00:00")).toBeInTheDocument();
  });
});

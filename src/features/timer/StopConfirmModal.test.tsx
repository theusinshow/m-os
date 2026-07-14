import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StopConfirmModal } from "./StopConfirmModal";
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
  notes: null,
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
  archivedAt: null,
};

const running: ActiveTimer = {
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
};

const paused: ActiveTimer = { ...running, status: "paused" };

function setup(timer: ActiveTimer) {
  const handlers = {
    onCancel: vi.fn(),
    onPause: vi.fn(),
    onStop: vi.fn(),
  };
  render(
    <StopConfirmModal
      open
      timer={timer}
      project={project}
      busy={false}
      {...handlers}
    />,
  );
  return handlers;
}

describe("StopConfirmModal", () => {
  it("mostra o projeto e o tempo que sera gravado", () => {
    setup(running);
    expect(screen.getByText(/Residencial Aurora/)).toBeInTheDocument();
    expect(screen.getByText(/^\d{2}:\d{2}:\d{2}$/)).toBeInTheDocument();
  });

  it("oferece pausar quando o cronometro esta rodando", async () => {
    const h = setup(running);
    await userEvent.click(
      screen.getByRole("button", { name: /Pausar em vez disso/i }),
    );
    expect(h.onPause).toHaveBeenCalledOnce();
    expect(h.onStop).not.toHaveBeenCalled();
  });

  it("nao oferece pausar quando o cronometro ja esta pausado", () => {
    setup(paused);
    expect(
      screen.queryByRole("button", { name: /Pausar em vez disso/i }),
    ).not.toBeInTheDocument();
  });

  it("encerra somente quando o usuario confirma", async () => {
    const h = setup(running);
    await userEvent.click(
      screen.getByRole("button", { name: /Encerrar mesmo assim/i }),
    );
    expect(h.onStop).toHaveBeenCalledOnce();
  });

  it("cancelar nao encerra nem pausa", async () => {
    const h = setup(running);
    await userEvent.click(screen.getByRole("button", { name: /^Cancelar$/i }));
    expect(h.onCancel).toHaveBeenCalledOnce();
    expect(h.onStop).not.toHaveBeenCalled();
    expect(h.onPause).not.toHaveBeenCalled();
  });
});

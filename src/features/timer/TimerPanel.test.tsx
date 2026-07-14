import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import type { ActiveTimer, Project } from "@/types/domain";

vi.mock("@/services/timer", () => ({
  getActiveTimer: vi.fn(),
  startTimer: vi.fn(),
  pauseTimer: vi.fn(),
  resumeTimer: vi.fn(),
  stopTimer: vi.fn(),
  discardTimer: vi.fn(),
  discountIdle: vi.fn(),
}));
vi.mock("@/services/timeEntries", () => ({
  listTimeEntries: vi.fn().mockResolvedValue([]),
}));

import * as timerService from "@/services/timer";
import { useTimerStore } from "@/stores/timerStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { TimerPanel } from "./TimerPanel";

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
  description: null,
  activityType: "drawing",
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
};

function renderPanel() {
  render(
    <MemoryRouter>
      <TimerPanel />
    </MemoryRouter>,
  );
}

describe("TimerPanel — encerrar com confirmacao", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCatalogStore.setState({ projects: [project] });
    useTimerStore.setState({
      activeTimer: timer,
      loaded: true,
      error: null,
      recoveryPending: false,
    });
  });

  it("clicar em Encerrar nao encerra: apenas abre a confirmacao", async () => {
    renderPanel();
    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));

    expect(timerService.stopTimer).not.toHaveBeenCalled();
    expect(
      screen.getByRole("dialog", { name: /Encerrar sessao/i }),
    ).toBeInTheDocument();
  });

  it("encerra apenas apos confirmar", async () => {
    vi.mocked(timerService.stopTimer).mockResolvedValue({ id: "e1" } as never);
    renderPanel();

    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));
    await userEvent.click(
      screen.getByRole("button", { name: /Encerrar mesmo assim/i }),
    );

    expect(timerService.stopTimer).toHaveBeenCalledOnce();
  });

  it("Pausar em vez disso pausa e nunca encerra", async () => {
    vi.mocked(timerService.pauseTimer).mockResolvedValue({
      ...timer,
      status: "paused",
    });
    renderPanel();

    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));
    await userEvent.click(
      screen.getByRole("button", { name: /Pausar em vez disso/i }),
    );

    expect(timerService.pauseTimer).toHaveBeenCalledOnce();
    expect(timerService.stopTimer).not.toHaveBeenCalled();
  });

  it("cancelar fecha a confirmacao sem tocar no cronometro", async () => {
    renderPanel();

    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^Cancelar$/i }));

    expect(timerService.stopTimer).not.toHaveBeenCalled();
    expect(timerService.pauseTimer).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});

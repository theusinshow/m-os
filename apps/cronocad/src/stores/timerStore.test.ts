import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ActiveTimer } from "@/types/domain";

// Mocka o servico do cronometro para testar a logica do store isoladamente.
vi.mock("@/services/timer", () => ({
  getActiveTimer: vi.fn(),
  startTimer: vi.fn(),
  pauseTimer: vi.fn(),
  resumeTimer: vi.fn(),
  stopTimer: vi.fn(),
  discardTimer: vi.fn(),
}));
vi.mock("@/services/timeEntries", () => ({
  listTimeEntries: vi.fn().mockResolvedValue([]),
}));

import * as timerService from "@/services/timer";
import { useTimerStore } from "./timerStore";

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

describe("timerStore", () => {
  beforeEach(() => {
    useTimerStore.setState({
      activeTimer: null,
      loaded: false,
      error: null,
      recoveryPending: false,
    });
    vi.clearAllMocks();
  });

  it("marca recuperacao quando encontra cronometro no startup", async () => {
    vi.mocked(timerService.getActiveTimer).mockResolvedValue(timer);
    await useTimerStore.getState().loadActive(true);
    expect(useTimerStore.getState().activeTimer).toEqual(timer);
    expect(useTimerStore.getState().recoveryPending).toBe(true);
  });

  it("nao marca recuperacao em recargas normais", async () => {
    vi.mocked(timerService.getActiveTimer).mockResolvedValue(timer);
    await useTimerStore.getState().loadActive(false);
    expect(useTimerStore.getState().recoveryPending).toBe(false);
  });

  it("encerrar limpa o cronometro e a recuperacao", async () => {
    useTimerStore.setState({ activeTimer: timer, recoveryPending: true });
    vi.mocked(timerService.stopTimer).mockResolvedValue({
      id: "e1",
    } as never);
    await useTimerStore.getState().stop();
    expect(useTimerStore.getState().activeTimer).toBeNull();
    expect(useTimerStore.getState().recoveryPending).toBe(false);
  });
});

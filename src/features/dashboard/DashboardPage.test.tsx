import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { DashboardPage } from "./DashboardPage";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useTimerStore } from "@/stores/timerStore";
import type { Project, TimeEntry } from "@/types/domain";

// O foco aqui e o painel de sessoes recentes; os vizinhos falam com o backend.
vi.mock("@/features/timer/RecoveryModal", () => ({
  RecoveryModal: () => null,
}));
vi.mock("./TodosPanel", () => ({ TodosPanel: () => null }));
vi.mock("@/features/timer/TimerPanel", () => ({ TimerPanel: () => null }));

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
  createdAt: "2026-08-01T08:00:00Z",
  updatedAt: "2026-08-01T08:00:00Z",
  archivedAt: null,
};

/** Cronometro esquecido: 24h atravessando a madrugada. */
function forgotten(): TimeEntry {
  const startedAt = new Date(2026, 7, 10, 22, 33).toISOString();
  const endedAt = new Date(2026, 7, 11, 22, 46).toISOString();
  return {
    id: "e1",
    projectId: "p1",
    startedAt,
    endedAt,
    durationSeconds: 87160,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 9000,
    source: "timer",
    createdAt: endedAt,
    updatedAt: endedAt,
    deletedAt: null,
  };
}

function renderPage() {
  render(
    <MemoryRouter>
      <DashboardPage />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  useTimerStore.setState({ activeTimer: null });
  useCatalogStore.setState({ projects: [project], loaded: true });
  useEntriesStore.setState({ entries: [forgotten()] });
});

describe("DashboardPage — sessoes recentes", () => {
  it("marca a sessao suspeita com o selo Conferir?", () => {
    renderPage();

    expect(screen.getByText("Conferir?")).toBeInTheDocument();
  });

  it("leva ao historico completo", () => {
    renderPage();

    expect(screen.getByRole("link", { name: /historico/i })).toHaveAttribute(
      "href",
      "/historico",
    );
  });

  it("abre o formulario de edicao pela linha da sessao", async () => {
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Editar" }));

    expect(screen.getByRole("dialog")).toHaveAccessibleName("Editar sessao");
  });
});

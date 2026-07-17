import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Project } from "@/types/domain";

vi.mock("@/services/timeEntries", () => ({
  listTimeEntries: vi.fn().mockResolvedValue([]),
  createTimeEntry: vi.fn(),
  updateTimeEntry: vi.fn(),
  deleteTimeEntry: vi.fn(),
  restoreTimeEntry: vi.fn(),
}));

import * as entriesService from "@/services/timeEntries";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { QuickTimeModal } from "./QuickTimeModal";

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

function renderModal() {
  const onClose = vi.fn();
  render(<QuickTimeModal open onClose={onClose} defaultProjectId="p1" />);
  return { onClose };
}

const click = (name: RegExp) =>
  userEvent.click(screen.getByRole("button", { name }));

describe("QuickTimeModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCatalogStore.setState({ projects: [project] });
    useEntriesStore.setState({ entries: [], loaded: true, error: null });
  });

  it("os incrementos acumulam num total", async () => {
    renderModal();
    await click(/^\+30min$/);
    await click(/^\+1h$/);

    expect(screen.getByTestId("quick-total")).toHaveTextContent("1h 30min");
  });

  it("-15min nao deixa o total ficar negativo", async () => {
    renderModal();
    await click(/^-15min$/);

    expect(screen.getByTestId("quick-total")).toHaveTextContent("0s");
  });

  it("Limpar zera o total", async () => {
    renderModal();
    await click(/^\+2h$/);
    await click(/^Limpar$/);

    expect(screen.getByTestId("quick-total")).toHaveTextContent("0s");
  });

  it("nao deixa salvar com total zerado", () => {
    renderModal();
    expect(screen.getByRole("button", { name: /^Salvar$/ })).toBeDisabled();
  });

  it("salva um registro manual com a duracao pedida", async () => {
    vi.mocked(entriesService.createTimeEntry).mockResolvedValue({
      id: "e1",
    } as never);
    const { onClose } = renderModal();

    await click(/^\+2h$/);
    await click(/^\+30min$/);
    await click(/^Salvar$/);

    expect(entriesService.createTimeEntry).toHaveBeenCalledOnce();
    const input = vi.mocked(entriesService.createTimeEntry).mock.calls[0]![0];

    expect(input.projectId).toBe("p1");
    expect(input.source).toBe("manual");
    expect(input.idleSeconds).toBe(0);
    const delta =
      (new Date(input.endedAt).getTime() - new Date(input.startedAt).getTime()) /
      1000;
    expect(delta).toBe(2.5 * 3600);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("erro ao salvar mantem o modal aberto, com o total preservado", async () => {
    vi.mocked(entriesService.createTimeEntry).mockRejectedValue(
      new Error("banco indisponivel"),
    );
    const { onClose } = renderModal();

    await click(/^\+1h$/);
    await click(/^Salvar$/);

    expect(screen.getByText(/Falha ao adicionar o tempo/i)).toBeInTheDocument();
    expect(screen.getByTestId("quick-total")).toHaveTextContent("1h 00min");
    expect(onClose).not.toHaveBeenCalled();
  });
});

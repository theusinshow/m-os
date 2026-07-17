import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Project, TimeEntry } from "@/types/domain";

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
import { QuickTimeModal, type QuickTimeModalProps } from "./QuickTimeModal";

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

const otherProject: Project = {
  ...project,
  id: "p2",
  name: "Escritorio Ipe",
  code: "091-24",
};

const anchorEntry: TimeEntry = {
  id: "e-anchor",
  projectId: "p1",
  startedAt: "2026-07-16T13:00:00.000Z",
  endedAt: "2026-07-16T15:00:00.000Z",
  durationSeconds: 7200,
  idleSeconds: 0,
  description: null,
  activityType: "drawing",
  billable: true,
  hourlyRateSnapshotCents: 9000,
  source: "timer",
  createdAt: "2026-07-16T13:00:00.000Z",
  updatedAt: "2026-07-16T15:00:00.000Z",
  deletedAt: null,
};

function renderModal(overrides: Partial<QuickTimeModalProps> = {}) {
  const onClose = vi.fn();
  render(
    <QuickTimeModal
      open
      onClose={onClose}
      defaultProjectId="p1"
      {...overrides}
    />,
  );
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
      (new Date(input.endedAt).getTime() -
        new Date(input.startedAt).getTime()) /
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

  it("sem defaultProjectId nem ancora, pre-seleciona o projeto usado mais recentemente", async () => {
    useCatalogStore.setState({ projects: [project, otherProject] });
    useEntriesStore.setState({
      entries: [{ ...anchorEntry, id: "e-recent", projectId: "p2" }],
      loaded: true,
      error: null,
    });
    vi.mocked(entriesService.createTimeEntry).mockResolvedValue({
      id: "e1",
    } as never);
    renderModal({ defaultProjectId: undefined });

    // Caminho do botao do cabecalho: sem escolher projeto manualmente.
    await click(/^\+2h$/);
    await click(/^Salvar$/);

    expect(entriesService.createTimeEntry).toHaveBeenCalledOnce();
    const input = vi.mocked(entriesService.createTimeEntry).mock.calls[0]![0];
    expect(input.projectId).toBe("p2");
    expect(screen.queryByText(/Selecione um projeto/i)).not.toBeInTheDocument();
  });

  it("dia vazio mostra erro claro e nao tenta salvar", async () => {
    const { onClose } = renderModal();
    await click(/^\+1h$/);
    await userEvent.clear(screen.getByLabelText(/^Dia$/i));
    await click(/^Salvar$/);

    expect(screen.getByText("Escolha o dia.")).toBeInTheDocument();
    expect(entriesService.createTimeEntry).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  describe("com sessao ancora", () => {
    it("o registro novo termina onde a sessao ancora comeca", async () => {
      vi.mocked(entriesService.createTimeEntry).mockResolvedValue({
        id: "e2",
      } as never);
      renderModal({ anchor: anchorEntry });

      await click(/^\+30min$/);
      await click(/^Salvar$/);

      expect(entriesService.createTimeEntry).toHaveBeenCalledOnce();
      const input = vi.mocked(entriesService.createTimeEntry).mock.calls[0]![0];
      expect(input.endedAt).toBe(anchorEntry.startedAt);
      const delta =
        (new Date(input.endedAt).getTime() -
          new Date(input.startedAt).getTime()) /
        1000;
      expect(delta).toBe(30 * 60);
    });

    it("trava o campo de projeto e nao mostra o seletor", () => {
      renderModal({ anchor: anchorEntry });

      const projectField = screen.getByLabelText(/^Projeto$/i);
      expect(projectField).toBeDisabled();
      expect(projectField).toHaveValue(project.name);
      expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
    });

    it("nao mostra o campo Dia", () => {
      renderModal({ anchor: anchorEntry });

      expect(screen.queryByLabelText(/^Dia$/i)).not.toBeInTheDocument();
    });

    it("nunca chama updateTimeEntry (a sessao original nao e alterada)", async () => {
      vi.mocked(entriesService.createTimeEntry).mockResolvedValue({
        id: "e2",
      } as never);
      renderModal({ anchor: anchorEntry });

      await click(/^\+15min$/);
      await click(/^Salvar$/);

      expect(entriesService.updateTimeEntry).not.toHaveBeenCalled();
    });
  });
});

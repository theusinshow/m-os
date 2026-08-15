import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { EntryForm } from "./EntryForm";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import type { Project, TimeEntry } from "@/types/domain";

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

const entry: TimeEntry = {
  id: "e1",
  projectId: "p1",
  startedAt: "2026-08-11T01:33:44Z",
  endedAt: "2026-08-11T03:33:44Z",
  durationSeconds: 7200,
  idleSeconds: 0,
  description: null,
  activityType: "drawing",
  billable: true,
  hourlyRateSnapshotCents: 9000,
  source: "timer",
  createdAt: "2026-08-11T03:33:44Z",
  updatedAt: "2026-08-11T03:33:44Z",
  deletedAt: null,
};

const remove = vi.fn().mockResolvedValue(undefined);

beforeEach(() => {
  remove.mockClear();
  useCatalogStore.setState({ projects: [project] });
  useEntriesStore.setState({ remove });
});

describe("EntryForm", () => {
  it("nao oferece excluir ao criar uma sessao nova", () => {
    render(<EntryForm open entry={null} onClose={vi.fn()} />);

    expect(
      screen.queryByRole("button", { name: "Excluir sessao" }),
    ).not.toBeInTheDocument();
  });

  it("oferece excluir ao editar uma sessao existente", () => {
    render(<EntryForm open entry={entry} onClose={vi.fn()} />);

    expect(
      screen.getByRole("button", { name: "Excluir sessao" }),
    ).toBeInTheDocument();
  });

  it("so exclui depois da confirmacao, e fecha o formulario", async () => {
    const onClose = vi.fn();
    render(<EntryForm open entry={entry} onClose={onClose} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Excluir sessao" }),
    );
    expect(remove).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Excluir" }));

    expect(remove).toHaveBeenCalledWith("e1");
    expect(onClose).toHaveBeenCalled();
  });

  it("nao exclui se a confirmacao for cancelada", async () => {
    render(<EntryForm open entry={entry} onClose={vi.fn()} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Excluir sessao" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(remove).not.toHaveBeenCalled();
  });
});

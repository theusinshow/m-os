import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DeleteEntryModal } from "./DeleteEntryModal";
import type { TimeEntry } from "@/types/domain";

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

function setup(overrides: Partial<TimeEntry> = {}) {
  const handlers = { onCancel: vi.fn(), onConfirm: vi.fn() };
  render(
    <DeleteEntryModal
      open
      entry={{ ...entry, ...overrides }}
      projectName="Residencial Aurora"
      {...handlers}
    />,
  );
  return handlers;
}

describe("DeleteEntryModal", () => {
  it("mostra o projeto, a duracao e o valor que sai da conta", () => {
    setup();

    expect(screen.getByText("Residencial Aurora")).toBeInTheDocument();
    expect(screen.getByText("2h 00min")).toBeInTheDocument();
    expect(screen.getByText("R$ 180,00")).toBeInTheDocument();
  });

  it("desconta a inatividade do valor exibido", () => {
    setup({ idleSeconds: 3600 });

    expect(screen.getByText("R$ 90,00")).toBeInTheDocument();
  });

  it("avisa que da para restaurar depois", () => {
    setup();

    expect(screen.getByText(/restaurar/i)).toBeInTheDocument();
  });

  it("confirma a exclusao ao clicar em Excluir", async () => {
    const { onConfirm, onCancel } = setup();

    await userEvent.click(screen.getByRole("button", { name: "Excluir" }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("nao exclui nada ao cancelar", async () => {
    const { onConfirm, onCancel } = setup();

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("nao renderiza nada sem sessao", () => {
    const { container } = render(
      <DeleteEntryModal
        open
        entry={null}
        projectName="Residencial Aurora"
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });
});

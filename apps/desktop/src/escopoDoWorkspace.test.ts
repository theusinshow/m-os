import { describe, expect, it } from "vitest";
import { filtrarPorEscopo } from "./escopoDoWorkspace";
import type { EscopoDoWorkspace } from "./types";

const itens = [
  { id: "a", workspaceId: "prosul" },
  { id: "b", workspaceId: null },
  { id: "c", workspaceId: "dev" },
];
const escopo: EscopoDoWorkspace = { tasks: ["b", "a"], captures: [], projects: [], resources: [] };

describe("filtrarPorEscopo", () => {
  it("Todos devolve a lista inteira", () => {
    expect(filtrarPorEscopo(itens, null, "tasks")).toEqual(itens);
  });

  it("devolve so o que o core disse que aparece, na ordem original", () => {
    expect(filtrarPorEscopo(itens, escopo, "tasks").map((item) => item.id)).toEqual(["a", "b"]);
  });

  it("cada tipo olha o proprio conjunto", () => {
    expect(filtrarPorEscopo(itens, escopo, "captures")).toEqual([]);
  });

  it("sem Workspace mostra so o que ainda nao foi classificado", () => {
    expect(filtrarPorEscopo(itens, null, "tasks", true).map((item) => item.id)).toEqual(["b"]);
    expect(filtrarPorEscopo(itens, escopo, "tasks", true).map((item) => item.id)).toEqual(["b"]);
  });
});

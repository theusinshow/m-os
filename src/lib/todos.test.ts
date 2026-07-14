import { describe, expect, it } from "vitest";
import { openTodosByProject } from "./todos";
import type { Project, ProjectTodo } from "@/types/domain";

function project(id: string, name: string): Project {
  return {
    id,
    clientId: null,
    name,
    code: null,
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
}

function todo(id: string, projectId: string, done = false): ProjectTodo {
  return {
    id,
    projectId,
    text: `Pendencia ${id}`,
    done,
    doneAt: done ? "2026-07-12T10:00:00Z" : null,
    createdAt: "2026-07-11T08:00:00Z",
    updatedAt: "2026-07-11T08:00:00Z",
  };
}

const aurora = project("p1", "Aurora");
const belaVista = project("p2", "Bela Vista");
const projects = [aurora, belaVista];

describe("openTodosByProject", () => {
  it("esconde as pendencias concluidas", () => {
    const groups = openTodosByProject(
      [todo("t1", "p1"), todo("t2", "p1", true)],
      projects,
      null,
    );
    expect(groups).toHaveLength(1);
    expect(groups[0].todos.map((t) => t.id)).toEqual(["t1"]);
  });

  it("agrupa por projeto e ordena por nome quando nao ha cronometro", () => {
    const groups = openTodosByProject(
      [todo("t1", "p2"), todo("t2", "p1")],
      projects,
      null,
    );
    expect(groups.map((g) => g.project.name)).toEqual(["Aurora", "Bela Vista"]);
  });

  it("coloca o projeto do cronometro ativo em primeiro", () => {
    const groups = openTodosByProject(
      [todo("t1", "p1"), todo("t2", "p2")],
      projects,
      "p2",
    );
    expect(groups.map((g) => g.project.name)).toEqual(["Bela Vista", "Aurora"]);
  });

  it("ignora pendencias de projetos fora da lista (ex.: arquivados)", () => {
    const groups = openTodosByProject([todo("t1", "p9")], projects, null);
    expect(groups).toEqual([]);
  });

  it("nao cria grupo para projeto sem pendencias abertas", () => {
    const groups = openTodosByProject([todo("t1", "p1", true)], projects, null);
    expect(groups).toEqual([]);
  });

  it("lista vazia resulta em nenhum grupo", () => {
    expect(openTodosByProject([], projects, null)).toEqual([]);
  });
});

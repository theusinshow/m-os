import { describe, expect, it } from "vitest";
import { commandRow, isAcademicResult } from "./commandRows";
import type { SearchItem } from "./types";

const extras = { riskLabel: (risk: string) => risk, host: () => "", dayLabel: (day: string) => day };

describe("commandRow", () => {
  /** O crash de 30/09: a busca devolvia disciplina/avaliação/atividade e a
   *  paleta lia `item.capture.content` de cada uma. */
  it("desenha os resultados da faculdade sem tocar em capture", () => {
    const subject = { kind: "subject", subject: { name: "Cálculo I", code: "MAT101", teacher: "" } } as unknown as SearchItem;
    const exam = { kind: "exam", exam: { name: "P1" }, subject: "Cálculo I" } as unknown as SearchItem;
    const assignment = { kind: "assignment", assignment: { title: "Lista 3" }, subject: "Cálculo I" } as unknown as SearchItem;

    expect(commandRow(subject, extras)).toEqual({ type: "DISCIPLINA", title: "Cálculo I", context: "MAT101" });
    expect(commandRow(exam, extras)).toEqual({ type: "AVALIAÇÃO", title: "P1", context: "Cálculo I" });
    expect(commandRow(assignment, extras)).toEqual({ type: "ATIVIDADE", title: "Lista 3", context: "Cálculo I" });
    expect(isAcademicResult(exam)).toBe(true);
  });

  it("tipo desconhecido vira rótulo neutro, não exceção", () => {
    const novo = { kind: "person", person: {} } as unknown as SearchItem;
    expect(() => commandRow(novo, extras)).not.toThrow();
    expect(commandRow(novo, extras).type).toBe("PERSON");
  });

  it("capture continua como antes", () => {
    const capture = { kind: "capture", capture: { content: "ideia" }, derivedTask: null, project: null } as unknown as SearchItem;
    expect(commandRow(capture, extras)).toEqual({ type: "CAPTURE", title: "ideia", context: "ideia" });
  });
});

/**
 * Como cada resultado da busca global aparece na paleta: o rótulo do tipo, o
 * título e a linha de contexto.
 *
 * Mora fora do `App.tsx` por causa de um crash. A busca do Rust devolve mais
 * tipos do que a paleta conhecia — disciplina, avaliação e atividade da
 * faculdade —, e a cadeia de ternários caía no ramo da Capture e lia
 * `item.capture.content` de algo que não era Capture. O React inteiro
 * desmontava e a janela ficava preta. Aqui todo tipo tem ramo próprio, e um
 * tipo que ainda não existir cai num rótulo neutro em vez de derrubar a tela.
 */
import type { Resource, SearchItem } from "./types";

type CommandRowInput =
  | SearchItem
  | { kind: "function"; function: { id: string; name: string; risk: string } }
  | { kind: "meeting"; meeting: { title: string; startedAt: string }; snippet: string };

export type CommandRow = { type: string; title: string; context: string };

export function commandRow(
  item: CommandRowInput,
  extras: {
    riskLabel: (risk: string) => string;
    host: (url: Resource["url"]) => string;
    dayLabel: (day: string) => string;
  },
): CommandRow {
  switch (item.kind) {
    case "meeting":
      return {
        type: "REUNIÃO",
        title: item.meeting.title,
        context:
          item.snippet ||
          new Date(item.meeting.startedAt).toLocaleDateString("pt-BR", { day: "2-digit", month: "short" }),
      };
    case "function":
      return {
        type: "FUNCTION",
        title: item.function.name,
        context: `${item.function.id} · risco ${extras.riskLabel(item.function.risk)}`,
      };
    case "project":
      return { type: "PROJECT", title: item.project.name, context: item.project.description };
    case "workspace":
      return { type: "WORKSPACE", title: item.workspace.name, context: item.workspace.description };
    case "task":
      return { type: "TASK", title: item.task.title, context: item.project?.name ?? "" };
    case "app":
      return {
        type: "APP",
        title: item.app.name,
        context: item.app.description || item.app.launchTarget || "",
      };
    case "resource":
      return {
        type: "RESOURCE",
        title: item.resource.title,
        context: `${extras.host(item.resource.url)}${item.resource.note ? ` · ${item.resource.note}` : ""}`,
      };
    case "daily_objective":
      return { type: "OBJETIVO", title: item.objective.title, context: extras.dayLabel(item.day) };
    case "subject":
      return { type: "DISCIPLINA", title: item.subject.name, context: item.subject.code || item.subject.teacher || "" };
    case "exam":
      return { type: "AVALIAÇÃO", title: item.exam.name, context: item.subject };
    case "assignment":
      return { type: "ATIVIDADE", title: item.assignment.title, context: item.subject };
    case "capture":
      return {
        type: item.derivedTask ? "TASK + CAPTURE" : "CAPTURE",
        title: item.derivedTask?.title ?? item.capture.content,
        context: item.project?.name ?? item.capture.content,
      };
    default: {
      // Tipo novo vindo do Rust antes de a paleta aprender a desenhá-lo.
      const unknown = item as { kind?: unknown };
      return { type: String(unknown.kind ?? "ITEM").toUpperCase(), title: "", context: "" };
    }
  }
}

/** Resultado que leva para a página da faculdade, e não para um drawer. */
export function isAcademicResult(item: { kind: string }) {
  return item.kind === "subject" || item.kind === "exam" || item.kind === "assignment";
}

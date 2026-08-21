/**
 * A tradução entre o que a busca do M/OS devolve e o que o contexto do Hermes
 * aceita.
 *
 * Vive fora dos componentes porque o composer, a menção e o botão `+` precisam
 * concordar sobre qual é o rótulo e qual é o id de um item. Três cópias dessa
 * decisão foi como o mesmo Project entrou duas vezes na lista de contexto.
 */
import type { ContextInput } from "../hermes";
import type { SearchItem } from "../types";

/** As quatro que o contexto do Hermes sabe carregar hoje. */
export const ENTIDADES_ANEXAVEIS = ["project", "task", "capture", "resource"] as const;

export const TAG_DA_ENTIDADE: Record<ContextInput["entity"], string> = {
  project: "PROJ",
  task: "TASK",
  capture: "CAP",
  resource: "RES",
  workspace: "WS",
  meeting: "REUN",
  reminder: "LEMB",
  screen: "TELA",
  search: "BUSCA",
};

export function rotuloDoItem(item: SearchItem): string {
  if (item.kind === "project") return item.project.name;
  if (item.kind === "resource") return item.resource.title;
  if (item.kind === "task") return item.task.title;
  if (item.kind === "capture") return item.capture.content.slice(0, 60);
  return "";
}

export function idDoItem(item: SearchItem): string {
  if (item.kind === "project") return item.project.id;
  if (item.kind === "resource") return item.resource.id;
  if (item.kind === "task") return item.task.id;
  if (item.kind === "capture") return item.capture.id;
  return "";
}

/** O item de busca como contexto explícito. Sempre `explicit`: passou pela mão
 *  de alguém. O automático é recalculado no envio e nunca nasce aqui. */
export function contextoDoItem(item: SearchItem): ContextInput {
  return {
    origin: "explicit",
    entity: item.kind as ContextInput["entity"],
    id: idDoItem(item),
    label: rotuloDoItem(item),
  };
}

/** Só o que o contexto sabe carregar, e nada repetido do que já está anexado. */
export function anexaveis(itens: SearchItem[], jaAnexados: ContextInput[]): SearchItem[] {
  const tidos = new Set(jaAnexados.map((contexto) => contexto.id));
  return itens
    .filter((item) => (ENTIDADES_ANEXAVEIS as readonly string[]).includes(item.kind))
    .filter((item) => !tidos.has(idDoItem(item)));
}

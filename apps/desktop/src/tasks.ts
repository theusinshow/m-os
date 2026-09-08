/**
 * O que a interface precisa saber de uma Task — e nada que o domínio já saiba.
 *
 * # A fronteira
 *
 * Tudo aqui é *apresentação*: como desenhar o progresso, como escrever um
 * prazo, o que mandar quando a pessoa mexe num campo só. A regra de o que É um
 * item de checklist, de qual prioridade existe e de como dois aparelhos
 * reconciliam vive em `mos-core`, e continua lá.
 *
 * A função mais importante é `edicaoDe`. A escrita de Task é **autoritativa
 * campo por campo** — `dueAt: null` significa *tire o prazo*, e não *não mexi
 * nisso*. Isso é o que torna possível desfazer um prazo; e é também o que faria
 * cada tela apagar em silêncio os campos que ela não desenha, se cada uma
 * montasse o payload à mão. `edicaoDe` é o único jeito de mexer num campo sem
 * mexer nos outros nove.
 */

import type { Task, TaskPriority, TaskState, UpdateTaskInput } from "./types";

/** A ordem das colunas do quadro, que é a ordem em que o trabalho anda. */
export const ORDEM_DOS_ESTADOS: TaskState[] = ["inbox", "backlog", "planned", "doing", "review", "done"];

export const ROTULO_DE_PRIORIDADE: Record<TaskPriority, string> = {
  low: "Baixa",
  normal: "Normal",
  high: "Alta",
  urgent: "Urgente",
};

/**
 * A Task como ela está, pronta para receber uma mudança.
 *
 * ```ts
 * api.updateTask({ ...edicaoDe(task), dueAt: null })   // tira o prazo
 * api.updateTask({ ...edicaoDe(task), priority: "high" })
 * ```
 */
export function edicaoDe(task: Task): UpdateTaskInput {
  return {
    id: task.id,
    title: task.title,
    description: task.description,
    projectId: task.projectId,
    dueAt: task.dueAt,
    priority: task.priority,
    estimateMinutes: task.estimateMinutes,
    parentTaskId: task.parentTaskId,
    blockedByTaskId: task.blockedByTaskId,
    waitingFor: task.waitingFor,
    followUpAt: task.followUpAt,
  };
}

/**
 * O progresso do checklist, de 0 a 1. `null` quando não há checklist.
 *
 * `null` e não `0`: uma barra vazia diz "começou e não andou", e uma Task sem
 * passos não começou nada. A tela desenha a barra só quando este valor existe.
 */
export function progressoDe(task: Task): number | null {
  return task.checklistTotal > 0 ? task.checklistDone / task.checklistTotal : null;
}

/**
 * Todos os passos feitos, e a Task ainda aberta.
 *
 * A tela usa isto para OFERECER a conclusão, nunca para executá-la. Uma Task
 * que se fecha sozinha é o sistema afirmando algo que a pessoa não disse — a
 * mesma inclinação que a ADR-035 gravou ao fazer o desfazer arquivar em vez de
 * apagar.
 */
export function checklistCompleto(task: Task): boolean {
  return task.checklistTotal > 0 && task.checklistDone === task.checklistTotal && task.state !== "done";
}

/** Quantas linhas com conteúdo há num texto colado. */
export function linhasColadas(texto: string): number {
  /* Só CONTA — quem divide é o domínio (`parse_checklist_lines`), e o backend é
     quem recebe o texto inteiro. Duplicar a regra de o que é um item aqui daria
     duas respostas para a mesma colagem: o desktop tiraria um hífen que o
     celular manteria. */
  return texto.split("\n").filter((linha) => linha.trim().length > 0).length;
}

/** O estado de um prazo, para a tela decidir a cor sem recalcular a regra. */
export type SituacaoDoPrazo = "sem_prazo" | "atrasado" | "hoje" | "em_breve" | "distante";

/**
 * Onde o prazo desta Task está em relação a agora.
 *
 * Uma Task concluída NUNCA está atrasada, e essa é a regra que mais importa
 * aqui: cobrar prazo de trabalho entregue é o comportamento que faz as pessoas
 * pararem de olhar para os avisos do sistema.
 */
export function situacaoDoPrazo(task: Task, agora = new Date()): SituacaoDoPrazo {
  if (!task.dueAt) return "sem_prazo";
  if (task.state === "done" || task.completedAt) return "distante";
  const prazo = new Date(task.dueAt);
  if (Number.isNaN(prazo.getTime())) return "sem_prazo";
  if (prazo.getTime() < agora.getTime()) return "atrasado";
  const fimDoDia = new Date(agora);
  fimDoDia.setHours(23, 59, 59, 999);
  if (prazo.getTime() <= fimDoDia.getTime()) return "hoje";
  const tresDias = new Date(fimDoDia);
  tresDias.setDate(tresDias.getDate() + 3);
  return prazo.getTime() <= tresDias.getTime() ? "em_breve" : "distante";
}

/**
 * O prazo escrito curto, para o cartão: "Hoje 17:00", "Ontem", "12 set".
 *
 * Curto porque ele divide a linha do rodapé com o nome do Project, e um
 * "quinta-feira, 12 de setembro, 17:00" ali empurraria o Project para fora.
 */
export function prazoCurto(iso: string, agora = new Date()): string {
  const prazo = new Date(iso);
  if (Number.isNaN(prazo.getTime())) return "";
  const dia = (data: Date) => new Date(data.getFullYear(), data.getMonth(), data.getDate()).getTime();
  const distancia = Math.round((dia(prazo) - dia(agora)) / 86_400_000);
  const hora = prazo.getHours() || prazo.getMinutes() ? ` ${String(prazo.getHours()).padStart(2, "0")}:${String(prazo.getMinutes()).padStart(2, "0")}` : "";
  if (distancia === 0) return `Hoje${hora}`;
  if (distancia === 1) return `Amanhã${hora}`;
  if (distancia === -1) return `Ontem${hora}`;
  const mes = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"][prazo.getMonth()];
  return `${prazo.getDate()} ${mes}${hora}`;
}

/** A estimativa em minutos, escrita como se fala. */
export function estimativaCurta(minutos: number | null): string {
  if (!minutos || minutos <= 0) return "";
  if (minutos < 60) return `${minutos} min`;
  const horas = Math.floor(minutos / 60);
  const resto = minutos % 60;
  return resto ? `${horas}h${String(resto).padStart(2, "0")}` : `${horas}h`;
}

/** As estimativas que a interface oferece com um toque. O resto se digita. */
export const ESTIMATIVAS_RAPIDAS = [15, 30, 60, 120] as const;

/**
 * Um `datetime-local` a partir de um instante ISO, e o contrário.
 *
 * O `<input type="datetime-local">` fala em hora LOCAL sem fuso, e o M/OS grava
 * UTC. As duas conversões moram aqui juntas de propósito: separadas, uma delas
 * ganharia um fuso a mais na volta e o prazo andaria três horas por edição.
 */
export function paraCampoLocal(iso: string | null): string {
  if (!iso) return "";
  const data = new Date(iso);
  if (Number.isNaN(data.getTime())) return "";
  const p = (valor: number) => String(valor).padStart(2, "0");
  return `${data.getFullYear()}-${p(data.getMonth() + 1)}-${p(data.getDate())}T${p(data.getHours())}:${p(data.getMinutes())}`;
}

export function doCampoLocal(valor: string): string | null {
  if (!valor.trim()) return null;
  const data = new Date(valor);
  return Number.isNaN(data.getTime()) ? null : data.toISOString();
}

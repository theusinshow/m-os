/**
 * As decisões do drop que não precisam de tela para existir.
 *
 * O componente cuida de eventos e pintura; aqui ficam as perguntas que dá para
 * responder com uma função pura — e que, por isso, dá para testar sem abrir a
 * janela do Tauri (ver `vitest.config.ts`).
 */

import type { DropContext, IngestionStatus } from "./types";

/**
 * O tamanho do pedaço que atravessa a ponte de cada vez.
 *
 * Quatro mega é grande o bastante para que um PDF de 40 MB caiba em dez
 * chamadas, e pequeno o bastante para que nenhuma alocação única faça a janela
 * engasgar. O arquivo inteiro nunca fica na memória do renderer: cada fatia é
 * lida, enviada e descartada.
 */
export const CHUNK_BYTES = 4 * 1024 * 1024;

/**
 * Os tipos que o M/OS usa para arrastar as próprias coisas.
 *
 * Arrastar um widget na Home ou um card no Kanban não pode acordar a Drop Zone.
 * A distinção é feita pelo TIPO do payload, e não por coordenada ou por
 * elemento: quem arrasta de dentro sempre carimba um destes, e nada que venha
 * de fora consegue carimbar.
 */
export const TIPOS_INTERNOS = ["text/mos-widget", "text/task-id"] as const;

export type ConteudoDoDrop = "arquivos" | "url" | "texto" | "nenhum";

export function arrastoInterno(tipos: readonly string[]): boolean {
  return TIPOS_INTERNOS.some((tipo) => tipos.includes(tipo));
}

/**
 * O que está sendo arrastado, na ordem em que importa.
 *
 * Arquivo vence texto porque o Explorer do Windows anuncia os dois ao arrastar
 * um arquivo: o nome vai junto como texto, e tratar aquilo como texto guardaria
 * o nome e jogaria fora o arquivo.
 */
export function conteudoDoDrop(tipos: readonly string[]): ConteudoDoDrop {
  if (arrastoInterno(tipos)) return "nenhum";
  if (tipos.includes("Files")) return "arquivos";
  if (tipos.includes("text/uri-list")) return "url";
  if (tipos.includes("text/plain")) return "texto";
  return "nenhum";
}

/** Um endereço http(s) sozinho na linha — e não um texto que contém um link. */
export function pareceUrl(texto: string): boolean {
  const limpo = texto.trim();
  if (/\s/.test(limpo)) return false;
  return /^https?:\/\/\S+$/i.test(limpo);
}

/**
 * A primeira entrada útil de um `text/uri-list`.
 *
 * O formato permite comentários começando com `#`, e alguns navegadores mandam
 * o título do link na segunda linha.
 */
export function primeiraUrl(uriList: string): string {
  return (
    uriList
      .split(/\r?\n/)
      .map((linha) => linha.trim())
      .find((linha) => linha.length > 0 && !linha.startsWith("#") && pareceUrl(linha)) ?? ""
  );
}

/** Os intervalos de bytes em que um arquivo será fatiado. */
export function fatias(tamanho: number, pedaco = CHUNK_BYTES): [number, number][] {
  if (tamanho <= 0) return [];
  const saida: [number, number][] = [];
  for (let inicio = 0; inicio < tamanho; inicio += pedaco) {
    saida.push([inicio, Math.min(inicio + pedaco, tamanho)]);
  }
  return saida;
}

/** Um item do lote, do jeito que o painel o enxerga. */
export type ItemDoLote = {
  chave: string;
  nome: string;
  status: IngestionStatus;
  /** Para onde foi, quando já foi. */
  destino?: string;
  /** O que falhou, quando falhou. */
  erro?: string;
  /** O id da ingestão, quando ela chegou a existir. */
  ingestionId?: string;
  /** O Project que o sistema sugeriu sem confiança para aplicar sozinho. */
  sugestao?: { projectId: string; nome: string };
};

/**
 * O painel fecha sozinho, exceto quando ainda tem algo a dizer.
 *
 * Erro e sugestão esperam a pessoa; sucesso não espera nada, porque o recibo já
 * disse o que aconteceu e oferece o desfazer.
 */
export function painelEspera(itens: readonly ItemDoLote[]): boolean {
  return itens.some((item) => item.status === "erro" || (item.sugestao !== undefined && item.status !== "desfeito"));
}

export function loteTerminou(itens: readonly ItemDoLote[]): boolean {
  return itens.every((item) => item.status !== "esperando" && item.status !== "lendo" && item.status !== "entendendo");
}

/** O texto do recibo, no plural certo. */
export function recibo(itens: readonly ItemDoLote[]): string {
  const guardados = itens.filter((item) => item.status === "guardado");
  const repetidos = itens.filter((item) => item.status === "repetido");
  const falhos = itens.filter((item) => item.status === "erro");
  const partes: string[] = [];

  if (guardados.length === 1) {
    const destino = guardados[0].destino;
    partes.push(destino && destino !== "Library" ? `Guardado em ${destino}` : "Guardado no M/OS");
  } else if (guardados.length > 1) {
    partes.push(`${guardados.length} itens guardados`);
  }
  if (repetidos.length === 1) partes.push("1 já estava aqui");
  else if (repetidos.length > 1) partes.push(`${repetidos.length} já estavam aqui`);
  if (falhos.length === 1) partes.push("1 falhou");
  else if (falhos.length > 1) partes.push(`${falhos.length} falharam`);

  return partes.join(" · ");
}

/**
 * O contexto que acompanha o drop.
 *
 * Uma Task aberta manda junto o Project dela: é ele que tem relação com
 * Resource hoje. O `taskId` vai também, e não é decoração — ele é o registro de
 * que o alvo real era a Task, e é dele que a relação sai quando ela existir.
 */
export function contextoDoDrop(entrada: {
  pagina: string;
  projectId?: string | null;
  workspaceId?: string | null;
  taskId?: string | null;
  taskProjectId?: string | null;
}): DropContext {
  return {
    page: entrada.pagina,
    projectId: entrada.projectId || entrada.taskProjectId || null,
    workspaceId: entrada.workspaceId || null,
    taskId: entrada.taskId || null,
  };
}

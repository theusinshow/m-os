/**
 * As paradas do lado da tela: só o que dá para verificar.
 *
 * Mesma divisão do `daily.ts` e do `weekly.ts`, e pelo mesmo motivo: não há
 * teste de DOM neste repositório (`vitest.config.ts`), então tudo que decide
 * alguma coisa — o rótulo, o corte da lista, quando o ponto acende — mora aqui,
 * e o componente só desenha o resultado.
 *
 * **Nenhuma regra de domínio.** Qual é a tolerância de cada coluna, o que conta
 * como atividade e em que ordem a lista sai vivem em `mos-core::stale`, com
 * teste. Aqui é apresentação.
 */
import type { Parada, ProjectActivity } from "./types";

/** Quantas paradas cabem no widget antes de o resto virar contagem. */
export const PARADAS_VISIVEIS = 5;

/**
 * "12d". Curto porque mora dentro de um card de Kanban estreito, ao lado do
 * título — "parada há 12 dias" empurraria o título para duas linhas em toda
 * task marcada.
 *
 * Acima de 99 vira "99+d": o número exato de um abandono de um ano não muda
 * decisão nenhuma, e três dígitos quebram a linha do card.
 */
export function rotuloDeDias(days: number): string {
  if (!Number.isFinite(days) || days <= 0) return "";
  return days > 99 ? "99+d" : `${Math.trunc(days)}d`;
}

/**
 * As primeiras, e quantas ficaram de fora.
 *
 * O resto vira contagem em vez de sumir: uma lista cortada em silêncio faz o
 * widget dizer "cinco paradas" quando são vinte.
 */
export function paradasVisiveis(
  paradas: Parada[],
  limite = PARADAS_VISIVEIS,
): { visiveis: Parada[]; restantes: number } {
  return {
    visiveis: paradas.slice(0, limite),
    restantes: Math.max(0, paradas.length - limite),
  };
}

/** Id da Task para dias parados. É o que o card do Kanban consulta. */
export function diasPorTask(paradas: Parada[]): Map<string, number> {
  return new Map(
    paradas.filter((parada) => parada.kind === "task").map((parada) => [parada.id, parada.days]),
  );
}

/** Os ids dos Projects parados. */
export function projectsParados(paradas: Parada[]): Set<string> {
  return new Set(paradas.filter((parada) => parada.kind === "project").map((parada) => parada.id));
}

/** Id do Project para o instante da última atividade real dele. */
export function atividadePorProject(activity: ProjectActivity[]): Map<string, string> {
  return new Map(activity.map((linha) => [linha.projectId, linha.lastActivity]));
}

/**
 * O instante caiu no dia de hoje, no fuso de quem está olhando.
 *
 * O dia é local aqui de propósito, e isso não contradiz o domínio: lá a conta é
 * de DURAÇÃO ("parado há 12 dias") e não precisa de fuso; aqui a pergunta é
 * "mexi nisto hoje?", que é uma data civil, e data civil é do renderer — o mesmo
 * raciocínio do `calendar.rs`.
 */
export function mexidoHoje(iso: string | undefined, agora = new Date()): boolean {
  if (!iso) return false;
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return false;
  return quando.toDateString() === agora.toDateString();
}

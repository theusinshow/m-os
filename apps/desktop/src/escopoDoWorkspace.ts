/**
 * O Workspace como filtro do app inteiro (ADR-093).
 *
 * A REGRA nao mora aqui. Quem decide se uma Task aparece em PROSUL e
 * `mos_core::pertence`, e o core devolve o `EscopoDoWorkspace` — os ids que
 * aparecem. Este arquivo so testa pertinencia e guarda qual e o Workspace
 * atual. Reescrever a regra em TypeScript seria a segunda copia que o
 * `AGENTS.md` proibe, e a primeira a envelhecer em silencio.
 */
import type { EscopoDoWorkspace } from "./types";

/** A mesma chave que a Home ja usava. Vazio e "Todos". */
export const CHAVE_DO_WORKSPACE_ATUAL = "m-os-current-workspace";

/** O Workspace atual desta maquina, ou `null` para "Todos". */
export function workspaceAtual(): string | null {
  try {
    return localStorage.getItem(CHAVE_DO_WORKSPACE_ATUAL) || null;
  } catch {
    return null;
  }
}

export type TipoDoEscopo = keyof EscopoDoWorkspace;

/**
 * O que aparece, na ordem em que veio.
 *
 * `escopo === null` e "Todos": a lista inteira. `semWorkspace` e a triagem —
 * so o que ainda nao foi classificado, que e onde o historico mora.
 */
export function filtrarPorEscopo<T extends { id: string; workspaceId?: string | null }>(
  itens: T[],
  escopo: EscopoDoWorkspace | null,
  tipo: TipoDoEscopo,
  semWorkspace = false,
): T[] {
  const visiveis = escopo ? new Set(escopo[tipo]) : null;
  return itens.filter((item) => (!visiveis || visiveis.has(item.id)) && (!semWorkspace || !item.workspaceId));
}

/**
 * A fila de capturas que ainda não chegaram ao M/OS (F3.3, ADR-077).
 *
 * O bolso não é um banco offline: o domínio mora no servidor. O que ele
 * garante é mais estreito e mais importante — **uma captura escrita sem rede
 * não se perde**. O texto e uma chave vão para o armazenamento do aparelho
 * ANTES de a tela dizer "guardado", e são reenviados quando a conexão volta.
 * O servidor usa a chave como id da Capture, então reenviar nunca duplica.
 *
 * Três estados, e a tela diz qual: guardado neste aparelho, aguardando envio,
 * recebido pelo M/OS. O armazenamento do navegador pode faltar (modo privado,
 * cota, limpeza pelo sistema); quando falta, a tela NÃO diz "guardado" — o
 * texto fica no campo.
 *
 * Não há envio em segundo plano garantido: a fila anda quando o app abre, volta
 * a ficar visível ou o navegador avisa que a rede voltou.
 */
import { SemConexao, SemSessao } from "./api";

export type Pendente = {
  chave: string;
  texto: string;
  /** RFC3339 com o fuso do aparelho: a hora em que a pessoa capturou. */
  capturadaEm: string;
  /** A última recusa do servidor que não foi falta de rede nem de sessão. */
  recusa?: string;
};

/** O mínimo do `Storage` que a fila usa — injetável para teste. */
export type Armazem = Pick<Storage, "getItem" | "setItem">;

const CHAVE = "mos-capturas-pendentes";

export function pendentes(armazem: Armazem | null = padrao()): Pendente[] {
  if (!armazem) return [];
  try {
    const cru = armazem.getItem(CHAVE);
    const lido: unknown = cru ? JSON.parse(cru) : [];
    return Array.isArray(lido) ? (lido as Pendente[]).filter((p) => p && typeof p.chave === "string" && typeof p.texto === "string") : [];
  } catch {
    return [];
  }
}

function gravar(lista: Pendente[], armazem: Armazem) {
  armazem.setItem(CHAVE, JSON.stringify(lista));
}

/** Guarda no aparelho. LANÇA se o armazenamento recusar — e aí quem chamou
 *  não pode dizer "guardado". */
export function guardar(item: Pendente, armazem: Armazem | null = padrao()) {
  if (!armazem) throw new Error("Este navegador não deixa guardar nada no aparelho.");
  const lista = pendentes(armazem).filter((p) => p.chave !== item.chave);
  gravar([...lista, item], armazem);
}

export function remover(chave: string, armazem: Armazem | null = padrao()) {
  if (!armazem) return;
  try {
    gravar(pendentes(armazem).filter((p) => p.chave !== chave), armazem);
  } catch {
    /* sem espaço para regravar a lista: o item fica e o reenvio não duplica */
  }
}

function marcarRecusa(chave: string, recusa: string, armazem: Armazem) {
  try {
    gravar(pendentes(armazem).map((p) => (p.chave === chave ? { ...p, recusa } : p)), armazem);
  } catch {
    /* idem */
  }
}

export type Esvaziamento = { enviadas: number; restantes: number; parou: "rede" | "sessao" | null };

/**
 * Manda o que estiver na fila, na ordem em que foi capturado.
 *
 * Sem rede ou sem sessão, PARA e mantém tudo: a sessão expirada não pode fazer
 * a fila sumir, nem mandar o texto para outra conta. Recusa do servidor (texto
 * inválido, por exemplo) fica marcada no item e a fila segue com os outros.
 */
export async function esvaziar(
  enviar: (item: Pendente) => Promise<unknown>,
  armazem: Armazem | null = padrao(),
): Promise<Esvaziamento> {
  if (!armazem) return { enviadas: 0, restantes: 0, parou: null };
  let enviadas = 0;
  let parou: Esvaziamento["parou"] = null;
  for (const item of pendentes(armazem)) {
    try {
      await enviar(item);
      remover(item.chave, armazem);
      enviadas += 1;
    } catch (causa) {
      if (causa instanceof SemSessao) { parou = "sessao"; break; }
      if (causa instanceof SemConexao) { parou = "rede"; break; }
      marcarRecusa(item.chave, causa instanceof Error ? causa.message : String(causa), armazem);
    }
  }
  return { enviadas, restantes: pendentes(armazem).length, parou };
}

/** Uma chave nova. `randomUUID` exige contexto seguro; o bolso roda em HTTPS,
 *  e o fallback só existe para o navegador que não tiver a função. */
export function novaChave(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") return crypto.randomUUID();
  const hex = Array.from({ length: 32 }, () => Math.floor(Math.random() * 16).toString(16));
  hex[12] = "4";
  hex[16] = ((parseInt(hex[16], 16) & 0x3) | 0x8).toString(16);
  const s = hex.join("");
  return `${s.slice(0, 8)}-${s.slice(8, 12)}-${s.slice(12, 16)}-${s.slice(16, 20)}-${s.slice(20)}`;
}

function padrao(): Armazem | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

import { timingSafeEqual } from "node:crypto";
import { env } from "@/lib/env";

/**
 * Dois escopos, dois secrets (ADR-073):
 *
 * - `action` (`MOS_ACTION_SECRET`) escreve pela Action API — e também lê: quem
 *   pode lançar uma conta pode ver as contas.
 * - `read` (`MOS_FINANCE_READ_SECRET`) só lê. A Action API nunca o aceita.
 *
 * Secret vazio no servidor nunca autoriza nada, com ou sem header: é o que
 * segura o buraco se um proxy entregar `Bearer ` sem aparar.
 */
export type MosScope = "read" | "action";

function matches(header: string | null, secret: string) {
  if (!secret || !header) return false;
  const expected = Buffer.from(`Bearer ${secret}`);
  const received = Buffer.from(header);
  return expected.length === received.length && timingSafeEqual(expected, received);
}

export function mosScopeOf(request: Request): MosScope | null {
  const header = request.headers.get("authorization");
  if (matches(header, env.mosActionSecret)) return "action";
  if (matches(header, env.mosFinanceReadSecret)) return "read";
  return null;
}

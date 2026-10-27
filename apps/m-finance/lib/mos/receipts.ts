import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { mosActionReceipts } from "@/db/schema";

export type Claim =
  | { status: "new" }
  | { status: "done"; result: unknown }
  | { status: "in_progress" };

/**
 * Reserva a idempotency key ANTES de executar.
 *
 * O insert com `on conflict do nothing` é a trava: dois pedidos com a mesma
 * chave chegando juntos (duplo clique, retry de rede) — só um insere. O outro
 * lê o recibo pronto, ou descobre que a primeira execução ainda está em curso.
 */
export async function claimReceipt(userId: string, key: string, actionId: string): Promise<Claim> {
  if (!db) return { status: "new" };
  const inserted = await db
    .insert(mosActionReceipts)
    .values({ userId, idempotencyKey: key, actionId, status: "pending" })
    .onConflictDoNothing()
    .returning({ id: mosActionReceipts.id });
  if (inserted.length > 0) return { status: "new" };

  const [existing] = await db
    .select({ status: mosActionReceipts.status, result: mosActionReceipts.result })
    .from(mosActionReceipts)
    .where(and(eq(mosActionReceipts.userId, userId), eq(mosActionReceipts.idempotencyKey, key)))
    .limit(1);
  if (existing?.status === "completed") return { status: "done", result: existing.result };
  return { status: "in_progress" };
}

export async function completeReceipt(userId: string, key: string, result: unknown) {
  if (!db) return;
  await db
    .update(mosActionReceipts)
    .set({ status: "completed", result, updatedAt: new Date() })
    .where(and(eq(mosActionReceipts.userId, userId), eq(mosActionReceipts.idempotencyKey, key)));
}

/** Execução que estourou exceção não deixou efeito conhecido: libera para tentar de novo. */
export async function releaseReceipt(userId: string, key: string) {
  if (!db) return;
  await db
    .delete(mosActionReceipts)
    .where(
      and(
        eq(mosActionReceipts.userId, userId),
        eq(mosActionReceipts.idempotencyKey, key),
        eq(mosActionReceipts.status, "pending"),
      ),
    );
}

import { and, desc, eq, gt } from "drizzle-orm";
import { db } from "@/db/client";
import {
  whatsappMessages,
  whatsappPendingActions,
  whatsappMessageDirection,
  whatsappMessageStatus,
  whatsappPendingActionType,
  whatsappPendingActionStatus,
} from "@/db/schema";

type WhatsappMessageDirection = (typeof whatsappMessageDirection.enumValues)[number];
type WhatsappMessageStatus = (typeof whatsappMessageStatus.enumValues)[number];
type WhatsappPendingActionType = (typeof whatsappPendingActionType.enumValues)[number];
type WhatsappPendingActionStatus = (typeof whatsappPendingActionStatus.enumValues)[number];

export async function logWhatsappMessage(input: {
  userId?: string | null;
  direction: WhatsappMessageDirection;
  status: WhatsappMessageStatus;
  from?: string | null;
  to?: string | null;
  body?: string | null;
  twilioMessageSid?: string | null;
  error?: string | null;
  metadata?: unknown;
}) {
  if (!db) {
    return;
  }

  await db
    .insert(whatsappMessages)
    .values({
      userId: input.userId ?? null,
      direction: input.direction,
      status: input.status,
      from: input.from ?? null,
      to: input.to ?? null,
      body: input.body ?? null,
      twilioMessageSid: input.twilioMessageSid || null,
      error: input.error ?? null,
      metadata: input.metadata ?? null,
    })
    .onConflictDoNothing();
}

export async function getActiveWhatsappPendingAction(userId: string, phone: string) {
  if (!db) {
    return null;
  }

  const [pendingAction] = await db
    .select()
    .from(whatsappPendingActions)
    .where(
      and(
        eq(whatsappPendingActions.userId, userId),
        eq(whatsappPendingActions.phone, phone),
        eq(whatsappPendingActions.status, "pending"),
        gt(whatsappPendingActions.expiresAt, new Date()),
      ),
    )
    .orderBy(desc(whatsappPendingActions.createdAt))
    .limit(1);

  return pendingAction ?? null;
}

export async function createWhatsappPendingAction(input: {
  userId: string;
  phone: string;
  actionType: WhatsappPendingActionType;
  summary: string;
  payload: unknown;
  expiresInMinutes?: number;
}) {
  if (!db) {
    return null;
  }

  const expiresAt = new Date();
  expiresAt.setMinutes(expiresAt.getMinutes() + (input.expiresInMinutes ?? 15));

  const [pendingAction] = await db
    .insert(whatsappPendingActions)
    .values({
      userId: input.userId,
      phone: input.phone,
      actionType: input.actionType,
      summary: input.summary,
      payload: input.payload,
      expiresAt,
    })
    .returning();

  return pendingAction ?? null;
}

export async function updateWhatsappPendingActionStatus(
  id: string,
  status: Exclude<WhatsappPendingActionStatus, "pending">,
) {
  if (!db) {
    return;
  }

  const now = new Date();

  await db
    .update(whatsappPendingActions)
    .set({
      status,
      confirmedAt: status === "confirmed" ? now : null,
      cancelledAt: status === "cancelled" ? now : null,
      updatedAt: now,
    })
    .where(eq(whatsappPendingActions.id, id));
}

import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { pushSubscriptions } from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";

type SubscriptionBody = {
  endpoint?: string;
  keys?: { p256dh?: string; auth?: string };
  userAgent?: string;
};

/** Stores (or refreshes) a Web Push subscription for the current user/device. */
export async function POST(request: Request) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) {
    return Response.json({ error: "Usuário interno não configurado." }, { status: 400 });
  }

  const body = (await request.json().catch(() => null)) as SubscriptionBody | null;
  if (!body?.endpoint || !body.keys?.p256dh || !body.keys?.auth) {
    return Response.json({ error: "Inscrição inválida." }, { status: 400 });
  }

  await db
    .insert(pushSubscriptions)
    .values({
      userId: appUser.id,
      endpoint: body.endpoint,
      p256dh: body.keys.p256dh,
      auth: body.keys.auth,
      userAgent: body.userAgent ?? null,
    })
    .onConflictDoUpdate({
      target: pushSubscriptions.endpoint,
      set: {
        userId: appUser.id,
        p256dh: body.keys.p256dh,
        auth: body.keys.auth,
        updatedAt: new Date(),
      },
    });

  return Response.json({ ok: true });
}

/** Removes a subscription by endpoint (user disabled notifications). */
export async function DELETE(request: Request) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) {
    return Response.json({ error: "Usuário interno não configurado." }, { status: 400 });
  }

  const body = (await request.json().catch(() => null)) as { endpoint?: string } | null;
  if (!body?.endpoint) {
    return Response.json({ error: "Endpoint ausente." }, { status: 400 });
  }

  await db
    .delete(pushSubscriptions)
    .where(
      and(
        eq(pushSubscriptions.userId, appUser.id),
        eq(pushSubscriptions.endpoint, body.endpoint),
      ),
    );

  return Response.json({ ok: true });
}

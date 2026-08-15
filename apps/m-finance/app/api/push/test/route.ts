import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { sendPushToUser } from "@/lib/push/web-push";

/** Sends a test push to all of the current user's devices. */
export async function POST() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) {
    return Response.json({ error: "Usuário interno não configurado." }, { status: 400 });
  }

  const delivered = await sendPushToUser(appUser.id, {
    title: "M Finance",
    body: "Notificações ativadas! É assim que você será avisado antes de uma cobrança.",
    url: "/app/subscriptions",
    tag: "test",
  });

  return Response.json({ ok: true, delivered });
}

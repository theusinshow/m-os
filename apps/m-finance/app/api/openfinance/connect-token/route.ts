import { requireUser } from "@/lib/auth/guard";
import { env, isPluggyConfigured } from "@/lib/env";
import { getAppUserBySupabaseId } from "@/lib/months";
import { createConnectToken } from "@/lib/openfinance/pluggy";

/**
 * Mints a Pluggy Connect token for the authenticated user. The frontend widget
 * (pluggy-connect) consumes it to render the bank-consent flow. Pass an
 * `itemId` to reopen an existing connection for re-consent/update.
 */
export async function POST(request: Request) {
  const user = await requireUser();

  if (!isPluggyConfigured()) {
    return Response.json({ error: "Pluggy não configurado." }, { status: 503 });
  }

  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) {
    return Response.json({ error: "Usuário interno não configurado." }, { status: 400 });
  }

  let itemId: string | undefined;
  try {
    const body = (await request.json()) as { itemId?: string } | null;
    itemId = body?.itemId;
  } catch {
    // No body is fine — fresh connection.
  }

  // Register the webhook so synced items notify us automatically. Pluggy only
  // accepts https URLs, so we skip it on http/localhost — there the manual
  // "Sincronizar agora" button covers syncing.
  const origin = new URL(request.url).origin;
  const webhookUrl =
    env.pluggyWebhookSecret && origin.startsWith("https://")
      ? `${origin}/api/openfinance/webhook?secret=${env.pluggyWebhookSecret}`
      : undefined;

  try {
    const accessToken = await createConnectToken({
      itemId,
      clientUserId: appUser.id,
      webhookUrl,
    });
    return Response.json({ accessToken });
  } catch (error) {
    const message =
      (error as { message?: string })?.message ?? "Erro ao gerar token.";
    return Response.json({ error: message }, { status: 502 });
  }
}

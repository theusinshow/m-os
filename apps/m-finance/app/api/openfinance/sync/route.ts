import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { syncPluggyItem } from "@/lib/openfinance/sync";

/** Manually triggers a sync for one connected item (a "refresh now" button). */
export async function POST(request: Request) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) {
    return Response.json({ error: "Usuário interno não configurado." }, { status: 400 });
  }

  let itemId: string | undefined;
  try {
    const body = (await request.json()) as { itemId?: string } | null;
    itemId = body?.itemId;
  } catch {
    // handled below
  }

  if (!itemId) {
    return Response.json({ error: "itemId é obrigatório." }, { status: 400 });
  }

  try {
    const results = await syncPluggyItem(appUser.id, itemId);
    return Response.json({ results });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Erro ao sincronizar.";
    return Response.json({ error: message }, { status: 502 });
  }
}

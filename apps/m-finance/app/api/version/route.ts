// A versão que está no ar agora. A aba aberta compara com a dela e, se ficou
// para trás, recarrega (ADR-091). Fora do `/app`, então sem login: um commit
// não é segredo, e a aba precisa perguntar mesmo com a sessão expirada.
export const dynamic = "force-dynamic";

export function GET() {
  return Response.json(
    { version: process.env.VERCEL_GIT_COMMIT_SHA ?? "dev" },
    { headers: { "Cache-Control": "no-store" } },
  );
}

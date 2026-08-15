export const runtime = "nodejs";

import { NextRequest } from "next/server";
import { promises as fs } from "node:fs";
import { catalogPath, caseDraftPath } from "@/lib/storage/paths";
import { generateCaseDraft } from "@/lib/capture/generate-case";
import type { Catalog } from "@/lib/types";

export async function POST(req: NextRequest): Promise<Response> {
  let slug: string;
  try {
    const body = (await req.json()) as { slug: unknown };
    if (typeof body.slug !== "string" || !body.slug.trim()) {
      return Response.json({ error: "slug inválido" }, { status: 400 });
    }
    slug = body.slug.trim();
  } catch {
    return Response.json({ error: "corpo da requisição inválido" }, { status: 400 });
  }

  try {
    const raw = await fs.readFile(catalogPath(slug), "utf-8");
    const catalog = JSON.parse(raw) as Catalog;

    const mdx = generateCaseDraft(catalog);
    await fs.writeFile(caseDraftPath(slug), mdx, "utf-8");

    console.log(`[atlas:${slug}] case-draft.mdx gerado`);

    return Response.json({ path: `/generated/${slug}/case-draft.mdx` });
  } catch (err) {
    const isNotFound =
      err instanceof Error && "code" in err && (err as NodeJS.ErrnoException).code === "ENOENT";

    console.error(`[atlas:${slug}]`, err instanceof Error ? err.message : err);

    if (isNotFound) {
      return Response.json(
        { error: "Catálogo não encontrado. Gere o catálogo primeiro." },
        { status: 404 }
      );
    }

    return Response.json(
      { error: "Não foi possível gerar o rascunho de case." },
      { status: 500 }
    );
  }
}

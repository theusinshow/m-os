export const runtime = "nodejs";

import { NextRequest } from "next/server";
import { promises as fs } from "node:fs";
import { catalogPath } from "@/lib/storage/paths";
import { deleteProject } from "@/lib/storage/delete-project";
import { AtlasError } from "@/lib/errors";

interface Params {
  params: Promise<{ slug: string }>;
}

/** Remove um projeto gerado (pasta inteira em public/generated/[slug]). */
export async function DELETE(_req: NextRequest, { params }: Params): Promise<Response> {
  const { slug } = await params;

  try {
    await fs.access(catalogPath(slug));
  } catch {
    return Response.json({ error: "Projeto não encontrado." }, { status: 404 });
  }

  try {
    await deleteProject(slug);
  } catch (err) {
    const message = err instanceof AtlasError ? err.userMessage : "Falha ao remover o projeto.";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ ok: true });
}

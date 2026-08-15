export const runtime = "nodejs";

import { listProjects } from "@/lib/storage/list-projects";

/** Lista resumida dos projetos gerados — alimenta a paleta de comando. */
export async function GET(): Promise<Response> {
  const projects = await listProjects();
  return Response.json(
    { projects },
    { headers: { "Cache-Control": "no-store" } }
  );
}

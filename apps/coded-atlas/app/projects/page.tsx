import Link from "next/link";
import type { Metadata } from "next";
import { listProjects } from "@/lib/storage/list-projects";
import { ProjectsLibrary } from "@/components/projects-library";

export const metadata: Metadata = {
  title: "Projetos — Coded Atlas",
};

export default async function ProjectsPage() {
  const projects = await listProjects();

  return (
    <main className="max-w-6xl mx-auto px-6 py-12">
      {/* Header */}
      <header className="flex items-end justify-between gap-6 mb-10">
        <div>
          <h1 className="text-2xl font-semibold text-zinc-50 tracking-tight">
            Biblioteca de projetos
          </h1>
          <p className="text-zinc-400 text-sm mt-1.5">
            {projects.length === 0
              ? "Nenhum projeto catalogado ainda."
              : `${projects.length} projeto${projects.length !== 1 ? "s" : ""} catalogado${projects.length !== 1 ? "s" : ""}.`}
          </p>
        </div>
        <Link
          href="/generate"
          className="shrink-0 inline-flex items-center gap-2 px-5 py-2.5 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
        >
          Gerar novo →
        </Link>
      </header>

      {projects.length === 0 ? (
        <div className="border border-line bg-surface/30 p-12 text-center space-y-4">
          <p className="text-sm text-zinc-300">
            Sua biblioteca está vazia. Gere o primeiro catálogo a partir de uma URL.
          </p>
          <Link
            href="/generate"
            className="inline-flex items-center gap-2 px-5 py-2.5 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
          >
            Gerar primeiro catálogo →
          </Link>
        </div>
      ) : (
        <ProjectsLibrary projects={projects} />
      )}
    </main>
  );
}

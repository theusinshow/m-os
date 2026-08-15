import Link from "next/link";
import { listProjects } from "@/lib/storage/list-projects";
import { ProjectCatalogCard } from "@/components/project-catalog-card";

export default async function HomePage() {
  const projects = await listProjects();
  const recent = projects.slice(0, 6);

  return (
    <main className="min-h-[calc(100vh-3.5rem)]">
      {/* ── Hero ── */}
      <section className="relative border-b border-line overflow-hidden">
        <div className="bg-grid absolute inset-0 pointer-events-none" aria-hidden />
        <div className="relative max-w-6xl mx-auto px-6 py-20 md:py-28">
          <p className="text-[11px] font-mono text-accent uppercase tracking-[0.2em] mb-5 flex items-center gap-2">
            <span className="tri" aria-hidden />
            Coded by M · Laboratório interno
          </p>
          <h1 className="text-4xl md:text-5xl font-semibold tracking-tight text-zinc-50 leading-[1.05] max-w-3xl">
            Transforme uma URL em um catálogo visual de projeto.
          </h1>
          <p className="text-zinc-300 text-base md:text-lg leading-relaxed mt-6 max-w-2xl">
            Capture screenshots desktop e mobile, full page, seções e vídeo de scroll,
            e organize tudo num pacote pronto para virar case no portfólio.
          </p>

          <div className="flex flex-wrap items-center gap-3 mt-9">
            <Link
              href="/generate"
              className="inline-flex items-center gap-2 px-6 py-3 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
            >
              Gerar catálogo
              <span aria-hidden>→</span>
            </Link>
            <Link
              href="/projects"
              className="inline-flex items-center gap-2 px-6 py-3 border border-line text-zinc-200 text-sm font-medium hover:border-line-soft hover:bg-surface transition-colors"
            >
              Ver projetos
              {projects.length > 0 && (
                <span className="font-mono text-zinc-500">{projects.length}</span>
              )}
            </Link>
          </div>
        </div>
      </section>

      {/* ── Projetos recentes / histórico ── */}
      <section className="max-w-6xl mx-auto px-6 py-14">
        <div className="flex items-end justify-between gap-4 mb-7">
          <div>
            <h2 className="text-lg font-semibold text-zinc-100">Projetos recentes</h2>
            <p className="text-[13px] text-zinc-400 mt-0.5">
              {projects.length === 0
                ? "Nenhum catálogo gerado ainda."
                : `${projects.length} projeto${projects.length !== 1 ? "s" : ""} no catálogo · ${recent.length} mais recente${recent.length !== 1 ? "s" : ""}.`}
            </p>
          </div>
          {projects.length > recent.length && (
            <Link
              href="/projects"
              className="shrink-0 text-[13px] font-medium text-accent hover:text-accent-bright transition-colors"
            >
              Ver todos →
            </Link>
          )}
        </div>

        {projects.length === 0 ? (
          <EmptyState />
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {recent.map((project) => (
              <ProjectCatalogCard key={project.slug} project={project} />
            ))}
          </div>
        )}
      </section>
    </main>
  );
}

function EmptyState() {
  const steps = [
    { n: "01", t: "URL + dados", d: "Cole a URL e dê um nome. A categoria já vem em lista." },
    { n: "02", t: "Captura", d: "Desktop e mobile, full page, seções e vídeo de scroll." },
    { n: "03", t: "Catálogo", d: "Screenshots, thumbnails, capa e catalog.json organizados." },
  ];
  return (
    <div className="border border-line bg-surface/30 p-8 md:p-10">
      <div className="grid md:grid-cols-3 gap-6 mb-8">
        {steps.map(({ n, t, d }) => (
          <div key={n} className="space-y-1.5">
            <span className="text-[11px] font-mono text-accent">{n}</span>
            <p className="text-sm font-semibold text-zinc-100">{t}</p>
            <p className="text-[13px] text-zinc-400 leading-relaxed">{d}</p>
          </div>
        ))}
      </div>
      <Link
        href="/generate"
        className="inline-flex items-center gap-2 px-5 py-2.5 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
      >
        Gerar o primeiro catálogo →
      </Link>
    </div>
  );
}

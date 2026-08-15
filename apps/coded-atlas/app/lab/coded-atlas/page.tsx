import { promises as fs } from "node:fs";
import Link from "next/link";
import type { Metadata } from "next";
import { listProjects } from "@/lib/storage/list-projects";
import { catalogPath } from "@/lib/storage/paths";
import { config } from "@/lib/config";
import type { Catalog } from "@/lib/types";

export const metadata: Metadata = {
  title: "Coded Atlas — Laboratório · Coded by M",
  description:
    "Experimento do Laboratório da Coded by M: uma ferramenta própria que transforma a URL de um projeto em um catálogo visual premium.",
};

const STEPS = [
  { n: "01", title: "URL", desc: "Informe a URL do projeto e os metadados básicos: nome, slug, categoria." },
  { n: "02", title: "Captura", desc: "Playwright abre o site, dispensa overlays e fotografa viewport, full page e seções em desktop (1440px) e mobile (390px) — com gravação de scroll." },
  { n: "03", title: "Catálogo", desc: "Sharp gera thumbnails e capa; o catalog.json organiza tudo, pronto para virar case no portfólio." },
];

const NEXT = [
  "Publicação assistida: enviar catálogo + case-draft direto para o repositório do portfólio.",
  "Página pública de cada case em /cases/[slug], alimentada pelo portfolio.json.",
  "Fragmentos na Paisagem Digital da Home a partir do acento e da thumbnail.",
];

export default async function LabAtlasPage() {
  const projects = await listProjects();
  const featured = projects[0];

  let featuredCatalog: Catalog | undefined;
  if (featured) {
    try {
      const raw = await fs.readFile(catalogPath(featured.slug), "utf-8");
      featuredCatalog = JSON.parse(raw) as Catalog;
    } catch {
      featuredCatalog = undefined;
    }
  }

  const featuredVideo =
    featuredCatalog?.videos?.desktop ?? featuredCatalog?.videos?.mobile;

  return (
    <main className="min-h-screen px-6 py-16 flex flex-col">
      {/* Grid técnico de fundo */}
      <div className="bg-grid fixed inset-0 pointer-events-none" aria-hidden />

      <div className="relative w-full max-w-3xl mx-auto flex flex-col flex-1 pt-8">
        {/* ── Hero do experimento ── */}
        <section className="space-y-8">
          <div>
            <p className="text-[11px] font-mono text-accent uppercase tracking-[0.2em] mb-3 flex items-center gap-2">
              <span aria-hidden style={triangle} />
              Experimento · v{config.atlasVersion}
            </p>
            <h1 className="text-6xl font-mono font-bold tracking-tight text-zinc-100 leading-none">
              CODED<br />ATLAS
            </h1>
          </div>

          <div className="border-t border-line pt-7 space-y-3">
            <p className="text-xl text-zinc-300 font-light leading-snug">
              Uma ferramenta própria que transforma uma URL em um catálogo visual de projeto.
            </p>
            <p className="text-sm text-zinc-500 leading-relaxed max-w-lg">
              Construída pela Coded by M para documentar e apresentar melhor os projetos que
              entregamos — prova de método, detalhismo e capacidade técnica.
            </p>
          </div>

          <div className="flex items-center gap-4 pt-2">
            <Link
              href="/generate"
              className="inline-block px-7 py-3 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors cursor-pointer"
            >
              Gerar Catálogo →
            </Link>
            <Link href="/projects" className="text-sm text-zinc-500 hover:text-zinc-300 transition-colors">
              Ver projetos
            </Link>
          </div>
        </section>

        {/* ── Problema ── */}
        <section className="mt-24">
          <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-6">
            O problema
          </p>
          <p className="text-sm text-zinc-400 leading-relaxed max-w-prose">
            Criar cases é manual e repetitivo: abrir o site, tirar prints, ajustar tamanhos,
            capturar mobile e desktop, gravar tela, organizar arquivos, montar thumbnails. Isso
            consome tempo, gera inconsistência visual e atrasa a publicação dos projetos. O Atlas
            transforma essa tarefa operacional em um pacote visual consistente, gerado a partir de
            uma única URL.
          </p>
        </section>

        {/* ── Como funciona ── */}
        <section className="mt-20">
          <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-8">
            Como funciona
          </p>
          <div>
            {STEPS.map(({ n, title, desc }) => (
              <div key={n} className="flex gap-6 py-5 border-b border-line last:border-b-0">
                <span className="text-[11px] font-mono text-accent w-5 shrink-0 pt-px">{n}</span>
                <div className="space-y-1">
                  <p className="text-sm text-zinc-300 font-medium">
                    <span className="text-zinc-500 mr-2" aria-hidden>▸</span>
                    {title}
                  </p>
                  <p className="text-xs text-zinc-500 leading-relaxed">{desc}</p>
                </div>
              </div>
            ))}
          </div>
        </section>

        {/* ── Capturas geradas (projeto real, quando existir) ── */}
        {featured && (
          <section className="mt-20">
            <div className="flex items-baseline justify-between mb-6">
              <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
                Capturas geradas
              </p>
              <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                {projects.length} no catálogo
              </span>
            </div>
            <Link
              href={`/projects/${featured.slug}`}
              className="group block border border-line hover:border-accent transition-colors"
            >
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={featuredCatalog?.cover?.image ?? featured.thumbnail}
                alt={`${featured.name} — capturado pelo Atlas`}
                className="w-full block"
                loading="lazy"
              />
              <div className="flex items-center justify-between px-4 py-3 border-t border-line">
                <span className="text-sm text-zinc-300 group-hover:text-white transition-colors">
                  {featured.name}
                </span>
                <span className="text-[11px] font-mono text-zinc-400">{featured.category}</span>
              </div>
            </Link>
          </section>
        )}

        {/* ── Vídeo de navegação ── */}
        {featuredVideo && (
          <section className="mt-16">
            <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-6">
              Vídeo de navegação
            </p>
            <video
              controls
              preload="metadata"
              className="w-full block border border-line"
              style={{ maxHeight: "26rem", background: "#000" }}
            >
              <source src={featuredVideo} type="video/webm" />
            </video>
          </section>
        )}

        {/* ── Estrutura técnica ── */}
        <section className="mt-20">
          <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-6">
            Estrutura técnica
          </p>
          <div className="flex flex-wrap gap-2 mb-6">
            {["Next.js", "TypeScript", "Tailwind CSS", "Playwright", "Sharp", "SSE"].map((t) => (
              <span
                key={t}
                className="text-[11px] font-mono text-zinc-200 bg-surface border border-line px-2 py-1"
              >
                {t}
              </span>
            ))}
          </div>
          <div className="border border-line bg-surface/60 px-5 py-4">
            <pre className="text-[11px] font-mono text-zinc-500 leading-[1.7]">{`public/generated/[slug]/
├─ catalog.json
├─ screenshots/   desktop + mobile · viewport, full page e seções
├─ thumbnails/    thumb-main · thumb-mobile · cover
├─ videos/        gravações de scroll (webm)
└─ case-draft.mdx`}</pre>
          </div>
        </section>

        {/* ── Próximos passos ── */}
        <section className="mt-20">
          <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-6">
            Próximos passos
          </p>
          <ul className="space-y-3">
            {NEXT.map((item) => (
              <li key={item} className="flex gap-3 text-sm text-zinc-400 leading-relaxed">
                <span className="text-accent shrink-0" aria-hidden>▸</span>
                <span>{item}</span>
              </li>
            ))}
          </ul>
        </section>

        {/* ── CTA final ── */}
        <section className="mt-20 border border-line bg-surface/40 p-8 flex flex-col sm:flex-row sm:items-center justify-between gap-6">
          <div>
            <p className="text-zinc-100 text-base font-medium">Veja o Atlas em ação.</p>
            <p className="text-zinc-500 text-sm mt-1">Gere um catálogo a partir de qualquer URL.</p>
          </div>
          <Link
            href="/generate"
            className="shrink-0 inline-block px-7 py-3 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors cursor-pointer"
          >
            Gerar Catálogo →
          </Link>
        </section>

        {/* ── Footer ── */}
        <footer className="mt-auto pt-16">
          <div className="border-t border-line pt-6 flex items-center justify-between">
            <span className="text-[11px] font-mono text-zinc-400 uppercase tracking-widest">
              Coded Atlas · Coded by M
            </span>
            <span className="text-[11px] font-mono text-zinc-400">v{config.atlasVersion}</span>
          </div>
        </footer>
      </div>
    </main>
  );
}

const triangle: React.CSSProperties = {
  display: "inline-block",
  width: 0,
  height: 0,
  borderTop: "4px solid transparent",
  borderBottom: "4px solid transparent",
  borderLeft: "6px solid rgb(113 113 122)",
};

import { promises as fs } from "node:fs";
import { notFound } from "next/navigation";
import Link from "next/link";
import type { Metadata } from "next";
import { catalogPath, caseDraftPath } from "@/lib/storage/paths";
import type { Catalog } from "@/lib/types";
import { buildPortfolioManifest } from "@/lib/capture/build-portfolio-manifest";
import { GeneratedGallery } from "@/components/generated-gallery";
import { AssetDownloadItems } from "@/components/asset-downloads";
import { CaseDraftSection } from "@/components/case-draft-section";
import { PortfolioExport } from "@/components/portfolio-export";
import { DeleteProject } from "@/components/delete-project";
import { VisualDiff } from "@/components/visual-diff";
import { ZoomImage } from "@/components/zoom-image";

interface Props {
  params: Promise<{ slug: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  try {
    const raw = await fs.readFile(catalogPath(slug), "utf-8");
    const catalog = JSON.parse(raw) as Catalog;
    return { title: `${catalog.project.name} — Coded Atlas` };
  } catch {
    return { title: "Projeto — Coded Atlas" };
  }
}

export default async function ProjectPage({ params }: Props) {
  const { slug } = await params;

  const raw = await fs
    .readFile(catalogPath(slug), "utf-8")
    .catch(() => notFound());

  const catalog = JSON.parse(raw) as Catalog;
  const { project, captures, thumbnails, meta, createdAt } = catalog;
  const portfolioManifest = buildPortfolioManifest(catalog);

  const caseExists = await fs
    .access(caseDraftPath(slug))
    .then(() => true)
    .catch(() => false);
  const casePublicPath = caseExists ? `/generated/${slug}/case-draft.mdx` : undefined;

  const durationSec = (meta.durationMs / 1000).toFixed(1);
  const createdDate = new Date(createdAt).toLocaleDateString("pt-BR", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });

  return (
    <main className="max-w-4xl mx-auto px-6 py-10">
      <div className="space-y-16">

        {/* ── Voltar ── */}
        <Link
          href="/projects"
          className="inline-flex items-center gap-2 text-[13px] text-zinc-400 hover:text-zinc-100 transition-colors"
        >
          <span aria-hidden>←</span> Biblioteca
        </Link>

        {/* ── Hero ── */}
        <section className="grid md:grid-cols-[1fr_auto] gap-8 items-start">
          {/* Info */}
          <div className="space-y-4 min-w-0">
            <div>
              <p className="text-[11px] font-mono text-accent uppercase tracking-widest mb-2">
                {project.category}
              </p>
              <h1 className="text-3xl font-semibold text-zinc-50 tracking-tight">
                {project.name}
              </h1>
              {project.client && (
                <p className="text-zinc-300 text-sm mt-1.5">{project.client}</p>
              )}
            </div>

            <a
              href={project.url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-zinc-400 text-[13px] font-mono hover:text-accent transition-colors break-all block"
            >
              {project.url} ↗
            </a>

            {project.description && (
              <p className="text-zinc-300 text-sm leading-relaxed">
                {project.description}
              </p>
            )}

            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 pt-2 text-[11px] font-mono text-zinc-400">
              <span>{createdDate}</span>
              <span className="text-zinc-600">·</span>
              <span>{durationSec}s</span>
              <span className="text-zinc-600">·</span>
              <span>v{catalog.version}</span>
            </div>

            <div className="pt-3">
              <Link
                href={`/generate?reprocess=${project.slug}`}
                className="inline-flex items-center gap-2 px-4 py-2 border border-line text-zinc-200 text-[13px] font-medium hover:border-accent hover:text-accent-bright transition-colors cursor-pointer"
              >
                <span aria-hidden>↻</span>
                Reprocessar capturas
              </Link>
            </div>
          </div>

          {/* Thumbnails */}
          <div className="shrink-0 w-full md:w-72 space-y-2">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img
              src={thumbnails.main}
              alt={`${project.name} — thumbnail desktop`}
              className="w-full block border border-line"
              loading="eager"
            />
            <div className="flex gap-2 items-start">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={thumbnails.mobile}
                alt={`${project.name} — thumbnail mobile`}
                className="w-20 block border border-line"
                loading="lazy"
              />
              <div className="text-[11px] font-mono text-zinc-400 pt-1 space-y-1">
                <p>640 × 400</p>
                <p>320 × 640</p>
              </div>
            </div>
          </div>
        </section>

        {/* ── Capa do catálogo ── */}
        {catalog.cover && (
          <div className="border-t border-line pt-16">
            <div className="flex items-baseline justify-between mb-4">
              <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
                Capa do catálogo
              </p>
              <div className="flex items-center gap-4">
                <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                  {catalog.cover.source === "og-image" ? "og:image do site" : "smart crop · attention"}
                </span>
                <a
                  href={catalog.cover.image}
                  download
                  className="text-[11px] font-mono text-accent hover:text-accent-bright transition-colors uppercase tracking-wider cursor-pointer"
                >
                  Baixar →
                </a>
              </div>
            </div>
            <ZoomImage
              src={catalog.cover.image}
              alt={`${project.name} — capa`}
              className="w-full block border border-line"
              style={{ aspectRatio: "1200/630", objectFit: "cover" }}
              loading="eager"
            />
          </div>
        )}

        {/* ── Composições para redes ── */}
        {catalog.compositions && catalog.compositions.length > 0 && (
          <div className="border-t border-line pt-16">
            <div className="flex items-baseline justify-between mb-6">
              <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
                Composições para redes
              </p>
              <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                prontas para postar
              </span>
            </div>
            <div className="grid sm:grid-cols-3 gap-4">
              {catalog.compositions.map((comp) => (
                <div key={comp.name} className="space-y-2">
                  <div className="border border-line bg-surface h-48 flex items-center justify-center p-3">
                    <ZoomImage
                      src={comp.image}
                      alt={`${project.name} — ${comp.label}`}
                      className="max-h-full max-w-full object-contain"
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] font-mono text-zinc-400">
                      {comp.label}{" "}
                      <span className="text-zinc-600">{comp.width}×{comp.height}</span>
                    </span>
                    <a
                      href={comp.image}
                      download
                      className="text-[11px] font-mono text-accent hover:text-accent-bright transition-colors uppercase tracking-wider"
                    >
                      Baixar →
                    </a>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── Mockups ── */}
        {catalog.mockups && catalog.mockups.length > 0 && (
          <div className="border-t border-line pt-16">
            <div className="flex items-baseline justify-between mb-6">
              <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
                Mockups
              </p>
              <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                fundo transparente
              </span>
            </div>
            <div className="grid sm:grid-cols-2 gap-4">
              {catalog.mockups.map((mk) => (
                <div key={mk.name} className="space-y-2">
                  <div className="border border-line bg-surface/40 h-64 flex items-center justify-center p-4">
                    <ZoomImage
                      src={mk.image}
                      alt={`${project.name} — ${mk.label}`}
                      className="max-h-full max-w-full object-contain"
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] font-mono text-zinc-400">{mk.label}</span>
                    <a
                      href={mk.image}
                      download
                      className="text-[11px] font-mono text-accent hover:text-accent-bright transition-colors uppercase tracking-wider"
                    >
                      Baixar →
                    </a>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── Desktop gallery ── */}
        <div className="border-t border-line pt-16">
          <GeneratedGallery
            type="desktop"
            capture={captures.desktop}
            projectName={project.name}
          />
        </div>

        {/* ── Mobile gallery ── */}
        <div className="border-t border-line pt-16">
          <GeneratedGallery
            type="mobile"
            capture={captures.mobile}
            projectName={project.name}
          />
        </div>

        {/* ── Outras páginas ── */}
        {catalog.pages && catalog.pages.length > 0 && (
          <div className="border-t border-line pt-16 space-y-10">
            <div className="flex items-baseline justify-between">
              <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
                Outras páginas
              </p>
              <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                {catalog.pages.length} página{catalog.pages.length !== 1 ? "s" : ""}
              </span>
            </div>
            {catalog.pages.map((pg) => (
              <div key={pg.path} className="space-y-3">
                <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                  <span className="text-sm font-medium text-zinc-100">{pg.path}</span>
                  <a
                    href={pg.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-[11px] font-mono text-zinc-500 hover:text-accent transition-colors break-all"
                  >
                    {pg.url} ↗
                  </a>
                </div>
                <div className="grid md:grid-cols-[1fr_10rem] gap-4 items-start">
                  <div className="border border-line bg-surface overflow-hidden max-h-80">
                    <ZoomImage
                      src={pg.desktop.screenshot}
                      alt={`${pg.path} — desktop`}
                      className="w-full block"
                    />
                  </div>
                  <div className="border border-line bg-surface overflow-hidden">
                    <ZoomImage
                      src={pg.mobile.screenshot}
                      alt={`${pg.path} — mobile`}
                      className="w-full block"
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* ── Estados de interação ── */}
        {catalog.states && catalog.states.length > 0 && (
          <div className="border-t border-line pt-16">
            <div className="flex items-baseline justify-between mb-6">
              <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
                Estados de interação
              </p>
              <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                desktop
              </span>
            </div>
            <div className="grid sm:grid-cols-2 gap-4">
              {catalog.states.map((st) => (
                <div key={st.screenshot} className="space-y-2">
                  <div className="border border-line bg-surface overflow-hidden max-h-80">
                    <ZoomImage
                      src={st.screenshot}
                      alt={`${project.name} — ${st.name}`}
                      className="w-full block"
                    />
                  </div>
                  <div className="flex items-baseline justify-between gap-3">
                    <span className="text-[13px] text-zinc-200">{st.name}</span>
                    <span className="text-[10px] font-mono text-zinc-500 truncate">{st.selector}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ── Section strips ── */}
        {catalog.sections && catalog.sections.length > 0 && (() => {
          const desktopSections = catalog.sections.filter(s => s.device === "desktop");
          const mobileSections  = catalog.sections.filter(s => s.device === "mobile");
          return (
            <div className="border-t border-line pt-16 space-y-12">
              <div>
                <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-8">
                  Seções capturadas
                </p>

                {desktopSections.length > 0 && (
                  <div className="mb-8">
                    <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-3">
                      Desktop · {desktopSections.length} seções
                    </p>
                    <div className="overflow-x-auto pb-3">
                      <div className="flex gap-3" style={{ width: "max-content" }}>
                        {desktopSections.map((section) => {
                          const label = section.suggestedName ?? section.heading ?? section.name;
                          return (
                            <div key={section.name} className="shrink-0 w-72 group">
                              <ZoomImage
                                src={section.screenshot}
                                alt={label}
                                className="w-full block border border-line group-hover:border-accent transition-colors"
                                style={{ aspectRatio: "16/10", objectFit: "cover", objectPosition: "top" }}
                              />
                              <div className="flex items-baseline gap-2 mt-1.5">
                                {section.semanticTag && (
                                  <span className="text-[10px] font-mono text-zinc-500 uppercase shrink-0">
                                    &lt;{section.semanticTag}&gt;
                                  </span>
                                )}
                                <p className="text-[10px] font-mono text-zinc-500 truncate">
                                  {label}
                                </p>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  </div>
                )}

                {mobileSections.length > 0 && (
                  <div>
                    <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-3">
                      Mobile · {mobileSections.length} seções
                    </p>
                    <div className="overflow-x-auto pb-3">
                      <div className="flex gap-3" style={{ width: "max-content" }}>
                        {mobileSections.map((section) => {
                          const label = section.suggestedName ?? section.heading ?? section.name;
                          return (
                            <div key={section.name} className="shrink-0 w-32 group">
                              <ZoomImage
                                src={section.screenshot}
                                alt={label}
                                className="w-full block border border-line group-hover:border-accent transition-colors"
                                style={{ aspectRatio: "9/16", objectFit: "cover", objectPosition: "top" }}
                              />
                              <div className="mt-1.5 space-y-0.5">
                                {section.semanticTag && (
                                  <span className="block text-[10px] font-mono text-zinc-500 uppercase">
                                    &lt;{section.semanticTag}&gt;
                                  </span>
                                )}
                                <p className="text-[10px] font-mono text-zinc-500 truncate">
                                  {label}
                                </p>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          );
        })()}

        {/* ── Vídeos ── */}
        {catalog.videos && (catalog.videos.desktop || catalog.videos.mobile) && (
          <div className="border-t border-line pt-16">
            <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-8">
              Gravações de scroll
            </p>
            <div className="grid md:grid-cols-2 gap-8">
              {catalog.videos.desktop && (
                <div className="space-y-2">
                  <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">Desktop</p>
                  <video
                    controls
                    preload="metadata"
                    className="w-full block border border-line"
                    style={{ maxHeight: "24rem" }}
                  >
                    <source src={catalog.videos.desktop} type="video/webm" />
                  </video>
                </div>
              )}
              {catalog.videos.mobile && (
                <div className="space-y-2">
                  <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">Mobile</p>
                  <video
                    controls
                    preload="metadata"
                    className="w-full block border border-line"
                    style={{ maxHeight: "24rem", objectFit: "contain", background: "#000" }}
                  >
                    <source src={catalog.videos.mobile} type="video/webm" />
                  </video>
                </div>
              )}
            </div>
          </div>
        )}

        {/* ── Inspeção do site ── */}
        {catalog.inspection && (
          catalog.inspection.colors.length > 0 ||
          catalog.inspection.fonts.length > 0 ||
          catalog.inspection.techStack.length > 0
        ) && (
          <div className="border-t border-line pt-16">
            <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-8">
              Inspeção do site
            </p>
            <div className="grid md:grid-cols-3 gap-8">

              {/* Paleta */}
              {catalog.inspection.colors.length > 0 && (
                <div className="space-y-3">
                  <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">
                    Paleta
                  </p>
                  <div className="flex flex-wrap gap-2">
                    {catalog.inspection.colors.map((color) => (
                      <div
                        key={color}
                        title={color}
                        className="w-8 h-8 border border-line shrink-0"
                        style={{ backgroundColor: color }}
                      />
                    ))}
                  </div>
                  <div className="flex flex-wrap gap-x-3 gap-y-1">
                    {catalog.inspection.colors.map((color) => (
                      <span key={color} className="text-[10px] font-mono text-zinc-400">
                        {color}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Tipografia */}
              {catalog.inspection.fonts.length > 0 && (
                <div className="space-y-3">
                  <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">
                    Tipografia
                  </p>
                  <div className="space-y-2">
                    {catalog.inspection.fonts.map((font) => (
                      <div key={font} className="flex items-baseline gap-2">
                        <span
                          className="text-zinc-300 text-base leading-none"
                          style={{ fontFamily: font }}
                        >
                          Aa
                        </span>
                        <span className="text-[10px] font-mono text-zinc-500">{font}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Tech Stack */}
              {catalog.inspection.techStack.length > 0 && (
                <div className="space-y-3">
                  <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">
                    Tech Stack
                  </p>
                  <div className="flex flex-wrap gap-2">
                    {catalog.inspection.techStack.map((tech) => (
                      <span
                        key={tech}
                        className="text-[11px] font-mono text-zinc-200 bg-surface border border-line px-2 py-1"
                      >
                        {tech}
                      </span>
                    ))}
                  </div>
                </div>
              )}

            </div>
          </div>
        )}

        {/* ── Downloads ── */}
        <div className="border-t border-line pt-16">
          <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-4">
            Downloads
          </p>
          <ul className="divide-y divide-line border border-line">
            {/* ZIP — item destacado */}
            <li>
              <a
                href={`/api/zip/${slug}`}
                className="flex items-center justify-between px-4 py-3.5 text-zinc-100 hover:bg-surface hover:text-white transition-colors group cursor-pointer"
              >
                <span className="text-sm font-medium">
                  Baixar tudo (.zip)
                </span>
                <span className="flex items-center gap-2 text-[10px] font-mono text-zinc-500 group-hover:text-zinc-300 transition-colors">
                  <span>ZIP</span>
                  <span>↓</span>
                </span>
              </a>
            </li>
            <AssetDownloadItems catalog={catalog} />
          </ul>
        </div>

        {/* ── Rascunho de case ── */}
        <div className="border-t border-line pt-16">
          <CaseDraftSection slug={slug} existingPath={casePublicPath} />
        </div>

        {/* ── Export para o portfólio ── */}
        <div className="border-t border-line pt-16">
          <PortfolioExport manifest={portfolioManifest} />
        </div>

        {/* ── Monitoramento (diff visual) ── */}
        <div className="border-t border-line pt-16">
          <VisualDiff slug={project.slug} beforeImage={captures.desktop.screenshot} />
        </div>

        {/* ── Zona de risco ── */}
        <div className="border-t border-line pt-16">
          <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-4">
            Zona de risco
          </p>
          <div className="border border-bad/30 bg-bad/[0.04] p-5">
            <DeleteProject slug={project.slug} name={project.name} />
          </div>
        </div>

        {/* ── Footer ── */}
        <footer className="border-t border-line pt-8 flex items-center justify-between">
          <span className="text-[11px] font-mono text-zinc-400 uppercase tracking-widest">
            Coded Atlas · Coded by M
          </span>
          <a
            href="/generate"
            className="text-[11px] font-mono text-zinc-400 hover:text-accent transition-colors uppercase tracking-wider cursor-pointer"
          >
            Gerar outro →
          </a>
        </footer>

      </div>
    </main>
  );
}

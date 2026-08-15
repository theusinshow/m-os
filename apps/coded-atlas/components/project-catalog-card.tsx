import Link from "next/link";
import type { ProjectSummary } from "@/lib/storage/list-projects";

interface Props {
  project: ProjectSummary;
  selectable?: boolean;
  selected?: boolean;
  onToggle?: (slug: string) => void;
}

function hostOf(url: string): string {
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return url;
  }
}

export function ProjectCatalogCard({ project, selectable = false, selected = false, onToggle }: Props) {
  const date = new Date(project.createdAt).toLocaleDateString("pt-BR", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });

  const inner = (
    <>
      {/* Thumbnail */}
      <div className="relative overflow-hidden bg-surface aspect-video border-b border-line">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src={project.thumbnail}
          alt={`${project.name} — thumbnail`}
          className="w-full h-full object-cover object-top transition-transform duration-500 ease-out group-hover:scale-[1.03]"
          loading="lazy"
        />
        <span className="absolute top-2 left-2 text-[10px] font-mono uppercase tracking-wider text-zinc-300 bg-base/80 backdrop-blur px-1.5 py-0.5 border border-line">
          {project.category}
        </span>
      </div>

      {/* Info */}
      <div className="p-4">
        <p className="text-[15px] font-semibold text-zinc-100 group-hover:text-white transition-colors leading-snug">
          {project.name}
        </p>
        {project.client && <p className="text-[13px] text-zinc-400 mt-0.5">{project.client}</p>}

        <div className="flex items-center justify-between mt-3 pt-3 border-t border-line/70">
          <span className="text-[11px] font-mono text-zinc-400 truncate max-w-[60%]">
            {hostOf(project.url)}
          </span>
          <span className="text-[11px] font-mono text-zinc-500 shrink-0">{date}</span>
        </div>
      </div>
    </>
  );

  // ── Modo seleção: o card vira um botão que alterna a seleção ──
  if (selectable) {
    return (
      <button
        type="button"
        onClick={() => onToggle?.(project.slug)}
        aria-pressed={selected}
        className={[
          "group relative block w-full text-left border transition-colors cursor-pointer",
          selected ? "border-accent bg-accent/[0.06]" : "border-line hover:border-line-soft",
        ].join(" ")}
      >
        {inner}
        <span
          className={[
            "absolute top-2 right-2 w-5 h-5 border flex items-center justify-center text-[11px] leading-none",
            selected ? "bg-accent border-accent text-zinc-950" : "bg-base/80 backdrop-blur border-line text-transparent",
          ].join(" ")}
          aria-hidden
        >
          ✓
        </span>
      </button>
    );
  }

  // ── Modo normal: navega para o projeto ──
  return (
    <div className="group relative border border-line hover:border-line-soft hover:bg-surface/40 transition-colors">
      <Link href={`/projects/${project.slug}`} className="block">
        {inner}
      </Link>
      <Link
        href={`/generate?reprocess=${project.slug}`}
        title="Reprocessar capturas"
        aria-label={`Reprocessar ${project.name}`}
        className="absolute top-2 right-2 flex items-center gap-1 text-[11px] font-mono text-zinc-300 bg-base/80 backdrop-blur px-2 py-1 border border-line opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:border-accent hover:text-accent-bright transition-all"
      >
        <span aria-hidden>↻</span>
      </Link>
    </div>
  );
}

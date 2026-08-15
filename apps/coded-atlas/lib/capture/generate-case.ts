import type { Catalog, CatalogSection } from "../types";

/**
 * Gera o conteúdo MDX do rascunho de case a partir de um Catalog.
 * Sem IA — derivado inteiramente dos dados capturados.
 */
export function generateCaseDraft(catalog: Catalog): string {
  const { project, captures, thumbnails, sections, inspection, cover, createdAt } = catalog;
  const date = new Date(createdAt).toISOString().split("T")[0];

  const lines: string[] = [];

  // ── Frontmatter ────────────────────────────────────────────────────────────
  lines.push("---");
  lines.push(`title: "${project.name}"`);
  if (project.client) lines.push(`client: "${project.client}"`);
  lines.push(`category: "${project.category}"`);
  lines.push(`url: "${project.url}"`);
  lines.push(`date: "${date}"`);
  lines.push(`slug: "${project.slug}"`);
  lines.push(`thumbnail: "${thumbnails.main}"`);
  lines.push(`thumbnailMobile: "${thumbnails.mobile}"`);
  if (cover) lines.push(`cover: "${cover.image}"`);
  if (inspection?.techStack.length) {
    lines.push(`techStack: [${inspection.techStack.map(t => `"${t}"`).join(", ")}]`);
  }
  lines.push("---");
  lines.push("");

  // ── Título e metadados ─────────────────────────────────────────────────────
  lines.push(`# ${project.name}`);
  lines.push("");

  const meta: string[] = [];
  if (project.client) meta.push(`**Cliente:** ${project.client}`);
  meta.push(`**Categoria:** ${project.category}`);
  meta.push(`**URL:** [${project.url}](${project.url})`);
  lines.push(meta.join("  \n"));
  lines.push("");

  if (project.description) {
    lines.push(project.description);
    lines.push("");
  }

  // ── Capa ───────────────────────────────────────────────────────────────────
  if (cover) {
    lines.push("## Capa");
    lines.push("");
    const sourceLabel = cover.source === "og-image" ? "og:image" : "smart crop";
    lines.push(`![Capa do catálogo — ${project.name}](${cover.image})`);
    lines.push(`*Fonte: ${sourceLabel}*`);
    lines.push("");
  }

  // ── Desktop ────────────────────────────────────────────────────────────────
  lines.push("## Desktop");
  lines.push("");
  lines.push(`*Viewport ${captures.desktop.viewport}*`);
  lines.push("");
  lines.push(`![${project.name} — desktop viewport](${captures.desktop.screenshot})`);
  lines.push("");
  lines.push(`![${project.name} — desktop full page](${captures.desktop.fullpage})`);
  lines.push("");

  // ── Mobile ─────────────────────────────────────────────────────────────────
  lines.push("## Mobile");
  lines.push("");
  lines.push(`*Viewport ${captures.mobile.viewport}*`);
  lines.push("");
  lines.push(`![${project.name} — mobile viewport](${captures.mobile.screenshot})`);
  lines.push("");
  lines.push(`![${project.name} — mobile full page](${captures.mobile.fullpage})`);
  lines.push("");

  // ── Seções detectadas ──────────────────────────────────────────────────────
  const desktopSections = sections.filter((s): s is CatalogSection => s.device === "desktop");
  const mobileSections  = sections.filter((s): s is CatalogSection => s.device === "mobile");

  if (desktopSections.length > 0 || mobileSections.length > 0) {
    lines.push("## Seções");
    lines.push("");

    if (desktopSections.length > 0) {
      lines.push("### Desktop");
      lines.push("");
      for (const section of desktopSections) {
        const label = section.suggestedName ?? section.heading ?? section.name;
        const tag = section.semanticTag ? `\`<${section.semanticTag}>\`` : "";
        lines.push(`#### ${label}${tag ? ` ${tag}` : ""}`);
        lines.push("");
        lines.push(`![${label}](${section.screenshot})`);
        lines.push("");
      }
    }

    if (mobileSections.length > 0) {
      lines.push("### Mobile");
      lines.push("");
      for (const section of mobileSections) {
        const label = section.suggestedName ?? section.heading ?? section.name;
        lines.push(`#### ${label}`);
        lines.push("");
        lines.push(`![${label}](${section.screenshot})`);
        lines.push("");
      }
    }
  }

  // ── Identidade visual ──────────────────────────────────────────────────────
  if (
    inspection &&
    (inspection.colors.length > 0 || inspection.fonts.length > 0 || inspection.techStack.length > 0)
  ) {
    lines.push("## Identidade Visual");
    lines.push("");

    if (inspection.colors.length > 0) {
      lines.push(`**Paleta:** ${inspection.colors.join(" · ")}`);
      lines.push("");
    }

    if (inspection.fonts.length > 0) {
      lines.push(`**Tipografia:** ${inspection.fonts.join(", ")}`);
      lines.push("");
    }

    if (inspection.techStack.length > 0) {
      lines.push(`**Stack:** ${inspection.techStack.join(", ")}`);
      lines.push("");
    }
  }

  // ── Rodapé ─────────────────────────────────────────────────────────────────
  lines.push("---");
  lines.push("");
  lines.push(`*Gerado pelo Coded Atlas · Coded by M · ${date}*`);
  lines.push("");

  return lines.join("\n");
}

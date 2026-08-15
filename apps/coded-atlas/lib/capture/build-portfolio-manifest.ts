import type { Catalog, PortfolioManifest } from "../types";

/**
 * Monta o manifesto que o portfólio Coded by M consome a partir de um Catalog.
 * Função pura, sem IO e sem dados novos — só projeta o que já foi capturado.
 */
export function buildPortfolioManifest(catalog: Catalog): PortfolioManifest {
  const { project, thumbnails, cover, inspection, videos, createdAt } = catalog;

  const palette = inspection?.colors ?? [];
  const hasVideo = Boolean(videos?.desktop || videos?.mobile);
  const date = new Date(createdAt).toISOString().split("T")[0];

  return {
    slug: project.slug,
    name: project.name,
    category: project.category,
    ...(project.description ? { description: project.description } : {}),
    url: project.url,
    thumbnail: thumbnails.main,
    thumbnailMobile: thumbnails.mobile,
    ...(cover ? { cover: cover.image } : {}),
    ...(palette[0] ? { accent: palette[0] } : {}),
    palette,
    techStack: inspection?.techStack ?? [],
    hasVideo,
    date,
    atlasVersion: catalog.version,
  };
}

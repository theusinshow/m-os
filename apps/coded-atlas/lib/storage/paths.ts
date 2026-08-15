import path from "node:path";
import { config } from "../config";

export const projectDir = (slug: string) =>
  path.join(config.outputDir, slug);

export const screenshotDir = (slug: string) =>
  path.join(projectDir(slug), "screenshots");

export const thumbnailDir = (slug: string) =>
  path.join(projectDir(slug), "thumbnails");

export const catalogPath = (slug: string) =>
  path.join(projectDir(slug), "catalog.json");

// v0.2 — seções e vídeos
export const sectionDir = (slug: string, device: string) =>
  path.join(screenshotDir(slug), `sections-${device}`);

export const videoDir = (slug: string) =>
  path.join(projectDir(slug), "videos");

export const videoFilePath = (slug: string, label: string) =>
  path.join(videoDir(slug), `${label}-scroll.webm`);

export const coverPath = (slug: string) =>
  path.join(thumbnailDir(slug), "cover.webp");

// v1.4 — composições para redes
export const compositionDir = (slug: string) =>
  path.join(projectDir(slug), "compositions");

// v1.4 — mockups com moldura
export const mockupDir = (slug: string) =>
  path.join(projectDir(slug), "mockups");

// v1.5 — páginas extras (screenshots por página)
export const pageScreenshotDir = (slug: string, pageSlug: string) =>
  path.join(screenshotDir(slug), "pages", pageSlug);

// v1.5 — estados de interação
export const stateScreenshotDir = (slug: string) =>
  path.join(screenshotDir(slug), "states");

// v1.7 — diff visual de recaptura
export const diffDir = (slug: string) =>
  path.join(projectDir(slug), "diffs");

export const caseDraftPath = (slug: string) =>
  path.join(projectDir(slug), "case-draft.mdx");

export const publicPath = (slug: string, ...parts: string[]) =>
  "/" + path.posix.join("generated", slug, ...parts);

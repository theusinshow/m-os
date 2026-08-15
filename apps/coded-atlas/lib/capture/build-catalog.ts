import { config } from "../config";
import type { Catalog, ProjectInput, PageCapture, StateCapture } from "../types";
import type { DeviceCaptureResult } from "./capture-device";
import type { ThumbnailResult } from "./generate-thumbnails";
import type { CoverResult } from "./generate-cover";
import type { CompositionResult } from "./generate-compositions";
import type { MockupResult } from "./generate-mockups";

/** Artefatos opcionais agregados ao catálogo (cresce sem mexer na assinatura). */
export interface CatalogExtras {
  compositions?: CompositionResult[];
  mockups?: MockupResult[];
  pages?: PageCapture[];
  states?: StateCapture[];
}

export function buildCatalog(
  input: ProjectInput,
  captures: { desktop: DeviceCaptureResult; mobile: DeviceCaptureResult },
  thumbnails: ThumbnailResult,
  cover: CoverResult | undefined,
  extras: CatalogExtras,
  startedAt: number
): Catalog {
  const compositions = extras.compositions ?? [];
  const mockups = extras.mockups ?? [];
  const pages = extras.pages ?? [];
  const states = extras.states ?? [];
  const hasVideos =
    captures.desktop.videoPublicPath !== undefined ||
    captures.mobile.videoPublicPath  !== undefined;

  return {
    version: config.atlasVersion,
    project: input,
    captures: {
      // Extrai só os campos públicos — absPath nunca vai no catalog.json
      desktop: {
        viewport:   captures.desktop.viewport,
        screenshot: captures.desktop.screenshot,
        fullpage:   captures.desktop.fullpage,
      },
      mobile: {
        viewport:   captures.mobile.viewport,
        screenshot: captures.mobile.screenshot,
        fullpage:   captures.mobile.fullpage,
      },
    },
    thumbnails: {
      main:   thumbnails.main,
      mobile: thumbnails.mobile,
    },
    ...(hasVideos
      ? {
          videos: {
            desktop: captures.desktop.videoPublicPath,
            mobile:  captures.mobile.videoPublicPath,
          },
        }
      : {}),
    sections: [
      ...captures.desktop.sections,
      ...captures.mobile.sections,
    ],
    ...(pages.length ? { pages } : {}),
    ...(states.length ? { states } : {}),
    ...(captures.desktop.inspection ? { inspection: captures.desktop.inspection } : {}),
    ...(cover ? { cover: { image: cover.cover, source: cover.source } } : {}),
    ...(compositions.length ? { compositions } : {}),
    ...(mockups.length ? { mockups } : {}),
    meta: {
      captureDelayMs: config.captureDelayMs,
      navTimeoutMs:   config.navTimeoutMs,
      durationMs:     Date.now() - startedAt,
      userAgent:      config.userAgent,
    },
    createdAt: new Date().toISOString(),
  };
}

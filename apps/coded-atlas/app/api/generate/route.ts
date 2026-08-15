export const runtime = "nodejs";
export const maxDuration = 120;

import { NextRequest } from "next/server";
import { chromium } from "playwright";
import type { Browser } from "playwright";
import { validateProjectInput } from "@/lib/validation/validate-project-input";
import { ensureProjectFolder } from "@/lib/storage/ensure-project-folder";
import { captureDevice } from "@/lib/capture/capture-device";
import { generateThumbnails } from "@/lib/capture/generate-thumbnails";
import { generateCover } from "@/lib/capture/generate-cover";
import { generateCompositions } from "@/lib/capture/generate-compositions";
import { generateMockups } from "@/lib/capture/generate-mockups";
import { generateMockups3D } from "@/lib/mockup/render-3d";
import { captureExtraPage, resolvePageUrl } from "@/lib/capture/capture-page";
import { captureState } from "@/lib/capture/capture-states";
import { buildCatalog } from "@/lib/capture/build-catalog";
import { writeJson } from "@/lib/storage/write-json";
import { catalogPath } from "@/lib/storage/paths";
import { config } from "@/lib/config";
import { AtlasError } from "@/lib/errors";
import type {
  AtlasErrorPayload,
  ProjectInput,
  PageCapture,
  StateCapture,
  ProgressEvent,
  ResultEvent,
} from "@/lib/types";

const SSE_HEADERS = {
  "Content-Type": "text/event-stream",
  "Cache-Control": "no-cache, no-transform",
  "Connection": "keep-alive",
} as const;

const encoder = new TextEncoder();

function frame(data: ProgressEvent | ResultEvent | AtlasErrorPayload): Uint8Array {
  return encoder.encode(`data: ${JSON.stringify(data)}\n\n`);
}

export async function POST(req: NextRequest): Promise<Response> {
  // Parse — falha de JSON retorna um único evento de erro no stream
  let input: ProjectInput;
  try {
    input = (await req.json()) as ProjectInput;
  } catch {
    const payload: AtlasErrorPayload = {
      step: "error",
      code: "INVALID_URL",
      message: "A URL informada não é válida. Verifique e tente de novo.",
      detail: "Failed to parse request body as JSON",
    };
    return new Response(`data: ${JSON.stringify(payload)}\n\n`, {
      status: 400,
      headers: SSE_HEADERS,
    });
  }

  const startedAt = Date.now();

  const stream = new ReadableStream({
    async start(controller) {
      // Cliente pode fechar a conexão SSE antes do finally (ao receber done/error).
      // Guard silencioso evita "Invalid state: Controller is already closed".
      function emit(data: ProgressEvent | ResultEvent | AtlasErrorPayload): void {
        try { controller.enqueue(frame(data)); } catch { /* cliente desconectou */ }
      }

      // Alias com tipo exato — passado a captureDevice como onProgress
      const emitProgress: (event: ProgressEvent) => void = emit;

      let browser: Browser | undefined;
      try {
        emit({ step: "validating", message: "Validando URL...", progress: 5 });
        validateProjectInput(input);

        emit({ step: "launching", message: "Abrindo navegador...", progress: 10 });
        await ensureProjectFolder(input.slug);
        browser = await chromium.launch({
          headless: config.headless,
          args: ["--no-sandbox", "--disable-dev-shm-usage"],
        });

        // Desktop: emite capturing-desktop (30%) e capturing-fullpage-desktop (40%)
        const desktop = await captureDevice(
          browser, input, config.viewports.desktop, emitProgress
        );

        // Mobile: emite capturing-mobile (50%) e capturing-fullpage-mobile (65%)
        const mobile = await captureDevice(
          browser, input, config.viewports.mobile, emitProgress
        );

        // Páginas extras (v1.5) — captura leve por página, falha por página é silenciosa.
        const pages: PageCapture[] = [];
        const seen = new Set<string>([input.url]);
        const pageEntries = (input.pages ?? [])
          .map((p) => p.trim())
          .filter(Boolean)
          .filter((e) => {
            try {
              const u = resolvePageUrl(e, input.url);
              if (seen.has(u)) return false;
              seen.add(u);
              return true;
            } catch {
              return false;
            }
          })
          .slice(0, config.maxExtraPages);

        for (let i = 0; i < pageEntries.length; i++) {
          emit({
            step: "capturing-pages",
            message: `Capturando página ${i + 1}/${pageEntries.length}: ${pageEntries[i]}`,
            progress: 70,
          });
          try {
            pages.push(await captureExtraPage(browser, input, pageEntries[i], i));
          } catch (err) {
            console.warn(`[atlas:${input.slug}] página "${pageEntries[i]}" falhou: ${err}`);
          }
        }

        // Estados de interação (v1.5) — clica um seletor e fotografa; falha é silenciosa.
        const states: StateCapture[] = [];
        const stateInputs = (input.states ?? [])
          .filter((s) => s?.name?.trim() && s?.selector?.trim())
          .slice(0, config.maxStates);

        for (let i = 0; i < stateInputs.length; i++) {
          emit({
            step: "capturing-states",
            message: `Capturando estado ${i + 1}/${stateInputs.length}: ${stateInputs[i].name}`,
            progress: 75,
          });
          try {
            states.push(await captureState(browser, input, stateInputs[i], i));
          } catch (err) {
            console.warn(`[atlas:${input.slug}] estado "${stateInputs[i].name}" falhou: ${err}`);
          }
        }

        emit({ step: "generating-thumbnails", message: "Gerando thumbnails, capa e composições...", progress: 80 });
        const thumbnails = await generateThumbnails(input.slug, desktop, mobile);
        const cover = await generateCover(
          input.slug,
          desktop.screenshotAbsPath,
          input.url,
          desktop.inspection?.ogImage
        ).catch(() => undefined);
        const compositions = await generateCompositions(input.slug, desktop, mobile).catch(() => []);
        const flatMockups = await generateMockups(input.slug, desktop, mobile).catch(() => []);
        const mockups3d = await generateMockups3D(
          browser, input.slug, desktop.screenshotAbsPath, mobile.screenshotAbsPath
        ).catch(() => []);
        const mockups = [...flatMockups, ...mockups3d];

        emit({ step: "writing-catalog", message: "Montando catálogo...", progress: 92 });
        const catalog = buildCatalog(input, { desktop, mobile }, thumbnails, cover, { compositions, mockups, pages, states }, startedAt);
        await writeJson(catalogPath(input.slug), catalog);

        const result: ResultEvent = {
          step: "done",
          catalog,
          projectUrl: `/projects/${input.slug}`,
        };
        emit(result);

        console.log(`[atlas:${input.slug}] Concluído em ${Date.now() - startedAt}ms`);
      } catch (err) {
        console.error(`[atlas:${input.slug}]`, err instanceof Error ? err.message : err);

        if (err instanceof AtlasError) {
          emit({
            step: "error",
            code: err.code,
            message: err.userMessage,
            detail: err.detail,
          } satisfies AtlasErrorPayload);
        } else {
          emit({
            step: "error",
            code: "UNKNOWN",
            message: "Algo deu errado ao gerar o catálogo.",
            detail: String(err),
          } satisfies AtlasErrorPayload);
        }
      } finally {
        await browser?.close();
        try { controller.close(); } catch { /* já fechado pelo cliente */ }
      }
    },
  });

  return new Response(stream, { headers: SSE_HEADERS });
}

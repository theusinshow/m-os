import { buildCatalog } from "../lib/capture/build-catalog";
import { resolvePageUrl } from "../lib/capture/capture-page";
import type { ProjectInput, PageCapture } from "../lib/types";
import type { DeviceCaptureResult } from "../lib/capture/capture-device";
import type { ThumbnailResult } from "../lib/capture/generate-thumbnails";

let pass = 0, fail = 0;
function ok(label: string, cond: boolean) {
  if (cond) { console.log(`  ✓ ${label}`); pass++; }
  else { console.log(`  ✗ ${label}`); fail++; }
}

const dev = (): DeviceCaptureResult => ({
  viewport: "1440x900",
  screenshot: "/generated/p/screenshots/desktop-1440x900.png",
  fullpage: "/generated/p/screenshots/desktop-fullpage.png",
  screenshotAbsPath: "x",
  fullpageAbsPath: "y",
  sections: [],
});

const thumbs: ThumbnailResult = {
  main: "/generated/p/thumbnails/thumb-main.webp",
  mobile: "/generated/p/thumbnails/thumb-mobile.webp",
};

const page: PageCapture = {
  path: "/sobre",
  url: "https://x.com/sobre",
  desktop: {
    viewport: "1440x900",
    screenshot: "/generated/p/screenshots/pages/sobre/desktop-viewport.png",
    fullpage: "/generated/p/screenshots/pages/sobre/desktop-fullpage.png",
  },
  mobile: {
    viewport: "390x844",
    screenshot: "/generated/p/screenshots/pages/sobre/mobile-viewport.png",
    fullpage: "/generated/p/screenshots/pages/sobre/mobile-fullpage.png",
  },
};

const input: ProjectInput = {
  url: "https://x.com",
  name: "P",
  slug: "p",
  category: "Landing Page",
  pages: ["/sobre"],
};

// ── Com páginas ──
const withPages = buildCatalog(input, { desktop: dev(), mobile: dev() }, thumbs, undefined, { pages: [page] }, Date.now() - 1000);
ok("catalog.pages presente com 1 página", Array.isArray(withPages.pages) && withPages.pages.length === 1);
ok("page tem desktop+mobile com screenshot/fullpage", Boolean(withPages.pages?.[0].desktop.screenshot && withPages.pages?.[0].mobile.fullpage));
ok("project.pages preservado (reprocess herda)", withPages.project.pages?.[0] === "/sobre");
ok("caminhos das páginas são públicos", JSON.stringify(withPages.pages).split('"').filter((s) => s.includes("/generated/")).every((s) => s.startsWith("/generated/")));

// ── Sem páginas ──
const noPages = buildCatalog(input, { desktop: dev(), mobile: dev() }, thumbs, undefined, {}, Date.now() - 1000);
ok("sem páginas → catalog.pages ausente", noPages.pages === undefined);

// ── Round-trip JSON ──
const rt = JSON.parse(JSON.stringify(input)) as ProjectInput;
ok("pages sobrevive ao round-trip", rt.pages?.[0] === "/sobre");

// ── Resolução de URL ──
ok("path relativo resolve contra a base", resolvePageUrl("/sobre", "https://x.com/") === "https://x.com/sobre");
ok("URL completa é usada como está", resolvePageUrl("https://outro.com/x", "https://x.com") === "https://outro.com/x");

console.log(`\n${pass}/${pass + fail} passaram.`);
process.exit(fail === 0 ? 0 : 1);

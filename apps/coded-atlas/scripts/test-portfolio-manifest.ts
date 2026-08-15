import { buildPortfolioManifest } from "../lib/capture/build-portfolio-manifest";
import type { Catalog } from "../lib/types";

let pass = 0, fail = 0;
function ok(label: string, cond: boolean) {
  if (cond) { console.log(`  ✓ ${label}`); pass++; }
  else { console.log(`  ✗ ${label}`); fail++; }
}

const base: Catalog = {
  version: "0.2.0",
  project: {
    url: "https://exemplo.com",
    name: "Projeto X",
    slug: "projeto-x",
    category: "Site Institucional",
    description: "Descrição curta.",
  },
  captures: {
    desktop: { viewport: "1440x900", screenshot: "/generated/projeto-x/screenshots/d.png", fullpage: "/generated/projeto-x/screenshots/df.png" },
    mobile: { viewport: "390x844", screenshot: "/generated/projeto-x/screenshots/m.png", fullpage: "/generated/projeto-x/screenshots/mf.png" },
  },
  thumbnails: { main: "/generated/projeto-x/thumbnails/thumb-main.webp", mobile: "/generated/projeto-x/thumbnails/thumb-mobile.webp" },
  videos: { desktop: "/generated/projeto-x/videos/desktop-scroll.webm" },
  sections: [],
  inspection: { colors: ["#3355ff", "#222222"], fonts: ["Inter"], techStack: ["Next.js", "React"] },
  cover: { image: "/generated/projeto-x/thumbnails/cover.webp", source: "smart-crop" },
  meta: { captureDelayMs: 3000, navTimeoutMs: 30000, durationMs: 12000, userAgent: "ua" },
  createdAt: "2026-06-19T13:00:00.000Z",
};

// ── Caso completo ──
const m = buildPortfolioManifest(base);
ok("slug/name/category mapeados", m.slug === "projeto-x" && m.name === "Projeto X" && m.category === "Site Institucional");
ok("accent = primeira cor da paleta", m.accent === "#3355ff");
ok("palette completa", m.palette.length === 2);
ok("hasVideo true quando há vídeo", m.hasVideo === true);
ok("cover incluída", m.cover === "/generated/projeto-x/thumbnails/cover.webp");
ok("techStack mapeada", m.techStack.join(",") === "Next.js,React");
ok("date em YYYY-MM-DD", m.date === "2026-06-19");
ok("thumbnails públicas (sem caminho absoluto)", m.thumbnail.startsWith("/generated/") && m.thumbnailMobile.startsWith("/generated/"));
ok("atlasVersion propagada", m.atlasVersion === "0.2.0");
// Guard rail: nenhum caminho de arquivo absoluto nos campos de asset
// (a url do projeto é uma URL externa legítima e fica de fora).
const assetFields = [m.thumbnail, m.thumbnailMobile, m.cover ?? ""];
ok("assets sem caminho de arquivo absoluto", assetFields.every(p => !p.includes("C:\\") && !/^[a-zA-Z]:\//.test(p)));

// ── Caso mínimo: sem inspection, sem cover, sem vídeo, sem descrição ──
const minimal: Catalog = {
  ...base,
  project: { url: "https://min.com", name: "Min", slug: "min", category: "App" },
  videos: undefined,
  inspection: undefined,
  cover: undefined,
};
const m2 = buildPortfolioManifest(minimal);
ok("sem inspection → accent ausente", m2.accent === undefined);
ok("sem inspection → palette vazia", m2.palette.length === 0);
ok("sem vídeo → hasVideo false", m2.hasVideo === false);
ok("sem cover → cover ausente", m2.cover === undefined);
ok("sem descrição → description ausente", m2.description === undefined);
ok("techStack vazia (não quebra)", Array.isArray(m2.techStack) && m2.techStack.length === 0);

console.log(`\n${pass}/${pass + fail} passaram.`);
process.exit(fail === 0 ? 0 : 1);

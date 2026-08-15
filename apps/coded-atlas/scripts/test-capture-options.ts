import { buildCatalog } from "../lib/capture/build-catalog";
import type { ProjectInput } from "../lib/types";
import type { DeviceCaptureResult } from "../lib/capture/capture-device";
import type { ThumbnailResult } from "../lib/capture/generate-thumbnails";

let pass = 0, fail = 0;
function ok(label: string, cond: boolean) {
  if (cond) { console.log(`  ✓ ${label}`); pass++; }
  else { console.log(`  ✗ ${label}`); fail++; }
}

const input: ProjectInput = {
  url: "https://exemplo.com",
  name: "Proj",
  slug: "proj",
  category: "Landing Page",
  options: { video: false, sections: false },
};

function device(withVideo: boolean): DeviceCaptureResult {
  return {
    viewport: "1440x900",
    screenshot: "/generated/proj/screenshots/d.png",
    fullpage: "/generated/proj/screenshots/df.png",
    screenshotAbsPath: "x",
    fullpageAbsPath: "y",
    sections: [],
    ...(withVideo ? { videoPublicPath: "/generated/proj/videos/desktop-scroll.webm", videoAbsPath: "z" } : {}),
  };
}

const thumbs: ThumbnailResult = {
  main: "/generated/proj/thumbnails/thumb-main.webp",
  mobile: "/generated/proj/thumbnails/thumb-mobile.webp",
};

// ── Sem vídeo e sem seções ──
const noExtras = buildCatalog(
  input,
  { desktop: device(false), mobile: device(false) },
  thumbs,
  undefined,
  {},
  Date.now() - 1000
);
ok("sem vídeo → sem chave 'videos' no catálogo", noExtras.videos === undefined);
ok("sem seções → sections === []", Array.isArray(noExtras.sections) && noExtras.sections.length === 0);
ok("project.options preservado no catálogo", noExtras.project.options?.video === false && noExtras.project.options?.sections === false);

// ── Com vídeo ──
const withVid = buildCatalog(
  { ...input, options: { video: true, sections: true } },
  { desktop: device(true), mobile: device(true) },
  thumbs,
  undefined,
  {},
  Date.now() - 1000
);
ok("com vídeo → chave 'videos' presente", withVid.videos !== undefined && withVid.videos.desktop !== undefined);

// ── Round-trip JSON preserva options ──
const rt = JSON.parse(JSON.stringify(input)) as ProjectInput;
ok("options sobrevive ao round-trip JSON (reprocess)", rt.options?.video === false && rt.options?.sections === false);

// ── Resolução: ausência cai no default (semântica usada na engine) ──
const resolve = (opt: boolean | undefined, def: boolean) => opt ?? def;
ok("undefined → usa default true", resolve(undefined, true) === true);
ok("false explícito → sobrescreve default", resolve(false, true) === false);

console.log(`\n${pass}/${pass + fail} passaram.`);
process.exit(fail === 0 ? 0 : 1);

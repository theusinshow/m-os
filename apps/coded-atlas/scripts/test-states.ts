import { buildCatalog } from "../lib/capture/build-catalog";
import type { ProjectInput, StateCapture } from "../lib/types";
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

const state: StateCapture = {
  name: "Menu aberto",
  selector: ".menu-toggle",
  screenshot: "/generated/p/screenshots/states/menu-aberto-1.png",
};

const input: ProjectInput = {
  url: "https://x.com",
  name: "P",
  slug: "p",
  category: "Landing Page",
  states: [{ name: "Menu aberto", selector: ".menu-toggle" }],
};

const withStates = buildCatalog(input, { desktop: dev(), mobile: dev() }, thumbs, undefined, { states: [state] }, Date.now() - 1000);
ok("catalog.states presente com 1 estado", Array.isArray(withStates.states) && withStates.states.length === 1);
ok("state tem name/selector/screenshot", Boolean(withStates.states?.[0].name && withStates.states?.[0].selector && withStates.states?.[0].screenshot));
ok("screenshot é público", withStates.states?.[0].screenshot.startsWith("/generated/") === true);
ok("project.states preservado (reprocess herda)", withStates.project.states?.[0].selector === ".menu-toggle");

const noStates = buildCatalog(input, { desktop: dev(), mobile: dev() }, thumbs, undefined, {}, Date.now() - 1000);
ok("sem estados → catalog.states ausente", noStates.states === undefined);

const rt = JSON.parse(JSON.stringify(input)) as ProjectInput;
ok("states sobrevive ao round-trip", rt.states?.[0].name === "Menu aberto");

console.log(`\n${pass}/${pass + fail} passaram.`);
process.exit(fail === 0 ? 0 : 1);

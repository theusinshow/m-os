import path from "node:path";

export const config = {
  outputDir: process.env.ATLAS_OUTPUT_DIR
    ? path.resolve(process.env.ATLAS_OUTPUT_DIR)
    : path.join(process.cwd(), "public", "generated"),

  headless: process.env.ATLAS_HEADLESS !== "false",
  navTimeoutMs: Number(process.env.ATLAS_NAV_TIMEOUT_MS ?? 30000),
  captureDelayMs: Number(process.env.ATLAS_CAPTURE_DELAY_MS ?? 3000),

  // ── Seções (v0.2) ──────────────────────────────────────────────────────────
  // % do viewport que cada passo de scroll avança (0.9 = 90% → leve sobreposição)
  sectionScrollRatio: Number(process.env.ATLAS_SECTION_SCROLL_RATIO ?? 0.9),
  // ms de pausa em cada seção (para animações assentarem e o vídeo mostrar o conteúdo)
  sectionDelayMs: Number(process.env.ATLAS_SECTION_DELAY_MS ?? 1200),
  // cap de segurança: máximo de seções por dispositivo
  maxSections: Number(process.env.ATLAS_MAX_SECTIONS ?? 20),

  // altura mínima (px) para um elemento ser considerado uma seção
  sectionMinHeight: Number(process.env.ATLAS_SECTION_MIN_HEIGHT ?? 200),

  // ── Capa do catálogo ──────────────────────────────────────────────────────
  // Proporção 1.91:1 (padrão OG), ideal para apresentações e redes sociais
  coverWidth:  Number(process.env.ATLAS_COVER_WIDTH  ?? 1200),
  coverHeight: Number(process.env.ATLAS_COVER_HEIGHT ?? 630),

  // ── Composições para redes (v1.4) ──────────────────────────────────────────
  // A captura centralizada sobre um fundo da marca, com moldura e sombra, nos
  // formatos prontos para postar. Tudo tunável aqui (nada hardcoded na engine).
  compositions: {
    background: "#0f1014",   // fundo da marca (~ --color-base)
    paddingRatio: 0.07,      // margem = fração da menor dimensão do canvas
    cornerRadius: 18,        // cantos arredondados da captura
    shadowSigma: 26,         // suavidade da sombra
    shadowOpacity: 0.5,      // opacidade da sombra
    shadowOffsetY: 28,       // deslocamento vertical da sombra
    formats: [
      { name: "social-1x1",  label: "Quadrado (1:1)", width: 1080, height: 1080, source: "desktop" },
      { name: "social-9x16", label: "Story (9:16)",   width: 1080, height: 1920, source: "mobile"  },
      { name: "social-16x9", label: "Paisagem (16:9)", width: 1200, height: 675, source: "desktop" },
    ],
  },

  // ── Mockups com moldura (v1.4) ─────────────────────────────────────────────
  // Molduras desenhadas via SVG (sem asset externo). Fundo transparente para
  // reuso em qualquer layout de case. Tudo tunável aqui.
  mockups: {
    pad: 90,            // margem ao redor (espaço para a sombra)
    shadowSigma: 30,
    shadowOpacity: 0.45,
    shadowOffsetY: 30,
    browser: {
      width: 1600,
      chromeHeight: 48,
      radius: 14,
      chrome: "#15171c",
      addressBar: "#23262d",
      border: "#2a2d34",
      dots: ["#ff5f57", "#febc2e", "#28c840"],
    },
    phone: {
      screenWidth: 460,
      bezelRatio: 0.035,
      bodyRadiusRatio: 0.16,
      body: "#0a0a0c",
      border: "#2a2d34",
    },
  },

  // ── Mockups 3D em perspectiva (v1.4 item 12) ───────────────────────────────
  // Renderizados via HTML+CSS 3D, fotografados pelo Playwright (fundo transparente).
  mockups3d: {
    deviceScaleFactor: 2,
    desktop: { sceneW: 1640, sceneH: 1180, windowW: 1120, rotateY: -20, rotateX: 6, perspective: 2200 },
    mobile:  { sceneW: 880,  sceneH: 1340, phoneW: 380,   rotateY: 18,  rotateX: 5, perspective: 2000 },
  },

  // ── Animação de scroll (vídeo) ─────────────────────────────────────────────
  // Mais passos + timing alinhado a 60fps → ~24 frames de animação por seção
  scrollSteps: Number(process.env.ATLAS_SCROLL_STEPS ?? 60),
  scrollStepMs: Number(process.env.ATLAS_SCROLL_STEP_MS ?? 16),

  // ── Features v0.2 (default ON; defina "false" via env para desabilitar) ────
  captureSections: process.env.ATLAS_CAPTURE_SECTIONS !== "false",
  captureVideo:    process.env.ATLAS_CAPTURE_VIDEO    !== "false",

  // ── Páginas extras (v1.5) ──────────────────────────────────────────────────
  // Cap de segurança: máximo de páginas adicionais por projeto.
  maxExtraPages: Number(process.env.ATLAS_MAX_EXTRA_PAGES ?? 10),

  // ── Estados de interação (v1.5) ────────────────────────────────────────────
  maxStates: Number(process.env.ATLAS_MAX_STATES ?? 10),
  // pausa após clicar o seletor, para a animação assentar antes do print
  stateSettleMs: Number(process.env.ATLAS_STATE_SETTLE_MS ?? 800),

  userAgent:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 " +
    "(KHTML, like Gecko) Chrome/124.0 Safari/537.36",

  viewports: {
    desktop: { label: "desktop", width: 1440, height: 900,  deviceScaleFactor: 2 },
    mobile:  { label: "mobile",  width: 390,  height: 844,  deviceScaleFactor: 3 },
  },

  onSlugConflict: "overwrite" as "overwrite" | "fail",
  atlasVersion: "0.2.0",
} as const;

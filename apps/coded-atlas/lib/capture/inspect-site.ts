import type { Page } from "playwright";
import type { SiteInspection } from "../types";

/**
 * Inspeciona a página para extrair paleta, tipografia e tech stack.
 * Executa em uma única page.evaluate para minimizar round-trips.
 * Falhas são silenciosas — retorna objeto vazio em caso de erro.
 */
export async function inspectSite(page: Page): Promise<SiteInspection> {
  const result = await page
    .evaluate((): { colors: string[]; fonts: string[]; techStack: string[]; ogImage?: string } => {
      // ── Helpers de cor ──────────────────────────────────────────────────────
      function parseHex(css: string): string | null {
        if (!css || css === "transparent") return null;
        const m = css.match(/rgba?\s*\(\s*(\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\s*\)/);
        if (!m) return null;
        const alpha = m[4] !== undefined ? parseFloat(m[4]) : 1;
        if (alpha < 0.15) return null; // quase transparente — ignora
        return (
          "#" +
          [m[1], m[2], m[3]]
            .map((n) => Number(n).toString(16).padStart(2, "0"))
            .join("")
        );
      }

      // Cores puramente neutras (branco/preto absoluto) são muito comuns
      // como defaults e não identificam a paleta do projeto
      const IGNORE_COLORS = new Set(["#000000", "#ffffff"]);

      const colorSet = new Set<string>();
      const COLOR_PROPS = ["backgroundColor", "color", "borderTopColor"] as const;
      const SAMPLE_SELECTORS = [
        "body", "header", "footer", "main", "nav",
        "h1", "h2", "p", "a", "button",
        "[class*='btn']", "[class*='hero']", "[class*='primary']",
      ];

      for (const sel of SAMPLE_SELECTORS) {
        try {
          const el = document.querySelector(sel);
          if (!el) continue;
          const style = getComputedStyle(el);
          for (const prop of COLOR_PROPS) {
            const hex = parseHex(style[prop] ?? "");
            if (hex && !IGNORE_COLORS.has(hex)) colorSet.add(hex);
          }
        } catch {
          // ignora erros por elemento
        }
      }

      // ── Fontes ──────────────────────────────────────────────────────────────
      const fontSet = new Set<string>();
      const SYSTEM_FONTS = new Set([
        "system-ui", "-apple-system", "sans-serif", "serif", "monospace",
        "ui-sans-serif", "ui-serif", "ui-monospace", "cursive", "fantasy",
      ]);

      try {
        document.fonts.forEach((f) => {
          const name = f.family.replace(/['"]/g, "").trim();
          if (name && !SYSTEM_FONTS.has(name.toLowerCase())) fontSet.add(name);
        });
      } catch { /* alguns browsers não suportam */ }

      for (const sel of ["body", "h1", "h2", "p"]) {
        try {
          const el = document.querySelector(sel);
          if (!el) continue;
          getComputedStyle(el)
            .fontFamily.split(",")
            .forEach((f) => {
              const name = f.trim().replace(/['"]/g, "");
              if (name && !SYSTEM_FONTS.has(name.toLowerCase()) && !name.startsWith("-")) {
                fontSet.add(name);
              }
            });
        } catch { /* ignora */ }
      }

      // ── Tech Stack ──────────────────────────────────────────────────────────
      const stack: string[] = [];
      const w = window as unknown as Record<string, unknown>;

      // JS frameworks — globals injetados em runtime
      if (w.__NEXT_DATA__) stack.push("Next.js");
      if (w.__NUXT__ || w.__nuxt) stack.push("Nuxt.js");
      if (w.React || document.querySelector("[data-reactroot]")) stack.push("React");
      if (w.Vue || w.__vue_app__) stack.push("Vue.js");
      if (w.angular || document.querySelector("[ng-version]")) stack.push("Angular");
      if (w.svelte) stack.push("Svelte");
      if (w.gsap) stack.push("GSAP");
      if (w.Webflow) stack.push("Webflow");
      if (w.Framer) stack.push("Framer");

      // CMS / construtores — meta generator
      const gen = (
        document.querySelector('meta[name="generator"]')?.getAttribute("content") ?? ""
      ).toLowerCase();
      if (gen.includes("wordpress")) stack.push("WordPress");
      else if (gen.includes("webflow") && !stack.includes("Webflow")) stack.push("Webflow");
      else if (gen.includes("framer") && !stack.includes("Framer")) stack.push("Framer");
      else if (gen.includes("wix")) stack.push("Wix");
      else if (gen.includes("squarespace")) stack.push("Squarespace");
      else if (gen.includes("shopify")) stack.push("Shopify");
      else if (gen.includes("ghost")) stack.push("Ghost");

      // Atributos DOM proprietários
      if (!stack.includes("Webflow") && document.querySelector("[data-wf-site], html[data-wf-page]")) {
        stack.push("Webflow");
      }
      if (document.querySelector(".elementor")) stack.push("Elementor");
      if (document.querySelector("[data-reactroot]") && !stack.includes("React")) {
        stack.push("React");
      }

      // ── OG Image ────────────────────────────────────────────────────────────
      const ogImage =
        document.querySelector('meta[property="og:image"]')?.getAttribute("content") ||
        document.querySelector('meta[name="twitter:image"]')?.getAttribute("content") ||
        document.querySelector('meta[property="og:image:url"]')?.getAttribute("content") ||
        undefined;

      return {
        colors: [...colorSet].slice(0, 10),
        fonts: [...fontSet].slice(0, 6),
        techStack: [...new Set(stack)],
        ogImage: ogImage || undefined,
      };
    })
    .catch(() => ({ colors: [] as string[], fonts: [] as string[], techStack: [] as string[] }));

  return result;
}

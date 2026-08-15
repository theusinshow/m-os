import type { Page } from "playwright";
import { config } from "../config";

/**
 * Aguarda a página estabilizar antes de capturar.
 *
 * Usa `config.captureDelayMs` como orçamento total de tempo:
 * - dentro desse orçamento, resolve condições de estabilidade em paralelo;
 * - o tempo restante é consumido como buffer de segurança.
 * - se as verificações levarem mais que o orçamento, aguarda até concluírem.
 *
 * As verificações nunca bloqueiam a captura — todas têm `.catch(() => {})`.
 */
export async function waitForPageStability(page: Page): Promise<void> {
  const budget = config.captureDelayMs;
  const start = Date.now();
  const remaining = () => Math.max(50, budget - (Date.now() - start));

  // 1. Fontes carregadas (document.fonts.ready é uma Promise nativa)
  await page.evaluate(() => document.fonts.ready).catch(() => {});

  // 2. Imagens acima do fold carregadas (lazy-load incluso)
  await page
    .waitForFunction(
      () => {
        const imgs = Array.from(document.querySelectorAll("img")) as HTMLImageElement[];
        return imgs
          .filter((img) => img.getBoundingClientRect().top < window.innerHeight)
          .every((img) => img.complete && img.naturalWidth > 0);
      },
      { timeout: Math.min(remaining(), 6000), polling: 200 }
    )
    .catch(() => {});

  // 3. Animações CSS finitas concluídas (ignora animações infinitas como spinners)
  await page
    .waitForFunction(
      () => {
        const finite = document.getAnimations().filter((a) => {
          const dur = a.effect?.getTiming().duration;
          return typeof dur === "number" && isFinite(dur) && dur < 4000;
        });
        return finite.length === 0;
      },
      { timeout: Math.min(remaining(), 3000), polling: 100 }
    )
    .catch(() => {});

  // 4. Buffer final: consome o restante do orçamento
  const rest = remaining();
  if (rest > 0) await page.waitForTimeout(rest);
}

import type { Page } from "playwright";
import { config } from "../config";

/**
 * Scroll suave com easeInOutQuad executado inteiramente no browser.
 *
 * Roda em um único page.evaluate (sem round-trips Node↔browser por passo),
 * o que garante timing uniforme via setTimeout nativo e permite que o
 * compositor do Chromium grave cada frame no vídeo sem interferência.
 *
 * 60 passos × 16ms ≈ 960ms de animação → ~24 frames a 25fps = scroll fluido.
 */
export async function smoothScrollTo(page: Page, targetY: number): Promise<void> {
  const currentY: number = await page.evaluate(() => window.scrollY);
  const diff = targetY - currentY;
  if (Math.abs(diff) < 2) return;

  await page.evaluate(
    ({ from, to, steps, stepMs }: { from: number; to: number; steps: number; stepMs: number }) =>
      new Promise<void>((resolve) => {
        let i = 0;
        const d = to - from;
        const tick = () => {
          i++;
          const t = i / steps;
          // easeInOutQuad
          const e = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
          window.scrollTo(0, from + d * e);
          if (i < steps) setTimeout(tick, stepMs);
          else resolve();
        };
        setTimeout(tick, stepMs);
      }),
    { from: currentY, to: targetY, steps: config.scrollSteps, stepMs: config.scrollStepMs }
  );
}

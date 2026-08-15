import type { Page } from "playwright";
import { deriveSectionName, disambiguate, type SectionHints } from "./section-name";

export interface SectionCandidate {
  tag: string;
  id?: string;
  y: number;            // scroll Y absoluto para capturar esta seção
  elementHeight: number;
  suggestedName?: string;
  heading?: string;
}

interface RawSection extends SectionHints {
  y: number;
  elementHeight: number;
}

/**
 * Detecta seções semânticas da página via análise do DOM.
 *
 * Estratégia:
 * 1. Busca elementos com tags estruturais: header, section, footer, article,
 *    e atributos de marcação: [data-section], [data-block].
 * 2. Filtra elementos com altura < minHeight ou sem largura (invisíveis).
 * 3. Aplica regra de "folha": descarta um elemento se ele contém outro
 *    candidato — evita capturar container + conteúdo duas vezes.
 * 4. O `page.evaluate` só extrai pistas (tag, id, classe, aria, heading); os
 *    nomes legíveis são derivados no Node por `section-name.ts` (testável).
 * 5. Desambigua nomes repetidos e ordena por Y crescente.
 *
 * Retorna array vazio se a página não tiver estrutura semântica suficiente.
 */
export async function detectPageSections(
  page: Page,
  minHeight: number
): Promise<SectionCandidate[]> {
  const raw: RawSection[] = await page.evaluate((minH: number) => {
    const SELECTOR = "header, section, footer, article, [data-section], [data-block]";
    const all: Element[] = Array.from(document.querySelectorAll(SELECTOR));

    const candidates = all.filter((el) => {
      const he = el as HTMLElement;
      return he.offsetHeight >= minH && he.offsetWidth > 0;
    });

    // Prefere folhas: descarta container que contém outro candidato.
    const leaves = candidates.filter(
      (el) => !candidates.some((other) => other !== el && el.contains(other))
    );

    const scrollY = window.scrollY;

    return leaves
      .map((el) => {
        const rect = el.getBoundingClientRect();
        const absTop = Math.max(0, Math.round(rect.top + scrollY));
        const tag = el.tagName.toLowerCase();

        const dataSection = el.getAttribute("data-section") ?? undefined;
        const dataBlock = el.getAttribute("data-block") ?? undefined;
        const id = dataSection || dataBlock || (el as HTMLElement).id || undefined;

        const hEl = el.querySelector("h1, h2, h3");
        const heading = hEl
          ? (hEl.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 100) || undefined
          : undefined;

        return {
          tag,
          id,
          className: el.getAttribute("class") ?? undefined,
          ariaLabel: el.getAttribute("aria-label") ?? undefined,
          dataSection,
          dataBlock,
          heading,
          y: absTop,
          elementHeight: (el as HTMLElement).offsetHeight,
        };
      })
      .sort((a, b) => a.y - b.y);
  }, minHeight);

  // Nomes derivados no Node (lógica pura, testável) + desambiguação.
  const names = disambiguate(raw.map((r) => deriveSectionName(r)));

  return raw.map((r, i) => ({
    tag: r.tag,
    id: r.id,
    y: r.y,
    elementHeight: r.elementHeight,
    suggestedName: names[i],
    heading: r.heading,
  }));
}

import type { ReactNode } from "react";
import type { RegisteredApp } from "./types";

/**
 * Identidade visual de um App do registro.
 *
 * Duas fontes, nesta ordem: uma marca, quando o App **e** um produto conhecido;
 * um monograma, quando ele e seu.
 *
 * A regra que separa as duas e que hospedagem nao e identidade. `m-finance`
 * mora em `vercel.app` e o codigo mora em `github.com`, mas o App nao e a
 * Vercel nem o GitHub — e o M Finance. Dominio de hospedagem nunca vira marca,
 * senao metade do registro pessoal apareceria com o triangulo da Vercel.
 *
 * Marcas sao preenchidas e monocromaticas, em `currentColor`. Elas nao seguem o
 * sistema de traco dos icones do M/OS porque nao sao icones do M/OS: sao
 * logotipos de terceiros, e distorce-los para caber numa grade de 1.25 seria
 * pior que respeitar a forma original.
 *
 * So entram marcas que da para desenhar com fidelidade. Um logo aproximado le
 * como erro, e um monograma bem feito le como decisao — por isso Notion, Linear
 * e Discord ainda saem como monograma em vez de sairem tortos.
 */

type Brand = { viewBox: string; path: ReactNode };

const BRANDS: Record<string, Brand> = {
  github: {
    viewBox: "0 0 16 16",
    path: (
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 2-.27c.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    ),
  },
  // Cinco formas de 19 unidades, raio 9.5. E geometria pura, entao sobrevive a
  // monocromia sem virar borrao.
  figma: {
    viewBox: "0 0 38 57",
    path: (
      <>
        <path d="M19 28.5a9.5 9.5 0 1 1 19 0 9.5 9.5 0 0 1-19 0z" />
        <path d="M0 47.5A9.5 9.5 0 0 1 9.5 38H19v9.5a9.5 9.5 0 0 1-19 0z" />
        <path d="M19 0v19h9.5a9.5 9.5 0 1 0 0-19H19z" />
        <path d="M0 9.5A9.5 9.5 0 0 0 9.5 19H19V0H9.5A9.5 9.5 0 0 0 0 9.5z" />
        <path d="M0 28.5A9.5 9.5 0 0 0 9.5 38H19V19H9.5A9.5 9.5 0 0 0 0 28.5z" />
      </>
    ),
  },
  vercel: { viewBox: "0 0 16 16", path: <path d="M8 1 15.5 14.8H.5z" /> },
  // O triangulo e um FURO, via evenodd, e nao uma forma pintada por cima. Com
  // `fill` de token ele so ficaria certo sobre o fundo que o token descreve — e
  // o App icon fica sobre `--surface-raised`, nao sobre o canvas.
  youtube: {
    viewBox: "0 0 24 24",
    path: (
      <path
        fillRule="evenodd"
        d="M23 7.5a3 3 0 0 0-2.1-2.1C19 4.8 12 4.8 12 4.8s-7 0-8.9.6A3 3 0 0 0 1 7.5C.4 9.4.4 12 .4 12s0 2.6.6 4.5a3 3 0 0 0 2.1 2.1c1.9.6 8.9.6 8.9.6s7 0 8.9-.6a3 3 0 0 0 2.1-2.1c.6-1.9.6-4.5.6-4.5s0-2.6-.6-4.5zM9.8 15.5V8.5l5.8 3.5z"
      />
    ),
  },
};

/** Dominios que sao HOSPEDAGEM, nunca identidade. */
const HOSTING = [
  "vercel.app",
  "netlify.app",
  "github.io",
  "pages.dev",
  "onrender.com",
  "fly.dev",
  "surge.sh",
  "herokuapp.com",
];

/** Dominio de produto -> marca. Casado por sufixo, para pegar subdominios. */
const PRODUCTS: [string, string][] = [
  ["github.com", "github"],
  ["figma.com", "figma"],
  ["vercel.com", "vercel"],
  ["youtube.com", "youtube"],
  ["youtu.be", "youtube"],
];

function hostOf(url: string | null): string {
  if (!url) return "";
  try {
    return new URL(url).hostname.toLowerCase();
  } catch {
    return "";
  }
}

/**
 * Escolhe a marca de um App.
 *
 * O alvo de abertura vale mais que a origem: um App que ABRE no Figma e o
 * Figma, mesmo que o codigo dele viva no GitHub. A origem so decide quando nao
 * ha alvo — e mesmo ai, hospedagem nao conta.
 */
export function brandOf(app: Pick<RegisteredApp, "launchKind" | "launchTarget" | "sourceUrl">): string | null {
  const candidates = [app.launchKind === "url" ? app.launchTarget : null, app.sourceUrl];
  for (const candidate of candidates) {
    const host = hostOf(candidate ?? null);
    if (!host) continue;
    if (HOSTING.some((suffix) => host === suffix || host.endsWith(`.${suffix}`))) continue;
    const product = PRODUCTS.find(([domain]) => host === domain || host.endsWith(`.${domain}`));
    if (product) return product[1];
  }
  return null;
}

/**
 * Monograma. Iniciais de ate duas palavras; uma palavra so devolve uma letra.
 *
 * Duas letras de uma palavra so ("Cr", "Cо") leem como abreviacao truncada e
 * nao como marca. Uma letra e mais silenciosa e mais honesta.
 */
export function monogramOf(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (!words.length) return "?";
  if (words.length === 1) return words[0].charAt(0).toUpperCase();
  return (words[0].charAt(0) + words[1].charAt(0)).toUpperCase();
}

export function AppIcon({ app }: { app: Pick<RegisteredApp, "name" | "launchKind" | "launchTarget" | "sourceUrl"> }) {
  const brand = brandOf(app);
  const mark = brand ? BRANDS[brand] : undefined;

  return (
    <span className="app-icon" data-brand={brand ?? undefined} aria-hidden="true">
      {mark ? (
        <svg className="app-mark" viewBox={mark.viewBox} width="22" height="22" focusable="false">
          {mark.path}
        </svg>
      ) : (
        <span className="app-monogram" data-len={monogramOf(app.name).length}>{monogramOf(app.name)}</span>
      )}
    </span>
  );
}

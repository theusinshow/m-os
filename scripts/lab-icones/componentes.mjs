// Os icones que moram em componente React, renderizados de verdade.
//
// O Lab nao copia o desenho do `Icon.tsx`: ele empacota o proprio arquivo com o
// esbuild do desktop e chama `renderToStaticMarkup`. Assim o que a bancada mostra
// e o que o app desenha, e uma mudanca no componente aparece na proxima rodada.

import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

/** Os nomes de um union de string literal, lidos do fonte. */
function nomesDoUnion(fonte, tipo) {
  const corpo = fonte.match(new RegExp(`export type ${tipo} =([^;]+);`))?.[1] ?? "";
  return [...corpo.matchAll(/"([^"]+)"/g)].map(([, nome]) => nome);
}

function chavesDoObjeto(fonte, constante) {
  const inicio = fonte.indexOf(`const ${constante}`);
  const fim = fonte.indexOf("\n};", inicio);
  return [...fonte.slice(inicio, fim).matchAll(/^\s{2}([a-z_]+):/gm)].map(([, nome]) => nome);
}

export async function renderizarComponentes(raiz) {
  const desktop = join(raiz, "apps/desktop");
  const src = join(desktop, "src");
  const fonteIcon = readFileSync(join(src, "Icon.tsx"), "utf8");
  const fonteApp = readFileSync(join(src, "AppIcon.tsx"), "utf8");
  const nomes = nomesDoUnion(fonteIcon, "IconName");
  const solidos = chavesDoObjeto(fonteIcon, "SOLID_20");
  const pequenos = chavesDoObjeto(fonteIcon, "OUTLINE_16");
  const marcas = chavesDoObjeto(fonteApp, "BRANDS");

  const entrada = `
    import { renderToStaticMarkup as r } from "react-dom/server";
    import { createElement as h } from "react";
    import { Icon, SmallIcon } from "./Icon";
    import { MosSymbol } from "./Symbol";
    import { AppIcon } from "./AppIcon";
    import { MarcaDeIA } from "./MarcaDeIA";
    export const icones = ${JSON.stringify(nomes)}.map((nome) => ({ nome, svg: r(h(Icon, { name: nome })) }));
    export const solidos = ${JSON.stringify(solidos)}.map((nome) => ({ nome, svg: r(h(Icon, { name: nome, filled: true })) }));
    export const pequenos = ${JSON.stringify(pequenos)}.map((nome) => ({ nome, svg: r(h(SmallIcon, { name: nome })) }));
    export const simbolos = [16, 24, 32, 48, 64, 128].map((tamanho) => ({ tamanho, svg: r(h(MosSymbol, { size: tamanho })) }));
    const hosts = { github: "github.com", figma: "figma.com", vercel: "vercel.com", youtube: "youtube.com" };
    export const marcas = ${JSON.stringify(marcas)}.map((nome) => ({
      nome,
      svg: r(h(AppIcon, { app: { name: nome, launchKind: "url", launchTarget: "https://" + (hosts[nome] ?? nome + ".com"), sourceUrl: null } })),
    })).concat([{ nome: "monograma", svg: r(h(AppIcon, { app: { name: "Meu App", launchKind: "url", launchTarget: null, sourceUrl: null } })) }]);
    export const ias = ["Claude", "Codex", "Cursor", "Gemini", "Grok", "Outro"].map((nome) => ({ nome, svg: r(h(MarcaDeIA, { nome, tamanho: 18 })) }));
  `;

  const { build } = await import(pathToFileURL(join(desktop, "node_modules/esbuild/lib/main.js")).href);
  const pasta = mkdtempSync(join(tmpdir(), "lab-icones-"));
  const saida = join(pasta, "componentes.mjs");
  try {
    await build({
      stdin: { contents: entrada, resolveDir: src, loader: "tsx" },
      bundle: true,
      format: "esm",
      platform: "node",
      jsx: "automatic",
      // O react-dom/server e CommonJS e pede `require("util")`; em ESM o
      // esbuild precisa de um `require` de verdade para isso.
      banner: { js: 'import { createRequire } from "node:module"; const require = createRequire(import.meta.url);' },
      outfile: saida,
      logLevel: "error",
    });
    const modulo = await import(pathToFileURL(saida).href);
    return { ...modulo };
  } finally {
    rmSync(pasta, { recursive: true, force: true });
  }
}

/** As regras de CSS do app que desenham icone, copiadas do App.css real. */
export function cssDosIcones(appCss) {
  const seletores = [".mos-icon", ".mos-symbol", ".app-icon", ".app-mark", ".app-monogram"];
  const regras = [];
  let profundidade = 0;
  let inicio = 0;
  for (let i = 0; i < appCss.length; i++) {
    if (appCss[i] === "{") {
      if (profundidade === 0) inicio = appCss.lastIndexOf("}", i) + 1;
      profundidade++;
    } else if (appCss[i] === "}") {
      profundidade--;
      if (profundidade === 0) {
        const regra = appCss.slice(inicio, i + 1).replace(/\/\*[\s\S]*?\*\//g, "").trim();
        const seletor = regra.slice(0, regra.indexOf("{"));
        if (!seletor.startsWith("@") && seletores.some((s) => seletor.includes(s))) regras.push(regra);
      }
    }
  }
  return regras.join("\n");
}

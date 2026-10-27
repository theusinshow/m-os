// Lab de icones e logos — gera `spikes/lab-icones.html`.
//
//   cd apps/desktop && npm run lab
//
// Varre o repositorio, mede cada icone contra a regra que o codigo declara e
// escreve uma pagina so, com tudo embutido. A bancada so aponta: nada aqui
// corrige icone nem bloqueia build. Ver
// `docs/superpowers/specs/2026-10-27-lab-de-icones-design.md`.

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { cssDosIcones, renderizarComponentes } from "./componentes.mjs";
import { escreverPng, lerIcns, lerIco, lerPng, lerRgba, reduzir } from "./imagens.mjs";
import {
  conferirIcone,
  coresForaDaPaleta,
  diferenca,
  faixaDe,
  inclinacaoDoPoligono,
  inclinacaoNoBitmap,
  raioNoBitmap,
  selo,
} from "./medidas.mjs";
import { montarPagina } from "./pagina.mjs";

const RAIZ = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const ler = (caminho) => readFileSync(join(RAIZ, caminho), "utf8");
const git = (...args) => execFileSync("git", args, { cwd: RAIZ, encoding: "utf8", maxBuffer: 1 << 26 });

// ---------------------------------------------------------------------------
// O esperado, lido das fontes. Nada de numero repetido aqui.

function lerFontes() {
  const symbol = ler("apps/desktop/src/Symbol.tsx");
  const barras = {};
  const rotulos = {};
  for (const [, graus, faixa, pontos] of symbol.matchAll(/\/\*\*[^*]*?(\d+) graus \*\/\s*(\w+): "([^"]+)"/g)) {
    barras[faixa] = pontos.split(" ").map((p) => p.split(",").map(Number));
    rotulos[faixa] = Number(graus);
  }

  const py = ler("scripts/gerar-icones.py");
  const raioMos = Number(py.match(/^RAIO = ([\d.]+)/m)[1]);

  const familia = ler("scripts/generate-family-icons.py");
  const receita = {};
  for (const [, lado, raio] of familia.matchAll(/^\s+(\d+): \((\d+), [\d.]+, "/gm)) receita[lado] = Number(raio);
  const campos = {};
  for (const [, app, campo] of familia.matchAll(/"([\w-]+)": \("(#[0-9A-Fa-f]{6})", FIELD\)/g)) campos[app] = campo;
  const glifos = {};
  for (const [, app, glifo] of familia.matchAll(/"([\w-]+)": "(\w+)",/g)) glifos[app] = glifo;

  const web = ler("scripts/gerar-icones-web.py");
  const webCheio = !/rounded_rectangle/.test(web);

  const tokens = ler("packages/design-system/tokens.css");
  const escuro = tokens.slice(0, tokens.indexOf("[data-theme='light']"));
  const claro = tokens.slice(tokens.indexOf("[data-theme='light']"));
  const token = (bloco, nome) => bloco.match(new RegExp(`--${nome}:\\s*(#[0-9A-Fa-f]{6}|[\\d.]+px)`))?.[1];
  const nomesDePaleta = ["canvas", "surface", "border", "border-strong", "text", "text-secondary", "text-system", "signal-fill", "on-signal"];
  const paleta = [...new Set(nomesDePaleta.flatMap((n) => [token(escuro, n), token(claro, n)]).filter(Boolean).map((h) => h.toUpperCase()))];
  const tokensEscuros = Object.fromEntries(nomesDePaleta.map((n) => [n, token(escuro, n)]));
  const traco = { 20: parseFloat(token(tokens, "stroke-20")), 16: parseFloat(token(tokens, "stroke-16")) };

  const brief = ler("docs/BRIEF-SISTEMA-DE-LOGOS.md");
  const briefPaleta = {};
  const linhaPaleta = brief.match(/\*\*Paleta:\*\*([^\n]+(?:\n[^\n]+)?)/)?.[1] ?? "";
  for (const [, nome, hex] of linhaPaleta.matchAll(/([a-zà-ú ]+?) `(#[0-9A-Fa-f]{6})`/gi)) briefPaleta[nome.trim()] = hex.toUpperCase();
  const briefRaio = Number(brief.match(/Raio do ícone ≈ (\d+)%/)?.[1]) / 100;
  const briefTraco = brief.match(/Stroke[^\n]*/i)?.[0] ?? null;

  return { barras, rotulos, raioMos, receita, campos, glifos, webCheio, paleta, tokensEscuros, traco, briefPaleta, briefRaio, briefTraco, tokens };
}

// ---------------------------------------------------------------------------
// Inventario

const GRUPOS = [
  { prefixo: "apps/desktop/src-tauri/icons/", grupo: "M/OS desktop", familia: "mos", app: "mos" },
  { prefixo: "apps/mos-web/ui/public/", grupo: "M/OS de bolso", familia: "mos", app: "mos-web" },
  { prefixo: "apps/cronocad/", grupo: "CronoCAD", familia: "irmao", app: "cronocad" },
  { prefixo: "apps/m-finance/", grupo: "M-Finance", familia: "irmao", app: "m-finance" },
  { prefixo: "apps/coded-atlas/", grupo: "Coded Atlas", familia: "irmao", app: "coded-atlas" },
  { prefixo: "packages/design-system/marks/", grupo: "Glifos da família", familia: "glifo", app: null },
  { prefixo: "spikes/", grupo: "Spikes", familia: "spike", app: null },
];

// Arquivos que o bundler acha pelo nome, sem ninguem cita-los.
const IMPLICITOS = [
  /src-tauri\/icons\/(Square\d+x\d+Logo|StoreLogo)\.png$/,
  /\/app\/(icon\.svg|apple-icon\.png)$/,
];

function inventario() {
  return git("ls-files")
    .split("\n")
    .filter((c) => /\.(svg|png|ico|icns|rgba)$/i.test(c) && !c.includes("node_modules"))
    .map((caminho) => ({ caminho, ...(GRUPOS.find((g) => caminho.startsWith(g.prefixo)) ?? { grupo: "Outros", familia: "outro", app: null }) }));
}

function usosDe(caminho, familia) {
  if (IMPLICITOS.some((r) => r.test(caminho))) return { usos: ["achado pelo bundler pelo nome"], orfao: false };
  // O glifo e montado por nome no script: `f"{glyph}-{variant}.svg"`. Ele e
  // citado pela raiz ("timer") e pela variante ("wght500fill1"), nunca inteiro.
  const nome = familia === "glifo" ? basename(caminho).split("-")[0] : basename(caminho);
  let saida = "";
  try {
    saida = git("grep", "-l", "-F", nome, "--", ".", ":!docs", ":!*.md", ":!*.png", ":!*.ico", ":!*.icns");
  } catch {
    saida = "";
  }
  const usos = saida.split("\n").filter(Boolean);
  return { usos, orfao: usos.length === 0 };
}

// ---------------------------------------------------------------------------
// Medidas por quadro

const TINTA = [10, 12, 14];
const SODIO = [231, 194, 78];

function tolerancia(tamanho) {
  return tamanho >= 48 ? 2.5 : 5;
}

function seloDaBarra(medido, tamanho, fontes) {
  const faixa = faixaDe(tamanho);
  const esperado = inclinacaoDoPoligono(fontes.barras[faixa]);
  if (medido === null) return selo("nao-medivel", "barra", "", "", "a tinta nao formou linhas suficientes");
  const parecida = Object.entries(fontes.barras)
    .map(([f, p]) => [f, Math.abs(inclinacaoDoPoligono(p) - medido)])
    .sort((a, b) => a[1] - b[1])[0][0];
  const ok = Math.abs(medido - esperado) <= tolerancia(tamanho);
  return selo(
    ok ? "ok" : "desvio",
    "barra",
    `${medido.toFixed(1)}°`,
    `${esperado.toFixed(1)}° (${faixa})`,
    ok ? "" : `parece a barra ${parecida}`,
  );
}

function seloDoRaio(imagem, esperado, fonte) {
  const medido = raioNoBitmap(imagem);
  if (medido === null) return selo("nao-medivel", "raio", "", "", "o campo nao vai ate a borda");
  const folga = Math.max(0.015, 0.6 / imagem.largura);
  const ok = Math.abs(medido - esperado) <= folga;
  return selo(ok ? "ok" : "desvio", "raio", `${(medido * 100).toFixed(1)}%`, `${(esperado * 100).toFixed(0)}%`, ok ? "" : fonte);
}

function seloDaCor(imagem, fontes, { excecoes = [], icc = false } = {}) {
  const achado = coresForaDaPaleta(imagem, fontes.paleta);
  if (achado.fracao <= 0.005) return selo("ok", "cor");
  const medido = achado.exemplos.map(([hex]) => hex).join(" ");
  if (icc) return selo("excecao", "cor", medido, "paleta sRGB", "PNG com perfil ICC (Display P3): o numero do pixel nao e sRGB");
  if (excecoes.length && coresForaDaPaleta(imagem, [...fontes.paleta, ...excecoes]).fracao <= 0.005) {
    return selo("excecao", "cor", medido, "paleta", `campo declarado em generate-family-icons.py: ${excecoes.join(" ")}`);
  }
  return selo("desvio", "cor", `${(achado.fracao * 100).toFixed(1)}% fora: ${medido}`, "paleta e sodio");
}

function raioEsperado(item, tamanho, fontes) {
  const nome = basename(item.caminho);
  if (/maskable/i.test(nome)) return [0, "maskable vai ate a borda"];
  if (/apple|icone-\d+/i.test(nome)) {
    return [0, "gerar-icones-web.py: o iOS arredonda sozinho, e PNG arredondado ganha canto preto"];
  }
  if (item.familia === "mos") return [fontes.raioMos, "RAIO do gerar-icones.py"];
  const px = fontes.receita[tamanho];
  return [px ? px / tamanho : 0.18, "RECIPE do generate-family-icons.py"];
}

function medirBitmap(item, quadro, fontes, extras) {
  const { imagem, tamanho } = quadro;
  const selos = [];
  if (item.familia === "mos") {
    selos.push(seloDaBarra(inclinacaoNoBitmap(imagem, { tinta: TINTA, campo: SODIO }), tamanho, fontes));
  }
  if (item.familia === "mos" || item.familia === "irmao") {
    const [esperado, fonte] = raioEsperado(item, tamanho, fontes);
    selos.push(seloDoRaio(imagem, esperado, fonte));
    const campo = fontes.campos[item.app];
    selos.push(seloDaCor(imagem, fontes, { excecoes: campo ? [campo] : [], icc: extras.icc }));
  }
  return selos;
}

/** "Este tamanho e o maior reduzido?" — so desvia entre faixas diferentes. */
function seloDeEscala(quadro, maior, familia) {
  if (!maior || maior === quadro || quadro.tamanho >= maior.tamanho) return null;
  const reduzido = reduzir(maior.imagem, quadro.tamanho);
  const d = diferenca(reduzido, quadro.imagem);
  const mesmaFaixa = familia === "mos" && faixaDe(quadro.tamanho) === faixaDe(maior.tamanho);
  if (d < 0.012 && !mesmaFaixa) {
    return selo("desvio", "desenho proprio", `igual ao ${maior.tamanho}px reduzido (dif. ${(d * 100).toFixed(1)}%)`, "desenho do tamanho");
  }
  return selo("ok", "desenho proprio", `dif. ${(d * 100).toFixed(1)}% do ${maior.tamanho}px reduzido`);
}

function dataUri(imagem) {
  return `data:image/png;base64,${escreverPng(imagem).toString("base64")}`;
}

function quadrosDoArquivo(item) {
  const buf = readFileSync(join(RAIZ, item.caminho));
  const ext = item.caminho.split(".").pop().toLowerCase();
  if (ext === "png") return [{ rotulo: basename(item.caminho), imagem: lerPng(buf), icc: buf.includes("iCCP") }];
  if (ext === "rgba") {
    const lado = Number(item.caminho.match(/(\d+)\.rgba$/)[1]);
    return [{ rotulo: basename(item.caminho), imagem: lerRgba(buf, lado) }];
  }
  if (ext === "ico") {
    return lerIco(buf).map((q) => ({ rotulo: `${basename(item.caminho)} · ${q.tamanho}`, imagem: q.imagem, motivo: q.motivo }));
  }
  if (ext === "icns") {
    return lerIcns(buf).map((q) => ({ rotulo: `${basename(item.caminho)} · ${q.tipo}`, imagem: q.imagem, motivo: q.motivo }));
  }
  return [];
}

function medirSvg(item, texto, fontes) {
  const selos = [];
  const hexes = [...new Set([...texto.matchAll(/(?:fill|stroke)="(#[0-9A-Fa-f]{3,6})"/g)].map(([, h]) => h.toUpperCase()))];
  const campo = fontes.campos[item.app];
  const fora = hexes.filter((h) => !fontes.paleta.includes(h));
  if (item.familia === "mos" || item.familia === "irmao") {
    if (!fora.length) selos.push(selo("ok", "cor", hexes.join(" ")));
    else if (fora.every((h) => h === campo?.toUpperCase())) {
      selos.push(selo("excecao", "cor", fora.join(" "), "paleta", "campo declarado em generate-family-icons.py"));
    } else selos.push(selo("desvio", "cor", fora.join(" "), "paleta e sodio"));

    const rect = texto.match(/<rect[^>]*width="([\d.]+)"[^>]*?(?:rx="([\d.]+)")?[^>]*\/>/);
    if (rect) {
      const lado = Number(rect[1]);
      const rx = Number(texto.match(/<rect[^>]*rx="([\d.]+)"/)?.[1] ?? 0);
      const [esperado, fonte] = raioEsperado(item, lado, fontes);
      const medido = rx / lado;
      const ok = Math.abs(medido - esperado) <= 0.015;
      selos.push(selo(ok ? "ok" : "desvio", "raio", `${(medido * 100).toFixed(1)}%`, `${(esperado * 100).toFixed(0)}%`, ok ? "" : fonte));
    }
  }
  const poligono = texto.match(/<polygon points="([^"]+)"/)?.[1];
  if (poligono && item.familia === "mos") {
    const pontos = poligono.trim().split(/\s+/).map((p) => p.split(",").map(Number));
    const lean = inclinacaoDoPoligono(pontos);
    const faixa = Object.entries(fontes.barras).find(([, p]) => Math.abs(inclinacaoDoPoligono(p) - lean) < 1.5)?.[0];
    selos.push(
      selo(
        "desvio",
        "barra",
        `${lean.toFixed(1)}° (${faixa ?? "fora das tres"})`,
        "um desenho por faixa",
        "SVG unico: o navegador desenha a mesma barra em 16px na aba e em 512px na instalacao",
      ),
    );
  }
  if (item.familia === "glifo") selos.push(selo("nao-medivel", "fonte", "", "", "glifo de origem, entra no icone pelo generate-family-icons.py"));
  return selos;
}

function svgDataUri(texto) {
  return `data:image/svg+xml;base64,${Buffer.from(texto).toString("base64")}`;
}

function medirArquivos(fontes) {
  const itens = [];
  for (const item of inventario()) {
    const { usos, orfao } = usosDe(item.caminho, item.familia);
    const registro = { ...item, usos, orfao, quadros: [] };
    if (item.caminho.endsWith(".svg")) {
      const texto = readFileSync(join(RAIZ, item.caminho), "utf8");
      registro.quadros.push({ rotulo: basename(item.caminho), tamanho: null, src: svgDataUri(texto), vetor: true, selos: medirSvg(item, texto, fontes) });
    } else {
      const quadros = quadrosDoArquivo(item).map((q) => ({ ...q, tamanho: q.imagem?.largura ?? null }));
      itens.push(registro);
      registro._quadros = quadros;
      for (const q of quadros) {
        if (!q.imagem) {
          registro.quadros.push({ rotulo: q.rotulo, tamanho: null, selos: [selo("nao-medivel", "leitura", "", "", q.motivo)] });
          continue;
        }
        registro.quadros.push({
          rotulo: q.rotulo,
          tamanho: q.tamanho,
          src: dataUri(q.imagem),
          imagem: q.imagem,
          selos: medirBitmap(item, q, fontes, { icc: q.icc }),
        });
      }
      continue;
    }
    itens.push(registro);
  }

  // Desenho proprio: cada quadro contra o MAIOR bitmap do mesmo app.
  const porApp = new Map();
  for (const item of itens) {
    if (item.familia !== "mos" && item.familia !== "irmao") continue;
    for (const q of item.quadros) {
      if (!q.imagem || /maskable|Square|Store/i.test(item.caminho)) continue;
      const chave = item.grupo;
      const atual = porApp.get(chave);
      if (!atual || q.tamanho > atual.tamanho) porApp.set(chave, q);
    }
  }
  for (const item of itens) {
    for (const q of item.quadros) {
      if (!q.imagem) continue;
      const s = seloDeEscala(q, porApp.get(item.grupo), item.familia);
      if (s) q.selos.push(s);
    }
  }
  for (const item of itens) {
    delete item._quadros;
    if (item.orfao) item.quadros.forEach((q) => q.selos.push(selo("desvio", "uso", "ninguem cita este arquivo", "citado por config, HTML ou script")));
  }
  return itens;
}

// ---------------------------------------------------------------------------
// Componentes React

function usosDeFilled() {
  const usos = [];
  const arquivos = git("ls-files", "apps/desktop/src").split("\n").filter((c) => c.endsWith(".tsx"));
  for (const arquivo of arquivos) {
    const fonte = readFileSync(join(RAIZ, arquivo), "utf8");
    for (const m of fonte.matchAll(/<Icon [^>]*filled=\{([^}]+)\}[^>]*\/>/g)) {
      const antes = fonte.slice(Math.max(0, m.index - 400), m.index);
      const linha = fonte.slice(0, m.index).split("\n").length;
      const navegacao = /aria-current/.test(antes.slice(antes.lastIndexOf("<button")));
      usos.push({ local: `${arquivo.replace("apps/desktop/src/", "")}:${linha}`, condicao: m[1], navegacao });
    }
  }
  return usos;
}

async function medirComponentes(fontes) {
  const c = await renderizarComponentes(RAIZ);
  const icone = (nome, svg, lado, contorno, variante) => {
    const achados = conferirIcone(svg, { lado, contorno });
    const selos = achados.length
      ? achados.map((a) => selo("desvio", a.regra, a.medido, a.esperado))
      : [selo("ok", "grade e traco")];
    return { nome, variante, svg, lado, selos };
  };
  const filled = usosDeFilled();
  return {
    icones: [
      ...c.icones.map((i) => icone(i.nome, i.svg, 20, i.nome !== "home", "contorno 20")),
      ...c.solidos.map((i) => icone(i.nome, i.svg, 20, false, "preenchido 20")),
      ...c.pequenos.map((i) => icone(i.nome, i.svg, 16, true, "contorno 16")),
    ],
    simbolos: c.simbolos.map((s) => {
      const pontos = s.svg.match(/points="([^"]+)"/)[1].split(" ").map((p) => p.split(",").map(Number));
      const lean = inclinacaoDoPoligono(pontos);
      const esperado = inclinacaoDoPoligono(fontes.barras[faixaDe(s.tamanho)]);
      return { ...s, selos: [selo(Math.abs(lean - esperado) < 0.01 ? "ok" : "desvio", "barra", `${lean.toFixed(1)}°`, `${esperado.toFixed(1)}° (${faixaDe(s.tamanho)})`)] };
    }),
    marcas: c.marcas,
    ias: c.ias,
    filled: filled.map((u) => ({
      ...u,
      selo: u.navegacao
        ? selo("ok", "filled", u.condicao, "destino de navegacao")
        : selo("desvio", "filled", u.condicao, "so destino de navegacao", "o botao nao marca aria-current"),
    })),
    css: cssDosIcones(ler("apps/desktop/src/App.css")),
  };
}

// ---------------------------------------------------------------------------
// Regras em conflito: o brief contra o codigo

function conflitos(fontes) {
  const lista = [];
  const leans = Object.fromEntries(Object.entries(fontes.barras).map(([f, p]) => [f, inclinacaoDoPoligono(p)]));
  lista.push({
    regra: "Ângulo da barra",
    brief: Object.entries(fontes.rotulos).map(([f, g]) => `${f} ${g}°`).join(" · "),
    codigo: Object.entries(leans).map(([f, g]) => `${f} ${g.toFixed(1)}° da vertical (${(90 - g).toFixed(1)}° da horizontal)`).join(" · "),
    nota: "Os polígonos são os mesmos nas duas fontes; o rótulo em graus não mede nenhuma reta deles. A ordem confere (o ângulo abre ao encolher), o número não.",
  });
  lista.push({
    regra: "Raio do campo",
    brief: `${(fontes.briefRaio * 100).toFixed(0)}% do lado`,
    codigo: `M/OS ${(fontes.raioMos * 100).toFixed(0)}% (gerar-icones.py, --rail-symbol-radius) · irmãos 18% (RECIPE) · PNG de tela inicial 0% (gerar-icones-web.py)`,
    nota: "O M/OS e os irmãos lado a lado na barra de tarefas têm cantos diferentes.",
  });
  const mapa = { canvas: "canvas", surface: "surface", border: "border", "border forte": "border-strong", texto: "text", secundário: "text-secondary", sistema: "text-system" };
  const diferentes = Object.entries(fontes.briefPaleta)
    .filter(([nome, hex]) => mapa[nome] && fontes.tokensEscuros[mapa[nome]]?.toUpperCase() !== hex)
    .map(([nome, hex]) => `${nome}: brief ${hex}, token ${fontes.tokensEscuros[mapa[nome]]}`);
  lista.push({
    regra: "Paleta",
    brief: Object.entries(fontes.briefPaleta).map(([n, h]) => `${n} ${h}`).join(" · "),
    codigo: diferentes.length ? diferentes.join(" · ") : "igual ao tokens.css",
    nota: diferentes.length ? "O tokens.css é o que roda." : "",
    ok: !diferentes.length,
  });
  lista.push({
    regra: "Traço",
    brief: "1.25 em 20px, 1 em 16px (Icon.tsx)",
    codigo: `--stroke-20 ${fontes.traco[20]}px · --stroke-16 ${fontes.traco[16]}px`,
    nota: "",
    ok: fontes.traco[20] === 1.25 && fontes.traco[16] === 1,
  });
  return lista;
}

// ---------------------------------------------------------------------------

async function main() {
  const fontes = lerFontes();
  const arquivos = medirArquivos(fontes);
  const componentes = await medirComponentes(fontes);
  const html = montarPagina({
    arquivos,
    componentes,
    conflitos: conflitos(fontes),
    tokensCss: fontes.tokens,
    gerado: new Date().toISOString().slice(0, 16).replace("T", " "),
    commit: git("rev-parse", "--short", "HEAD").trim(),
  });
  const saida = join(RAIZ, "spikes/lab-icones.html");
  writeFileSync(saida, html);
  const todos = [...arquivos.flatMap((a) => a.quadros.flatMap((q) => q.selos)), ...componentes.icones.flatMap((i) => i.selos)];
  const conta = (t) => todos.filter((s) => s.tipo === t).length;
  console.log(`spikes/lab-icones.html — ${arquivos.length} arquivos, ${componentes.icones.length} icones de interface`);
  console.log(`ok ${conta("ok")} · desvio ${conta("desvio")} · excecao ${conta("excecao")} · nao medivel ${conta("nao-medivel")}`);
  // `--listar`: os desvios no terminal, para quem nao quer abrir a pagina.
  if (process.argv.includes("--listar")) {
    const linha = (onde, s) => `  ${onde} · ${s.regra}: ${s.medido}${s.esperado ? ` (esperado ${s.esperado})` : ""}${s.nota ? ` — ${s.nota}` : ""}`;
    for (const a of arquivos) for (const q of a.quadros) for (const s of q.selos) if (s.tipo !== "ok") console.log(`[${s.tipo}]` + linha(q.rotulo, s));
    for (const i of componentes.icones) for (const s of i.selos) if (s.tipo === "desvio") console.log("[desvio]" + linha(`${i.nome} ${i.variante}`, s));
    for (const u of componentes.filled) if (u.selo.tipo === "desvio") console.log("[desvio]" + linha(u.local, u.selo));
  }
}

await main();

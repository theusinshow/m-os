// O HTML do Lab. Recebe o que o `gerar.mjs` mediu e devolve uma pagina so.
//
// A pagina usa os tokens reais (o `tokens.css` vai embutido) e o CSS de icone do
// `App.css`, copiado regra a regra. As maquetes de contexto — barra de tarefas,
// bandeja, sidebar, tela do iPhone — sao desenho a mao: elas respondem por
// "como fica ao lado", e nao por pixel do app.

const esc = (s) =>
  String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

const NOMES_DO_SELO = { ok: "ok", desvio: "desvio", excecao: "exceção", "nao-medivel": "não medível" };

function seloHtml(s) {
  const detalhe = [s.medido && `medido ${s.medido}`, s.esperado && `esperado ${s.esperado}`, s.nota]
    .filter(Boolean)
    .join(" · ");
  return `<li class="selo" data-tipo="${s.tipo}"><b>${esc(s.regra)}</b> <span class="selo-tipo">${NOMES_DO_SELO[s.tipo]}</span>${
    detalhe ? `<span class="selo-detalhe">${esc(detalhe)}</span>` : ""
  }</li>`;
}

const temDesvio = (selos) => selos.some((s) => s.tipo === "desvio");

/** Zoom inteiro que deixa o quadro entre ~128 e 384px de lado. */
function zoomPara(tamanho) {
  if (!tamanho || tamanho > 64) return 0;
  return Math.max(2, Math.min(8, Math.floor(384 / tamanho)));
}

function quadroHtml(q) {
  const zoom = zoomPara(q.tamanho);
  const real = q.src
    ? `<img class="real" src="${q.src}" alt="" ${q.tamanho ? `width="${Math.min(q.tamanho, 256)}" height="${Math.min(q.tamanho, 256)}"` : 'width="64" height="64"'}>`
    : `<div class="vazio">sem imagem</div>`;
  const ampliado =
    zoom && q.src
      ? `<div class="ampliado" style="--z:${zoom}px;width:${q.tamanho * zoom}px;height:${q.tamanho * zoom}px"><img src="${q.src}" alt="" width="${q.tamanho * zoom}" height="${q.tamanho * zoom}"></div>`
      : "";
  return `<figure class="quadro${temDesvio(q.selos) ? " tem-desvio" : ""}">
    <div class="pecas">${real}${ampliado}</div>
    <figcaption>${esc(q.rotulo)}${q.tamanho ? ` <span class="px">${q.tamanho}px${zoom ? ` · ${zoom}×` : ""}</span>` : q.vetor ? ' <span class="px">vetor</span>' : ""}</figcaption>
    <ul class="selos">${q.selos.map(seloHtml).join("")}</ul>
  </figure>`;
}

function cartaoHtml(a) {
  const desvio = a.quadros.some((q) => temDesvio(q.selos));
  const usos = a.orfao
    ? '<span class="orfao">órfão — ninguém cita</span>'
    : `<span class="usos">usado por ${a.usos.map((u) => `<code>${esc(u)}</code>`).join(", ")}</span>`;
  return `<article class="cartao${desvio ? " tem-desvio" : ""}">
    <header><code class="caminho">${esc(a.caminho)}</code>${usos}</header>
    <div class="quadros">${a.quadros.map(quadroHtml).join("")}</div>
  </article>`;
}

// ---------------------------------------------------------------------------

function achar(arquivos, caminho) {
  return arquivos.find((a) => a.caminho === caminho);
}

/** O quadro de um tamanho exato; se faltar, o menor maior, marcado como aproximado. */
function quadroDe(arquivos, caminho, tamanho) {
  const a = achar(arquivos, caminho);
  if (!a) return null;
  const vetor = a.quadros.find((q) => q.vetor);
  if (vetor) return { src: vetor.src, nota: "SVG" };
  const exato = a.quadros.find((q) => q.tamanho === tamanho && q.src);
  if (exato) return { src: exato.src, nota: "" };
  const maior = a.quadros.filter((q) => q.src && q.tamanho > tamanho).sort((x, y) => x.tamanho - y.tamanho)[0];
  return maior ? { src: maior.src, nota: `≈ ${maior.tamanho}` } : null;
}

const FAMILIA = [
  ["M/OS", "apps/desktop/src-tauri/icons/icon.ico"],
  ["M/OS de bolso", "apps/mos-web/ui/public/icone.svg"],
  ["CronoCAD", "apps/cronocad/src-tauri/icons/icon.ico"],
  ["M-Finance", "apps/m-finance/app/icon.svg"],
  ["Coded Atlas", "apps/coded-atlas/app/icon.svg"],
];

function familiaHtml(arquivos) {
  const tamanhos = [16, 24, 32, 48, 64, 256];
  const linhas = FAMILIA.map(([nome, caminho]) => {
    const celulas = tamanhos
      .map((t) => {
        const q = quadroDe(arquivos, caminho, t);
        return `<td>${q ? `<img src="${q.src}" width="${t}" height="${t}" alt="">${q.nota ? `<small>${esc(q.nota)}</small>` : ""}` : "—"}</td>`;
      })
      .join("");
    return `<tr><th>${esc(nome)}<small>${esc(caminho.split("/").slice(-1)[0])}</small></th>${celulas}</tr>`;
  }).join("");
  return `<table class="familia"><thead><tr><th></th>${tamanhos.map((t) => `<th>${t}</th>`).join("")}</tr></thead><tbody>${linhas}</tbody></table>`;
}

function barraDeTarefasHtml(arquivos) {
  const icones = FAMILIA.map(([nome, caminho]) => {
    const q = quadroDe(arquivos, caminho, 24);
    return q ? `<span class="tarefa" title="${esc(nome)}"><img src="${q.src}" width="24" height="24" alt=""></span>` : "";
  }).join("");
  return ["escura", "clara"]
    .map((tema) => `<div class="barra-tarefas" data-tema="${tema}"><span class="iniciar"></span>${icones}</div>`)
    .join("");
}

function bandejaHtml(arquivos) {
  const tamanhos = [16, 20, 24, 32];
  return ["escura", "clara"]
    .map(
      (tema) =>
        `<div class="bandeja" data-tema="${tema}">${tamanhos
          .map((t) => {
            const q = quadroDe(arquivos, `apps/desktop/src-tauri/icons/bandeja-${t}.rgba`, t);
            return q ? `<span><img src="${q.src}" width="${t}" height="${t}" alt=""><small>${t}</small></span>` : "";
          })
          .join("")}<span class="relogio">14:32</span></div>`,
    )
    .join("");
}

function iphoneHtml(arquivos) {
  const apps = [
    ["M/OS", "apps/mos-web/ui/public/icone-180.png"],
    ["M-Finance", "apps/m-finance/app/apple-icon.png"],
    ["Coded Atlas", "apps/coded-atlas/app/apple-icon.png"],
  ];
  const celula = (mascara) =>
    apps
      .map(([nome, caminho]) => {
        const q = quadroDe(arquivos, caminho, 180);
        if (!q) return "";
        return `<span class="app-ios"><span class="${mascara ? "mascara" : "cru"}"><img src="${q.src}" width="60" height="60" alt=""></span><small>${esc(nome)}</small></span>`;
      })
      .join("");
  return `<div class="iphone"><div class="tela">${celula(true)}</div><p class="legenda">Com a máscara do iOS (raio contínuo ≈ 22%, fundo preto onde o PNG é transparente)</p></div>
    <div class="iphone cru-linha"><div class="tela">${celula(false)}</div><p class="legenda">O arquivo como ele é</p></div>`;
}

function sidebarHtml(componentes) {
  const nav = ["home", "hermes", "inbox", "projects", "board", "library", "calendar", "meetings", "cronocad", "finance", "academic"];
  const icone = (nome, ativo) => {
    const lista = ativo ? componentes.icones.filter((i) => i.variante === "preenchido 20") : componentes.icones;
    return (lista.find((i) => i.nome === nome) ?? componentes.icones.find((i) => i.nome === nome))?.svg ?? "";
  };
  const simbolo = componentes.simbolos.find((s) => s.tamanho === 16)?.svg ?? "";
  const rail = (classe) => `<div class="rail ${classe}">
      <span class="rail-simbolo">${simbolo}</span>
      ${nav.map((n, i) => `<span class="rail-item${i === 1 ? " ativo" : ""}" title="${n}">${icone(n, i === 1)}</span>`).join("")}
      <span class="rail-espaco"></span>
      <span class="rail-item">${icone("attention", false)}</span>
      <span class="rail-item">${icone("capture", false)}</span>
      <span class="rail-item">${icone("settings", false)}</span>
    </div>`;
  return rail("lab-escuro") + rail("lab-claro");
}

function iconesHtml(componentes) {
  const grupos = ["contorno 20", "preenchido 20", "contorno 16"];
  return grupos
    .map((variante) => {
      const lista = componentes.icones.filter((i) => i.variante === variante);
      return `<h3>${esc(variante)} <small>${lista.length}</small></h3><div class="grade-icones">${lista
        .map(
          (i) => `<figure class="icone${temDesvio(i.selos) ? " tem-desvio" : ""}">
            <div class="pecas"><span class="real-icone">${i.svg}</span><canvas class="raster" data-lado="${i.lado}" data-svg="${esc(i.svg)}" width="${i.lado}" height="${i.lado}" style="width:${i.lado * 8}px;height:${i.lado * 8}px;--z:8px"></canvas></div>
            <figcaption>${esc(i.nome)}</figcaption>
            <ul class="selos">${i.selos.map(seloHtml).join("")}</ul>
          </figure>`,
        )
        .join("")}</div>`;
    })
    .join("");
}

function simbolosHtml(componentes) {
  return `<div class="grade-icones">${componentes.simbolos
    .map(
      (s) => `<figure class="icone${temDesvio(s.selos) ? " tem-desvio" : ""}">
        <div class="pecas"><span class="simbolo" style="--t:${s.tamanho}px">${s.svg}</span></div>
        <figcaption>MosSymbol ${s.tamanho}</figcaption><ul class="selos">${s.selos.map(seloHtml).join("")}</ul></figure>`,
    )
    .join("")}</div>`;
}

function filledHtml(componentes) {
  return `<ul class="lista-filled">${componentes.filled
    .map((u) => `<li class="${u.selo.tipo === "desvio" ? "tem-desvio" : ""}"><code>${esc(u.local)}</code> <code>filled={${esc(u.condicao)}}</code> <ul class="selos">${seloHtml(u.selo)}</ul></li>`)
    .join("")}</ul>`;
}

function terceirosHtml(componentes) {
  const celula = (x) => `<figure class="terceiro"><span>${x.svg}</span><figcaption>${esc(x.nome)}</figcaption></figure>`;
  return `<p class="nota">Galeria sem selo: marcas de terceiros seguem a forma original, e não o brief (AppIcon.tsx, MarcaDeIA.tsx).</p>
    <h3>AppIcon</h3><div class="grade-terceiros">${componentes.marcas.map(celula).join("")}</div>
    <h3>MarcaDeIA</h3><div class="grade-terceiros">${componentes.ias.map(celula).join("")}</div>`;
}

function conflitosHtml(conflitos) {
  return `<table class="conflitos"><thead><tr><th>Regra</th><th>Brief</th><th>Código</th><th></th></tr></thead><tbody>${conflitos
    .map(
      (c) => `<tr class="${c.ok ? "" : "tem-desvio"}"><th>${esc(c.regra)}</th><td>${esc(c.brief)}</td><td>${esc(c.codigo)}</td><td>${
        c.ok ? '<span class="selo" data-tipo="ok"><span class="selo-tipo">confere</span></span>' : `<span class="selo" data-tipo="desvio"><span class="selo-tipo">conflito</span></span> ${esc(c.nota)}`
      }</td></tr>`,
    )
    .join("")}</tbody></table>`;
}

/** As variaveis de cor de um bloco do tokens.css, para forcar um tema num painel. */
function tokensDeCor(tokensCss) {
  const inicioEscuro = tokensCss.indexOf("Cor · DARK");
  const escuro = tokensCss.slice(inicioEscuro, tokensCss.indexOf("}", inicioEscuro));
  const inicioClaro = tokensCss.indexOf("[data-theme='light']");
  const claro = tokensCss.slice(inicioClaro, tokensCss.indexOf("}", inicioClaro));
  const decl = (bloco) => (bloco.match(/--[\w-]+:\s*[^;]+;/g) ?? []).join("");
  return `.lab-escuro{${decl(escuro)};color-scheme:dark}.lab-claro{${decl(claro)};color-scheme:light}`;
}

export function montarPagina({ arquivos, componentes, conflitos, tokensCss, gerado, commit }) {
  const todos = [
    ...arquivos.flatMap((a) => a.quadros.flatMap((q) => q.selos)),
    ...componentes.icones.flatMap((i) => i.selos),
    ...componentes.simbolos.flatMap((s) => s.selos),
    ...componentes.filled.map((u) => u.selo),
  ];
  const conta = (t) => todos.filter((s) => s.tipo === t).length;
  const grupos = [...new Set(arquivos.map((a) => a.grupo))];

  return `<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Lab de ícones</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&family=Schibsted+Grotesk:wght@400;500;700&display=swap" rel="stylesheet">
<style>
${tokensCss}
${tokensDeCor(tokensCss)}
${componentes.css}
${CSS}
</style>
</head>
<body>
<header class="topo">
  <div>
    <h1>Lab de ícones</h1>
    <p class="meta">gerado ${esc(gerado)} · commit ${esc(commit)} · <code>cd apps/desktop && npm run lab</code></p>
  </div>
  <div class="placar">
    <span class="selo" data-tipo="ok"><span class="selo-tipo">ok</span> ${conta("ok")}</span>
    <span class="selo" data-tipo="desvio"><span class="selo-tipo">desvio</span> ${conta("desvio")}</span>
    <span class="selo" data-tipo="excecao"><span class="selo-tipo">exceção</span> ${conta("excecao")}</span>
    <span class="selo" data-tipo="nao-medivel"><span class="selo-tipo">não medível</span> ${conta("nao-medivel")}</span>
  </div>
  <div class="controles">
    <label><input type="checkbox" id="so-desvios"> só desvios</label>
    <button type="button" id="tema">tema claro</button>
  </div>
</header>
<nav class="indice">
  <a href="#conflitos">Regras em conflito</a><a href="#familia">Família</a><a href="#contexto">Em contexto</a><a href="#interface">Ícones de interface</a>
  ${grupos.map((g) => `<a href="#g-${esc(g)}">${esc(g)}</a>`).join("")}<a href="#terceiros">Terceiros</a>
</nav>
<main>
  <section id="conflitos"><h2>Regras em conflito</h2><p class="nota">O brief (docs/BRIEF-SISTEMA-DE-LOGOS.md) contra o que o código faz. Os selos abaixo medem contra o código, que é o que roda.</p>${conflitosHtml(conflitos)}</section>
  <section id="familia"><h2>A família lado a lado</h2><p class="nota">Tamanho real. A pergunta do brief: sem ler, dá para saber que são da mesma casa — e distinguir um do outro a 16px?</p>${familiaHtml(arquivos)}</section>
  <section id="contexto"><h2>Em contexto</h2><p class="nota">Maquetes à mão: respondem por "como fica ao lado", não por pixel do sistema.</p>
    <h3>Barra de tarefas do Windows (24px)</h3>${barraDeTarefasHtml(arquivos)}
    <h3>Bandeja</h3>${bandejaHtml(arquivos)}
    <h3>Sidebar do M/OS</h3><div class="rails">${sidebarHtml(componentes)}</div>
    <h3>Tela inicial do iPhone</h3>${iphoneHtml(arquivos)}
  </section>
  <section id="interface"><h2>Ícones de interface <small>Icon.tsx</small></h2><p class="nota">À esquerda o SVG real no tamanho real; à direita o mesmo rasterizado em ${"20"}px e ampliado 8×, como o pixel cai.</p>
    ${iconesHtml(componentes)}
    <h3>Símbolo <small>Symbol.tsx</small></h3>${simbolosHtml(componentes)}
    <h3>Usos de <code>filled</code></h3><p class="nota">Regra 3 do Icon.tsx: preenchido significa destino ativo na navegação, e nada mais.</p>${filledHtml(componentes)}
  </section>
  ${grupos
    .map((g) => `<section id="g-${esc(g)}"><h2>${esc(g)}</h2>${arquivos.filter((a) => a.grupo === g).map(cartaoHtml).join("")}</section>`)
    .join("")}
  <section id="terceiros"><h2>Marcas de terceiros</h2>${terceirosHtml(componentes)}</section>
</main>
<script>${SCRIPT}</script>
</body>
</html>`;
}

const SCRIPT = `
const raiz = document.documentElement;
try { if (localStorage.getItem("lab-tema") === "light") raiz.dataset.theme = "light"; } catch {}
const botao = document.getElementById("tema");
const rotular = () => { botao.textContent = raiz.dataset.theme === "light" ? "tema escuro" : "tema claro"; };
rotular();
botao.addEventListener("click", () => {
  if (raiz.dataset.theme === "light") delete raiz.dataset.theme; else raiz.dataset.theme = "light";
  try { localStorage.setItem("lab-tema", raiz.dataset.theme || "dark"); } catch {}
  rotular();
  rasterizar();
});
document.getElementById("so-desvios").addEventListener("change", (e) => document.body.classList.toggle("so-desvios", e.target.checked));

// O icone de interface rasterizado no tamanho real, com o traco e a cor
// resolvidos — o SVG solto numa <img> nao recebe o CSS da pagina.
function rasterizar() {
  const cor = getComputedStyle(raiz).getPropertyValue("--text").trim() || "#E7EAEC";
  const tinta = getComputedStyle(raiz).getPropertyValue("--signal-ink").trim() || "#E7C24E";
  for (const canvas of document.querySelectorAll("canvas.raster")) {
    const lado = Number(canvas.dataset.lado);
    let svg = canvas.dataset.svg;
    const cheio = /data-filled|data-icon="home"/.test(svg);
    const traco = lado === 16 ? 1 : 1.25;
    const estilo = cheio
      ? "fill:" + cor + ";stroke:none"
      : "fill:none;stroke:" + cor + ";stroke-width:" + traco + "px;stroke-linecap:butt;stroke-linejoin:miter";
    svg = svg.replace("<svg", '<svg xmlns="http://www.w3.org/2000/svg" style="' + estilo + '"').replace('fill="var(--canvas)"', 'fill="' + getComputedStyle(raiz).getPropertyValue("--canvas").trim() + '"').replace(/currentColor/g, cor);
    if (/data-filled/.test(svg) && /data-icon="cronocad"/.test(svg)) svg = svg.replace("fill:" + cor, "fill:" + tinta);
    const img = new Image();
    img.onload = () => {
      const ctx = canvas.getContext("2d");
      ctx.clearRect(0, 0, lado, lado);
      ctx.drawImage(img, 0, 0, lado, lado);
    };
    img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  }
}
rasterizar();
`;

const CSS = `
* { box-sizing: border-box; }
body { margin: 0; background: var(--canvas); color: var(--text); font: 14px/1.45 "Schibsted Grotesk", system-ui, sans-serif; }
code, .meta, .px, .selo, small, figcaption { font-family: "JetBrains Mono", ui-monospace, monospace; }
h1 { font-size: 20px; margin: 0; font-weight: 700; }
h2 { font-size: 16px; margin: 0 0 8px; font-weight: 700; }
h3 { font-size: 13px; margin: 24px 0 8px; font-weight: 500; color: var(--text-secondary); }
h2 small, h3 small { color: var(--text-system); font-weight: 400; }
.nota, .meta { color: var(--text-secondary); font-size: 12px; margin: 0 0 12px; }
.topo { position: sticky; top: 0; z-index: 2; display: flex; flex-wrap: wrap; gap: 16px; align-items: center; justify-content: space-between; padding: 12px 24px; background: var(--surface); border-bottom: 1px solid var(--border); }
.placar { display: flex; gap: 8px; flex-wrap: wrap; }
.controles { display: flex; gap: 12px; align-items: center; font-size: 12px; }
.controles button { font: inherit; color: var(--text); background: var(--surface-raised); border: 1px solid var(--border-strong); padding: 4px 10px; cursor: pointer; }
.indice { display: flex; flex-wrap: wrap; gap: 4px 16px; padding: 8px 24px; border-bottom: 1px solid var(--border); font-size: 12px; }
.indice a { color: var(--text-secondary); text-decoration: none; }
.indice a:hover { color: var(--text); }
main { padding: 0 24px 64px; max-width: 1400px; }
section { padding-top: 32px; }

.selo { display: inline-flex; flex-wrap: wrap; gap: 4px; align-items: baseline; font-size: 11px; padding: 2px 6px; border-left: 2px solid var(--text-system); background: var(--surface); }
.selo[data-tipo="ok"] { border-color: var(--success); }
.selo[data-tipo="desvio"] { border-color: var(--danger); }
.selo[data-tipo="excecao"] { border-color: var(--signal-fill); }
.selo-tipo { color: var(--text-secondary); }
.selo[data-tipo="desvio"] .selo-tipo { color: var(--danger); }
.selo-detalhe { flex-basis: 100%; color: var(--text-secondary); }
.selos { list-style: none; margin: 6px 0 0; padding: 0; display: grid; gap: 3px; }

.conflitos, .familia { border-collapse: collapse; width: 100%; }
.conflitos th, .conflitos td, .familia th, .familia td { border-bottom: 1px solid var(--border); padding: 8px; text-align: left; vertical-align: top; font-size: 12px; }
.conflitos td:nth-child(2), .conflitos td:nth-child(3) { font-family: "JetBrains Mono", monospace; font-size: 11px; color: var(--text-secondary); }
.familia td { vertical-align: middle; text-align: center; }
.familia th small, .familia td small { display: block; color: var(--text-system); font-size: 10px; }
.familia img { display: inline-block; }

.cartao { border: 1px solid var(--border); background: var(--surface); margin-bottom: 12px; }
.cartao header { display: flex; flex-wrap: wrap; gap: 8px 16px; padding: 8px 12px; border-bottom: 1px solid var(--border); font-size: 11px; }
.caminho { color: var(--text); }
.usos { color: var(--text-system); }
.orfao { color: var(--danger); }
.quadros { display: flex; flex-wrap: wrap; gap: 16px; padding: 12px; }
.quadro { margin: 0; max-width: 420px; }
.pecas { display: flex; gap: 12px; align-items: flex-start; }
.real { display: block; }
.ampliado { position: relative; flex: none; }
.ampliado img, canvas.raster { image-rendering: pixelated; display: block; }
.ampliado::after, .raster-grade::after { content: ""; position: absolute; inset: 0; pointer-events: none;
  background-image: linear-gradient(to right, color-mix(in srgb, var(--text) 14%, transparent) 1px, transparent 1px), linear-gradient(to bottom, color-mix(in srgb, var(--text) 14%, transparent) 1px, transparent 1px);
  background-size: var(--z) var(--z); }
canvas.raster { background-image: linear-gradient(to right, color-mix(in srgb, var(--text) 10%, transparent) 1px, transparent 1px), linear-gradient(to bottom, color-mix(in srgb, var(--text) 10%, transparent) 1px, transparent 1px); background-size: var(--z) var(--z); border: 1px solid var(--border); }
figcaption { font-size: 11px; color: var(--text-secondary); margin-top: 6px; word-break: break-all; }
.px { color: var(--text-system); }
.vazio { width: 64px; height: 64px; display: grid; place-items: center; font-size: 10px; color: var(--text-system); border: 1px dashed var(--border-strong); }

.grade-icones { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 12px; }
.icone { margin: 0; padding: 10px; border: 1px solid var(--border); background: var(--surface); }
.real-icone { color: var(--text); display: inline-flex; }
.simbolo { display: inline-grid; place-items: center; width: calc(var(--t) * 1.75); height: calc(var(--t) * 1.75); background: var(--signal-fill); color: var(--on-signal); border-radius: 21%; }
.simbolo .mos-symbol { width: var(--t); height: var(--t); }
.lista-filled { list-style: none; padding: 0; display: grid; gap: 8px; font-size: 12px; }
.grade-terceiros { display: flex; flex-wrap: wrap; gap: 12px; }
.terceiro { margin: 0; padding: 12px; border: 1px solid var(--border); background: var(--surface); color: var(--text); text-align: center; }

.barra-tarefas { display: flex; gap: 4px; align-items: center; justify-content: center; height: 48px; margin-bottom: 8px; }
.barra-tarefas[data-tema="escura"] { background: #202020; }
.barra-tarefas[data-tema="clara"] { background: #EEEEEE; }
.tarefa, .iniciar { width: 40px; height: 40px; display: grid; place-items: center; border-radius: 4px; }
.tarefa:hover { background: rgba(128,128,128,.18); }
.iniciar::before { content: ""; width: 20px; height: 20px; background: linear-gradient(135deg, #3a8ee6, #1d5fbf); border-radius: 2px; }
.bandeja { display: inline-flex; gap: 14px; align-items: flex-end; padding: 10px 16px; margin-right: 12px; }
.bandeja[data-tema="escura"] { background: #202020; color: #fff; }
.bandeja[data-tema="clara"] { background: #EEEEEE; color: #000; }
.bandeja span { display: grid; justify-items: center; gap: 4px; }
.bandeja small { font-size: 9px; opacity: .6; }
.relogio { font: 12px system-ui; }
.rails { display: flex; gap: 24px; }
.rail { display: flex; flex-direction: column; align-items: center; gap: 4px; width: 56px; padding: 12px 0; height: 560px; background: var(--surface); border-right: 1px solid var(--border); color: var(--text-secondary); }
.rail-simbolo { width: var(--rail-symbol); height: var(--rail-symbol); border-radius: var(--rail-symbol-radius); background: var(--signal-fill); color: var(--on-signal); display: grid; place-items: center; margin-bottom: 12px; }
.rail-simbolo .mos-symbol { width: 16px; height: 16px; }
.rail-item { position: relative; width: 40px; height: 36px; display: grid; place-items: center; }
.rail-item.ativo { color: var(--text); }
.rail-item.ativo::before { content: ""; position: absolute; left: -8px; top: 8px; bottom: 8px; width: 2px; background: var(--signal-fill); }
.rail-espaco { flex: 1; }
.iphone { display: inline-block; margin-right: 24px; vertical-align: top; }
.tela { display: flex; gap: 20px; padding: 24px; background: linear-gradient(160deg, #2c3a4a, #121820); border-radius: 28px; }
.app-ios { display: grid; justify-items: center; gap: 6px; color: #fff; font: 11px -apple-system, system-ui, sans-serif; }
.mascara { width: 60px; height: 60px; border-radius: 13.4px; overflow: hidden; background: #000; display: block; }
.cru { width: 60px; height: 60px; display: block; outline: 1px dashed rgba(255,255,255,.3); }
.legenda { font-size: 11px; color: var(--text-secondary); }

body.so-desvios .quadro:not(.tem-desvio),
body.so-desvios .icone:not(.tem-desvio),
body.so-desvios .cartao:not(.tem-desvio),
body.so-desvios .lista-filled li:not(.tem-desvio),
body.so-desvios .conflitos tr:not(.tem-desvio):not(:first-child) { display: none; }
@media (max-width: 640px) { main, .topo, .indice { padding-left: 16px; padding-right: 16px; } .rails { flex-direction: column; } .rail { height: auto; flex-direction: row; width: 100%; overflow-x: auto; } }
`;

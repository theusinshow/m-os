// As regras do Lab, em funcoes puras.
//
// Nada aqui le arquivo nem sabe de onde a imagem veio: recebe pixels ou SVG e
// devolve numero. O `gerar.mjs` decide o que e esperado para cada item — e o
// esperado sai do codigo (`Symbol.tsx`, os geradores de icone, `tokens.css`),
// nunca de uma constante repetida aqui.

const GRAUS = 180 / Math.PI;

/** A faixa da barra pelo lado do icone, como o `barFor` do `Symbol.tsx`. */
export function faixaDe(tamanho) {
  if (tamanho >= 128) return "large";
  if (tamanho >= 48) return "medium";
  return "small";
}

/**
 * Inclinacao da linha de centro da barra, em graus a partir da vertical.
 *
 * Linha de centro: do meio da aresta de cima ao meio da de baixo. Medir uma
 * aresta so daria o mesmo numero no paralelogramo, mas a linha de centro e a
 * que da para comparar com o bitmap, onde as arestas antialiazam.
 */
export function inclinacaoDoPoligono(pontos) {
  const ys = pontos.map(([, y]) => y);
  const topo = Math.min(...ys);
  const base = Math.max(...ys);
  const meio = (y) => {
    const xs = pontos.filter(([, py]) => py === y).map(([x]) => x);
    return xs.reduce((a, b) => a + b, 0) / xs.length;
  };
  return Math.atan(Math.abs(meio(topo) - meio(base)) / (base - topo)) * GRAUS;
}

function distancia(a, b) {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
}

/**
 * Inclinacao da barra num bitmap, em graus a partir da vertical.
 *
 * Cada pixel pesa o quanto ele e tinta: alfa vezes a posicao da cor no segmento
 * campo → tinta. Por linha, o centro de massa da tinta; nas linhas cheias (as
 * pontas cortadas na horizontal ficam de fora), a reta que passa por esses
 * centros. A inclinacao dela e a da barra, com precisao abaixo do pixel.
 */
export function inclinacaoNoBitmap(imagem, { tinta, campo = null }) {
  const { largura, altura, rgba } = imagem;
  const linhas = [];
  for (let y = 0; y < altura; y++) {
    let massa = 0;
    let soma = 0;
    for (let x = 0; x < largura; x++) {
      const i = (y * largura + x) * 4;
      const alfa = rgba[i + 3] / 255;
      if (alfa === 0) continue;
      const cor = [rgba[i], rgba[i + 1], rgba[i + 2]];
      let quanto;
      if (campo) {
        const total = distancia(campo, tinta);
        quanto = Math.min(1, Math.max(0, 1 - distancia(cor, tinta) / total));
        quanto *= alfa;
      } else {
        quanto = distancia(cor, tinta) < 60 ? alfa : 0;
      }
      massa += quanto;
      soma += (x + 0.5) * quanto;
    }
    if (massa > 0) linhas.push({ y: y + 0.5, massa, centro: soma / massa });
  }
  if (linhas.length < 3) return null;
  const cheia = Math.max(...linhas.map((l) => l.massa));
  const uteis = linhas.filter((l) => l.massa >= cheia * 0.6);
  if (uteis.length < 3) return null;
  let sw = 0;
  let sy = 0;
  let sx = 0;
  for (const l of uteis) {
    sw += l.massa;
    sy += l.y * l.massa;
    sx += l.centro * l.massa;
  }
  const my = sy / sw;
  const mx = sx / sw;
  let num = 0;
  let den = 0;
  for (const l of uteis) {
    num += l.massa * (l.y - my) * (l.centro - mx);
    den += l.massa * (l.y - my) ** 2;
  }
  return Math.atan(Math.abs(num / den)) * GRAUS;
}

function alfaEm(imagem, x, y) {
  // Bilinear sobre os centros de pixel, com a borda de fora contando como vazia.
  const { largura, altura, rgba } = imagem;
  const px = x - 0.5;
  const py = y - 0.5;
  const x0 = Math.floor(px);
  const y0 = Math.floor(py);
  const fx = px - x0;
  const fy = py - y0;
  const em = (cx, cy) =>
    cx < 0 || cy < 0 || cx >= largura || cy >= altura ? 0 : rgba[(cy * largura + cx) * 4 + 3] / 255;
  return (
    em(x0, y0) * (1 - fx) * (1 - fy) +
    em(x0 + 1, y0) * fx * (1 - fy) +
    em(x0, y0 + 1) * (1 - fx) * fy +
    em(x0 + 1, y0 + 1) * fx * fy
  );
}

/**
 * Raio do campo, como fracao do lado.
 *
 * Num canto arredondado de raio r, a diagonal do canto cruza a borda a
 * r·(1 − 1/√2) de cada lado. Mede-se esse cruzamento (alfa = 0.5) nos quatro
 * cantos e se devolve a media. `null` quando o campo nao vai ate a borda —
 * icone com margem transparente, onde "raio" nao quer dizer nada.
 */
export function raioNoBitmap(imagem) {
  const { largura: lado } = imagem;
  if (alfaEm(imagem, lado / 2, 0.5) < 0.5 || alfaEm(imagem, 0.5, lado / 2) < 0.5) return null;
  const fator = 1 - Math.SQRT1_2;
  const cantos = [
    (t) => [t, t],
    (t) => [lado - t, t],
    (t) => [t, lado - t],
    (t) => [lado - t, lado - t],
  ];
  const raios = cantos.map((ponto) => {
    let anterior = alfaEm(imagem, ...ponto(0.001));
    if (anterior >= 0.5) return 0;
    for (let t = 0.05; t <= lado / 2; t += 0.05) {
      const atual = alfaEm(imagem, ...ponto(t));
      if (atual >= 0.5) {
        const cruzamento = t - 0.05 + (0.05 * (0.5 - anterior)) / (atual - anterior);
        return cruzamento / fator;
      }
      anterior = atual;
    }
    return null;
  });
  if (raios.some((r) => r === null)) return null;
  return raios.reduce((a, b) => a + b, 0) / 4 / lado;
}

export function hexParaRgb(hex) {
  const h = hex.replace("#", "");
  const cheio = h.length === 3 ? [...h].map((c) => c + c).join("") : h;
  return [0, 2, 4].map((i) => parseInt(cheio.slice(i, i + 2), 16));
}

export function rgbParaHex([r, g, b]) {
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase()}`;
}

function distanciaAoSegmento(p, a, b) {
  const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  const comprimento = ab[0] ** 2 + ab[1] ** 2 + ab[2] ** 2;
  if (comprimento === 0) return distancia(p, a);
  let t = ((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / comprimento;
  t = Math.min(1, Math.max(0, t));
  return distancia(p, [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t]);
}

/**
 * Pixels opacos cuja cor nao e da paleta nem uma mistura de duas cores dela.
 *
 * A mistura passa porque e o antialias: a borda da barra preta sobre o sodio e
 * um castanho que nenhum token nomeia, e nem deveria.
 */
export function coresForaDaPaleta(imagem, paleta, limiar = 12) {
  const cores = paleta.map(hexParaRgb);
  const pares = [];
  for (let i = 0; i < cores.length; i++) {
    for (let j = i; j < cores.length; j++) pares.push([cores[i], cores[j]]);
  }
  const memoria = new Map();
  const contagem = new Map();
  let total = 0;
  let fora = 0;
  const { rgba } = imagem;
  for (let i = 0; i < rgba.length; i += 4) {
    if (rgba[i + 3] < 230) continue;
    total++;
    const chave = (rgba[i] << 16) | (rgba[i + 1] << 8) | rgba[i + 2];
    let ruim = memoria.get(chave);
    if (ruim === undefined) {
      const p = [rgba[i], rgba[i + 1], rgba[i + 2]];
      ruim = !pares.some(([a, b]) => distanciaAoSegmento(p, a, b) <= limiar);
      memoria.set(chave, ruim);
    }
    if (ruim) {
      fora++;
      contagem.set(chave, (contagem.get(chave) ?? 0) + 1);
    }
  }
  const exemplos = [...contagem.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 4)
    .map(([chave, n]) => [rgbParaHex([chave >> 16, (chave >> 8) & 255, chave & 255]), n]);
  return { total, fora, fracao: total ? fora / total : 0, exemplos };
}

/** Diferenca media (0 a 1) entre duas imagens do mesmo tamanho, com alfa pesando. */
export function diferenca(a, b) {
  let soma = 0;
  for (let i = 0; i < a.rgba.length; i += 4) {
    const aa = a.rgba[i + 3] / 255;
    const ab = b.rgba[i + 3] / 255;
    for (let c = 0; c < 3; c++) soma += Math.abs(a.rgba[i + c] * aa - b.rgba[i + c] * ab) / 255;
    soma += Math.abs(aa - ab);
  }
  return soma / (a.rgba.length);
}

const ATRIBUTOS_NUMERICOS = ["x", "y", "width", "height", "cx", "cy", "r", "rx", "x1", "y1", "x2", "y2"];

/** Os numeros de um `d`, sem as flags de arco (que sao 0 e 1 e nao sao posicao). */
function numerosDoCaminho(d) {
  const numeros = [];
  const partes = d.match(/[a-zA-Z]|-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?/g) ?? [];
  let comando = "";
  let indice = 0;
  for (const parte of partes) {
    if (/[a-zA-Z]/.test(parte)) {
      comando = parte.toLowerCase();
      indice = 0;
      continue;
    }
    const valor = Number(parte);
    if (comando === "a") {
      const posicao = indice % 7;
      if (posicao !== 2 && posicao !== 3 && posicao !== 4) numeros.push(valor);
    } else {
      numeros.push(valor);
    }
    indice++;
  }
  return numeros;
}

const naGrade = (n) => Math.abs(n * 2 - Math.round(n * 2)) < 1e-6;
const round = (n) => Math.round(n * 1000) / 1000;
const ARGUMENTOS = { m: 2, l: 2, h: 1, v: 1, c: 6, s: 4, q: 4, t: 2, a: 7, z: 0 };

/**
 * A coordenada que importa de cada trecho reto de um `d`: o y de uma linha
 * horizontal, o x de uma vertical.
 *
 * E so ai que a grade de .5 vale. Uma diagonal (a lombada inclinada do
 * `library`, os raios do `settings`) nao tem como cair inteira no pixel, e
 * cobra-la seria ruido.
 */
export function linhasRetas(d) {
  const partes = d.match(/[a-zA-Z]|-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?/g) ?? [];
  const valores = [];
  let x = 0;
  let y = 0;
  let inicioX = 0;
  let inicioY = 0;
  let i = 0;
  let comando = "";
  const linha = (nx, ny) => {
    if (Math.abs(ny - y) < 1e-9 && Math.abs(nx - x) > 1e-9) valores.push(y);
    else if (Math.abs(nx - x) < 1e-9 && Math.abs(ny - y) > 1e-9) valores.push(x);
    x = nx;
    y = ny;
  };
  while (i < partes.length) {
    if (/[a-zA-Z]/.test(partes[i])) comando = partes[i++];
    const tipo = comando.toLowerCase();
    const relativo = comando !== comando.toUpperCase();
    if (tipo === "z") {
      linha(inicioX, inicioY);
      continue;
    }
    const n = ARGUMENTOS[tipo];
    if (!n) break;
    const args = partes.slice(i, i + n).map(Number);
    if (args.length < n || args.some(Number.isNaN)) break;
    i += n;
    const dx = relativo ? x : 0;
    const dy = relativo ? y : 0;
    if (tipo === "m") {
      x = args[0] + dx;
      y = args[1] + dy;
      inicioX = x;
      inicioY = y;
      comando = relativo ? "l" : "L";
    } else if (tipo === "l") linha(args[0] + dx, args[1] + dy);
    else if (tipo === "h") linha(args[0] + dx, y);
    else if (tipo === "v") linha(x, args[0] + dy);
    else {
      x = args[n - 2] + dx;
      y = args[n - 1] + dy;
    }
  }
  return valores;
}

/**
 * Confere um icone do `Icon.tsx` ja renderizado.
 *
 * `contorno` liga a regra da grade de .5: ela vale para o traco, que precisa
 * cair sobre o pixel. Silhueta preenchida nao tem traco, e as coordenadas dela
 * em .4 e .9 sao o que faz o preenchido ocupar o mesmo espaco optico do
 * contorno — nao e desvio.
 */
export function conferirIcone(svg, { lado, contorno }) {
  const achados = [];
  const viewBox = svg.match(/viewBox="([^"]+)"/)?.[1];
  if (viewBox !== `0 0 ${lado} ${lado}`) {
    achados.push({ regra: "grade", medido: `viewBox ${viewBox}`, esperado: `0 0 ${lado} ${lado}` });
  }
  if (/stroke-width=/.test(svg)) {
    achados.push({ regra: "traco", medido: "stroke-width no proprio SVG", esperado: "o traco do tokens.css" });
  }
  const numeros = [];
  for (const [, d] of svg.matchAll(/\sd="([^"]+)"/g)) numeros.push(...numerosDoCaminho(d));
  for (const nome of ATRIBUTOS_NUMERICOS) {
    for (const [, v] of svg.matchAll(new RegExp(`\\s${nome}="(-?[\\d.]+)"`, "g"))) numeros.push(Number(v));
  }
  const maior = Math.max(0, ...numeros.map(Math.abs));
  if (maior > lado + 0.01) {
    achados.push({ regra: "grade", medido: `coordenada ate ${maior}`, esperado: `dentro de ${lado}` });
  }
  if (contorno) {
    // Pontos de traco preenchido (`stroke="none"`) nao tem linha para alinhar.
    const tracado = svg.replace(/<g[^>]*stroke="none"[^>]*>[\s\S]*?<\/g>/g, "").replace(/<[^>]*stroke="none"[^>]*>/g, "");
    const fora = new Set();
    for (const [, d] of tracado.matchAll(/\sd="([^"]+)"/g)) {
      for (const valor of linhasRetas(d)) if (!naGrade(valor)) fora.add(round(valor));
    }
    for (const [rect] of tracado.matchAll(/<rect[^>]*>/g)) {
      const v = (nome) => Number(rect.match(new RegExp(`\s${nome}="(-?[\d.]+)"`))?.[1] ?? 0);
      for (const valor of [v("x"), v("y"), v("x") + v("width"), v("y") + v("height")]) if (!naGrade(valor)) fora.add(round(valor));
    }
    if (fora.size) {
      achados.push({
        regra: "grade .5",
        medido: `${fora.size} linha(s) reta(s) fora: ${[...fora].slice(0, 4).join(", ")}`,
        esperado: "linha horizontal ou vertical em multiplo de .5",
      });
    }
  }
  return achados;
}

/** Um selo: ok, desvio, excecao ou nao-medivel. */
export function selo(tipo, regra, medido = "", esperado = "", nota = "") {
  return { tipo, regra, medido, esperado, nota };
}

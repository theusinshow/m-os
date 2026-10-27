// Leitores e escritor de imagem do Lab, sem dependencia.
//
// O Lab precisa olhar pixel por pixel o que o Windows, o macOS e o iPhone vao
// mostrar — e o que eles mostram e o PNG dentro do .ico, nao o SVG de origem.
// Um decodificador proprio, curto, evita puxar `sharp` ou `pngjs` so para uma
// bancada que ninguem instala.
//
// Imagem, em todo o Lab: { largura, altura, rgba: Uint8Array } — RGBA de 8 bits,
// sem pre-multiplicar, linha a linha de cima para baixo.

import { deflateSync, inflateSync } from "node:zlib";

const ASSINATURA_PNG = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

export function ehPng(buf) {
  return buf.length >= 8 && buf.subarray(0, 8).equals(ASSINATURA_PNG);
}

export function lerPng(buf) {
  if (!ehPng(buf)) throw new Error("nao e PNG");
  let pos = 8;
  let ihdr = null;
  let paleta = null;
  let transparencia = null;
  const dados = [];
  while (pos < buf.length) {
    const tamanho = buf.readUInt32BE(pos);
    const tipo = buf.toString("latin1", pos + 4, pos + 8);
    const corpo = buf.subarray(pos + 8, pos + 8 + tamanho);
    if (tipo === "IHDR") {
      ihdr = {
        largura: corpo.readUInt32BE(0),
        altura: corpo.readUInt32BE(4),
        bits: corpo[8],
        cor: corpo[9],
        entrelacado: corpo[12],
      };
    } else if (tipo === "PLTE") paleta = corpo;
    else if (tipo === "tRNS") transparencia = corpo;
    else if (tipo === "IDAT") dados.push(corpo);
    else if (tipo === "IEND") break;
    pos += 12 + tamanho;
  }
  if (!ihdr) throw new Error("PNG sem IHDR");
  const { largura, altura, bits, cor, entrelacado } = ihdr;
  if (bits !== 8) throw new Error(`PNG de ${bits} bits nao suportado`);
  if (entrelacado) throw new Error("PNG entrelacado nao suportado");
  const canais = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[cor];
  if (!canais) throw new Error(`tipo de cor ${cor} nao suportado`);

  const cru = inflateSync(Buffer.concat(dados));
  const passo = largura * canais;
  const linhas = new Uint8Array(passo * altura);
  for (let y = 0; y < altura; y++) {
    const filtro = cru[y * (passo + 1)];
    const origem = y * (passo + 1) + 1;
    const destino = y * passo;
    for (let x = 0; x < passo; x++) {
      const v = cru[origem + x];
      const a = x >= canais ? linhas[destino + x - canais] : 0;
      const b = y > 0 ? linhas[destino - passo + x] : 0;
      const c = x >= canais && y > 0 ? linhas[destino - passo + x - canais] : 0;
      let previsto = 0;
      if (filtro === 1) previsto = a;
      else if (filtro === 2) previsto = b;
      else if (filtro === 3) previsto = (a + b) >> 1;
      else if (filtro === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        previsto = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      linhas[destino + x] = (v + previsto) & 255;
    }
  }

  const rgba = new Uint8Array(largura * altura * 4);
  for (let i = 0; i < largura * altura; i++) {
    const s = i * canais;
    const d = i * 4;
    if (cor === 6) {
      rgba.set(linhas.subarray(s, s + 4), d);
    } else if (cor === 2) {
      rgba[d] = linhas[s];
      rgba[d + 1] = linhas[s + 1];
      rgba[d + 2] = linhas[s + 2];
      rgba[d + 3] = 255;
    } else if (cor === 0) {
      rgba[d] = rgba[d + 1] = rgba[d + 2] = linhas[s];
      rgba[d + 3] = 255;
    } else if (cor === 4) {
      rgba[d] = rgba[d + 1] = rgba[d + 2] = linhas[s];
      rgba[d + 3] = linhas[s + 1];
    } else {
      const indice = linhas[s];
      rgba[d] = paleta[indice * 3];
      rgba[d + 1] = paleta[indice * 3 + 1];
      rgba[d + 2] = paleta[indice * 3 + 2];
      rgba[d + 3] = transparencia && indice < transparencia.length ? transparencia[indice] : 255;
    }
  }
  return { largura, altura, rgba };
}

/** Os quadros de um .ico. BMP de 32 bits e PNG; o resto volta sem imagem. */
export function lerIco(buf) {
  const quantos = buf.readUInt16LE(4);
  const quadros = [];
  for (let i = 0; i < quantos; i++) {
    const e = 6 + i * 16;
    const largura = buf[e] || 256;
    const bpp = buf.readUInt16LE(e + 6);
    const tamanho = buf.readUInt32LE(e + 8);
    const offset = buf.readUInt32LE(e + 12);
    const dados = buf.subarray(offset, offset + tamanho);
    let imagem = null;
    let motivo = null;
    if (ehPng(dados)) imagem = lerPng(dados);
    else if (dados.readUInt16LE(14) === 32) imagem = lerDib32(dados);
    else motivo = `BMP de ${bpp || dados.readUInt16LE(14)} bits`;
    quadros.push({ tamanho: imagem ? imagem.largura : largura, imagem, motivo });
  }
  return quadros;
}

function lerDib32(dados) {
  const cabecalho = dados.readUInt32LE(0);
  const largura = dados.readInt32LE(4);
  const altura = Math.abs(dados.readInt32LE(8)) / 2;
  const rgba = new Uint8Array(largura * altura * 4);
  for (let y = 0; y < altura; y++) {
    // DIB e de baixo para cima.
    const origem = cabecalho + (altura - 1 - y) * largura * 4;
    for (let x = 0; x < largura; x++) {
      const s = origem + x * 4;
      const d = (y * largura + x) * 4;
      rgba[d] = dados[s + 2];
      rgba[d + 1] = dados[s + 1];
      rgba[d + 2] = dados[s];
      rgba[d + 3] = dados[s + 3];
    }
  }
  return { largura, altura, rgba };
}

/** Os blocos de um .icns. So os PNG viram imagem; os formatos antigos, nao. */
export function lerIcns(buf) {
  if (buf.toString("latin1", 0, 4) !== "icns") throw new Error("nao e ICNS");
  const blocos = [];
  let pos = 8;
  while (pos + 8 <= buf.length) {
    const tipo = buf.toString("latin1", pos, pos + 4);
    const tamanho = buf.readUInt32BE(pos + 4);
    if (tamanho < 8) break;
    const dados = buf.subarray(pos + 8, pos + tamanho);
    if (tipo !== "TOC " && tipo !== "icnV" && tipo !== "info") {
      const imagem = ehPng(dados) ? lerPng(dados) : null;
      blocos.push({
        tipo,
        tamanho: imagem ? imagem.largura : null,
        imagem,
        motivo: imagem ? null : `bloco ${tipo} em formato antigo`,
      });
    }
    pos += tamanho;
  }
  return blocos;
}

/** O .rgba da bandeja: pixels crus, quadrado, sem cabecalho. */
export function lerRgba(buf, tamanho) {
  if (buf.length !== tamanho * tamanho * 4) {
    throw new Error(`rgba de ${buf.length} bytes nao fecha ${tamanho}x${tamanho}`);
  }
  return { largura: tamanho, altura: tamanho, rgba: new Uint8Array(buf) };
}

const TABELA_CRC = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = TABELA_CRC[(c ^ byte) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function bloco(tipo, corpo) {
  const cabeca = Buffer.alloc(8);
  cabeca.writeUInt32BE(corpo.length, 0);
  cabeca.write(tipo, 4, "latin1");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([cabeca.subarray(4), corpo])), 0);
  return Buffer.concat([cabeca, corpo, crc]);
}

/** PNG RGBA de 8 bits, sem filtro: para embutir na pagina, e nao para guardar. */
export function escreverPng({ largura, altura, rgba }) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(largura, 0);
  ihdr.writeUInt32BE(altura, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const cru = Buffer.alloc((largura * 4 + 1) * altura);
  for (let y = 0; y < altura; y++) {
    cru[y * (largura * 4 + 1)] = 0;
    Buffer.from(rgba.buffer, rgba.byteOffset + y * largura * 4, largura * 4).copy(
      cru,
      y * (largura * 4 + 1) + 1,
    );
  }
  return Buffer.concat([
    ASSINATURA_PNG,
    bloco("IHDR", ihdr),
    bloco("IDAT", deflateSync(cru)),
    bloco("IEND", Buffer.alloc(0)),
  ]);
}

function pesos(origem, destino) {
  // Para cada pixel de destino, os pixels de origem que ele cobre e quanto.
  const escala = origem / destino;
  const lista = [];
  for (let i = 0; i < destino; i++) {
    const inicio = i * escala;
    const fim = inicio + escala;
    const partes = [];
    for (let s = Math.floor(inicio); s < Math.ceil(fim); s++) {
      const peso = Math.min(fim, s + 1) - Math.max(inicio, s);
      if (peso > 1e-9) partes.push([s, peso / escala]);
    }
    lista.push(partes);
  }
  return lista;
}

/**
 * Reduz por media de area, com alfa pre-multiplicado.
 *
 * Media de area e a definicao do que um bom redutor faz — e e o que o
 * `gerar-icones.py` usa (BOX). E o padrao contra o qual "este tamanho e so o
 * maior reduzido?" se pergunta.
 */
export function reduzir(imagem, lado) {
  const px = pesos(imagem.largura, lado);
  const py = pesos(imagem.altura, lado);
  const rgba = new Uint8Array(lado * lado * 4);
  for (let y = 0; y < lado; y++) {
    for (let x = 0; x < lado; x++) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      for (const [sy, wy] of py[y]) {
        for (const [sx, wx] of px[x]) {
          const s = (sy * imagem.largura + sx) * 4;
          const w = wx * wy;
          const alfa = imagem.rgba[s + 3] / 255;
          r += imagem.rgba[s] * alfa * w;
          g += imagem.rgba[s + 1] * alfa * w;
          b += imagem.rgba[s + 2] * alfa * w;
          a += alfa * w;
        }
      }
      const d = (y * lado + x) * 4;
      if (a > 0) {
        rgba[d] = Math.round(r / a);
        rgba[d + 1] = Math.round(g / a);
        rgba[d + 2] = Math.round(b / a);
      }
      rgba[d + 3] = Math.round(a * 255);
    }
  }
  return { largura: lado, altura: lado, rgba };
}

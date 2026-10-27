import assert from "node:assert/strict";
import { test } from "node:test";

import {
  conferirIcone,
  linhasRetas,
  coresForaDaPaleta,
  diferenca,
  faixaDe,
  inclinacaoDoPoligono,
  inclinacaoNoBitmap,
  raioNoBitmap,
} from "./medidas.mjs";
import { reduzir } from "./imagens.mjs";

const SODIO = [231, 194, 78];
const TINTA = [10, 12, 14];
const BARRAS = {
  large: [[38, 8], [53, 8], [26, 56], [11, 56]],
  small: [[42, 12], [56, 12], [22, 52], [8, 52]],
};

function dentro([px, py], pontos) {
  let dentro = false;
  for (let i = 0, j = pontos.length - 1; i < pontos.length; j = i++) {
    const [xi, yi] = pontos[i];
    const [xj, yj] = pontos[j];
    if (yi > py !== yj > py && px < ((xj - xi) * (py - yi)) / (yj - yi) + xi) dentro = !dentro;
  }
  return dentro;
}

/** Icone do M/OS rasterizado com supersampling: campo de raio `raio`, barra `pontos`. */
function rasterizar(lado, pontos, raio = 0.21) {
  const rgba = new Uint8Array(lado * lado * 4);
  const amostras = 8;
  const r = raio * lado;
  for (let y = 0; y < lado; y++) {
    for (let x = 0; x < lado; x++) {
      let campo = 0;
      let barra = 0;
      for (let sy = 0; sy < amostras; sy++) {
        for (let sx = 0; sx < amostras; sx++) {
          const px = x + (sx + 0.5) / amostras;
          const py = y + (sy + 0.5) / amostras;
          const cx = Math.min(Math.max(px, r), lado - r);
          const cy = Math.min(Math.max(py, r), lado - r);
          if (Math.hypot(px - cx, py - cy) > r) continue;
          campo++;
          if (dentro([(px / lado) * 64, (py / lado) * 64], pontos)) barra++;
        }
      }
      const total = amostras * amostras;
      const i = (y * lado + x) * 4;
      const t = campo ? barra / campo : 0;
      for (let c = 0; c < 3; c++) rgba[i + c] = Math.round(SODIO[c] * (1 - t) + TINTA[c] * t);
      rgba[i + 3] = Math.round((campo / total) * 255);
    }
  }
  return { largura: lado, altura: lado, rgba };
}

test("a faixa segue o barFor do Symbol.tsx", () => {
  assert.equal(faixaDe(256), "large");
  assert.equal(faixaDe(128), "large");
  assert.equal(faixaDe(64), "medium");
  assert.equal(faixaDe(48), "medium");
  assert.equal(faixaDe(32), "small");
});

test("os poligonos dao a inclinacao da linha de centro", () => {
  assert.ok(Math.abs(inclinacaoDoPoligono(BARRAS.large) - 29.36) < 0.05);
  assert.ok(Math.abs(inclinacaoDoPoligono(BARRAS.small) - 40.36) < 0.05);
});

test("o bitmap devolve a inclinacao do desenho, ate a 16px", () => {
  const grande = inclinacaoNoBitmap(rasterizar(64, BARRAS.large), { tinta: TINTA, campo: SODIO });
  assert.ok(Math.abs(grande - 29.36) < 2.5, `64px: ${grande}`);
  const pequeno = inclinacaoNoBitmap(rasterizar(16, BARRAS.small), { tinta: TINTA, campo: SODIO });
  assert.ok(Math.abs(pequeno - 40.36) < 5, `16px: ${pequeno}`);
});

test("o raio do campo sai como fracao do lado", () => {
  const medido = raioNoBitmap(rasterizar(128, BARRAS.large, 0.21));
  assert.ok(Math.abs(medido - 0.21) < 0.01, `raio ${medido}`);
  assert.ok(raioNoBitmap(rasterizar(64, BARRAS.large, 0)) < 0.02);
});

test("a mistura de duas cores da paleta passa, e o vermelho nao", () => {
  const paleta = ["#E7C24E", "#0A0C0E"];
  const misto = { rgba: new Uint8Array([120, 103, 46, 255]) };
  assert.equal(coresForaDaPaleta(misto, paleta).fora, 0);
  const vermelho = { rgba: new Uint8Array([220, 30, 30, 255]) };
  const achado = coresForaDaPaleta(vermelho, paleta);
  assert.equal(achado.fora, 1);
  assert.equal(achado.exemplos[0][0], "#DC1E1E");
});

test("o maior reduzido se parece com ele mesmo, e nao com outra faixa", () => {
  const maior = rasterizar(128, BARRAS.large);
  const reduzido = reduzir(maior, 16);
  assert.ok(diferenca(reduzido, rasterizar(16, BARRAS.large)) < 0.02);
  assert.ok(diferenca(reduzido, rasterizar(16, BARRAS.small)) > 0.02);
});

test("viewBox de 48 num icone de 20 e vertical em .3 sao marcados", () => {
  const home = '<svg viewBox="0 0 48 48"><path d="M23.9 4L8 15"/></svg>';
  assert.ok(conferirIcone(home, { lado: 20, contorno: true }).some((a) => a.regra === "grade"));
  const torto = '<svg viewBox="0 0 20 20"><path d="M3.3 4.5v10"/></svg>';
  assert.equal(conferirIcone(torto, { lado: 20, contorno: true })[0].regra, "grade .5");
  const certo = '<svg viewBox="0 0 20 20"><path d="M3.5 4.5h13a4.5 4.5 0 0 1 2 2"/><path d="m13 4.5 3-.7 1.5 11.7"/></svg>';
  assert.deepEqual(conferirIcone(certo, { lado: 20, contorno: true }), []);
});

test("so a linha reta entra na grade: o y da horizontal, o x da vertical", () => {
  assert.deepEqual(linhasRetas("M3.5 5.5h5l1.5 2h6.5v8.5h-13z"), [5.5, 7.5, 16.5, 16, 3.5]);
});

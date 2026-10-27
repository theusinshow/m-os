import assert from "node:assert/strict";
import { test } from "node:test";

import { escreverPng, lerIco, lerPng, lerRgba, reduzir } from "./imagens.mjs";

function xadrez(lado) {
  const rgba = new Uint8Array(lado * lado * 4);
  for (let i = 0; i < lado * lado; i++) {
    const claro = ((i % lado) + Math.floor(i / lado)) % 2 === 0;
    rgba.set(claro ? [231, 194, 78, 255] : [10, 12, 14, 128], i * 4);
  }
  return { largura: lado, altura: lado, rgba };
}

test("o PNG escrito volta com os mesmos pixels", () => {
  const original = xadrez(5);
  const lido = lerPng(escreverPng(original));
  assert.equal(lido.largura, 5);
  assert.deepEqual([...lido.rgba], [...original.rgba]);
});

test("o ICO devolve cada quadro PNG no seu tamanho", () => {
  const quadros = [xadrez(16), xadrez(32)].map(escreverPng);
  const cabeca = Buffer.alloc(6 + 16 * quadros.length);
  cabeca.writeUInt16LE(1, 2);
  cabeca.writeUInt16LE(quadros.length, 4);
  let offset = cabeca.length;
  quadros.forEach((png, i) => {
    const e = 6 + i * 16;
    cabeca[e] = [16, 32][i];
    cabeca.writeUInt16LE(32, e + 6);
    cabeca.writeUInt32LE(png.length, e + 8);
    cabeca.writeUInt32LE(offset, e + 12);
    offset += png.length;
  });
  const lidos = lerIco(Buffer.concat([cabeca, ...quadros]));
  assert.deepEqual(
    lidos.map((q) => q.tamanho),
    [16, 32],
  );
});

test("o rgba da bandeja exige o tamanho exato", () => {
  assert.equal(lerRgba(Buffer.alloc(16 * 16 * 4), 16).largura, 16);
  assert.throws(() => lerRgba(Buffer.alloc(10), 16));
});

test("reduzir uma cor lisa devolve a mesma cor", () => {
  const rgba = new Uint8Array(4 * 4 * 4);
  for (let i = 0; i < 16; i++) rgba.set([231, 194, 78, 255], i * 4);
  const menor = reduzir({ largura: 4, altura: 4, rgba }, 3);
  assert.deepEqual([...menor.rgba.subarray(0, 4)], [231, 194, 78, 255]);
});

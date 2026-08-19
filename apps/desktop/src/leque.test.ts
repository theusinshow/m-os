import { describe, expect, it } from "vitest";
import {
  PETALAS_DE_FABRICA,
  SLOTS,
  anguloDaPetala,
  posicaoDaPetala,
  resolverPetalas,
} from "./leque";
import type { RadialPin } from "./types";

describe("o padrão de fábrica", () => {
  it("nasce com os cinco slots preenchidos", () => {
    expect(PETALAS_DE_FABRICA).toHaveLength(SLOTS);
    expect(PETALAS_DE_FABRICA.map((p) => p.target)).toEqual([
      "calendario",
      "finance",
      "reunioes",
      "019ffc4f-2936-7152-84b7-672d7bdb5bfc",
      "quick_capture",
    ]);
  });

  it("lista vazia devolve o desenho, e não um leque vazio", () => {
    expect(resolverPetalas([], null)).toEqual(PETALAS_DE_FABRICA);
  });
});

describe("resolverPetalas", () => {
  it("um slot gravado substitui só aquele", () => {
    const pins: RadialPin[] = [
      { workspaceId: null, slot: 1, kind: "acao", target: "attention_create" },
    ];
    const petalas = resolverPetalas(pins, null);
    expect(petalas).toHaveLength(SLOTS);
    expect(petalas[1]).toEqual({ slot: 1, kind: "acao", target: "attention_create" });
    // Os outros quatro continuam sendo o desenho.
    expect(petalas[0]).toEqual(PETALAS_DE_FABRICA[0]);
    expect(petalas[4]).toEqual(PETALAS_DE_FABRICA[4]);
  });

  it("ignora pino de outro Workspace", () => {
    const pins: RadialPin[] = [
      { workspaceId: "outro", slot: 0, kind: "acao", target: "quick_capture" },
    ];
    expect(resolverPetalas(pins, null)).toEqual(PETALAS_DE_FABRICA);
  });

  it("ignora slot fora da faixa que o desenho oferece", () => {
    // O banco aceita 0..11 de propósito; a interface oferece cinco.
    const pins: RadialPin[] = [
      { workspaceId: null, slot: 9, kind: "acao", target: "quick_capture" },
    ];
    expect(resolverPetalas(pins, null)).toEqual(PETALAS_DE_FABRICA);
  });

  it("kind desconhecido cai fora em vez de virar pétala morta", () => {
    const pins: RadialPin[] = [{ workspaceId: null, slot: 2, kind: "widget3", target: "x" }];
    expect(resolverPetalas(pins, null)[2]).toEqual(PETALAS_DE_FABRICA[2]);
  });

  it("alvo em branco cai fora", () => {
    const pins: RadialPin[] = [{ workspaceId: null, slot: 3, kind: "pagina", target: "   " }];
    expect(resolverPetalas(pins, null)[3]).toEqual(PETALAS_DE_FABRICA[3]);
  });
});

describe("a geometria", () => {
  it("os ângulos são simétricos em torno da vertical", () => {
    const angulos = Array.from({ length: SLOTS }, (_, i) => anguloDaPetala(i));
    // -90° é para cima. O arco é simétrico: o primeiro e o último são espelhos.
    expect(angulos[0] + angulos[SLOTS - 1]).toBeCloseTo(-180, 5);
    expect(angulos[2]).toBeCloseTo(-90, 5);
  });

  it("os ângulos são crescentes, da esquerda para a direita", () => {
    for (let i = 1; i < SLOTS; i += 1) {
      expect(anguloDaPetala(i)).toBeGreaterThan(anguloDaPetala(i - 1));
    }
  });

  it("nenhuma pétala chega à horizontal, onde moram o recibo e o toast", () => {
    for (let i = 0; i < SLOTS; i += 1) {
      expect(anguloDaPetala(i)).toBeLessThan(-20);
      expect(anguloDaPetala(i)).toBeGreaterThan(-160);
    }
  });

  it("o ângulo de um slot não depende de quantos estão preenchidos", () => {
    // É a razão de o leque existir: o alvo não pode se mover debaixo da mão.
    const antes = anguloDaPetala(3);
    resolverPetalas([{ workspaceId: null, slot: 0, kind: "acao", target: "quick_capture" }], null);
    expect(anguloDaPetala(3)).toBe(antes);
  });

  it("posicaoDaPetala põe o slot do meio direto acima da âncora", () => {
    const { x, y } = posicaoDaPetala(2, 100);
    expect(x).toBeCloseTo(0, 5);
    expect(y).toBeCloseTo(-100, 5);
  });

  it("as pétalas das pontas caem para os lados opostos", () => {
    expect(posicaoDaPetala(0, 100).x).toBeLessThan(0);
    expect(posicaoDaPetala(SLOTS - 1, 100).x).toBeGreaterThan(0);
  });
});

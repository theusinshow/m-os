import { describe, expect, it } from "vitest";
import { barRects, bulletGeometry, compensatedLength, MIN_DASH, sparkPath, stackSegments } from "./plotGeometry";

describe("compensatedLength", () => {
  it("desconta uma espessura inteira, meia por ponta", () => {
    expect(compensatedLength(100, 6)).toBe(94);
  });

  it("zero continua nao desenhando nada", () => {
    expect(compensatedLength(0, 6)).toBe(0);
    expect(compensatedLength(-5, 6)).toBe(0);
  });

  it("abaixo de uma espessura cai no piso e vira disco", () => {
    expect(compensatedLength(3, 6)).toBe(MIN_DASH);
    expect(compensatedLength(6, 6)).toBe(MIN_DASH);
  });
});

describe("barRects", () => {
  it("divide a largura em barras iguais com os vaos entre elas", () => {
    const rects = barRects([1, 1, 1, 1, 1, 1, 1], { width: 140, height: 60, gap: 4 });
    expect(rects).toHaveLength(7);
    expect(rects[0].x).toBe(0);
    expect(rects[0].width).toBeCloseTo(16.571, 3);
    // A ultima barra encosta exatamente na borda direita.
    expect(rects[6].x + rects[6].width).toBeCloseTo(140, 6);
  });

  it("assenta a barra na linha de base", () => {
    const [rect] = barRects([0.5], { width: 20, height: 60, gap: 4 });
    expect(rect.height).toBe(30);
    expect(rect.y).toBe(30);
  });

  it("zero devolve altura zero, para o chamador nao desenhar", () => {
    const [rect] = barRects([0], { width: 20, height: 60, gap: 4 });
    expect(rect.height).toBe(0);
  });

  it("nao inventa altura minima: o rx da conta de arredondar barra baixa", () => {
    const [rect] = barRects([0.01], { width: 20, height: 60, gap: 4 });
    expect(rect.height).toBeCloseTo(0.6, 6);
  });
});

describe("stackSegments", () => {
  it("reparte a largura na proporcao dos valores, descontando os vaos", () => {
    const segments = stackSegments([3, 1], { width: 100, gap: 4 });
    expect(segments).toHaveLength(2);
    expect(segments[0].x).toBe(0);
    expect(segments[0].width).toBeCloseTo(72, 6);
    expect(segments[1].x).toBeCloseTo(76, 6);
    expect(segments[1].width).toBeCloseTo(24, 6);
  });

  it("pula os zeros e nao gasta vao com eles", () => {
    const segments = stackSegments([1, 0, 1], { width: 100, gap: 4 });
    expect(segments.map((segment) => segment.index)).toEqual([0, 2]);
    expect(segments[0].width).toBeCloseTo(48, 6);
  });

  it("sem total devolve vazio", () => {
    expect(stackSegments([0, 0], { width: 100, gap: 4 })).toEqual([]);
  });
});

describe("bulletGeometry", () => {
  it("abaixo da meta, a marca fica no fim e a barra e proporcional", () => {
    const geometry = bulletGeometry(30, 40, 100);
    expect(geometry.fill).toBeCloseTo(75, 6);
    expect(geometry.mark).toBeCloseTo(100, 6);
    expect(geometry.over).toBe(false);
  });

  it("acima da meta, a barra vai ao fim e a marca recua para dentro", () => {
    const geometry = bulletGeometry(50, 40, 100);
    expect(geometry.fill).toBeCloseTo(100, 6);
    expect(geometry.mark).toBeCloseTo(80, 6);
    expect(geometry.over).toBe(true);
  });

  it("sem meta e sem valor nao desenha nada", () => {
    expect(bulletGeometry(0, 0, 100)).toEqual({ fill: 0, mark: 0, over: false });
  });
});

describe("sparkPath", () => {
  it("desenha do canto inferior esquerdo ao superior direito, respeitando o inset", () => {
    expect(sparkPath([0, 1], { width: 100, height: 20, inset: 2 })).toBe("M2.00 18.00 L98.00 2.00");
  });

  it("com menos de dois pontos nao ha linha", () => {
    expect(sparkPath([1], { width: 100, height: 20, inset: 2 })).toBe("");
  });
});

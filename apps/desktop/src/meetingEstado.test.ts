import { describe, expect, it } from "vitest";
import { proximoPasso, rotuloDoEstado } from "./meetingEstado";

describe("rotuloDoEstado", () => {
  it("diz o estado em uma palavra", () => {
    expect(rotuloDoEstado("recorded")).toBe("gravada");
    expect(rotuloDoEstado("ready")).toBe("pronta");
  });
});

describe("proximoPasso", () => {
  it("gravada diz o que falta, porque foi essa palavra que enganou", () => {
    // 20/08: a reuniao ficou em `recorded`, a tela disse so "gravada", e a
    // leitura honesta de quem olhou foi "nao gravou nada".
    const passo = proximoPasso("recorded");
    expect(passo).toMatch(/transcrever/i);
  });

  it("transcrita aponta a analise", () => {
    expect(proximoPasso("transcribed")).toMatch(/análise/i);
  });

  it("o que esta em curso nao pede acao nenhuma", () => {
    expect(proximoPasso("transcribing")).toBeNull();
    expect(proximoPasso("analyzing")).toBeNull();
    expect(proximoPasso("recording")).toBeNull();
  });

  it("pronta nao inventa passo seguinte", () => {
    expect(proximoPasso("ready")).toBeNull();
  });

  it("falhou manda tentar de novo, e diz que a gravacao esta salva", () => {
    const passo = proximoPasso("failed");
    expect(passo).toMatch(/segura|salva/i);
  });
})

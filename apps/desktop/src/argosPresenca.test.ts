import { describe, expect, it } from "vitest";
import { corDaPresenca, presencaDe, rotuloDaPresenca } from "./argosPresenca";

describe("presencaDe", () => {
  it("traduz os tres estados da conexao", () => {
    expect(presencaDe("online")).toBe("conectado");
    expect(presencaDe("connecting")).toBe("conectando");
    expect(presencaDe("offline")).toBe("desconectado");
  });

  it("antes da primeira resposta, ninguem esta desconectado ainda", () => {
    // Nascer "desconectado" acenderia o balao de queda em toda abertura do app,
    // antes de o Hermes ter tido chance de responder.
    expect(presencaDe(null)).toBe("conectando");
  });
});

describe("corDaPresenca", () => {
  it("conectado e a cor da marca, desconectado e a cor apagada", () => {
    expect(corDaPresenca("conectado")).toBe("--signal-ink");
    expect(corDaPresenca("desconectado")).toBe("--text-system");
  });

  it("conectando nao mente para nenhum dos dois lados", () => {
    const meio = corDaPresenca("conectando");
    expect(meio).not.toBe(corDaPresenca("conectado"));
    expect(meio).not.toBe(corDaPresenca("desconectado"));
  });
});

describe("rotuloDaPresenca", () => {
  it("o leitor de tela ouve o estado, e nao so a pose", () => {
    expect(rotuloDaPresenca("desconectado")).toMatch(/desconectado/i);
    expect(rotuloDaPresenca("conectado")).toMatch(/conectado/i);
  });
});

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

  it("na carencia de abertura, offline ainda e conectando", () => {
    // O tunel nao sobe junto com a janela: o primeiro `offline` e "ainda nao
    // subiu". Um balao que acende na abertura e se desdiz tres segundos depois
    // e o "pisca e some" que o proprio balao existe para nao ser.
    expect(presencaDe("offline", true)).toBe("conectando");
  });

  it("passada a carencia, offline e queda de verdade", () => {
    expect(presencaDe("offline", false)).toBe("desconectado");
  });

  it("a carencia nunca segura um online", () => {
    // Ela adia MA noticia, e nao boa: quem ja respondeu esta la.
    expect(presencaDe("online", true)).toBe("conectado");
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

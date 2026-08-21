import { describe, expect, it } from "vitest";
import { capacidades, pode, type Capacidade } from "./platform";

describe("capacidades por plataforma", () => {
  it("o que e do desktop nao existe no iPhone", () => {
    // Atalho global e monitoramento de processo nao sao pendencia no iOS: sao
    // impossibilidade da plataforma. Marcar como `true` faria a interface
    // desenhar um botao que nunca funciona.
    expect(pode("globalShortcut", "windows")).toBe(true);
    expect(pode("globalShortcut", "ios")).toBe(false);
    expect(pode("processMonitoring", "ios")).toBe(false);
    expect(pode("systemAudioCapture", "ios")).toBe(false);
  });

  it("o que e do iPhone nao existe no desktop", () => {
    expect(pode("haptics", "ios")).toBe(true);
    expect(pode("haptics", "windows")).toBe(false);
    expect(pode("nativeShare", "windows")).toBe(false);
  });

  it("push remoto e falso nos dois ate existir de verdade", () => {
    // Nao ha plugin oficial de APNs no Tauri 2 e nao ha infraestrutura.
    // Capacidade prometida e ausente e pior que capacidade ausente.
    expect(pode("pushRemote", "ios")).toBe(false);
    expect(pode("pushRemote", "windows")).toBe(false);
  });

  it("as duas plataformas concordam no que o M/OS depende", () => {
    // Estas quatro sustentam Capture, Voice, lembrete e credencial. Se alguma
    // cair para `false` numa plataforma alvo, e uma feature inteira que muda de
    // forma — e o teste tem que doer antes de a tela ficar errada.
    const essenciais: Capacidade[] = [
      "microphone",
      "localNotifications",
      "secureStorage",
    ];
    for (const capacidade of essenciais) {
      expect(pode(capacidade, "windows"), capacidade).toBe(true);
      expect(pode(capacidade, "ios"), capacidade).toBe(true);
    }
  });

  it("plataforma desconhecida nao promete nada de exclusivo", () => {
    // Na duvida, nao prometa. Uma plataforma que ninguem previu nao deve ganhar
    // atalho global nem bandeja por herança acidental.
    const c = capacidades("desconhecida");
    expect(c.globalShortcut).toBe(false);
    expect(c.tray).toBe(false);
    expect(c.processMonitoring).toBe(false);
  });

  it("toda plataforma responde sobre toda capacidade", () => {
    // Uma capacidade nova sem resposta numa plataforma vira `undefined`, que em
    // `if` se comporta como `false` sem ninguem notar. O teste transforma
    // esquecimento em falha.
    const nomes = Object.keys(capacidades("windows")) as Capacidade[];
    for (const plataforma of ["windows", "ios", "macos", "android", "desconhecida"] as const) {
      const c = capacidades(plataforma);
      for (const nome of nomes) {
        expect(typeof c[nome], `${plataforma}.${nome}`).toBe("boolean");
      }
    }
  });
});

import { describe, expect, it } from "vitest";
import { rotuloDoProcessamento, type Processamento } from "./processamento";

const transcrevendo = (canal: "mic" | "system", progress: number): Processamento =>
  ({ tipo: "transcrevendo", meetingId: "m", canal, progress });

describe("o que a barra promete", () => {
  it("a transcrição mostra percentual, porque ele é medido", () => {
    const r = rotuloDoProcessamento(transcrevendo("mic", 0.62));
    expect(r.titulo).toMatch(/transcrevendo/i);
    expect(r.detalhe).toMatch(/você/i);
    expect(r.fracao).toBeCloseTo(0.62);
  });

  it("nomeia o canal, porque são duas passadas e não uma", () => {
    // Esconder o canal faria a barra ir até a metade, parecer travada e recomeçar.
    expect(rotuloDoProcessamento(transcrevendo("system", 0.4)).detalhe).toMatch(/outros|remoto/i);
  });

  it("a análise conta janelas, e nunca inventa percentual", () => {
    const r = rotuloDoProcessamento({ tipo: "analisando", meetingId: "m", window: 2, windows: 5 });
    expect(r.detalhe).toMatch(/2 de 5/);
    // Rede não tem fração: ou voltou, ou não voltou.
    expect(r.fracao).toBeNull();
  });

  it("a janela zero é a que junta, e ela se diz por nome", () => {
    const r = rotuloDoProcessamento({ tipo: "analisando", meetingId: "m", window: 0, windows: 5 });
    expect(r.detalhe).toMatch(/junt/i);
    expect(r.detalhe).not.toMatch(/0 de 5/);
  });

  it("a falha não vira barra: ela vira a mensagem e fica", () => {
    const r = rotuloDoProcessamento({ tipo: "falhou", meetingId: "m", detalhe: "o modelo sumiu" });
    expect(r.fracao).toBeNull();
    expect(r.detalhe).toBe("o modelo sumiu");
    expect(r.erro).toBe(true);
  });
});

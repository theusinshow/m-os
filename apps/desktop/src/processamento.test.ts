import { describe, expect, it } from "vitest";
import { rotuloDoProcessamento, type Processamento } from "./processamento";

const progresso = (stage: "transcription" | "analysis", overall: number, detail: string): Processamento => ({
  tipo: "progresso",
  meetingId: "m",
  evento: { meetingId: "m", stage, progress: 0.5, overall, detail },
});

describe("o que a barra promete", () => {
  it("mostra a fração global medida, e nomeia o canal", () => {
    const r = rotuloDoProcessamento(progresso("transcription", 0.36, "mic"));
    expect(r.titulo).toBe("Organizando reunião");
    expect(r.detalhe).toMatch(/você/);
    expect(r.fracao).toBeCloseTo(0.36);
    expect(rotuloDoProcessamento(progresso("transcription", 0.6, "system")).detalhe).toMatch(/outros/);
  });

  it("a organização diz o que faz, sem janela crua", () => {
    expect(rotuloDoProcessamento(progresso("analysis", 0.8, "2/3")).detalhe).toMatch(/decisões e tarefas/);
    expect(rotuloDoProcessamento(progresso("analysis", 0.95, "juntando")).detalhe).toMatch(/junt/);
  });

  it("pronta conta ações e decisões e oferece revisar", () => {
    const r = rotuloDoProcessamento({ tipo: "pronta", meetingId: "m", titulo: "X", acoes: 3, decisoes: 2 });
    expect(r.titulo).toBe("Reunião pronta");
    expect(r.detalhe).toBe("3 ações · 2 decisões");
    expect(r.acao).toBe("Revisar");
  });

  it("pronta sem ações não inventa tarefa", () => {
    const r = rotuloDoProcessamento({ tipo: "pronta", meetingId: "m", titulo: "Alinhamento", acoes: 0, decisoes: 0 });
    expect(r.detalhe).toBe("Alinhamento");
    expect(r.acao).toBe("Abrir");
  });

  it("falha passageira não é erro: é espera", () => {
    const r = rotuloDoProcessamento({ tipo: "aguardando", meetingId: "m" });
    expect(r.erro).toBe(false);
    expect(r.detalhe).toMatch(/sozinho/);
  });

  it("a falha que precisa da pessoa vira a mensagem e fica", () => {
    const r = rotuloDoProcessamento({ tipo: "falhou", meetingId: "m", detalhe: "o transcritor sumiu" });
    expect(r.erro).toBe(true);
    expect(r.detalhe).toBe("o transcritor sumiu");
  });
});

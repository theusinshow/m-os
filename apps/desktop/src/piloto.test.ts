import { describe, expect, it } from "vitest";
import {
  fraseDeErroDeSync,
  fraseDeHoje,
  linhasDeAusencia,
  minutosCurto,
  proximaTentativa,
  rotuloDaAcao,
  seloDeSync,
} from "./piloto";
import type { SaudeDoSync } from "./types";

describe("seloDeSync", () => {
  it("desligado sai mudo", () => {
    expect(seloDeSync(null, null).tom).toBe("mudo");
    expect(seloDeSync({ kind: "desligado" }, "2026-09-15T10:00:00Z").tom).toBe("mudo");
  });

  it("girando ganha de tudo e conta as alterações", () => {
    const selo = seloDeSync({ kind: "sincronizando", pendentes: 3 }, null);
    expect(selo.tom).toBe("girando");
    expect(selo.texto).toBe("Sincronizando 3 alterações...");
  });

  it("erro e offline têm tons próprios, e offline não é erro", () => {
    expect(seloDeSync({ kind: "erro", pendentes: 1, tipo: "credencial", mensagem: "401" }, null).tom).toBe("erro");
    const off = seloDeSync({ kind: "offline", pendentes: 2, proximaTentativaEm: null }, null);
    expect(off.tom).toBe("aviso");
    expect(off.texto).toBe("Não sincronizado");
  });

  it("em dia diz quando", () => {
    const selo = seloDeSync({ kind: "em_dia" }, new Date(Date.now() - 30_000).toISOString());
    expect(selo.icone).toBe("✓");
    expect(selo.texto).toBe("Sincronizado agora");
  });
});

describe("fraseDeErroDeSync", () => {
  const base: SaudeDoSync = {
    estado: { kind: "em_dia" },
    registro: { ultimoOkEm: null, ultimaRodadaEm: null, ultimoErro: "x", tipoDoErro: null, falhasSeguidas: 1, proximaTentativaEm: null },
    ligado: true, rodando: false, pendentes: 0, emRetry: 0, conflitosAbertos: 0, dispositivos: [], deviceId: "", appVersion: "",
  };
  it("diz que os dados estão seguros e se vai tentar de novo", () => {
    const off = fraseDeErroDeSync({ ...base, registro: { ...base.registro, tipoDoErro: "offline" } });
    expect(off).toContain("salvas neste dispositivo");
    expect(off).toContain("automaticamente");
    const cred = fraseDeErroDeSync({ ...base, registro: { ...base.registro, tipoDoErro: "credencial" } });
    expect(cred).toContain("Ajustes");
  });
});

describe("proximaTentativa", () => {
  it("formata segundos, minutos e o já passou", () => {
    const agora = Date.parse("2026-09-15T10:00:00Z");
    expect(proximaTentativa({ proximaTentativaEm: "2026-09-15T10:00:30Z" }, agora)).toBe("em 30 s");
    expect(proximaTentativa({ proximaTentativaEm: "2026-09-15T10:05:00Z" }, agora)).toBe("em 5 min");
    expect(proximaTentativa({ proximaTentativaEm: "2026-09-15T09:00:00Z" }, agora)).toBe("agora");
    expect(proximaTentativa({ proximaTentativaEm: null }, agora)).toBe("");
  });
});

describe("frases da Home", () => {
  it("concorda em número", () => {
    expect(fraseDeHoje({ concluidas: 1, restantes: 4, progresso: 20 })).toEqual({ feitas: "1 concluída", restantes: "4 restantes" });
    expect(fraseDeHoje({ concluidas: 3, restantes: 1, progresso: 75 }).restantes).toBe("1 restante");
  });
  it("rotula as ações", () => {
    expect(rotuloDaAcao({ acao: "cobrar", id: "1", quem: "Victor" })).toBe("Cobrar Victor");
    expect(rotuloDaAcao({ acao: "nenhuma" })).toBe("");
  });
  it("conta a ausência só no que não é zero", () => {
    expect(linhasDeAusencia({ tarefasAtrasadas: 7, captures: 12, waitingFor: 0, deadlinesProximos: 2, lembretesVencidos: 0 })).toEqual([
      "7 tarefas atrasadas", "12 Captures", "2 deadlines próximos",
    ]);
  });
  it("minutos curtos", () => {
    expect(minutosCurto(18)).toBe("18 min");
    expect(minutosCurto(65)).toBe("1h05");
    expect(minutosCurto(120)).toBe("2h");
  });
});

import { describe, expect, it } from "vitest";
import {
  aguardandoOutros, copyDoGuardian, duracao, exigeAtencao, filtrarTranscricao, instanteDoInput,
  limitarCorte, linhaDeContagem, passosVisiveis, pedacosDoTrecho, relogio, rotuloDePrazo,
  rotuloDoLote, secaoDa, seloDoCartao,
} from "./reuniao";
import type { MeetingInsight, MeetingOverview, TranscriptSegment } from "./types";

function item(parcial: Partial<MeetingInsight>): MeetingInsight {
  return {
    id: "i", meetingId: "m", kind: "my_action", seq: 0, text: "Enviar bases", owner: null,
    dueHint: null, confidence: "high", status: "proposed", createdTaskId: null,
    createdReminderId: null, evidence: [{ segmentId: "s1", seq: 0, charStart: null, charEnd: null }],
    origin: "spoken", dueAt: null, dueConfidence: null, ...parcial,
  };
}

function linha(parcial: Partial<MeetingOverview>): MeetingOverview {
  return {
    meeting: {} as MeetingOverview["meeting"], phase: "ready",
    progress: { steps: [], fraction: 1 }, job: null, pendingActions: 0, tasksCreated: 0,
    decisions: 0, waiting: 0, questions: 0, attention: "", ...parcial,
  };
}

function trecho(parcial: Partial<TranscriptSegment>): TranscriptSegment {
  return {
    id: "s1", meetingId: "m", seq: 0, startMs: 0, endMs: 2000, channel: "mic",
    text: "Precisamos enviar para criciuma sexta", speaker: null, confidence: null,
    textNormalized: null, corrections: [], ...parcial,
  };
}

describe("tempo", () => {
  it("relógio e duração falam como gente", () => {
    expect(relogio(32 * 60_000 + 14_000)).toBe("32:14");
    expect(relogio(3_729_000)).toBe("1:02:09");
    expect(duracao(48 * 60_000)).toBe("48 min");
    expect(duracao(72 * 60_000)).toBe("1h12");
    expect(duracao(20_000)).toBe("menos de 1 min");
  });
});

describe("a lista", () => {
  it("separa em andamento, atenção e recentes", () => {
    expect(secaoDa(linha({ phase: "processing" }))).toBe("em_andamento");
    expect(secaoDa(linha({ phase: "needs_attention" }))).toBe("atencao");
    expect(secaoDa(linha({ phase: "ready", pendingActions: 2 }))).toBe("atencao");
    expect(secaoDa(linha({ phase: "ready" }))).toBe("recentes");
  });

  it("conta o que importa e não inventa zero", () => {
    expect(linhaDeContagem(linha({ tasksCreated: 3, decisions: 2, waiting: 1 })))
      .toBe("3 Tasks · 2 decisões · 1 aguardando");
    expect(linhaDeContagem(linha({ pendingActions: 1 }))).toBe("1 ação para revisar");
    expect(linhaDeContagem(linha({}))).toBe("");
  });

  it("o selo mostra progresso medido", () => {
    expect(seloDoCartao(linha({ phase: "processing", progress: { steps: [], fraction: 0.72 } })))
      .toBe("Organizando · 72%");
    expect(seloDoCartao(linha({ phase: "processing", progress: { steps: [], fraction: null } })))
      .toBe("Organizando");
    expect(seloDoCartao(linha({ phase: "ready" }))).toBe("✓ Pronta");
  });
});

describe("progresso", () => {
  it("diz o passo feito no passado e o em curso no gerúndio", () => {
    const passos = passosVisiveis({
      steps: [
        { key: "saved", state: "done" },
        { key: "transcription", state: "done" },
        { key: "analysis", state: "waiting" },
      ],
      fraction: 0.7,
    });
    expect(passos.map((p) => p.texto)).toEqual([
      "Gravação salva",
      "Transcrição concluída",
      "Identificando decisões e tarefas — vai tentar de novo",
    ]);
  });
});

describe("a revisão em lote", () => {
  it("alta vem marcada, média pede revisão, baixa não tem caixa", () => {
    const linhas = exigeAtencao([
      item({ id: "a", confidence: "high" }),
      item({ id: "b", confidence: "medium" }),
      item({ id: "c", confidence: "low" }),
    ]);
    expect(linhas.map((l) => [l.item.id, l.selecionavel, l.marcadaDeInicio, l.revisar])).toEqual([
      ["a", true, true, false],
      ["b", true, false, true],
      ["c", false, false, false],
    ]);
  });

  it("dito sem evidência não entra; escrito entra", () => {
    const [semProva, escrito] = exigeAtencao([
      item({ id: "a", evidence: [] }),
      item({ id: "b", evidence: [], origin: "written" }),
    ]);
    expect(semProva.selecionavel).toBe(false);
    expect(escrito.selecionavel).toBe(true);
  });

  it("minhas e de outros ficam separadas, e resolvido some", () => {
    const itens = [
      item({ id: "a", kind: "my_action" }),
      item({ id: "b", kind: "commitment", owner: "Victor" }),
      item({ id: "c", kind: "other_action" }),
      item({ id: "d", kind: "my_action", status: "accepted" }),
    ];
    expect(exigeAtencao(itens).map((l) => l.item.id)).toEqual(["a"]);
    expect(aguardandoOutros(itens).map((l) => l.item.id)).toEqual(["b", "c"]);
  });

  it("o botão diz quantas", () => {
    expect(rotuloDoLote(2)).toBe("Criar 2 tarefas");
    expect(rotuloDoLote(1)).toBe("Criar 1 tarefa");
    expect(rotuloDoLote(0)).toBe("Nada marcado");
  });
});

describe("prazos", () => {
  const quarta = new Date(2026, 8, 16, 14, 32);
  it("fala o dia até uma semana, depois a data", () => {
    expect(rotuloDePrazo(null, quarta)).toBe("sem prazo");
    expect(rotuloDePrazo(new Date(2026, 8, 16, 18).toISOString(), quarta)).toBe("hoje");
    expect(rotuloDePrazo(new Date(2026, 8, 17, 18).toISOString(), quarta)).toBe("amanhã");
    expect(rotuloDePrazo(new Date(2026, 8, 18, 18).toISOString(), quarta)).toBe("sexta");
    expect(rotuloDePrazo(new Date(2026, 8, 30, 18).toISOString(), quarta)).toBe("30/09");
  });

  it("a data do campo vence às 18h", () => {
    const instante = instanteDoInput("2026-09-18");
    expect(instante?.getHours()).toBe(18);
    expect(instanteDoInput("")).toBeNull();
  });
});

describe("o Guardian na tela", () => {
  it("parado sem excesso só pergunta", () => {
    const copy = copyDoGuardian({ kind: "ask", prompt: "ended", trigger: "app_released_mic", suggestedEndMs: 100_000, excessMs: 90_000 }, 200_000);
    expect(copy?.titulo).toBe("Parece que sua reunião terminou.");
    expect(copy?.encerrar).toBe("Encerrar gravação");
    expect(copy?.cortarEm).toBeNull();
  });

  it("esquecida há meia hora já oferece ignorar o fim", () => {
    const copy = copyDoGuardian({ kind: "ask", prompt: "ended", trigger: "app_closed", suggestedEndMs: 40 * 60_000, excessMs: 31 * 60_000 }, 71 * 60_000);
    expect(copy?.encerrar).toBe("Encerrar e ignorar os últimos 31 min");
    expect(copy?.cortarEm).toBe(40 * 60_000);
  });

  it("contagem regressiva e reunião longa", () => {
    expect(copyDoGuardian({ kind: "countdown", secondsLeft: 18, trigger: "", suggestedEndMs: null, excessMs: 0 }, 0)?.corpo)
      .toBe("A gravação será encerrada em 18 s.");
    const longa = copyDoGuardian({ kind: "ask", prompt: "long", trigger: "long_meeting", suggestedEndMs: null, excessMs: 0 }, 157 * 60_000);
    expect(longa?.titulo).toBe("Esta reunião está sendo gravada há 2h37.");
    expect(longa?.continuar).toBe("Sim, continuar");
    expect(copyDoGuardian({ kind: "idle" }, 0)).toBeNull();
  });
});

describe("a transcrição", () => {
  it("filtra por canal, busca sem acento, marcados e itens", () => {
    const trechos = [
      trecho({ id: "s1", channel: "mic", text: "Mando amanhã", startMs: 0, endMs: 2000 }),
      trecho({ id: "s2", channel: "system", text: "Reviso a prancha", startMs: 60_000, endMs: 62_000 }),
    ];
    const base = { busca: "", canal: "todos" as const, soMarcados: false, soComItens: false };
    expect(filtrarTranscricao(trechos, { ...base, canal: "system" }, [], []).map((t) => t.id)).toEqual(["s2"]);
    expect(filtrarTranscricao(trechos, { ...base, busca: "AMANHA" }, [], []).map((t) => t.id)).toEqual(["s1"]);
    expect(filtrarTranscricao(trechos, { ...base, soMarcados: true },
      [{ id: "b", meetingId: "m", atMs: 61_000, note: "", createdAt: "" }], []).map((t) => t.id)).toEqual(["s2"]);
    expect(filtrarTranscricao(trechos, { ...base, soComItens: true }, [], [item({})]).map((t) => t.id)).toEqual(["s1"]);
  });

  it("grifa a troca do vocabulário mesmo com acento antes dela", () => {
    const normal = "Não mandar para Criciúma sexta";
    const inicio = new TextEncoder().encode("Não mandar para ").length;
    const fim = inicio + new TextEncoder().encode("Criciúma").length;
    const pedacos = pedacosDoTrecho(trecho({
      text: "Não mandar para criciuma sexta",
      textNormalized: normal,
      corrections: [{ original: "criciuma", term: "Criciúma", start: inicio, end: fim, uncertain: true }],
    }));
    expect(pedacos.map((p) => p.texto)).toEqual(["Não mandar para ", "Criciúma", " sexta"]);
    expect(pedacos[1].incerto).toBe(true);
    expect(pedacos[1].original).toBe("criciuma");
  });
});

describe("o corte", () => {
  it("fica dentro da gravação e o fim vem depois do início", () => {
    expect(limitarCorte(-500, 90_400, 60_000)).toEqual([0, 60_000]);
    expect(limitarCorte(30_000, 30_000, 60_000)).toEqual([29_000, 30_000]);
  });
});

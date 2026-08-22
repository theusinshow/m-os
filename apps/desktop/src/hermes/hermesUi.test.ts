import { describe, expect, it } from "vitest";
import { aplicarComando, comandosPara, tokenDeComando } from "./comandos";
import { aplicarMencao, coladoNoFim, LINHAS_MAXIMAS, LINHAS_MINIMAS, medirCampo, tokenDeMencao } from "./composer";
import { decorridoDe, explicaOFim, reciboAlerta, reciboDosPassos } from "./atividade";

describe("comandos de barra", () => {
  it("so abre quando a barra E o rascunho inteiro", () => {
    // Barra no meio de frase e barra: data, caminho, "e/ou". Abrir menu ali
    // interromperia a escrita normal.
    expect(tokenDeComando("/ta")).toBe("ta");
    expect(tokenDeComando("hoje 12/08 as 9h")).toBeNull();
    expect(tokenDeComando("faca /task")).toBeNull();
  });

  it("filtra pelo que ja foi digitado", () => {
    expect(comandosPara("/ca").map((comando) => comando.nome)).toEqual(["capture", "calendar"]);
    expect(comandosPara("/").length).toBeGreaterThan(0);
    expect(comandosPara("nada")).toEqual([]);
  });

  it("expande para prosa, e nunca para sintaxe", () => {
    // O que chega ao gateway e sempre portugues: um segundo idioma seria um
    // segundo caminho para manter, dos dois lados.
    const task = comandosPara("/task")[0];
    expect(aplicarComando(task)).toBe("crie uma task para ");
    expect(aplicarComando(task)).not.toContain("/");
  });
});

describe("altura do campo", () => {
  const LINHA = 20;

  it("nunca nasce menor que o piso", () => {
    expect(medirCampo(0, LINHA).altura).toBe(LINHA * LINHAS_MINIMAS);
    expect(medirCampo(10, LINHA).rolando).toBe(false);
  });

  it("cresce com o texto ate o teto", () => {
    expect(medirCampo(LINHA * 5, LINHA).altura).toBe(LINHA * 5);
    expect(medirCampo(LINHA * 5, LINHA).rolando).toBe(false);
  });

  it("no teto, para de crescer e passa a rolar", () => {
    const medida = medirCampo(LINHA * 40, LINHA);
    expect(medida.altura).toBe(LINHA * LINHAS_MAXIMAS);
    expect(medida.rolando).toBe(true);
  });

  it("soma a moldura ao piso e ao teto", () => {
    // Sem isto, o padding do campo comia duas linhas do texto no teto.
    expect(medirCampo(0, LINHA, 24).altura).toBe(LINHA * LINHAS_MINIMAS + 24);
  });
});

describe("colado no fim", () => {
  it("quem esta no fim continua sendo levado junto", () => {
    expect(coladoNoFim(1000, 900, 100)).toBe(true);
  });

  it("quem subiu para ler nao e puxado de volta", () => {
    expect(coladoNoFim(4000, 200, 800)).toBe(false);
  });

  it("a folga absorve o crescimento do proprio campo", () => {
    // Alguns pixels de diferenca nao sao rolagem de ninguem.
    expect(coladoNoFim(1000, 880, 100, 96)).toBe(true);
  });
});

describe("mencoes", () => {
  it("vale em qualquer posicao da frase", () => {
    expect(tokenDeMencao("fale do @jabo")).toBe("jabo");
    expect(aplicarMencao("fale do @jabo", "JABOTICATUBA")).toBe("fale do @JABOTICATUBA ");
  });

  it("espera dois caracteres antes de buscar", () => {
    // Com um so, toda mencao devolveria o acervo inteiro.
    expect(tokenDeMencao("@j")).toBeNull();
    expect(tokenDeMencao("@ja")).toBe("ja");
  });
});

describe("recibo da atividade", () => {
  it("fala em fontes, e nao em ferramentas", () => {
    // "3 fontes consultadas" diz o que o usuario ganhou; "3 tool calls" diz
    // como o backend trabalhou.
    expect(reciboDosPassos([{ name: "a", state: "success" }, { name: "b", state: "success" }]))
      .toBe("2 fontes consultadas");
  });

  it("a falha sobe para a linha colapsada", () => {
    const passos = [{ name: "a", state: "success" as const }, { name: "b", state: "error" as const }];
    expect(reciboDosPassos(passos)).toBe("2 fontes · 1 falhou");
    expect(reciboAlerta(passos)).toBe(true);
  });

  it("sem passo nenhum nao ha recibo", () => {
    expect(reciboDosPassos([])).toBe("");
    expect(reciboAlerta([])).toBe(false);
  });

  it("engole duracao curta demais para informar", () => {
    expect(decorridoDe(1000, 1200)).toBe("");
    expect(decorridoDe(1000, 3400)).toBe("2.4s");
    expect(decorridoDe(0, 9999)).toBe("");
  });
});

describe("a linha que explica o fim do turno", () => {
  const parte = (kind: string) => ({ body: { kind } });

  it("reconhece a linha gravada pelo settle_turn", () => {
    // "A conexão caiu." é gravada como parte de status, no fim. O componente
    // não pode empilhar um segundo "Interrompido." embaixo dela.
    expect(explicaOFim({ parts: [parte("text"), parte("status")] })).toBe(true);
  });

  it("uma resposta que termina em texto não explica nada", () => {
    expect(explicaOFim({ parts: [parte("status"), parte("text")] })).toBe(false);
  });

  it("mensagem antiga, sem parte nenhuma, cai no fallback", () => {
    // As mensagens gravadas antes de 2026-08-22 não têm a linha do motivo, e
    // precisam do "Interrompido." genérico do componente.
    expect(explicaOFim({ parts: [] })).toBe(false);
  });
});

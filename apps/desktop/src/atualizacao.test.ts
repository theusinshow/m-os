import { describe, expect, it } from "vitest";
import {
  deveConferirSozinho,
  linhaDaVerificacao,
  linhaDaVersao,
  rotulo,
  situacao,
  type EstadoDaAtualizacao,
} from "./atualizacao";

/**
 * As cinco respostas de "estou atualizado?", cada uma com o estado que a produz.
 *
 * O defeito que estes testes guardam não é de cálculo: é de CONFUSÃO. "Conferi e
 * você está em dia" e "não consegui conferir" apareciam iguais na tela — as duas
 * como nada —, e um M/OS que nunca conseguiu falar com o GitHub parecia
 * atualizado. É daí que sai a queixa de que a atualização às vezes não funciona.
 */

const VAZIO: EstadoDaAtualizacao = {
  versao: "0.3.1",
  instaladaEm: "",
  verificadaEm: "",
  disponivel: "",
  publicadaEm: "",
  falha: "",
  falhaEm: "",
  endpoint: "https://exemplo/latest.json",
};

/** Um relógio de mentira: os testes falam de estado, e não de "há 3 minutos". */
const relativa = (iso: string) => (iso ? "há pouco" : "");

describe("qual das cinco", () => {
  it("nunca conferido não é o mesmo que em dia", () => {
    // Este é o estado de uma instalação recém-feita. Chamá-lo de "em dia" seria
    // afirmar algo que ninguém verificou.
    expect(situacao(VAZIO, false)).toBe("nunca");
    expect(rotulo(situacao(VAZIO, false))).toBe("NUNCA CONFERI");
  });

  it("conferido e sem novidade é em dia", () => {
    const estado = { ...VAZIO, verificadaEm: "2026-08-28T12:00:00Z" };
    expect(situacao(estado, false)).toBe("em-dia");
  });

  it("uma falha NÃO se disfarça de em dia", () => {
    // O defeito original: rede fora deixava a tela igual a uma verificação que
    // deu certo.
    const estado = { ...VAZIO, falha: "Sem conexão.", falhaEm: "2026-08-28T12:00:00Z" };
    expect(situacao(estado, false)).toBe("sem-resposta");
    expect(rotulo(situacao(estado, false))).toBe("NÃO CONFERIDO");
  });

  it("uma versão nova conhecida vence uma falha de hoje", () => {
    // Saber que existe a 0.3.2 continua verdade mesmo que a tentativa de agora
    // tenha caído — e é sobre isso que dá para agir.
    const estado = {
      ...VAZIO,
      verificadaEm: "2026-08-27T12:00:00Z",
      disponivel: "0.3.2",
      falha: "Sem conexão.",
      falhaEm: "2026-08-28T12:00:00Z",
    };
    expect(situacao(estado, false)).toBe("atrasado");
  });

  it("a anotação de uma versão que JÁ foi instalada não deixa o selo mentir", () => {
    // Instalou a 0.3.2 e a anotação ficou para trás: o M/OS está em dia, e é a
    // nota que envelheceu.
    const estado = { ...VAZIO, versao: "0.3.2", disponivel: "0.3.2", verificadaEm: "2026-08-28T12:00:00Z" };
    expect(situacao(estado, false)).toBe("em-dia");
  });

  it("enquanto trabalha, a resposta honesta é 'ainda não sei'", () => {
    expect(situacao({ ...VAZIO, verificadaEm: "2026-08-28T12:00:00Z" }, true)).toBe("trabalhando");
  });
});

describe("a linha da versão", () => {
  it("diz a versão e o dia em que ela chegou neste computador", () => {
    const linha = linhaDaVersao({ ...VAZIO, instaladaEm: "2026-08-26T14:31:00Z" });
    expect(linha).toContain("0.3.1");
    expect(linha).toContain("instalada em");
    expect(linha).toContain("26/08");
  });

  it("sem carimbo, diz a versão e para de falar", () => {
    // Não saber a data é uma informação a menos. Inventar "hoje" seria pior.
    expect(linhaDaVersao(VAZIO)).toBe("Versão 0.3.1");
  });
});

describe("a linha da verificação", () => {
  it("em dia diz quando conferiu, e não só que está em dia", () => {
    const estado = { ...VAZIO, verificadaEm: "2026-08-28T12:00:00Z" };
    expect(linhaDaVerificacao(estado, relativa)).toBe("Conferido há pouco. Nenhuma versão nova.");
  });

  it("atrasado nomeia a versão e o dia em que ela saiu", () => {
    // Meio-dia UTC, e não madrugada: `toLocaleDateString` fala no fuso de quem
    // lê, e um instante perto da virada faria este teste falhar do Brasil para
    // leste sem que nada estivesse errado.
    const estado = { ...VAZIO, disponivel: "0.3.2", publicadaEm: "2026-08-28T12:00:00Z" };
    const linha = linhaDaVerificacao(estado, relativa);
    expect(linha).toContain("0.3.2");
    expect(linha).toContain("28/08");
  });

  it("a falha carrega o motivo, e não só a queixa", () => {
    const estado = { ...VAZIO, falha: "Sem conexão.", falhaEm: "2026-08-28T12:00:00Z" };
    expect(linhaDaVerificacao(estado, relativa)).toContain("Sem conexão.");
  });

  it("a falha NÃO apaga o que se soube antes", () => {
    // Apagar transformaria uma queda de rede em "você nunca verificou" — some a
    // única informação que ainda valia.
    const estado = {
      ...VAZIO,
      verificadaEm: "2026-08-25T12:00:00Z",
      falha: "Sem conexão.",
      falhaEm: "2026-08-28T12:00:00Z",
    };
    expect(linhaDaVerificacao(estado, relativa)).toContain("deu certo");
  });

  it("nunca conferido diz isso com todas as letras", () => {
    expect(linhaDaVerificacao(VAZIO, relativa)).toContain("Ainda não conferi");
  });
});

describe("conferir sozinho", () => {
  const agora = new Date("2026-08-28T12:00:00Z");

  it("confere quando nunca conferiu", () => {
    expect(deveConferirSozinho(VAZIO, agora)).toBe(true);
  });

  it("não confere de novo logo depois de conferir", () => {
    // O M/OS abre no logon; uma ida à rede a cada abertura seria um pedágio.
    const estado = { ...VAZIO, verificadaEm: "2026-08-28T10:00:00Z" };
    expect(deveConferirSozinho(estado, agora)).toBe(false);
  });

  it("confere quando a última resposta está velha", () => {
    const estado = { ...VAZIO, verificadaEm: "2026-08-27T12:00:00Z" };
    expect(deveConferirSozinho(estado, agora)).toBe(true);
  });

  it("uma FALHA recente também segura a próxima tentativa", () => {
    // Sem isto, um GitHub fora do ar viraria uma tentativa por abertura de
    // janela — e o painel ficaria piscando erro sem nenhum ganho.
    const estado = { ...VAZIO, falha: "Sem conexão.", falhaEm: "2026-08-28T11:00:00Z" };
    expect(deveConferirSozinho(estado, agora)).toBe(false);
  });

  it("não confere sozinho quando já sabe que há versão nova", () => {
    // Não há o que descobrir: a tela já está dizendo, e o que falta é um clique.
    const estado = { ...VAZIO, disponivel: "0.3.2", verificadaEm: "2026-08-20T12:00:00Z" };
    expect(deveConferirSozinho(estado, agora)).toBe(false);
  });

  it("sem estado lido ainda, não sai atirando", () => {
    // `null` é "o disco ainda não respondeu", e não "nunca conferi".
    expect(deveConferirSozinho(null, agora)).toBe(false);
  });
});

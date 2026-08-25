import { describe, expect, it } from "vitest";
import {
  aqui, avancar, comecar, PASSOS_GUARDADOS, podeAvancar, podeVoltar, visitar, voltar,
} from "./navegacao";

describe("a trilha de navegacao", () => {
  it("comeca sem volta e sem avanco", () => {
    const trilha = comecar("home");
    expect(aqui(trilha)).toBe("home");
    expect(podeVoltar(trilha)).toBe(false);
    expect(podeAvancar(trilha)).toBe(false);
  });

  it("volta e avanca pelo caminho andado", () => {
    let trilha = comecar("home");
    trilha = visitar(trilha, "projects");
    trilha = visitar(trilha, "library");
    expect(aqui(trilha)).toBe("library");

    trilha = voltar(trilha);
    expect(aqui(trilha)).toBe("projects");
    expect(podeAvancar(trilha)).toBe(true);

    trilha = voltar(trilha);
    expect(aqui(trilha)).toBe("home");
    expect(podeVoltar(trilha)).toBe(false);

    trilha = avancar(avancar(trilha));
    expect(aqui(trilha)).toBe("library");
  });

  /* O clique no rail estando ja na pagina.
     Sem esta guarda, voltar depois de dois cliques em "Home" continuaria na
     Home — e o botao que nao sai do lugar e o botao em que ninguem confia de
     novo. */
  it("ir para onde ja se esta nao anda a trilha", () => {
    let trilha = visitar(comecar("home"), "tasks");
    const antes = trilha;
    trilha = visitar(trilha, "tasks");
    expect(trilha).toBe(antes);
    expect(podeVoltar(trilha)).toBe(true);
    expect(aqui(voltar(trilha))).toBe("home");
  });

  /* O caminho novo apaga o antigo, como em qualquer navegador. Manter os dois
     exigiria da pessoa um modelo de arvore que ninguem tem. */
  it("andar depois de voltar corta o futuro", () => {
    let trilha = comecar("home");
    trilha = visitar(trilha, "projects");
    trilha = visitar(trilha, "library");
    trilha = voltar(trilha);
    trilha = visitar(trilha, "reunioes");

    expect(aqui(trilha)).toBe("reunioes");
    expect(podeAvancar(trilha)).toBe(false);
    expect(aqui(voltar(trilha))).toBe("projects");
  });

  /* A janela do M/OS nao fecha, ela esconde — uma sessao dura dias. Um
     historico sem teto seria um vazamento com cara de recurso. */
  it("guarda um teto de passos, e o corte mantem os mais recentes", () => {
    let trilha = comecar("home");
    for (let volta = 0; volta < PASSOS_GUARDADOS + 10; volta += 1) {
      trilha = visitar(trilha, volta % 2 === 0 ? "tasks" : "inbox");
    }
    expect(trilha.paginas).toHaveLength(PASSOS_GUARDADOS);
    expect(trilha.indice).toBe(PASSOS_GUARDADOS - 1);
    // O indice continua apontando para a ultima visitada, e nao para o lugar
    // que ela ocupava antes do corte.
    expect(aqui(trilha)).toBe(trilha.paginas[trilha.paginas.length - 1]);
  });

  it("voltar no comeco e avancar no fim nao mudam nada", () => {
    const trilha = comecar("home");
    expect(voltar(trilha)).toBe(trilha);
    expect(avancar(trilha)).toBe(trilha);
  });
});

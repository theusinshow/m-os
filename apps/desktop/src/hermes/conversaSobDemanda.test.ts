import { describe, expect, it } from "vitest";
import { conversaSobDemanda, type PortaDaConversa } from "./conversaSobDemanda";
import type { Conversation } from "../hermes";

function conversaFalsa(id: string): Conversation {
  return {
    id,
    title: "",
    hermesSessionId: null,
    lifecycleState: "active",
    createdAt: "2026-09-15T00:00:00Z",
    updatedAt: "2026-09-15T00:00:00Z",
  } as unknown as Conversation;
}

/** Porta de mentira que conta o que foi pedido a ela. */
function portaFalsa(falhasIniciais = 0) {
  const criadas: string[] = [];
  const selecionadas: string[] = [];
  let proxima = 0;
  let falhasRestantes = falhasIniciais;
  const porta: PortaDaConversa = {
    async create() {
      if (falhasRestantes > 0) {
        falhasRestantes -= 1;
        throw new Error("banco fora");
      }
      const id = `c${++proxima}`;
      criadas.push(id);
      return conversaFalsa(id);
    },
    async selecionar(id) {
      selecionadas.push(id);
    },
  };
  return { porta, criadas, selecionadas };
}

describe("a conversa nasce quando a pessoa fala", () => {
  it("a primeira pergunta cria a conversa e a seleciona", async () => {
    // Selecionar e obrigatorio: o lado Rust guarda a conversa corrente num slot
    // proprio, que o connect() ja apontou para a mais recente. Sem isso a
    // primeira frase cairia na conversa velha.
    const { porta, criadas, selecionadas } = portaFalsa();
    const demanda = conversaSobDemanda(porta);

    const conversa = await demanda.garantir(null);

    expect(conversa?.id).toBe("c1");
    expect(criadas).toEqual(["c1"]);
    expect(selecionadas).toEqual(["c1"]);
  });

  it("a segunda pergunta reusa a conversa que ja existe", async () => {
    const { porta, criadas, selecionadas } = portaFalsa();
    const demanda = conversaSobDemanda(porta);

    const conversa = await demanda.garantir(conversaFalsa("ja-existe"));

    expect(conversa?.id).toBe("ja-existe");
    expect(criadas).toEqual([]);
    expect(selecionadas).toEqual([]);
  });

  it("duas perguntas no mesmo instante criam uma conversa so", async () => {
    // O estado do React nao voltou ainda quando a segunda chamada entra, entao
    // as duas veem `null`. Sem memoria da criacao em curso virariam duas
    // conversas, e a segunda frase ficaria orfa numa delas.
    const { porta, criadas } = portaFalsa();
    const demanda = conversaSobDemanda(porta);

    const [uma, outra] = await Promise.all([demanda.garantir(null), demanda.garantir(null)]);

    expect(criadas).toEqual(["c1"]);
    expect(uma?.id).toBe("c1");
    expect(outra?.id).toBe("c1");
  });

  it("o botao de nova conversa faz a proxima pergunta criar outra", async () => {
    const { porta, criadas } = portaFalsa();
    const demanda = conversaSobDemanda(porta);
    await demanda.garantir(null);

    demanda.reiniciar();
    const segunda = await demanda.garantir(null);

    expect(segunda?.id).toBe("c2");
    expect(criadas).toEqual(["c1", "c2"]);
  });

  it("quando a criacao falha nada e selecionado, e a proxima tentativa tenta de novo", async () => {
    // Guardar a promessa falhada deixaria a tela travada em "nao da para
    // enviar" ate trocar de pagina.
    const { porta, criadas, selecionadas } = portaFalsa(1);
    const demanda = conversaSobDemanda(porta);

    expect(await demanda.garantir(null)).toBeNull();
    expect(criadas).toEqual([]);
    expect(selecionadas).toEqual([]);

    const segunda = await demanda.garantir(null);

    expect(segunda?.id).toBe("c1");
    expect(selecionadas).toEqual(["c1"]);
  });
});

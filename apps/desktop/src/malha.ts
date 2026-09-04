import type { AparelhoNaMalha } from "./types";

export type Alinhamento = {
  id: string;
  estado: "alinhado" | "atras" | "divergente";
  /** A frase curta que a linha mostra. Vazia quando não há o que dizer. */
  detalhe: string;
};

/**
 * Compara cada aparelho com ESTE, família a família.
 *
 * "Atrás" e "divergente" são estados distintos porque pedem ações distintas: o
 * primeiro se resolve sozinho na próxima rodada; o segundo é a mesma contagem
 * com conteúdo diferente, e aí a linha oferece reparo.
 *
 * Aparelho sem manifesto não é acusado de nada. Ele é um M/OS que ainda não
 * atualizou, e marcá-lo de divergente ensinaria a ignorar o aviso — que é
 * exatamente o que um aviso não pode fazer.
 */
export function alinhamento(aparelhos: AparelhoNaMalha[], meuId: string): Alinhamento[] {
  const eu = aparelhos.find((aparelho) => aparelho.id === meuId);
  const referencia = new Map((eu?.manifesto ?? []).map((familia) => [familia.familia, familia]));

  return aparelhos.map((aparelho) => {
    if (aparelho.id === meuId) {
      return { id: aparelho.id, estado: "alinhado", detalhe: "" };
    }
    if (aparelho.manifesto.length === 0) {
      return { id: aparelho.id, estado: "alinhado", detalhe: "sem manifesto" };
    }

    const dele = new Map(aparelho.manifesto.map((familia) => [familia.familia, familia]));
    const atrasadas: string[] = [];
    const divergentes: string[] = [];
    for (const [nome, minha] of referencia) {
      const dela = dele.get(nome);
      if (!dela || dela.contagem < minha.contagem) {
        atrasadas.push(`${nome}: ${dela?.contagem ?? 0} de ${minha.contagem}`);
      } else if (dela.hash !== minha.hash) {
        divergentes.push(`${nome}: mesma contagem, conteúdo diferente`);
      }
    }

    if (atrasadas.length > 0) {
      return { id: aparelho.id, estado: "atras", detalhe: atrasadas.join(" · ") };
    }
    if (divergentes.length > 0) {
      return { id: aparelho.id, estado: "divergente", detalhe: divergentes.join(" · ") };
    }
    return { id: aparelho.id, estado: "alinhado", detalhe: "" };
  });
}

/**
 * Onde Argos cabe agora.
 *
 * Tres estados e nao dois porque ha disputantes pelos cantos de baixo:
 * `.attention-toast`, `.drop-panel` e as GAVETAS na direita, `.receipt` na
 * esquerda. Quando os dois lados estao tomados, quem sai e o bicho — o aviso
 * carrega dado que ele nao carrega.
 *
 * A gaveta entrou depois, e por um defeito visto na janela real: ela e ancorada
 * a direita e vai ate o rodape, entao Argos ficava POR CIMA do botao primario
 * dela. Um botao coberto por enfeite e um botao que erra o clique.
 *
 * **A ocupacao vem do estado do shell, nunca de medir o DOM.** Medicao aqui
 * decidiria sobre leitura em cache, que e o erro que ja custou uma investigacao
 * inteira neste projeto.
 */
export type ArgosCanto = "direita" | "esquerda" | "oculto";

export type ArgosOcupacao = {
  direitaOcupada: boolean;
  esquerdaOcupada: boolean;
};

export function cantoPara({ direitaOcupada, esquerdaOcupada }: ArgosOcupacao): ArgosCanto {
  if (!direitaOcupada) return "direita";
  if (!esquerdaOcupada) return "esquerda";
  return "oculto";
}

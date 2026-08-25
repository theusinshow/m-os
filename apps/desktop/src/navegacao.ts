import type { Page } from "./types";

/**
 * A trilha de telas por onde a pessoa passou, e onde ela está nela.
 *
 * # Por que existe
 *
 * Até 2026-08-25 a navegação do M/OS era `useState<Page>("home")`: um ponteiro
 * para a tela de agora, sem memória de como se chegou nela. O rail levava a
 * qualquer lugar e **nenhum caminho levava de volta**.
 *
 * Isso não é uma falta de recurso, é uma falta de chão. Abrir um Project a
 * partir de um card da Home, ir dele para a Library atrás de um arquivo e
 * querer voltar significava: achar o rail, clicar em Home, e reencontrar o card
 * — que é exatamente o custo mental que o `UX-PRINCIPLES` §37 ("preserve
 * context") e §38 ("context switching deve ser barato") existem para remover. O
 * §41 pede uma navegação PREVISÍVEL, e uma navegação sem volta é a mais
 * imprevisível de todas: cada passo é definitivo.
 *
 * A posição de rolagem já era guardada por página no `App.tsx`. Isto completa o
 * par: aquilo lembra ONDE na tela, isto lembra QUAL tela.
 *
 * # Por que é uma trilha, e não uma pilha
 *
 * Pilha só volta. Trilha volta e avança — e voltar sem poder avançar transforma
 * um clique de curiosidade ("deixa eu ver o que tinha lá atrás") em perda de
 * lugar. O modelo é o do navegador porque é o que qualquer pessoa já tem na
 * cabeça, e reinventar navegação é o tipo de originalidade que o §41 recusa.
 *
 * # Por que módulo próprio, com teste
 *
 * A aritmética de índice em histórico erra em silêncio: um `slice` com o corte
 * um número fora deixa a tela certa com o botão de voltar errado, e nada
 * quebra. O `App.tsx` não tem teste de DOM neste repo; esta parte tem teste
 * porque ela é a parte que dá para testar.
 */
export type Trilha = {
  /** Da mais antiga para a mais recente. Nunca vazia. */
  readonly paginas: readonly Page[];
  /** Onde a pessoa está. Sempre um índice válido de `paginas`. */
  readonly indice: number;
};

/**
 * Quantos passos a trilha guarda.
 *
 * Trinta é muito além de qualquer volta que alguém dá de verdade, e é o
 * bastante para o teto nunca ser percebido. Ele existe porque uma sessão do
 * M/OS dura dias — a janela não fecha, ela esconde (ver `on_window_event` no
 * `lib.rs`) — e um array sem teto num app que não reinicia é um vazamento com
 * cara de histórico.
 */
export const PASSOS_GUARDADOS = 30;

export function comecar(page: Page): Trilha {
  return { paginas: [page], indice: 0 };
}

/** Onde a pessoa está agora. */
export function aqui(trilha: Trilha): Page {
  return trilha.paginas[trilha.indice];
}

export function podeVoltar(trilha: Trilha): boolean {
  return trilha.indice > 0;
}

export function podeAvancar(trilha: Trilha): boolean {
  return trilha.indice < trilha.paginas.length - 1;
}

/**
 * A pessoa foi para outra tela.
 *
 * Duas decisões moram aqui:
 *
 * 1. **Ir para onde já se está não anda.** O rail é clicável mesmo na página
 *    atual, e sem esta guarda um clique distraído em "Home" estando na Home
 *    empilharia Home sobre Home — e daí voltar não sairia do lugar, que é a
 *    forma mais rápida de a pessoa deixar de confiar no botão.
 * 2. **Andar corta o futuro.** Voltar duas telas e seguir por outro caminho
 *    descarta o caminho antigo, como em qualquer navegador. Manter os dois
 *    exigiria da pessoa um modelo de árvore que ninguém tem.
 */
export function visitar(trilha: Trilha, page: Page): Trilha {
  if (aqui(trilha) === page) return trilha;
  const ate_aqui = trilha.paginas.slice(0, trilha.indice + 1);
  const paginas = [...ate_aqui, page].slice(-PASSOS_GUARDADOS);
  return { paginas, indice: paginas.length - 1 };
}

/**
 * Um passo atrás. No começo da trilha, não faz nada.
 *
 * Devolver a mesma trilha em vez de erro é deliberado: quem chama é um atalho
 * de teclado e um botão de mouse, e os dois disparam o tempo todo sem que
 * ninguém tenha "pedido" para voltar.
 */
export function voltar(trilha: Trilha): Trilha {
  return podeVoltar(trilha) ? { ...trilha, indice: trilha.indice - 1 } : trilha;
}

/** Um passo à frente. No fim da trilha, não faz nada. */
export function avancar(trilha: Trilha): Trilha {
  return podeAvancar(trilha) ? { ...trilha, indice: trilha.indice + 1 } : trilha;
}

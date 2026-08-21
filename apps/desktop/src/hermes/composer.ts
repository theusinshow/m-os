/**
 * As contas do Smart Composer.
 *
 * Ficam fora do componente porque são as únicas partes dele que podem estar
 * erradas em silêncio: altura que não cresce, altura que cresce para sempre, e
 * a decisão de quando puxar a conversa para baixo.
 */

/** O campo nunca nasce menor que isto, mesmo vazio. */
export const LINHAS_MINIMAS = 2;
/** Nem cresce além disto: a partir daqui ele rola por dentro. */
export const LINHAS_MAXIMAS = 12;

export type MedidaDoCampo = {
  /** Altura a aplicar, em px. */
  altura: number;
  /** Verdadeiro quando o texto passou do teto e o campo passa a rolar. */
  rolando: boolean;
};

/**
 * A altura do campo para um dado conteúdo.
 *
 * `conteudo` é o `scrollHeight` medido com a altura zerada — é o único jeito de
 * o navegador informar o tamanho do texto sem a altura anterior contaminando a
 * medida.
 *
 * O teto existe para que uma colagem de trinta linhas não engula a conversa. O
 * piso existe porque um campo de uma linha só parece caixa de busca, e este é o
 * ponto de comando da tela.
 */
export function medirCampo(conteudo: number, linha: number, moldura = 0): MedidaDoCampo {
  const piso = linha * LINHAS_MINIMAS + moldura;
  const teto = linha * LINHAS_MAXIMAS + moldura;
  if (conteudo <= piso) return { altura: piso, rolando: false };
  if (conteudo >= teto) return { altura: teto, rolando: true };
  return { altura: conteudo, rolando: false };
}

/**
 * Se a conversa deve continuar colada no fim.
 *
 * A regra do redesign §19: quem subiu para ler não é puxado de volta. A folga
 * existe porque "no fim" não é pixel exato — o crescimento do campo, a barra de
 * rolagem e o arredondamento do zoom movem o número por alguns pixels sem que
 * ninguém tenha rolado nada.
 */
export function coladoNoFim(alturaTotal: number, topo: number, alturaVisivel: number, folga = 96): boolean {
  return alturaTotal - topo - alturaVisivel <= folga;
}

/**
 * O rascunho depois de escolher uma menção.
 *
 * Troca só o `@token` que está sendo digitado, e em qualquer posição da frase —
 * menção no meio do texto é o uso comum. Devolve o rascunho inteiro para que o
 * chamador não precise saber onde o token estava.
 */
export function aplicarMencao(rascunho: string, rotulo: string): string {
  return rascunho.replace(/@([\wÀ-ú]*)$/, `@${rotulo} `);
}

/** O token de menção em digitação, ou `null`. Dois caracteres antes de buscar:
 *  com um só, toda menção dispara uma busca que devolve o acervo inteiro. */
export function tokenDeMencao(rascunho: string, minimo = 2): string | null {
  const encontrado = /@([\wÀ-ú]*)$/.exec(rascunho);
  if (!encontrado || encontrado[1].length < minimo) return null;
  return encontrado[1];
}

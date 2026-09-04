import type { CartaoDaHome } from "./cartoes";

/**
 * Como ESTE aparelho arruma a Home.
 *
 * # Por que o arranjo não sincroniza
 *
 * É a mesma decisão que mantém `workspace_widget_layout` fora do sync no
 * desktop: arranjo de tela é da tela. O que você quer ver primeiro no celular,
 * na fila do ônibus, não é o que quer ver no monitor de 27 polegadas — e
 * replicar o layout faria um mandar no outro.
 *
 * Por isso ele vive no `localStorage`, e não no banco: banco é o que atravessa.
 */
export type Arranjo = {
  /** As chaves na ordem escolhida. Chave ausente aqui vai para o fim. */
  ordem: string[];
  /** O que foi escondido. Escondido é diferente de ausente: o cartão continua
   *  existindo, e volta quando você quiser. */
  ocultos: string[];
};

export const ARRANJO_VAZIO: Arranjo = { ordem: [], ocultos: [] };

const CHAVE = "mos.home.arranjo";

/**
 * Aplica o arranjo aos cartões que a Home montou.
 *
 * O que não está na ordem vai para o FIM, e não some: um cartão novo — de uma
 * versão futura — apareceria invisível se a ausência significasse ocultar, e
 * ninguém descobriria que ele existe.
 */
export function aplicarArranjo(cartoes: CartaoDaHome[], arranjo: Arranjo): CartaoDaHome[] {
  const posicao = new Map(arranjo.ordem.map((chave, indice) => [chave, indice]));
  return cartoes
    .filter((cartao) => !arranjo.ocultos.includes(cartao.chave))
    .sort(
      (um, outro) =>
        (posicao.get(um.chave) ?? Number.MAX_SAFE_INTEGER) -
        (posicao.get(outro.chave) ?? Number.MAX_SAFE_INTEGER),
    );
}

/**
 * Move um cartão uma posição.
 *
 * Trabalha sobre a ordem VISÍVEL que foi passada, e não sobre `arranjo.ordem`:
 * quem está arrumando vê a tela, e mover "para cima" tem que trocar com o
 * cartão que está acima na tela — mesmo que a ordem guardada esteja pela metade.
 */
export function mover(
  arranjo: Arranjo,
  visiveis: string[],
  chave: string,
  direcao: "cima" | "baixo",
): Arranjo {
  const atual = [...visiveis];
  const de = atual.indexOf(chave);
  if (de < 0) return arranjo;
  const para = direcao === "cima" ? de - 1 : de + 1;
  if (para < 0 || para >= atual.length) return arranjo;
  [atual[de], atual[para]] = [atual[para], atual[de]];
  return { ...arranjo, ordem: atual };
}

/** Esconde um cartão. */
export function ocultar(arranjo: Arranjo, chave: string): Arranjo {
  if (arranjo.ocultos.includes(chave)) return arranjo;
  return { ...arranjo, ocultos: [...arranjo.ocultos, chave] };
}

/** Traz de volta. */
export function mostrar(arranjo: Arranjo, chave: string): Arranjo {
  return { ...arranjo, ocultos: arranjo.ocultos.filter((oculto) => oculto !== chave) };
}

/**
 * Lê o arranjo guardado.
 *
 * Falha em silêncio de propósito: janela anônima e "bloquear dados de sites"
 * fazem o `localStorage` LANÇAR, e uma Home que não abre porque o arranjo não
 * pôde ser lido troca um problema de preferência por um problema de acesso.
 */
export function lerArranjo(): Arranjo {
  try {
    const cru = window.localStorage.getItem(CHAVE);
    if (!cru) return ARRANJO_VAZIO;
    const lido = JSON.parse(cru) as Partial<Arranjo>;
    return {
      ordem: Array.isArray(lido.ordem) ? lido.ordem.filter((c) => typeof c === "string") : [],
      ocultos: Array.isArray(lido.ocultos)
        ? lido.ocultos.filter((c) => typeof c === "string")
        : [],
    };
  } catch {
    return ARRANJO_VAZIO;
  }
}

/** Guarda. Falha em silêncio pela mesma razão da leitura. */
export function gravarArranjo(arranjo: Arranjo): void {
  try {
    window.localStorage.setItem(CHAVE, JSON.stringify(arranjo));
  } catch {
    // Sem onde guardar, o arranjo vale para esta sessão. É menos do que se
    // queria, e infinitamente melhor que a tela recusar o toque.
  }
}

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
 * Os cartões na ordem escolhida — todos, inclusive os escondidos.
 *
 * O que não está na ordem vai para o FIM, e não some: um cartão novo — de uma
 * versão futura — apareceria invisível se a ausência significasse ocultar, e
 * ninguém descobriria que ele existe.
 */
export function ordenar(cartoes: CartaoDaHome[], arranjo: Arranjo): CartaoDaHome[] {
  const posicao = new Map(arranjo.ordem.map((chave, indice) => [chave, indice]));
  return [...cartoes].sort(
    (um, outro) =>
      (posicao.get(um.chave) ?? Number.MAX_SAFE_INTEGER) -
      (posicao.get(outro.chave) ?? Number.MAX_SAFE_INTEGER),
  );
}

/** O que a Home mostra fora do modo de arrumar: a ordem, sem os escondidos. */
export function aplicarArranjo(cartoes: CartaoDaHome[], arranjo: Arranjo): CartaoDaHome[] {
  return ordenar(cartoes, arranjo).filter((cartao) => !arranjo.ocultos.includes(cartao.chave));
}

/**
 * Tira o cartão de uma posição e o enfia noutra.
 *
 * Trabalha sobre a lista de chaves que está NA TELA, e não sobre
 * `arranjo.ordem`: quem arrasta vê a grade, e soltar sobre o terceiro cartão
 * tem que pôr o cartão em terceiro — mesmo que a ordem guardada esteja pela
 * metade ou vazia.
 */
export function reordenar(
  arranjo: Arranjo,
  chaves: string[],
  de: number,
  para: number,
): Arranjo {
  if (de === para) return arranjo;
  if (de < 0 || de >= chaves.length) return arranjo;
  if (para < 0 || para >= chaves.length) return arranjo;
  const ordem = [...chaves];
  const [movido] = ordem.splice(de, 1);
  ordem.splice(para, 0, movido);
  return { ...arranjo, ordem };
}

/** Esconde, ou traz de volta. Um alvo só, porque é um estado só. */
export function alternarOculto(arranjo: Arranjo, chave: string): Arranjo {
  return arranjo.ocultos.includes(chave)
    ? { ...arranjo, ocultos: arranjo.ocultos.filter((oculto) => oculto !== chave) }
    : { ...arranjo, ocultos: [...arranjo.ocultos, chave] };
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

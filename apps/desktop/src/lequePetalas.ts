/**
 * O leque: quais pétalas existem, onde o desenho as pôs, e o que a pessoa mudou.
 *
 * ESTA É A ÚNICA CÓPIA DA REGRA, e isso é decisão e não acaso. O `homeLayout.ts`
 * registra o que aconteceu quando a regra do arranjo viveu em dois lugares: a
 * cópia do Rust "ficou para tras em silencio — com os testes dela passando, que
 * e o pior jeito de ficar para tras". O que ficou no core é só o que o BANCO
 * precisa para não aceitar lixo, que é o validador de forma do `kind`.
 *
 * Vive fora do `App.tsx` para poder ser testado: não há teste de DOM neste repo,
 * então o que se verifica tem de ser função pura.
 */
import type { RadialPin } from "./types";

/** Cinco, e o número é a feature.
 *
 *  O leque só é mais rápido que o Ctrl+K enquanto for memória muscular, e
 *  memória muscular exige que o alvo não se mova. Se o número de pétalas
 *  variasse com quantas estão fixadas, cada nova pétala moveria as outras
 *  quatro — e o que sobraria seria um Ctrl+K pior, sem busca. */
export const SLOTS = 5;

export type PetalaKind = "app" | "acao" | "pagina";

export type Petala = {
  slot: number;
  kind: PetalaKind;
  target: string;
};

const KINDS: readonly PetalaKind[] = ["app", "acao", "pagina"];

/** O padrão de fábrica.
 *
 *  Os três primeiros são exatamente os que saíram do rail. Não é conveniência:
 *  a ADR-038 tirou Apps do rail e acrescentou a porta nova NO MESMO commit,
 *  registrando que sem ela "a pagina ficaria inalcancavel". Aqui é a mesma
 *  dívida, paga do mesmo jeito.
 *
 *  O quarto é o M-Finance porque, dos cinco apps cadastrados, ele é o único com
 *  `launchKind` e `canOpen` — os outros quatro dariam pétalas que não fazem
 *  nada. O id é o do registro, e não o nome, porque o nome muda. */
export const PETALAS_DE_FABRICA: Petala[] = [
  { slot: 0, kind: "pagina", target: "calendario" },
  { slot: 1, kind: "pagina", target: "finance" },
  { slot: 2, kind: "pagina", target: "reunioes" },
  { slot: 3, kind: "app", target: "019ffc4f-2936-7152-84b7-672d7bdb5bfc" },
  { slot: 4, kind: "acao", target: "quick_capture" },
];

/**
 * O leque efetivo: o desenho, com os slots que a pessoa trocou por cima.
 *
 * Lista vazia devolve o desenho INTEIRO, e não um leque vazio — é a inversão que
 * a migration 0021 documenta. Trocar um slot não congela os outros quatro, então
 * mudar o padrão de fábrica ainda alcança quem nunca personalizou.
 */
export function resolverPetalas(pins: RadialPin[], workspaceId: string | null): Petala[] {
  const doEscopo = new Map<number, Petala>();
  for (const pin of pins) {
    if ((pin.workspaceId ?? null) !== workspaceId) continue;
    // O banco aceita slot 0..11 de propósito — ele guarda forma. QUANTAS
    // posições a interface oferece é vocabulário, e o vocabulário é este SLOTS.
    if (!Number.isInteger(pin.slot) || pin.slot < 0 || pin.slot >= SLOTS) continue;
    // `kind` é opaco no banco pelo mesmo motivo. Um tipo que este front não
    // conhece cai fora aqui, e o slot volta ao desenho, em vez de virar uma
    // pétala que não sabe o que fazer quando clicada.
    if (!KINDS.includes(pin.kind as PetalaKind)) continue;
    if (!pin.target.trim()) continue;
    doEscopo.set(pin.slot, { slot: pin.slot, kind: pin.kind as PetalaKind, target: pin.target });
  }
  return PETALAS_DE_FABRICA.map((padrao) => doEscopo.get(padrao.slot) ?? padrao);
}

/** Abertura total do arco, em graus. 120° cabe cinco pétalas com folga de toque
 *  sem que as das pontas cheguem à horizontal, onde elas apontariam para o
 *  recibo de desfazer e para o toast de atenção. */
const ARCO = 120;

/**
 * O ângulo de um slot, em graus, com -90 apontando para cima.
 *
 * Depende do slot e de `SLOTS`, e de mais NADA — em particular, não depende de
 * quantas pétalas estão preenchidas. É essa independência que a memória muscular
 * consome, e há um teste só para ela.
 */
export function anguloDaPetala(slot: number): number {
  const passo = ARCO / (SLOTS - 1);
  return -90 - ARCO / 2 + slot * passo;
}

/** O deslocamento da pétala em relação à âncora, em pixels. `y` negativo sobe,
 *  como no sistema de coordenadas da tela. */
export function posicaoDaPetala(slot: number, raio: number): { x: number; y: number } {
  const radianos = (anguloDaPetala(slot) * Math.PI) / 180;
  return { x: raio * Math.cos(radianos), y: raio * Math.sin(radianos) };
}

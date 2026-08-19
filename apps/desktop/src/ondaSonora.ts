/**
 * A onda do card de gravação: a janela de níveis e a altura de cada barra.
 *
 * Vive fora do componente para poder ser testada — não há teste de DOM neste
 * repo, então a regra tem de ser função pura. O componente desenha; aqui está o
 * que ele desenha.
 */

/** Trinta barras a 15 Hz são dois segundos de história. Menos que isso não
 *  mostra a cadência da fala; mais vira gráfico, e gráfico é o cockpit que o
 *  desenho recusa. */
export const BARRAS = 30;

/** Os degraus do modo sem movimento. Oito, os mesmos que a barra da topbar
 *  usava antes de o nível mudar de casa. */
export const DEGRAUS = 8;

/** O nível que o backend chama de cheio. RMS em milésimos, como o tick. */
const TETO = 1000;

/** A menor altura visível. Silêncio precisa desenhar ALGUMA coisa: uma barra de
 *  altura zero leria como "o áudio morreu", e silêncio não é queda — distinguir
 *  os dois é a razão de a onda existir. */
const PISO = 0.08;

/**
 * Empurra um nível pela direita e descarta o mais velho.
 *
 * Uma janela curta demais é preenchida com silêncio à esquerda, e não deixada
 * curta: uma onda que cresce da esquerda nos dois primeiros segundos parece
 * animação de entrada, e não medida.
 */
export function empurrar(janela: number[], nivel: number): number[] {
  const base = janela.length >= BARRAS
    ? janela.slice(janela.length - BARRAS + 1)
    : [...Array.from({ length: BARRAS - 1 - janela.length }, () => 0), ...janela];
  return [...base, nivel];
}

/** A altura da barra, de 0 a 1. Satura no teto em vez de estourar: um pico
 *  acima do esperado não deve desenhar fora da caixa. */
export function alturaDaBarra(nivel: number): number {
  const bruto = Math.max(0, nivel) / TETO;
  return Math.min(1, Math.max(PISO, bruto));
}

/** Quantos degraus acender, para quem pediu menos movimento.
 *
 *  Com `prefers-reduced-motion` a onda deixa de mostrar HISTÓRIA e passa a
 *  mostrar só o agora, porque a história é justamente o que rola. O que sobra
 *  ainda distingue silêncio de queda, que é a razão de a onda existir. */
export function degrausAcesos(nivel: number): number {
  const bruto = Math.max(0, nivel) / TETO;
  return Math.min(DEGRAUS, Math.max(1, Math.round(bruto * DEGRAUS)));
}

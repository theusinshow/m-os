/**
 * FLIP: fazer o cartão ir até o lugar novo, em vez de reaparecer nele.
 *
 * # Por que não uma biblioteca
 *
 * Motion One resolveria isto e custa ~18 KB. O truque, porém, é quatro linhas
 * de aritmética sobre `getBoundingClientRect` mais `Element.animate` — que o
 * Safari tem desde 2020 e não pesa nada, porque já está no aparelho. Num app
 * que abre no 4G, 18 KB por uma subtração é caro.
 *
 * O nome é a técnica: **F**irst (medir antes), **L**ast (deixar o React pintar),
 * **I**nvert (empurrar de volta ao lugar antigo, por transformação), **P**lay
 * (soltar). O olho vê o cartão viajar; o DOM nunca saiu do lugar novo.
 */

/** Onde cada cartão está agora. */
export function medir(nos: Map<string, HTMLElement>): Map<string, DOMRect> {
  const medidas = new Map<string, DOMRect>();
  for (const [chave, no] of nos) medidas.set(chave, no.getBoundingClientRect());
  return medidas;
}

/**
 * Anima cada cartão da posição anterior até a atual.
 *
 * `excecao` é o cartão que está sob o dedo: ele já está sendo posicionado pelo
 * arrasto, e animá-lo faria a mão e o desenho discordarem — o cartão iria para
 * onde ele estava enquanto o dedo já está noutro lugar.
 */
export function animarDe(
  anteriores: Map<string, DOMRect>,
  nos: Map<string, HTMLElement>,
  excecao: string | null,
  duracao: number,
): void {
  // Quem pediu menos movimento não recebe nenhum aqui: o CSS global zera as
  // durações, mas `Element.animate` não passa por CSS nenhum.
  if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) return;

  for (const [chave, no] of nos) {
    if (chave === excecao) continue;
    const antes = anteriores.get(chave);
    if (!antes) continue;
    const agora = no.getBoundingClientRect();
    const dx = antes.left - agora.left;
    const dy = antes.top - agora.top;
    // Meio pixel de diferença é ruído de layout, e animá-lo produziria um tremor
    // em cartões que não se moveram.
    if (Math.abs(dx) < 1 && Math.abs(dy) < 1) continue;
    no.animate(
      [{ transform: `translate(${dx}px, ${dy}px)` }, { transform: "translate(0, 0)" }],
      { duration: duracao, easing: "cubic-bezier(0.16, 1, 0.3, 1)" },
    );
  }
}

/**
 * Sobre qual cartão o dedo está.
 *
 * Compara com o CENTRO de cada cartão, e não com a área: numa grade de duas
 * colunas as áreas se tocam, e o índice trocaria no instante em que a borda do
 * dedo cruzasse a fronteira — o que faz a grade piscar entre duas ordens
 * enquanto o dedo está parado em cima da linha.
 */
export function alvoDoDedo(
  nos: Map<string, HTMLElement>,
  chaves: string[],
  x: number,
  y: number,
): number {
  let melhor = -1;
  let menor = Number.POSITIVE_INFINITY;
  chaves.forEach((chave, indice) => {
    const no = nos.get(chave);
    if (!no) return;
    const caixa = no.getBoundingClientRect();
    const dx = x - (caixa.left + caixa.width / 2);
    const dy = y - (caixa.top + caixa.height / 2);
    const distancia = dx * dx + dy * dy;
    if (distancia < menor) {
      menor = distancia;
      melhor = indice;
    }
  });
  return melhor;
}

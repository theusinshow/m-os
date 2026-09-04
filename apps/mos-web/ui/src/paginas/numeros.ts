/**
 * Como o bolso escreve tempo e dinheiro.
 *
 * Uma cópia só, e não uma por tela: dois formatadores divergem no dia em que
 * alguém arredonda diferente, e aí a Home e a página de Horas passam a discordar
 * sobre o mesmo número.
 */

/** `9h08`. Minuto é a menor unidade que importa numa fatura de hora. */
export function emHoras(segundos: number): string {
  const minutos = Math.round(segundos / 60);
  const horas = Math.floor(minutos / 60);
  return `${horas}h${String(minutos % 60).padStart(2, "0")}`;
}

/**
 * `R$ 274,00`, e `R$ 1.234,56`.
 *
 * O "R$" entra à mão, e só o número passa pelo `toLocaleString`: o formato de
 * moeda do navegador usa espaço NÃO-QUEBRÁVEL entre símbolo e valor, e um
 * caractere invisível dentro de uma string é armadilha para quem for comparar
 * isso depois — em teste ou em log.
 */
export function emReais(centavos: number): string {
  const valor = (centavos / 100).toLocaleString("pt-BR", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  return `R$ ${valor}`;
}

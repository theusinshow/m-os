export function formatCurrency(cents: number) {
  return new Intl.NumberFormat("pt-BR", {
    style: "currency",
    currency: "BRL",
  }).format(cents / 100);
}

/**
 * Valor curto o bastante para caber num eixo de gráfico.
 *
 * `formatCurrency` devolve `R$ 12.345,67`, que num `YAxis` ou empurra a área
 * de desenho para fora ou é cortado. Aqui a precisão é trocada por largura de
 * propósito: quem precisa do centavo lê o tooltip, que continua usando
 * `formatCurrency`.
 */
export function formatCurrencyCompact(cents: number) {
  const reais = cents / 100;
  const absolute = Math.abs(reais);
  const sign = reais < 0 ? "-" : "";

  if (absolute >= 1_000_000) {
    return `${sign}R$ ${decimal(absolute / 1_000_000)} mi`;
  }
  if (absolute >= 1_000) {
    return `${sign}R$ ${decimal(absolute / 1_000)} mil`;
  }
  return `${sign}R$ ${Math.round(absolute)}`;
}

/** Uma casa decimal, vírgula no lugar do ponto, sem `,0` pendurado. */
function decimal(value: number) {
  return value.toFixed(1).replace(/\.0$/, "").replace(".", ",");
}

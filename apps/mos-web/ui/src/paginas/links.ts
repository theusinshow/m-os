/**
 * O endereço dentro de um texto capturado.
 *
 * # Por que a tela precisa disto, se o servidor também acha
 *
 * Porque são perguntas diferentes. O servidor acha o link para GRAVAR o
 * Resource; a tela acha para OFERECER — ela precisa saber, antes de qualquer
 * toque, que aquela captura parece uma referência e não uma tarefa.
 *
 * É exatamente o sintoma que originou isto: um link salvo só para consultar
 * aparecia na lista de coisas a fazer, sem nada distinguindo os dois.
 */
export function enderecoEm(texto: string): string | null {
  for (const bruto of texto.split(/\s+/)) {
    // A pontuação que cerca o link sai dos DOIS lados. Um link colado no fim de
    // uma frase leva o ponto junto — e um endereço com ponto no fim abre uma
    // página que não existe. Entre parênteses, ele nem seria reconhecido.
    const palavra = bruto.replace(/^[([<"']+/, "").replace(/[.,;:)\]>"']+$/, "");
    if (palavra.startsWith("http://") || palavra.startsWith("https://")) return palavra;
  }
  return null;
}

/**
 * `exemplo.com` — o domínio, para a linha dizer de onde é sem gastar a largura
 * de uma URL inteira.
 *
 * Falha em silêncio para endereço malformado: a captura continua legível pelo
 * texto, que é o que importa.
 */
export function dominioDe(url: string): string | null {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return null;
  }
}

/**
 * Texto do usuário que vai para um modelo.
 *
 * O nome de uma conta é digitado por uma pessoa — ou por um webhook, ou pelo
 * próprio Hermes em outra conversa — e desce no prompt. Ele é DADO citado,
 * nunca instrução: sem cerca de código (que o M/OS lê como bloco de ação), sem
 * caractere de controle, sem colchete de seção, e curto o bastante para não
 * carregar um parágrafo inteiro de "ignore as instruções anteriores".
 */
export const LABEL_MAX = 60;

export function sanitizeLabel(value: string | null | undefined, max = LABEL_MAX) {
  const cleaned = (value ?? "")
    .replace(/[\u0000-\u001f\u007f]/g, " ")
    .replace(/`{3,}/g, "'")
    .replace(/[[\]{}<>]/g, "")
    .replace(/\s+/g, " ")
    .trim();
  if (cleaned.length <= max) return cleaned;
  return `${cleaned.slice(0, max - 1).trimEnd()}…`;
}

/** Para comparar nomes: sem acento, sem caixa, sem pontuação. */
export function normalizeForMatch(value: string) {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLocaleLowerCase("pt-BR")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/** Os primeiros 8 caracteres de um uuid — o que cabe numa linha de prompt. */
export function shortId(id: string) {
  return id.slice(0, 8);
}

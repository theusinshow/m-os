/**
 * Janela deslizante em memória, por chave.
 *
 * Melhor esforço, e dito assim: na Vercel cada instância tem a sua memória, e
 * uma instância fria começa zerada. Não é defesa contra ataque distribuído —
 * é o freio para um laço do Hermes pedindo a mesma ferramenta sem parar.
 */
export const RATE_LIMIT_PER_MINUTE = 60;

const windows = new Map<string, number[]>();

export function allowRequest(key: string, now = Date.now(), limit = RATE_LIMIT_PER_MINUTE) {
  const since = now - 60_000;
  const recent = (windows.get(key) ?? []).filter((at) => at > since);
  if (recent.length >= limit) {
    windows.set(key, recent);
    return false;
  }
  recent.push(now);
  windows.set(key, recent);
  return true;
}

export function resetRateLimitForTests() {
  windows.clear();
}

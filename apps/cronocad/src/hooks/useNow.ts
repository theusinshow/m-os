import { useEffect, useState } from "react";

/**
 * Retorna o instante atual (ms) e o atualiza no intervalo informado.
 *
 * Usado apenas para atualizar a exibicao do cronometro. A duracao real e
 * derivada de timestamps persistidos (a fonte da verdade e o backend), de
 * modo que este "tick" nao acumula erro: cada render recalcula a partir do
 * timestamp, nao de um contador incremental.
 */
export function useNow(intervalMs = 1000): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);

  return now;
}

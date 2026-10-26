/**
 * Decide se a aba recarrega para pegar a versão nova (ADR-076).
 *
 * Recarrega quando o servidor serve outra versão — uma vez por versão: se
 * depois do reload a aba continuar velha (CDN ainda propagando), ela não entra
 * num laço de recarregar; a próxima versão publicada tenta de novo.
 */
export function shouldReload({
  running,
  served,
  alreadyReloadedFor,
}: {
  running: string;
  served: string | null;
  alreadyReloadedFor: string | null;
}) {
  if (!served || running === "dev" || served === "dev") return false;
  if (served === running) return false;
  return alreadyReloadedFor !== served;
}

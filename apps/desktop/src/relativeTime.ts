/**
 * "agora", "há 9 minutos", "ontem".
 *
 * Mora fora do `App.tsx` porque a pagina de Settings extraida tambem precisa
 * dela — e importa-la de um arquivo que importa o Settings seria um ciclo. E o
 * mesmo motivo do `functionLabels.ts`.
 */
const relativeFormatter = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

export function relativeTime(value: string) {
  const seconds = Math.round((new Date(value).getTime() - Date.now()) / 1_000);
  if (Math.abs(seconds) < 60) return "agora";
  const minutes = Math.round(seconds / 60);
  if (Math.abs(minutes) < 60) return relativeFormatter.format(minutes, "minute");
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 24) return relativeFormatter.format(hours, "hour");
  return relativeFormatter.format(Math.round(hours / 24), "day");
}

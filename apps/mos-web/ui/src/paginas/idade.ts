/**
 * Há quanto tempo isto chegou.
 *
 * "há 3 min", "ontem". O relógio exato não ajuda a decidir nada aqui: a pergunta
 * que se faz diante de uma captura parada é *ainda importa?*, e a resposta está
 * na distância, não no horário.
 *
 * Morava dentro da página de Inbox, que deixou de existir quando Inbox e Tasks
 * viraram FAZER. Aqui não pertence a tela nenhuma.
 */
export function idade(iso: string, agora: Date = new Date()): string {
  const momento = new Date(iso).getTime();
  if (Number.isNaN(momento)) return "";
  const minutos = Math.round((agora.getTime() - momento) / 60_000);
  if (minutos < 1) return "agora";
  if (minutos < 60) return `há ${minutos} min`;
  const horas = Math.round(minutos / 60);
  if (horas < 24) return `há ${horas} h`;
  const dias = Math.round(horas / 24);
  return dias === 1 ? "ontem" : `há ${dias} dias`;
}

import type { CalendarItem } from "./types";

/**
 * O dia, decidido no fuso de quem está olhando.
 *
 * Todo este arquivo existe por causa de uma coisa: o banco guarda UTC, o
 * usuário trabalha de madrugada, e uma sessão iniciada às 23:31 do dia 30 é
 * `2026-07-31T02:31:00Z`. Qualquer agrupamento que olhe o TEXTO da data UTC a
 * põe no dia 31 — e a grade mostra horas num dia em que ninguém trabalhou, sem
 * nada quebrar e sem nenhum erro aparecer.
 *
 * O construtor `new Date(ano, mês, dia)` é local por definição, e é ele que
 * mantém a conta honesta. `toISOString` nunca entra aqui.
 */
export function startOfLocalDay(moment: Date) {
  return new Date(moment.getFullYear(), moment.getMonth(), moment.getDate());
}

/**
 * As 42 células de um mês: seis semanas, começando na SEGUNDA.
 *
 * Sempre 42, e não o mínimo necessário. Uma grade que muda de altura conforme o
 * mês faz o conteúdo abaixo dela pular a cada navegação — e navegar entre meses
 * é a ação mais frequente da tela.
 *
 * Segunda como primeiro dia porque é assim no resto do M/OS: `WeekRings` e
 * `MonthDensity` já leem a semana de trabalho, e um calendário que começasse no
 * domingo desalinharia a leitura entre as três telas.
 */
export function monthGrid(reference: Date) {
  const first = new Date(reference.getFullYear(), reference.getMonth(), 1);
  const weekday = (first.getDay() + 6) % 7;
  const start = new Date(first);
  start.setDate(first.getDate() - weekday);

  return Array.from({ length: 42 }, (_, index) => {
    const day = new Date(start);
    day.setDate(start.getDate() + index);
    return day;
  });
}

/**
 * Os itens por dia local.
 *
 * A chave é o instante da meia-noite local do dia — um número, e não um texto,
 * porque texto de data convida a formatá-lo em algum lugar e formatar é
 * exatamente onde o fuso volta a errar.
 */
export function groupByLocalDay(items: CalendarItem[]) {
  const days = new Map<number, CalendarItem[]>();
  for (const item of items) {
    const key = startOfLocalDay(new Date(item.at)).getTime();
    const bucket = days.get(key);
    if (bucket) {
      bucket.push(item);
    } else {
      days.set(key, [item]);
    }
  }
  return days;
}

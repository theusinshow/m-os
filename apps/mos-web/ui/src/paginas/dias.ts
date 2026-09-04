import type { ItemDaAgenda } from "../api";

/** Um dia da agenda, com o que aconteceu nele. */
export type DiaDaAgenda = {
  /** `2026-09-04`, no fuso de quem está olhando. É a chave da seção. */
  data: string;
  /** `HOJE`, `AMANHÃ`, ou `QUI · 11 DE SET`. */
  titulo: string;
  /** Segundos faturáveis somados no dia. Zero: o dia não teve hora. */
  segundos: number;
  itens: ItemDaAgenda[];
};

/**
 * Agrupa os itens por dia, no fuso do aparelho.
 *
 * O servidor devolve instantes; onde um dia começa é decisão de quem olha. A
 * mesma razão que faz o corte da semana vir do aparelho no panorama.
 */
export function porDia(itens: ItemDaAgenda[], hoje: Date = new Date()): DiaDaAgenda[] {
  const dias = new Map<string, DiaDaAgenda>();

  for (const item of itens) {
    const quando = new Date(item.at);
    const data = diaLocal(quando);
    let dia = dias.get(data);
    if (!dia) {
      dia = { data, titulo: tituloDoDia(quando, hoje), segundos: 0, itens: [] };
      dias.set(data, dia);
    }
    dia.itens.push(item);
    // Só a sessão de trabalho soma: uma prova marcada tem `seconds` zero, e uma
    // captura não é tempo trabalhado. Somar tudo daria um "total do dia" que não
    // corresponde a nada que se cobre.
    if (item.kind === "session") dia.segundos += item.seconds;
  }

  return [...dias.values()].sort((um, outro) => um.data.localeCompare(outro.data));
}

/** `2026-09-04` no fuso local — e não `toISOString`, que devolve UTC. */
function diaLocal(quando: Date): string {
  const mes = String(quando.getMonth() + 1).padStart(2, "0");
  const dia = String(quando.getDate()).padStart(2, "0");
  return `${quando.getFullYear()}-${mes}-${dia}`;
}

const DIAS = ["DOM", "SEG", "TER", "QUA", "QUI", "SEX", "SÁB"];
const MESES = [
  "JAN", "FEV", "MAR", "ABR", "MAI", "JUN",
  "JUL", "AGO", "SET", "OUT", "NOV", "DEZ",
];

/**
 * `HOJE`, `AMANHÃ`, `ONTEM`, ou `QUI · 11 DE SET`.
 *
 * As três palavras existem porque são as que se lê sem contar: numa agenda de
 * sete dias, "QUI · 04 DE SET" obriga a comparar com o calendário mental antes
 * de saber se é hoje.
 */
export function tituloDoDia(quando: Date, hoje: Date): string {
  const distancia = Math.round(
    (new Date(diaLocal(quando)).getTime() - new Date(diaLocal(hoje)).getTime()) / 86_400_000,
  );
  if (distancia === 0) return "HOJE";
  if (distancia === 1) return "AMANHÃ";
  if (distancia === -1) return "ONTEM";
  return `${DIAS[quando.getDay()]} · ${quando.getDate()} DE ${MESES[quando.getMonth()]}`;
}

/**
 * O que cobra atenção, e o que é registro do que já passou.
 *
 * Prova e prazo são compromissos que ainda vão acontecer, e é por eles que se
 * abre a agenda. Hora trabalhada, captura e task feita são o rastro — importam,
 * e não competem pelo olho.
 */
export function cobraAtencao(item: ItemDaAgenda): boolean {
  return (
    item.kind === "exam_scheduled" ||
    item.kind === "assignment_due" ||
    item.kind === "academic_planned"
  );
}

/** `14:00`, ou `dia` quando o item não marca hora. */
export function horaDoItem(item: ItemDaAgenda): string {
  const quando = new Date(item.at);
  // Meia-noite em ponto é como o domínio grava prazo sem hora marcada. Mostrar
  // "00:00" faria um prazo do dia parecer um compromisso de madrugada.
  if (quando.getHours() === 0 && quando.getMinutes() === 0) return "dia";
  return `${String(quando.getHours()).padStart(2, "0")}:${String(quando.getMinutes()).padStart(2, "0")}`;
}

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

/** Uma célula da grade do mês. */
export type CelulaDoMes = {
  /** O dia do mês, ou 0 para a folga antes do dia 1. */
  numero: number;
  /** `2026-09-04`, para casar com a seção da lista. Vazio na folga. */
  data: string;
  /** Quantos registros caíram no dia. A grade desenha até três pontos. */
  registros: number;
  /** Há prova, prazo ou bloco planejado nesse dia. */
  cobra: boolean;
  hoje: boolean;
};

/**
 * O mês inteiro em células, com a folga do começo.
 *
 * # Por que a semana começa na segunda
 *
 * É como o M/OS conta horas em toda parte — a semana de trabalho, e não a
 * semana do calendário de parede. Um mês que começasse no domingo poria o
 * sábado e o domingo em pontas opostas da linha, e é justamente o par que se
 * lê junto quando se procura o fim de semana.
 *
 * A folga do começo são células vazias, e não os dias do mês anterior: dia de
 * outro mês numa grade que diz "Setembro" é um convite a tocar no que não está
 * ali.
 */
export function porMes(itens: ItemDaAgenda[], mes: Date, hoje: Date = new Date()): CelulaDoMes[] {
  const registros = new Map<string, number>();
  const cobram = new Set<string>();
  for (const item of itens) {
    const data = diaLocal(new Date(item.at));
    registros.set(data, (registros.get(data) ?? 0) + 1);
    if (cobraAtencao(item)) cobram.add(data);
  }

  const primeiro = new Date(mes.getFullYear(), mes.getMonth(), 1);
  const ultimo = new Date(mes.getFullYear(), mes.getMonth() + 1, 0).getDate();
  const folga = (primeiro.getDay() + 6) % 7;

  const celulas: CelulaDoMes[] = [];
  for (let i = 0; i < folga; i += 1) {
    celulas.push({ numero: 0, data: "", registros: 0, cobra: false, hoje: false });
  }
  for (let numero = 1; numero <= ultimo; numero += 1) {
    const quando = new Date(mes.getFullYear(), mes.getMonth(), numero);
    const data = diaLocal(quando);
    celulas.push({
      numero,
      data,
      registros: registros.get(data) ?? 0,
      cobra: cobram.has(data),
      hoje: data === diaLocal(hoje),
    });
  }
  return celulas;
}

/** Os nove dias que a faixa da lista mostra: de ontem a uma semana. */
export function faixaDeDias(hoje: Date, itens: ItemDaAgenda[]): CelulaDoMes[] {
  const registros = new Map<string, number>();
  const cobram = new Set<string>();
  for (const item of itens) {
    const data = diaLocal(new Date(item.at));
    registros.set(data, (registros.get(data) ?? 0) + 1);
    if (cobraAtencao(item)) cobram.add(data);
  }
  const faixa: CelulaDoMes[] = [];
  for (let passo = -1; passo <= 7; passo += 1) {
    const quando = new Date(hoje.getFullYear(), hoje.getMonth(), hoje.getDate() + passo);
    const data = diaLocal(quando);
    faixa.push({
      numero: quando.getDate(),
      data,
      registros: registros.get(data) ?? 0,
      cobra: cobram.has(data),
      hoje: passo === 0,
    });
  }
  return faixa;
}

/** `SEG`, `TER`… para a faixa e para o cabeçalho da grade. */
export function siglaDoDia(quando: Date): string {
  return DIAS[quando.getDay()];
}

/** `Setembro`, com inicial maiúscula — é título, e não rótulo de sistema. */
export function nomeDoMes(quando: Date): string {
  const nomes = [
    "Janeiro", "Fevereiro", "Março", "Abril", "Maio", "Junho",
    "Julho", "Agosto", "Setembro", "Outubro", "Novembro", "Dezembro",
  ];
  return nomes[quando.getMonth()];
}

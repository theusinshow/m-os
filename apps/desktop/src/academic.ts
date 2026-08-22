/**
 * O M/Academic do lado da tela: só o que dá para verificar.
 *
 * Mesma divisão do `daily.ts`, do `weekly.ts` e do `stale.ts`, e pelo mesmo
 * motivo: não há teste de DOM neste repositório (`vitest.config.ts`), então
 * tudo que decide alguma coisa — como um prazo vira frase, como os
 * compromissos se agrupam, quando o botão de estudar aparece — mora aqui, e o
 * componente só desenha o resultado.
 *
 * **Nenhuma regra de domínio.** O que é "chegando", como a média pondera peso e
 * escala, o que conta como atraso e qual é o semestre corrente vivem em
 * `mos-core::academic`, com teste. Aqui é apresentação.
 */
import type { Compromisso, Horizonte, SubjectOverview } from "./types";

/**
 * As faixas na ordem em que a tela as mostra.
 *
 * NOW / SOON / LATER, e não uma lista corrida de datas: o §23 do pedido é
 * explícito, e a diferença prática é que "amanhã" e "daqui a três semanas"
 * param de competir pelo mesmo espaço visual.
 */
export const FAIXAS: { horizonte: Horizonte; titulo: string }[] = [
  { horizonte: "overdue", titulo: "Atrasado" },
  { horizonte: "today", titulo: "Hoje" },
  { horizonte: "tomorrow", titulo: "Amanhã" },
  { horizonte: "this_week", titulo: "Esta semana" },
  { horizonte: "later", titulo: "Depois" },
];

export type FaixaDeCompromissos = {
  horizonte: Horizonte;
  titulo: string;
  itens: Compromisso[];
};

/**
 * Agrupa os compromissos nas faixas, na ordem da urgência.
 *
 * **Faixa vazia não vira rótulo.** Um "Atrasado" com nada embaixo ensina a
 * ignorar o título, e no dia em que houver algo ali ele já terá virado ruído —
 * é a mesma regra que a Weekly Review seguiu.
 */
export function faixasDe(compromissos: Compromisso[]): FaixaDeCompromissos[] {
  return FAIXAS.map((faixa) => ({
    ...faixa,
    itens: compromissos.filter((item) => item.horizonte === faixa.horizonte),
  })).filter((faixa) => faixa.itens.length > 0);
}

/**
 * "hoje, 23:59", "amanhã, 14h", "sex, 29/08", "29/08/2027".
 *
 * O formato encurta conforme a distância: para hoje e amanhã o que importa é a
 * HORA, e repetir a data seria dizer o que a faixa já disse. Para depois, o dia
 * da semana ajuda a situar sem contar nos dedos.
 */
export function quandoDe(iso: string, horizonte: Horizonte, agora = new Date()): string {
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return "";

  const hora = new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(quando);
  const dataCurta = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "2-digit" }).format(quando);

  if (horizonte === "today") return hora;
  if (horizonte === "tomorrow") return `amanhã, ${hora}`;
  if (horizonte === "overdue") {
    const dias = Math.floor((agora.getTime() - quando.getTime()) / 86_400_000);
    if (dias <= 0) return `venceu às ${hora}`;
    return dias === 1 ? "venceu ontem" : `venceu há ${dias} dias`;
  }
  if (horizonte === "this_week") {
    const semana = new Intl.DateTimeFormat("pt-BR", { weekday: "short" })
      .format(quando)
      .replace(".", "")
      .replace("-feira", "");
    return `${semana}, ${dataCurta}`;
  }
  // Ano só quando ele muda: "29/08/2026" no mesmo ano é ruído.
  return quando.getFullYear() === agora.getFullYear()
    ? dataCurta
    : new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "2-digit", year: "numeric" }).format(quando);
}

/**
 * "1h 45min", "45min", "—".
 *
 * Nunca "0h 45min": a hora zerada ocupa espaço para não dizer nada, e a lista
 * de disciplinas tem quatro delas lado a lado.
 */
export function duracaoDe(segundos: number): string {
  if (!Number.isFinite(segundos) || segundos < 60) return "—";
  const minutos = Math.floor(segundos / 60);
  const horas = Math.floor(minutos / 60);
  const resto = minutos % 60;
  if (!horas) return `${resto}min`;
  return resto ? `${horas}h ${resto}min` : `${horas}h`;
}

/**
 * A média com uma casa, ou vazio quando não há nota nenhuma.
 *
 * Vazio e não "0,0": zero é uma nota, e uma disciplina sem prova corrigida não
 * tirou zero — ela não tirou nada. Confundir os dois é o erro que faria o
 * painel anunciar reprovação em março.
 */
export function mediaDe(media: number | null): string {
  if (media === null || !Number.isFinite(media)) return "";
  return media.toFixed(1).replace(".", ",");
}

/** "3 de 5 avaliações já valem nota" vira "60% avaliado". */
export function avaliadoDe(peso: number | null): string {
  if (peso === null || !Number.isFinite(peso)) return "";
  return `${Math.round(peso * 100)}% avaliado`;
}

/**
 * A frase que resume o que a disciplina está pedindo.
 *
 * Uma frase e não três números soltos: "3 pendentes · 1 atrasada · 1 prova" faz
 * o olho somar; "1 atrasada" diz o que fazer. O pior estado ganha a frase.
 */
export function situacaoDe(subject: SubjectOverview): string {
  if (subject.overdue) {
    return subject.overdue === 1 ? "1 atrasada" : `${subject.overdue} atrasadas`;
  }
  if (subject.pending) {
    return subject.pending === 1 ? "1 pendente" : `${subject.pending} pendentes`;
  }
  if (subject.upcomingExams) {
    return subject.upcomingExams === 1 ? "1 avaliação marcada" : `${subject.upcomingExams} avaliações marcadas`;
  }
  return "em dia";
}

/**
 * Quanto tempo uma sessão em curso já acumulou, em segundos.
 *
 * O componente chama isto a cada segundo com o `Date.now()` dele. A conta mora
 * aqui para o teste poder fixar os dois instantes.
 */
export function decorridoDe(startedAt: string, agora = new Date()): number {
  const inicio = new Date(startedAt);
  if (Number.isNaN(inicio.getTime())) return 0;
  return Math.max(0, Math.floor((agora.getTime() - inicio.getTime()) / 1000));
}

/** "00:45:12" — o cronômetro rodando. */
export function cronometroDe(segundos: number): string {
  const seguro = Math.max(0, Math.floor(segundos));
  const horas = Math.floor(seguro / 3600);
  const minutos = Math.floor((seguro % 3600) / 60);
  const resto = seguro % 60;
  const dois = (valor: number) => String(valor).padStart(2, "0");
  return `${dois(horas)}:${dois(minutos)}:${dois(resto)}`;
}

/**
 * O `datetime-local` do formulário para o instante que o backend guarda.
 *
 * O campo entrega `2026-08-29T23:59` SEM fuso, e interpretá-lo como UTC jogaria
 * uma entrega das 23h59 para o dia seguinte. `new Date(ano, mes, ...)` monta no
 * fuso local, que é o que a pessoa digitou.
 */
export function instanteDoCampo(valor: string): string | null {
  if (!valor.trim()) return null;
  const [data, hora = "00:00"] = valor.split("T");
  const [ano, mes, dia] = data.split("-").map(Number);
  const [h, m] = hora.split(":").map(Number);
  if (!ano || !mes || !dia) return null;
  const quando = new Date(ano, mes - 1, dia, h || 0, m || 0);
  return Number.isNaN(quando.getTime()) ? null : quando.toISOString();
}

/** O caminho de volta: o instante guardado vira valor de `datetime-local`. */
export function campoDoInstante(iso: string | null): string {
  if (!iso) return "";
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return "";
  const dois = (valor: number) => String(valor).padStart(2, "0");
  return `${quando.getFullYear()}-${dois(quando.getMonth() + 1)}-${dois(quando.getDate())}T${dois(quando.getHours())}:${dois(quando.getMinutes())}`;
}

/** Os rótulos que a tela usa para os estados. Um lugar só. */
export const STATUS_ATIVIDADE: Record<string, string> = {
  pending: "Pendente",
  in_progress: "Em andamento",
  submitted: "Entregue",
  graded: "Corrigida",
  cancelled: "Cancelada",
};

export const STATUS_AVALIACAO: Record<string, string> = {
  scheduled: "Marcada",
  done: "Feita",
  graded: "Corrigida",
  cancelled: "Cancelada",
};

export const STATUS_SEMESTRE: Record<string, string> = {
  upcoming: "A começar",
  active: "Em curso",
  completed: "Concluído",
};

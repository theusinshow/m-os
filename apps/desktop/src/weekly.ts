/**
 * A Weekly Review do lado da tela: só o que dá para verificar.
 *
 * Mesma divisão do `daily.ts`, e pelo mesmo motivo: não há teste de DOM neste
 * repositório (`vitest.config.ts`), então tudo que decide alguma coisa — que
 * seção aparece, como um número vira frase, quando o fecho é oferecido — mora
 * aqui, e o componente só desenha o resultado.
 *
 * **Nenhuma regra de domínio.** O que a semana é, o que dominou e o que se
 * repetiu vivem em `mos-core::weekly`, com teste. Aqui é apresentação.
 */
import type { Day, Week, WeekSummary } from "./types";

/** Uma linha de seção: o assunto, e o número que é o assunto. */
export type LinhaDaSemana = { texto: string; detalhe: string };

export type SecaoDaSemana = { chave: string; titulo: string; linhas: LinhaDaSemana[] };

/**
 * `AAAA-MM-DD` para `Date` local, **sem passar por `new Date(texto)`**.
 *
 * `new Date("2026-08-19")` é lido como meia-noite UTC, e num fuso negativo volta
 * como o dia 18 — a quarta viraria terça. É o mesmo cuidado do `daily.ts`.
 */
function dataLocal(dia: string): Date | null {
  const [ano, mes, data] = dia.split("-").map(Number);
  if (!ano || !mes || !data) return null;
  const resolvida = new Date(ano, mes - 1, data);
  return Number.isNaN(resolvida.getTime()) ? null : resolvida;
}

/**
 * "17 a 23 de agosto", "28 de setembro a 4 de outubro".
 *
 * O rótulo diz o **intervalo**, e nunca "a semana passada": quem passa duas
 * semanas sem abrir o M/OS vê a linha apontando para a semana retrasada, e
 * "passada" seria mentira.
 */
export function rotuloDaSemana(week: Week, locale = "pt-BR"): string {
  const inicio = dataLocal(week);
  if (!inicio) return week;
  const fim = new Date(inicio);
  fim.setDate(fim.getDate() + 6);

  const mesDe = (quando: Date) => new Intl.DateTimeFormat(locale, { month: "long" }).format(quando);

  if (inicio.getFullYear() !== fim.getFullYear()) {
    return `${inicio.getDate()} de ${mesDe(inicio)} de ${inicio.getFullYear()} a ${fim.getDate()} de ${mesDe(fim)} de ${fim.getFullYear()}`;
  }
  if (inicio.getMonth() !== fim.getMonth()) {
    return `${inicio.getDate()} de ${mesDe(inicio)} a ${fim.getDate()} de ${mesDe(fim)}`;
  }
  return `${inicio.getDate()} a ${fim.getDate()} de ${mesDe(fim)}`;
}

/** "seg", "qua". Vazio quando a data não resolve. */
export function diaDaSemanaCurto(day: Day, locale = "pt-BR"): string {
  const quando = dataLocal(day);
  if (!quando) return "";
  return new Intl.DateTimeFormat(locale, { weekday: "short" })
    .format(quando)
    .replace(".", "")
    .toLowerCase();
}

/** A semana seguinte ou a anterior, sem passar por `new Date(texto)`. */
export function semanaVizinha(week: Week, direcao: -1 | 1): Week {
  const quando = dataLocal(week);
  if (!quando) return week;
  quando.setDate(quando.getDate() + direcao * 7);
  return [
    quando.getFullYear(),
    String(quando.getMonth() + 1).padStart(2, "0"),
    String(quando.getDate()).padStart(2, "0"),
  ].join("-");
}

function plural(valor: number, singular: string, muitos: string): string {
  return `${valor} ${valor === 1 ? singular : muitos}`;
}

/**
 * As seções da semana, na ordem da leitura.
 *
 * **Seção vazia não vira rótulo.** Uma semana sem nada largado não deve mostrar
 * "O QUE VOCÊ LARGOU" seguido de vazio — é a mesma regra do `resumoDoDia`, onde
 * zero não vira linha.
 */
export function secoesDaSemana(resumo: WeekSummary): SecaoDaSemana[] {
  const secoes: SecaoDaSemana[] = [
    {
      chave: "dominated",
      titulo: "O QUE DOMINOU",
      linhas: resumo.dominated.map((item) => ({
        texto: item.label,
        // "principal em 0 dias" seria uma frase que informa o contrário do que
        // parece. Quem apareceu sem nunca ser principal diz só em quantos dias.
        detalhe: item.mainDays
          ? `principal em ${plural(item.mainDays, "dia", "dias")}`
          : `em ${plural(item.days, "dia", "dias")}`,
      })),
    },
    {
      chave: "recurring",
      titulo: "O QUE VOLTOU TODA VEZ",
      linhas: resumo.recurring.map((item) => ({
        texto: item.title,
        detalhe: `carregado ${plural(item.timesCarried, "vez", "vezes")}`,
      })),
    },
    {
      chave: "dropped",
      titulo: "O QUE VOCÊ LARGOU",
      linhas: resumo.dropped.map((titulo) => ({ texto: titulo, detalhe: "" })),
    },
    {
      chave: "blocked",
      titulo: "DIAS TRAVADOS",
      // Uma linha só, e não uma por dia. Cada dia é uma palavra de três letras;
      // empilhá-los gastaria cinco linhas de altura para dizer o que cabe em
      // uma — e foi assim que apareceu na primeira foto da janela.
      linhas: resumo.blockedDays.length
        ? [{ texto: resumo.blockedDays.map((dia) => diaDaSemanaCurto(dia)).filter(Boolean).join(", "), detalhe: "" }]
        : [],
    },
  ];
  return secoes.filter((secao) => secao.linhas.length > 0);
}

/**
 * A semana pode ser fechada?
 *
 * Semana sem sessão nenhuma **não** oferece o fecho: não há o que revisar, e um
 * botão ali ensinaria que o M/OS quer um registro por semana mesmo quando não
 * houve semana — que é a carga de organização que o `VISION.md` §14 proíbe
 * criar.
 *
 * Semana já fechada continua podendo: o botão vira "Salvar", e corrigir o texto
 * é a única mudança possível num registro.
 */
export function podeFechar(resumo: WeekSummary): boolean {
  return !resumo.empty;
}

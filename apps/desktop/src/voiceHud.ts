import type { VoiceResult } from "./api";

/**
 * As funções puras do HUD de voz.
 *
 * Elas vivem fora do componente porque são as únicas coisas dele que podem
 * estar erradas de um jeito que um teste pega: quanto cada traço de amplitude
 * cresce, como o relógio lê, e o que o recibo diz. O resto é tela — e
 * `vitest.config.ts` é explícito sobre não fingir testá-la.
 */

/** Quantos traços a amplitude tem. Espelha o markup do Quick Capture. */
export const AMPLITUDE_BARS = 4;

/**
 * A altura de cada traço, de 0 a 1.
 *
 * O RMS chega em `0..1000` e é comprimido por raiz quadrada: a energia sonora
 * cresce rápido demais para uma escala linear, e sem a compressão os traços
 * ficariam colados no chão em fala normal e estourariam num "ãh" mais alto.
 *
 * Cada traço tem um piso diferente e uma resposta diferente, e é isso que faz
 * a coisa ler como forma de onda em vez de barra de progresso: quatro barras
 * subindo juntas na mesma altura são um medidor, não uma voz.
 */
export function amplitudeScale(level: number, index: number): number {
  const normalized = Math.sqrt(Math.max(0, Math.min(1000, level)) / 1000);
  // Pesos fixos, e não aleatórios: um traço que sorteia a própria altura a cada
  // quadro treme mesmo com a voz parada, e passaria a mentir sobre o sinal.
  const weights = [0.55, 1, 0.8, 0.65];
  const weight = weights[index % weights.length];
  // O piso de 0.08 é o traço em repouso — ele nunca some, porque um traço que
  // desaparece durante uma pausa parece um microfone que caiu.
  return Math.max(0.08, Math.min(1, normalized * weight));
}

/** O relógio da gravação. `0:04`, e não `4s`: ele conta, não mede. */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

/**
 * Quanto falta do teto, para o HUD avisar antes de cortar.
 *
 * Devolve `null` enquanto o fim está longe. O aviso só existe nos últimos dez
 * segundos porque antes disso ele seria pressão sobre alguém que está pensando.
 */
export function remainingWarning(ms: number, capMs: number): string | null {
  const left = Math.ceil((capMs - ms) / 1000);
  if (left > 10 || left < 0) return null;
  return `${left}s`;
}

const WEEKDAYS = ["domingo", "segunda", "terça", "quarta", "quinta", "sexta", "sábado"];

/**
 * O prazo, como ele deve ser LIDO — e não como foi dito.
 *
 * As duas coisas convivem no recibo de propósito. O texto falado prova que o
 * M/OS entendeu a frase; a data resolvida prova que ele entendeu o dia. Só a
 * primeira deixaria "sexta" ambíguo; só a segunda esconderia um erro de
 * interpretação atrás de um carimbo que parece certo.
 */
export function formatWhen(iso: string, now: Date): string {
  const when = new Date(iso);
  if (Number.isNaN(when.getTime())) return "";
  const hours = `${String(when.getHours()).padStart(2, "0")}:${String(when.getMinutes()).padStart(2, "0")}`;

  const dayStart = (date: Date) => new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const days = Math.round((dayStart(when) - dayStart(now)) / 86_400_000);

  if (days === 0) return `Hoje · ${hours}`;
  if (days === 1) return `Amanhã · ${hours}`;
  // Dentro da semana o dia da semana diz mais que a data: "sexta" localiza sem
  // que ninguém precise contar.
  if (days > 1 && days < 7) return `${WEEKDAYS[when.getDay()]} · ${hours}`;
  return `${String(when.getDate()).padStart(2, "0")}/${String(when.getMonth() + 1).padStart(2, "0")} · ${hours}`;
}

export type Receipt = {
  /** A linha de estado, em micro-label. */
  headline: string;
  /** O que ficou registrado. É o título da Task, ou a fala. */
  subject: string;
  /** Prazo e Project, já formatados. Vazio quando não há nem um nem outro. */
  meta: string;
  /** O texto do convite, quando há um a fazer. `null` quando não há. */
  offer: string | null;
};

/**
 * O que o HUD mostra depois de entender.
 *
 * Três desfechos, e a diferença entre eles é AUTORIZAÇÃO e não certeza:
 *
 * - **executou** — a confiança era alta e o M/OS agiu. O recibo diz o que fez, e
 *   oferece o caminho de volta;
 * - **oferece** — a confiança era média. A Capture já está salva; o convite é
 *   por ⏎ e ignorá-lo é uma resposta válida, não um erro;
 * - **guardou** — foi só uma Capture, e isso não é falha. É o §19 do brief: o
 *   que não foi compreendido termina em segurança na Inbox.
 */
export function receiptOf(result: VoiceResult, now: Date): Receipt {
  const partes: string[] = [];
  if (result.when) partes.push(formatWhen(result.when, now));
  if (result.projectName) {
    // O Project que veio da tela e não da fala é marcado. Ele foi um palpite do
    // contexto, e um palpite que não se anuncia é indistinguível de uma
    // afirmação.
    partes.push(result.projectFromContext ? `${result.projectName} (contexto)` : result.projectName);
  }
  const meta = partes.join(" · ");

  if (result.executed) {
    return {
      headline: result.reminderId ? "LEMBRETE CRIADO" : "TASK CRIADA",
      subject: result.title,
      meta,
      offer: null,
    };
  }

  if (result.action !== "keep") {
    const verbo = result.action === "create_task_with_reminder" ? "Criar lembrete" : "Criar Task";
    return {
      headline: "CAPTURADO",
      subject: result.title,
      meta,
      offer: `${verbo} ⏎`,
    };
  }

  return {
    headline: "CAPTURADO",
    subject: result.transcript,
    meta: result.hedged ? "" : meta,
    offer: null,
  };
}

/**
 * A frase de uma recusa antes da transcrição.
 *
 * As duas não persistiram nada, e por isso nenhuma delas oferece desfazer ou
 * tentar de novo: não há o que desfazer, e tentar de novo é falar de novo.
 */
export function refusalLabel(outcome: "tooShort" | "tooQuiet"): string {
  return outcome === "tooShort" ? "Curto demais" : "Não ouvi nada";
}

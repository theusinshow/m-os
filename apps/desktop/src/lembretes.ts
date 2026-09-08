/**
 * As regras de tela do Attention Center — fora de qualquer componente React.
 *
 * O §66 do pedido é explícito: *não deixar business logic dentro de componentes
 * React*. O que mora aqui é o que decide **em que grupo** um lembrete cai, **que
 * adiamentos** fazem sentido às onze da noite e **como** o tempo se lê em
 * palavras. Nada disso precisa de DOM, e tudo isso precisa de teste.
 *
 * O que NÃO mora aqui, e a fronteira importa: quando um lembrete vence, se ele
 * insiste, se ele repete e se ele está sendo esquecido. Isso é do domínio, em
 * Rust, e chega pronto — `needsAttention` devolve os motivos junto. Recalcular
 * aqui seria a segunda implementação da mesma regra, e duas implementações
 * divergem.
 */

import type { AttentionReason, Reminder, ReminderTrigger } from "./types";

/** Os grupos do Reminder Center, na ordem em que se lê. */
export type Bucket = "attention" | "today" | "upcoming" | "snoozed" | "someday";

export const BUCKET_LABEL: Record<Bucket, string> = {
  attention: "Precisa de atenção",
  today: "Hoje",
  upcoming: "Em breve",
  snoozed: "Adiados",
  someday: "Algum dia",
};

/**
 * Em que grupo cada lembrete cai.
 *
 * `precisaDeAtencao` vem do backend — é a lista determinística do domínio, e
 * não uma segunda opinião calculada aqui. Passá-la como conjunto é o que impede
 * a tela de discordar do sistema sobre o que está sendo esquecido.
 */
export function bucketOf(
  reminder: Reminder,
  precisaDeAtencao: ReadonlySet<string>,
  now: number,
): Bucket {
  if (precisaDeAtencao.has(reminder.id)) return "attention";
  if (reminder.status === "snoozed") return "snoozed";
  if (!reminder.nextDueAt) return "someday";

  const due = Date.parse(reminder.nextDueAt);
  if (Number.isNaN(due)) return "someday";
  return due <= fimDoDia(now) ? "today" : "upcoming";
}

/** O último instante do dia de quem está olhando. */
function fimDoDia(now: number): number {
  const fim = new Date(now);
  fim.setHours(23, 59, 59, 999);
  return fim.getTime();
}

/**
 * Tempo em palavras, nunca só um número solto.
 *
 * `DESIGN-FOUNDATIONS.md` §14 exige que nenhum estado dependa apenas de cor, e a
 * mesma lógica vale para o tempo: "1h" sozinho não diz se falta ou se passou.
 */
export function whenLabel(reminder: Reminder, now: number): string {
  if (!reminder.nextDueAt) return "sem data";

  const due = Date.parse(reminder.nextDueAt);
  if (Number.isNaN(due)) return "";

  const span = emPalavras(Math.abs(due - now));
  if (reminder.status === "snoozed") return `adiado · volta em ${span}`;
  return due < now ? `atrasado ${span}` : `em ${span}`;
}

function emPalavras(millis: number): string {
  const minutes = Math.round(millis / 60000);
  if (minutes < 1) return "menos de um minuto";
  if (minutes < 60) return `${minutes} min`;
  if (minutes < 60 * 24) return `${Math.round(minutes / 60)} h`;
  return `${Math.round(minutes / (60 * 24))} d`;
}

/** Como cada motivo do domínio se lê na tela. */
export const REASON_LABEL: Record<AttentionReason, string> = {
  missed: "perdido",
  overdue: "atrasado",
  ignored: "ignorado",
  snooze_fatigue: "adiado demais",
  persistent: "não deixar esquecer",
  high_priority: "prioridade alta",
  carried_over: "veio de ontem",
};

/** Um adiamento oferecido, já com o instante resolvido. */
export type SnoozeChoice = { label: string; resolve: () => Date };

/**
 * Os adiamentos que fazem sentido AGORA.
 *
 * Contextuais de verdade (§10 do pedido): às onze da noite, "hoje à noite" não
 * quer dizer nada e "amanhã de manhã" quer dizer tudo. Uma lista fixa obrigaria
 * a pessoa a ler cinco opções para achar a única aplicável — e é a leitura, e
 * não o clique, que custa caro num gesto que se faz dez vezes por dia.
 *
 * `now` entra como parâmetro para o teste poder viajar no tempo sem mexer no
 * relógio da máquina.
 */
export function snoozeChoices(now: Date): SnoozeChoice[] {
  const hora = now.getHours();
  const escolhas: SnoozeChoice[] = [
    { label: "10 min", resolve: () => somar(now, 10) },
    { label: "1 hora", resolve: () => somar(now, 60) },
  ];

  // "Mais tarde hoje" só existe enquanto ainda houver hoje pela frente.
  if (hora < 19) {
    escolhas.push({ label: "Mais tarde", resolve: () => somar(now, 3 * 60) });
  }
  // "Hoje à noite" às 22h seria daqui a pouco, e às 23h seria ontem.
  if (hora < 17) {
    escolhas.push({ label: "À noite", resolve: () => emHoraLocal(now, 0, 20) });
  }
  escolhas.push({ label: "Amanhã 9h", resolve: () => emHoraLocal(now, 1, 9) });
  escolhas.push({ label: "Semana que vem", resolve: () => emHoraLocal(now, 7, 9) });
  return escolhas;
}

function somar(now: Date, minutos: number): Date {
  return new Date(now.getTime() + minutos * 60000);
}

/** O dia `dias` à frente, na `hora` local cheia. */
function emHoraLocal(now: Date, dias: number, hora: number): Date {
  const quando = new Date(now.getTime());
  quando.setDate(quando.getDate() + dias);
  quando.setHours(hora, 0, 0, 0);
  return quando;
}

/**
 * As opções de "quando" ao criar. Só o que ainda não passou.
 *
 * "Hoje 18h" às 19h não quer dizer nada, e o backend a recusaria de todo jeito.
 */
export function whenChoices(now: Date): SnoozeChoice[] {
  const todas: SnoozeChoice[] = [
    { label: "15 min", resolve: () => somar(now, 15) },
    { label: "1 hora", resolve: () => somar(now, 60) },
    { label: "3 horas", resolve: () => somar(now, 180) },
    { label: "Hoje 18h", resolve: () => emHoraLocal(now, 0, 18) },
    { label: "Amanhã 9h", resolve: () => emHoraLocal(now, 1, 9) },
    { label: "Segunda 9h", resolve: () => proximaSegunda(now) },
  ];
  return todas.filter((escolha) => escolha.resolve().getTime() > now.getTime());
}

function proximaSegunda(now: Date): Date {
  const quando = new Date(now.getTime());
  // 8 - dia da semana, com resto 7 quando hoje já é segunda: pedir "segunda"
  // numa segunda quer dizer a próxima, e não daqui a instante nenhum.
  const adiante = (8 - quando.getDay()) % 7 || 7;
  quando.setDate(quando.getDate() + adiante);
  quando.setHours(9, 0, 0, 0);
  return quando;
}

/**
 * Quantos alertas ainda vão tocar. É o "🔔 4 alertas" do §16.
 *
 * Conta só os pendentes: mostrar quatro quando três já tocaram diria à pessoa
 * que ela vai ser interrompida mais três vezes do que vai.
 */
export function alertasPendentes(triggers: ReminderTrigger[]): number {
  return triggers.filter(
    (trigger) => trigger.status === "pending" && trigger.lifecycleState === "active",
  ).length;
}

/** "1 dia antes", "no prazo". A mesma frase que o domínio escreve. */
export function triggerLabel(trigger: ReminderTrigger): string {
  if (trigger.kind === "at_due") return "no prazo";
  if (trigger.kind === "lead" && trigger.leadMinutes !== null) {
    return `${minutosEmPalavras(trigger.leadMinutes)} antes`;
  }
  return "alerta";
}

function minutosEmPalavras(minutos: number): string {
  if (minutos >= 1440 && minutos % 1440 === 0) {
    const dias = minutos / 1440;
    return dias === 1 ? "1 dia" : `${dias} dias`;
  }
  if (minutos >= 60 && minutos % 60 === 0) {
    const horas = minutos / 60;
    return horas === 1 ? "1 hora" : `${horas} horas`;
  }
  return `${minutos} min`;
}

/** O texto do botão principal, que muda com o tipo do lembrete. */
export function acaoPrincipal(reminder: Reminder): string {
  return reminder.kind === "follow_up" ? "Respondeu" : "Concluir";
}

/** A pergunta que o cartão faz. Um follow-up não se responde com "concluir". */
export function perguntaDoCartao(reminder: Reminder): string | null {
  if (reminder.kind !== "follow_up") return null;
  const quem = reminder.waitingFor.trim();
  return quem ? `${quem} respondeu?` : "Já respondeu?";
}

/** Como a regra de repetição se lê. Espelha `Recurrence::describe` do domínio. */
export function recurrenceLabel(reminder: Reminder): string | null {
  const regra = reminder.recurrence;
  if (!regra) return null;

  const hora = `${String(regra.hour).padStart(2, "0")}:${String(regra.minute).padStart(2, "0")}`;
  const dias = ["segunda", "terça", "quarta", "quinta", "sexta", "sábado", "domingo"];
  let base: string;
  switch (regra.rule.kind) {
    case "daily":
      base = "Todo dia";
      break;
    case "weekdays":
      base = "Todo dia útil";
      break;
    case "weekly":
      base = `Toda ${[...regra.rule.days].sort((a, b) => a - b).map((dia) => dias[dia] ?? "?").join(", ")}`;
      break;
    case "monthly":
      base = mensalEmPalavras(regra.rule.day, dias);
      break;
    case "yearly":
      base = `Todo dia ${regra.rule.day}/${regra.rule.month}`;
      break;
    case "everyDays":
      base = `A cada ${regra.rule.days} dias`;
      break;
    case "everyWeeks":
      base = `A cada ${regra.rule.weeks} semanas`;
      break;
  }
  const sufixo = regra.anchor === "completion" ? ", depois de concluir" : "";
  return `${base} às ${hora}${sufixo}`;
}

function mensalEmPalavras(dia: { kind: string } & Record<string, unknown>, dias: string[]): string {
  if (dia.kind === "day") return `Todo dia ${dia.day}`;
  if (dia.kind === "lastBusinessDay") return "Último dia útil do mês";
  const ordinais = ["", "Primeira", "Segunda", "Terceira", "Quarta", "Última"];
  const ordinal = ordinais[Number(dia.ordinal)] ?? "Última";
  return `${ordinal} ${dias[Number(dia.weekday)] ?? "?"} do mês`;
}

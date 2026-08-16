import type { ActivityType } from "./types";

/**
 * O vocabulário comum das telas de Tempo.
 *
 * Saiu do `TempoPage` quando a segunda tela precisou formatar as mesmas horas.
 * Duas implementações de "quanto isso dá em horas" divergiriam no dia em que uma
 * fosse corrigida, e o número que diverge aqui é o número que vai na fatura.
 */

/** `16,1 h` — a unidade do trabalho cobrado é a hora, e uma casa basta. */
export function hoursOf(seconds: number) {
  return `${(seconds / 3600).toFixed(1)} h`;
}

export function moneyOf(cents: number) {
  return (cents / 100).toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
}

/** `2h07` na linha da sessão: minutos importam quando se olha uma sessão só. */
export function durationOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h${String(minutes).padStart(2, "0")}` : `${minutes}min`;
}

export function dayOf(iso: string) {
  return new Date(iso).toLocaleDateString("pt-BR", { day: "2-digit", month: "short" });
}

/** `2026-08-16` no fuso LOCAL: `toISOString` devolve UTC e vira o dia errado à noite. */
export function dateInputOf(iso: string) {
  const moment = new Date(iso);
  const local = new Date(moment.getTime() - moment.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 10);
}

/** Monta o instante a partir do dia escolhido, preservando a hora original. */
export function momentOf(day: string, keepTimeFrom?: string) {
  const base = keepTimeFrom ? new Date(keepTimeFrom) : new Date();
  const [year, month, date] = day.split("-").map(Number);
  const moment = new Date(base);
  moment.setFullYear(year, month - 1, date);
  return moment.toISOString();
}

/** O primeiro instante do dia local, em ISO — a borda de todo filtro de período. */
export function startOfDay(day: string) {
  const [year, month, date] = day.split("-").map(Number);
  return new Date(year, month - 1, date, 0, 0, 0, 0).toISOString();
}

/** O último instante do dia local. Fim de período é inclusivo do dia inteiro. */
export function endOfDay(day: string) {
  const [year, month, date] = day.split("-").map(Number);
  return new Date(year, month - 1, date, 23, 59, 59, 999).toISOString();
}

export const ACTIVITY: { value: ActivityType; label: string }[] = [
  { value: "drawing", label: "desenho" },
  { value: "detailing", label: "detalhamento" },
  { value: "revision", label: "revisão" },
  { value: "meeting", label: "reunião" },
  { value: "study", label: "estudo" },
  { value: "other", label: "outro" },
];

export const ACTIVITY_LABEL: Record<string, string> =
  Object.fromEntries(ACTIVITY.map((item) => [item.value, item.label]));

export const SOURCE_LABEL: Record<string, string> = {
  timer: "cronômetro",
  manual: "manual",
  reconstructed: "reconstruída",
};

/** Os campos que o lançamento e a correção compartilham. */
export type Draft = {
  day: string;
  hours: number;
  minutes: number;
  description: string;
  activityType: ActivityType;
  billable: boolean;
};

export function emptyDraft(): Draft {
  return {
    day: dateInputOf(new Date().toISOString()),
    hours: 1,
    minutes: 0,
    description: "",
    activityType: "other",
    billable: true,
  };
}

export const secondsOf = (draft: Draft) => draft.hours * 3600 + draft.minutes * 60;

/**
 * Horas e minutos em campos separados, e não um texto livre.
 *
 * "1h30", "1:30" e "1,5" são a mesma duração escrita de três jeitos, e qualquer
 * parser que aceite os três vai errar um quarto. Dois números não têm ambiguidade
 * — e o teclado numérico do sistema já valida.
 */
export function DurationFields({ draft, onChange, idPrefix }: {
  draft: Draft;
  onChange: (next: Draft) => void;
  idPrefix: string;
}) {
  return (
    <div className="tempo-duration">
      <label htmlFor={`${idPrefix}-hours`}>Horas</label>
      <input
        id={`${idPrefix}-hours`}
        type="number"
        min={0}
        max={23}
        value={draft.hours}
        onChange={(event) => onChange({ ...draft, hours: Math.max(0, Number(event.currentTarget.value) || 0) })}
      />
      <label htmlFor={`${idPrefix}-minutes`}>Minutos</label>
      <input
        id={`${idPrefix}-minutes`}
        type="number"
        min={0}
        max={59}
        step={5}
        value={draft.minutes}
        onChange={(event) => onChange({ ...draft, minutes: Math.min(59, Math.max(0, Number(event.currentTarget.value) || 0)) })}
      />
    </div>
  );
}

export function DraftFields({ draft, onChange, idPrefix }: {
  draft: Draft;
  onChange: (next: Draft) => void;
  idPrefix: string;
}) {
  return (
    <>
      <div className="tempo-field">
        <label htmlFor={`${idPrefix}-day`}>Dia</label>
        <input
          id={`${idPrefix}-day`}
          type="date"
          value={draft.day}
          onChange={(event) => onChange({ ...draft, day: event.currentTarget.value })}
        />
      </div>
      <DurationFields draft={draft} onChange={onChange} idPrefix={idPrefix} />
      <div className="tempo-field">
        <label htmlFor={`${idPrefix}-activity`}>Atividade</label>
        <select
          id={`${idPrefix}-activity`}
          value={draft.activityType}
          onChange={(event) => onChange({ ...draft, activityType: event.currentTarget.value as ActivityType })}
        >
          {ACTIVITY.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
        </select>
      </div>
      <div className="tempo-field">
        <label htmlFor={`${idPrefix}-description`}>Descrição</label>
        <input
          id={`${idPrefix}-description`}
          value={draft.description}
          placeholder="opcional"
          onChange={(event) => onChange({ ...draft, description: event.currentTarget.value })}
        />
      </div>
      <label className="tempo-check">
        <input
          type="checkbox"
          checked={draft.billable}
          onChange={(event) => onChange({ ...draft, billable: event.currentTarget.checked })}
        />
        Cobrável
      </label>
    </>
  );
}

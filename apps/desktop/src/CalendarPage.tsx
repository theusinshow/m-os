import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { groupByLocalDay, monthGrid, startOfLocalDay } from "./calendarDays";
import { Card, ContextPath, EmptyState, PageHeader } from "./Surface";
import type { CalendarItem, CalendarKind } from "./types";

const WEEKDAYS = ["SEG", "TER", "QUA", "QUI", "SEX", "SÁB", "DOM"];

/** O que cada tipo quer dizer na tela. O nome técnico nunca aparece. */
const KIND_LABEL: Record<CalendarKind, string> = {
  session: "sessão",
  task_done: "Task concluída",
  task_created: "Task criada",
  capture: "capture",
  app_opened: "programa aberto",
};

/** `2h30` ou `45min`. */
function durationOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h${String(minutes).padStart(2, "0")}` : `${minutes}min`;
}

function clockOf(iso: string) {
  return new Date(iso).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

/**
 * O Calendário (fase 1).
 *
 * Mostra o que **aconteceu** — sessões, Tasks, Captures e programas abertos —
 * numa grade de mês. Não agenda nada: prazo e compromisso são a fase 2, e a
 * ausência aqui é deliberada, não um esquecimento.
 *
 * A janela pedida ao backend é a GRADE inteira, e não o mês: as células da
 * primeira e da última semana pertencem aos meses vizinhos, e sem elas esses
 * dias apareceriam sempre vazios — o que se leria como "não trabalhei", e não
 * como "não perguntei".
 */
export function CalendarPage() {
  // Sempre o dia 1, para a navegação não escorregar em meses de tamanhos
  // diferentes: 31 de março mais um mês daria 1 de maio.
  const [month, setMonth] = useState(() => {
    const now = new Date();
    return new Date(now.getFullYear(), now.getMonth(), 1);
  });
  const [items, setItems] = useState<CalendarItem[]>([]);
  const [chosen, setChosen] = useState<number | null>(null);
  const [note, setNote] = useState("");

  const grid = useMemo(() => monthGrid(month), [month]);

  const load = useCallback(async () => {
    setNote("");
    const since = grid[0].toISOString();
    const last = grid[grid.length - 1];
    const until = new Date(
      last.getFullYear(), last.getMonth(), last.getDate(), 23, 59, 59, 999,
    ).toISOString();
    try {
      setItems(await api.calendarWindow(since, until));
    } catch (error) {
      setItems([]);
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, [grid]);

  useEffect(() => { void load(); }, [load]);

  const byDay = useMemo(() => groupByLocalDay(items), [items]);
  const today = startOfLocalDay(new Date()).getTime();
  const monthLabel = month
    .toLocaleDateString("pt-BR", { month: "long", year: "numeric" })
    .toUpperCase();

  const step = (months: number) =>
    setMonth(new Date(month.getFullYear(), month.getMonth() + months, 1));

  function goToday() {
    const now = new Date();
    setMonth(new Date(now.getFullYear(), now.getMonth(), 1));
    setChosen(today);
  }

  return (
    <div className="page tempo-page">
      <ContextPath segments={["M", "CALENDÁRIO"]} />

      <PageHeader
        title="Calendário"
        subtitle="O que aconteceu em cada dia."
        actions={
          <>
            <Button variant="ghost" size="sm" onClick={() => step(-1)} aria-label="Mês anterior">‹</Button>
            <Button variant="ghost" size="sm" onClick={goToday}>Hoje</Button>
            <Button variant="ghost" size="sm" onClick={() => step(1)} aria-label="Próximo mês">›</Button>
          </>
        }
      />

      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      <Card label={monthLabel}>
        <div className="calendar-grid">
          {WEEKDAYS.map((weekday) => (
            <span className="micro-label calendar-weekday" key={weekday}>{weekday}</span>
          ))}
          {grid.map((day) => {
            const key = startOfLocalDay(day).getTime();
            const dayItems = byDay.get(key) ?? [];
            const worked = dayItems
              .filter((item) => item.kind === "session")
              .reduce((sum, item) => sum + item.seconds, 0);
            // Um ponto por TIPO presente, nunca um por item: três Tasks
            // concluídas fazem um ponto, e não três. A célula responde "houve
            // Task aqui"; a contagem exata é o que o detalhe do dia dá. Sem
            // isso, um dia movimentado vira uma nuvem que não se conta de
            // relance nem se lê como número.
            const kinds = [...new Set(dayItems.map((item) => item.kind))];
            return (
              <button
                type="button"
                key={key}
                className="calendar-cell"
                data-outside={day.getMonth() !== month.getMonth() || undefined}
                data-today={key === today || undefined}
                aria-pressed={key === chosen}
                aria-label={`${day.getDate()} — ${dayItems.length} registros`}
                onClick={() => setChosen(key)}
              >
                <span className="calendar-day">{day.getDate()}</span>
                {worked ? <span className="calendar-hours">{durationOf(worked)}</span> : null}
                {kinds.length ? (
                  <span className="calendar-dots" aria-hidden="true">
                    {kinds.map((kind) => <span key={kind} data-kind={kind} />)}
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
      </Card>

      {chosen !== null ? <DayDetail at={chosen} items={byDay.get(chosen) ?? []} /> : null}
    </div>
  );
}

/**
 * O dia aberto.
 *
 * Vive fora da grade de propósito: a célula responde "houve algo aqui" em meio
 * segundo, e é este painel que responde "o quê". Misturar os dois faria a
 * célula crescer até a grade deixar de caber num mês.
 */
function DayDetail({ at, items }: { at: number; items: CalendarItem[] }) {
  const label = new Date(at).toLocaleDateString("pt-BR", {
    weekday: "long",
    day: "2-digit",
    month: "long",
  });

  return (
    <Card label={label.toUpperCase()} count={items.length ? String(items.length) : undefined}>
      {items.length ? (
        <ul className="calendar-day-list">
          {items.map((item, index) => (
            <li key={`${item.at}-${index}`}>
              <span className="calendar-item-time">{clockOf(item.at)}</span>
              <span className="calendar-item-kind">{KIND_LABEL[item.kind]}</span>
              <span className="calendar-item-title">{item.title}</span>
              {item.seconds ? (
                <span className="calendar-item-duration">{durationOf(item.seconds)}</span>
              ) : null}
            </li>
          ))}
        </ul>
      ) : (
        <EmptyState>Nada registrado neste dia.</EmptyState>
      )}
    </Card>
  );
}

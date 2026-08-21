import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { groupByLocalDay, monthGrid, startOfLocalDay } from "./calendarDays";
import { EmptyState, Inspector, PaneHeader, StateMessage } from "./Surface";
import type { CalendarItem, CalendarKind } from "./types";

const WEEKDAYS = ["SEG", "TER", "QUA", "QUI", "SEX", "SÁB", "DOM"];

/** O que cada tipo quer dizer na tela. O nome técnico nunca aparece. */
const KIND_LABEL: Record<CalendarKind, string> = {
  session: "sessão",
  task_done: "Task concluída",
  task_created: "Task criada",
  capture: "capture",
  app_opened: "programa aberto",
  meeting: "reunião",
  // As três da Daily Session. Elas entram aqui, e não numa Linha do Tempo nova:
  // este calendário retrospectivo JÁ é a linha do tempo do M/OS, e o dia é um
  // fato tão registrável quanto uma sessão de trabalho.
  day_started: "dia iniciado",
  day_ended: "dia encerrado",
  objective_done: "objetivo concluído",
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
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">("list");
  const [note, setNote] = useState("");
  const gridRef = useRef<HTMLDivElement>(null);
  const inspector = useRef<HTMLElement>(null);

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

  const step = (months: number) => {
    setMonth(new Date(month.getFullYear(), month.getMonth() + months, 1));
    setChosen(null);
    setNarrowPane("list");
  };

  function goToday() {
    const now = new Date();
    setMonth(new Date(now.getFullYear(), now.getMonth(), 1));
    selectDay(today);
  }

  function selectDay(key: number) {
    setChosen(key);
    setNarrowPane("detail");
    if (window.matchMedia("(max-width: 960px)").matches) {
      requestAnimationFrame(() => inspector.current?.focus());
    }
  }

  function returnToGrid() {
    setNarrowPane("list");
    requestAnimationFrame(() => {
      const selected = gridRef.current?.querySelector<HTMLButtonElement>(".calendar-cell[aria-pressed='true']");
      selected?.focus();
    });
  }

  function moveDayFocus(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    if (!["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    let next = index;
    if (event.key === "Home") next = 0;
    else if (event.key === "End") next = grid.length - 1;
    else if (event.key === "ArrowLeft") next = Math.max(0, index - 1);
    else if (event.key === "ArrowRight") next = Math.min(grid.length - 1, index + 1);
    else if (event.key === "ArrowUp") next = Math.max(0, index - 7);
    else next = Math.min(grid.length - 1, index + 7);
    const cells = gridRef.current?.querySelectorAll<HTMLButtonElement>(".calendar-cell");
    cells?.[next]?.focus();
    const day = grid[next];
    if (day) {
      setChosen(startOfLocalDay(day).getTime());
      setNarrowPane(window.matchMedia("(max-width: 960px)").matches ? "list" : "detail");
    }
  }

  const chosenItems = chosen !== null ? byDay.get(chosen) ?? [] : [];

  return (
    <div className="split-page inspector-page calendar-page" data-pane={narrowPane} data-has-day={chosen !== null || undefined}>
      <section className="list-pane calendar-month-pane" aria-label="Grade do mês">
        <PaneHeader
          segments={["M", "CALENDÁRIO"]}
          meta={monthLabel}
          actions={
            <div className="calendar-nav" role="group" aria-label="Navegação do mês">
              <Button variant="ghost" size="sm" onClick={() => step(-1)} aria-label="Mês anterior" title="Mês anterior">‹</Button>
              <Button variant="ghost" size="sm" onClick={goToday} title="Ir para hoje">Hoje</Button>
              <Button variant="ghost" size="sm" onClick={() => step(1)} aria-label="Próximo mês" title="Próximo mês">›</Button>
            </div>
          }
        />

        {note ? <StateMessage state="error" label="Não foi possível carregar o calendário." detail={note} /> : null}

        <div ref={gridRef} className="calendar-grid" role="grid" aria-label={monthLabel}>
          {WEEKDAYS.map((weekday) => (
            <span className="micro-label calendar-weekday" key={weekday} role="columnheader">{weekday}</span>
          ))}
          {grid.map((day, index) => {
            const key = startOfLocalDay(day).getTime();
            const dayItems = byDay.get(key) ?? [];
            const worked = dayItems
              .filter((item) => item.kind === "session")
              .reduce((sum, item) => sum + item.seconds, 0);
            // Um ponto por TIPO presente, nunca um por item: três Tasks
            // concluídas fazem um ponto, e não três. A célula responde "houve
            // Task aqui"; a contagem exata é o que o detalhe do dia dá.
            const kinds = [...new Set(dayItems.map((item) => item.kind))];
            return (
              <button
                type="button"
                key={key}
                role="gridcell"
                className="calendar-cell"
                data-outside={day.getMonth() !== month.getMonth() || undefined}
                data-today={key === today || undefined}
                aria-pressed={key === chosen}
                aria-label={`${day.toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "long" })} — ${dayItems.length} registros`}
                onClick={() => selectDay(key)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    selectDay(key);
                    return;
                  }
                  moveDayFocus(event, index);
                }}
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
      </section>

      {chosen !== null ? (
        <Inspector
          ref={inspector}
          label="Detalhe do dia"
          open={narrowPane === "detail"}
          onBack={returnToGrid}
          onEscape={returnToGrid}
        >
          <DayDetail at={chosen} items={chosenItems} />
        </Inspector>
      ) : (
        <aside className="detail-pane calendar-day-placeholder" aria-label="Detalhe do dia">
          <span className="micro-label">DIA</span>
          <p className="empty-state">Selecione um dia na grade para ver o que aconteceu.</p>
        </aside>
      )}
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
    <>
      <header className="detail-header">
        <div>
          <span className="micro-label">DIA</span>
          <h1>{label}</h1>
          <p>{items.length ? `${items.length} ${items.length === 1 ? "registro" : "registros"}` : "Sem registros."}</p>
        </div>
      </header>
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
    </>
  );
}

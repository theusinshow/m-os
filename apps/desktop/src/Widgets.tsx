import { useMemo } from "react";
import { Bars } from "./Plot";
import { Ring, RingLabel, stagger } from "./Ring";
import type { Capture, Task } from "./types";

/**
 * Widgets visuais da Home — `M-OS Widgets - Visuais e Animados v0.1`.
 *
 * Só entram aqui os widgets do desenho que têm dado REAL no M/OS hoje. Um anel
 * bonito preenchido com número inventado é pior que a ausência: ele ensina a
 * confiar numa medida que não existe.
 *
 * Os que dependiam de TEMPO RASTREADO saíram deste bloqueio quando o CronoCAD
 * foi absorvido, e vivem em `TimeWidgets.tsx` — separados porque carregam o
 * próprio dado, fora do `refresh()` da Home. Os que restam dependem de
 * calendário e de hábitos, que continuam não existindo no domínio.
 */

const WEEKDAYS = ["SEG", "TER", "QUA", "QUI", "SEX", "SÁB", "DOM"];

/** Segunda como início da semana, que é como a semana de trabalho é lida. */
function startOfWeek(reference: Date) {
  const date = new Date(reference);
  date.setHours(0, 0, 0, 0);
  const weekday = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() - weekday);
  return date;
}

function sameDay(left: Date, right: Date) {
  return left.toDateString() === right.toDateString();
}

/**
 * W03 · WEEK RINGS — como foi cada dia, sem gráfico de barras.
 *
 * Passado em sódio 55%, hoje em sódio cheio com a contagem no centro, futuro em
 * traço tracejado. O vazio à direita é a informação: ele diz quanto da semana
 * ainda não aconteceu, o que uma barra de progresso esconderia.
 *
 * A proporção é contra o melhor dia da semana, e não contra uma meta. O M/OS
 * não tem meta diária de tasks, e inventar uma faria o anel medir uma régua
 * que ninguém definiu.
 */
export function WeekRings({ tasks, onOpen }: { tasks: Task[]; onOpen: () => void }) {
  const week = useMemo(() => {
    const today = new Date();
    const start = startOfWeek(today);
    const days = Array.from({ length: 7 }, (_, index) => {
      const date = new Date(start);
      date.setDate(start.getDate() + index);
      const done = tasks.filter(
        (task) => task.completedAt && sameDay(new Date(task.completedAt), date),
      ).length;
      return { date, done, isToday: sameDay(date, today), isFuture: date > today && !sameDay(date, today) };
    });
    const peak = Math.max(1, ...days.map((day) => day.done));
    return { days, peak, total: days.reduce((sum, day) => sum + day.done, 0) };
  }, [tasks]);

  return (
    <div className="widget-week">
      <div className="widget-week-head">
        <span className="micro-label">SEMANA</span>
        <button type="button" className="filter-label" onClick={onOpen}>
          {week.total} {week.total === 1 ? "TASK CONCLUÍDA" : "TASKS CONCLUÍDAS"}
        </button>
      </div>
      {/* Barras e não anéis: comparar sete alturas é mais rápido que comparar
          sete ângulos, e 44px era justamente o tamanho em que o cap arredondado
          mais distorceria (ADR-040). A proporção continua sendo contra o melhor
          dia da semana, e não contra uma meta que ninguém definiu. */}
      <div className="widget-plot">
        <Bars
          ratios={week.days.map((day) => (day.isFuture ? 0 : day.done / week.peak))}
          labels={week.days.map((day, index) => (day.isToday ? "HOJE" : WEEKDAYS[index]))}
          highlight={week.days.findIndex((day) => day.isToday)}
        />
      </div>
    </div>
  );
}

/**
 * W07 · DENSIDADE — o mês como área ocupada, não como tabela.
 *
 * A carga de um dia é o que passou por ele: Task criada, Task concluída,
 * Capture registrada. O desenho fala em "eventos", que viriam de um calendário
 * — como ele não existe, a medida é a atividade que o M/OS de fato registra, e
 * o rótulo diz isso em vez de fingir agenda.
 *
 * Uma escolha contra o desenho: ele pede o passado a 45%, o que faz sentido no
 * arco do dia, onde o passado é contexto e o próximo evento é o assunto. Aqui o
 * passado É o conteúdo — apagá-lo apagaria o widget inteiro.
 */
export function MonthDensity({ tasks, captures }: { tasks: Task[]; captures: Capture[] }) {
  const month = useMemo(() => {
    const today = new Date();
    const first = new Date(today.getFullYear(), today.getMonth(), 1);
    const last = new Date(today.getFullYear(), today.getMonth() + 1, 0);
    const lead = (first.getDay() + 6) % 7;

    const activity = new Map<string, number>();
    const bump = (value: string | null) => {
      if (!value) return;
      const date = new Date(value);
      if (date.getMonth() !== today.getMonth() || date.getFullYear() !== today.getFullYear()) return;
      const key = String(date.getDate());
      activity.set(key, (activity.get(key) ?? 0) + 1);
    };
    tasks.forEach((task) => { bump(task.createdAt); bump(task.completedAt); });
    captures.forEach((capture) => bump(capture.capturedAt));

    const peak = Math.max(1, ...activity.values());
    const cells = Array.from({ length: lead }, () => null as null | { day: number; load: number; isToday: boolean });
    for (let day = 1; day <= last.getDate(); day += 1) {
      const count = activity.get(String(day)) ?? 0;
      cells.push({
        day,
        // Quatro degraus. Zero fica no fundo neutro: ausência de carga não é
        // um degrau de carga.
        load: count === 0 ? 0 : Math.max(1, Math.ceil((count / peak) * 4)),
        isToday: day === today.getDate(),
      });
    }
    return { cells, total: [...activity.values()].reduce((sum, value) => sum + value, 0), label: first };
  }, [tasks, captures]);

  return (
    <div className="widget-density">
      <div className="widget-week-head">
        <span className="micro-label">
          {new Intl.DateTimeFormat("pt-BR", { month: "long" }).format(month.label).toUpperCase()}
        </span>
        <span className="micro-label">
          {month.total} {month.total === 1 ? "REGISTRO" : "REGISTROS"}
        </span>
      </div>
      <div className="mos-density" aria-hidden="true">
        {["S", "T", "Q", "Q", "S", "S", "D"].map((initial, index) => (
          <span className="mos-density-head" key={index}>{initial}</span>
        ))}
      </div>
      <div className="mos-density">
        {month.cells.map((cell, index) =>
          cell ? (
            <span
              className="mos-density-cell"
              key={index}
              data-load={cell.load || undefined}
              data-today={cell.isToday || undefined}
              style={{ ["--ring-delay" as string]: stagger(index) }}
            >
              {cell.day}
            </span>
          ) : (
            <span className="mos-density-cell" data-outside key={index} />
          ),
        )}
      </div>
    </div>
  );
}

/**
 * Progresso do trabalho: quanto do que existe já está concluído.
 *
 * Substitui a contagem crua do widget de Tasks. O anel responde em meio segundo
 * a pergunta que a lista respondia em três leituras — e o número continua lá,
 * no centro, para quem precisa dele exato.
 */
export function TaskProgressRing({ tasks }: { tasks: Task[] }) {
  const { done, total } = useMemo(() => {
    const active = tasks.filter((task) => task.lifecycleState === "active");
    return { done: active.filter((task) => task.state === "done").length, total: active.length };
  }, [tasks]);

  if (!total) return null;

  return (
    <div className="widget-progress">
      <Ring size={88} segments={[{ value: done / total }]}>
        <RingLabel value={`${Math.round((done / total) * 100)}%`} unit={`${done} DE ${total}`} />
      </Ring>
      <div className="widget-progress-copy">
        <span className="micro-label">CONCLUÍDO</span>
        <p className="hermes-quiet">
          {total - done === 0
            ? "Nenhuma Task aberta."
            : `${total - done} ${total - done === 1 ? "Task aberta" : "Tasks abertas"}.`}
        </p>
      </div>
    </div>
  );
}

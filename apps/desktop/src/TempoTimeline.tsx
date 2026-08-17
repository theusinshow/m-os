import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Card, EmptyState, PageHeader } from "./Surface";
import type { ActivityEvent, ActivityType, Period, Project } from "./types";

/**
 * O que cada evento observado quer dizer, em português.
 *
 * O nome técnico (`app_opened`) atravessa a ponte e vive no banco com CHECK
 * constraint; ele não deve aparecer na tela. Traduzir aqui, e não renomear lá,
 * mantém as duas coisas separadas.
 */
const EVENT_LABEL: Record<string, string> = {
  app_opened: "programa aberto",
  app_closed: "programa fechado",
  idle_started: "inatividade iniciada",
  idle_ended: "atividade retomada",
  timer_started: "cronômetro iniciado",
  timer_paused: "cronômetro pausado",
  timer_resumed: "cronômetro retomado",
  timer_stopped: "cronômetro encerrado",
};

/** `2026-08-16` no fuso LOCAL: `toISOString` devolve UTC e vira o dia errado à noite. */
function todayInput() {
  const now = new Date();
  const local = new Date(now.getTime() - now.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 10);
}

function boundsOf(day: string) {
  const [year, month, date] = day.split("-").map(Number);
  const start = new Date(year, month - 1, date, 0, 0, 0, 0);
  const end = new Date(year, month - 1, date, 23, 59, 59, 999);
  return { since: start.toISOString(), until: end.toISOString() };
}

function clockOf(iso: string) {
  return new Date(iso).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

function durationOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return hours ? `${hours}h${String(minutes).padStart(2, "0")}` : `${minutes}min`;
}

/**
 * A Linha do Tempo do dia.
 *
 * Mostra só os períodos em que um programa monitorado esteve aberto e **não há
 * sessão registrada**. O que já foi registrado não aparece — oferecê-lo
 * convidaria a contar a mesma hora duas vezes, e a segunda contagem só apareceria
 * na fatura.
 *
 * Nada vira sessão sozinho. O sistema observou; quem decide se aquilo foi
 * trabalho, e de qual Project, é você.
 */
export function TempoTimeline({ projects, onChanged }: {
  projects: Project[];
  onChanged?: () => void;
}) {
  const [day, setDay] = useState(todayInput);
  const [gaps, setGaps] = useState<Period[]>([]);
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  const [choice, setChoice] = useState<Record<string, string>>({});
  const [activity, setActivity] = useState<ActivityType>("drawing");
  const [note, setNote] = useState("");
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const { since, until } = boundsOf(day);
    const [nextGaps, nextEvents] = await Promise.all([
      api.monitoringTimeline(since, until).catch(() => []),
      api.activityEvents(since, until).catch(() => [] as ActivityEvent[]),
    ]);
    setGaps(nextGaps);
    setEvents(nextEvents);
    setLoading(false);
  }, [day]);

  useEffect(() => { void load(); }, [load]);

  const active = projects.filter((project) => project.lifecycleState === "active");

  async function record(period: Period) {
    const projectId = choice[period.start];
    if (!projectId) return;
    setNote("");
    try {
      await api.recordFromTimeline(projectId, period.start, period.end, activity);
      await load();
      onChanged?.();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  const total = gaps.reduce(
    (sum, period) => sum + (new Date(period.end).getTime() - new Date(period.start).getTime()) / 1000,
    0,
  );

  return (
    <>
      <PageHeader
        title="Linha do tempo detectada"
        subtitle="Eventos do dia e períodos sem registro para reconstruir."
        actions={
          <input
            type="date"
            value={day}
            aria-label="Dia da linha do tempo"
            onChange={(event) => setDay(event.currentTarget.value)}
          />
        }
      />

      <div className="tempo-cols" data-cols="2">
        <Card label="LACUNAS DETECTADAS" count={gaps.length ? durationOf(total) : undefined}>
      <p className="support-copy">
        Períodos com um programa monitorado aberto e <strong>sem sessão registrada</strong>. Nada vira hora sozinho —
        o sistema observou, e quem decide se foi trabalho é você.
      </p>

      {loading ? (
        <EmptyState>Lendo o dia…</EmptyState>
      ) : gaps.length ? (
        <>
          <div className="tempo-field">
            <label htmlFor="timeline-activity">Atividade dos períodos registrados</label>
            <select
              id="timeline-activity"
              value={activity}
              onChange={(event) => setActivity(event.currentTarget.value as ActivityType)}
            >
              <option value="drawing">desenho</option>
              <option value="detailing">detalhamento</option>
              <option value="revision">revisão</option>
              <option value="meeting">reunião</option>
              <option value="study">estudo</option>
              <option value="other">outro</option>
            </select>
          </div>

          <div className="tempo-sessions">
            {gaps.map((period) => {
              const seconds = (new Date(period.end).getTime() - new Date(period.start).getTime()) / 1000;
              return (
                <div className="tempo-session" key={period.start}>
                  <span>
                    <strong>{clockOf(period.start)} — {clockOf(period.end)}</strong>
                    <small>{durationOf(seconds)} sem registro</small>
                  </span>
                  <span className="tempo-session-actions" data-always>
                    <select
                      aria-label={`Project para o período das ${clockOf(period.start)}`}
                      value={choice[period.start] ?? ""}
                      onChange={(event) => setChoice({ ...choice, [period.start]: event.currentTarget.value })}
                    >
                      <option value="">Project</option>
                      {active.map((project) => (
                        <option key={project.id} value={project.id}>{project.name}</option>
                      ))}
                    </select>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={!choice[period.start]}
                      onClick={() => void record(period)}
                    >
                      Registrar
                    </Button>
                  </span>
                </div>
              );
            })}
          </div>
          {/* Dito antes de clicar: hora que o sistema propôs não é hora medida,
              e a sessão nasce marcada como reconstruída por isso. */}
          <p className="support-copy">
            O que você registrar aqui entra como <strong>reconstruída</strong> — proposta pelo sistema e aceita por
            você, distinta da hora que o cronômetro mediu.
          </p>
        </>
      ) : (
        <EmptyState>
          Nenhum período com programa aberto sem registro neste dia.
        </EmptyState>
      )}
      {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
        </Card>

        {/* O que o sistema VIU, sem interpretação. A coluna da esquerda propõe
            trabalho; esta só presta contas — e é ela que permite conferir por
            que uma lacuna existe, ou por que nenhuma apareceu. */}
        <Card label="EVENTOS DO DIA" count={events.length ? String(events.length) : undefined}>
          {events.length ? (
            <ul className="tempo-events">
              {events.map((event) => (
                <li key={event.id}>
                  <span className="tempo-event-time">{clockOf(event.detectedAt)}</span>
                  <span>
                    {EVENT_LABEL[event.kind] ?? event.kind}
                    {event.processName ? ` · ${event.processName}` : ""}
                  </span>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState>Nada observado neste dia.</EmptyState>
          )}
        </Card>
      </div>
    </>
  );
}

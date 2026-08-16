import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState, Panel } from "./Surface";
import type { ActivityType, Period, Project } from "./types";

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
  const [choice, setChoice] = useState<Record<string, string>>({});
  const [activity, setActivity] = useState<ActivityType>("drawing");
  const [note, setNote] = useState("");
  const [loading, setLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const { since, until } = boundsOf(day);
    setGaps(await api.monitoringTimeline(since, until).catch(() => []));
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
    <Panel
      label="LINHA DO TEMPO"
      count={gaps.length ? durationOf(total) : undefined}
      action={
        <input
          type="date"
          value={day}
          aria-label="Dia da linha do tempo"
          onChange={(event) => setDay(event.currentTarget.value)}
        />
      }
    >
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
          Nada sem registro neste dia. Ou tudo já virou sessão, ou nenhum programa monitorado esteve aberto.
        </EmptyState>
      )}
      {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
    </Panel>
  );
}

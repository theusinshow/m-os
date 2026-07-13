import { useEffect, useMemo, useState } from "react";
import { ArrowRight } from "lucide-react";
import type { ActivityEvent } from "@/types/domain";
import { listActivityEvents } from "@/services/activityEvents";
import { useEntriesStore } from "@/stores/entriesStore";
import { useSettingsStore } from "@/stores/settingsStore";
import {
  EVENT_LABELS,
  findGaps,
  pairAppSessions,
  type AppInterval,
} from "@/lib/timeline";
import { formatDuration, formatTime } from "@/lib/format";
import { isoToDateInput, isSameLocalDay } from "@/lib/datetime";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { Field, Input } from "@/components/ui/Field";
import { EntryForm, type EntryPrefill } from "@/features/history/EntryForm";

/**
 * Linha do tempo detectada (secao 14). Mostra os eventos do dia e as lacunas
 * (programa aberto sem sessao) que podem ser transformadas em registro
 * (`source = reconstructed`) — sem decisoes automaticas.
 */
export function ReconstructionPage() {
  const entries = useEntriesStore((s) => s.entries);
  const apps = useSettingsStore((s) => s.apps);

  const [date, setDate] = useState(() => isoToDateInput(new Date().toISOString()));
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [prefill, setPrefill] = useState<EntryPrefill | null>(null);

  const dayStart = useMemo(() => new Date(`${date}T00:00:00`), [date]);

  useEffect(() => {
    let active = true;
    setLoading(true);
    const from = dayStart.toISOString();
    const to = new Date(dayStart.getTime() + 86_400_000).toISOString();
    void listActivityEvents(from, to)
      .then((evs) => active && setEvents(evs))
      .finally(() => active && setLoading(false));
    return () => {
      active = false;
    };
  }, [dayStart]);

  const displayName = (process: string | null) =>
    apps.find((a) => a.processName === process)?.displayName ?? process ?? "?";

  const dayEntries = entries.filter((e) => isSameLocalDay(e.startedAt, dayStart));
  const gaps = useMemo(
    () => findGaps(pairAppSessions(events), dayEntries, Date.now()),
    [events, dayEntries],
  );

  function reconstruct(gap: AppInterval) {
    setPrefill({
      startedAt: gap.start,
      endedAt: gap.end ?? new Date().toISOString(),
      description: `Reconstruido de ${displayName(gap.processName)}`,
      source: "reconstructed",
    });
  }

  return (
    <div>
      <PageHeader
        title="Linha do tempo detectada"
        description="Eventos do dia e periodos sem registro para reconstruir."
        action={
          <div className="w-44">
            <Field label="Dia" htmlFor="tl-date">
              <Input
                id="tl-date"
                type="date"
                value={date}
                onChange={(e) => setDate(e.target.value)}
              />
            </Field>
          </div>
        }
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <Panel>
          <PanelHeader title="Lacunas detectadas" />
          {gaps.length === 0 ? (
            <p className="px-4 py-6 text-sm text-text-muted">
              Nenhum periodo com programa aberto sem registro neste dia.
            </p>
          ) : (
            <ul className="divide-y divide-border">
              {gaps.map((gap) => {
                const end = gap.end ?? new Date().toISOString();
                const seconds = Math.max(
                  0,
                  Math.round(
                    (Date.parse(end) - Date.parse(gap.start)) / 1000,
                  ),
                );
                return (
                  <li
                    key={`${gap.processName}-${gap.start}`}
                    className="flex items-center justify-between px-4 py-3"
                  >
                    <div>
                      <p className="text-sm text-text">
                        {displayName(gap.processName)}
                      </p>
                      <p className="tabular text-xs text-text-muted">
                        {formatTime(gap.start)}
                        {gap.end ? `–${formatTime(gap.end)}` : " (em aberto)"} ·{" "}
                        {formatDuration(seconds)}
                      </p>
                    </div>
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => reconstruct(gap)}
                      icon={<ArrowRight size={15} strokeWidth={1.75} />}
                    >
                      Transformar em registro
                    </Button>
                  </li>
                );
              })}
            </ul>
          )}
        </Panel>

        <Panel>
          <PanelHeader title="Eventos do dia" />
          {loading ? (
            <p className="px-4 py-6 text-sm text-text-muted">Carregando…</p>
          ) : events.length === 0 ? (
            <div className="p-4">
              <EmptyState
                title="Nenhum evento detectado"
                description="Eventos de programas monitorados, cronometro e inatividade aparecem aqui."
              />
            </div>
          ) : (
            <ul className="space-y-3 p-4">
              {events.map((ev) => (
                <li key={ev.id} className="flex gap-3">
                  <span className="tabular w-12 shrink-0 text-sm text-text-subtle">
                    {formatTime(ev.detectedAt)}
                  </span>
                  <span className="text-sm text-text-muted">
                    {ev.processName
                      ? `${displayName(ev.processName)} ${EVENT_LABELS[ev.eventType]}`
                      : EVENT_LABELS[ev.eventType]}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <EntryForm
        open={prefill !== null}
        entry={null}
        prefill={prefill ?? undefined}
        onClose={() => setPrefill(null)}
      />
    </div>
  );
}

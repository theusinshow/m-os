import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState, Panel } from "./Surface";
import type { MonitoredApp, TrackingSettings } from "./types";

const MODES: { value: TrackingSettings["rounding"]["mode"]; label: string }[] = [
  { value: "nearest", label: "mais próximo" },
  { value: "up", label: "para cima" },
  { value: "down", label: "para baixo" },
];

/**
 * Arredondamento e inatividade.
 *
 * O aviso na tela não é decoração: o arredondamento **não** altera o que está
 * gravado. O banco guarda sempre o tempo real, e isto muda só o que se cobra.
 * Sem dizer isso, mudar de 15 para 30 minutos pareceria reescrever o passado.
 */
export function TempoSettings({ onChanged }: { onChanged?: () => void }) {
  const [settings, setSettings] = useState<TrackingSettings | null>(null);
  const [apps, setApps] = useState<MonitoredApp[]>([]);
  const [process, setProcess] = useState("");
  const [label, setLabel] = useState("");
  const [note, setNote] = useState("");

  const load = useCallback(async () => {
    const [next, list] = await Promise.all([
      api.trackingSettings().catch(() => null),
      api.monitoredApps().catch(() => [] as MonitoredApp[]),
    ]);
    setSettings(next);
    setApps(list);
  }, []);

  useEffect(() => { void load(); }, [load]);

  async function guard(run: () => Promise<unknown>) {
    setNote("");
    try {
      await run();
      await load();
      onChanged?.();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  function save(next: TrackingSettings) {
    setSettings(next);
    void guard(() => api.trackingSetSettings(next));
  }

  return (
    <>
      <Panel label="ARREDONDAMENTO">
        {settings ? (
          <div className="tempo-form">
            <label className="tempo-check">
              <input
                type="checkbox"
                checked={settings.rounding.enabled}
                onChange={(event) =>
                  save({ ...settings, rounding: { ...settings.rounding, enabled: event.currentTarget.checked } })
                }
              />
              Arredondar na cobrança
            </label>
            <div className="tempo-field">
              <label htmlFor="rounding-interval">Intervalo (min)</label>
              <input
                id="rounding-interval"
                type="number"
                min={1}
                max={120}
                step={5}
                value={settings.rounding.intervalMinutes}
                disabled={!settings.rounding.enabled}
                onChange={(event) =>
                  save({
                    ...settings,
                    rounding: {
                      ...settings.rounding,
                      intervalMinutes: Math.max(1, Number(event.currentTarget.value) || 1),
                    },
                  })
                }
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="rounding-mode">Modo</label>
              <select
                id="rounding-mode"
                value={settings.rounding.mode}
                disabled={!settings.rounding.enabled}
                onChange={(event) =>
                  save({
                    ...settings,
                    rounding: {
                      ...settings.rounding,
                      mode: event.currentTarget.value as TrackingSettings["rounding"]["mode"],
                    },
                  })
                }
              >
                {MODES.map((mode) => <option key={mode.value} value={mode.value}>{mode.label}</option>)}
              </select>
            </div>
            <div className="tempo-field">
              <label htmlFor="idle-threshold">Inatividade a partir de (min)</label>
              <input
                id="idle-threshold"
                type="number"
                min={1}
                max={120}
                value={settings.idleThresholdMinutes}
                onChange={(event) =>
                  save({ ...settings, idleThresholdMinutes: Math.max(1, Number(event.currentTarget.value) || 1) })
                }
              />
            </div>
            <p className="support-copy">
              O arredondamento <strong>não</strong> altera o que está gravado. O banco guarda sempre o tempo real —
              isto muda só o número que você cobra, e o arredondamento é aplicado por sessão, não sobre a soma.
            </p>
          </div>
        ) : (
          <EmptyState>Configuração indisponível.</EmptyState>
        )}
      </Panel>

      <Panel label="PROGRAMAS MONITORADOS" count={apps.length ? String(apps.length) : undefined}>
        {/* Observar não é cronometrar: o programa aberto vira um evento na Linha
            do Tempo, e continua sendo você quem decide se aquilo foi trabalho. */}
        <p className="support-copy">
          Abrir um destes vira evento na Linha do Tempo. Nenhuma hora é registrada sozinha.
        </p>
        {apps.length ? (
          <div className="tempo-sessions">
            {apps.map((entry) => (
              <div className="tempo-session" key={entry.id}>
                <span>
                  <strong>{entry.displayName}</strong>
                  <small>{entry.processName}{entry.enabled ? "" : " · desativado"}</small>
                </span>
                <span className="tempo-session-actions">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void guard(() => api.saveMonitoredApp({ ...entry, enabled: !entry.enabled }))}
                  >
                    {entry.enabled ? "Desativar" : "Ativar"}
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => void guard(() => api.deleteMonitoredApp(entry.id))}>
                    Remover
                  </Button>
                </span>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState>Nenhum programa monitorado.</EmptyState>
        )}

        <form
          className="tempo-form"
          onSubmit={(event) => {
            event.preventDefault();
            void guard(async () => {
              await api.saveMonitoredApp({
                id: crypto.randomUUID(),
                displayName: label.trim() || process.trim(),
                processName: process.trim(),
                enabled: true,
                remindOnOpen: true,
                remindOnClose: true,
              });
              setProcess("");
              setLabel("");
            });
          }}
        >
          <div className="tempo-field">
            <label htmlFor="app-label">Nome</label>
            <input id="app-label" value={label} placeholder="Revit" onChange={(event) => setLabel(event.currentTarget.value)} />
          </div>
          <div className="tempo-field">
            <label htmlFor="app-process">Executável</label>
            <input
              id="app-process"
              value={process}
              placeholder="revit.exe"
              onChange={(event) => setProcess(event.currentTarget.value)}
            />
          </div>
          <Button variant="secondary" size="sm" type="submit" disabled={!process.trim()}>Adicionar</Button>
        </form>
        {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
      </Panel>
    </>
  );
}

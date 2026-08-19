import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { ChannelOutcome, Meeting, MeetingTick } from "./types";

/**
 * A barra de gravação.
 *
 * Ela vive no shell, e não numa página, por uma razão que é promessa e não
 * conveniência: **nunca gravar sem indicação visível** (`MEETING-AGENT.md`
 * §17.2). Se ela morasse na tela de Reuniões, navegar para a Home apagaria da
 * vista o fato de que o microfone está aberto.
 *
 * Sem waveform, sem medidor grande, sem cockpit. Um nível discreto é permitido
 * porque responde a uma pergunta real — "está me ouvindo?" — e qualquer coisa
 * maior seria o showcase que o desenho proíbe.
 */

function clock(ms: number) {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (value: number) => String(value).padStart(2, "0");
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/** Oito degraus. Mais que isso vira animação, e animação aqui não carrega dado. */
function Level({ value }: { value: number }) {
  const filled = Math.min(8, Math.round((value / 1000) * 8));
  return (
    <span className="meeting-level" aria-hidden="true">
      {Array.from({ length: 8 }, (_, index) => (
        <i key={index} data-on={index < filled || undefined} />
      ))}
    </span>
  );
}

/**
 * O estado de um canal, em uma palavra e uma cor de estado.
 *
 * `lost` não vira erro vermelho de tela cheia: o outro canal pode estar
 * gravando, e §20 exige que a pessoa consiga distinguir "perdi a gravação" de
 * "um canal caiu e o outro continua".
 */
function Channel({ label, outcome, level }: {
  label: string;
  outcome: ChannelOutcome;
  level: number;
}) {
  const state = outcome.state;
  const detail =
    state === "lost" ? `perdido aos ${clock(outcome.atMs)}`
      : state === "unavailable" ? "indisponível"
        : null;
  return (
    <span className="meeting-channel" data-state={state} title={detail ?? undefined}>
      <span className="micro-label">{label}</span>
      {state === "capturing" || state === "captured"
        ? <Level value={level} />
        : <span className="meeting-channel-note">{detail}</span>}
    </span>
  );
}

export function RecordingBar({ onStopped, openMeeting }: {
  onStopped: (meeting: Meeting) => void;
  openMeeting: (id: string) => void;
}) {
  const [tick, setTick] = useState<MeetingTick | null>(null);
  const [stopping, setStopping] = useState(false);
  const [note, setNote] = useState("");
  // Guardado num ref para o `listen` não precisar ser refeito a cada tick.
  const stoppingRef = useRef(false);

  useEffect(() => {
    let alive = true;
    // O primeiro estado vem por pergunta, e não por evento: se o app abriu com
    // uma gravação já em curso — o que acontece quando a janela fecha e volta
    // pelo tray — esperar o próximo evento deixaria a barra ausente por até um
    // segundo, e uma barra ausente lê-se como "não está gravando".
    void api.meetingRecording().then((current) => { if (alive) setTick(current); }).catch(() => undefined);

    const unlisten = listen<MeetingTick>("meeting-tick", (event) => {
      if (!stoppingRef.current) setTick(event.payload);
    });
    return () => {
      alive = false;
      void unlisten.then((off) => off());
    };
  }, []);

  const stop = useCallback(async () => {
    setStopping(true);
    stoppingRef.current = true;
    setNote("");
    try {
      const meeting = await api.meetingStop();
      setTick(null);
      onStopped(meeting);
    } catch (error) {
      // Parar que falha NÃO limpa a barra: a gravação pode continuar viva, e
      // apagar o indicador seria a mentira mais cara desta tela.
      setNote(error instanceof Error ? error.message : String(error));
    } finally {
      setStopping(false);
      stoppingRef.current = false;
    }
  }, [onStopped]);

  if (!tick) return null;

  const bothGone = !hasAudio(tick.mic) && !hasAudio(tick.system);

  return (
    <div className="recording-bar" role="status" aria-live="polite" data-warning={bothGone || undefined}>
      <button
        type="button"
        className="recording-open"
        onClick={() => openMeeting(tick.meetingId)}
        aria-label="Abrir a reunião em gravação"
      >
        <span className="recording-dot" aria-hidden="true" />
        <span className="recording-clock">{clock(tick.durationMs)}</span>
      </button>
      <Channel label="MIC" outcome={tick.mic} level={tick.micLevel} />
      <Channel label="SISTEMA" outcome={tick.system} level={tick.systemLevel} />
      {note ? <span className="recording-note">{note}</span> : null}
      <button type="button" className="recording-stop" onClick={() => void stop()} disabled={stopping}>
        {stopping ? "PARANDO…" : "PARAR"}
      </button>
    </div>
  );
}

export function hasAudio(outcome: ChannelOutcome) {
  return outcome.state !== "unavailable";
}

export { clock as formatMeetingClock };

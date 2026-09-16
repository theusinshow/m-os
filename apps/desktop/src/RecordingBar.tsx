import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { copyDoGuardian, relogio } from "./reuniao";
import type { ChannelOutcome, GuardianView, Meeting, MeetingTick } from "./types";

/**
 * O mini card da reunião em curso.
 *
 * Ele vive no shell, e não numa página, por uma razão que é promessa e não
 * conveniência: **nunca gravar sem indicação visível** (`MEETING-AGENT.md`
 * §17.2). Se ele morasse em Reuniões, navegar para a Home apagaria da vista o
 * fato de que o microfone está aberto.
 *
 * `● Gravando · 24:32  [⭐] [Pausar] [Encerrar]` — e, quando o Recording
 * Guardian suspeita que a reunião acabou, a pergunta aparece AQUI, sem modal:
 * a pessoa pode estar no meio de outra coisa, e nada no M/OS trava por causa
 * dela.
 */

/**
 * O estado de um canal, só quando saiu do normal.
 *
 * `lost` não vira erro de tela cheia: o outro canal pode estar gravando, e §20
 * exige distinguir "perdi a gravação" de "um canal caiu e o outro continua".
 */
function CanalComProblema({ label, outcome }: { label: string; outcome: ChannelOutcome }) {
  if (outcome.state !== "lost" && outcome.state !== "unavailable") return null;
  const detalhe = outcome.state === "lost" ? `caiu aos ${relogio(outcome.atMs)}` : "indisponível";
  return (
    <span className="meeting-channel" data-state={outcome.state} title={detalhe}>
      <span className="micro-label">{label}</span>
      <span className="meeting-channel-note">{detalhe}</span>
    </span>
  );
}

export function RecordingBar({ onStopped, openMeeting }: {
  onStopped: (meeting: Meeting) => void;
  openMeeting: (id: string) => void;
}) {
  const [tick, setTick] = useState<MeetingTick | null>(null);
  const [ocupado, setOcupado] = useState(false);
  const [nota, setNota] = useState("");
  const [marcou, setMarcou] = useState(false);
  const parandoRef = useRef(false);

  useEffect(() => {
    let vivo = true;
    // O primeiro estado vem por pergunta, e não por evento: se o app abriu com
    // uma gravação já em curso, esperar o próximo evento deixaria a barra
    // ausente por até um segundo — e ausente lê-se como "não está gravando".
    void api.meetingRecording().then((atual) => { if (vivo) setTick(atual); }).catch(() => undefined);
    const offs = [
      listen<MeetingTick>("meeting-tick", (evento) => {
        if (!parandoRef.current) setTick(evento.payload);
      }),
      listen<GuardianView>("meeting-guardian", (evento) => {
        setTick((atual) => (atual ? { ...atual, guardian: evento.payload } : atual));
      }),
      listen<Meeting>("meeting-started", () => {
        void api.meetingRecording().then(setTick).catch(() => undefined);
      }),
      // Parou por outro caminho — tray, atalho, Guardian: a barra some junto.
      listen<Meeting>("meeting-stopped", () => setTick(null)),
    ];
    return () => {
      vivo = false;
      offs.forEach((off) => void off.then((fn) => fn()));
    };
  }, []);

  const agir = useCallback(async (acao: () => Promise<unknown>) => {
    setOcupado(true);
    setNota("");
    try {
      await acao();
    } catch (erro) {
      // Falhar NÃO limpa a barra: a gravação pode continuar viva, e apagar o
      // indicador seria a mentira mais cara desta tela.
      setNota(erro instanceof Error ? erro.message : String(erro));
    } finally {
      setOcupado(false);
    }
  }, []);

  const encerrar = useCallback((cortarEm: number | null) => agir(async () => {
    parandoRef.current = true;
    try {
      const meeting = cortarEm != null ? await api.meetingStopAndTrim(cortarEm) : await api.meetingStop();
      setTick(null);
      onStopped(meeting);
    } finally {
      parandoRef.current = false;
    }
  }), [agir, onStopped]);

  const marcar = useCallback(() => agir(async () => {
    await api.meetingMarkMoment();
    // A confirmação é um brilho curto na estrela, e não um recibo: marcar é um
    // gesto de meio segundo no meio de uma conversa.
    setMarcou(true);
    window.setTimeout(() => setMarcou(false), 900);
  }), [agir]);

  if (!tick) return null;

  const semAudio = tick.mic.state === "unavailable" && tick.system.state === "unavailable";
  const copy = copyDoGuardian(tick.guardian, tick.durationMs);

  return (
    <div
      className="recording-bar"
      role="status"
      aria-live="polite"
      data-warning={semAudio || undefined}
      data-guardian={copy ? tick.guardian.kind : undefined}
    >
      <button
        type="button"
        className="recording-open"
        onClick={() => openMeeting(tick.meetingId)}
        aria-label="Abrir a reunião em gravação"
      >
        <span className="recording-dot" data-pausada={tick.paused || undefined} aria-hidden="true" />
        <span className="micro-label">{tick.paused ? "PAUSADA" : "GRAVANDO"}</span>
        <span className="recording-clock">{relogio(tick.durationMs)}</span>
      </button>

      <CanalComProblema label="MIC" outcome={tick.mic} />
      <CanalComProblema label="SISTEMA" outcome={tick.system} />

      {copy ? (
        /* A pergunta do Guardian no lugar dos controles comuns: enquanto ela
           existe, as duas respostas são o que importa. */
        <span className="recording-guardian">
          <span className="recording-guardian-texto" title={copy.corpo}>{copy.titulo}</span>
          <Button variant="outline" size="sm" disabled={ocupado} onClick={() => void encerrar(copy.cortarEm)}>
            {tick.guardian.kind === "countdown" ? `${copy.encerrar} · ${tick.guardian.secondsLeft}s` : copy.encerrar}
          </Button>
          <Button variant="ghost" size="sm" disabled={ocupado} onClick={() => void agir(() => api.meetingGuardianContinue())}>
            {copy.continuar}
          </Button>
        </span>
      ) : (
        <span className="recording-controles">
          <button
            type="button"
            className="recording-marcar"
            data-marcou={marcou || undefined}
            onClick={() => void marcar()}
            disabled={ocupado || tick.paused}
            aria-label="Marcar momento"
            title="Marcar momento · Ctrl+Alt+M"
          >★</button>
          <Button
            variant="ghost"
            size="sm"
            disabled={ocupado}
            onClick={() => void agir(() => (tick.paused ? api.meetingResume() : api.meetingPause()))}
          >
            {tick.paused ? "Retomar" : "Pausar"}
          </Button>
          <Button variant="outline" size="sm" className="recording-stop" disabled={ocupado} onClick={() => void encerrar(null)}>
            Encerrar
          </Button>
        </span>
      )}

      {nota ? <span className="recording-note" title={nota}>{nota}</span> : null}
    </div>
  );
}

export function hasAudio(outcome: ChannelOutcome) {
  return outcome.state !== "unavailable";
}

export { relogio as formatMeetingClock };

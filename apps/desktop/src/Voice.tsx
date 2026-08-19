import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, appError, type VoiceResult } from "./api";
import { conversations } from "./hermes";
import type { VoiceNote as VoiceNoteSummary, VoiceStopped, VoiceTick } from "./types";
import {
  AMPLITUDE_BARS,
  amplitudeScale,
  formatElapsed,
  receiptOf,
  refusalLabel,
  remainingWarning,
} from "./voiceHud";

/**
 * O Voice Inbox dentro do Quick Capture.
 *
 * **Voz não é um modo, é uma forma de digitar** (`mos-design-system.md` §Voz).
 * Por isso ela não abre janela: ela acontece no mesmo overlay de 640px que o
 * texto já usa, sobre a mesma barra `/`, com os mesmos traços de amplitude que
 * o markup já reservava — e que o comentário do componente descrevia como
 * "apagados até a voz existir (fase adiada)".
 *
 * Segurar, e não alternar. Duas portas para o mesmo gesto:
 *
 * - `Ctrl+Alt+Space` **global**, tratado no Rust, que também revela a janela;
 * - `Alt` **dentro do HUD**, tratado aqui, que é o `⌥` que o design system pede.
 *
 * As duas chamam os mesmos comandos, e a máquina de estados é uma só.
 */

/** O teto de uma gravação, espelhando `mos_core::voice::MAX_DURATION_MS`. */
const MAX_DURATION_MS = 120_000;

/** ~15 Hz. É a mesma cadência da onda do Meeting Agent. */
const TICK_MS = 66;

/**
 * Os estados do HUD, e por que eles são um enum e não booleanos.
 *
 * `gravando` + `transcrevendo` + `temResultado` como flags independentes
 * permitiriam "gravando e com resultado na tela", que é impossível — e é
 * exatamente o tipo de estado que aparece uma vez em produção e ninguém
 * consegue reproduzir.
 */
export type VoiceStage =
  | { stage: "idle" }
  | { stage: "listening"; tick: VoiceTick | null }
  | { stage: "transcribing" }
  | { stage: "result"; result: VoiceResult }
  | { stage: "refused"; label: string }
  | { stage: "failed"; message: string; noteId: string; retryable: boolean };

export function useVoiceHud(onDone: () => void) {
  const [state, setState] = useState<VoiceStage>({ stage: "idle" });
  const [undone, setUndone] = useState(false);
  /* O estágio vivo, para os ouvintes de evento e os atalhos lerem sem que cada
     mudança recrie os `listen` — um `unlisten` por tecla digitada perderia
     evento no meio da troca. */
  const current = useRef<VoiceStage>(state);
  current.current = state;
  const dismissTimer = useRef<number | null>(null);

  const clearDismiss = useCallback(() => {
    if (dismissTimer.current !== null) {
      window.clearTimeout(dismissTimer.current);
      dismissTimer.current = null;
    }
  }, []);

  const scheduleDismiss = useCallback(
    (ms: number) => {
      clearDismiss();
      dismissTimer.current = window.setTimeout(() => {
        setState({ stage: "idle" });
        onDone();
      }, ms);
    },
    [clearDismiss, onDone],
  );

  const start = useCallback(async () => {
    if (current.current.stage === "listening") return;
    clearDismiss();
    setUndone(false);
    setState({ stage: "listening", tick: null });
    try {
      await api.voiceStart();
    } catch (error) {
      setState({ stage: "refused", label: appError(error).message });
      scheduleDismiss(2_400);
    }
  }, [clearDismiss, scheduleDismiss]);

  const stop = useCallback(async () => {
    if (current.current.stage !== "listening") return;
    try {
      const stopped: VoiceStopped = await api.voiceStop();
      applyStopped(stopped);
    } catch (error) {
      setState({ stage: "refused", label: appError(error).message });
      scheduleDismiss(2_400);
    }
  }, [scheduleDismiss]);

  const applyStopped = useCallback(
    (stopped: VoiceStopped) => {
      /* O outro caminho ja parou. Deixar o estagio como esta e o certo: o
         desfecho de verdade chega pelo evento que aquele caminho vai emitir. */
      if (stopped.outcome === "notRecording") return;
      if (stopped.outcome === "transcribing") {
        setState({ stage: "transcribing" });
        return;
      }
      // Nada foi persistido, então não há o que desfazer nem o que tentar de
      // novo: tentar de novo é falar de novo.
      setState({ stage: "refused", label: refusalLabel(stopped.outcome) });
      scheduleDismiss(1_600);
    },
    [scheduleDismiss],
  );

  const cancel = useCallback(async () => {
    clearDismiss();
    setState({ stage: "idle" });
    try {
      await api.voiceCancel();
    } catch {
      // Cancelar não tem falha visível: o que ele desfaz é uma gravação que a
      // pessoa já disse que não queria.
    }
  }, [clearDismiss]);

  /* A gravação viva. O laço só existe enquanto ela existe: um `setInterval`
     permanente acordaria o renderer a 15 Hz para ler `null`. */
  useEffect(() => {
    if (state.stage !== "listening") return;
    let alive = true;
    const timer = window.setInterval(() => {
      void api
        .voiceRecording()
        .then((tick) => {
          if (!alive || !tick) return;
          setState((previous) =>
            previous.stage === "listening" ? { stage: "listening", tick } : previous,
          );
        })
        .catch(() => undefined);
    }, TICK_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [state.stage]);

  /* Os eventos do Rust. Eles chegam pelos DOIS caminhos — o atalho global
     dispara do lado de lá, e nesse caso não houve chamada daqui para devolver
     resultado. */
  useEffect(() => {
    const disposers = [
      listen("voice-armed", () => setState({ stage: "listening", tick: null })),
      listen<VoiceStopped>("voice-stopped", (event) => applyStopped(event.payload)),
      listen("voice-transcribing", () => setState({ stage: "transcribing" })),
      listen<VoiceResult>("voice-captured", (event) => {
        setUndone(false);
        setState({ stage: "result", result: event.payload });
        scheduleDismiss(event.payload.receiptMs);
      }),
      listen<{ noteId: string; message: string; retryable: boolean }>("voice-failed", (event) => {
        // Falha NÃO some sozinha. Se o áudio continua em disco, sumir levaria
        // junto a única porta de volta para ele.
        clearDismiss();
        setState({
          stage: "failed",
          message: event.payload.message,
          noteId: event.payload.noteId,
          retryable: event.payload.retryable,
        });
      }),
      listen<string>("voice-refused", (event) => {
        setState({ stage: "refused", label: event.payload });
        scheduleDismiss(2_400);
      }),
      listen("voice-cancelled", () => {
        clearDismiss();
        setState({ stage: "idle" });
      }),
    ];
    return () => {
      disposers.forEach((disposer) => void disposer.then((dispose) => dispose()));
    };
  }, [applyStopped, clearDismiss, scheduleDismiss]);

  /* `Alt` segurado. `repeat` é a guarda: o auto-repeat do Windows dispara
     `keydown` continuamente enquanto a tecla está afundada, e sem ela cada
     repetição tentaria abrir o microfone de novo. */
  useEffect(() => {
    function down(event: KeyboardEvent) {
      if (event.key !== "Alt" || event.repeat) return;
      /* `Alt` COM `Ctrl` e o atalho global — `Ctrl+Alt+G` —, e ele ja e tratado
         no Rust, que inclusive revela esta janela. Entrar por aqui tambem faria
         o mesmo gesto abrir o microfone duas vezes. O `Alt` sozinho e a porta
         desta janela, e e a que o design system pede. */
      if (event.ctrlKey || event.metaKey) return;
      event.preventDefault();
      void start();
    }
    function up(event: KeyboardEvent) {
      if (event.key !== "Alt") return;
      void stop();
    }
    /* **Perder o foco NAO encerra a gravação**, e a ausência deste ouvinte é a
       decisão. Ele existia como terceira guarda do microfone, e rodar o app
       provou que ele era o oposto disso: numa fala iniciada pelo atalho
       global, o usuário está — por definição — trabalhando em outro programa.
       O Windows restringe a ativação em primeiro plano vinda de um processo em
       segundo plano, então o HUD aparece e o foco volta para onde estava; o
       `blur` disparava e matava a gravação em milissegundos. O sintoma era
       "Curto demais" para uma tecla que continuava afundada.

       O microfone continua com duas guardas contra um `Released` perdido, e
       elas não dependem de foco nenhum: o `Esc` e o teto de 120 s do watchdog,
       no Rust. */
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, [start, stop]);

  const accept = useCallback(async () => {
    if (current.current.stage !== "result") return;
    const { result } = current.current;
    if (result.executed || result.action === "keep") return;
    clearDismiss();
    try {
      const acted = await api.voiceAct(result.noteId);
      setState({ stage: "result", result: acted });
      scheduleDismiss(acted.receiptMs);
    } catch (error) {
      setState({ stage: "refused", label: appError(error).message });
      scheduleDismiss(2_400);
    }
  }, [clearDismiss, scheduleDismiss]);

  const undo = useCallback(async () => {
    if (current.current.stage !== "result") return;
    const step = current.current.result.undo;
    if (!step) return;
    clearDismiss();
    try {
      await conversations.undoAction(step);
      setUndone(true);
      scheduleDismiss(1_400);
    } catch (error) {
      setState({ stage: "refused", label: appError(error).message });
      scheduleDismiss(2_400);
    }
  }, [clearDismiss, scheduleDismiss]);

  const retry = useCallback(async () => {
    if (current.current.stage !== "failed") return;
    const { noteId } = current.current;
    setState({ stage: "transcribing" });
    try {
      await api.voiceRetry(noteId);
    } catch (error) {
      setState({ stage: "failed", message: appError(error).message, noteId, retryable: true });
    }
  }, []);

  const discard = useCallback(async () => {
    if (current.current.stage !== "failed") return;
    const { noteId } = current.current;
    setState({ stage: "idle" });
    try {
      await api.voiceDiscard(noteId);
    } catch {
      // Descartar já foi a decisão; falhar em apagar não a desfaz.
    }
    onDone();
  }, [onDone]);

  /** Fecha o recibo na hora, sem esperar o relogio. E o `Esc` depois do fato. */
  const dismiss = useCallback(() => {
    clearDismiss();
    setState({ stage: "idle" });
    onDone();
  }, [clearDismiss, onDone]);

  useEffect(() => () => clearDismiss(), [clearDismiss]);

  return { state, undone, start, stop, cancel, accept, undo, retry, discard, dismiss };
}

/** Os traços de amplitude. Em repouso são quatro traços apagados. */
export function Amplitude({ level, active }: { level: number; active: boolean }) {
  return (
    <span className="amplitude" data-listening={active || undefined} aria-hidden="true">
      {Array.from({ length: AMPLITUDE_BARS }, (_, index) => (
        <i key={index} style={active ? { transform: `scaleY(${amplitudeScale(level, index)})` } : undefined} />
      ))}
    </span>
  );
}

/**
 * A camada de voz do HUD.
 *
 * Devolve `null` em repouso: o campo de texto continua sendo o que aparece, e
 * a voz só toma a linha quando existe.
 */
export function VoiceSurface({ state, undone }: { state: VoiceStage; undone: boolean }) {
  const now = new Date();

  if (state.stage === "listening") {
    const ms = state.tick?.durationMs ?? 0;
    const aviso = remainingWarning(ms, MAX_DURATION_MS);
    return (
      <p className="voice-line" role="status">
        <span className="voice-live">Ouvindo</span>
        <span className="voice-clock">{formatElapsed(ms)}</span>
        {aviso ? <span className="voice-cap">resta {aviso}</span> : null}
        {state.tick?.problem ? <span className="voice-problem">{state.tick.problem}</span> : null}
      </p>
    );
  }

  if (state.stage === "transcribing") {
    return (
      <p className="voice-line" role="status">
        <span className="voice-pending">Entendendo</span>
      </p>
    );
  }

  if (state.stage === "refused") {
    return (
      <p className="voice-line" role="status">
        <span className="voice-pending">{state.label}</span>
      </p>
    );
  }

  if (state.stage === "failed") {
    /* Linguagem de warning: ícone e frase, nunca só cor (AGENTS.md §4.4). E a
       frase diz onde o áudio está — "falhou" sem saída é um beco. */
    return (
      <p className="voice-line" role="status">
        <span className="voice-warning" aria-hidden="true">
          !
        </span>
        <span className="voice-subject">{state.message}</span>
        {state.retryable ? <span className="voice-meta">O áudio está guardado.</span> : null}
      </p>
    );
  }

  if (state.stage === "result") {
    const receipt = receiptOf(state.result, now);
    return (
      <p className="voice-line" role="status">
        <span className="micro-label">{undone ? "DESFEITO" : receipt.headline}</span>
        <span className="voice-subject">{receipt.subject}</span>
        {receipt.meta && !undone ? <span className="voice-meta">{receipt.meta}</span> : null}
      </p>
    );
  }

  return null;
}

/** O rodapé: o que dá para fazer agora, e nada além disso. */
export function VoiceFooter({
  state,
  undone,
  onAccept,
  onUndo,
  onRetry,
  onDiscard,
}: {
  state: VoiceStage;
  undone: boolean;
  onAccept: () => void;
  onUndo: () => void;
  onRetry: () => void;
  onDiscard: () => void;
}) {
  if (state.stage === "listening") {
    return <span className="micro-label">SOLTE PARA GUARDAR · ESC CANCELA</span>;
  }
  if (state.stage === "transcribing") {
    return <span className="micro-label">TRANSCREVENDO LOCALMENTE</span>;
  }
  if (state.stage === "failed") {
    return (
      <span className="voice-actions">
        {state.retryable ? (
          <button type="button" onClick={onRetry}>
            TENTAR DE NOVO
          </button>
        ) : null}
        <button type="button" onClick={onDiscard}>
          DESCARTAR
        </button>
      </span>
    );
  }
  if (state.stage === "result" && !undone) {
    const receipt = receiptOf(state.result, new Date());
    if (receipt.offer) {
      return (
        <span className="voice-actions">
          <button type="button" onClick={onAccept}>
            {receipt.offer.toUpperCase()}
          </button>
        </span>
      );
    }
    if (state.result.undo) {
      return (
        <span className="voice-actions">
          <button type="button" onClick={onUndo}>
            DESFAZER · CTRL Z
          </button>
        </span>
      );
    }
    return <span className="micro-label">NA INBOX</span>;
  }
  return null;
}

/**
 * O áudio que ficou esperando, entre uma abertura do M/OS e outra.
 *
 * Ele existe por uma razão só, e ela é o critério F: **se a transcrição
 * falhar, a fala não pode virar um arquivo que ninguém encontra.** O HUD já
 * mostra a falha no instante em que ela acontece, mas o HUD some — e um M/OS
 * fechado no meio de uma gravação não deixa HUD nenhum na tela da próxima
 * abertura.
 *
 * Mora na Inbox porque é onde o que ainda não foi decidido mora. E aparece
 * mesmo com a Inbox vazia: uma Inbox que se diz limpa enquanto há áudio
 * pendente estaria mentindo sobre a única coisa que ela promete.
 */
export function PendingVoice() {
  const [notes, setNotes] = useState<VoiceNoteSummary[]>([]);
  const [busy, setBusy] = useState("");
  const [problem, setProblem] = useState("");

  const load = useCallback(() => {
    void api
      .voicePending()
      .then((pending) => setNotes(pending.filter((note) => !note.audioDeletedAt)))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    load();
    const disposers = [
      listen("voice-captured", load),
      listen("voice-failed", load),
      listen("voice-cancelled", load),
    ];
    return () => {
      disposers.forEach((disposer) => void disposer.then((dispose) => dispose()));
    };
  }, [load]);

  if (!notes.length) return null;

  async function run(id: string, action: "retry" | "discard") {
    setBusy(id);
    setProblem("");
    try {
      if (action === "retry") await api.voiceRetry(id);
      else await api.voiceDiscard(id);
    } catch (error) {
      setProblem(appError(error).message);
    } finally {
      setBusy("");
      load();
    }
  }

  return (
    <section className="voice-pending-list" aria-label="Áudio de voz por transcrever">
      <span className="micro-label">
        {notes.length === 1 ? "1 FALA POR TRANSCREVER" : `${notes.length} FALAS POR TRANSCREVER`}
      </span>
      {notes.map((note) => (
        <div className="voice-pending-row" key={note.id}>
          <span className="voice-pending-when">
            {formatElapsed(note.durationMs)} · {new Date(note.startedAt).toLocaleString("pt-BR")}
          </span>
          {/* A frase da falha vai junto: "por transcrever" sem motivo manda a
              pessoa tentar de novo às cegas. */}
          {note.failureMessage ? <span className="voice-meta">{note.failureMessage}</span> : null}
          <span className="voice-actions">
            <button type="button" disabled={busy === note.id} onClick={() => void run(note.id, "retry")}>
              {busy === note.id ? "TRANSCREVENDO" : "TRANSCREVER"}
            </button>
            <button type="button" disabled={busy === note.id} onClick={() => void run(note.id, "discard")}>
              DESCARTAR
            </button>
          </span>
        </div>
      ))}
      {problem ? <span className="voice-problem">{problem}</span> : null}
    </section>
  );
}

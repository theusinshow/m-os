import { useState } from "react";
import { useMonitorStore } from "@/stores/monitorStore";
import { useTimerStore } from "@/stores/timerStore";
import { quitApp } from "@/services/app";
import { formatDuration } from "@/lib/format";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Field, Input } from "@/components/ui/Field";

/**
 * Lembretes de monitoramento (secao 10). Renderizado no layout para aparecer em
 * qualquer tela. Nunca vincula projeto nem encerra silenciosamente: sempre
 * apresenta a decisao ao usuario.
 */
export function MonitorPrompts() {
  return (
    <>
      <ClosePrompt />
      <IdlePrompt />
      <QuitPrompt />
    </>
  );
}

function QuitPrompt() {
  const quitRequested = useMonitorStore((s) => s.quitRequested);
  const clearQuit = useMonitorStore((s) => s.clearQuit);
  const stop = useTimerStore((s) => s.stop);
  const pause = useTimerStore((s) => s.pause);
  const [busy, setBusy] = useState(false);

  if (!quitRequested) return null;

  async function act(before?: () => Promise<void>) {
    setBusy(true);
    try {
      if (before) await before();
      await quitApp();
    } catch {
      setBusy(false);
    }
  }

  return (
    <Modal
      open
      title="Sair com cronometro ativo"
      onClose={clearQuit}
      footer={
        <>
          <Button variant="ghost" onClick={clearQuit} disabled={busy}>
            Cancelar
          </Button>
          <Button variant="ghost" onClick={() => void act()} disabled={busy}>
            Sair assim mesmo
          </Button>
          <Button
            variant="danger"
            onClick={() => void act(stop)}
            disabled={busy}
          >
            Encerrar e sair
          </Button>
          <Button
            variant="primary"
            onClick={() => void act(pause)}
            disabled={busy}
          >
            Pausar e sair
          </Button>
        </>
      }
    >
      <p className="text-sm text-text">
        Ha um cronometro em andamento. Como deseja sair?
      </p>
      <p className="mt-2 text-xs text-text-muted">
        Se sair assim mesmo, o cronometro sera recuperado na proxima abertura.
      </p>
    </Modal>
  );
}

function IdlePrompt() {
  const idleSeconds = useMonitorStore((s) => s.idleSeconds);
  const clearIdle = useMonitorStore((s) => s.clearIdle);
  const discountIdle = useTimerStore((s) => s.discountIdle);
  const activeTimer = useTimerStore((s) => s.activeTimer);

  const [minutes, setMinutes] = useState("0");
  const [initialized, setInitialized] = useState(false);
  const [busy, setBusy] = useState(false);

  // Preenche o campo com o periodo detectado ao abrir.
  if (idleSeconds !== null && !initialized) {
    setMinutes(String(Math.round(idleSeconds / 60)));
    setInitialized(true);
  }
  if (idleSeconds === null && initialized) setInitialized(false);

  // Sem cronometro ativo, nao ha o que descontar.
  if (idleSeconds === null || !activeTimer) return null;

  async function discount() {
    setBusy(true);
    try {
      const mins = Math.max(0, Number(minutes.replace(",", ".")) || 0);
      await discountIdle(Math.round(mins * 60));
      clearIdle();
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      open
      title="Voce ficou sem atividade"
      onClose={clearIdle}
      footer={
        <>
          <Button variant="secondary" onClick={clearIdle} disabled={busy}>
            Manter como trabalhado
          </Button>
          <Button
            variant="primary"
            onClick={() => void discount()}
            disabled={busy}
          >
            Descontar
          </Button>
        </>
      }
    >
      <p className="text-sm text-text">
        Detectamos {formatDuration(idleSeconds)} sem uso do teclado ou mouse.
        O que deseja fazer com esse periodo?
      </p>
      <div className="mt-4 max-w-[12rem]">
        <Field label="Minutos a descontar" htmlFor="idle-min">
          <Input
            id="idle-min"
            inputMode="numeric"
            value={minutes}
            onChange={(e) => setMinutes(e.target.value)}
          />
        </Field>
      </div>
      <p className="mt-3 text-2xs text-text-subtle">
        O tempo real e sempre preservado; o desconto afeta apenas a duracao
        liquida e o valor da sessao.
      </p>
    </Modal>
  );
}

function ClosePrompt() {
  const closeApp = useMonitorStore((s) => s.closeApp);
  const clearClose = useMonitorStore((s) => s.clearClose);
  const stop = useTimerStore((s) => s.stop);
  const pause = useTimerStore((s) => s.pause);

  const [busy, setBusy] = useState(false);

  if (!closeApp) return null;

  async function act(action: () => Promise<void> | void) {
    setBusy(true);
    try {
      await action();
      clearClose();
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      open
      title={`${closeApp.displayName} foi fechado`}
      onClose={clearClose}
      footer={
        <>
          <Button variant="ghost" onClick={clearClose} disabled={busy}>
            Manter ativo
          </Button>
          <Button
            variant="danger"
            onClick={() => void act(stop)}
            disabled={busy}
          >
            Encerrar
          </Button>
          <Button
            variant="primary"
            onClick={() => void act(pause)}
            disabled={busy}
          >
            Pausar
          </Button>
        </>
      }
    >
      <p className="text-sm text-text">
        Ha um cronometro em andamento. Deseja encerrar o registro atual?
      </p>
    </Modal>
  );
}

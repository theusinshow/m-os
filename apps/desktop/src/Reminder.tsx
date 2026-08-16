import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import type { PendingReminder, Project } from "./types";

/**
 * O instante em que o "hoje" de quem clicou acaba.
 *
 * Calculado AQUI e mandado pronto para o backend, que só conhece UTC. Meia-noite
 * em UTC é nove da noite no Brasil — o lembrete voltaria a incomodar justamente
 * na hora extra, que é quando ele mais atrapalha.
 */
function endOfToday() {
  const midnight = new Date();
  midnight.setHours(23, 59, 59, 999);
  return midnight.toISOString();
}

/**
 * A janelinha que aparece sobre o CAD.
 *
 * Existe porque a notificação do sistema some sozinha e some no canto: quem
 * está desenhando não olha para a bandeja. Esta fica na tela, sobre o AutoCAD,
 * até alguém responder — e a resposta é UM clique, porque o Project já vem
 * escolhido.
 *
 * Ela **não rouba o foco**. Uma janela que captura o teclado no meio de um
 * comando de CAD não é um lembrete, é um acidente.
 */
export function Reminder() {
  const [pending, setPending] = useState<PendingReminder | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [choice, setChoice] = useState("");
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState("");

  /** Recarrega tudo: o lembrete pode ser outro programa, e o último Project
   *  trabalhado pode ter mudado desde a última vez que a janela abriu. */
  const load = useCallback(async () => {
    setNote("");
    const [next, list, entries] = await Promise.all([
      api.reminderPending().catch(() => null),
      api.projects().catch(() => [] as Project[]),
      api.trackingEntries().catch(() => []),
    ]);
    setPending(next);
    const active = list.filter((project) => project.lifecycleState === "active");
    setProjects(active);
    // O último Project trabalhado, porque quem reabre o CAD quase sempre volta
    // para a mesma obra. Uma escolha errada custa um clique; um seletor vazio
    // custa a decisão inteira, que é onde o registro se perde.
    const last = entries.find((entry) => active.some((project) => project.id === entry.projectId));
    setChoice(last?.projectId ?? active[0]?.id ?? "");
  }, []);

  useEffect(() => { void load(); }, [load]);

  // A janela sobrevive entre lembretes — é escondida, não destruída. Sem ouvir
  // o evento, o segundo lembrete mostraria o texto do primeiro.
  useEffect(() => {
    const unlisten = listen<PendingReminder>("reminder", () => { void load(); });
    return () => { void unlisten.then((dispose) => dispose()); };
  }, [load]);

  async function act(run: () => Promise<unknown>) {
    setBusy(true);
    setNote("");
    try {
      await run();
      await api.reminderDismiss();
    } catch (error) {
      // O erro fica NA janelinha. Ela é o único lugar que o usuário está
      // olhando agora, e mandá-lo abrir o M/OS para descobrir o que houve
      // desfaz o motivo de a janelinha existir.
      setNote(error instanceof Error ? error.message : String(error));
      setBusy(false);
      return;
    }
    setBusy(false);
  }

  if (!pending) {
    return (
      <main className="reminder-shell">
        <div className="reminder-body">
          <p className="support-copy">Nada pendente.</p>
          <Button variant="ghost" size="sm" onClick={() => void api.reminderDismiss()}>Fechar</Button>
        </div>
      </main>
    );
  }

  const opened = pending.opened;
  return (
    <main className="reminder-shell">
      <header className="reminder-head">
        <span className="micro-label">M/OS · TEMPO</span>
        <button
          type="button"
          className="reminder-close"
          aria-label="Fechar"
          onClick={() => void api.reminderDismiss()}
        >
          ×
        </button>
      </header>

      <div className="reminder-body">
        <p className="reminder-title">
          <strong>{pending.displayName}</strong> {opened ? "foi aberto" : "foi fechado"}
        </p>
        <p className="support-copy">
          {opened
            ? "O cronômetro não está contando. Começo agora?"
            : "O cronômetro continua correndo. Encerro a sessão?"}
        </p>

        {opened ? (
          projects.length ? (
            <>
              <label className="visually-hidden" htmlFor="reminder-project">Project</label>
              <select
                id="reminder-project"
                value={choice}
                onChange={(event) => setChoice(event.currentTarget.value)}
              >
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>{project.name}</option>
                ))}
              </select>
              <div className="reminder-actions">
                <Button
                  variant="primary"
                  size="sm"
                  disabled={busy || !choice}
                  onClick={() => void act(() => api.timerStart(choice, "", "other"))}
                >
                  Iniciar
                </Button>
                <Button variant="ghost" size="sm" disabled={busy} onClick={() => void api.reminderDismiss()}>
                  Agora não
                </Button>
                {/* O silêncio é por PROGRAMA e por hoje, e não geral: quem se
                    cansou do lembrete do AutoCAD hoje não pediu para perder o
                    do Revit amanhã. */}
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={busy}
                  onClick={() => void api.reminderSuppress(pending.processName, endOfToday())}
                >
                  Não lembrar hoje
                </Button>
              </div>
            </>
          ) : (
            <p className="support-copy">Crie um Project no M/OS para cronometrar tempo nele.</p>
          )
        ) : (
          <div className="reminder-actions">
            <Button
              variant="primary"
              size="sm"
              disabled={busy}
              onClick={() => void act(() => api.timerStop())}
            >
              Encerrar
            </Button>
            <Button variant="ghost" size="sm" disabled={busy} onClick={() => void api.reminderDismiss()}>
              Manter contando
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => void api.reminderSuppress(pending.processName, endOfToday())}
            >
              Não lembrar hoje
            </Button>
          </div>
        )}

        {note ? <p className="inline-error" role="alert">! {note}</p> : null}
      </div>
    </main>
  );
}

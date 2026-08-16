import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import type { ActiveTimer, Project, TimeEntry } from "./types";

/** Quantos atalhos de "começar agora" cabem antes de virarem uma lista. */
const QUICK = 3;

/**
 * Segundos que este cronômetro já contou.
 *
 * A regra é a mesma do `elapsed_seconds` do Rust, inclusive o delta negativo
 * virando zero — o relógio do sistema pode andar para trás, e um cronômetro que
 * anda junto apagaria trabalho na frente do usuário.
 *
 * A duplicação é deliberada e limitada: aqui ela só DESENHA. O número que vira
 * sessão gravada é calculado no backend, a partir dos timestamps persistidos, e
 * é ele que conta o dinheiro. Se as duas contas divergirem por um segundo, a que
 * vale continua sendo a de lá.
 */
function elapsedOf(timer: ActiveTimer, nowMs: number) {
  const accumulated = Math.max(0, timer.accumulatedSeconds);
  if (timer.status === "paused") return accumulated;
  const since = Math.floor((nowMs - new Date(timer.lastResumedAt).getTime()) / 1000);
  return accumulated + Math.max(0, since);
}

/** `26:03:12` — horas nunca viram dias. Quem cronometra trabalho lê horas. */
function clockOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${pad(hours)}:${pad(minutes)}:${pad(rest)}`;
}

/**
 * O cronômetro na Home.
 *
 * Só existe quando há Project: cronometrar exige saber para onde vai a hora, e
 * um seletor vazio prometeria uma função que não pode ser cumprida.
 */
export function Timer({ projects, entries = [], onChanged }: {
  projects: Project[];
  /** As sessões recentes, para o começo em um clique saber o que oferecer. */
  entries?: TimeEntry[];
  onChanged: () => void;
}) {
  const [timer, setTimer] = useState<ActiveTimer | null>(null);
  const [choice, setChoice] = useState("");
  const [note, setNote] = useState("");
  const [working, setWorking] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const frame = useRef<number | null>(null);

  const load = useCallback(async () => {
    setTimer(await api.timerCurrent().catch(() => null));
  }, []);

  useEffect(() => { void load(); }, [load]);

  // Só corre quando há o que correr: um intervalo ligado com o cronômetro
  // pausado gastaria um render por segundo para redesenhar o mesmo número.
  useEffect(() => {
    if (timer?.status !== "running") return;
    const handle = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(handle);
  }, [timer?.status]);

  useEffect(() => () => { if (frame.current) window.clearTimeout(frame.current); }, []);

  async function act(run: () => Promise<unknown>) {
    setWorking(true);
    setNote("");
    try {
      await run();
      await load();
      onChanged();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
    setWorking(false);
  }

  const active = projects.filter((project) => project.lifecycleState === "active");
  const projectName = timer
    ? projects.find((project) => project.id === timer.projectId)?.name ?? "Project removido"
    : "";

  /**
   * Os Projects em que se trabalhou por último, do mais recente para o mais
   * antigo.
   *
   * `entries` já vem ordenada por início decrescente, então a primeira aparição
   * de cada Project é a mais recente — não é preciso ordenar de novo.
   */
  const recent = useMemo(() => {
    const seen: Project[] = [];
    for (const entry of entries) {
      if (seen.length >= QUICK) break;
      if (seen.some((project) => project.id === entry.projectId)) continue;
      const project = active.find((candidate) => candidate.id === entry.projectId);
      if (project) seen.push(project);
    }
    return seen;
  }, [entries, active]);

  // Pré-seleciona o último Project trabalhado. Quem cronometra tende a voltar
  // para a mesma obra, e um seletor em branco cobra uma escolha que o histórico
  // já respondeu.
  useEffect(() => {
    if (choice || !active.length) return;
    setChoice(recent[0]?.id ?? active[0].id);
  }, [choice, active, recent]);

  if (!timer) {
    if (!active.length) {
      return <p className="empty-state">Crie um Project para cronometrar tempo nele.</p>;
    }
    return (
      <div className="timer-idle">
        <label className="visually-hidden" htmlFor="timer-project">Project</label>
        <select id="timer-project" value={choice} onChange={(event) => setChoice(event.currentTarget.value)}>
          <option value="">Escolha um Project</option>
          {active.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
        </select>
        <Button
          variant="primary"
          size="sm"
          disabled={!choice || working}
          onClick={() => void act(() => api.timerStart(choice, "", "other"))}
        >
          Iniciar
        </Button>
        {/* Começar em um clique: o atrito entre "vou trabalhar" e "estou
            contando" é exatamente onde o registro se perde. Só aparece com
            histórico — antes dele, seriam botões sem resposta para dar. */}
        {recent.length ? (
          <div className="timer-quick">
            <span className="micro-label">RETOMAR</span>
            {recent.map((project) => (
              <Button
                key={project.id}
                variant="ghost"
                size="sm"
                disabled={working}
                onClick={() => void act(() => api.timerStart(project.id, "", "other"))}
              >
                {project.name}
              </Button>
            ))}
          </div>
        ) : null}
        {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
      </div>
    );
  }

  const seconds = elapsedOf(timer, now);
  return (
    <div className="timer-live" data-status={timer.status}>
      <p className="timer-clock" aria-live="off">{clockOf(seconds)}</p>
      <p className="timer-project">{projectName}</p>
      <div className="button-line">
        <Button
          variant="outline"
          size="sm"
          disabled={working}
          onClick={() => void act(() => api.timerSetRunning(timer.status !== "running"))}
        >
          {timer.status === "running" ? "Pausar" : "Retomar"}
        </Button>
        <Button variant="secondary" size="sm" disabled={working} onClick={() => void act(() => api.timerStop())}>
          Encerrar
        </Button>
      </div>
      {/* O leitor de tela não acompanha o relógio segundo a segundo — seria
          ruído contínuo. Ele recebe o estado, que é o que muda de verdade. */}
      <span className="visually-hidden" aria-live="polite">
        {timer.status === "running" ? "Cronômetro correndo" : "Cronômetro pausado"} em {projectName}
      </span>
      {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
    </div>
  );
}

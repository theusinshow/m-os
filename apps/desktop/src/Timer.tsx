import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { ACTIVITY } from "./TempoShared";
import type { ActiveTimer, ActivityType, Project, TimeEntry } from "./types";

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
export function Timer({ projects, entries = [], onChanged, detailed = false }: {
  projects: Project[];
  /** As sessões recentes, para o começo em um clique saber o que oferecer. */
  entries?: TimeEntry[];
  onChanged: () => void;
  /**
   * A anatomia inteira do cronômetro, e não só o relógio.
   *
   * No Painel do CronoCAD ele é a intenção dominante da tela — a única
   * superfície elevada dela — e ganha rótulo, hora de início, relógio em corpo
   * de display e a linha de atividade. Na Home ele é UM widget entre vários, e
   * a mesma anatomia gastaria metade da altura da coluna para dizer o que o
   * número sozinho já diz.
   */
  detailed?: boolean;
}) {
  const [timer, setTimer] = useState<ActiveTimer | null>(null);
  const [choice, setChoice] = useState("");
  /* Atividade e descricao existiam na API desde sempre — `timerStart` recebe as
     tres coisas — e o formulario mandava `""` e `"other"` fixos. O CronoCAD
     pergunta as duas antes de comecar, e quem fatura precisa delas: a atividade
     e o que quebra o relatorio por tipo, e a descricao e o que faz uma sessao
     de tres horas ser reconhecivel um mes depois. */
  const [activity, setActivity] = useState<ActivityType>("drawing");
  const [description, setDescription] = useState("");
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
      <form
        className="timer-idle"
        data-detailed={detailed || undefined}
        onSubmit={(event) => {
          event.preventDefault();
          void act(() => api.timerStart(choice, description.trim(), activity));
        }}
      >
        {detailed ? (
          <header className="timer-live-head">
            <span className="micro-label">INICIAR TRABALHO</span>
            <span className="timer-since">O CRONÔMETRO CORRE COM O M/OS FECHADO</span>
          </header>
        ) : (
          <p className="support-copy">Escolha o projeto e comece a registrar as horas.</p>
        )}

        {/* Começar em um clique: o atrito entre "vou trabalhar" e "estou
            contando" é exatamente onde o registro se perde. Só aparece com
            histórico — antes dele, seriam botões sem resposta para dar.

            Vem ANTES do formulário porque é o caminho curto. Quem já sabe em que
            vai trabalhar não deveria atravessar três campos para dizer isso. */}
        {recent.length ? (
          <div className="timer-quick">
            <span className="micro-label">INICIAR RÁPIDO</span>
            {recent.map((project) => (
              <Button
                key={project.id}
                variant="outline"
                size="sm"
                disabled={working}
                onClick={() => void act(() => api.timerStart(project.id, "", activity))}
              >
                {project.name}
              </Button>
            ))}
          </div>
        ) : null}

        <div className="timer-fields">
          <div className="tempo-field">
            <label htmlFor="timer-project">Project <span aria-hidden="true">*</span></label>
            <select id="timer-project" required value={choice} onChange={(event) => setChoice(event.currentTarget.value)}>
              <option value="">Escolha um Project</option>
              {active.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
            </select>
          </div>
          <div className="tempo-field">
            <label htmlFor="timer-activity">Atividade</label>
            <select
              id="timer-activity"
              value={activity}
              onChange={(event) => setActivity(event.currentTarget.value as ActivityType)}
            >
              {ACTIVITY.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
            </select>
          </div>
        </div>

        <div className="tempo-field">
          <label htmlFor="timer-description">Descrição (opcional)</label>
          <input
            id="timer-description"
            value={description}
            onChange={(event) => setDescription(event.currentTarget.value)}
            placeholder="O que você vai fazer?"
          />
        </div>

        <div className="button-line">
          <Button variant="primary" size="sm" type="submit" disabled={!choice || working}>Iniciar</Button>
        </div>
        {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
      </form>
    );
  }

  const seconds = elapsedOf(timer, now);
  const startedAt = new Date(timer.lastResumedAt);
  const startedLabel = Number.isNaN(startedAt.getTime())
    ? null
    : `${String(startedAt.getHours()).padStart(2, "0")}:${String(startedAt.getMinutes()).padStart(2, "0")}`;

  return (
    <div className="timer-live" data-status={timer.status} data-detailed={detailed || undefined}>
      {detailed ? (
        <header className="timer-live-head">
          <span className="micro-label">
            {timer.status === "running" ? "SESSÃO EM CURSO" : "SESSÃO PAUSADA"}
          </span>
          {startedLabel ? <span className="timer-since">DESDE {startedLabel}</span> : null}
        </header>
      ) : null}

      <p className="timer-clock" aria-live="off">
        {clockOf(seconds)}
        {/* O caret só pisca enquanto o cronômetro corre. Pausado, ele para —
            um cursor piscando sobre um número parado prometeria movimento que
            não existe. `aria-hidden` porque ele não é texto: o estado já é
            anunciado pela região viva mais abaixo. */}
        {detailed ? <span className="timer-caret" aria-hidden="true" /> : null}
      </p>
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

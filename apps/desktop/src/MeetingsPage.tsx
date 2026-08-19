import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { conversations } from "./hermes";
import { Button } from "./Button";
import { formatMeetingClock } from "./RecordingBar";
import { EmptyState, Inspector, PageHeader, PaneHeader, Panel, StateMessage } from "./Surface";
import type {
  Confidence, InsightKind, Meeting, MeetingAnalysis, MeetingInsight,
  MeetingStatus, Project, TranscriptSegment,
} from "./types";

/**
 * A superfície de Reuniões.
 *
 * Duas views num controle segmentado, e **não quatro abas**. Controle segmentado
 * troca projeção da mesma informação; aba esconde coisas diferentes. Ações e
 * Decisões não são outra informação — são o resumo em outro nível de detalhe, e
 * são exatamente o que a pessoa veio ver. Escondê-las atrás de uma aba faria a
 * tela abrir vazia do conteúdo que a justifica.
 *
 * Só a transcrição merece view própria: é longa, tem busca e tem um modo de
 * leitura diferente.
 */

const STATUS_LABEL: Record<MeetingStatus, string> = {
  recording: "gravando",
  stopping: "encerrando",
  interrupted: "interrompida",
  recorded: "gravada",
  transcribing: "transcrevendo",
  transcribed: "transcrita",
  analyzing: "analisando",
  ready: "pronta",
  failed: "falhou",
  cancelled: "descartada",
};

/** O rótulo que a pessoa lê. O nome técnico nunca aparece. */
const KIND_LABEL: Record<InsightKind, string> = {
  my_action: "SUA AÇÃO",
  other_action: "AÇÃO DE OUTROS",
  decision: "DECISÃO",
  deadline: "PRAZO",
  follow_up: "FOLLOW-UP",
  open_question: "QUESTÃO EM ABERTO",
  risk: "RISCO",
  topic: "TÓPICO",
};

/** A ordem de leitura da Visão geral. É a ordem do §22.4. */
const SECTIONS: InsightKind[] = [
  "my_action", "decision", "other_action", "deadline",
  "follow_up", "open_question", "risk",
];

const CONFIDENCE_LABEL: Record<Confidence, string> = {
  high: "alta confiança",
  medium: "confiança média",
  low: "confiança baixa",
};

function dayLabel(iso: string) {
  const date = new Date(iso);
  const today = new Date();
  const same = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  if (same(date, today)) return "HOJE";
  const yesterday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1);
  if (same(date, yesterday)) return "ONTEM";
  return date.toLocaleDateString("pt-BR", { day: "2-digit", month: "long" }).toUpperCase();
}

function hourOf(iso: string) {
  return new Date(iso).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

/** `1h12` ou `42m`. Nunca segundos: numa reunião eles são ruído. */
function durationLabel(ms: number) {
  const minutes = Math.round(ms / 60000);
  if (minutes < 1) return "menos de 1m";
  const hours = Math.floor(minutes / 60);
  return hours ? `${hours}h${String(minutes % 60).padStart(2, "0")}` : `${minutes}m`;
}

// ---------------------------------------------------------------------------
// Evidência
// ---------------------------------------------------------------------------

/**
 * O `WHY?`.
 *
 * Clicar leva à fala que sustenta o item. Este botão é a diferença entre uma
 * afirmação e uma afirmação com procedência — e o documento é explícito: o
 * Meeting Agent não apresenta inferência como fato sem proveniência.
 */
function Evidence({ insight, segments, jump }: {
  insight: MeetingInsight;
  segments: TranscriptSegment[];
  jump: (segmentId: string) => void;
}) {
  if (!insight.evidence.length) {
    return (
      <p className="meeting-no-evidence">
        Sem evidência na transcrição. Confira antes de criar a Task.
      </p>
    );
  }
  return (
    <div className="meeting-evidence">
      {insight.evidence.map((evidence) => {
        const segment = segments.find((item) => item.id === evidence.segmentId);
        if (!segment) return null;
        return (
          <button
            key={`${evidence.segmentId}-${evidence.seq}`}
            type="button"
            className="meeting-evidence-link"
            onClick={() => jump(evidence.segmentId)}
            title={segment.text}
          >
            <span className="meeting-evidence-time">{formatMeetingClock(segment.startMs)}</span>
            <span className="meeting-evidence-who">{segment.channel === "mic" ? "VOCÊ" : "REMOTO"}</span>
            <span className="meeting-evidence-quote">{segment.text}</span>
          </button>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Criar Task a partir de um item
// ---------------------------------------------------------------------------

/**
 * O preview.
 *
 * **Todo item mostra preview, inclusive os de confiança alta.** O risco
 * classifica a consequência da ação; o preview responde a outra coisa — a
 * incerteza da interpretação. Numa reunião isso é extremo: ninguém escolheu
 * nada, alguém só falou.
 */
function AcceptDialog({ insight, projects, meetingProject, close, done }: {
  insight: MeetingInsight;
  projects: Project[];
  meetingProject: string | null;
  close: () => void;
  done: (action: { message: string; run: () => Promise<unknown> }) => void;
}) {
  const [title, setTitle] = useState(insight.text);
  const [projectId, setProjectId] = useState(meetingProject ?? "");
  const [remind, setRemind] = useState(Boolean(insight.dueHint));
  // O padrão é amanhã às 9h, e ele é uma SUGESTÃO editável — não uma leitura do
  // `dueHint`. Interpretar "sexta" aqui congelaria um palpite; mostrar um campo
  // põe a interpretação na tela, que é o que o §19 pede.
  const [when, setWhen] = useState(() => {
    const date = new Date();
    date.setDate(date.getDate() + 1);
    date.setHours(9, 0, 0, 0);
    // `datetime-local` quer hora local sem fuso.
    const pad = (value: number) => String(value).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  });
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState("");
  const first = useRef<HTMLInputElement>(null);

  useEffect(() => { first.current?.focus(); first.current?.select(); }, []);

  const submit = async () => {
    setBusy(true);
    setNote("");
    try {
      const receipt = await api.meetingAcceptInsight({
        insightId: insight.id,
        title,
        projectId: projectId || null,
        remindAt: remind ? new Date(when) : null,
      });
      done({
        message: receipt.reminderId ? "Task e lembrete criados" : "Task criada",
        run: () => conversations.undoAction(receipt.undo),
      });
      close();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
      setBusy(false);
    }
  };

  return (
    <div className="meeting-scrim" onClick={close}>
      <div
        className="meeting-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Criar Task a partir da reunião"
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => { if (event.key === "Escape") close(); }}
      >
        <header>
          <span className="micro-label">{KIND_LABEL[insight.kind]}</span>
          <h2>Criar Task</h2>
        </header>

        <label className="meeting-field">
          <span>Título</span>
          <input ref={first} value={title} onChange={(event) => setTitle(event.target.value)} />
        </label>

        <label className="meeting-field">
          <span>Project</span>
          <select value={projectId} onChange={(event) => setProjectId(event.target.value)}>
            <option value="">Sem Project</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>{project.name}</option>
            ))}
          </select>
        </label>

        <label className="meeting-field-inline">
          <input type="checkbox" checked={remind} onChange={(event) => setRemind(event.target.checked)} />
          <span>Criar lembrete</span>
          {insight.dueHint ? <em className="meeting-due-hint">na reunião: “{insight.dueHint}”</em> : null}
        </label>

        {remind ? (
          <label className="meeting-field">
            <span>Quando</span>
            <input type="datetime-local" value={when} onChange={(event) => setWhen(event.target.value)} />
          </label>
        ) : null}

        {note ? <StateMessage state="error" label="Não foi possível concluir" detail={note} /> : null}

        <footer className="form-actions">
          <Button variant="ghost" onClick={close}>Cancelar</Button>
          <Button onClick={() => void submit()} disabled={busy || !title.trim()}>
            {busy ? "Criando…" : "Criar"}
          </Button>
        </footer>
      </div>
    </div>
  );
}


/**
 * A tela de consentimento.
 *
 * **Uma vez, e não a cada reunião.** `UX-PRINCIPLES` §21 é explícito:
 * confirmações constantes ensinam a clicar sem ler, e uma tela jurídica
 * repetida seria pior que nenhuma porque ninguém a leria na décima vez.
 *
 * O que substitui a repetição é estado visível: barra de gravação persistente,
 * ícone no tray, e nenhum caminho de código que grave sem clique.
 */
function ConsentDialog({ close, granted }: { close: () => void; granted: () => void }) {
  const [busy, setBusy] = useState(false);
  return (
    <div className="meeting-scrim" onClick={close}>
      <div
        className="meeting-dialog meeting-consent"
        role="dialog"
        aria-modal="true"
        aria-label="Meeting Notes grava áudio"
        onClick={(event) => event.stopPropagation()}
      >
        <header><h2>Meeting Notes grava áudio</h2></header>
        <p>
          Enquanto estiver gravando, o M/OS captura o seu microfone e o áudio que
          sai pelos alto-falantes — o que inclui a voz das outras pessoas na
          chamada.
        </p>
        <p>
          O áudio fica neste computador e é apagado depois de processado. A
          transcrição é feita aqui. Para a análise, ela é enviada ao Hermes; você
          pode desligar isso em Settings.
        </p>
        <p>
          <b>Obter o consentimento dos outros participantes, quando necessário, é
          responsabilidade sua.</b>
        </p>
        <footer className="form-actions">
          <Button variant="ghost" onClick={close}>Cancelar</Button>
          <Button
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void api.meetingSetAnalysisConsent(true).then(granted).catch(() => setBusy(false));
            }}
          >Entendi, gravar</Button>
        </footer>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// A página
// ---------------------------------------------------------------------------

export function MeetingsPage({ projects, focus, receipt, refresh }: {
  projects: Project[];
  /** Abre direto numa reunião — usado pela barra de gravação e pela recuperação. */
  focus?: string | null;
  /**
   * O recibo do M/OS, que **é** o desfazer: ele só aparece quando há caminho de
   * volta (ADR-035). Confirmação sem volta não passa por aqui — ela vira uma
   * linha de estado na própria tela, que some sozinha.
   */
  receipt: (action: { message: string; run: () => Promise<unknown> }) => void;
  refresh: () => Promise<unknown>;
}) {
  const [meetings, setMeetings] = useState<Meeting[]>([]);
  const [chosenId, setChosenId] = useState<string | null>(focus ?? null);
  const [view, setView] = useState<"overview" | "transcript">("overview");
  const [segments, setSegments] = useState<TranscriptSegment[]>([]);
  const [insights, setInsights] = useState<MeetingInsight[]>([]);
  const [analysis, setAnalysis] = useState<MeetingAnalysis | null>(null);
  const [accepting, setAccepting] = useState<MeetingInsight | null>(null);
  const [query, setQuery] = useState("");
  const [note, setNote] = useState("");
  /** Confirmação sem volta. Some sozinha; não ocupa o recibo. */
  const [flash, setFlash] = useState("");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">(focus ? "detail" : "list");
  const [recording, setRecording] = useState(false);
  const [askConsent, setAskConsent] = useState(false);
  const inspector = useRef<HTMLElement>(null);
  const transcriptRef = useRef<HTMLDivElement>(null);

  const loadList = useCallback(async () => {
    try {
      setMeetings(await api.meetings(false));
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => { void loadList(); }, [loadList]);

  // Saber se já há gravação em curso é o que decide entre "Iniciar" e nada:
  // oferecer iniciar durante uma gravação daria um botão que só produz erro.
  useEffect(() => {
    void api.meetingRecording().then((tick) => setRecording(Boolean(tick))).catch(() => undefined);
  }, [meetings]);

  const start = useCallback(async () => {
    setNote("");
    try {
      // O consentimento é conferido ANTES de gravar, e não antes de analisar:
      // é a gravação que abre o microfone, e é ela que a pessoa precisa ter
      // autorizado uma vez.
      const consent = await api.meetingAnalysisConsent();
      if (!consent.granted) { setAskConsent(true); return; }
      const meeting = await api.meetingStart("", null);
      setRecording(true);
      setChosenId(meeting.id);
      setNarrowPane("detail");
      await loadList();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, [loadList]);

  // O backend avisa quando um estágio termina. Sem isto, uma transcrição de
  // vinte minutos só apareceria se a pessoa trocasse de tela e voltasse.
  useEffect(() => {
    const events = ["meeting-transcribed", "meeting-analyzed", "meeting-failed"];
    const offs = events.map((name) => listen(name, () => { void loadList(); void loadDetail(); }));
    return () => { offs.forEach((off) => void off.then((fn) => fn())); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadList, chosenId]);

  const chosen = useMemo(
    () => meetings.find((meeting) => meeting.id === chosenId) ?? null,
    [meetings, chosenId],
  );

  const loadDetail = useCallback(async () => {
    if (!chosenId) { setSegments([]); setInsights([]); setAnalysis(null); return; }
    try {
      const [transcript, items, summary] = await Promise.all([
        api.meetingTranscript(chosenId),
        api.meetingInsights(chosenId),
        api.meetingAnalysis(chosenId),
      ]);
      setSegments(transcript);
      setInsights(items);
      setAnalysis(summary);
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, [chosenId]);

  useEffect(() => { void loadDetail(); }, [loadDetail]);

  const act = async (run: () => Promise<unknown>, message?: string) => {
    setNote("");
    try {
      await run();
      await loadList();
      await loadDetail();
      if (message) {
        setFlash(message);
        window.setTimeout(() => setFlash(""), 4000);
      }
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  };

  const jump = (segmentId: string) => {
    setView("transcript");
    // O salto acontece depois do render da outra view.
    window.setTimeout(() => {
      const node = transcriptRef.current?.querySelector(`[data-segment="${segmentId}"]`);
      node?.scrollIntoView({ block: "center", behavior: "smooth" });
      node?.classList.add("is-target");
      window.setTimeout(() => node?.classList.remove("is-target"), 2000);
    }, 40);
  };

  const grouped = useMemo(() => {
    const groups = new Map<string, Meeting[]>();
    for (const meeting of meetings) {
      const key = dayLabel(meeting.startedAt);
      const list = groups.get(key) ?? [];
      list.push(meeting);
      groups.set(key, list);
    }
    return [...groups.entries()];
  }, [meetings]);

  const interrupted = useMemo(
    () => meetings.filter((meeting) => meeting.status === "interrupted"),
    [meetings],
  );
  const proposed = insights.filter((insight) => insight.status === "proposed");
  const filteredSegments = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return segments;
    return segments.filter((segment) => segment.text.toLowerCase().includes(needle));
  }, [segments, query]);

  return (
    <div className="page meetings-page">
      <PageHeader
        title="Reuniões"
        subtitle="O que foi dito, o que ficou decidido e o que você prometeu."
        actions={recording
          ? <span className="micro-label">GRAVANDO</span>
          : <Button variant="primary" onClick={() => void start()}>Iniciar Meeting Notes</Button>}
      />

      {interrupted.length ? (
        <div className="meeting-interrupted-notice" role="status">
          <p>
            {interrupted.length === 1
              ? "Uma reunião foi interrompida e espera decisão."
              : `${interrupted.length} reuniões foram interrompidas e esperam decisão.`}
          </p>
          <Button
            variant="ghost"
            onClick={() => {
              setChosenId(interrupted[0].id);
              setNarrowPane("detail");
              setView("overview");
            }}
          >Ver</Button>
        </div>
      ) : null}

      {note ? <StateMessage state="error" label="Não foi possível concluir" detail={note} /> : null}
      {flash ? <StateMessage state="saved" label={flash} /> : null}

      <div className="split-page inspector-page meetings-split">
        <section className="list-pane">
          <PaneHeader segments={["Reuniões"]} meta={`${meetings.length}`} />
          {meetings.length === 0 ? (
            <EmptyState>
              Nenhuma reunião ainda. Comece uma gravação para a primeira aparecer aqui.
            </EmptyState>
          ) : (
            <div className="meeting-groups">
              {grouped.map(([day, list]) => (
                <div className="meeting-group" key={day}>
                  <span className="micro-label">{day}</span>
                  {list.map((meeting) => (
                    <button
                      key={meeting.id}
                      type="button"
                      className="list-row meeting-row"
                      aria-current={meeting.id === chosenId ? "true" : undefined}
                      data-status={meeting.status}
                      onClick={() => { setChosenId(meeting.id); setView("overview"); setNarrowPane("detail"); }}
                    >
                      <span className="meeting-row-time">{hourOf(meeting.startedAt)}</span>
                      <span className="meeting-row-title">{meeting.title}</span>
                      <span className="meeting-row-duration">{durationLabel(meeting.durationMs)}</span>
                      <span className="meeting-row-meta">{STATUS_LABEL[meeting.status]}</span>
                    </button>
                  ))}
                </div>
              ))}
            </div>
          )}
        </section>

        <Inspector
          ref={inspector}
          label="Reunião"
          open={narrowPane === "detail"}
          onBack={() => setNarrowPane("list")}
          onEscape={() => setNarrowPane("list")}
        >
          {!chosen ? (
            <EmptyState>Escolha uma reunião.</EmptyState>
          ) : (
            <div className="meeting-detail">
              <header className="meeting-head">
                <h2>{chosen.title}</h2>
                <p className="meeting-head-meta">
                  {new Date(chosen.startedAt).toLocaleDateString("pt-BR", { day: "2-digit", month: "short" })}
                  {" · "}{hourOf(chosen.startedAt)}
                  {" · "}{durationLabel(chosen.durationMs)}
                  {chosen.projectId ? ` · ${projects.find((p) => p.id === chosen.projectId)?.name ?? "Project"}` : ""}
                </p>
                <ChannelHealth meeting={chosen} />
              </header>

              <MeetingActions meeting={chosen} act={act} refresh={refresh} />

              <div className="segmented" role="tablist" aria-label="Visão da reunião">
                <button
                  role="tab"
                  aria-selected={view === "overview"}
                  onClick={() => setView("overview")}
                >Visão geral</button>
                <button
                  role="tab"
                  aria-selected={view === "transcript"}
                  onClick={() => setView("transcript")}
                >Transcrição</button>
              </div>

              {view === "overview" ? (
                <div className="meeting-overview">
                  {analysis ? (
                    <Panel label="RESUMO">
                      <p className="meeting-summary">{analysis.summary}</p>
                      {analysis.windows > 1 ? (
                        <p className="meeting-windows">
                          A transcrição não coube num envio: foi analisada em {analysis.windows} partes.
                        </p>
                      ) : null}
                    </Panel>
                  ) : null}

                  {SECTIONS.map((kind) => {
                    const items = proposed.filter((insight) => insight.kind === kind);
                    if (!items.length) return null;
                    return (
                      <Panel label={KIND_LABEL[kind]} count={String(items.length)} key={kind}>
                        {items.map((insight) => (
                          <article className="meeting-insight" key={insight.id} data-confidence={insight.confidence}>
                            <p className="meeting-insight-text">{insight.text}</p>
                            <p className="meeting-insight-meta">
                              {insight.owner ? <span>{insight.owner}</span> : null}
                              {insight.dueHint ? <span>prazo: {insight.dueHint}</span> : null}
                              <span>{CONFIDENCE_LABEL[insight.confidence]}</span>
                            </p>
                            <Evidence insight={insight} segments={segments} jump={jump} />
                            <div className="meeting-insight-actions">
                              {kind === "my_action" || kind === "other_action" || kind === "deadline" || kind === "follow_up" ? (
                                <Button variant="ghost" onClick={() => setAccepting(insight)}>Criar Task</Button>
                              ) : null}
                              <Button
                                variant="ghost"
                                onClick={() => void act(() => api.meetingDismissInsight(insight.id))}
                              >Descartar</Button>
                            </div>
                          </article>
                        ))}
                      </Panel>
                    );
                  })}

                  {!analysis && !proposed.length ? (
                    <EmptyState>
                      {chosen.status === "transcribed"
                        ? "Transcrição pronta. A análise ainda não foi feita."
                        : "Nada analisado ainda."}
                    </EmptyState>
                  ) : null}
                </div>
              ) : (
                <div className="meeting-transcript" ref={transcriptRef}>
                  <label className="meeting-field">
                    <span className="micro-label">BUSCAR NA TRANSCRIÇÃO</span>
                    <input
                      value={query}
                      onChange={(event) => setQuery(event.target.value)}
                      placeholder="palavra ou frase"
                    />
                  </label>
                  <p className="micro-label">{filteredSegments.length} de {segments.length} segmentos</p>
                  {segments.length === 0 ? (
                    <EmptyState>Esta reunião ainda não foi transcrita.</EmptyState>
                  ) : (
                    filteredSegments.map((segment) => (
                      <p
                        className="meeting-line"
                        key={segment.id}
                        data-segment={segment.id}
                        data-channel={segment.channel}
                      >
                        <span className="meeting-line-time">{formatMeetingClock(segment.startMs)}</span>
                        <span className="meeting-line-who">{segment.channel === "mic" ? "VOCÊ" : "REMOTO"}</span>
                        <span className="meeting-line-text">{segment.text}</span>
                      </p>
                    ))
                  )}
                </div>
              )}
            </div>
          )}
        </Inspector>
      </div>

      {askConsent ? (
        <ConsentDialog
          close={() => setAskConsent(false)}
          granted={() => { setAskConsent(false); void start(); }}
        />
      ) : null}

      {accepting ? (
        <AcceptDialog
          insight={accepting}
          projects={projects}
          meetingProject={chosen?.projectId ?? null}
          close={() => setAccepting(null)}
          done={(action) => { receipt(action); void loadDetail(); void refresh(); }}
        />
      ) : null}
    </div>
  );
}

/**
 * A saúde dos canais, depois que a gravação terminou.
 *
 * Só aparece quando algo saiu do normal. Uma linha dizendo "os dois canais
 * funcionaram" em toda reunião viraria ruído, e o que precisa ser visível é a
 * exceção.
 */
function ChannelHealth({ meeting }: { meeting: Meeting }) {
  const problems: string[] = [];
  for (const [label, outcome] of [["Microfone", meeting.mic], ["Áudio do sistema", meeting.system]] as const) {
    if (outcome.state === "lost") {
      problems.push(`${label} caiu aos ${formatMeetingClock(outcome.atMs)}. O restante foi preservado.`);
    } else if (outcome.state === "unavailable") {
      problems.push(`${label} não foi capturado: ${outcome.reason}`);
    }
  }
  if (!problems.length) return null;
  return (
    <div className="meeting-health">
      {problems.map((problem) => <p key={problem}>{problem}</p>)}
    </div>
  );
}

/**
 * O que dá para fazer com esta reunião agora.
 *
 * Um botão por estado, e nunca todos ao mesmo tempo: oferecer "analisar" numa
 * reunião sem transcrição ensinaria que o botão às vezes não faz nada.
 */
function MeetingActions({ meeting, act, refresh }: {
  meeting: Meeting;
  act: (run: () => Promise<unknown>, message?: string) => Promise<void>;
  refresh: () => Promise<unknown>;
}) {
  const after = async (run: () => Promise<unknown>, message?: string) => {
    await act(run, message);
    await refresh();
  };

  if (meeting.status === "interrupted") {
    return (
      <div className="meeting-recovery">
        <p>
          Esta gravação foi interrompida. <b>{durationLabel(meeting.durationMs)}</b> foram recuperados.
        </p>
        <div className="meeting-insight-actions">
          <Button onClick={() => void after(() => api.meetingProcessRecovered(meeting.id), "Reunião recuperada")}>
            Processar
          </Button>
          <Button
            variant="ghost"
            onClick={() => void after(() => api.meetingDiscard(meeting.id), "Gravação descartada")}
          >
            Descartar
          </Button>
        </div>
      </div>
    );
  }

  if (meeting.status === "failed" && meeting.failure) {
    // A separação que o §20 exige: "a gravação está segura" e "perdi a
    // gravação" pedem respostas opostas, e o estágio é o que as distingue.
    const safe = meeting.failure.stage !== "audio";
    return (
      <div className="meeting-failure">
        <p>
          {safe ? "A gravação está segura. " : ""}
          {meeting.failure.message}
        </p>
        <Button onClick={() => void after(() => api.meetingRetry(meeting.id), "Tentando de novo")}>
          Tentar de novo
        </Button>
      </div>
    );
  }

  const buttons: React.ReactNode[] = [];
  if (meeting.status === "recorded") {
    buttons.push(
      <Button key="t" onClick={() => void after(() => api.meetingTranscribe(meeting.id), "Transcrevendo…")}>
        Transcrever
      </Button>,
    );
  }
  if (meeting.status === "transcribed" || meeting.status === "ready") {
    buttons.push(
      <Button key="a" variant={meeting.status === "ready" ? "ghost" : "primary"}
        onClick={() => void after(() => api.meetingAnalyze(meeting.id), "Analisando…")}>
        {meeting.status === "ready" ? "Analisar de novo" : "Analisar com o Hermes"}
      </Button>,
    );
  }
  if (meeting.status === "transcribing" || meeting.status === "analyzing") {
    return <p className="meeting-working">{STATUS_LABEL[meeting.status]}…</p>;
  }
  if (!buttons.length) return null;
  return <div className="meeting-insight-actions">{buttons}</div>;
}

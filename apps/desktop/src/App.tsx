import { DragEvent, FormEvent, KeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, appError } from "./api";
import { Icon, type IconName } from "./Icon";
import type { AppStatus, BackupInspection, Capture, Project, SearchItem, Task, TaskState } from "./types";
import "./App.css";

type Page = "home" | "inbox" | "projects" | "tasks" | "settings";
type UndoAction = { message: string; run: () => Promise<unknown> };
type Theme = "dark" | "light";

const stateOrder: TaskState[] = ["backlog", "doing", "done"];
const stateLabels: Record<TaskState, string> = { backlog: "Backlog", doing: "Doing", done: "Done" };
const relativeFormatter = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });

function relativeTime(value: string) {
  const seconds = Math.round((new Date(value).getTime() - Date.now()) / 1_000);
  if (Math.abs(seconds) < 60) return "agora";
  const minutes = Math.round(seconds / 60);
  if (Math.abs(minutes) < 60) return relativeFormatter.format(minutes, "minute");
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 24) return relativeFormatter.format(hours, "hour");
  return relativeFormatter.format(Math.round(hours / 24), "day");
}

function sourceLabel(source: Capture["source"]) {
  return source === "quick_capture" ? "Quick Capture" : "Home";
}

function Button({ variant = "secondary", children, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "outline" | "ghost" | "danger" }) {
  return <button className={`button ${variant}`} type="button" {...props}>{children}</button>;
}

function IconButton({ label, icon, active = false, onClick }: { label: string; icon: IconName; active?: boolean; onClick: () => void }) {
  return <button className="icon-button" type="button" aria-label={label} title={label} onClick={onClick}><Icon name={icon} filled={active} /></button>;
}

function ContextPath({ segments }: { segments: string[] }) {
  return <div className="context-path" aria-label={segments.join(" / ")}>{segments.map((segment, index) => <span key={`${segment}-${index}`} className={index === segments.length - 1 ? "current" : undefined}>{index ? <b>/</b> : null}{segment}</span>)}</div>;
}

function Panel({ label, action, children, className = "" }: { label: string; action?: ReactNode; children: ReactNode; className?: string }) {
  return <section className={`panel ${className}`}><header className="panel-header"><h2>{label}</h2>{action}</header>{children}</section>;
}

function EmptyState({ children }: { children: ReactNode }) {
  return <p className="empty-state">{children}</p>;
}

function CaptureComposer({ onSaved }: { onSaved: (capture: Capture) => void }) {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "success" | "error">("idle");
  const [feedback, setFeedback] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!content.trim() || state === "saving") return;
    setState("saving");
    setFeedback("Salvando localmente...");
    try {
      const capture = await api.createCapture(content, "home");
      setContent("");
      setState("success");
      setFeedback("Salvo na Inbox");
      onSaved(capture);
    } catch (error) {
      setState("error");
      setFeedback(`${appError(error).message} O texto continua aqui.`);
    }
  }

  return <form className="capture-field" onSubmit={submit}>
    <div className="capture-line"><span className="slash">/</span><textarea aria-label="Conteúdo da captura" value={content} onChange={(event) => setContent(event.currentTarget.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} placeholder="What's on your mind?" rows={1} /></div>
    <div className="capture-footer"><span className={`feedback ${state}`} aria-live="polite">{feedback}</span><Button variant="primary" type="submit" disabled={!content.trim() || state === "saving"}>{state === "saving" ? "Salvando" : "Capturar"}</Button></div>
  </form>;
}

function DataRow({ primary, meta, secondary, selected = false, completed = false, onClick, onKeyDown, onPointerDown, draggable, onDragStart, onDragEnd }: { primary: string; meta?: string; secondary?: string; selected?: boolean; completed?: boolean; onClick?: () => void; onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void; onPointerDown?: React.PointerEventHandler<HTMLButtonElement>; draggable?: boolean; onDragStart?: React.DragEventHandler<HTMLButtonElement>; onDragEnd?: React.DragEventHandler<HTMLButtonElement> }) {
  return <button className="data-row" type="button" data-selected={selected || undefined} data-completed={completed || undefined} onClick={onClick} onKeyDown={onKeyDown} onPointerDown={onPointerDown} draggable={draggable} onDragStart={onDragStart} onDragEnd={onDragEnd}><span className="row-copy"><strong>{primary}</strong>{secondary ? <small>{secondary}</small> : null}</span>{meta ? <span className="row-meta">{meta}</span> : null}</button>;
}

function HomePage({ recent, projects, tasks, refresh, openCapture, openProject, openTask }: { recent: Capture[]; projects: Project[]; tasks: Task[]; refresh: () => Promise<void>; openCapture: (capture: Capture) => void; openProject: (project: Project) => void; openTask: (task: Task) => void }) {
  const doing = tasks.filter((task) => task.state === "doing" && task.lifecycleState === "active").slice(0, 5);
  return <div className="page home-page">
    <ContextPath segments={["M/OS", "HOME"]} />
    <CaptureComposer onSaved={() => void refresh()} />
    <div className="home-sections">
      <Panel label="EM ANDAMENTO">{doing.length ? doing.map((task) => <DataRow key={task.id} primary={task.title} meta={stateLabels[task.state]} onClick={() => openTask(task)} />) : <EmptyState>Nenhuma Task em andamento.</EmptyState>}</Panel>
      <Panel label="RECENTES">{recent.length ? recent.map((capture) => <DataRow key={capture.id} primary={capture.content} meta={relativeTime(capture.capturedAt)} onClick={() => openCapture(capture)} />) : <EmptyState>Suas Captures recentes aparecerão aqui.</EmptyState>}</Panel>
      <Panel label="PROJECTS">{projects.filter((project) => project.lifecycleState === "active").slice(0, 5).map((project) => <DataRow key={project.id} primary={project.name} secondary={project.description || undefined} meta={relativeTime(project.updatedAt)} onClick={() => openProject(project)} />)}{!projects.length ? <EmptyState>Projects criados aparecerão aqui.</EmptyState> : null}</Panel>
    </div>
  </div>;
}

function CaptureTaskForm({ capture, projects, onCreated, cancel }: { capture: Capture; projects: Project[]; onCreated: (task: Task) => void; cancel: () => void }) {
  const [title, setTitle] = useState(capture.content);
  const [description, setDescription] = useState("");
  const [projectId, setProjectId] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    try {
      const task = await api.createTask(title, description, projectId || null, capture.id);
      onCreated(task);
    } catch (nextError) {
      setError(appError(nextError).message);
      setSaving(false);
    }
  }
  return <form className="stack-form" onSubmit={submit}>
    <label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={3} /></label>
    <label><span>PROJECT</span><select value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label>
    {error ? <p className="inline-error" role="alert">! {error}</p> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={!title.trim() || saving}>{saving ? "Salvando" : "Criar Task"}</Button></div>
  </form>;
}

function InboxPage({ captures, projects, refresh, receipt, openTask }: { captures: Capture[]; projects: Project[]; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openTask: (task: Task) => void }) {
  const [selectedId, setSelectedId] = useState(captures[0]?.id ?? "");
  const [taskForm, setTaskForm] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => { if (!captures.some((capture) => capture.id === selectedId)) setSelectedId(captures[0]?.id ?? ""); }, [captures, selectedId]);
  const selected = captures.find((capture) => capture.id === selectedId) ?? null;

  async function mutate(action: "processed" | "archive" | "trash") {
    if (!selected) return;
    try {
      if (action === "processed") {
        await api.markProcessed(selected.id);
        receipt({ message: "Capture marcada como processada.", run: () => api.moveToInbox(selected.id) });
      } else if (action === "archive") {
        await api.archive(selected.id);
        receipt({ message: "Capture arquivada.", run: () => api.restore(selected.id) });
      } else {
        await api.trash(selected.id);
        receipt({ message: "Capture movida para a Lixeira.", run: () => api.restore(selected.id) });
      }
      setError("");
      await refresh();
    } catch (nextError) { setError(appError(nextError).message); }
  }

  if (!captures.length) return <div className="page"><ContextPath segments={["M/OS", "INBOX"]} /><EmptyState>Inbox limpa.</EmptyState></div>;
  return <div className="split-page">
    <section className="list-pane"><ContextPath segments={["M/OS", "INBOX"]} /><div className="row-list">{captures.map((capture) => <DataRow key={capture.id} primary={capture.content} secondary={sourceLabel(capture.source)} meta={relativeTime(capture.capturedAt)} selected={capture.id === selectedId} onClick={() => { setSelectedId(capture.id); setTaskForm(false); }} />)}</div></section>
    {selected ? <article className="detail-pane"><header className="detail-header"><div><span className="micro-label">CAPTURE</span><h1>{selected.content}</h1><p>{sourceLabel(selected.source)} · {relativeTime(selected.capturedAt)}</p></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => void mutate("archive")}>Arquivar</button><button className="danger-text" onClick={() => void mutate("trash")}>Mover para a Lixeira</button></div></details></header>
      {error ? <p className="inline-error" role="alert">! {error}</p> : null}
      {taskForm ? <CaptureTaskForm capture={selected} projects={projects} cancel={() => setTaskForm(false)} onCreated={(task) => { setTaskForm(false); void refresh(); openTask(task); }} /> : <div className="detail-actions"><Button variant="primary" onClick={() => setTaskForm(true)}>Criar Task</Button><Button variant="ghost" onClick={() => void mutate("processed")}>Marcar como processada</Button></div>}
    </article> : null}
  </div>;
}

function ProjectForm({ project, cancel, saved }: { project?: Project; cancel: () => void; saved: (project: Project) => void }) {
  const [name, setName] = useState(project?.name ?? "");
  const [description, setDescription] = useState(project?.description ?? "");
  const [error, setError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    try { saved(project ? await api.updateProject(project.id, name, description) : await api.createProject(name, description)); }
    catch (nextError) { setError(appError(nextError).message); }
  }
  return <form className="stack-form" onSubmit={submit}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    {error ? <p className="inline-error" role="alert">! {error}</p> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={!name.trim()}>Salvar</Button></div>
  </form>;
}

function DirectTaskForm({ projectId = null, projects, cancel, saved }: { projectId?: string | null; projects: Project[]; cancel: () => void; saved: (task: Task) => void }) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [selectedProject, setSelectedProject] = useState(projectId ?? "");
  const [error, setError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    try { saved(await api.createTask(title, description, selectedProject || null)); }
    catch (nextError) { setError(appError(nextError).message); }
  }
  return <form className="stack-form compact-form" onSubmit={submit}>
    <label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={2} /></label>
    <label><span>PROJECT</span><select value={selectedProject} onChange={(event) => setSelectedProject(event.currentTarget.value)}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label>
    {error ? <p className="inline-error" role="alert">! {error}</p> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={!title.trim()}>Criar Task</Button></div>
  </form>;
}

function ProjectsPage({ projects, tasks, initialProjectId, refresh, openTask }: { projects: Project[]; tasks: Task[]; initialProjectId: string; refresh: () => Promise<void>; openTask: (task: Task) => void }) {
  const activeProjects = projects.filter((project) => project.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialProjectId || activeProjects[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new" | "task">("view");
  useEffect(() => { if (initialProjectId) setSelectedId(initialProjectId); }, [initialProjectId]);
  useEffect(() => { if (!activeProjects.some((project) => project.id === selectedId)) setSelectedId(activeProjects[0]?.id ?? ""); }, [activeProjects, selectedId]);
  const selected = activeProjects.find((project) => project.id === selectedId) ?? null;
  const relatedTasks = tasks.filter((task) => task.projectId === selectedId && task.lifecycleState === "active");
  return <div className="split-page projects-page">
    <section className="list-pane"><ContextPath segments={["M/OS", "PROJECTS"]} /><div className="list-command"><Button variant="outline" onClick={() => setMode("new")}>Novo Project</Button></div><div className="row-list">{activeProjects.map((project) => <DataRow key={project.id} primary={project.name} secondary={project.description || undefined} meta={`${tasks.filter((task) => task.projectId === project.id && task.lifecycleState === "active").length} TASKS`} selected={project.id === selectedId} onClick={() => { setSelectedId(project.id); setMode("view"); }} />)}</div>{!activeProjects.length && mode !== "new" ? <EmptyState>Crie um Project para reunir trabalho relacionado.</EmptyState> : null}</section>
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO PROJECT</span><ProjectForm cancel={() => setMode("view")} saved={(project) => { setSelectedId(project.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <ProjectForm project={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">PROJECT</span><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setProjectArchived(selected.id, true).then(refresh)}>Arquivar</button></div></details></header>{mode === "task" ? <DirectTaskForm projectId={selected.id} projects={projects} cancel={() => setMode("view")} saved={(task) => { setMode("view"); void refresh(); openTask(task); }} /> : <Panel label="TASKS" action={<Button variant="primary" onClick={() => setMode("task")}>Criar Task</Button>}>{relatedTasks.length ? relatedTasks.map((task) => <DataRow key={task.id} primary={task.title} meta={stateLabels[task.state]} completed={task.state === "done"} onClick={() => openTask(task)} />) : <EmptyState>Nenhuma Task neste Project.</EmptyState>}</Panel>}</>}</> : null}</article>
  </div>;
}

function BoardPage({ tasks, projects, refresh, openTask }: { tasks: Task[]; projects: Project[]; refresh: () => Promise<void>; openTask: (task: Task) => void }) {
  const [creating, setCreating] = useState(false);
  const [draggingTaskId, setDraggingTaskId] = useState<string | null>(null);
  const [dragOverState, setDragOverState] = useState<TaskState | null>(null);
  const pointerDrag = useRef<{ taskId: string; x: number; y: number; active: boolean } | null>(null);
  const suppressClickTaskId = useRef<string | null>(null);
  async function move(task: Task, state: TaskState) { if (task.state !== state) await api.setTaskState(task.id, state).then(refresh); }
  function draggedTask(event: DragEvent<HTMLElement>) {
    const id = event.dataTransfer.getData("text/task-id") || event.dataTransfer.getData("text/plain") || draggingTaskId;
    return tasks.find((item) => item.id === id);
  }
  function finishDrag() {
    setDraggingTaskId(null);
    setDragOverState(null);
  }
  function keyboardMove(event: KeyboardEvent<HTMLButtonElement>, task: Task) {
    if (!event.altKey || (event.key !== "ArrowLeft" && event.key !== "ArrowRight")) return;
    event.preventDefault();
    const index = stateOrder.indexOf(task.state);
    const next = stateOrder[Math.max(0, Math.min(stateOrder.length - 1, index + (event.key === "ArrowRight" ? 1 : -1)))];
    void move(task, next);
  }
  useEffect(() => {
    function columnFromPoint(x: number, y: number) {
      const column = document.elementFromPoint(x, y)?.closest<HTMLElement>(".kanban-column");
      const state = column?.dataset.kanbanState;
      return stateOrder.includes(state as TaskState) ? state as TaskState : null;
    }
    function handlePointerMove(event: PointerEvent) {
      const drag = pointerDrag.current;
      if (!drag) return;
      if (!drag.active && Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < 6) return;
      drag.active = true;
      suppressClickTaskId.current = drag.taskId;
      setDraggingTaskId(drag.taskId);
      setDragOverState(columnFromPoint(event.clientX, event.clientY));
    }
    function handlePointerUp(event: PointerEvent) {
      const drag = pointerDrag.current;
      if (!drag) return;
      pointerDrag.current = null;
      if (!drag.active) return;
      const targetState = columnFromPoint(event.clientX, event.clientY);
      const task = tasks.find((item) => item.id === drag.taskId);
      finishDrag();
      if (task && targetState) void move(task, targetState);
      window.setTimeout(() => { if (suppressClickTaskId.current === drag.taskId) suppressClickTaskId.current = null; }, 0);
    }
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", finishDrag);
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", finishDrag);
    };
  }, [tasks, refresh]);
  return <div className="page board-page"><div className="board-heading"><ContextPath segments={["M/OS", "TASKS"]} />{!creating ? <Button variant="primary" onClick={() => setCreating(true)}>Criar Task</Button> : null}</div>{creating ? <DirectTaskForm projects={projects} cancel={() => setCreating(false)} saved={() => { setCreating(false); void refresh(); }} /> : null}<div className="kanban">{stateOrder.map((state) => { const column = tasks.filter((task) => task.lifecycleState === "active" && task.state === state); const visible = column.slice(0, 20); return <section key={state} className="kanban-column" data-kanban-state={state} data-drop-target={dragOverState === state || undefined} onDragEnter={(event) => { event.preventDefault(); setDragOverState(state); }} onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setDragOverState(state); }} onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragOverState(null); }} onDrop={(event) => { event.preventDefault(); const task = draggedTask(event); finishDrag(); if (task) void move(task, state); }}><header><h2>{stateLabels[state]}</h2><span>{column.length}</span></header><div>{visible.map((task) => <DataRow key={task.id} primary={task.title} secondary={projects.find((project) => project.id === task.projectId)?.name} completed={task.state === "done"} onClick={() => { if (suppressClickTaskId.current === task.id) { suppressClickTaskId.current = null; return; } openTask(task); }} onKeyDown={(event) => keyboardMove(event, task)} onPointerDown={(event) => { if (event.button !== 0) return; pointerDrag.current = { taskId: task.id, x: event.clientX, y: event.clientY, active: false }; }} draggable onDragStart={(event) => { setDraggingTaskId(task.id); event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/task-id", task.id); event.dataTransfer.setData("text/plain", task.id); }} onDragEnd={finishDrag} />)}{!column.length ? <EmptyState>Nenhuma Task.</EmptyState> : null}{column.length > visible.length ? <p className="more-count">+ {column.length - visible.length} mais</p> : null}</div></section>; })}</div></div>;
}

function TaskDrawer({ task, projects, close, refresh, openCapture }: { task: Task; projects: Project[]; close: () => void; refresh: () => Promise<void>; openCapture: (capture: Capture) => void }) {
  const [title, setTitle] = useState(task.title);
  const [description, setDescription] = useState(task.description);
  const [projectId, setProjectId] = useState(task.projectId ?? "");
  const [state, setState] = useState(task.state);
  const [source, setSource] = useState<Capture | null>(null);
  const [error, setError] = useState("");
  const drawer = useRef<HTMLElement>(null);
  useEffect(() => { drawer.current?.focus(); if (task.sourceCaptureId) void api.getCapture(task.sourceCaptureId).then(setSource); }, [task.sourceCaptureId]);
  async function submit(event: FormEvent) {
    event.preventDefault();
    try { await api.updateTask(task.id, title, description, projectId || null); if (state !== task.state) await api.setTaskState(task.id, state); await refresh(); close(); }
    catch (nextError) { setError(appError(nextError).message); }
  }
  return <aside ref={drawer} className="task-drawer" aria-label="Detalhe da Task" tabIndex={-1} onKeyDown={(event) => { if (event.key === "Escape") close(); }}><header><span className="micro-label">TASK</span><IconButton label="Fechar" icon="close" onClick={close} /></header><form className="stack-form" onSubmit={submit}><label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} /></label><label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label><label><span>PROJECT</span><select value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label><label><span>ESTADO</span><select value={state} onChange={(event) => setState(event.currentTarget.value as TaskState)}>{stateOrder.map((value) => <option key={value} value={value}>{stateLabels[value]}</option>)}</select></label>{source ? <div className="provenance"><span className="micro-label">ORIGEM</span><button type="button" onClick={() => openCapture(source)}>{source.content}</button><small>{sourceLabel(source.source)} · {relativeTime(source.capturedAt)}</small></div> : null}{error ? <p className="inline-error" role="alert">! {error}</p> : null}<div className="form-actions spread"><Button variant="danger" onClick={() => void api.setTaskArchived(task.id, true).then(async () => { await refresh(); close(); })}>Arquivar</Button><Button variant="primary" type="submit" disabled={!title.trim()}>Salvar</Button></div></form></aside>;
}

function CaptureViewer({ capture, close }: { capture: Capture; close: () => void }) {
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => dialog.current?.focus(), []);
  return <div className="overlay-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}><article ref={dialog} className="entity-viewer" role="dialog" aria-modal="true" tabIndex={-1} onKeyDown={(event) => { if (event.key === "Escape") close(); }}><header><span className="micro-label">CAPTURE</span><IconButton label="Fechar" icon="close" onClick={close} /></header><h1>{capture.content}</h1><dl><div><dt>ORIGEM</dt><dd>{sourceLabel(capture.source)}</dd></div><div><dt>ESTADO</dt><dd>{capture.lifecycleState === "archived" ? "Arquivada" : capture.processingState === "processed" ? "Processada" : "Na Inbox"}</dd></div><div><dt>CAPTURADA</dt><dd>{new Date(capture.capturedAt).toLocaleString("pt-BR")}</dd></div></dl></article></div>;
}

function CommandSurface({ close, openCapture, openTask, openProject }: { close: () => void; openCapture: (capture: Capture) => void; openTask: (task: Task) => void; openProject: (project: Project) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchItem[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [error, setError] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  useEffect(() => { input.current?.focus(); return () => previousFocus.current?.focus(); }, []);
  useEffect(() => { if (!query.trim()) { setResults([]); return; } const timeout = window.setTimeout(() => void api.search(query, includeArchived).then((items) => { setResults(items); setError(""); }).catch((nextError) => setError(appError(nextError).message)), 80); return () => window.clearTimeout(timeout); }, [query, includeArchived]);
  function openItem(item: SearchItem) { close(); if (item.kind === "project") openProject(item.project); else if (item.kind === "task") openTask(item.task); else if (item.derivedTask) openTask(item.derivedTask); else openCapture(item.capture); }
  return <div className="overlay-backdrop command-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}><section className="command-surface" role="dialog" aria-modal="true" aria-label="Command" onKeyDown={(event) => { if (event.key === "Escape") close(); }}><div className="command-input"><span className="slash">/</span><input ref={input} value={query} onChange={(event) => setQuery(event.currentTarget.value)} placeholder="Buscar ou executar comando" aria-label="Buscar no M/OS" /><IconButton label="Fechar" icon="close" onClick={close} /></div>{query ? <label className="check-control"><input type="checkbox" checked={includeArchived} onChange={(event) => setIncludeArchived(event.currentTarget.checked)} /><span>Incluir arquivados</span></label> : null}<div className="command-results" aria-live="polite">{error ? <div className="command-error"><p>! {error}</p><Button variant="outline" onClick={() => void api.rebuildSearch().then(() => api.search(query, includeArchived).then(setResults))}>Reconstruir busca</Button></div> : null}{!query ? <EmptyState>Digite para buscar.</EmptyState> : null}{query && !error && !results.length ? <EmptyState>Nenhum resultado para “{query}”.</EmptyState> : null}{results.map((item, index) => { const type = item.kind === "project" ? "PROJECT" : item.kind === "task" ? "TASK" : item.derivedTask ? "TASK + CAPTURE" : "CAPTURE"; const title = item.kind === "project" ? item.project.name : item.kind === "task" ? item.task.title : item.derivedTask?.title ?? item.capture.content; const context = item.kind === "project" ? item.project.description : item.kind === "task" ? item.project?.name : item.project?.name ?? (item.kind === "capture" ? item.capture.content : ""); return <button key={`${item.kind}-${index}-${title}`} className="command-row" onClick={() => openItem(item)}><span>{type}</span><strong>{title}</strong><small>{context}</small></button>; })}</div></section></div>;
}

function SettingsPage({ theme, setTheme, status, capturesArchived, capturesTrashed, projects, tasks, refresh }: { theme: Theme; setTheme: (theme: Theme) => void; status: AppStatus | null; capturesArchived: Capture[]; capturesTrashed: Capture[]; projects: Project[]; tasks: Task[]; refresh: () => Promise<void> }) {
  const [shortcut, setShortcut] = useState("Ctrl+Shift+Space");
  const [message, setMessage] = useState("");
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [restorePath, setRestorePath] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  async function backup() { const path = await save({ defaultPath: "m-os-backup.mos-backup", filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (path) void api.createBackup(path).then((receipt) => setMessage(`Backup criado: ${receipt.path}`)).catch((error) => setMessage(appError(error).message)); }
  async function exportData() { const path = await save({ defaultPath: "m-os-export.json", filters: [{ name: "JSON", extensions: ["json"] }] }); if (path) void api.exportJson(path).then((receipt) => setMessage(`Export criado: ${receipt.path}`)).catch((error) => setMessage(appError(error).message)); }
  async function chooseRestore() { const path = await open({ multiple: false, filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (!path) return; try { setInspection(await api.inspectBackup(path)); setRestorePath(path); dialog.current?.showModal(); } catch (error) { setMessage(appError(error).message); } }
  async function confirmRestore() { try { const safety = await api.restoreBackup(restorePath); dialog.current?.close(); setMessage(`Dados restaurados. Safety backup: ${safety.path}`); await refresh(); } catch (error) { setMessage(appError(error).message); } }
  const archivedProjects = projects.filter((project) => project.lifecycleState === "archived");
  const archivedTasks = tasks.filter((task) => task.lifecycleState === "archived");
  return <div className="page settings-page"><ContextPath segments={["M/OS", "SETTINGS"]} /><Panel label="APARÊNCIA"><div className="setting-row"><div><strong>Tema claro</strong><p>Dark permanece o padrão do sistema.</p></div><label className="switch"><input type="checkbox" checked={theme === "light"} onChange={(event) => setTheme(event.currentTarget.checked ? "light" : "dark")} /><span /></label></div></Panel><Panel label="CAPTURA RÁPIDA"><form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setShortcut(shortcut).then(setMessage).catch((error) => setMessage(appError(error).message)); }}><div><label htmlFor="shortcut">Atalho global</label><p>{status?.shortcut}</p></div><div className="inline-form"><input id="shortcut" value={shortcut} onChange={(event) => setShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form></Panel><Panel label="DADOS E PORTABILIDADE"><p className="support-copy">Backups e exports podem conter dados pessoais em texto claro.</p><div className="button-line"><Button variant="secondary" onClick={() => void backup()}>Criar backup</Button><Button variant="outline" onClick={() => void chooseRestore()}>Restaurar backup</Button><Button variant="outline" onClick={() => void exportData()}>Exportar JSON</Button></div></Panel><Panel label="ARCHIVE E TRASH"><details className="disclosure"><summary>Captures arquivadas <span>{capturesArchived.length}</span></summary>{capturesArchived.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Captures <span>{capturesTrashed.length}</span></summary>{capturesTrashed.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Projects arquivados <span>{archivedProjects.length}</span></summary>{archivedProjects.map((project) => <div className="restore-row" key={project.id}><span>{project.name}</span><Button variant="ghost" onClick={() => void api.setProjectArchived(project.id, false).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Tasks arquivadas <span>{archivedTasks.length}</span></summary>{archivedTasks.map((task) => <div className="restore-row" key={task.id}><span>{task.title}</span><Button variant="ghost" onClick={() => void api.setTaskArchived(task.id, false).then(refresh)}>Restaurar</Button></div>)}</details></Panel><Panel label="INTEGRIDADE"><dl className="health-list"><div><dt>Banco</dt><dd>{status?.storage.integrity === "ok" ? "Íntegro" : status?.storage.integrity}</dd></div><div><dt>Schema</dt><dd>v{status?.storage.schemaVersion}</dd></div><div><dt>Durabilidade</dt><dd>{status?.storage.journalMode.toUpperCase()} / {status?.storage.synchronous}</dd></div><div><dt>Snapshot</dt><dd>{status?.snapshot}</dd></div></dl></Panel>{message ? <p className="settings-message" aria-live="polite">{message}</p> : null}<dialog ref={dialog} className="restore-dialog" onCancel={() => dialog.current?.close()}><span className="micro-label">RESTORE</span><h2>Substituir o dataset local?</h2><p>Um safety backup será criado primeiro. O arquivo contém {inspection?.captureCount} Captures e usa schema v{inspection?.schemaVersion}.</p><div className="form-actions"><Button variant="ghost" onClick={() => dialog.current?.close()}>Cancelar</Button><Button variant="danger" onClick={() => void confirmRestore()}>Restaurar</Button></div></dialog></div>;
}

function QuickCapture() {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "error">("idle");
  const [feedback, setFeedback] = useState("Enter para salvar · Esc para fechar");
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => { input.current?.focus(); const unlisten = listen("window-revealed", () => input.current?.focus()); return () => { void unlisten.then((dispose) => dispose()); }; }, []);
  async function submit(event: FormEvent) { event.preventDefault(); if (!content.trim() || state === "saving") return; setState("saving"); setFeedback("Salvando localmente..."); try { await api.createCapture(content, "quick_capture"); setContent(""); setState("idle"); setFeedback("Salvo na Inbox"); window.setTimeout(() => void api.hideQuickCapture(), 160); } catch (error) { setState("error"); setFeedback(`${appError(error).message} O texto continua aqui.`); } }
  return <main className="quick-shell"><form className="quick-capture" onSubmit={submit}><div className="capture-line"><span className="slash">/</span><textarea ref={input} value={content} onChange={(event) => setContent(event.currentTarget.value)} onKeyDown={(event) => { if (event.key === "Escape") void api.hideQuickCapture(); if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} aria-label="Texto da captura" placeholder="What's on your mind?" rows={1} /></div><div className="capture-footer"><span className={`feedback ${state}`} aria-live="polite">{feedback}</span><Button variant="primary" type="submit" disabled={!content.trim() || state === "saving"}>Capturar</Button></div></form></main>;
}

function DesktopApp() {
  const [page, setPage] = useState<Page>("home");
  const [recent, setRecent] = useState<Capture[]>([]);
  const [inbox, setInbox] = useState<Capture[]>([]);
  const [archived, setArchived] = useState<Capture[]>([]);
  const [trashed, setTrashed] = useState<Capture[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [commandOpen, setCommandOpen] = useState(false);
  const [viewedCapture, setViewedCapture] = useState<Capture | null>(null);
  const [drawerTask, setDrawerTask] = useState<Task | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState("");
  const [undo, setUndo] = useState<UndoAction | null>(null);
  const [theme, setThemeState] = useState<Theme>(() => localStorage.getItem("m-os-theme") === "light" ? "light" : "dark");
  const undoTimer = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    const [nextRecent, nextInbox, nextArchived, nextTrashed, nextProjects, nextTasks, nextStatus] = await Promise.all([api.recent(), api.inbox(), api.archived(), api.trashed(), api.projects(true), api.tasks(true), api.status()]);
    setRecent(nextRecent); setInbox(nextInbox); setArchived(nextArchived); setTrashed(nextTrashed); setProjects(nextProjects); setTasks(nextTasks); setStatus(nextStatus);
    setDrawerTask((current) => current ? nextTasks.find((task) => task.id === current.id) ?? null : null);
  }, []);
  useEffect(() => { void refresh(); const events = [listen("capture-changed", () => void refresh()), listen("data-changed", () => void refresh()), listen("dataset-restored", () => void refresh()), listen("snapshot-status-changed", () => void refresh())]; return () => { events.forEach((event) => void event.then((dispose) => dispose())); }; }, [refresh]);
  useEffect(() => { document.documentElement.dataset.theme = theme; localStorage.setItem("m-os-theme", theme); }, [theme]);
  useEffect(() => { const handler = (event: globalThis.KeyboardEvent) => { if (event.ctrlKey && event.key.toLowerCase() === "k") { event.preventDefault(); setCommandOpen(true); } if (event.ctrlKey && event.key.toLowerCase() === "z" && undo) { event.preventDefault(); void undo.run().then(() => { setUndo(null); return refresh(); }); } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, [refresh, undo]);

  function showReceipt(action: UndoAction) { setUndo(action); if (undoTimer.current) window.clearTimeout(undoTimer.current); undoTimer.current = window.setTimeout(() => setUndo(null), 8_000); }
  function openProject(project: Project) { setSelectedProjectId(project.id); setPage("projects"); }
  const nav: { page: Page; label: string; icon: IconName; count?: number }[] = [{ page: "home", label: "Home", icon: "home" }, { page: "inbox", label: "Inbox", icon: "inbox", count: inbox.length }, { page: "projects", label: "Projects", icon: "projects" }, { page: "tasks", label: "Tasks", icon: "board" }, { page: "settings", label: "Settings", icon: "settings" }];
  const content = useMemo(() => {
    if (page === "home") return <HomePage recent={recent} projects={projects} tasks={tasks} refresh={refresh} openCapture={setViewedCapture} openProject={openProject} openTask={setDrawerTask} />;
    if (page === "inbox") return <InboxPage captures={inbox} projects={projects} refresh={refresh} receipt={showReceipt} openTask={setDrawerTask} />;
    if (page === "projects") return <ProjectsPage projects={projects} tasks={tasks} initialProjectId={selectedProjectId} refresh={refresh} openTask={setDrawerTask} />;
    if (page === "tasks") return <BoardPage tasks={tasks} projects={projects} refresh={refresh} openTask={setDrawerTask} />;
    return <SettingsPage theme={theme} setTheme={setThemeState} status={status} capturesArchived={archived} capturesTrashed={trashed} projects={projects} tasks={tasks} refresh={refresh} />;
  }, [page, recent, projects, tasks, refresh, inbox, selectedProjectId, theme, status, archived, trashed]);

  return <div className="app-shell"><aside className="nav-rail"><div className="symbol">M<span>/</span></div><nav aria-label="Navegação principal">{nav.map((item) => <button key={item.page} aria-current={page === item.page ? "page" : undefined} aria-label={item.label} title={item.label} onClick={() => setPage(item.page)}><Icon name={item.icon} filled={page === item.page} />{item.count ? <span>{item.count}</span> : null}</button>)}</nav><IconButton label="Quick Capture" icon="capture" onClick={() => void api.showQuickCapture()} /></aside><div className="main-column"><header className="topbar"><button className="command-trigger" onClick={() => setCommandOpen(true)}><span className="slash">/</span><span>Command</span><kbd>CTRL K</kbd></button></header><main className="content">{content}</main></div>{commandOpen ? <CommandSurface close={() => setCommandOpen(false)} openCapture={setViewedCapture} openTask={setDrawerTask} openProject={openProject} /> : null}{viewedCapture ? <CaptureViewer capture={viewedCapture} close={() => setViewedCapture(null)} /> : null}{drawerTask ? <TaskDrawer key={drawerTask.id} task={drawerTask} projects={projects} close={() => setDrawerTask(null)} refresh={refresh} openCapture={(capture) => { setDrawerTask(null); setViewedCapture(capture); }} /> : null}{undo ? <div className="receipt" role="status"><span>{undo.message}</span><button onClick={() => void undo.run().then(() => { setUndo(null); return refresh(); })}>DESFAZER · CTRL Z</button></div> : null}</div>;
}

export default function App() {
  return getCurrentWindow().label === "quick-capture" ? <QuickCapture /> : <DesktopApp />;
}

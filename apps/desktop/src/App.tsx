import { DragEvent, FormEvent, KeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, appError } from "./api";
import { resolveFunctionTarget, type FunctionIntentTarget } from "./functionIntents";
import { hermes, hermesUnavailableLabel, type HermesConnectionState, type HermesStatus } from "./hermes";
import { HermesPage } from "./HermesPage";
import { Icon, type IconName } from "./Icon";
import { MosSymbol } from "./Symbol";
import type { AppCapabilities, AppCatalogEntry, AppLaunchKind, AppStatus, BackupInspection, Capture, FunctionDefinition, HiddenWidget, Project, RegisteredApp, Resource, ResourceKind, SearchItem, Task, TaskState, UpdateInfo, UpdateProgress, Workspace } from "./types";
import "./App.css";

type Page = "home" | "hermes" | "inbox" | "projects" | "workspaces" | "apps" | "library" | "tasks" | "settings";
type UndoAction = { message: string; run: () => Promise<unknown> };
type Theme = "dark" | "light";
type CommandResult = SearchItem | { kind: "function"; function: FunctionDefinition };
type FunctionIntent = { target: FunctionIntentTarget; key: number };

// A ordem e a ordem das colunas do kanban. DOING e a unica coluna em sodio:
// e o estado que importa.
/* Espelha o teto que list_inbox pede em src-tauri/src/lib.rs:84. Se mudar la, muda
   aqui: e o que permite a Home admitir que a contagem esta truncada. */
const INBOX_PAGE = 200;

const stateOrder: TaskState[] = ["inbox", "backlog", "planned", "doing", "review", "done"];
const stateLabels: Record<TaskState, string> = { inbox: "Inbox", backlog: "Backlog", planned: "Planned", doing: "Doing", review: "Review", done: "Done" };
const functionCategories: FunctionDefinition["category"][] = ["capture", "work", "memory", "app", "data", "system"];
const functionCategoryLabels: Record<FunctionDefinition["category"], string> = { capture: "CAPTURE", work: "WORK", memory: "MEMORY", app: "APP", data: "DATA", system: "SYSTEM" };
const functionRiskLabels: Record<FunctionDefinition["risk"], string> = { low: "baixo", medium: "medio", high: "alto" };
const functionConfirmationLabels: Record<FunctionDefinition["confirmation"], string> = { none: "sem confirmacao", explicit: "confirmacao explicita" };
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

function resourceHost(url: string) {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

function Button({ variant = "secondary", size, className = "", children, ...props }: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "outline" | "ghost" | "danger"; size?: "sm" }) {
  // className e somado, nunca sobrescrito: espalhar props depois do className
  // fazia um className de fora apagar "button primary" inteiro.
  return <button className={`button ${variant} ${size ?? ""} ${className}`.replace(/\s+/g, " ").trim()} type="button" {...props}>{children}</button>;
}

function IconButton({ label, icon, active = false, onClick }: { label: string; icon: IconName; active?: boolean; onClick: () => void }) {
  return <button className="icon-button" type="button" aria-label={label} title={label} onClick={onClick}><Icon name={icon} filled={active} /></button>;
}

function ContextPath({ segments }: { segments: string[] }) {
  return <div className="context-path" aria-label={segments.join(" / ")}>{segments.map((segment, index) => <span key={`${segment}-${index}`} className={index === segments.length - 1 ? "current" : undefined}>{index ? <b>/</b> : null}{segment}</span>)}</div>;
}

/**
 * `rule` troca a regua: em vez de sublinhar o cabecalho inteiro, ela sai do
 * rotulo e atravessa a linha. E como o desenho separa uma secao que abre a
 * pagina (CONTEXTO) de um painel dentro da grade.
 */
function Panel({ label, count, action, rule = false, children, className = "" }: { label: string; count?: string; action?: ReactNode; rule?: boolean; children: ReactNode; className?: string }) {
  return <section className={`panel ${className}`} data-panel={label} data-rule={rule || undefined}><header className="panel-header"><h2>{label}</h2>{rule ? <span className="panel-rule" aria-hidden="true" /> : null}{count ? <span className="panel-count">{count}</span> : null}{action}</header>{children}</section>;
}

function EmptyState({ children }: { children: ReactNode }) {
  return <p className="empty-state">{children}</p>;
}

/* Cuida so do posicionamento na grade. A moldura e o rotulo continuam no Panel, para
   que a etapa 2 (modo de edicao) mude posicao sem tocar em nenhum widget.
   `hidden` devolve null: a regra de visibilidade fica num lugar so, e a grade nao
   precisa saber de nada — os widgets restantes reflowam sozinhos. */
function Widget({ id, size, hidden = false, children }: { id: string; size: "1x1" | "2x1" | "2x2" | "full"; hidden?: boolean; children: ReactNode }) {
  if (hidden) return null;
  return <div className="widget" data-widget={id} data-size={size}>{children}</div>;
}

/* A promessa central do produto e confianca: o que entrou esta guardado. Quando
   tudo esta bem este e o elemento mais silencioso da tela; so ganha peso ao falhar. */
function SystemHealth({ status }: { status: AppStatus | null }) {
  const [hermesState, setHermesState] = useState<HermesConnectionState>("offline");
  useEffect(() => {
    void hermes.status().then((next) => setHermesState(next.state)).catch(() => undefined);
    const subscription = hermes.onState((next) => setHermesState(next.state));
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);
  const saved = status?.storage.integrity === "ok";
  // Falha fechado de proposito. `snapshot` e texto livre vindo de schedule_snapshot
  // (src-tauri/src/lib.rs:801) e tem tres formas: "Snapshot diario criado.",
  // "Snapshot diario ja existe." e "Falha no snapshot diario: ...". Marcar como ok
  // qualquer string nao vazia pintaria a falha de verde — justamente o caso em que
  // este widget precisa ser honesto. Casar com o sucesso, e nao com a falha, faz
  // qualquer mensagem futura desconhecida ficar sem o verde em vez de ganha-lo.
  const backupOk = status?.snapshot.startsWith("Snapshot") ?? false;
  return <dl className="health-list">
    <div><dt>Dados</dt><dd data-ok={saved || undefined}>{saved ? "Salvos" : status ? status.storage.integrity : "—"}</dd></div>
    <div><dt>Backup</dt><dd data-ok={backupOk || undefined}>{status?.snapshot || "—"}</dd></div>
    <div><dt>Hermes</dt><dd data-ok={hermesState === "online" || undefined}>{hermesState === "online" ? "Online" : hermesState === "connecting" ? "Conectando" : "Offline"}</dd></div>
  </dl>;
}

/* O vazio de um painel com escopo tem duas causas que a mensagem antiga confundia:
   nada cadastrado, ou nada vinculado ao Workspace ativo. Sem separar as duas, a Home
   afirma que o usuario nao tem apps enquanto esconde os que ele tem. */
/* Fonte de verdade unica dos ids de widget. Os ids VAO PARA O BANCO: renomear
   um deles apaga em silencio a escolha de quem tinha ocultado o widget, porque
   a linha guardada deixa de casar com qualquer widget do catalogo. O rotulo
   pode mudar a vontade; o id, nunca. */
const HOME_WIDGETS: { id: string; label: string }[] = [
  { id: "now", label: "EM ANDAMENTO" },
  { id: "recent", label: "RECENTES" },
  { id: "projects", label: "PROJECTS" },
  { id: "apps", label: "APPS" },
  { id: "inbox_pulse", label: "INBOX" },
  { id: "quick_actions", label: "AÇÕES" },
  { id: "system_health", label: "SISTEMA" },
];

function ScopedEmptyState({ total, workspace, noun, onLink }: { total: number; workspace: Workspace | null; noun: "app" | "project"; onLink: () => void }) {
  if (total === 0 || !workspace) {
    return <EmptyState>{noun === "app" ? "Apps cadastrados aparecerão aqui." : "Projects criados aparecerão aqui."}</EmptyState>;
  }
  const counted = noun === "app"
    ? `${total} ${total === 1 ? "app cadastrado" : "apps cadastrados"}`
    : `${total} ${total === 1 ? "Project criado" : "Projects criados"}`;
  return <div className="scoped-empty"><EmptyState>{`${counted}, nenhum em ${workspace.name}.`}</EmptyState><Button variant="outline" size="sm" onClick={onLink}>Vincular</Button></div>;
}

function CaptureComposer({ onSaved, focusKey }: { onSaved: (capture: Capture) => void; focusKey?: number }) {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "success" | "error">("idle");
  const [feedback, setFeedback] = useState("");
  const [focused, setFocused] = useState(false);
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => { if (focusKey !== undefined) input.current?.focus(); }, [focusKey]);

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

  // Sem caixa e sem borda: a excecao deliberada do sistema. O caret de bloco so
  // aparece com o campo vazio, como convite — a partir do primeiro caractere
  // quem manda e o caret nativo, e dois caretes na tela seria mentira visual.
  // Sem caixa e sem borda: a excecao deliberada do sistema.
  //
  // O placeholder e o caret sao desenhados por cima do textarea, e nao pelo
  // atributo placeholder, porque o desenho poe o caret de bloco colado no fim
  // do texto. Com placeholder nativo o caret so poderia ficar na borda do
  // campo, que e onde ele estava antes desta correcao. Some ao focar: dali em
  // diante quem manda e o caret nativo, e dois caretes seriam mentira visual.
  return <form className="capture-field" onSubmit={submit}>
    <div className="capture-line">
      <span className="capture-bar" aria-hidden="true" />
      <div className="capture-input">
        <textarea ref={input} aria-label="Conteúdo da captura" value={content} onChange={(event) => setContent(event.currentTarget.value)} onFocus={() => setFocused(true)} onBlur={() => setFocused(false)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} rows={1} />
        {!content && !focused ? <span className="capture-ghost" aria-hidden="true">What's on your mind?<i className="capture-caret" /></span> : null}
      </div>
      <Button className="capture-save" variant="primary" type="submit" disabled={!content.trim() || state === "saving"}>{state === "saving" ? "Salvando" : "Salvar ⏎"}</Button>
    </div>
    {feedback && state === "error" ? <p className="feedback error" role="alert">{feedback}</p> : null}
  </form>;
}

/** Progresso do Project: barra de 2px mais a contagem. A barra diz o quanto
 *  falta antes de o olho ler o numero — e o numero confirma. */
function RowProgress({ done, total }: { done: number; total: number }) {
  return <><span className="row-progress" aria-hidden="true"><i style={{ width: `${total ? Math.round((done / total) * 100) : 0}%` }} /></span><span className="row-progress-count">{done}/{total}</span></>;
}

/** `secondaryKind` decide a familia da segunda linha. Origem de captura e tipo
 *  de lancamento sao dado de sistema e vao em mono; descricao de Project e
 *  texto do usuario e vai em grotesk. O AGENTS.md e explicito: mono nunca
 *  vaza para conteudo. */
function DataRow({ primary, meta, secondary, secondaryKind = "text", marker, progress, selected = false, completed = false, saved = false, onClick, onKeyDown, onPointerDown, draggable, onDragStart, onDragEnd }: { primary: string; meta?: string; secondary?: string; secondaryKind?: "text" | "system"; marker?: ReactNode; progress?: { done: number; total: number }; selected?: boolean; completed?: boolean; saved?: boolean; onClick?: () => void; onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void; onPointerDown?: React.PointerEventHandler<HTMLButtonElement>; draggable?: boolean; onDragStart?: React.DragEventHandler<HTMLButtonElement>; onDragEnd?: React.DragEventHandler<HTMLButtonElement> }) {
  return <button className="data-row" type="button" aria-current={selected ? "true" : undefined} data-selected={selected || undefined} data-completed={completed || undefined} data-saved={saved || undefined} onClick={onClick} onKeyDown={onKeyDown} onPointerDown={onPointerDown} draggable={draggable} onDragStart={onDragStart} onDragEnd={onDragEnd}>{marker}<span className="row-copy"><strong>{primary}</strong>{secondary ? <small data-system={secondaryKind === "system" || undefined}>{secondary}</small> : null}</span>{progress ? <RowProgress done={progress.done} total={progress.total} /> : null}{meta ? <span className="row-meta">{meta}</span> : null}</button>;
}

function moveListFocus(event: KeyboardEvent<HTMLButtonElement>) {
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return null;
  const rows = Array.from(event.currentTarget.closest(".row-list")?.querySelectorAll<HTMLButtonElement>(".data-row") ?? []);
  const currentIndex = rows.indexOf(event.currentTarget);
  if (currentIndex < 0 || !rows.length) return null;
  event.preventDefault();
  const nextIndex = event.key === "Home"
    ? 0
    : event.key === "End"
      ? rows.length - 1
      : Math.max(0, Math.min(rows.length - 1, currentIndex + (event.key === "ArrowDown" ? 1 : -1)));
  rows[nextIndex]?.focus();
  return nextIndex;
}

function HomePage({ recent, inbox, projects, tasks, workspaces, apps, status, hiddenWidgets, refresh, openCapture, openProject, openWorkspace, openTask, openApp, openInbox, openTasksPage, openProjectsPage, intent }: { recent: Capture[]; inbox: Capture[]; projects: Project[]; tasks: Task[]; workspaces: Workspace[]; apps: RegisteredApp[]; status: AppStatus | null; hiddenWidgets: HiddenWidget[]; refresh: () => Promise<void>; openCapture: (capture: Capture) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openTask: (task: Task) => void; openApp: (app: RegisteredApp) => void; openInbox: () => void; openTasksPage: () => void; openProjectsPage: () => void; intent?: FunctionIntent }) {
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const [currentWorkspaceId, setCurrentWorkspaceId] = useState(() => localStorage.getItem("m-os-current-workspace") ?? "");
  const [workspaceProjects, setWorkspaceProjects] = useState<Project[]>([]);
  const [workspaceApps, setWorkspaceApps] = useState<RegisteredApp[]>([]);
  const currentWorkspace = activeWorkspaces.find((workspace) => workspace.id === currentWorkspaceId) ?? null;
  useEffect(() => {
    if (!currentWorkspaceId || !activeWorkspaces.some((workspace) => workspace.id === currentWorkspaceId)) {
      setWorkspaceProjects([]);
      setWorkspaceApps([]);
      localStorage.removeItem("m-os-current-workspace");
      localStorage.removeItem("m-os-current-workspace-name");
      return;
    }
    localStorage.setItem("m-os-current-workspace", currentWorkspaceId);
    // O nome alimenta o segmento do meio do caminho de contexto nas outras
    // telas. Guardado junto do id para nao exigir uma busca so para um rotulo.
    localStorage.setItem("m-os-current-workspace-name", activeWorkspaces.find((workspace) => workspace.id === currentWorkspaceId)?.name ?? "");
    void Promise.all([api.workspaceProjects(currentWorkspaceId), api.workspaceApps(currentWorkspaceId)])
      .then(([nextProjects, nextApps]) => {
        setWorkspaceProjects(nextProjects);
        setWorkspaceApps(nextApps);
      })
      .catch(() => {
        setWorkspaceProjects([]);
        setWorkspaceApps([]);
      });
  }, [currentWorkspaceId, workspaces]);
  const scopedProjectIds = new Set(workspaceProjects.map((project) => project.id));
  const scopedProjects = currentWorkspace ? workspaceProjects : projects.filter((project) => project.lifecycleState === "active");
  const scopedApps = currentWorkspace ? workspaceApps : apps.filter((app) => app.lifecycleState === "active");
  const doing = tasks.filter((task) => task.state === "doing" && task.lifecycleState === "active" && (!currentWorkspace || (task.projectId && scopedProjectIds.has(task.projectId)))).slice(0, 5);
  const activeApps = scopedApps
    .filter((app) => app.lifecycleState === "active")
    .sort((left, right) => {
      const leftDate = left.lastOpenedAt ?? left.updatedAt;
      const rightDate = right.lastOpenedAt ?? right.updatedAt;
      return rightDate.localeCompare(leftDate);
    })
    .slice(0, 5);
  // Efemero: marca a Capture recem-criada para o savedWash e some. Nao e
  // estado de dominio, entao nao vale persistir nem subir para o App.
  const [savedIds, setSavedIds] = useState<Set<string>>(() => new Set());
  function markSaved(capture: Capture) {
    setSavedIds((current) => new Set(current).add(capture.id));
    window.setTimeout(() => setSavedIds((current) => { const next = new Set(current); next.delete(capture.id); return next; }), 900);
  }
  // Tres dias e o limiar do catalogo (IDEAS.md 155).
  //
  // Nao existe contagem verdadeira da Inbox no front: list_inbox pede 200
  // (src-tauri/src/lib.rs:84) e o proprio AppStatus.inboxCount e calculado com o
  // mesmo teto (lib.rs:685). Pior, a query ordena por captured_at DESC
  // (repository.rs:105), entao o corte descarta as capturas MAIS ANTIGAS — que sao
  // exatamente as que este widget existe para denunciar.
  //
  // Enquanto o core nao expuser um COUNT real, a saida honesta e admitir o teto:
  // no limite, mostrar "200+" em vez de "200" e "N+" em vez de "N".
  const staleInbox = inbox.filter((capture) => Date.now() - new Date(capture.capturedAt).getTime() > 3 * 24 * 60 * 60 * 1000).length;
  const inboxCapped = inbox.length >= INBOX_PAGE;
  // Sem Workspace selecionado nada e ocultado: "Todos" e a visao sem filtro, e
  // sem Workspace nao ha escolha a aplicar.
  const hiddenIds = useMemo(() => new Set(currentWorkspaceId ? hiddenWidgets.filter((entry) => entry.workspaceId === currentWorkspaceId).map((entry) => entry.widgetId) : []), [hiddenWidgets, currentWorkspaceId]);
  const allWidgetsHidden = HOME_WIDGETS.every((widget) => hiddenIds.has(widget.id));
  const projectName = (id: string | null) => projects.find((project) => project.id === id)?.name;
  const isActiveToday = (project: Project) => new Date(project.updatedAt).toDateString() === new Date().toDateString();
  return <div className="page home-page">
    <ContextPath segments={["M", "HOME"]} />
    <CaptureComposer onSaved={(capture) => { markSaved(capture); void refresh(); }} focusKey={intent?.target === "home_capture" ? intent.key : undefined} />
    <Panel label="CONTEXTO" rule action={currentWorkspace ? <Button variant="ghost" onClick={() => setCurrentWorkspaceId("")}>Todos</Button> : undefined}><div className="context-switcher">{activeWorkspaces.map((workspace) => <button key={workspace.id} type="button" data-selected={workspace.id === currentWorkspaceId || undefined} onClick={() => setCurrentWorkspaceId(workspace.id)} onDoubleClick={() => openWorkspace(workspace)}><strong>{workspace.name}</strong><small>{workspace.description || "Workspace"}</small></button>)}{!activeWorkspaces.length ? <EmptyState>Workspaces ativos aparecerão aqui.</EmptyState> : null}</div></Panel>
    <div className="home-grid">
      <Widget id="now" hidden={hiddenIds.has("now")} size="2x1"><Panel label="EM ANDAMENTO" count={doing.length ? String(doing.length) : undefined}>{doing.length ? doing.map((task) => <DataRow key={task.id} primary={task.title} meta={projectName(task.projectId)} onClick={() => openTask(task)} />) : <EmptyState>Nothing in progress right now.</EmptyState>}</Panel></Widget>
      {/* Sem contagem. O badge dizia `INBOX ${recent.length}` e mentia duas vezes:
          list_recent nao filtra por processing_state (repository.rs:91), entao a
          lista traz tambem o que ja foi processado, e o comando pede so 8
          (src-tauri/src/lib.rs:80), entao o numero parava em 8 por mais cheia que
          a Inbox estivesse. A contagem verdadeira da Inbox e a do widget INBOX,
          logo abaixo — duas contagens do mesmo nome que discordam sao pior que
          nenhuma. */}
      <Widget id="recent" hidden={hiddenIds.has("recent")} size="2x1"><Panel label="RECENTES">{recent.length ? recent.map((capture) => <DataRow key={capture.id} primary={capture.content} meta={relativeTime(capture.capturedAt)} saved={savedIds.has(capture.id)} onClick={() => openCapture(capture)} />) : <EmptyState>Nothing on your mind right now.</EmptyState>}</Panel></Widget>
      {/* Sem contagem: o desenho so conta o que exige decisao — o que esta em
          andamento e o que espera na Inbox. Project e App voce navega, nao
          processa. */}
      {/* O corte em 5 e silencioso: com 12 projects o painel mostra 5 e nada diz
          que existem outros. Continua sem contagem, pela decisao acima, mas o
          link so aparece quando ha o que ver alem do corte — se cabem todos, o
          cabecalho fica limpo. */}
      <Widget id="projects" hidden={hiddenIds.has("projects")} size="2x2"><Panel label="PROJECTS" action={scopedProjects.length > 5 ? <Button variant="ghost" onClick={() => openProjectsPage()}>Ver todos</Button> : undefined}>{scopedProjects.slice(0, 5).map((project) => <DataRow key={project.id} primary={project.name} marker={<span className="project-dot" data-active={isActiveToday(project) || undefined} aria-hidden="true" />} meta={relativeTime(project.updatedAt)} onClick={() => openProject(project)} />)}{!scopedProjects.length ? <ScopedEmptyState total={projects.filter((project) => project.lifecycleState === "active").length} workspace={currentWorkspace} noun="project" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel></Widget>
      {/* O nome do app nao entra: o icone com a inicial e o atalho ja o
          identificam, e a linha de nomes competiria com as rows ao lado. */}
      <Widget id="apps" hidden={hiddenIds.has("apps")} size="2x1"><Panel label="APPS"><div className="app-row">{activeApps.map((app, index) => <button key={app.id} type="button" className="app-tile" onClick={() => openApp(app)} title={app.name} aria-label={app.name}><span className="app-icon" aria-hidden="true">{app.name.trim().charAt(0).toUpperCase()}</span>{index < 9 ? <span className="app-shortcut">⌘{index + 1}</span> : null}</button>)}</div>{!activeApps.length ? <ScopedEmptyState total={apps.filter((app) => app.lifecycleState === "active").length} workspace={currentWorkspace} noun="app" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel></Widget>
      <Widget id="inbox_pulse" hidden={hiddenIds.has("inbox_pulse")} size="1x1"><Panel label="INBOX"><button type="button" className="pulse" onClick={() => openInbox()}><strong className="pulse-count">{inboxCapped ? `${INBOX_PAGE}+` : inbox.length}</strong><small>{inbox.length === 1 ? "capture por processar" : "captures por processar"}</small>{staleInbox ? <small className="pulse-stale">{staleInbox === 1 && !inboxCapped ? "1 com mais de 3 dias" : `${staleInbox}${inboxCapped ? "+" : ""} com mais de 3 dias`}</small> : null}</button></Panel></Widget>
      <Widget id="quick_actions" hidden={hiddenIds.has("quick_actions")} size="1x1"><Panel label="AÇÕES"><div className="quick-actions"><Button variant="outline" size="sm" onClick={() => void api.showQuickCapture()}>Capturar</Button><Button variant="outline" size="sm" onClick={() => openTasksPage()}>Nova Task</Button><Button variant="outline" size="sm" onClick={() => openProjectsPage()}>Novo Project</Button></div></Panel></Widget>
      <Widget id="system_health" hidden={hiddenIds.has("system_health")} size="1x1"><Panel label="SISTEMA"><SystemHealth status={status} /></Panel></Widget>
    </div>
    {/* Ocultar os sete e escolha legitima. O que nao pode e a Home virar um
        branco sem explicacao — quem escondeu tudo precisa do caminho de volta. */}
    {allWidgetsHidden ? <div className="scoped-empty"><EmptyState>Todos os widgets estão ocultos neste Workspace.</EmptyState><Button variant="outline" size="sm" onClick={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }}>Ajustar</Button></div> : null}
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

function InboxPage({ captures, projects, refresh, receipt, openTask, openResource, intent }: { captures: Capture[]; projects: Project[]; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openTask: (task: Task) => void; openResource: (resource: Resource) => void; intent?: FunctionIntent }) {
  const [selectedId, setSelectedId] = useState(captures[0]?.id ?? "");
  const [taskForm, setTaskForm] = useState(false);
  const [resourceForm, setResourceForm] = useState(false);
  const [error, setError] = useState("");
  const detailActions = useRef<HTMLDivElement>(null);
  useEffect(() => { if (!captures.some((capture) => capture.id === selectedId)) setSelectedId(captures[0]?.id ?? ""); }, [captures, selectedId]);
  const selected = captures.find((capture) => capture.id === selectedId) ?? null;
  useEffect(() => {
    if (!intent || !selected) return;
    if (intent.target === "inbox_create_task") {
      setTaskForm(true);
      setResourceForm(false);
      return;
    }
    if (intent.target === "inbox_create_resource") {
      setTaskForm(false);
      setResourceForm(true);
      return;
    }
    if (intent.target === "inbox_process") {
      setTaskForm(false);
      window.requestAnimationFrame(() => detailActions.current?.querySelector<HTMLButtonElement>("[data-function-action='capture.mark_processed']")?.focus());
    }
  }, [intent?.key, selected?.id]);

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

  if (!captures.length) return <div className="page"><ContextPath segments={["M", "INBOX"]} /><EmptyState>Nothing to process. Everything captured has been dealt with.</EmptyState></div>;
  return <div className="split-page">
    <section className="list-pane">
      <div className="pane-heading"><ContextPath segments={["M", "INBOX"]} /><span className="micro-label">{captures.length} {captures.length === 1 ? "ITEM" : "ITENS"}</span></div>
      <div className="row-list">{captures.map((capture) => <DataRow key={capture.id} primary={capture.content} secondary={sourceLabel(capture.source)} secondaryKind="system" meta={relativeTime(capture.capturedAt)} selected={capture.id === selectedId} onClick={() => { setSelectedId(capture.id); setTaskForm(false); setResourceForm(false); }} />)}</div>
    </section>
    {selected ? <article className="detail-pane"><header className="detail-header"><div><span className="micro-label">SELECIONADO</span><h1>{selected.content}</h1><div className="chip-line"><span className="chip">{sourceLabel(selected.source)}</span><span className="chip">{relativeTime(selected.capturedAt)}</span></div></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => void mutate("archive")}>Arquivar</button><button className="danger-text" onClick={() => void mutate("trash")}>Mover para a Lixeira</button></div></details></header>
      {error ? <p className="inline-error" role="alert">! {error}</p> : null}
      {/* Moldura pronta, conteudo honesto. A interpretacao do Hermes e a fase 3
          da integracao; ate la este bloco diz o que e, em vez de fabricar uma
          interpretacao falsa para a tela parecer completa. */}
      <section className="hermes-block" aria-label="Interpretação do Hermes">
        <p className="hermes-empty">Interpretação automática ainda não está ligada. Classifique manualmente abaixo — nada se perde.</p>
      </section>
      {taskForm ? <CaptureTaskForm capture={selected} projects={projects} cancel={() => setTaskForm(false)} onCreated={(task) => { setTaskForm(false); void refresh(); openTask(task); }} /> : resourceForm ? <ResourceForm capture={selected} cancel={() => setResourceForm(false)} saved={(resource) => { setResourceForm(false); void refresh(); openResource(resource); }} /> : <div ref={detailActions} className="detail-actions"><Button variant="primary" onClick={() => { setTaskForm(true); setResourceForm(false); }}>Criar Task</Button><Button variant="secondary" onClick={() => { setTaskForm(false); setResourceForm(true); }}>Salvar Resource</Button><Button variant="secondary" data-function-action="capture.mark_processed" onClick={() => void mutate("processed")}>Arquivar</Button></div>}
      <p className="pane-footnote">J / K percorre · Espaço processa · ⌘Z desfaz</p>
    </article> : null}
  </div>;
}

function ProjectForm({ project, cancel, saved }: { project?: Project; cancel: () => void; saved: (project: Project) => void }) {
  const [name, setName] = useState(project?.name ?? "");
  const [description, setDescription] = useState(project?.description ?? "");
  const [repository, setRepository] = useState(project?.repository ?? "");
  const [error, setError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    try { saved(project ? await api.updateProject(project.id, name, description, repository) : await api.createProject(name, description, repository)); }
    catch (nextError) { setError(appError(nextError).message); }
  }
  return <form className="stack-form" onSubmit={submit}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    <label><span>REPOSITÓRIO</span><input className="mono-input" value={repository} onChange={(event) => setRepository(event.currentTarget.value)} placeholder="usuario/repo ou URL" /></label>
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

function ProjectsPage({ projects, tasks, initialProjectId, refresh, openTask, intent }: { projects: Project[]; tasks: Task[]; initialProjectId: string; refresh: () => Promise<void>; openTask: (task: Task) => void; intent?: FunctionIntent }) {
  const activeProjects = projects.filter((project) => project.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialProjectId || activeProjects[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new" | "task">("view");
  useEffect(() => { if (initialProjectId) setSelectedId(initialProjectId); }, [initialProjectId]);
  useEffect(() => { if (intent?.target === "projects_create") setMode("new"); }, [intent?.key]);
  useEffect(() => { if (!activeProjects.some((project) => project.id === selectedId)) setSelectedId(activeProjects[0]?.id ?? ""); }, [activeProjects, selectedId]);
  const selected = activeProjects.find((project) => project.id === selectedId) ?? null;
  const relatedTasks = tasks.filter((task) => task.projectId === selectedId && task.lifecycleState === "active");
  return <div className="split-page projects-page">
    <section className="list-pane"><ContextPath segments={["M", "PROJECTS"]} /><div className="list-command"><Button variant="outline" size="sm" onClick={() => setMode("new")}>Novo Project</Button></div><div className="row-list">{activeProjects.map((project) => <DataRow key={project.id} primary={project.name} secondary={project.description || undefined} progress={{ done: tasks.filter((task) => task.projectId === project.id && task.lifecycleState === "active" && task.state === "done").length, total: tasks.filter((task) => task.projectId === project.id && task.lifecycleState === "active").length }} selected={project.id === selectedId} onClick={() => { setSelectedId(project.id); setMode("view"); }} />)}</div>{!activeProjects.length && mode !== "new" ? <EmptyState>Crie um Project para reunir trabalho relacionado.</EmptyState> : null}</section>
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO PROJECT</span><ProjectForm cancel={() => setMode("view")} saved={(project) => { setSelectedId(project.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <ProjectForm project={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">PROJECT</span><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setProjectArchived(selected.id, true).then(refresh)}>Arquivar</button></div></details></header><dl className="fact-grid"><div><dt>REPOSITÓRIO</dt><dd className="mono-value">{selected.repository || <span className="fact-empty">Nenhum associado</span>}</dd></div><div><dt>ATUALIZADO</dt><dd>{relativeTime(selected.updatedAt)}</dd></div></dl>{mode === "task" ? <DirectTaskForm projectId={selected.id} projects={projects} cancel={() => setMode("view")} saved={(task) => { setMode("view"); void refresh(); openTask(task); }} /> : <Panel label="TASKS" action={<Button variant="primary" onClick={() => setMode("task")}>Criar Task</Button>}>{relatedTasks.length ? relatedTasks.map((task) => <DataRow key={task.id} primary={task.title} meta={stateLabels[task.state]} completed={task.state === "done"} onClick={() => openTask(task)} />) : <EmptyState>Nenhuma Task neste Project.</EmptyState>}</Panel>}</>}</> : null}</article>
  </div>;
}

function WorkspaceForm({ workspace, cancel, saved }: { workspace?: Workspace; cancel: () => void; saved: (workspace: Workspace) => void }) {
  const [name, setName] = useState(workspace?.name ?? "");
  const [description, setDescription] = useState(workspace?.description ?? "");
  const [error, setError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    try { saved(workspace ? await api.updateWorkspace(workspace.id, name, description) : await api.createWorkspace(name, description)); }
    catch (nextError) { setError(appError(nextError).message); }
  }
  return <form className="stack-form" onSubmit={submit}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    {error ? <p className="inline-error" role="alert">! {error}</p> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={!name.trim()}>Salvar</Button></div>
  </form>;
}

function WorkspacesPage({ workspaces, projects, apps, hiddenWidgets, initialWorkspaceId, refresh, openProject, openApp, intent }: { workspaces: Workspace[]; projects: Project[]; apps: RegisteredApp[]; hiddenWidgets: HiddenWidget[]; initialWorkspaceId: string; refresh: () => Promise<void>; openProject: (project: Project) => void; openApp: (app: RegisteredApp) => void; intent?: FunctionIntent }) {
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const activeProjects = projects.filter((project) => project.lifecycleState === "active");
  const activeApps = apps.filter((app) => app.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialWorkspaceId || activeWorkspaces[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new">("view");
  const [workspaceProjects, setWorkspaceProjects] = useState<Project[]>([]);
  const [workspaceApps, setWorkspaceApps] = useState<RegisteredApp[]>([]);
  const [message, setMessage] = useState("");
  useEffect(() => { if (initialWorkspaceId) setSelectedId(initialWorkspaceId); }, [initialWorkspaceId]);
  useEffect(() => {
    if (!intent) return;
    if (intent.target === "workspaces_create" || !activeWorkspaces.length) {
      setMode("new");
      return;
    }
    const sections: Partial<Record<FunctionIntentTarget, string>> = {
      workspaces_link_project: "workspace.link_project",
      workspaces_link_app: "workspace.link_app",
      workspaces_set_widget: "workspace.set_widget",
    };
    const relation = sections[intent.target];
    if (relation) {
      setMode("view");
      window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-function-section='${relation}'] input`)?.focus());
    }
  }, [intent?.key]);
  useEffect(() => { if (!activeWorkspaces.some((workspace) => workspace.id === selectedId)) setSelectedId(activeWorkspaces[0]?.id ?? ""); }, [activeWorkspaces, selectedId]);
  const selected = activeWorkspaces.find((workspace) => workspace.id === selectedId) ?? null;
  const linkedProjectIds = new Set(workspaceProjects.map((project) => project.id));
  const linkedAppIds = new Set(workspaceApps.map((app) => app.id));
  const hiddenWidgetIds = new Set(hiddenWidgets.filter((entry) => entry.workspaceId === selectedId).map((entry) => entry.widgetId));
  const refreshLinks = useCallback(async () => {
    if (!selectedId) {
      setWorkspaceProjects([]);
      setWorkspaceApps([]);
      return;
    }
    const [nextProjects, nextApps] = await Promise.all([api.workspaceProjects(selectedId), api.workspaceApps(selectedId)]);
    setWorkspaceProjects(nextProjects);
    setWorkspaceApps(nextApps);
  }, [selectedId]);
  useEffect(() => { void refreshLinks().catch((error) => setMessage(appError(error).message)); }, [refreshLinks]);
  async function toggleProject(project: Project, linked: boolean) {
    if (!selected) return;
    try {
      await api.setProjectWorkspace(project.id, selected.id, linked);
      setMessage(linked ? "Project vinculado." : "Project removido do Workspace.");
      await refreshLinks();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }
  async function toggleApp(app: RegisteredApp, linked: boolean) {
    if (!selected) return;
    try {
      await api.setAppWorkspace(app.id, selected.id, linked);
      setMessage(linked ? "App vinculado." : "App removido do Workspace.");
      await refreshLinks();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }
  // `refresh` e nao `refreshLinks`: o dado dos ocultos vem do componente raiz,
  // nao do estado local desta pagina.
  async function toggleWidget(widget: { id: string; label: string }, visible: boolean) {
    if (!selected) return;
    try {
      await api.setWorkspaceWidget(widget.id, selected.id, visible);
      setMessage(visible ? "Widget visível na Home." : "Widget oculto na Home.");
      await refresh();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }
  return <div className="split-page workspaces-page">
    <section className="list-pane"><ContextPath segments={["M", "WORKSPACES"]} /><div className="list-command"><Button variant="outline" size="sm" onClick={() => setMode("new")}>Novo Workspace</Button></div><div className="row-list">{activeWorkspaces.map((workspace) => <DataRow key={workspace.id} primary={workspace.name} secondary={workspace.description || undefined} meta={relativeTime(workspace.updatedAt)} selected={workspace.id === selectedId} onClick={() => { setSelectedId(workspace.id); setMode("view"); setMessage(""); }} />)}</div>{!activeWorkspaces.length && mode !== "new" ? <EmptyState>Crie contextos amplos como Engineering, Finance ou Learning.</EmptyState> : null}</section>
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO WORKSPACE</span><WorkspaceForm cancel={() => setMode("view")} saved={(workspace) => { setSelectedId(workspace.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <WorkspaceForm workspace={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">WORKSPACE</span><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setWorkspaceArchived(selected.id, true).then(refresh)}>Arquivar</button></div></details></header><div className="workspace-grid"><div data-function-section="workspace.link_project"><Panel label="PROJECTS">{activeProjects.length ? activeProjects.map((project) => <div className="relation-row" key={project.id}><label><input type="checkbox" checked={linkedProjectIds.has(project.id)} onChange={(event) => void toggleProject(project, event.currentTarget.checked)} /><span><strong>{project.name}</strong><small>{project.description || "Sem descrição."}</small></span></label><button type="button" onClick={() => openProject(project)}>Abrir</button></div>) : <EmptyState>Projects ativos aparecerão aqui.</EmptyState>}</Panel></div><div data-function-section="workspace.link_app"><Panel label="APPS">{activeApps.length ? activeApps.map((app) => <div className="relation-row" key={app.id}><label><input type="checkbox" checked={linkedAppIds.has(app.id)} onChange={(event) => void toggleApp(app, event.currentTarget.checked)} /><span><strong>{app.name}</strong><small>{app.description || app.launchTarget || "Sem descrição."}</small></span></label><button type="button" onClick={() => openApp(app)}>Abrir</button></div>) : <EmptyState>Apps ativos aparecerão aqui.</EmptyState>}</Panel></div>{/* Caixa marcada significa VISIVEL: a interface fala em visivel, so a
                    tabela guarda o oculto. Sem botao Abrir — widget nao e entidade
                    que se abre. */}
      <div data-function-section="workspace.set_widget"><Panel label="WIDGETS">{HOME_WIDGETS.map((widget) => <div className="relation-row" key={widget.id}><label><input type="checkbox" checked={!hiddenWidgetIds.has(widget.id)} onChange={(event) => void toggleWidget(widget, event.currentTarget.checked)} /><span><strong>{widget.label}</strong><small>Widget da Home.</small></span></label></div>)}</Panel></div></div>{message ? <p className="settings-message" aria-live="polite">{message}</p> : null}</>}</> : null}</article>
  </div>;
}

function launchKindLabel(kind: AppLaunchKind | null) {
  if (kind === "url") return "URL";
  if (kind === "path") return "Path";
  return "Sem alvo";
}

function RegisteredAppForm({ app, cancel, saved }: { app?: RegisteredApp; cancel: () => void; saved: (app: RegisteredApp) => void }) {
  const [name, setName] = useState(app?.name ?? "");
  const [description, setDescription] = useState(app?.description ?? "");
  const [sourceUrl, setSourceUrl] = useState(app?.sourceUrl ?? "");
  const [launchKind, setLaunchKind] = useState<AppLaunchKind | "">((app?.launchKind ?? "") as AppLaunchKind | "");
  const [launchTarget, setLaunchTarget] = useState(app?.launchTarget ?? "");
  // Um app com alvo de lancamento ja abre — declarar o contrario seria mentir
  // sobre uma capacidade em uso. Mesma regra da migration 0007.
  const [capabilities, setCapabilities] = useState<AppCapabilities>(() => app ? { canOpen: app.canOpen, canRead: app.canRead, canWrite: app.canWrite, canAutomate: app.canAutomate } : { canOpen: true, canRead: false, canWrite: false, canAutomate: false });
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function choosePath(directory: boolean) {
    const selected = await open({ multiple: false, directory });
    if (typeof selected === "string") setLaunchTarget(selected);
  }
  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    const kind = launchKind || null;
    const target = launchTarget.trim() ? launchTarget : null;
    const source = sourceUrl.trim() ? sourceUrl : null;
    try {
      saved(app ? await api.updateRegisteredApp(app.id, name, description, source, kind, target, capabilities) : await api.createRegisteredApp(name, description, source, kind, target));
    } catch (nextError) {
      setError(appError(nextError).message);
      setSaving(false);
    }
  }
  return <form className="stack-form" onSubmit={submit}>
    <label><span>NOME</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} autoFocus /></label>
    <label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label>
    <label><span>ORIGEM</span><input value={sourceUrl} onChange={(event) => setSourceUrl(event.currentTarget.value)} placeholder="https://github.com/..." /></label>
    <label><span>TIPO DE ABERTURA</span><select value={launchKind} onChange={(event) => { setLaunchKind(event.currentTarget.value as AppLaunchKind | ""); if (!event.currentTarget.value) setLaunchTarget(""); }}><option value="">Sem alvo por enquanto</option><option value="url">URL</option><option value="path">Path local</option></select></label>
    {launchKind ? <label><span>ALVO</span>{launchKind === "path" ? <div className="target-picker"><input value={launchTarget} onChange={(event) => setLaunchTarget(event.currentTarget.value)} placeholder={"C:\\Apps\\app.exe"} /><Button variant="outline" onClick={() => void choosePath(false)}>Escolher arquivo</Button><Button variant="ghost" onClick={() => void choosePath(true)}>Escolher pasta</Button></div> : <input value={launchTarget} onChange={(event) => setLaunchTarget(event.currentTarget.value)} placeholder="https://..." />}</label> : null}
    <fieldset className="capability-fieldset"><legend className="micro-label">CAPACIDADES</legend>{([["canOpen", "OPEN"], ["canRead", "READ"], ["canWrite", "WRITE"], ["canAutomate", "AUTOMATE"]] as const).map(([key, label]) => <label className="capability-check" key={key}><input type="checkbox" checked={capabilities[key]} onChange={(event) => setCapabilities((current) => ({ ...current, [key]: event.currentTarget.checked }))} /><span className="micro-label">{label}</span></label>)}</fieldset>
    {error ? <p className="inline-error" role="alert">! {error}</p> : null}
    <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={!name.trim() || saving}>{saving ? "Salvando" : "Salvar"}</Button></div>
  </form>;
}

function AppsPage({ apps, initialAppId, refresh, intent }: { apps: RegisteredApp[]; initialAppId: string; refresh: () => Promise<void>; intent?: FunctionIntent }) {
  const visibleApps = apps.filter((app) => app.lifecycleState === "active" || app.id === initialAppId);
  const [selectedId, setSelectedId] = useState(initialAppId || visibleApps[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new">("view");
  const [message, setMessage] = useState("");
  const [creatingSuggestions, setCreatingSuggestions] = useState(false);
  const [catalog, setCatalog] = useState<AppCatalogEntry[]>([]);
  const missingSuggestions = catalog.filter((suggestion) => !apps.some((app) => app.sourceUrl === suggestion.sourceUrl || app.name.toLowerCase() === suggestion.name.toLowerCase()));
  useEffect(() => { void api.appCatalog().then(setCatalog).catch((error) => setMessage(appError(error).message)); }, []);
  useEffect(() => { if (initialAppId) setSelectedId(initialAppId); }, [initialAppId]);
  useEffect(() => { if (intent?.target === "apps_register") setMode("new"); }, [intent?.key]);
  useEffect(() => { if (!visibleApps.some((app) => app.id === selectedId)) setSelectedId(visibleApps[0]?.id ?? ""); }, [visibleApps, selectedId]);
  const selected = visibleApps.find((app) => app.id === selectedId) ?? null;
  async function openApp(app: RegisteredApp) {
    try {
      await api.openRegisteredApp(app.id);
      setMessage("App aberto.");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    }
  }
  async function addSuggestions() {
    if (!missingSuggestions.length || creatingSuggestions) return;
    setCreatingSuggestions(true);
    try {
      const created = await api.registerAppCatalog(missingSuggestions.map((suggestion) => suggestion.id));
      const lastCreated = created[created.length - 1] ?? null;
      if (lastCreated) setSelectedId(lastCreated.id);
      setMode("view");
      setMessage(`${missingSuggestions.length} Apps conhecidos adicionados.`);
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setCreatingSuggestions(false);
    }
  }
  return <div className="split-page apps-page">
    <section className="list-pane"><ContextPath segments={["M", "APPS"]} /><div className="list-command"><Button variant="outline" size="sm" onClick={() => setMode("new")}>Novo App</Button>{missingSuggestions.length ? <Button variant="ghost" onClick={() => void addSuggestions()} disabled={creatingSuggestions}>{creatingSuggestions ? "Adicionando" : "Adicionar meus Apps"}</Button> : null}</div><div className="row-list">{visibleApps.map((app) => <DataRow key={app.id} primary={app.name} secondary={app.description || app.launchTarget || undefined} meta={app.lifecycleState === "archived" ? "ARQUIVADO" : launchKindLabel(app.launchKind)} selected={app.id === selectedId} onClick={() => { setSelectedId(app.id); setMode("view"); setMessage(""); }} />)}</div>{!visibleApps.length && mode !== "new" ? <EmptyState>Cadastre as ferramentas que você usa para não depender da memória.</EmptyState> : null}</section>
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO APP</span><RegisteredAppForm cancel={() => setMode("view")} saved={(app) => { setSelectedId(app.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <RegisteredAppForm app={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">APP</span><div className="app-identity"><span className="app-icon" aria-hidden="true">{selected.name.trim().charAt(0).toUpperCase()}</span><div><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div></div></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setRegisteredAppArchived(selected.id, true).then(refresh)}>Arquivar</button></div></details></header><div className="detail-actions"><Button variant="primary" onClick={() => void openApp(selected)} disabled={!selected.launchTarget || selected.lifecycleState !== "active"}>Abrir</Button><Button variant="secondary" onClick={() => setMode("edit")}>Editar</Button></div><dl className="fact-grid" data-framed><div><dt>TIPO</dt><dd>{launchKindLabel(selected.launchKind)}</dd></div><div><dt>ORIGEM</dt><dd>{selected.sourceUrl || <span className="fact-empty">Não definida</span>}</dd></div><div><dt>DESTINO</dt><dd className="mono-value">{selected.launchTarget || <span className="fact-empty">Não definido</span>}</dd></div><div><dt>ÚLTIMA ABERTURA</dt><dd>{selected.lastOpenedAt ? relativeTime(selected.lastOpenedAt) : <span className="fact-empty">Nunca</span>}</dd></div></dl><Panel label="CAPACIDADES" className="capability-panel">{([["OPEN", selected.canOpen], ["READ", selected.canRead], ["WRITE", selected.canWrite], ["AUTOMATE", selected.canAutomate]] as const).map(([label, granted]) => <div className="capability-row" key={label}><span className="micro-label">{label}</span><span data-granted={granted || undefined}>{granted ? "✓" : "—"}</span></div>)}</Panel><p className="pane-footnote">Capacidade não declarada é capacidade que o Hermes não tenta usar.</p>{message ? <p className="settings-message" aria-live="polite">{message}</p> : null}</>}</> : null}</article>
  </div>;
}

function ResourceForm({ resource, capture, cancel, saved }: { resource?: Resource; capture?: Capture; cancel: () => void; saved: (resource: Resource) => void }) {
  const captureContent = capture?.content.trim() ?? "";
  const captureIsUrl = /^https?:\/\//i.test(captureContent);
  const [url, setUrl] = useState(resource?.url ?? (captureIsUrl ? captureContent : ""));
  const [title, setTitle] = useState(resource?.title ?? "");
  const [note, setNote] = useState(resource?.note ?? (captureIsUrl ? "" : captureContent));
  // Uma Capture que nao e URL vira Note por padrao: o texto ja e o conteudo.
  const [kind, setKind] = useState<ResourceKind>(resource?.kind ?? (capture && !captureIsUrl ? "note" : "site"));
  const needsUrl = kind !== "note";
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (saving) return;
    if (needsUrl && !url.trim()) return;
    if (!needsUrl && !title.trim()) return;
    setSaving(true);
    try {
      const next = resource
        ? await api.updateResource(resource.id, kind, title, needsUrl ? url : "", note)
        : await api.createResource(kind, title, needsUrl ? url : "", note, capture?.id ?? null);
      saved(next);
    } catch (nextError) {
      setError(appError(nextError).message);
      setSaving(false);
    }
  }
  return <form className="stack-form" onSubmit={submit} aria-busy={saving}>
    <fieldset className="form-fields" disabled={saving}>
      <label><span>TIPO</span><select value={kind} onChange={(event) => setKind(event.currentTarget.value as ResourceKind)}><option value="site">Site</option><option value="library">Library</option><option value="image">Imagem</option><option value="note">Nota</option></select></label>
      {needsUrl ? <label><span>{kind === "image" ? "ENDEREÇO OU CAMINHO" : "URL"}</span><input value={url} onChange={(event) => setUrl(event.currentTarget.value)} placeholder={kind === "image" ? "https://... ou C:\\imagens\\hero.png" : "https://..."} autoFocus /></label> : null}
      <label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} placeholder={needsUrl ? "Opcional · usa a URL quando vazio" : "Obrigatório para uma nota"} autoFocus={!needsUrl} /></label>
      <label><span>POR QUÊ?</span><textarea value={note} onChange={(event) => setNote(event.currentTarget.value)} placeholder="O que merece ser lembrado sobre este link?" rows={4} /></label>
      {capture ? <div className="provenance"><span className="micro-label">ORIGEM PRESERVADA</span><span>{capture.content}</span><small>{sourceLabel(capture.source)} · {relativeTime(capture.capturedAt)}</small></div> : null}
      {error ? <p className="inline-error" role="alert">! {error} Os campos continuam aqui.</p> : null}
      <div className="form-actions"><Button variant="ghost" onClick={cancel}>Cancelar</Button><Button variant="primary" type="submit" disabled={saving || (needsUrl ? !url.trim() : !title.trim())}>{saving ? "Salvando" : "Salvar Resource"}</Button></div>
    </fieldset>
  </form>;
}

function LibraryPage({ resources, initialResourceId, initialResourceKey, refresh, receipt, openCapture, intent }: { resources: Resource[]; initialResourceId: string; initialResourceKey: number; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openCapture: (capture: Capture) => void; intent?: FunctionIntent }) {
  const activeResources = resources.filter((resource) => resource.lifecycleState === "active");
  const [selectedId, setSelectedId] = useState(initialResourceId || activeResources[0]?.id || "");
  const [mode, setMode] = useState<"view" | "edit" | "new">("view");
  const [narrowPane, setNarrowPane] = useState<"list" | "detail">(initialResourceId ? "detail" : "list");
  const [source, setSource] = useState<Capture | null>(null);
  const [sourceError, setSourceError] = useState(false);
  const [message, setMessage] = useState("");
  const [pendingAction, setPendingAction] = useState<"open" | "archive" | "trash" | "restore" | null>(null);
  const list = useRef<HTMLDivElement>(null);
  const detail = useRef<HTMLElement>(null);
  // Filtro e apresentacao sao preferencia de leitura, nao dado: vivem aqui e
  // nao no banco. O alternador GRID/LISTA e do proprio design.
  const [kindFilter, setKindFilter] = useState<ResourceKind | "all">("all");
  const [view, setView] = useState<"grid" | "list">("grid");
  // O workspace escolhido na Home nomeia o segmento do meio do caminho.
  const workspaceSegment = (localStorage.getItem("m-os-current-workspace-name") ?? "").toUpperCase() || null;
  const liveResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
  const visibleResources = kindFilter === "all" ? liveResources : liveResources.filter((resource) => resource.kind === kindFilter || resource.id === selectedId);
  const selected = visibleResources.find((resource) => resource.id === selectedId) ?? null;

  useEffect(() => {
    if (!initialResourceId) return;
    setSelectedId(initialResourceId);
    setMode("view");
    setNarrowPane("detail");
  }, [initialResourceId, initialResourceKey]);

  useEffect(() => {
    if (intent?.target !== "library_create") return;
    setMode("new");
    setNarrowPane("detail");
  }, [intent?.key]);

  useEffect(() => {
    const nextVisibleResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
    if (nextVisibleResources.some((resource) => resource.id === selectedId)) return;
    const nextId = nextVisibleResources[0]?.id ?? "";
    setSelectedId(nextId);
    if (!nextId) setNarrowPane("list");
  }, [resources, selectedId]);

  useEffect(() => {
    setSource(null);
    setSourceError(false);
    if (selected?.sourceCaptureId) void api.getCapture(selected.sourceCaptureId).then(setSource).catch(() => setSourceError(true));
  }, [selected?.id, selected?.sourceCaptureId]);

  function startNew() {
    setMode("new");
    setNarrowPane("detail");
    setMessage("");
  }

  function selectResource(resource: Resource) {
    setSelectedId(resource.id);
    setMode("view");
    setNarrowPane("detail");
    setMessage("");
  }

  function returnToList() {
    setMode("view");
    setNarrowPane("list");
    requestAnimationFrame(() => {
      const selectedRow = list.current?.querySelector<HTMLButtonElement>(".data-row[data-selected]");
      const emptyAction = list.current?.closest(".list-pane")?.querySelector<HTMLButtonElement>(".library-empty .button");
      (selectedRow ?? emptyAction)?.focus();
    });
  }

  async function openLink(resource: Resource) {
    setPendingAction("open");
    setMessage("");
    try {
      await api.openResource(resource.id);
      setMessage("Link aberto no navegador padrão.");
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function archive(resource: Resource) {
    setPendingAction("archive");
    try {
      await api.setResourceArchived(resource.id, true);
      receipt({ message: "Resource arquivado.", run: () => api.setResourceArchived(resource.id, false) });
      setSelectedId(activeResources.find((candidate) => candidate.id !== resource.id)?.id ?? "");
      setNarrowPane("list");
      setMessage("");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function trash(resource: Resource) {
    setPendingAction("trash");
    try {
      await api.trashResource(resource.id);
      receipt({ message: "Resource movido para a Lixeira.", run: () => api.restoreResource(resource.id) });
      setSelectedId(activeResources.find((candidate) => candidate.id !== resource.id)?.id ?? "");
      setNarrowPane("list");
      setMessage("");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  async function restore(resource: Resource) {
    setPendingAction("restore");
    setMessage("");
    try {
      await api.setResourceArchived(resource.id, false);
      setMessage("Resource restaurado para a Library.");
      await refresh();
    } catch (nextError) {
      setMessage(appError(nextError).message);
    } finally {
      setPendingAction(null);
    }
  }

  const libraryIsEmpty = !visibleResources.length && mode === "view";

  return <div className="split-page library-page" data-pane={narrowPane} data-empty={libraryIsEmpty || undefined}>
    <section className="list-pane" aria-labelledby="library-title">
      <h1 id="library-title" className="visually-hidden">Library</h1>
      {/* O caminho carrega o workspace ativo quando existe: M / WEB-DESIGN /
          LIBRARY. E o que diz de qual acervo voce esta olhando. */}
      <div className="pane-heading"><ContextPath segments={workspaceSegment ? ["M", workspaceSegment, "LIBRARY"] : ["M", "LIBRARY"]} /><span className="micro-label">{liveResources.length} {liveResources.length === 1 ? "ITEM" : "ITENS"}</span></div>
      {/* Filtros sao texto, nao chip: um chip por tipo viraria cinco caixas
          competindo com o acervo, que e o que importa nesta tela. */}
      <div className="filter-bar">
        <div className="filter-group" role="group" aria-label="Filtrar por tipo">
          {([["all", "TUDO"], ["site", "SITES"], ["library", "LIBRARIES"], ["image", "IMAGENS"], ["note", "NOTAS"]] as const).map(([value, label]) => <button key={value} type="button" className="filter-label" data-active={kindFilter === value || undefined} aria-pressed={kindFilter === value} onClick={() => setKindFilter(value)}>{label}</button>)}
        </div>
        <div className="filter-group" role="group" aria-label="Apresentação">
          {([["grid", "GRID"], ["list", "LISTA"]] as const).map(([value, label]) => <button key={value} type="button" className="filter-label" data-active={view === value || undefined} aria-pressed={view === value} onClick={() => setView(value)}>{label}</button>)}
        </div>
      </div>
      {visibleResources.length ? <div className="list-command"><Button variant="outline" size="sm" onClick={startNew}>Novo Resource</Button></div> : null}
      {view === "grid" ? <div className="tile-grid" aria-label="Resources salvos">{visibleResources.map((resource) => <button key={resource.id} type="button" className="tile" data-selected={resource.id === selectedId || undefined} onClick={() => selectResource(resource)} onDoubleClick={() => { if (resource.url) void api.openResource(resource.id); }}><span className="tile-face" aria-hidden="true"><span className="tile-kind">{resource.kind.toUpperCase()}</span></span><strong className="tile-title">{resource.title}</strong>{/* O motivo e o que torna o acervo recuperavel: ele nunca e omitido. */}<span className="tile-reason" data-missing={resource.note ? undefined : true}>{resource.note || "Sem motivo registrado — abra e diga por que isto merece ser lembrado."}</span><span className="tile-origin">{resourceHost(resource.url) || "LOCAL"}</span></button>)}</div> : <div ref={list} className="row-list" aria-label="Resources salvos">
        {visibleResources.map((resource) => <DataRow
          key={resource.id}
          primary={resource.title}
          secondary={resourceHost(resource.url)}
          meta={resource.lifecycleState === "archived" ? "ARQUIVADO" : relativeTime(resource.updatedAt)}
          selected={resource.id === selectedId}
          onClick={() => selectResource(resource)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              selectResource(resource);
              requestAnimationFrame(() => detail.current?.focus());
              return;
            }
            const nextIndex = moveListFocus(event);
            if (nextIndex === null) return;
            const nextResource = visibleResources[nextIndex];
            if (nextResource) {
              setSelectedId(nextResource.id);
              setMode("view");
              setMessage("");
            }
          }}
        />)}
      </div>}
      {!visibleResources.length && mode !== "new" ? <div className="library-empty"><EmptyState>Guarde um link junto do motivo pelo qual ele merece ser lembrado.</EmptyState><Button variant="primary" onClick={startNew}>Salvar primeiro link</Button></div> : null}
    </section>
    <article
      ref={detail}
      className="detail-pane"
      tabIndex={-1}
      aria-label="Detalhe do Resource"
      onKeyDown={(event) => {
        if (event.key === "Escape" && mode === "view") {
          event.preventDefault();
          returnToList();
        }
      }}
    >
      <div className="library-detail-nav"><Button variant="ghost" onClick={returnToList}><Icon name="back" />Voltar à lista</Button></div>
      {mode === "new" ? <>
        <span className="micro-label">NOVO RESOURCE</span>
        <ResourceForm
          cancel={() => {
            if (selected) {
              setMode("view");
              setNarrowPane("detail");
              requestAnimationFrame(() => detail.current?.focus());
            } else {
              returnToList();
            }
          }}
          saved={(resource) => {
            setSelectedId(resource.id);
            setMode("view");
            setNarrowPane("detail");
            void refresh().then(() => requestAnimationFrame(() => detail.current?.focus()));
          }}
        />
      </> : selected ? mode === "edit" ? <>
        <span className="micro-label">EDITAR RESOURCE</span>
        <ResourceForm
          resource={selected}
          cancel={() => { setMode("view"); requestAnimationFrame(() => detail.current?.focus()); }}
          saved={() => { setMode("view"); void refresh().then(() => requestAnimationFrame(() => detail.current?.focus())); }}
        />
      </> : <>
        <header className="detail-header">
          <div>
            <span className="micro-label">RESOURCE · LINK{selected.lifecycleState === "archived" ? " · ARQUIVADO" : ""}</span>
            <h1>{selected.title}</h1>
            <p className="resource-url">{selected.url}</p>
          </div>
          <details className="menu">
            <summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary>
            <div>
              <button disabled={pendingAction !== null} onClick={() => setMode("edit")}>Editar</button>
              {selected.lifecycleState === "active" ? <>
                <button disabled={pendingAction !== null} onClick={() => void archive(selected)}>{pendingAction === "archive" ? "Arquivando" : "Arquivar"}</button>
                <button disabled={pendingAction !== null} className="danger-text" onClick={() => void trash(selected)}>{pendingAction === "trash" ? "Movendo" : "Mover para a Lixeira"}</button>
              </> : <button disabled={pendingAction !== null} onClick={() => void restore(selected)}>{pendingAction === "restore" ? "Restaurando" : "Restaurar"}</button>}
            </div>
          </details>
        </header>
        <div className="resource-note"><span className="micro-label">POR QUÊ?</span><p>{selected.note || "Nenhum contexto adicional foi registrado."}</p></div>
        {source ? <div className="provenance"><span className="micro-label">ORIGEM</span><button type="button" onClick={() => openCapture(source)}>{source.content}</button><small>{sourceLabel(source.source)} · {relativeTime(source.capturedAt)}</small></div> : null}
        {sourceError ? <p className="inline-error" role="status">Não foi possível carregar a Capture de origem agora.</p> : null}
        <div className="detail-actions">
          <Button variant="primary" onClick={() => void openLink(selected)} disabled={selected.lifecycleState !== "active" || pendingAction !== null}>{pendingAction === "open" ? "Abrindo" : "Abrir link"}</Button>
          <Button variant="ghost" onClick={() => setMode("edit")} disabled={pendingAction !== null}>Editar</Button>
        </div>
        {message ? <p className="settings-message" aria-live="polite">{message}</p> : null}
      </> : null}
    </article>
  </div>;
}

function BoardPage({ tasks, projects, refresh, openTask, intent }: { tasks: Task[]; projects: Project[]; refresh: () => Promise<void>; openTask: (task: Task) => void; intent?: FunctionIntent }) {
  const [creating, setCreating] = useState(false);
  const [draggingTaskId, setDraggingTaskId] = useState<string | null>(null);
  const [dragOverState, setDragOverState] = useState<TaskState | null>(null);
  const pointerDrag = useRef<{ taskId: string; x: number; y: number; active: boolean } | null>(null);
  const suppressClickTaskId = useRef<string | null>(null);
  const board = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (intent?.target === "tasks_create") setCreating(true);
    if (intent?.target === "tasks_move") window.requestAnimationFrame(() => board.current?.focus());
  }, [intent?.key]);
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
  return <div className="page board-page"><div className="board-heading"><ContextPath segments={["M", "TASKS"]} />{!creating ? <Button variant="primary" onClick={() => setCreating(true)}>Criar Task</Button> : null}</div>{creating ? <DirectTaskForm projects={projects} cancel={() => setCreating(false)} saved={() => { setCreating(false); void refresh(); }} /> : null}<div ref={board} className="kanban" tabIndex={-1} aria-label="Kanban de Tasks">{stateOrder.map((state) => { const column = tasks.filter((task) => task.lifecycleState === "active" && task.state === state); const visible = column.slice(0, 20); return <section key={state} className="kanban-column" data-kanban-state={state} data-drop-target={dragOverState === state || undefined} onDragEnter={(event) => { event.preventDefault(); setDragOverState(state); }} onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setDragOverState(state); }} onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragOverState(null); }} onDrop={(event) => { event.preventDefault(); const task = draggedTask(event); finishDrag(); if (task) void move(task, state); }}><header><h2>{stateLabels[state]}</h2><span>{column.length}</span></header><div>{visible.map((task) => <DataRow key={task.id} primary={task.title} secondary={projects.find((project) => project.id === task.projectId)?.name} completed={task.state === "done"} onClick={() => { if (suppressClickTaskId.current === task.id) { suppressClickTaskId.current = null; return; } openTask(task); }} onKeyDown={(event) => keyboardMove(event, task)} onPointerDown={(event) => { if (event.button !== 0) return; pointerDrag.current = { taskId: task.id, x: event.clientX, y: event.clientY, active: false }; }} draggable onDragStart={(event) => { setDraggingTaskId(task.id); event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/task-id", task.id); event.dataTransfer.setData("text/plain", task.id); }} onDragEnd={finishDrag} />)}{!column.length ? <EmptyState>Nenhuma Task.</EmptyState> : null}{column.length > visible.length ? <p className="more-count">+ {column.length - visible.length} mais</p> : null}</div></section>; })}</div></div>;
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

type HermesTurn = { question: string; answer: string; reasoning: string };

/** Acumula o delta no turno corrente sem reagrupar nada: o servidor desliga o
 *  Nagle de proposito para preservar a cadencia. */
function appendToAnswer(turns: HermesTurn[], text: string): HermesTurn[] {
  if (!turns.length) return turns;
  return turns.map((turn, index) => index === turns.length - 1 ? { ...turn, answer: turn.answer + text } : turn);
}

function appendToReasoning(turns: HermesTurn[], text: string): HermesTurn[] {
  if (!turns.length) return turns;
  return turns.map((turn, index) => index === turns.length - 1 ? { ...turn, reasoning: turn.reasoning + text } : turn);
}

function CommandSurface({ close, closing = false, openCapture, openTask, openProject, openWorkspace, openApp, openResource, routeFunction }: {
  closing?: boolean; close: () => void; openCapture: (capture: Capture) => void; openTask: (task: Task) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openApp: (app: RegisteredApp) => void; openResource: (resource: Resource) => void; routeFunction: (definition: FunctionDefinition) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CommandResult[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [error, setError] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  const searchSequence = useRef(0);
  // Modo do campo. Tab alterna, e o modo fica visivel no proprio campo em vez
  // de ser folclore de atalho.
  const [mode, setMode] = useState<"search" | "hermes">("search");
  const [hermesStatus, setHermesStatus] = useState<HermesStatus | null>(null);
  const [turns, setTurns] = useState<HermesTurn[]>([]);
  const [running, setRunning] = useState(false);
  const [approval, setApproval] = useState<string | null>(null);
  const [showReasoning, setShowReasoning] = useState(false);
  useEffect(() => { input.current?.focus(); return () => previousFocus.current?.focus(); }, []);
  useEffect(() => {
    void hermes.status().then(setHermesStatus).catch(() => setHermesStatus(null));
    const subscriptions = [
      hermes.onState(setHermesStatus),
      hermes.onEvent((event) => {
        if (event.outcome === "delta") return setTurns((current) => appendToAnswer(current, event.text));
        if (event.outcome === "reasoning") return setTurns((current) => appendToReasoning(current, event.text));
        if (event.outcome === "complete") return setRunning(false);
        if (event.outcome === "busy") { setRunning(true); return; }
        if (event.outcome === "approval") return setApproval(event.prompt);
        if (event.outcome === "failed") { setRunning(false); return setTurns((current) => appendToAnswer(current, `\n${event.message}`)); }
        // tool e unknown_frame nao interrompem a leitura da resposta.
      }),
    ];
    return () => { subscriptions.forEach((subscription) => void subscription.then((dispose) => dispose())); };
  }, []);
  // Conexao preguicosa: UMA tentativa ao entrar no modo Hermes.
  //
  // O ref existe para nao repetir. Sem ele, o efeito reagia a hermesStatus.state,
  // e como uma falha de conexao anuncia Offline, o proprio efeito se redisparava:
  // connect -> offline -> connect, o mais rapido que o IPC permitisse. Com o
  // tunel aberto e senha errada isso martelava o login, que o gateway responde
  // com 429 — o loop trancaria a conta do dashboard do usuario.
  const connectAttempted = useRef(false);
  useEffect(() => {
    if (mode !== "hermes" || connectAttempted.current) return;
    if (hermesStatus?.state === "offline" && hermesStatus.hasCredentials) {
      connectAttempted.current = true;
      void hermes.connect().catch(() => undefined);
    }
  }, [mode, hermesStatus?.state, hermesStatus?.hasCredentials]);
  async function askHermes() {
    const text = query.trim();
    if (!text || running) return;
    setTurns((current) => [...current, { question: text, answer: "", reasoning: "" }]);
    setQuery("");
    setRunning(true);
    try { await hermes.send(text); }
    catch (nextError) { setRunning(false); setTurns((current) => appendToAnswer(current, String(nextError))); }
  }
  async function searchCommand(requestId: number) {
    try {
      const [items, resources, functions] = await Promise.all([api.search(query, includeArchived), api.searchResources(query, includeArchived), api.searchFunctions(query)]);
      if (requestId !== searchSequence.current) return;
      setResults([...items, ...resources.map((resource) => ({ kind: "resource" as const, resource })), ...functions.map((definition) => ({ kind: "function" as const, function: definition }))]);
      setActiveIndex(0);
      setError("");
    } catch (nextError) {
      if (requestId !== searchSequence.current) return;
      setResults([]);
      setError(appError(nextError).message);
    }
  }
  useEffect(() => {
    const requestId = ++searchSequence.current;
    setResults([]);
    setActiveIndex(0);
    setError("");
    if (!query.trim()) {
      return;
    }
    const timeout = window.setTimeout(() => void searchCommand(requestId), 80);
    return () => window.clearTimeout(timeout);
  }, [query, includeArchived]);
  useEffect(() => { document.getElementById(`command-result-${activeIndex}`)?.scrollIntoView({ block: "nearest" }); }, [activeIndex]);
  function openItem(item: CommandResult) {
    close();
    if (item.kind === "function") routeFunction(item.function);
    else if (item.kind === "project") openProject(item.project);
    else if (item.kind === "workspace") openWorkspace(item.workspace);
    else if (item.kind === "task") openTask(item.task);
    else if (item.kind === "app") openApp(item.app);
    else if (item.kind === "resource") openResource(item.resource);
    else if (item.derivedTask) openTask(item.derivedTask);
    else openCapture(item.capture);
  }
  function handleInputKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.nativeEvent.isComposing) return;
    if (event.key === "Tab") {
      event.preventDefault();
      setMode((current) => current === "search" ? "hermes" : "search");
      return;
    }
    if (mode === "hermes") {
      if (event.key === "Enter") { event.preventDefault(); void askHermes(); }
      return;
    }
    if (!results.length) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const direction = event.key === "ArrowDown" ? 1 : -1;
      setActiveIndex((current) => (current + direction + results.length) % results.length);
    }
    if (event.key === "Enter") {
      event.preventDefault();
      openItem(results[activeIndex] ?? results[0]);
    }
  }
  return <div className="overlay-backdrop command-backdrop" data-closing={closing || undefined} onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}>
    <section className="command-surface" role="dialog" aria-modal="true" aria-label="Command" onKeyDown={(event) => { if (event.key === "Escape") close(); }}>
      <div className="command-input"><span className="slash">/</span><input ref={input} aria-controls="command-results" value={query} onChange={(event) => setQuery(event.currentTarget.value)} onKeyDown={handleInputKeyDown} placeholder={mode === "hermes" ? "O que você quer fazer?" : "Buscar ou executar comando"} aria-label={mode === "hermes" ? "Perguntar ao Hermes" : "Buscar no M/OS"} /><span className="micro-label">ESC FECHA</span></div>
      {/* O modo fica visivel no campo, nao escondido num atalho que so quem
          leu o rodape descobre. */}
      <div className="command-modes" role="group" aria-label="Modo">
        {([["search", "Search"], ["hermes", "Hermes"]] as const).map(([value, label]) => <button key={value} type="button" className="command-mode" data-active={mode === value || undefined} aria-pressed={mode === value} onClick={() => { setMode(value); input.current?.focus(); }}>{label}</button>)}
        {mode === "hermes" && hermesStatus ? <span className="command-mode-state" data-state={hermesStatus.state}>{hermesStatus.state === "online" ? (hermesStatus.sessionReady ? "ONLINE" : "ABRINDO SESSÃO") : hermesStatus.state === "connecting" ? "CONECTANDO" : "OFFLINE"}</span> : null}
      </div>
      {mode === "hermes" ? <div className="hermes-thread" aria-live="polite">
        {hermesStatus && hermesStatus.state !== "online" ? <p className="hermes-offline">{hermesUnavailableLabel(hermesStatus)}</p> : null}
        {!turns.length && hermesStatus?.state === "online" ? <EmptyState>Pergunte alguma coisa. É o mesmo Hermes do WhatsApp, numa conversa separada.</EmptyState> : null}
        {turns.map((turn, index) => <div className="hermes-turn" key={index}>
          <p className="hermes-question">{turn.question}</p>
          {/* Barra de 2px em sodio: o marcador de autoria do sistema que o
              design ja define. Nao ha bolha, nao ha avatar. */}
          {turn.answer ? <p className="hermes-answer">{turn.answer}</p> : null}
          {turn.reasoning ? <details className="hermes-reasoning" open={showReasoning} onToggle={(event) => setShowReasoning(event.currentTarget.open)}><summary className="micro-label">RACIOCÍNIO</summary><p>{turn.reasoning}</p></details> : null}
        </div>)}
        {approval ? <div className="hermes-approval" role="alertdialog" aria-label="Aprovação do Hermes">
          <p>{approval}</p>
          <div className="form-actions">
            {/* Fechar sem escolher nega: o servidor tambem tem deny como
                default, e aprovar por omissao seria o pior erro deste caminho. */}
            <Button variant="ghost" onClick={() => { setApproval(null); void hermes.approve(false); }}>Negar</Button>
            <Button variant="primary" onClick={() => { setApproval(null); void hermes.approve(true); }}>Aprovar</Button>
          </div>
        </div> : null}
        {running ? <div className="hermes-running"><MosSymbol size={16} spinning /><Button variant="ghost" onClick={() => { setRunning(false); void hermes.interrupt(); }}>Cancelar</Button></div> : null}
      </div> : <>
      {query ? <label className="check-control"><input type="checkbox" checked={includeArchived} onChange={(event) => setIncludeArchived(event.currentTarget.checked)} /><span>Incluir arquivados</span></label> : null}
      <div id="command-results" className="command-results" aria-label="Resultados" aria-live="polite">
        {error ? <div className="command-error"><p>! {error}</p><Button variant="outline" onClick={() => void searchCommand(++searchSequence.current)}>Tentar novamente</Button></div> : null}
        {!query ? <EmptyState>Digite para buscar.</EmptyState> : null}
        {query && !error && !results.length ? <EmptyState>Nenhum resultado para “{query}”.</EmptyState> : null}
        {results.map((item, index) => {
          const type = item.kind === "function" ? "FUNCTION" : item.kind === "project" ? "PROJECT" : item.kind === "workspace" ? "WORKSPACE" : item.kind === "task" ? "TASK" : item.kind === "app" ? "APP" : item.kind === "resource" ? "RESOURCE" : item.derivedTask ? "TASK + CAPTURE" : "CAPTURE";
          const title = item.kind === "function" ? item.function.name : item.kind === "project" ? item.project.name : item.kind === "workspace" ? item.workspace.name : item.kind === "task" ? item.task.title : item.kind === "app" ? item.app.name : item.kind === "resource" ? item.resource.title : item.derivedTask?.title ?? item.capture.content;
          const context = item.kind === "function" ? `${item.function.id} · risco ${functionRiskLabels[item.function.risk]}` : item.kind === "project" ? item.project.description : item.kind === "workspace" ? item.workspace.description : item.kind === "task" ? item.project?.name : item.kind === "app" ? item.app.description || item.app.launchTarget || "" : item.kind === "resource" ? `${resourceHost(item.resource.url)}${item.resource.note ? ` · ${item.resource.note}` : ""}` : item.project?.name ?? item.capture.content;
          return <button id={`command-result-${index}`} aria-current={index === activeIndex ? "true" : undefined} data-active={index === activeIndex || undefined} key={`${item.kind}-${index}-${title}`} className="command-row" onFocus={() => setActiveIndex(index)} onMouseEnter={() => setActiveIndex(index)} onClick={() => openItem(item)}><span>{type}</span><strong>{title}</strong><small>{context}</small></button>;
        })}
      </div></>}
      <div className="command-footer">{(mode === "hermes" ? ["⏎ PERGUNTA", "TAB SEARCH", "ESC FECHA"] : ["↑↓ NAVEGA", "⏎ ABRE", "/ COMANDO", "TAB HERMES"]).map((hint) => <span key={hint}>{hint}</span>)}</div>
    </section>
  </div>;
}

function HermesSettings() {
  const [status, setStatus] = useState<HermesStatus | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    void hermes.status().then((next) => { setStatus(next); setBaseUrl(next.baseUrl); }).catch(() => undefined);
    const subscription = hermes.onState(setStatus);
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);

  async function save(event: FormEvent) {
    event.preventDefault();
    try {
      if (baseUrl.trim()) await hermes.setBaseUrl(baseUrl);
      if (username.trim() && password) await hermes.setCredentials(username, password);
      // A senha some da memoria do renderer assim que sai daqui. Ela vive no
      // Credential Manager, e nem o proprio campo a mantem.
      setPassword("");
      setMessage("Credencial guardada no Windows Credential Manager.");
      setStatus(await hermes.status());
    } catch (error) { setMessage(String(error)); }
  }

  const stateLabel = status?.state === "online" ? "Conectado" : status?.state === "connecting" ? "Conectando" : "Desconectado";
  return <Panel label="HERMES">
    <p className="support-copy">O M/OS é mais uma superfície do Hermes que já roda na sua VPS — a mesma que você usa pelo WhatsApp, numa conversa separada. O acesso é pelo túnel SSH; o M/OS não abre porta nem inicia o túnel.</p>
    <form className="stack-form" onSubmit={save}>
      <label><span>ENDEREÇO LOCAL</span><input className="mono-input" value={baseUrl} onChange={(event) => setBaseUrl(event.currentTarget.value)} placeholder="http://127.0.0.1:9119" /></label>
      <label><span>USUÁRIO</span><input value={username} onChange={(event) => setUsername(event.currentTarget.value)} autoComplete="off" /></label>
      <label><span>SENHA</span><input type="password" value={password} onChange={(event) => setPassword(event.currentTarget.value)} autoComplete="off" /></label>
      <div className="form-actions">
        <Button variant="ghost" onClick={() => void hermes.clearCredentials().then(() => hermes.status()).then(setStatus).catch(() => undefined)}>Remover credencial</Button>
        <Button variant="primary" type="submit">Salvar</Button>
      </div>
    </form>
    <dl className="fact-grid">
      <div><dt>ESTADO</dt><dd>{stateLabel}</dd></div>
      <div><dt>CREDENCIAL</dt><dd>{status?.hasCredentials ? "Configurada" : <span className="fact-empty">Não configurada</span>}</dd></div>
    </dl>
    {status?.detail ? <p className="support-copy">{status.detail}</p> : null}
    {message ? <p className="settings-message" aria-live="polite">{message}</p> : null}
  </Panel>;
}

function SettingsPage({ theme, setTheme, status, capturesArchived, capturesTrashed, projects, tasks, workspaces, apps, resources, trashedResources, refresh, intent }: { theme: Theme; setTheme: (theme: Theme) => void; status: AppStatus | null; capturesArchived: Capture[]; capturesTrashed: Capture[]; projects: Project[]; tasks: Task[]; workspaces: Workspace[]; apps: RegisteredApp[]; resources: Resource[]; trashedResources: Resource[]; refresh: () => Promise<void>; intent?: FunctionIntent }) {
  const [shortcut, setShortcut] = useState("Ctrl+Shift+Space");
  const [message, setMessage] = useState("");
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [restorePath, setRestorePath] = useState("");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [updateState, setUpdateState] = useState<"idle" | "checking" | "current" | "available" | "installing" | "error">("idle");
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress>({ downloaded: 0, total: null });
  const [functions, setFunctions] = useState<FunctionDefinition[]>([]);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { void api.functions().then(setFunctions).catch((error) => setMessage(appError(error).message)); }, []);
  async function backup() { const path = await save({ defaultPath: "m-os-backup.mos-backup", filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (path) void api.createBackup(path).then((receipt) => setMessage(`Backup criado: ${receipt.path}`)).catch((error) => setMessage(appError(error).message)); }
  async function exportData() { const path = await save({ defaultPath: "m-os-export.json", filters: [{ name: "JSON", extensions: ["json"] }] }); if (path) void api.exportJson(path).then((receipt) => setMessage(`Export criado: ${receipt.path}`)).catch((error) => setMessage(appError(error).message)); }
  async function chooseRestore() { const path = await open({ multiple: false, filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (!path) return; try { setInspection(await api.inspectBackup(path)); setRestorePath(path); dialog.current?.showModal(); } catch (error) { setMessage(appError(error).message); } }
  async function confirmRestore() { try { const safety = await api.restoreBackup(restorePath); dialog.current?.close(); setMessage(`Dados restaurados. Safety backup: ${safety.path}`); await refresh(); } catch (error) { setMessage(appError(error).message); } }
  async function checkUpdates() {
    setUpdateState("checking");
    setUpdateInfo(null);
    setUpdateProgress({ downloaded: 0, total: null });
    setMessage("");
    try {
      const update = await api.checkForUpdate();
      setUpdateInfo(update);
      setUpdateState(update ? "available" : "current");
      setMessage(update ? `Atualizacao ${update.version} disponivel.` : "M/OS ja esta atualizado.");
    } catch (error) {
      setUpdateState("error");
      setMessage(appError(error).message);
    }
  }
  useEffect(() => {
    if (intent?.target === "updates_check") void checkUpdates();
    if (intent?.target === "function_registry") window.requestAnimationFrame(() => document.querySelector<HTMLElement>("[data-panel='FUNCTIONS']")?.scrollIntoView({ block: "start" }));
  }, [intent?.key]);
  async function installUpdate() {
    setUpdateState("installing");
    setMessage("Baixando atualizacao...");
    try {
      await api.installUpdate(setUpdateProgress);
      setMessage("Atualizacao instalada. Reiniciando M/OS...");
    } catch (error) {
      setUpdateState("error");
      setMessage(appError(error).message);
    }
  }
  function updateProgressLabel() {
    if (updateState !== "installing") return null;
    if (!updateProgress.total) return "Baixando pacote de atualizacao...";
    const percent = Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100));
    return `Baixando atualizacao: ${percent}%`;
  }
  const archivedProjects = projects.filter((project) => project.lifecycleState === "archived");
  const archivedTasks = tasks.filter((task) => task.lifecycleState === "archived");
  const archivedApps = apps.filter((app) => app.lifecycleState === "archived");
  const archivedResources = resources.filter((resource) => resource.lifecycleState === "archived");
  const archivedWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "archived");
  const functionsByCategory = functionCategories.map((category) => ({ category, items: functions.filter((item) => item.category === category) })).filter((group) => group.items.length);
  return <div className="page settings-page"><ContextPath segments={["M", "SETTINGS"]} /><HermesSettings /><Panel label="APARÊNCIA"><div className="setting-row"><div><strong>Tema claro</strong><p>Dark permanece o padrão do sistema.</p></div><label className="switch"><input type="checkbox" checked={theme === "light"} onChange={(event) => setTheme(event.currentTarget.checked ? "light" : "dark")} /><span /></label></div></Panel><Panel label="ATUALIZAÇÕES"><div className="setting-row"><div><strong>Atualizar M/OS</strong><p>{updateInfo ? `Versão instalada: ${updateInfo.currentVersion} · disponível: ${updateInfo.version}` : "Procura uma versão assinada publicada no GitHub Releases."}</p>{updateInfo?.body ? <p className="support-copy">{updateInfo.body}</p> : null}{updateProgressLabel() ? <p className="support-copy">{updateProgressLabel()}</p> : null}</div><div className="button-line"><Button variant="secondary" onClick={() => void checkUpdates()} disabled={updateState === "checking" || updateState === "installing"}>{updateState === "checking" ? "Verificando" : "Verificar atualizações"}</Button>{updateState === "available" || updateState === "installing" ? <Button variant="primary" onClick={() => void installUpdate()} disabled={updateState === "installing"}>{updateState === "installing" ? "Instalando" : "Atualizar agora"}</Button> : null}</div></div></Panel><Panel label="CAPTURA RÁPIDA"><form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setShortcut(shortcut).then(setMessage).catch((error) => setMessage(appError(error).message)); }}><div><label htmlFor="shortcut">Atalho global</label><p>{status?.shortcut}</p></div><div className="inline-form"><input id="shortcut" value={shortcut} onChange={(event) => setShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form></Panel><Panel label="FUNCTIONS"><p className="support-copy">Registro local das capacidades internas ja existentes. Esta base nao executa automacoes, plugins ou Hermes.</p><div className="function-registry">{functionsByCategory.map((group) => <section key={group.category}><span className="micro-label">{functionCategoryLabels[group.category]}</span>{group.items.map((item) => <div className="function-row" key={item.id}><div><strong>{item.name}</strong><code>{item.id}</code><p>{item.description}</p></div><small>{functionRiskLabels[item.risk]} · {functionConfirmationLabels[item.confirmation]}</small></div>)}</section>)}</div></Panel><Panel label="DADOS E PORTABILIDADE"><p className="support-copy">Backups e exports podem conter dados pessoais em texto claro.</p><div className="button-line"><Button variant="secondary" onClick={() => void backup()}>Criar backup</Button><Button variant="outline" onClick={() => void chooseRestore()}>Restaurar backup</Button><Button variant="outline" onClick={() => void exportData()}>Exportar JSON</Button></div></Panel><Panel label="ARCHIVE E TRASH"><details className="disclosure"><summary>Captures arquivadas <span>{capturesArchived.length}</span></summary>{capturesArchived.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Captures <span>{capturesTrashed.length}</span></summary>{capturesTrashed.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Projects arquivados <span>{archivedProjects.length}</span></summary>{archivedProjects.map((project) => <div className="restore-row" key={project.id}><span>{project.name}</span><Button variant="ghost" onClick={() => void api.setProjectArchived(project.id, false).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Workspaces arquivados <span>{archivedWorkspaces.length}</span></summary>{archivedWorkspaces.map((workspace) => <div className="restore-row" key={workspace.id}><span>{workspace.name}</span><Button variant="ghost" onClick={() => void api.setWorkspaceArchived(workspace.id, false).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Apps arquivados <span>{archivedApps.length}</span></summary>{archivedApps.map((app) => <div className="restore-row" key={app.id}><span>{app.name}</span><Button variant="ghost" onClick={() => void api.setRegisteredAppArchived(app.id, false).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Resources arquivados <span>{archivedResources.length}</span></summary>{archivedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.setResourceArchived(resource.id, false).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Resources <span>{trashedResources.length}</span></summary>{trashedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.restoreResource(resource.id).then(refresh)}>Restaurar</Button></div>)}</details><details className="disclosure"><summary>Tasks arquivadas <span>{archivedTasks.length}</span></summary>{archivedTasks.map((task) => <div className="restore-row" key={task.id}><span>{task.title}</span><Button variant="ghost" onClick={() => void api.setTaskArchived(task.id, false).then(refresh)}>Restaurar</Button></div>)}</details></Panel><Panel label="INTEGRIDADE"><dl className="health-list"><div><dt>Banco</dt><dd>{status?.storage.integrity === "ok" ? "Íntegro" : status?.storage.integrity}</dd></div><div><dt>Schema</dt><dd>v{status?.storage.schemaVersion}</dd></div><div><dt>Durabilidade</dt><dd>{status?.storage.journalMode.toUpperCase()} / {status?.storage.synchronous}</dd></div><div><dt>Snapshot</dt><dd>{status?.snapshot}</dd></div></dl></Panel>{message ? <p className="settings-message" aria-live="polite">{message}</p> : null}<dialog ref={dialog} className="restore-dialog" onCancel={() => dialog.current?.close()}><span className="micro-label">RESTORE</span><h2>Substituir o dataset local?</h2><p>Um safety backup será criado primeiro. O arquivo contém {inspection?.captureCount} Captures e usa schema v{inspection?.schemaVersion}.</p><div className="form-actions"><Button variant="ghost" onClick={() => dialog.current?.close()}>Cancelar</Button><Button variant="danger" onClick={() => void confirmRestore()}>Restaurar</Button></div></dialog></div>;
}

function QuickCapture() {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "error">("idle");
  const [feedback, setFeedback] = useState("Enter para salvar · Esc para fechar");
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => { input.current?.focus(); const unlisten = listen("window-revealed", () => input.current?.focus()); return () => { void unlisten.then((dispose) => dispose()); }; }, []);
  async function submit(event: FormEvent) { event.preventDefault(); if (!content.trim() || state === "saving") return; setState("saving"); setFeedback("Salvando localmente..."); try { await api.createCapture(content, "quick_capture"); setContent(""); setState("idle"); setFeedback("Salvo na Inbox"); window.setTimeout(() => void api.hideQuickCapture(), 160); } catch (error) { setState("error"); setFeedback(`${appError(error).message} O texto continua aqui.`); } }
  // Os tres tracos de amplitude sao a unica presenca da voz em repouso — sem
  // icone de microfone. Ficam apagados ate a voz existir (fase adiada).
  return <main className="quick-shell"><form className="quick-capture" onSubmit={submit}>
    <div className="capture-line">
      <span className="capture-bar" aria-hidden="true" />
      <textarea ref={input} value={content} onChange={(event) => setContent(event.currentTarget.value)} onKeyDown={(event) => { if (event.key === "Escape") void api.hideQuickCapture(); if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} aria-label="Texto da captura" placeholder="What's on your mind?" rows={1} />
      {content ? null : <span className="capture-caret" aria-hidden="true" />}
      <span className="amplitude" aria-hidden="true"><i /><i /><i /><i /></span>
    </div>
    <div className="capture-footer"><span className="micro-label">⏎ SALVA E FECHA · ESC CANCELA</span><span className={`feedback ${state}`} aria-live="polite">{state === "error" ? feedback : ""}</span></div>
  </form></main>;
}

function DesktopApp() {
  const [page, setPage] = useState<Page>("home");
  const [recent, setRecent] = useState<Capture[]>([]);
  const [inbox, setInbox] = useState<Capture[]>([]);
  const [archived, setArchived] = useState<Capture[]>([]);
  const [trashed, setTrashed] = useState<Capture[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [apps, setApps] = useState<RegisteredApp[]>([]);
  const [resources, setResources] = useState<Resource[]>([]);
  const [trashedResources, setTrashedResources] = useState<Resource[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [hiddenWidgets, setHiddenWidgets] = useState<HiddenWidget[]>([]);
  const [commandOpen, setCommandOpen] = useState(false);
  // O overlay continua montado durante os 90ms de saida. Desmontar na hora
  // cortaria a animacao pela metade, que e pior que nao ter animacao.
  const [commandClosing, setCommandClosing] = useState(false);
  const [viewedCapture, setViewedCapture] = useState<Capture | null>(null);
  const [drawerTask, setDrawerTask] = useState<Task | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState("");
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState("");
  const [selectedAppId, setSelectedAppId] = useState("");
  const [selectedResourceId, setSelectedResourceId] = useState("");
  const [resourceOpenKey, setResourceOpenKey] = useState(0);
  const [functionIntent, setFunctionIntent] = useState<FunctionIntent | null>(null);
  const [undo, setUndo] = useState<UndoAction | null>(null);
  // Alimenta o indicador da topbar. Ate agora so existia implicito nas
  // chamadas de api.ts; o design exige que o sistema diga quando esta ocupado.
  const [busy, setBusy] = useState(false);
  const [bootState, setBootState] = useState<"loading" | "ready" | "error">("loading");
  const [bootMessage, setBootMessage] = useState("");
  const [showBootLoading, setShowBootLoading] = useState(false);
  const [theme, setThemeState] = useState<Theme>(() => localStorage.getItem("m-os-theme") === "light" ? "light" : "dark");
  const undoTimer = useRef<number | null>(null);
  const functionIntentKey = useRef(0);

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const [nextRecent, nextInbox, nextArchived, nextTrashed, nextProjects, nextWorkspaces, nextApps, nextResources, nextTrashedResources, nextTasks, nextStatus, nextHiddenWidgets] = await Promise.all([api.recent(), api.inbox(), api.archived(), api.trashed(), api.projects(true), api.workspaces(true), api.registeredApps(true), api.resources(true), api.trashedResources(), api.tasks(true), api.status(), api.hiddenWidgets()]);
      setRecent(nextRecent); setInbox(nextInbox); setArchived(nextArchived); setTrashed(nextTrashed); setProjects(nextProjects); setWorkspaces(nextWorkspaces); setApps(nextApps); setResources(nextResources); setTrashedResources(nextTrashedResources); setTasks(nextTasks); setStatus(nextStatus); setHiddenWidgets(nextHiddenWidgets);
      setDrawerTask((current) => current ? nextTasks.find((task) => task.id === current.id) ?? null : null);
    } finally {
      setBusy(false);
    }
  }, []);
  const initialize = useCallback(async () => {
    setBootState("loading");
    setBootMessage("");
    setShowBootLoading(false);
    const loadingTimer = window.setTimeout(() => setShowBootLoading(true), 150);
    try {
      await refresh();
      setBootState("ready");
    } catch (error) {
      setBootMessage(appError(error).message);
      setBootState("error");
    } finally {
      window.clearTimeout(loadingTimer);
      setShowBootLoading(false);
    }
  }, [refresh]);
  useEffect(() => {
    // O endereco do gateway e reaplicado antes de qualquer conexao: ele vive no
    // renderer porque nao e segredo, e sem isto voltava ao padrao a cada boot.
    void hermes.restoreBaseUrl();
    void initialize();
    const refreshFromEvent = () => void refresh().catch((error) => {
      setBootMessage(appError(error).message);
      setBootState("error");
    });
    const events = [listen("capture-changed", refreshFromEvent), listen("data-changed", refreshFromEvent), listen("dataset-restored", refreshFromEvent), listen("snapshot-status-changed", refreshFromEvent)];
    return () => { events.forEach((event) => void event.then((dispose) => dispose())); };
  }, [initialize, refresh]);
  useEffect(() => { document.documentElement.dataset.theme = theme; localStorage.setItem("m-os-theme", theme); }, [theme]);
  useEffect(() => { const handler = (event: globalThis.KeyboardEvent) => { if (event.ctrlKey && event.key.toLowerCase() === "k") { event.preventDefault(); setCommandOpen(true); } if (event.ctrlKey && event.key.toLowerCase() === "z" && undo) { event.preventDefault(); void undo.run().then(() => { setUndo(null); return refresh(); }); } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, [refresh, undo]);

  // ~5s: tempo de ler e decidir desfazer, sem virar mobilia na tela.
  function closeCommand() {
    setCommandClosing(true);
    window.setTimeout(() => { setCommandOpen(false); setCommandClosing(false); }, 90);
  }
  function showReceipt(action: UndoAction) { setUndo(action); if (undoTimer.current) window.clearTimeout(undoTimer.current); undoTimer.current = window.setTimeout(() => setUndo(null), 5_000); }
  function navigate(page: Page) { setFunctionIntent(null); setPage(page); }
  function openProject(project: Project) { setFunctionIntent(null); setSelectedProjectId(project.id); setPage("projects"); }
  function openWorkspace(workspace: Workspace) { setFunctionIntent(null); setSelectedWorkspaceId(workspace.id); setPage("workspaces"); }
  function openRegisteredApp(app: RegisteredApp) { setFunctionIntent(null); setSelectedAppId(app.id); setPage("apps"); }
  function openResource(resource: Resource) { setFunctionIntent(null); setSelectedResourceId(resource.id); setResourceOpenKey((key) => key + 1); setPage("library"); }
  function routeFunction(definition: FunctionDefinition) {
    const target = resolveFunctionTarget(definition);
    if (target === "quick_capture") {
      void api.showQuickCapture();
      return;
    }
    functionIntentKey.current += 1;
    setFunctionIntent({ target, key: functionIntentKey.current });
    if (target === "home_capture") setPage("home");
    else if (target === "inbox_process" || target === "inbox_create_task") setPage("inbox");
    else if (target === "tasks_create" || target === "tasks_move") setPage("tasks");
    else if (target === "projects_create") setPage("projects");
    else if (target === "library_create") setPage("library");
    else if (target === "inbox_create_resource") setPage("inbox");
    else if (target === "workspaces_create" || target === "workspaces_link_project" || target === "workspaces_link_app") setPage("workspaces");
    else if (target === "apps_register") setPage("apps");
    else setPage("settings");
  }
  // Seis destinos, na ordem do design: home · inbox · board · projects · library · apps.
  // O sistema tem oito paginas e o rail aceita seis. Workspaces entra pelo
  // Command; Settings fica no rodape do rail, que o README permite.
  // Sete destinos: o Hermes entrou como tela propria no desenho v0.1 do chat
  // completo, logo depois da Home. Ele deixou de ser so camada dentro do
  // Command — continua alcancavel por la, mas agora tem endereco.
  const nav: { page: Page; label: string; icon: IconName; count?: number }[] = [{ page: "home", label: "Home", icon: "home" }, { page: "hermes", label: "Hermes", icon: "hermes" }, { page: "inbox", label: "Inbox", icon: "inbox", count: inbox.length }, { page: "tasks", label: "Tasks", icon: "board" }, { page: "projects", label: "Projects", icon: "projects" }, { page: "library", label: "Library", icon: "library" }, { page: "apps", label: "Apps", icon: "apps" }];
  const pageLabels: Record<Page, string> = { home: "Home", hermes: "Hermes", inbox: "Inbox", tasks: "Tasks", projects: "Projects", library: "Library", apps: "Apps", workspaces: "Workspaces", settings: "Settings" };
  const pageMeta = useMemo(() => {
    if (page !== "home") return pageLabels[page].toUpperCase();
    return new Intl.DateTimeFormat("pt-BR", { weekday: "short", day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" }).format(new Date()).toUpperCase().replace(",", " ·");
  }, [page]);
  const pageContent = useMemo(() => {
    if (page === "hermes") return <HermesPage inbox={inbox} projects={projects} tasks={tasks} openProject={openProject} openResource={(id) => { const resource = resources.find((candidate) => candidate.id === id); if (resource) openResource(resource); }} />;
    if (page === "home") return <HomePage recent={recent} inbox={inbox} projects={projects} tasks={tasks} workspaces={workspaces} apps={apps} status={status} hiddenWidgets={hiddenWidgets} refresh={refresh} openCapture={setViewedCapture} openProject={openProject} openWorkspace={openWorkspace} openTask={setDrawerTask} openApp={openRegisteredApp} openInbox={() => setPage("inbox")} openTasksPage={() => setPage("tasks")} openProjectsPage={() => setPage("projects")} intent={functionIntent ?? undefined} />;
    if (page === "inbox") return <InboxPage captures={inbox} projects={projects} refresh={refresh} receipt={showReceipt} openTask={setDrawerTask} openResource={openResource} intent={functionIntent ?? undefined} />;
    if (page === "projects") return <ProjectsPage projects={projects} tasks={tasks} initialProjectId={selectedProjectId} refresh={refresh} openTask={setDrawerTask} intent={functionIntent ?? undefined} />;
    if (page === "workspaces") return <WorkspacesPage workspaces={workspaces} projects={projects} apps={apps} hiddenWidgets={hiddenWidgets} initialWorkspaceId={selectedWorkspaceId} refresh={refresh} openProject={openProject} openApp={openRegisteredApp} intent={functionIntent ?? undefined} />;
    if (page === "apps") return <AppsPage apps={apps} initialAppId={selectedAppId} refresh={refresh} intent={functionIntent ?? undefined} />;
    if (page === "library") return <LibraryPage resources={resources} initialResourceId={selectedResourceId} initialResourceKey={resourceOpenKey} refresh={refresh} receipt={showReceipt} openCapture={setViewedCapture} intent={functionIntent ?? undefined} />;
    if (page === "tasks") return <BoardPage tasks={tasks} projects={projects} refresh={refresh} openTask={setDrawerTask} intent={functionIntent ?? undefined} />;
    return <SettingsPage theme={theme} setTheme={setThemeState} status={status} capturesArchived={archived} capturesTrashed={trashed} projects={projects} tasks={tasks} workspaces={workspaces} apps={apps} resources={resources} trashedResources={trashedResources} refresh={refresh} intent={functionIntent ?? undefined} />;
  }, [page, recent, projects, workspaces, apps, resources, trashedResources, tasks, refresh, inbox, selectedProjectId, selectedWorkspaceId, selectedAppId, selectedResourceId, resourceOpenKey, theme, status, archived, trashed, functionIntent]);
  const content = bootState === "ready"
    ? pageContent
    : bootState === "error"
      ? <section className="page startup-state" role="alert"><h1>M/OS não abriu os dados locais com segurança.</h1><p>{bootMessage} Nenhuma alteração foi feita.</p><Button variant="primary" onClick={() => void initialize()}>Tentar novamente</Button></section>
      : showBootLoading
        ? <section className="page startup-state" role="status"><p>Abrindo dados locais...</p></section>
        : null;

  return <div className="app-shell"><aside className="nav-rail"><div className="rail-symbol" aria-hidden="true"><MosSymbol size={16} /></div><nav aria-label="Navegação principal">{nav.map((item) => <button key={item.page} aria-current={page === item.page ? "page" : undefined} aria-label={item.label} title={item.label} onClick={() => navigate(item.page)}><Icon name={item.icon} filled={page === item.page} />{/* Sem badge de contagem: o desenho nao tem, e um numero permanente no rail
    vira ansiedade de fundo. A contagem da Inbox aparece na Home e na propria
    tela, onde ela leva a uma acao. */}</button>)}</nav><div className="rail-footer"><IconButton label="Quick Capture" icon="capture" onClick={() => void api.showQuickCapture()} /><IconButton label="Settings" icon="settings" active={page === "settings"} onClick={() => navigate("settings")} /></div></aside><div className="main-column"><header className="topbar"><button className="command-trigger" onClick={() => setCommandOpen(true)}><span className="slash">/</span><span>Command</span><kbd>CTRL K</kbd></button>{/* O estado de sistema nao substitui o meta da pagina: os dois convivem, e o
    indicador de ocupado entra antes sem apagar onde voce esta. */}
<div className="system-state" aria-live="polite">{busy ? <><MosSymbol size={16} spinning /><span className="micro-label">SINCRONIZANDO</span></> : null}<span className="page-meta">{pageMeta}</span></div></header><main className="content">{content}</main></div>{commandOpen ? <CommandSurface closing={commandClosing} close={closeCommand} openCapture={setViewedCapture} openTask={setDrawerTask} openProject={openProject} openWorkspace={openWorkspace} openApp={openRegisteredApp} openResource={openResource} routeFunction={routeFunction} /> : null}{viewedCapture ? <CaptureViewer capture={viewedCapture} close={() => setViewedCapture(null)} /> : null}{drawerTask ? <TaskDrawer key={drawerTask.id} task={drawerTask} projects={projects} close={() => setDrawerTask(null)} refresh={refresh} openCapture={(capture) => { setDrawerTask(null); setViewedCapture(capture); }} /> : null}{undo ? <div className="receipt" role="status"><span>{undo.message}</span><button onClick={() => void undo.run().then(() => { setUndo(null); return refresh(); })}>DESFAZER · CTRL Z</button></div> : null}</div>;
}

export default function App() {
  return getCurrentWindow().label === "quick-capture" ? <QuickCapture /> : <DesktopApp />;
}

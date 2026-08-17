import { DragEvent, FormEvent, KeyboardEvent, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, appError } from "./api";
import { DotField } from "./DotField";
import { resolveFunctionTarget, type FunctionIntentTarget } from "./functionIntents";
import { hermes, type HermesConnectionState, type HermesFailure, type HermesStatus } from "./hermes";
import { HermesPage } from "./HermesPage";
import { AppIcon } from "./AppIcon";
import { Button } from "./Button";
import { ContextPath, EmptyState, Panel } from "./Surface";
import { Reminder } from "./Reminder";
import { BudgetRing, TodayHours, useTrackedTime, WeekByProject } from "./TimeWidgets";
import { TempoPage } from "./TempoPage";
import { Timer } from "./Timer";
import { Icon, type IconName } from "./Icon";
import { Ring, RingLabel } from "./Ring";
import { MonthDensity, TaskProgressRing, WeekRings } from "./Widgets";
import { MosSymbol } from "./Symbol";
import type { AppCapabilities, AppCatalogEntry, AppLaunchKind, AppStatus, BackupInspection, Capture, FunctionDefinition, HiddenWidget, ImportReport, Project, RegisteredApp, Resource, ResourceKind, ResourceWorkspace, SearchItem, Task, TaskState, UpdateInfo, UpdateProgress, Workspace } from "./types";
import "./App.css";

type Page = "home" | "hermes" | "inbox" | "projects" | "workspaces" | "apps" | "library" | "tasks" | "tempo" | "settings";
type UndoAction = { message: string; run: () => Promise<unknown> };

/**
 * Os atalhos que existem de verdade.
 *
 * A auditoria deu 1 de 10 em "ajuda e documentação", e o motivo não era falta
 * de recurso: era o contrário. O app é operável quase inteiro pelo teclado e
 * nada disso estava escrito em lugar nenhum — quem não descobrisse por acidente
 * nunca saberia.
 *
 * A lista é escrita à mão de propósito. Derivá-la dos handlers daria uma
 * garantia falsa de sincronia e produziria rótulos como "keydown ctrl+k"; o que
 * falta documentar aqui é o QUE a tecla faz, e isso só existe na cabeça de quem
 * escreveu. O preço é manutenção: atalho novo entra aqui na mão.
 */
const SHORTCUTS: { keys: string; does: string }[] = [
  { keys: "Ctrl + K", does: "Abrir a busca e os comandos" },
  { keys: "Ctrl + Z", does: "Desfazer a última ação, enquanto o recibo estiver na tela" },
  { keys: "Ctrl + 1…9", does: "Abrir o app na posição correspondente, na Home" },
  { keys: "Esc", does: "Fechar, cancelar ou interromper o que estiver em curso" },
  { keys: "↑ ↓ Home End", does: "Navegar entre as linhas de uma lista" },
  { keys: "Ctrl + N", does: "Nova conversa, no Hermes" },
  { keys: "Ctrl + /", does: "Mostrar ou ocultar a coluna de conversas, no Hermes" },
  { keys: "↑ (campo vazio)", does: "Editar a última pergunta enviada, no Hermes" },
  { keys: "Shift + Enter", does: "Quebrar linha em vez de enviar, no Hermes" },
];
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


function IconButton({ label, icon, active = false, onClick }: { label: string; icon: IconName; active?: boolean; onClick: () => void }) {
  return <button className="icon-button" type="button" aria-label={label} title={label} onClick={onClick}><Icon name={icon} filled={active} /></button>;
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
  { id: "timer", label: "CRONÔMETRO" },
  { id: "now", label: "EM ANDAMENTO" },
  // Os tres de tempo. Ids novos e nao renomeados: `week_rings` continua sendo a
  // semana de TASKS, e reaproveitar o id daria a quem ocultou um o outro
  // escondido sem ter pedido.
  { id: "today_hours", label: "HOJE" },
  { id: "week_by_project", label: "SEMANA POR PROJECT" },
  { id: "budget_ring", label: "META" },
  { id: "week_rings", label: "SEMANA" },
  { id: "task_progress", label: "CONCLUÍDO" },
  { id: "month_density", label: "MÊS" },
  { id: "recent", label: "RECENTES" },
  { id: "projects", label: "PROJECTS" },
  { id: "apps", label: "APPS" },
  { id: "recent_resources", label: "RECURSOS" },
  { id: "inbox_pulse", label: "INBOX" },
  { id: "quick_actions", label: "AÇÕES" },
  { id: "system_health", label: "SISTEMA" },
];

function ScopedEmptyState({ total, workspace, noun, onLink, linkLabel = "Vincular" }: { total: number; workspace: Workspace | null; noun: "app" | "project" | "resource"; onLink: () => void; linkLabel?: string }) {
  if (total === 0 || !workspace) {
    return <EmptyState>{noun === "app" ? "Apps cadastrados aparecerão aqui." : noun === "resource" ? "Referências salvas aparecerão aqui." : "Projects criados aparecerão aqui."}</EmptyState>;
  }
  const counted = noun === "app"
    ? `${total} ${total === 1 ? "app cadastrado" : "apps cadastrados"}`
    : noun === "resource"
      ? `${total} ${total === 1 ? "resource salvo" : "resources salvos"}`
      : `${total} ${total === 1 ? "Project criado" : "Projects criados"}`;
  return <div className="scoped-empty"><EmptyState>{`${counted}, nenhum em ${workspace.name}.`}</EmptyState><Button variant="outline" size="sm" onClick={onLink}>{linkLabel}</Button></div>;
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
  return <form className="capture-field" data-state={state} onSubmit={submit}>
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
  return <><span className="row-progress" aria-hidden="true"><i style={{ transform: `scaleX(${total ? done / total : 0})` }} /></span><span className="row-progress-count">{done}/{total}</span></>;
}

/** `secondaryKind` decide a familia da segunda linha. Origem de captura e tipo
 *  de lancamento sao dado de sistema e vao em mono; descricao de Project e
 *  texto do usuario e vai em grotesk. O AGENTS.md e explicito: mono nunca
 *  vaza para conteudo. */
function DataRow({ primary, meta, secondary, secondaryKind = "text", marker, progress, selected = false, completed = false, saved = false, dragging = false, onClick, onKeyDown, onPointerDown, draggable, onDragStart, onDragEnd }: { primary: string; meta?: string; secondary?: string; secondaryKind?: "text" | "system"; marker?: ReactNode; progress?: { done: number; total: number }; selected?: boolean; completed?: boolean; saved?: boolean; dragging?: boolean; onClick?: () => void; onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void; onPointerDown?: React.PointerEventHandler<HTMLButtonElement>; draggable?: boolean; onDragStart?: React.DragEventHandler<HTMLButtonElement>; onDragEnd?: React.DragEventHandler<HTMLButtonElement> }) {
  return <button className="data-row" type="button" aria-current={selected ? "true" : undefined} data-selected={selected || undefined} data-completed={completed || undefined} data-saved={saved || undefined} data-dragging={dragging || undefined} onClick={onClick} onKeyDown={onKeyDown} onPointerDown={onPointerDown} draggable={draggable} onDragStart={onDragStart} onDragEnd={onDragEnd}>{marker}<span className="row-copy"><strong>{primary}</strong>{secondary ? <small data-system={secondaryKind === "system" || undefined}>{secondary}</small> : null}</span>{progress ? <RowProgress done={progress.done} total={progress.total} /> : null}{meta ? <span className="row-meta">{meta}</span> : null}</button>;
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

function HomePage({ recent, inbox, projects, tasks, workspaces, apps, resources, resourceWorkspaces, status, hiddenWidgets, refresh, openCapture, openProject, openWorkspace, openTask, openApp, openResource, openInbox, openTasksPage, openTempoPage, openProjectsPage, openLibraryPage, currentWorkspaceId, setCurrentWorkspaceId, currentWorkspace, intent }: { recent: Capture[]; inbox: Capture[]; projects: Project[]; tasks: Task[]; workspaces: Workspace[]; apps: RegisteredApp[]; resources: Resource[]; resourceWorkspaces: ResourceWorkspace[]; status: AppStatus | null; hiddenWidgets: HiddenWidget[]; refresh: () => Promise<void>; openCapture: (capture: Capture) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openTask: (task: Task) => void; openApp: (app: RegisteredApp) => void; openResource: (resource: Resource) => void; openInbox: () => void; openTasksPage: () => void; openTempoPage: () => void; openProjectsPage: () => void; openLibraryPage: () => void; currentWorkspaceId: string; setCurrentWorkspaceId: (id: string) => void; currentWorkspace: Workspace | null; intent?: FunctionIntent }) {
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const [workspaceProjects, setWorkspaceProjects] = useState<Project[]>([]);
  const [workspaceApps, setWorkspaceApps] = useState<RegisteredApp[]>([]);
  // O contexto ativo e a persistencia dele moram no componente raiz desde que a
  // Library passou a filtrar por ele. Aqui so fica o que e da Home: buscar os
  // Projects e Apps daquele contexto.
  useEffect(() => {
    if (!currentWorkspaceId || !activeWorkspaces.some((workspace) => workspace.id === currentWorkspaceId)) {
      setWorkspaceProjects([]);
      setWorkspaceApps([]);
      return;
    }
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
  /** O atalho que o rótulo do tile promete.
   *
   *  Até aqui `app-shortcut` só era DESENHADO: a Home exibia ⌘1, ⌘2… e nenhum
   *  handler escutava. Rótulo que mente é pior que rótulo ausente — quem tenta
   *  uma vez e não funciona para de acreditar nos outros atalhos do app. E ⌘ é
   *  notação de macOS num aplicativo Windows.
   *
   *  Vive na Home, e não global, porque é aqui que os tiles estão: um atalho
   *  para a "quinta posição" de uma lista que você não está vendo não teria
   *  como ser previsto. */
  useEffect(() => {
    function handler(event: globalThis.KeyboardEvent) {
      if (!event.ctrlKey || event.altKey || event.shiftKey) return;
      const index = Number(event.key) - 1;
      if (!Number.isInteger(index) || index < 0 || index >= activeApps.length) return;
      event.preventDefault();
      openApp(activeApps[index]);
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activeApps, openApp]);
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
  // O tempo carrega por fora do `refresh()`: aquele é o caminho de boot do app
  // inteiro, e um erro no rastreio não pode ser motivo para a Home não abrir.
  const trackedTime = useTrackedTime();
  const hasBudget = trackedTime.tracking.some((entry) => entry.budgetMinutes > 0);
  // resources(true) traz arquivado junto — a Home so mostra o acervo vivo. A ordem
  // ja vem do banco por updated_at DESC (resource_repository.rs:185).
  const allActiveResources = resources.filter((resource) => resource.lifecycleState === "active");
  // Mesma regra dos vizinhos: com contexto ativo, so o que pertence a ele.
  const scopedResourceIds = new Set(currentWorkspace ? resourceWorkspaces.filter((link) => link.workspaceId === currentWorkspace.id).map((link) => link.resourceId) : []);
  const activeResources = currentWorkspace ? allActiveResources.filter((resource) => scopedResourceIds.has(resource.id)) : allActiveResources;
  const projectName = (id: string | null) => projects.find((project) => project.id === id)?.name;
  const isActiveToday = (project: Project) => new Date(project.updatedAt).toDateString() === new Date().toDateString();
  return <div className="page home-page">
    <DotField />
    <ContextPath segments={["M", "HOME"]} />
    <CaptureComposer onSaved={(capture) => { markSaved(capture); void refresh(); }} focusKey={intent?.target === "home_capture" ? intent.key : undefined} />
    <Panel label="CONTEXTO" rule action={currentWorkspace ? <Button variant="ghost" onClick={() => setCurrentWorkspaceId("")}>Todos</Button> : undefined}><div className="context-switcher">{activeWorkspaces.map((workspace) => <button key={workspace.id} type="button" data-selected={workspace.id === currentWorkspaceId || undefined} onClick={() => setCurrentWorkspaceId(workspace.id)} onDoubleClick={() => openWorkspace(workspace)}><strong>{workspace.name}</strong><small>{workspace.description || "Workspace"}</small></button>)}{!activeWorkspaces.length ? <EmptyState>Workspaces ativos aparecerão aqui.</EmptyState> : null}</div></Panel>
    {/* A HOME É QUATRO FAIXAS, e cada uma responde a uma pergunta.

        Onze widgets numa grade só liam como onze coisas do mesmo peso, e o
        PRODUCT.md §4 avisa contra exatamente isso: a Home responde "o que está
        acontecendo e o que preciso fazer?", não "tudo o que existe". A regra de
        carga cognitiva diz o mesmo por outro caminho — grupos de até quatro.

        Faixas em vez de rótulos de seção porque o app já usa rótulo mono em
        cada Panel; um segundo nível de rótulo acima deles seria rótulo sobre
        rótulo. O que separa as faixas é ar, não mais texto.

        Cada faixa é a sua própria grade de 12 colunas, e isso é o que torna o
        arranjo robusto: fecha sozinha, e esconder um widget só reflowa a faixa
        dele em vez de empurrar a Home inteira. Antes de existirem faixas, a
        ordem abria três buracos de 3 colunas no meio — era isso que se lia como
        bagunça, e não a quantidade.

        Preferi isto a `grid-auto-flow: dense`, que taparia vãos movendo itens
        para trás e faria a ordem visual divergir da ordem de foco. */}

    {/* AGORA. O que está acontecendo neste minuto, e nada mais: a hora que está
        correndo e o trabalho que está aberto. Os dois fecham a faixa em 12. */}
    <div className="home-grid">
      <Widget id="timer" hidden={hiddenIds.has("timer")} size="2x1"><Panel label="CRONÔMETRO"><Timer projects={projects} onChanged={() => void refresh()} /></Panel></Widget>
      <Widget id="now" hidden={hiddenIds.has("now")} size="2x1"><Panel label="EM ANDAMENTO" count={doing.length ? String(doing.length) : undefined}>{doing.length ? doing.map((task) => <DataRow key={task.id} primary={task.title} meta={projectName(task.projectId)} onClick={() => openTask(task)} />) : <EmptyState>Nada em andamento. Uma Task movida para Doing aparece aqui.</EmptyState>}</Panel></Widget>
    </div>

    {/* O TEMPO. Faixa própria, e não misturada com a de baixo, porque responde
        outra pergunta: aquela fala do trabalho: aqui fala das HORAS — que é de
        onde sai a renda de quem fatura por hora (ADR-036).

        Fecha em 12 exatamente: 3 + 6 + 3. A META vai por ÚLTIMO de propósito —
        ela some quando nenhum Project tem meta, e um vão no fim da faixa é só
        ar, enquanto no meio seria o buraco de 3 colunas que fazia a Home ler
        como bagunça.

        Todos mostram HORAS, nunca dinheiro: o valor passa por arredondamento e
        desconto de inatividade, que vivem no Rust, e repetir essa conta aqui
        criaria um segundo número capaz de divergir do que vai na fatura. */}
    <div className="home-grid">
      <Widget id="today_hours" hidden={hiddenIds.has("today_hours")} size="1x1"><Panel label="HOJE"><TodayHours time={trackedTime} /></Panel></Widget>
      <Widget id="week_by_project" hidden={hiddenIds.has("week_by_project")} size="2x1"><Panel label="SEMANA POR PROJECT"><WeekByProject time={trackedTime} projects={projects} onOpen={openTempoPage} /></Panel></Widget>
      {/* Escondido quando nenhum Project tem meta: um anel preenchido contra um
          alvo que ninguém definiu ensina a confiar numa medida que não existe. */}
      <Widget id="budget_ring" hidden={hiddenIds.has("budget_ring") || !hasBudget} size="1x1"><Panel label="META"><BudgetRing time={trackedTime} projects={projects} onOpen={openProject} /></Panel></Widget>
    </div>

    {/* O QUE ESTÁ ACONTECENDO. Anel, densidade e lista curta — tudo de relance,
        nada pedindo decisão, e é isso que os mantém no mesmo grupo.

        São cinco, um a mais que a regra de quatro por grupo. A alternativa era
        uma faixa pela metade, que lê pior que um grupo levemente cheio.

        RECENTES entra aqui e não na faixa de cima: capture recente é o pulso do
        que passou, não uma decisão pendente. A contagem verdadeira da Inbox é a
        do anel — o badge de RECENTES foi removido porque `list_recent` não
        filtra por `processing_state` (repository.rs:91) e o comando pede só 8
        (lib.rs:80), então o número mentia duas vezes.

        Os widgets do catálogo que ainda faltam dependem de calendário e de
        hábitos, que não existem no domínio, e por isso não estão aqui. Os de
        tempo rastreado saíram desta lista quando o CronoCAD foi absorvido, e
        ganharam faixa própria acima. */}
    <div className="home-grid">
      <Widget id="month_density" hidden={hiddenIds.has("month_density")} size="2x2"><Panel label="MÊS"><MonthDensity tasks={tasks} captures={recent} /></Panel></Widget>
      <Widget id="inbox_pulse" hidden={hiddenIds.has("inbox_pulse")} size="1x1"><Panel label="INBOX">{/* O numero cru vira anel. A proporcao mostrada e o que esta ENVELHECENDO
    dentro da Inbox, nao o tamanho dela: uma Inbox grande e processada hoje e
    saudavel, e uma pequena parada ha uma semana nao e. O anel vazio com o
    numero no centro le exatamente como "nada envelhecendo", que e o estado
    bom — e e por isso que zero nao desenha ponto de sodio. */}
<button type="button" className="pulse" onClick={() => openInbox()}><Ring size={88} segments={[{ value: inbox.length ? staleInbox / inbox.length : 0 }]}><RingLabel value={inboxCapped ? `${INBOX_PAGE}+` : String(inbox.length)} /></Ring><small>{inbox.length === 1 ? "capture por processar" : "captures por processar"}</small>{staleInbox ? <small className="pulse-stale">{staleInbox === 1 && !inboxCapped ? "1 com mais de 3 dias" : `${staleInbox}${inboxCapped ? "+" : ""} com mais de 3 dias`}</small> : null}</button></Panel></Widget>
      <Widget id="task_progress" hidden={hiddenIds.has("task_progress")} size="1x1"><Panel label="CONCLUÍDO"><TaskProgressRing tasks={tasks} /></Panel></Widget>
      <Widget id="week_rings" hidden={hiddenIds.has("week_rings")} size="2x1"><Panel label="SEMANA"><WeekRings tasks={tasks} onOpen={openTasksPage} /></Panel></Widget>
      <Widget id="recent" hidden={hiddenIds.has("recent")} size="2x1"><Panel label="RECENTES">{recent.length ? recent.map((capture) => <DataRow key={capture.id} primary={capture.content} meta={relativeTime(capture.capturedAt)} saved={savedIds.has(capture.id)} onClick={() => openCapture(capture)} />) : <EmptyState>Nada capturado ainda. O que você escrever no campo acima aparece aqui.</EmptyState>}</Panel></Widget>
    </div>

    {/* O ACERVO. Aqui você navega, não processa — e é por isso que nada nesta
        faixa tem contagem. O corte em 5 seria silencioso, então o link "Ver
        todos" só aparece quando existe algo além do corte; se cabem todos, o
        cabeçalho fica limpo. */}
    <div className="home-grid">
      <Widget id="projects" hidden={hiddenIds.has("projects")} size="2x2"><Panel label="PROJECTS" action={scopedProjects.length > 5 ? <Button variant="ghost" onClick={() => openProjectsPage()}>Ver todos</Button> : undefined}>{scopedProjects.slice(0, 5).map((project) => <DataRow key={project.id} primary={project.name} marker={<span className="project-dot" data-active={isActiveToday(project) || undefined} aria-hidden="true" />} meta={relativeTime(project.updatedAt)} onClick={() => openProject(project)} />)}{!scopedProjects.length ? <ScopedEmptyState total={projects.filter((project) => project.lifecycleState === "active").length} workspace={currentWorkspace} noun="project" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel></Widget>
      {/* O nome do app nao entra: o icone com a inicial e o atalho ja o
          identificam, e a linha de nomes competiria com as rows ao lado. */}
      <Widget id="apps" hidden={hiddenIds.has("apps")} size="2x1"><Panel label="APPS"><div className="app-row">{activeApps.map((app, index) => <button key={app.id} type="button" className="app-tile" onClick={() => openApp(app)} title={app.name} aria-label={app.name}><AppIcon app={app} />{index < 9 ? <span className="app-shortcut">Ctrl {index + 1}</span> : null}</button>)}</div>{!activeApps.length ? <ScopedEmptyState total={apps.filter((app) => app.lifecycleState === "active").length} workspace={currentWorkspace} noun="app" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel></Widget>
      <Widget id="recent_resources" hidden={hiddenIds.has("recent_resources")} size="2x1"><Panel label="RECURSOS" action={activeResources.length > 5 ? <Button variant="ghost" onClick={() => openLibraryPage()}>Ver todos</Button> : undefined}>{activeResources.length ? activeResources.slice(0, 5).map((resource) => <DataRow key={resource.id} primary={resource.title} secondary={resourceHost(resource.url)} meta={relativeTime(resource.updatedAt)} onClick={() => openResource(resource)} />) : <ScopedEmptyState total={allActiveResources.length} workspace={currentWorkspace} noun="resource" onLink={() => openLibraryPage()} linkLabel="Ver tudo" />}</Panel></Widget>
    </div>

    {/* O SISTEMA. Última faixa de propósito: atalho e tranquilidade não são a
        pergunta que abre a Home. SISTEMA não duplica INTEGRIDADE das Settings —
        aquele é diagnóstico (schema, WAL), este responde "está salvo?". */}
    <div className="home-grid">
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

  if (!captures.length) return <div className="page"><ContextPath segments={["M", "INBOX"]} /><EmptyState>Inbox limpa — tudo que você capturou já foi processado.</EmptyState></div>;
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

function ProjectsPage({ projects, tasks, initialProjectId, refresh, receipt, openTask, intent }: { projects: Project[]; tasks: Task[]; initialProjectId: string; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openTask: (task: Task) => void; intent?: FunctionIntent }) {
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
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO PROJECT</span><ProjectForm cancel={() => setMode("view")} saved={(project) => { setSelectedId(project.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <ProjectForm project={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">PROJECT</span><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setProjectArchived(selected.id, true).then(async () => { receipt({ message: "Project arquivado.", run: () => api.setProjectArchived(selected.id, false) }); await refresh(); })}>Arquivar</button></div></details></header><dl className="fact-grid"><div><dt>REPOSITÓRIO</dt><dd className="mono-value">{selected.repository || <span className="fact-empty">Nenhum associado</span>}</dd></div><div><dt>ATUALIZADO</dt><dd>{relativeTime(selected.updatedAt)}</dd></div></dl>{mode === "task" ? <DirectTaskForm projectId={selected.id} projects={projects} cancel={() => setMode("view")} saved={(task) => { setMode("view"); void refresh(); openTask(task); }} /> : <Panel label="TASKS" action={<Button variant="primary" onClick={() => setMode("task")}>Criar Task</Button>}>{relatedTasks.length ? relatedTasks.map((task) => <DataRow key={task.id} primary={task.title} meta={stateLabels[task.state]} completed={task.state === "done"} onClick={() => openTask(task)} />) : <EmptyState>Nenhuma Task neste Project.</EmptyState>}</Panel>}</>}</> : null}</article>
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

function WorkspacesPage({ workspaces, projects, apps, hiddenWidgets, initialWorkspaceId, refresh, receipt, openProject, openApp, intent }: { workspaces: Workspace[]; projects: Project[]; apps: RegisteredApp[]; hiddenWidgets: HiddenWidget[]; initialWorkspaceId: string; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openProject: (project: Project) => void; openApp: (app: RegisteredApp) => void; intent?: FunctionIntent }) {
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
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO WORKSPACE</span><WorkspaceForm cancel={() => setMode("view")} saved={(workspace) => { setSelectedId(workspace.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <WorkspaceForm workspace={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">WORKSPACE</span><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setWorkspaceArchived(selected.id, true).then(async () => { receipt({ message: "Workspace arquivado.", run: () => api.setWorkspaceArchived(selected.id, false) }); await refresh(); })}>Arquivar</button></div></details></header><div className="workspace-grid"><div data-function-section="workspace.link_project"><Panel label="PROJECTS">{activeProjects.length ? activeProjects.map((project) => <div className="relation-row" key={project.id}><label><input type="checkbox" checked={linkedProjectIds.has(project.id)} onChange={(event) => void toggleProject(project, event.currentTarget.checked)} /><span><strong>{project.name}</strong><small>{project.description || "Sem descrição."}</small></span></label><button type="button" onClick={() => openProject(project)}>Abrir</button></div>) : <EmptyState>Projects ativos aparecerão aqui.</EmptyState>}</Panel></div><div data-function-section="workspace.link_app"><Panel label="APPS">{activeApps.length ? activeApps.map((app) => <div className="relation-row" key={app.id}><label><input type="checkbox" checked={linkedAppIds.has(app.id)} onChange={(event) => void toggleApp(app, event.currentTarget.checked)} /><span><strong>{app.name}</strong><small>{app.description || app.launchTarget || "Sem descrição."}</small></span></label><button type="button" onClick={() => openApp(app)}>Abrir</button></div>) : <EmptyState>Apps ativos aparecerão aqui.</EmptyState>}</Panel></div>{/* Caixa marcada significa VISIVEL: a interface fala em visivel, so a
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

function AppsPage({ apps, initialAppId, refresh, receipt, intent }: { apps: RegisteredApp[]; initialAppId: string; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; intent?: FunctionIntent }) {
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
    <article className="detail-pane">{mode === "new" ? <><span className="micro-label">NOVO APP</span><RegisteredAppForm cancel={() => setMode("view")} saved={(app) => { setSelectedId(app.id); setMode("view"); void refresh(); }} /></> : selected ? <>{mode === "edit" ? <RegisteredAppForm app={selected} cancel={() => setMode("view")} saved={() => { setMode("view"); void refresh(); }} /> : <><header className="detail-header"><div><span className="micro-label">APP</span><div className="app-identity"><AppIcon app={selected} /><div><h1>{selected.name}</h1><p>{selected.description || "Sem descrição."}</p></div></div></div><details className="menu"><summary aria-label="Mais ações" title="Mais ações"><Icon name="more" /></summary><div><button onClick={() => setMode("edit")}>Editar</button><button className="danger-text" onClick={() => void api.setRegisteredAppArchived(selected.id, true).then(async () => { receipt({ message: "App arquivado.", run: () => api.setRegisteredAppArchived(selected.id, false) }); await refresh(); })}>Arquivar</button></div></details></header><div className="detail-actions"><Button variant="primary" onClick={() => void openApp(selected)} disabled={!selected.launchTarget || selected.lifecycleState !== "active"}>Abrir</Button><Button variant="secondary" onClick={() => setMode("edit")}>Editar</Button></div><dl className="fact-grid" data-framed><div><dt>TIPO</dt><dd>{launchKindLabel(selected.launchKind)}</dd></div><div><dt>ORIGEM</dt><dd>{selected.sourceUrl || <span className="fact-empty">Não definida</span>}</dd></div><div><dt>DESTINO</dt><dd className="mono-value">{selected.launchTarget || <span className="fact-empty">Não definido</span>}</dd></div><div><dt>ÚLTIMA ABERTURA</dt><dd>{selected.lastOpenedAt ? relativeTime(selected.lastOpenedAt) : <span className="fact-empty">Nunca</span>}</dd></div></dl><Panel label="CAPACIDADES" className="capability-panel">{([["OPEN", selected.canOpen], ["READ", selected.canRead], ["WRITE", selected.canWrite], ["AUTOMATE", selected.canAutomate]] as const).map(([label, granted]) => <div className="capability-row" key={label}><span className="micro-label">{label}</span><span data-granted={granted || undefined}>{granted ? "✓" : "—"}</span></div>)}</Panel><p className="pane-footnote">Capacidade não declarada é capacidade que o Hermes não tenta usar.</p>{message ? <p className="settings-message" aria-live="polite">{message}</p> : null}</>}</> : null}</article>
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

function LibraryPage({ resources, workspaces, resourceWorkspaces, currentWorkspace, initialResourceId, initialResourceKey, refresh, receipt, openCapture, intent }: { resources: Resource[]; workspaces: Workspace[]; resourceWorkspaces: ResourceWorkspace[]; currentWorkspace: Workspace | null; initialResourceId: string; initialResourceKey: number; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openCapture: (capture: Capture) => void; intent?: FunctionIntent }) {
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
  // Ligado por padrao quando ha contexto: o caminho anuncia o recorte, entao a
  // lista tem que cumpri-lo. Sem contexto ativo o estado nao tem efeito.
  const [scoped, setScoped] = useState(true);
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const linkedWorkspaceIds = new Set(resourceWorkspaces.filter((link) => link.resourceId === selectedId).map((link) => link.workspaceId));
  // O `currentWorkspace !== null` repetido abaixo nao e redundancia: o tsc nao
  // estreita um objeto a partir de um boolean derivado guardado em variavel.
  const scoping = scoped && currentWorkspace !== null;
  // O caminho so anuncia o recorte quando ele esta de fato aplicado. Anunciar
  // sem aplicar foi o que este ciclo veio corrigir.
  const workspaceSegment = scoping && currentWorkspace ? currentWorkspace.name.toUpperCase() : null;
  const scopedResourceIds = new Set(currentWorkspace ? resourceWorkspaces.filter((link) => link.workspaceId === currentWorkspace.id).map((link) => link.resourceId) : []);
  const liveResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
  // O selecionado nunca some da lista, mesmo fora do recorte: ele esta aberto ao
  // lado, e sumir da lista o que esta aberto e desorientador.
  const contextResources = scoping ? liveResources.filter((resource) => scopedResourceIds.has(resource.id) || resource.id === selectedId) : liveResources;
  const visibleResources = kindFilter === "all" ? contextResources : contextResources.filter((resource) => resource.kind === kindFilter || resource.id === selectedId);
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

  // Sem mensagem de sucesso: a caixa marcada ja e a confirmacao, e uma frase a
  // cada clique numa lista de cinco viraria ruido. O erro continua falando,
  // porque ai o silencio mentiria.
  async function toggleWorkspace(workspaceId: string, linked: boolean) {
    if (!selected) return;
    try {
      await api.setResourceWorkspace(selected.id, workspaceId, linked);
      await refresh();
    } catch (nextError) { setMessage(appError(nextError).message); }
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
      <div className="pane-heading"><ContextPath segments={workspaceSegment ? ["M", workspaceSegment, "LIBRARY"] : ["M", "LIBRARY"]} /><span className="micro-label">{contextResources.length} {contextResources.length === 1 ? "ITEM" : "ITENS"}</span></div>
      {/* Filtros sao texto, nao chip: um chip por tipo viraria cinco caixas
          competindo com o acervo, que e o que importa nesta tela. */}
      <div className="filter-bar">
        {/* Trocar de contexto continua sendo coisa da Home; aqui so se decide se
            o contexto vigente se aplica. Sem contexto ativo o grupo nao aparece:
            botao que nao muda nada e pior que botao nenhum. */}
        {currentWorkspace ? <div className="filter-group" role="group" aria-label="Filtrar por contexto">
          {([[true, "NESTE CONTEXTO"], [false, "TUDO"]] as const).map(([value, label]) => <button key={label} type="button" className="filter-label" data-active={scoped === value || undefined} aria-pressed={scoped === value} onClick={() => setScoped(value)}>{label}</button>)}
        </div> : null}
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
      {/* Dois vazios diferentes. Acervo vazio pede o primeiro link; recorte
          vazio com acervo cheio e o estado de TODO Workspace no dia seguinte a
          migration, e precisa dizer que o acervo esta intacto em vez de parecer
          perda de dado. */}
      {!visibleResources.length && mode !== "new" ? (scoping && liveResources.length ? <div className="library-empty"><ScopedEmptyState total={liveResources.length} workspace={currentWorkspace} noun="resource" onLink={() => setScoped(false)} linkLabel="Ver tudo" /></div> : <div className="library-empty"><EmptyState>Guarde um link junto do motivo pelo qual ele merece ser lembrado.</EmptyState><Button variant="primary" onClick={startNew}>Salvar primeiro link</Button></div>) : null}
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
      {/* Sem icone junto do texto: o design system lista "icone + texto em
          botao" entre o que nao existe. O rotulo sozinho ja diz para onde
          volta, que e mais do que a seta dizia. */}
      <div className="library-detail-nav"><Button variant="ghost" onClick={returnToList}>Voltar à lista</Button></div>
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
        {/* As duas perguntas se leem juntas: por que guardei isto, e a que lente
            pertence. Sem Workspace ativo o bloco nao aparece — marcar nada em
            lugar nenhum nao e escolha, e confusao. */}
        {activeWorkspaces.length ? <div className="resource-context"><span className="micro-label">CONTEXTO</span><div>{activeWorkspaces.map((workspace) => <label key={workspace.id}><input type="checkbox" checked={linkedWorkspaceIds.has(workspace.id)} onChange={(event) => void toggleWorkspace(workspace.id, event.currentTarget.checked)} /><span>{workspace.name}</span></label>)}</div></div> : null}
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
  /* Encerra QUALQUER arrasto, incluindo o registro do ponteiro.
     Nao limpar `pointerDrag.current` aqui era o bug mais grave do kanban: depois
     de um arrasto nativo o registro ficava armado com a task antiga, o proximo
     movimento de mouse passava dos 6px e o marcava como ativo, e o clique
     seguinte — em qualquer lugar do quadro — movia aquela task para a coluna sob
     o cursor. A task andava sozinha. */
  function finishDrag() {
    pointerDrag.current = null;
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
  /* Dois caminhos de arrasto, com hierarquia explicita.
     O nativo e o primario: da previa do cartao de graca e e o que o Windows
     espera. O do ponteiro e FALLBACK, acrescentado em ca5a831 porque o nativo
     nao dispara de forma confiavel dentro do WebView.
     A regra que faltava: quando o nativo comeca, ele desarma o registro do
     ponteiro no proprio onDragStart. Assim os dois nunca agem sobre o mesmo
     gesto — antes eles ficavam armados juntos e discordavam. */
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
  return <div className="page board-page"><div className="board-heading"><ContextPath segments={["M", "TASKS"]} />{/* A alternativa ao arrasto existia desde sempre e era invisivel.
          DESIGN-FOUNDATIONS 12 exige que o kanban ofereca caminho por teclado;
          oferecer sem dizer que existe e o mesmo que nao oferecer. */}
      <span className="board-hint micro-label">ALT ←→ MOVE</span>{!creating ? <Button variant="primary" onClick={() => setCreating(true)}>Criar Task</Button> : null}</div>{creating ? <DirectTaskForm projects={projects} cancel={() => setCreating(false)} saved={() => { setCreating(false); void refresh(); }} /> : null}<div ref={board} className="kanban" tabIndex={-1} aria-label="Kanban de Tasks">{stateOrder.map((state) => { const column = tasks.filter((task) => task.lifecycleState === "active" && task.state === state); const visible = column.slice(0, 20); return <section key={state} className="kanban-column" data-kanban-state={state} data-drop-target={dragOverState === state || undefined} onDragEnter={(event) => { event.preventDefault(); setDragOverState(state); }} onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setDragOverState(state); }} onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragOverState(null); }} onDrop={(event) => { event.preventDefault(); const task = draggedTask(event); finishDrag(); if (task) void move(task, state); }}><header><h2>{stateLabels[state]}</h2><span>{column.length}</span></header><div>{visible.map((task) => <DataRow key={task.id} primary={task.title} secondary={projects.find((project) => project.id === task.projectId)?.name} completed={task.state === "done"} dragging={draggingTaskId === task.id} onClick={() => { if (suppressClickTaskId.current === task.id) { suppressClickTaskId.current = null; return; } openTask(task); }} onKeyDown={(event) => keyboardMove(event, task)} onPointerDown={(event) => { if (event.button !== 0) return; pointerDrag.current = { taskId: task.id, x: event.clientX, y: event.clientY, active: false }; }} draggable onDragStart={(event) => { pointerDrag.current = null; setDraggingTaskId(task.id); event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/task-id", task.id); event.dataTransfer.setData("text/plain", task.id); }} onDragEnd={finishDrag} />)}{!column.length ? <EmptyState>Nenhuma Task.</EmptyState> : null}{column.length > visible.length ? <p className="more-count">+ {column.length - visible.length} mais</p> : null}</div></section>; })}</div></div>;
}

function TaskDrawer({ task, projects, close, refresh, receipt, openCapture }: { task: Task; projects: Project[]; close: () => void; refresh: () => Promise<void>; receipt: (action: UndoAction) => void; openCapture: (capture: Capture) => void }) {
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
  return <aside ref={drawer} className="task-drawer" aria-label="Detalhe da Task" tabIndex={-1} onKeyDown={(event) => { if (event.key === "Escape") close(); }}><header><span className="micro-label">TASK</span><IconButton label="Fechar" icon="close" onClick={close} /></header><form className="stack-form" onSubmit={submit}><label><span>TÍTULO</span><input value={title} onChange={(event) => setTitle(event.currentTarget.value)} /></label><label><span>DESCRIÇÃO</span><textarea value={description} onChange={(event) => setDescription(event.currentTarget.value)} rows={4} /></label><label><span>PROJECT</span><select value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)}><option value="">Sem Project</option>{projects.filter((project) => project.lifecycleState === "active").map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}</select></label><label><span>ESTADO</span><select value={state} onChange={(event) => setState(event.currentTarget.value as TaskState)}>{stateOrder.map((value) => <option key={value} value={value}>{stateLabels[value]}</option>)}</select></label>{source ? <div className="provenance"><span className="micro-label">ORIGEM</span><button type="button" onClick={() => openCapture(source)}>{source.content}</button><small>{sourceLabel(source.source)} · {relativeTime(source.capturedAt)}</small></div> : null}{error ? <p className="inline-error" role="alert">! {error}</p> : null}<div className="form-actions spread"><Button variant="danger" onClick={() => void api.setTaskArchived(task.id, true).then(async () => { receipt({ message: "Task arquivada.", run: () => api.setTaskArchived(task.id, false) }); await refresh(); close(); })}>Arquivar</Button><Button variant="primary" type="submit" disabled={!title.trim()}>Salvar</Button></div></form></aside>;
}

function CaptureViewer({ capture, close }: { capture: Capture; close: () => void }) {
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => dialog.current?.focus(), []);
  return <div className="overlay-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}><article ref={dialog} className="entity-viewer" role="dialog" aria-modal="true" tabIndex={-1} onKeyDown={(event) => { if (event.key === "Escape") close(); }}><header><span className="micro-label">CAPTURE</span><IconButton label="Fechar" icon="close" onClick={close} /></header><h1>{capture.content}</h1><dl><div><dt>ORIGEM</dt><dd>{sourceLabel(capture.source)}</dd></div><div><dt>ESTADO</dt><dd>{capture.lifecycleState === "archived" ? "Arquivada" : capture.processingState === "processed" ? "Processada" : "Na Inbox"}</dd></div><div><dt>CAPTURADA</dt><dd>{new Date(capture.capturedAt).toLocaleString("pt-BR")}</dd></div></dl></article></div>;
}

/* O Command NAO tem mais modo Hermes.
 *
 * Ele e esta tela assinavam `hermes-event` no mesmo barramento global, e o
 * Command monta POR CIMA da pagina — com as duas abertas, os deltas da mesma
 * resposta se dividiam entre dois estados independentes. Alem do defeito, eram
 * duas implementacoes da mesma thread.
 *
 * A divisao agora segue `UX-PRINCIPLES.md` §13: o Command encontra e executa, a
 * pagina Hermes conversa. Quem quer perguntar vai para a pagina — que e onde a
 * conversa fica guardada. */
function CommandSurface({ close, closing = false, openCapture, openTask, openProject, openWorkspace, openApp, openResource, routeFunction }: {
  closing?: boolean; close: () => void; openCapture: (capture: Capture) => void; openTask: (task: Task) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openApp: (app: RegisteredApp) => void; openResource: (resource: Resource) => void; routeFunction: (definition: FunctionDefinition) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CommandResult[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [error, setError] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  // O desfoque de borda so aparece quando ha conteudo abaixo do corte. Sem esta
  // medida ele seria decoracao: uma nevoa sobre espaco vazio, dizendo que ha
  // mais quando nao ha. Mede em scroll e resize, sem cronometro e sem RAF.
  const resultsPane = useRef<HTMLDivElement>(null);
  const [hasMoreBelow, setHasMoreBelow] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const previousFocus = useRef(document.activeElement as HTMLElement | null);
  const searchSequence = useRef(0);
  useEffect(() => { input.current?.focus(); return () => previousFocus.current?.focus(); }, []);
  useEffect(() => {
    const pane = resultsPane.current;
    if (!pane) { setHasMoreBelow(false); return; }
    const measure = () => setHasMoreBelow(pane.scrollTop + pane.clientHeight < pane.scrollHeight - 1);
    measure();
    pane.addEventListener("scroll", measure, { passive: true });
    const resize = new ResizeObserver(measure);
    resize.observe(pane);
    return () => { pane.removeEventListener("scroll", measure); resize.disconnect(); };
  }, [results.length, error]);

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
    // `Tab` volta a mover foco estrutural (`DESIGN-FOUNDATIONS.md` §12). Ele
    // trocava de modo, e o modo deixou de existir aqui.
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
      <div className="command-input"><span className="slash">/</span><input ref={input} aria-controls="command-results" value={query} onChange={(event) => setQuery(event.currentTarget.value)} onKeyDown={handleInputKeyDown} placeholder="Buscar ou executar comando" aria-label="Buscar no M/OS" /><span className="micro-label">ESC FECHA</span></div>
      {query ? <label className="check-control"><input type="checkbox" checked={includeArchived} onChange={(event) => setIncludeArchived(event.currentTarget.checked)} /><span>Incluir arquivados</span></label> : null}
      <div ref={resultsPane} id="command-results" className="command-results" aria-label="Resultados" aria-live="polite">
        {error ? <div className="command-error"><p>! {error}</p><Button variant="outline" onClick={() => void searchCommand(++searchSequence.current)}>Tentar novamente</Button></div> : null}
        {!query ? <EmptyState>Digite para buscar.</EmptyState> : null}
        {query && !error && !results.length ? <EmptyState>Nenhum resultado para “{query}”.</EmptyState> : null}
        {results.map((item, index) => {
          const type = item.kind === "function" ? "FUNCTION" : item.kind === "project" ? "PROJECT" : item.kind === "workspace" ? "WORKSPACE" : item.kind === "task" ? "TASK" : item.kind === "app" ? "APP" : item.kind === "resource" ? "RESOURCE" : item.derivedTask ? "TASK + CAPTURE" : "CAPTURE";
          const title = item.kind === "function" ? item.function.name : item.kind === "project" ? item.project.name : item.kind === "workspace" ? item.workspace.name : item.kind === "task" ? item.task.title : item.kind === "app" ? item.app.name : item.kind === "resource" ? item.resource.title : item.derivedTask?.title ?? item.capture.content;
          const context = item.kind === "function" ? `${item.function.id} · risco ${functionRiskLabels[item.function.risk]}` : item.kind === "project" ? item.project.description : item.kind === "workspace" ? item.workspace.description : item.kind === "task" ? item.project?.name : item.kind === "app" ? item.app.description || item.app.launchTarget || "" : item.kind === "resource" ? `${resourceHost(item.resource.url)}${item.resource.note ? ` · ${item.resource.note}` : ""}` : item.project?.name ?? item.capture.content;
          return <button id={`command-result-${index}`} aria-current={index === activeIndex ? "true" : undefined} data-active={index === activeIndex || undefined} key={`${item.kind}-${index}-${title}`} className="command-row" onFocus={() => setActiveIndex(index)} onMouseEnter={() => setActiveIndex(index)} onClick={() => openItem(item)}><span>{type}</span><strong>{title}</strong><small>{context}</small></button>;
        })}
      </div>
      {/* Tres camadas de desfoque crescente, ancoradas acima do rodape. So
          existem enquanto houver resultado abaixo do corte. */}
      {hasMoreBelow ? <div className="command-fade" aria-hidden="true"><i /><i /><i /></div> : null}
      <div className="command-footer">{["↑↓ NAVEGA", "⏎ ABRE", "/ COMANDO", "ESC FECHA"].map((hint) => <span key={hint}>{hint}</span>)}</div>
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
      // O provider `basic` do Hermes exige usuario E senha (o config.yaml
      // declara username como required). Antes, faltando um dos dois, a
      // chamada simplesmente nao acontecia — e a mensagem de sucesso aparecia
      // mesmo assim, afirmando ter guardado o que nunca foi guardado. Quem
      // preenchia so a senha clicava em Salvar, lia "Credencial guardada" e
      // ficava Offline para sempre, sem nada na tela explicando.
      const wantsCredential = username.trim().length > 0 || password.length > 0;
      if (wantsCredential && !(username.trim() && password)) {
        setMessage(username.trim()
          ? "Falta a senha. O Hermes exige usuário e senha."
          : "Falta o usuário. O Hermes exige usuário e senha — normalmente o mesmo login do dashboard.");
        return;
      }
      if (wantsCredential) {
        await hermes.setCredentials(username, password);
        // A senha some da memoria do renderer assim que sai daqui. Ela vive no
        // Credential Manager, e nem o proprio campo a mantem.
        setPassword("");
        setMessage("Credencial guardada no Windows Credential Manager.");
        // Conectar agora. O supervisor do raiz desiste em silencio quando nao ha
        // credencial, e nada o reagenda: sem este empurrao, guardar a senha nao
        // produziria efeito nenhum ate reabrir o app.
        void hermes.connect().catch(() => undefined);
      } else {
        setMessage("Endereço salvo.");
      }
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
  // A resposta do botao precisa morar ao lado do botao. Ela caia em `message`,
  // que renderiza no rodape da pagina — depois de Archive/Trash e Integridade,
  // varios scrolls abaixo de quem acabou de clicar. Estar atualizado e a
  // resposta MAIS comum, e era justamente a invisivel: clicar e nao ver nada
  // acontecer se le como botao morto.
  const [updateNote, setUpdateNote] = useState("");
  const [importing, setImporting] = useState(false);
  const [importReport, setImportReport] = useState<ImportReport | null>(null);
  const [importNote, setImportNote] = useState("");
  // Pergunta ao banco, e não à memória da sessão: fechar e reabrir o app não
  // deveria reabilitar um botão que não pode mais ser clicado.
  const [importedAt, setImportedAt] = useState<string | null>(null);
  useEffect(() => { void api.cronocadImportedAt().then(setImportedAt).catch(() => undefined); }, []);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress>({ downloaded: 0, total: null });
  const [functions, setFunctions] = useState<FunctionDefinition[]>([]);
  const dialog = useRef<HTMLDialogElement>(null);
  // Exclusao definitiva nao tem Undo, entao nao pode seguir a regra de
  // "executar e oferecer desfazer" que vale no resto do app (UX-PRINCIPLES 21).
  // Aqui vale a outra: acao destrutiva e inequivoca (UX-PRINCIPLES 54). O
  // dialogo nomeia o item e diz que o caminho de volta e o backup anterior.
  const deleteDialog = useRef<HTMLDialogElement>(null);
  const [pendingDelete, setPendingDelete] = useState<{ noun: string; label: string; run: () => Promise<unknown> } | null>(null);
  function askDelete(noun: string, label: string, run: () => Promise<unknown>) {
    setPendingDelete({ noun, label, run });
    deleteDialog.current?.showModal();
  }
  async function confirmDelete() {
    const target = pendingDelete;
    deleteDialog.current?.close();
    setPendingDelete(null);
    if (!target) return;
    try {
      await target.run();
      setMessage(`${target.noun} excluído definitivamente.`);
      await refresh();
    } catch (error) { setMessage(appError(error).message); }
  }
  useEffect(() => { void api.functions().then(setFunctions).catch((error) => setMessage(appError(error).message)); }, []);
  async function backup() { const path = await save({ defaultPath: "m-os-backup.mos-backup", filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (path) void api.createBackup(path).then((receipt) => setMessage(`Backup criado: ${receipt.path}`)).catch((error) => setMessage(appError(error).message)); }
  async function exportData() { const path = await save({ defaultPath: "m-os-export.json", filters: [{ name: "JSON", extensions: ["json"] }] }); if (path) void api.exportJson(path).then((receipt) => setMessage(`Export criado: ${receipt.path}`)).catch((error) => setMessage(appError(error).message)); }
  async function chooseRestore() { const path = await open({ multiple: false, filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] }); if (!path) return; try { setInspection(await api.inspectBackup(path)); setRestorePath(path); dialog.current?.showModal(); } catch (error) { setMessage(appError(error).message); } }
  async function confirmRestore() { try { const safety = await api.restoreBackup(restorePath); dialog.current?.close(); setMessage(`Dados restaurados. Safety backup: ${safety.path}`); await refresh(); } catch (error) { setMessage(appError(error).message); } }
  /** Traz as horas do CronoCAD. Caminho de mão única, roda uma vez.
   *
   *  O diálogo abre já no arquivo padrão quando o CronoCAD está instalado: o
   *  usuário não deveria precisar saber que `com.cronocad.app` existe. */
  async function importCronocad() {
    const suggested = await api.defaultCronocadPath().catch(() => null);
    const path = await open({
      multiple: false,
      defaultPath: suggested ?? undefined,
      filters: [{ name: "Banco do CronoCAD", extensions: ["sqlite", "db"] }],
    });
    if (!path) return;
    setImporting(true);
    setImportNote("");
    try {
      setImportReport(await api.importCronocad(path));
      setImportedAt(await api.cronocadImportedAt().catch(() => null));
      await refresh();
    } catch (error) {
      setImportNote(appError(error).message);
    }
    setImporting(false);
  }
  async function checkUpdates() {
    setUpdateState("checking");
    setUpdateInfo(null);
    setUpdateProgress({ downloaded: 0, total: null });
    setMessage("");
    setUpdateNote("Consultando o GitHub Releases…");
    try {
      const update = await api.checkForUpdate();
      setUpdateInfo(update);
      setUpdateState(update ? "available" : "current");
      setUpdateNote(update ? "" : "M/OS já está atualizado.");
    } catch (error) {
      setUpdateState("error");
      setUpdateNote(appError(error).message);
    }
  }
  useEffect(() => {
    if (intent?.target === "updates_check") void checkUpdates();
    if (intent?.target === "function_registry") window.requestAnimationFrame(() => document.querySelector<HTMLElement>("[data-panel='FUNCTIONS']")?.scrollIntoView({ block: "start" }));
  }, [intent?.key]);
  async function installUpdate() {
    setUpdateState("installing");
    setUpdateNote("");
    try {
      await api.installUpdate(setUpdateProgress);
      setUpdateNote("Atualização instalada. Reiniciando M/OS…");
    } catch (error) {
      setUpdateState("error");
      setUpdateNote(appError(error).message);
    }
  }
  /** Uma linha so, sempre no mesmo lugar: o progresso enquanto baixa, o
   *  recado nos demais estados. */
  function updateStatusLine() {
    if (updateNote) return updateNote;
    if (updateState !== "installing") return null;
    if (!updateProgress.total) return "Baixando pacote de atualização…";
    const percent = Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100));
    return `Baixando atualização: ${percent}%`;
  }
  const archivedProjects = projects.filter((project) => project.lifecycleState === "archived");
  const archivedTasks = tasks.filter((task) => task.lifecycleState === "archived");
  const archivedApps = apps.filter((app) => app.lifecycleState === "archived");
  const archivedResources = resources.filter((resource) => resource.lifecycleState === "archived");
  const archivedWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "archived");
  const functionsByCategory = functionCategories.map((category) => ({ category, items: functions.filter((item) => item.category === category) })).filter((group) => group.items.length);
  return <div className="page settings-page"><ContextPath segments={["M", "SETTINGS"]} /><HermesSettings /><Panel label="APARÊNCIA"><div className="setting-row"><div><strong>Tema claro</strong><p>Dark permanece o padrão do sistema.</p></div><label className="switch"><input type="checkbox" checked={theme === "light"} onChange={(event) => setTheme(event.currentTarget.checked ? "light" : "dark")} /><span /></label></div></Panel><Panel label="ATUALIZAÇÕES"><div className="setting-row"><div><strong>Atualizar M/OS</strong><p>{updateInfo ? `Versão instalada: ${updateInfo.currentVersion} · disponível: ${updateInfo.version}` : "Procura uma versão assinada publicada no GitHub Releases."}</p>{updateInfo?.body ? <p className="support-copy">{updateInfo.body}</p> : null}{updateStatusLine() ? <p className="support-copy" aria-live="polite">{updateStatusLine()}</p> : null}</div><div className="button-line"><Button variant="secondary" onClick={() => void checkUpdates()} disabled={updateState === "checking" || updateState === "installing"}>{updateState === "checking" ? "Verificando" : "Verificar atualizações"}</Button>{updateState === "available" || updateState === "installing" ? <Button variant="primary" onClick={() => void installUpdate()} disabled={updateState === "installing"}>{updateState === "installing" ? "Instalando" : "Atualizar agora"}</Button> : null}</div></div></Panel><Panel label="CAPTURA RÁPIDA"><form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setShortcut(shortcut).then(setMessage).catch((error) => setMessage(appError(error).message)); }}><div><label htmlFor="shortcut">Atalho global</label><p>{status?.shortcut}</p></div><div className="inline-form"><input id="shortcut" value={shortcut} onChange={(event) => setShortcut(event.currentTarget.value)} /><Button variant="primary" type="submit">Aplicar</Button></div></form></Panel><Panel label="ATALHOS"><p className="support-copy">O M/OS é operável quase inteiro pelo teclado. Nada aqui precisa ser decorado — esta lista existe para quando você quiser.</p><dl className="shortcut-list">{SHORTCUTS.map((entry) => <div key={entry.keys}><dt>{entry.keys}</dt><dd>{entry.does}</dd></div>)}</dl></Panel><Panel label="FUNCTIONS"><p className="support-copy">Registro local das capacidades internas ja existentes. Esta base nao executa automacoes, plugins ou Hermes.</p><div className="function-registry">{functionsByCategory.map((group) => <section key={group.category}><span className="micro-label">{functionCategoryLabels[group.category]}</span>{group.items.map((item) => <div className="function-row" key={item.id}><div><strong>{item.name}</strong><code>{item.id}</code><p>{item.description}</p></div><small>{functionRiskLabels[item.risk]} · {functionConfirmationLabels[item.confirmation]}</small></div>)}</section>)}</div></Panel><Panel label="CRONOCAD"><div className="setting-row"><div><strong>Importar horas do CronoCAD</strong><p>Traz projetos, sessões e pendências para o M/OS. As horas passam a pertencer aos Projects daqui, e o valor/hora de cada sessão é preservado como estava na época.</p><p className="support-copy">Vem tudo: sessões, pendências, programas monitorados, o histórico observado pelo sistema e a sua configuração de arredondamento — sem ela o valor cobrável aqui daria diferente do que o CronoCAD mostra. Roda uma vez, e o banco de origem é aberto somente para leitura. Compare o total com a tela dele antes de desinstalar.</p>{importReport ? <p className="support-copy" aria-live="polite">{importReport.projects} {importReport.projects === 1 ? "project" : "projects"} · {importReport.entries} {importReport.entries === 1 ? "sessão" : "sessões"} · {importReport.tasks} {importReport.tasks === 1 ? "task" : "tasks"} · <strong>{(importReport.trackedSeconds / 3600).toFixed(1)} h</strong>{importReport.activityEvents ? ` · ${importReport.activityEvents} eventos observados` : ""}{importReport.monitoredApps ? ` · ${importReport.monitoredApps} programas` : ""}{importReport.clients ? ` · ${importReport.clients} clientes` : ""}</p> : null}{importNote ? <p className="support-copy" aria-live="polite">{importNote}</p> : null}</div><div className="button-line"><Button variant="secondary" onClick={() => void importCronocad()} disabled={importing || Boolean(importedAt)}>{importing ? "Importando" : importedAt ? "Importado" : "Importar"}</Button></div></div></Panel><Panel label="DADOS E PORTABILIDADE"><p className="support-copy">Backups e exports podem conter dados pessoais em texto claro.</p><div className="button-line"><Button variant="secondary" onClick={() => void backup()}>Criar backup</Button><Button variant="outline" onClick={() => void chooseRestore()}>Restaurar backup</Button><Button variant="outline" onClick={() => void exportData()}>Exportar JSON</Button></div></Panel><Panel label="ARCHIVE E TRASH"><details className="disclosure"><summary>Captures arquivadas <span>{capturesArchived.length}</span></summary>{capturesArchived.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Capture", capture.content, () => api.deleteCapture(capture.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Captures <span>{capturesTrashed.length}</span></summary>{capturesTrashed.map((capture) => <div className="restore-row" key={capture.id}><span>{capture.content}</span><Button variant="ghost" onClick={() => void api.restore(capture.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Capture", capture.content, () => api.deleteCapture(capture.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Projects arquivados <span>{archivedProjects.length}</span></summary>{archivedProjects.map((project) => <div className="restore-row" key={project.id}><span>{project.name}</span><Button variant="ghost" onClick={() => void api.setProjectArchived(project.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Project", project.name, () => api.deleteProject(project.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Workspaces arquivados <span>{archivedWorkspaces.length}</span></summary>{archivedWorkspaces.map((workspace) => <div className="restore-row" key={workspace.id}><span>{workspace.name}</span><Button variant="ghost" onClick={() => void api.setWorkspaceArchived(workspace.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Workspace", workspace.name, () => api.deleteWorkspace(workspace.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Apps arquivados <span>{archivedApps.length}</span></summary>{archivedApps.map((app) => <div className="restore-row" key={app.id}><span>{app.name}</span><Button variant="ghost" onClick={() => void api.setRegisteredAppArchived(app.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("App", app.name, () => api.deleteRegisteredApp(app.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Resources arquivados <span>{archivedResources.length}</span></summary>{archivedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.setResourceArchived(resource.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Resource", resource.title, () => api.deleteResource(resource.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Lixeira de Resources <span>{trashedResources.length}</span></summary>{trashedResources.map((resource) => <div className="restore-row" key={resource.id}><span>{resource.title}</span><Button variant="ghost" onClick={() => void api.restoreResource(resource.id).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Resource", resource.title, () => api.deleteResource(resource.id))}>Excluir</Button></div>)}</details><details className="disclosure"><summary>Tasks arquivadas <span>{archivedTasks.length}</span></summary>{archivedTasks.map((task) => <div className="restore-row" key={task.id}><span>{task.title}</span><Button variant="ghost" onClick={() => void api.setTaskArchived(task.id, false).then(refresh)}>Restaurar</Button><Button variant="ghost" className="danger-text" onClick={() => askDelete("Task", task.title, () => api.deleteTask(task.id))}>Excluir</Button></div>)}</details></Panel><Panel label="INTEGRIDADE"><dl className="health-list"><div><dt>Banco</dt><dd>{status?.storage.integrity === "ok" ? "Íntegro" : status?.storage.integrity}</dd></div><div><dt>Schema</dt><dd>v{status?.storage.schemaVersion}</dd></div><div><dt>Durabilidade</dt><dd>{status?.storage.journalMode.toUpperCase()} / {status?.storage.synchronous}</dd></div><div><dt>Snapshot</dt><dd>{status?.snapshot}</dd></div></dl></Panel>{message ? <p className="settings-message" aria-live="polite">{message}</p> : null}<dialog ref={deleteDialog} className="restore-dialog" onCancel={() => { deleteDialog.current?.close(); setPendingDelete(null); }}><span className="micro-label">EXCLUSÃO DEFINITIVA</span><h2>Excluir {pendingDelete?.noun.toLowerCase()} “{pendingDelete?.label}”?</h2><p>Isto apaga o registro do banco. Não há Desfazer: o único caminho de volta é restaurar um backup anterior a esta ação.</p><div className="form-actions"><Button variant="ghost" onClick={() => { deleteDialog.current?.close(); setPendingDelete(null); }}>Cancelar</Button><Button variant="danger" onClick={() => void confirmDelete()}>Excluir</Button></div></dialog><dialog ref={dialog} className="restore-dialog" onCancel={() => dialog.current?.close()}><span className="micro-label">RESTORE</span><h2>Substituir o dataset local?</h2><p>Um safety backup será criado primeiro. O arquivo contém {inspection?.captureCount} Captures e usa schema v{inspection?.schemaVersion}.</p><div className="form-actions"><Button variant="ghost" onClick={() => dialog.current?.close()}>Cancelar</Button><Button variant="danger" onClick={() => void confirmRestore()}>Restaurar</Button></div></dialog></div>;
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
  const [resourceWorkspaces, setResourceWorkspaces] = useState<ResourceWorkspace[]>([]);
  // O contexto ativo deixou de ser assunto da Home: a Library filtra por ele.
  // Continua em localStorage porque e preferencia de leitura, nao dado do core.
  const [currentWorkspaceId, setCurrentWorkspaceId] = useState(() => localStorage.getItem("m-os-current-workspace") ?? "");
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
      const [nextRecent, nextInbox, nextArchived, nextTrashed, nextProjects, nextWorkspaces, nextApps, nextResources, nextTrashedResources, nextTasks, nextStatus, nextHiddenWidgets, nextResourceWorkspaces] = await Promise.all([api.recent(), api.inbox(), api.archived(), api.trashed(), api.projects(true), api.workspaces(true), api.registeredApps(true), api.resources(true), api.trashedResources(), api.tasks(true), api.status(), api.hiddenWidgets(), api.resourceWorkspaces()]);
      setRecent(nextRecent); setInbox(nextInbox); setArchived(nextArchived); setTrashed(nextTrashed); setProjects(nextProjects); setWorkspaces(nextWorkspaces); setApps(nextApps); setResources(nextResources); setTrashedResources(nextTrashedResources); setTasks(nextTasks); setStatus(nextStatus); setHiddenWidgets(nextHiddenWidgets); setResourceWorkspaces(nextResourceWorkspaces);
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
  /* Supervisor da ponte do Hermes.
   *
   * Antes a conexao era preguicosa e unica: uma tentativa, so ao entrar no modo
   * Hermes, com um ref impedindo a segunda. O freio existia por um motivo real
   * — sem ele, uma falha anunciava Offline, o efeito se redisparava, e com
   * senha errada isso martelava o login ate o gateway responder 429 e travar a
   * conta do dashboard.
   *
   * O freio agora e outro, e mais preciso: so a falha de TRANSPORTE e repetida.
   * `retriable` vem tipado da ponte (HermesError::retriable), entao a decisao
   * nao depende de ler substring de mensagem. Tunel fechado tenta de novo,
   * espaçando ate cinco minutos; credencial recusada e rate limit param, e
   * param para sempre — quem mexe em credencial e o usuario, em Settings, e e
   * a acao dele que deve destravar a proxima tentativa.
   *
   * Sem credencial configurada nao ha nem primeira tentativa. */
  useEffect(() => {
    let cancelled = false;
    let timer = 0;
    let delay = 5_000;
    // Trava definitiva. A ponte anuncia Offline em TODA falha, inclusive nas que
    // nao se deve repetir — sem esta trava o onState abaixo reagendaria mesmo
    // depois de uma credencial recusada, e o laco do 429 voltaria por outra
    // porta. Só acao do usuario em Settings destrava, remontando a aplicacao.
    let stopped = false;
    // Instante em que a conexao ficou de pe. Serve para distinguir uma sessao
    // que durou de uma que caiu no nascimento — sem essa distincao a espera
    // nunca crescia no caso pior.
    let onlineSince = 0;
    // A espera cresce a CADA agendamento, e nao so quando connect() lanca.
    //
    // Este era o defeito que produzia o laco: hermes_connect devolve Ok assim
    // que o handshake passa, e a sessao pode morrer logo depois. Nesse caminho
    // o catch nunca roda, a espera ficava em 5s, e o app refazia o login
    // inteiro a cada cinco segundos — ate o gateway responder 429.
    function schedule() {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void attempt(), delay);
      delay = Math.min(delay * 2, 300_000);
    }
    async function attempt() {
      if (cancelled || stopped) return;
      try {
        const current = await hermes.status();
        if (cancelled) return;
        // Ja online ou a caminho: nada a fazer. Se cair, onState reage.
        if (current.state !== "offline") return;
        if (!current.hasCredentials) return;
        await hermes.connect();
      } catch (error) {
        if (cancelled) return;
        const failure = error as Partial<HermesFailure> | null;
        if (!failure?.retriable) { stopped = true; return; }
        schedule();
      }
    }
    void attempt();
    const subscription = hermes.onState((next) => {
      // A queda do socket rearma o supervisor. O timer unico impede que varios
      // anuncios de Offline em sequencia virem varias tentativas paralelas.
      if (next.state === "online") { onlineSince = Date.now(); return; }
      if (next.state !== "offline" || stopped) return;
      // So uma conexao que se sustentou merece recomecar a contagem do zero.
      // Um minuto de pe significa que o problema anterior passou; cair em
      // seguida significa que nao passou, e insistir no mesmo ritmo e o que
      // vira martelo.
      if (onlineSince && Date.now() - onlineSince > 60_000) delay = 5_000;
      onlineSince = 0;
      schedule();
    });
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      void subscription.then((dispose) => dispose());
    };
  }, []);
  // A chave `m-os-current-workspace-name` deixou de ser escrita: existia so para
  // a Library desenhar o segmento do caminho sem ter o objeto. Com o Workspace
  // chegando por prop, guardar o nome seria uma segunda fonte de verdade.
  const currentWorkspace = workspaces.find((workspace) => workspace.id === currentWorkspaceId && workspace.lifecycleState === "active") ?? null;
  useEffect(() => {
    if (!currentWorkspace) {
      localStorage.removeItem("m-os-current-workspace");
      return;
    }
    localStorage.setItem("m-os-current-workspace", currentWorkspace.id);
  }, [currentWorkspace]);
  useEffect(() => { const handler = (event: globalThis.KeyboardEvent) => { if (event.ctrlKey && event.key.toLowerCase() === "k") { event.preventDefault(); setCommandOpen(true); } if (event.ctrlKey && event.key.toLowerCase() === "z" && undo) { event.preventDefault(); void undo.run().then(() => { setUndo(null); return refresh(); }); } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, [refresh, undo]);

  // ~5s: tempo de ler e decidir desfazer, sem virar mobilia na tela.
  function closeCommand() {
    setCommandClosing(true);
    window.setTimeout(() => { setCommandOpen(false); setCommandClosing(false); }, 90);
  }
  function showReceipt(action: UndoAction) { setUndo(action); if (undoTimer.current) window.clearTimeout(undoTimer.current); undoTimer.current = window.setTimeout(() => setUndo(null), 5_000); }
  /* Posicao de leitura por pagina.
   *
   * `UX-PRINCIPLES` §37 pede preservar contexto ao navegar — filtros, posicao,
   * selecao — e a posicao era a que se perdia: voltar para a Inbox depois de
   * abrir um Project devolvia o topo da lista, e reencontrar onde se estava e
   * exatamente o custo mental que o produto existe para remover.
   *
   * A posicao e guardada continuamente sob a pagina que esta na tela, e nao no
   * momento do clique, porque `setPage` e chamado de sete lugares diferentes —
   * salvar em cada um deles seria uma linha para esquecer na oitava. */
  const contentRef = useRef<HTMLElement>(null);
  const scrollByPage = useRef(new Map<Page, number>());
  const shownPage = useRef<Page>(page);

  useEffect(() => {
    const node = contentRef.current;
    if (!node) return;
    const remember = () => scrollByPage.current.set(shownPage.current, node.scrollTop);
    node.addEventListener("scroll", remember, { passive: true });
    return () => node.removeEventListener("scroll", remember);
  }, []);

  // `useLayoutEffect` para restaurar antes da pintura: com `useEffect` a pagina
  // aparece no topo e pula para a posicao guardada, que e pior que nao guardar.
  useLayoutEffect(() => {
    const node = contentRef.current;
    if (!node) return;
    shownPage.current = page;
    node.scrollTop = scrollByPage.current.get(page) ?? 0;
  }, [page]);

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
  const nav: { page: Page; label: string; icon: IconName; count?: number }[] = [{ page: "home", label: "Home", icon: "home" }, { page: "hermes", label: "Hermes", icon: "hermes" }, { page: "inbox", label: "Inbox", icon: "inbox", count: inbox.length }, { page: "tasks", label: "Tasks", icon: "board" }, { page: "projects", label: "Projects", icon: "projects" }, /* Workspaces entra no rail. O icone ja existia desenhado em Icon.tsx desde o
     handoff, para um item que nunca foi acrescentado — e ate agora a unica
     porta era o Ctrl+K. Workspace nao e feature nova que precise justificar
     presenca na navegacao (DESIGN-FOUNDATIONS 5): e um dos conceitos centrais
     da VISION 7, e estava invisivel para quem nao conhece o Command. */
  { page: "workspaces", label: "Workspaces", icon: "workspaces" },
  /* Tempo e o NONO destino, e a ADR-036 revisou o teto da ADR-031 para caber
     ele. O argumento nao e frequencia de uso: e que o usuario fatura por hora,
     entao tempo rastreado e o registro de onde sai a renda dele — e isso nao
     vive atras de um Ctrl+K. Entra depois de Projects porque a hora sempre
     pertence a um Project. */
  { page: "tempo", label: "Tempo", icon: "tempo" }, { page: "library", label: "Library", icon: "library" }, { page: "apps", label: "Apps", icon: "apps" }];
  const pageLabels: Record<Page, string> = { home: "Home", hermes: "Hermes", inbox: "Inbox", tasks: "Tasks", projects: "Projects", tempo: "Tempo", library: "Library", apps: "Apps", workspaces: "Workspaces", settings: "Settings" };
  const pageMeta = useMemo(() => {
    if (page !== "home") return pageLabels[page].toUpperCase();
    return new Intl.DateTimeFormat("pt-BR", { weekday: "short", day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" }).format(new Date()).toUpperCase().replace(",", " ·");
  }, [page]);
  const pageContent = useMemo(() => {
    if (page === "hermes") return <HermesPage inbox={inbox} projects={projects} tasks={tasks} receipt={showReceipt} openProject={openProject} openResource={(id) => { const resource = resources.find((candidate) => candidate.id === id); if (resource) openResource(resource); }} />;
    if (page === "home") return <HomePage recent={recent} inbox={inbox} projects={projects} tasks={tasks} workspaces={workspaces} apps={apps} resources={resources} resourceWorkspaces={resourceWorkspaces} status={status} hiddenWidgets={hiddenWidgets} refresh={refresh} openCapture={setViewedCapture} openProject={openProject} openWorkspace={openWorkspace} openTask={setDrawerTask} openApp={openRegisteredApp} openResource={openResource} openInbox={() => setPage("inbox")} openTasksPage={() => setPage("tasks")} openTempoPage={() => setPage("tempo")} openProjectsPage={() => setPage("projects")} openLibraryPage={() => setPage("library")} currentWorkspaceId={currentWorkspaceId} setCurrentWorkspaceId={setCurrentWorkspaceId} currentWorkspace={currentWorkspace} intent={functionIntent ?? undefined} />;
    if (page === "tempo") return <TempoPage projects={projects} openProject={openProject} receipt={showReceipt} />;
    if (page === "inbox") return <InboxPage captures={inbox} projects={projects} refresh={refresh} receipt={showReceipt} openTask={setDrawerTask} openResource={openResource} intent={functionIntent ?? undefined} />;
    if (page === "projects") return <ProjectsPage projects={projects} tasks={tasks} initialProjectId={selectedProjectId} refresh={refresh} receipt={showReceipt} openTask={setDrawerTask} intent={functionIntent ?? undefined} />;
    if (page === "workspaces") return <WorkspacesPage workspaces={workspaces} projects={projects} apps={apps} hiddenWidgets={hiddenWidgets} initialWorkspaceId={selectedWorkspaceId} refresh={refresh} receipt={showReceipt} openProject={openProject} openApp={openRegisteredApp} intent={functionIntent ?? undefined} />;
    if (page === "apps") return <AppsPage apps={apps} initialAppId={selectedAppId} refresh={refresh} receipt={showReceipt} intent={functionIntent ?? undefined} />;
    if (page === "library") return <LibraryPage resources={resources} workspaces={workspaces} resourceWorkspaces={resourceWorkspaces} currentWorkspace={currentWorkspace} initialResourceId={selectedResourceId} initialResourceKey={resourceOpenKey} refresh={refresh} receipt={showReceipt} openCapture={setViewedCapture} intent={functionIntent ?? undefined} />;
    if (page === "tasks") return <BoardPage tasks={tasks} projects={projects} refresh={refresh} openTask={setDrawerTask} intent={functionIntent ?? undefined} />;
    return <SettingsPage theme={theme} setTheme={setThemeState} status={status} capturesArchived={archived} capturesTrashed={trashed} projects={projects} tasks={tasks} workspaces={workspaces} apps={apps} resources={resources} trashedResources={trashedResources} refresh={refresh} intent={functionIntent ?? undefined} />;
  // ATENCAO: esta lista e manual e nao ha lint que a verifique. Um estado novo
  // que chegue as paginas por prop e nao entre aqui fica CONGELADO na tela: o
  // clique atualiza o estado, o memo devolve a arvore antiga, e nada acontece.
  //
  // Foi exatamente o que aconteceu com currentWorkspaceId quando o contexto
  // subiu para o raiz — trocar de Workspace parou de funcionar. Os outros tres
  // se salvavam por acidente, porque suas acoes chamam refresh() e o refresh
  // troca a identidade de workspaces/apps/resources, forcando o recalculo.
  // Contexto nao chama refresh, entao travava sozinho e para sempre.
  }, [page, recent, projects, workspaces, apps, resources, trashedResources, tasks, refresh, inbox, selectedProjectId, selectedWorkspaceId, selectedAppId, selectedResourceId, resourceOpenKey, theme, status, archived, trashed, functionIntent, currentWorkspaceId, currentWorkspace, hiddenWidgets, resourceWorkspaces]);
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
<div className="system-state" aria-live="polite">{busy ? <><MosSymbol size={16} spinning /><span className="micro-label">SINCRONIZANDO</span></> : null}<span className="page-meta">{pageMeta}</span></div></header><main className="content" ref={contentRef}>{content}</main></div>{commandOpen ? <CommandSurface closing={commandClosing} close={closeCommand} openCapture={setViewedCapture} openTask={setDrawerTask} openProject={openProject} openWorkspace={openWorkspace} openApp={openRegisteredApp} openResource={openResource} routeFunction={routeFunction} /> : null}{viewedCapture ? <CaptureViewer capture={viewedCapture} close={() => setViewedCapture(null)} /> : null}{drawerTask ? <TaskDrawer key={drawerTask.id} task={drawerTask} projects={projects} close={() => setDrawerTask(null)} refresh={refresh} receipt={showReceipt} openCapture={(capture) => { setDrawerTask(null); setViewedCapture(capture); }} /> : null}{undo ? <div className="receipt" role="status"><span>{undo.message}</span><button onClick={() => void undo.run().then(() => { setUndo(null); return refresh(); })}>DESFAZER · CTRL Z</button></div> : null}</div>;
}

/**
 * As três janelas do M/OS partem do mesmo bundle e se separam pelo rótulo.
 *
 * `main` é o aplicativo; `quick-capture` é a linha de captura global; `lembrete`
 * é a janelinha que aparece sobre o CAD quando o sistema percebe que o trabalho
 * começou sem cronômetro.
 */
export default function App() {
  switch (getCurrentWindow().label) {
    case "quick-capture":
      return <QuickCapture />;
    case "lembrete":
      return <Reminder />;
    default:
      return <DesktopApp />;
  }
}

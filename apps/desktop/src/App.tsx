import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Archive,
  Check,
  ChevronRight,
  DatabaseBackup,
  Home,
  Inbox,
  MoreHorizontal,
  RotateCcw,
  Search,
  Settings,
  Trash2,
  X,
} from "lucide-react";
import { api, appError } from "./api";
import type { AppStatus, BackupInspection, Capture } from "./types";
import "./App.css";

type Page = "home" | "inbox" | "data";
type DataView = "settings" | "archive" | "trash";
type UndoAction = { label: string; run: () => Promise<unknown> };

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

function IconButton({
  label,
  children,
  onClick,
}: {
  label: string;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button className="icon-button" type="button" aria-label={label} title={label} onClick={onClick}>
      {children}
    </button>
  );
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
      setFeedback(`${appError(error).message} Nada foi salvo.`);
    }
  }

  return (
    <form className="capture-composer" onSubmit={submit}>
      <textarea
        aria-label="Conteúdo da captura"
        value={content}
        onChange={(event) => setContent(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            event.currentTarget.form?.requestSubmit();
          }
        }}
        placeholder="What's on your mind?"
        rows={2}
      />
      <div className="composer-footer">
        <span className={`feedback ${state}`} aria-live="polite">{feedback}</span>
        <button className="primary-button" type="submit" disabled={!content.trim() || state === "saving"}>
          {state === "saving" ? "Salvando" : "Capturar"}
        </button>
      </div>
    </form>
  );
}

function HomePage({ recent, onSaved, openInbox }: { recent: Capture[]; onSaved: (capture: Capture) => void; openInbox: () => void }) {
  return (
    <div className="page home-page">
      <header className="page-header"><h1>Home</h1></header>
      <CaptureComposer onSaved={onSaved} />
      <section className="recent-section" aria-labelledby="recent-title">
        <div className="section-header">
          <h2 id="recent-title">Recentes</h2>
          <button className="quiet-button" type="button" onClick={openInbox}>Abrir Inbox <ChevronRight size={16} /></button>
        </div>
        {recent.length ? (
          <ol className="plain-list">
            {recent.map((capture) => (
              <li key={capture.id} className="recent-row">
                <span>{capture.content}</span>
                <time>{relativeTime(capture.capturedAt)}</time>
              </li>
            ))}
          </ol>
        ) : <p className="empty-state">Suas capturas recentes aparecerão aqui.</p>}
      </section>
    </div>
  );
}

function InboxPage({
  captures,
  onMutation,
  setUndo,
}: {
  captures: Capture[];
  onMutation: () => Promise<void>;
  setUndo: (undo: UndoAction) => void;
}) {
  const [selectedId, setSelectedId] = useState<string | null>(captures[0]?.id ?? null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [actionError, setActionError] = useState("");
  const list = useRef<HTMLOListElement>(null);
  const detailAction = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!captures.some((capture) => capture.id === selectedId)) {
      setSelectedId(captures[0]?.id ?? null);
    }
  }, [captures, selectedId]);
  const selected = captures.find((capture) => capture.id === selectedId) ?? null;

  async function mutate(action: "processed" | "archive" | "trash") {
    if (!selected) return;
    const id = selected.id;
    try {
      if (action === "processed") {
        await api.markProcessed(id);
        setUndo({ label: "Capture marcada como processada.", run: () => api.moveToInbox(id) });
      } else if (action === "archive") {
        await api.archive(id);
        setUndo({ label: "Capture arquivada.", run: () => api.restore(id) });
      } else {
        await api.trash(id);
        setUndo({ label: "Capture enviada para a Lixeira.", run: () => api.restore(id) });
      }
      setActionError("");
      setMenuOpen(false);
      await onMutation();
    } catch (error) {
      setActionError(appError(error).message);
    }
  }

  function moveSelection(currentIndex: number, offset: number) {
    const nextIndex = Math.max(0, Math.min(captures.length - 1, currentIndex + offset));
    const next = captures[nextIndex];
    setSelectedId(next.id);
    window.requestAnimationFrame(() => list.current?.querySelector<HTMLButtonElement>(`[data-capture-id="${next.id}"]`)?.focus());
  }

  if (!captures.length) {
    return <div className="page"><header className="page-header"><h1>Inbox</h1></header><p className="empty-state spacious">Nada aguardando decisão.</p></div>;
  }

  return (
    <div className="inbox-page">
      <div className="inbox-list-pane">
        <header className="page-header compact"><h1>Inbox</h1><span className="count-text">{captures.length}</span></header>
        <ol ref={list} className="capture-list" aria-label="Captures na Inbox">
          {captures.map((capture, index) => (
            <li key={capture.id}>
              <button
                className="capture-row"
                data-capture-id={capture.id}
                aria-current={selectedId === capture.id ? "true" : undefined}
                onClick={() => setSelectedId(capture.id)}
                onKeyDown={(event) => {
                  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                    event.preventDefault();
                    moveSelection(index, event.key === "ArrowDown" ? 1 : -1);
                  } else if (event.key === "Enter") {
                    event.preventDefault();
                    setSelectedId(capture.id);
                    window.requestAnimationFrame(() => detailAction.current?.focus());
                  }
                }}
              >
                <span className="capture-content">{capture.content}</span>
                <span className="capture-meta">{relativeTime(capture.capturedAt)} · {sourceLabel(capture.source)}</span>
              </button>
            </li>
          ))}
        </ol>
      </div>
      {selected ? (
        <article className="capture-detail">
          <div className="detail-heading">
            <div><h2>{selected.content}</h2><p>Capturado {relativeTime(selected.capturedAt)}</p></div>
            <div className="menu-wrap">
              <IconButton label="Mais ações" onClick={() => setMenuOpen((open) => !open)}><MoreHorizontal size={19} /></IconButton>
              {menuOpen ? (
                <div className="action-menu" role="menu">
                  <button role="menuitem" onClick={() => void mutate("archive")}><Archive size={16} /> Arquivar</button>
                  <button role="menuitem" onClick={() => void mutate("trash")}><Trash2 size={16} /> Mover para a Lixeira</button>
                </div>
              ) : null}
            </div>
          </div>
          <dl className="detail-metadata"><div><dt>Origem</dt><dd>{sourceLabel(selected.source)}</dd></div></dl>
          {actionError ? <p className="detail-error" role="alert">{actionError}</p> : null}
          <button ref={detailAction} className="primary-button detail-action" type="button" onClick={() => void mutate("processed")} onKeyDown={(event) => { if (event.key === "Escape" && selected) list.current?.querySelector<HTMLButtonElement>(`[data-capture-id="${selected.id}"]`)?.focus(); }}>
            <Check size={17} /> Marcar como processada
          </button>
        </article>
      ) : null}
    </div>
  );
}

function DataPage({
  view,
  setView,
  archived,
  trashed,
  status,
  refresh,
}: {
  view: DataView;
  setView: (view: DataView) => void;
  archived: Capture[];
  trashed: Capture[];
  status: AppStatus | null;
  refresh: () => Promise<void>;
}) {
  const [shortcut, setShortcut] = useState("Ctrl+Shift+Space");
  const [message, setMessage] = useState("");
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [restorePath, setRestorePath] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  const items = view === "archive" ? archived : trashed;

  async function createBackup() {
    const path = await save({ defaultPath: "m-os-backup.mos-backup", filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] });
    if (!path) return;
    try {
      const receipt = await api.createBackup(path);
      setMessage(`Backup criado em ${receipt.path}`);
    } catch (error) { setMessage(appError(error).message); }
  }

  async function chooseRestore() {
    const path = await open({ multiple: false, filters: [{ name: "M/OS Backup", extensions: ["mos-backup"] }] });
    if (!path) return;
    try {
      const nextInspection = await api.inspectBackup(path);
      setInspection(nextInspection);
      setRestorePath(path);
      dialog.current?.showModal();
    } catch (error) { setMessage(appError(error).message); }
  }

  async function confirmRestore() {
    try {
      const safety = await api.restoreBackup(restorePath);
      dialog.current?.close();
      setMessage(`Dados restaurados. Safety backup: ${safety.path}`);
      await refresh();
    } catch (error) { setMessage(appError(error).message); }
  }

  if (view !== "settings") {
    return (
      <div className="page">
        <header className="page-header">
          <div><button className="back-button" onClick={() => setView("settings")}><ChevronRight size={16} /> Dados</button><h1>{view === "archive" ? "Arquivo" : "Lixeira"}</h1></div>
        </header>
        {items.length ? <ol className="plain-list data-list">{items.map((capture) => <li key={capture.id}><span>{capture.content}</span><button className="quiet-button" onClick={() => void api.restore(capture.id).then(refresh)}><RotateCcw size={16} /> Restaurar</button></li>)}</ol> : <p className="empty-state spacious">Nenhuma Capture aqui.</p>}
      </div>
    );
  }

  return (
    <div className="page settings-page">
      <header className="page-header"><h1>Settings</h1></header>
      <section><h2>Captura rápida</h2><form className="setting-row" onSubmit={(event) => { event.preventDefault(); void api.setShortcut(shortcut).then(setMessage).catch((error) => setMessage(appError(error).message)); }}><div><label htmlFor="shortcut">Atalho global</label><p>{status?.shortcut}</p></div><div className="inline-control"><input id="shortcut" value={shortcut} onChange={(event) => setShortcut(event.currentTarget.value)} /><button type="submit">Aplicar</button></div></form></section>
      <section><h2>Dados locais</h2><div className="setting-links"><button onClick={() => setView("archive")}><Archive size={18} /><span>Arquivo<small>{archived.length} Captures</small></span><ChevronRight size={17} /></button><button onClick={() => setView("trash")}><Trash2 size={18} /><span>Lixeira<small>{trashed.length} Captures</small></span><ChevronRight size={17} /></button></div></section>
      <section><h2>Backup e restore</h2><p className="section-copy">Backups podem conter dados pessoais em texto claro.</p><div className="button-row"><button onClick={() => void createBackup()}><DatabaseBackup size={17} /> Criar backup...</button><button className="secondary-button" onClick={() => void chooseRestore()}><RotateCcw size={17} /> Restaurar backup...</button></div></section>
      <section><h2>Integridade</h2><dl className="health-list"><div><dt>Banco</dt><dd>{status?.storage.integrity === "ok" ? "Íntegro" : status?.storage.integrity}</dd></div><div><dt>Schema</dt><dd>v{status?.storage.schemaVersion}</dd></div><div><dt>Durabilidade</dt><dd>{status?.storage.journalMode.toUpperCase()} / {status?.storage.synchronous}</dd></div></dl></section>
      {message ? <p className="settings-message" aria-live="polite">{message}</p> : null}
      <dialog ref={dialog} className="restore-dialog" onCancel={() => dialog.current?.close()}>
        <h2>Substituir o dataset local?</h2>
        <p>O M/OS criará um safety backup antes de restaurar. O restore contém {inspection?.captureCount} Captures e usa schema v{inspection?.schemaVersion}.</p>
        <div className="dialog-actions"><button className="secondary-button" onClick={() => dialog.current?.close()}>Cancelar</button><button className="danger-button" onClick={() => void confirmRestore()}>Restaurar</button></div>
      </dialog>
    </div>
  );
}

function SearchSurface({ close, openCapture }: { close: () => void; openCapture: (capture: Capture) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Capture[]>([]);
  const [includeArchived, setIncludeArchived] = useState(false);
  const [error, setError] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const previousFocus = useRef<HTMLElement | null>(document.activeElement as HTMLElement | null);

  useEffect(() => {
    input.current?.focus();
    return () => previousFocus.current?.focus();
  }, []);
  useEffect(() => {
    if (!query.trim()) { setResults([]); return; }
    const timeout = window.setTimeout(() => void api.search(query, includeArchived).then((items) => { setResults(items); setError(""); }).catch((nextError) => setError(appError(nextError).message)), 80);
    return () => window.clearTimeout(timeout);
  }, [query, includeArchived]);

  return (
    <div className="search-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}>
      <section className="search-surface" role="dialog" aria-modal="true" aria-label="Busca global" onKeyDown={(event) => { if (event.key === "Escape") close(); }}>
        <div className="search-input-wrap"><Search size={19} /><input ref={input} value={query} onChange={(event) => setQuery(event.currentTarget.value)} placeholder="Buscar no M/OS..." aria-label="Buscar no M/OS" /><IconButton label="Fechar busca" onClick={close}><X size={18} /></IconButton></div>
        {query ? <label className="archive-toggle"><input type="checkbox" checked={includeArchived} onChange={(event) => setIncludeArchived(event.currentTarget.checked)} /> Incluir arquivados</label> : null}
        <div className="search-results" aria-live="polite">
          {error ? <div className="search-error"><p>{error}</p><button onClick={() => void api.rebuildSearch().then(() => api.search(query, includeArchived).then(setResults))}>Reconstruir busca</button></div> : null}
          {!error && !query ? <p className="empty-state">Digite para buscar.</p> : null}
          {!error && query && !results.length ? <p className="empty-state">Nenhuma captura encontrada.</p> : null}
          {results.length ? <ol className="search-list">{results.map((capture) => <li key={capture.id}><button onClick={() => openCapture(capture)}><span className="type-label">Capture</span><span>{capture.content}</span><time>{relativeTime(capture.capturedAt)}</time></button></li>)}</ol> : null}
        </div>
      </section>
    </div>
  );
}

function CaptureViewer({ capture, close }: { capture: Capture; close: () => void }) {
  const viewer = useRef<HTMLElement>(null);
  const state = capture.lifecycleState === "archived"
    ? "Arquivada"
    : capture.processingState === "processed"
      ? "Processada"
      : "Na Inbox";

  useEffect(() => viewer.current?.focus(), []);

  return (
    <div className="viewer-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) close(); }}>
      <article ref={viewer} className="capture-viewer" role="dialog" aria-modal="true" aria-labelledby="capture-viewer-title" onKeyDown={(event) => { if (event.key === "Escape") close(); }} tabIndex={-1}>
        <header>
          <span className="type-label">Capture</span>
          <IconButton label="Fechar captura" onClick={close}><X size={18} /></IconButton>
        </header>
        <h2 id="capture-viewer-title">{capture.content}</h2>
        <dl className="detail-metadata">
          <div><dt>Origem</dt><dd>{sourceLabel(capture.source)}</dd></div>
          <div><dt>Estado</dt><dd>{state}</dd></div>
          <div><dt>Capturada</dt><dd>{new Date(capture.capturedAt).toLocaleString("pt-BR")}</dd></div>
        </dl>
      </article>
    </div>
  );
}

function QuickCapture() {
  const [content, setContent] = useState("");
  const [state, setState] = useState<"idle" | "saving" | "error">("idle");
  const [feedback, setFeedback] = useState("Pronto para salvar localmente");
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    input.current?.focus();
    const unlisten = listen("window-revealed", () => input.current?.focus());
    return () => { void unlisten.then((dispose) => dispose()); };
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!content.trim() || state === "saving") return;
    setState("saving"); setFeedback("Salvando localmente...");
    try {
      await api.createCapture(content, "quick_capture");
      setContent(""); setState("idle"); setFeedback("Salvo na Inbox");
      window.setTimeout(() => void api.hideQuickCapture(), 260);
    } catch (error) {
      setState("error"); setFeedback(`${appError(error).message} Nada foi salvo.`);
    }
  }

  return <main className="quick-shell"><form className="quick-form" onSubmit={submit}><textarea ref={input} value={content} onChange={(event) => setContent(event.currentTarget.value)} onKeyDown={(event) => { if (event.key === "Escape") void api.hideQuickCapture(); if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); event.currentTarget.form?.requestSubmit(); } }} aria-label="Texto da captura" placeholder="What's on your mind?" rows={1} /><button type="submit" disabled={!content.trim() || state === "saving"}>{state === "error" ? "Tentar novamente" : state === "saving" ? "Salvando" : "Capturar"}</button></form><p className={`quick-feedback ${state}`} aria-live="polite">{feedback}</p></main>;
}

function DesktopApp() {
  const [page, setPage] = useState<Page>("home");
  const [dataView, setDataView] = useState<DataView>("settings");
  const [recent, setRecent] = useState<Capture[]>([]);
  const [inbox, setInbox] = useState<Capture[]>([]);
  const [archived, setArchived] = useState<Capture[]>([]);
  const [trashed, setTrashed] = useState<Capture[]>([]);
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [viewedCapture, setViewedCapture] = useState<Capture | null>(null);
  const [undo, setUndoState] = useState<UndoAction | null>(null);
  const undoTimer = useRef<number | null>(null);

  const refresh = useCallback(async () => {
    const [nextRecent, nextInbox, nextArchived, nextTrashed, nextStatus] = await Promise.all([api.recent(), api.inbox(), api.archived(), api.trashed(), api.status()]);
    setRecent(nextRecent); setInbox(nextInbox); setArchived(nextArchived); setTrashed(nextTrashed); setStatus(nextStatus);
  }, []);

  useEffect(() => { void refresh(); const changed = listen("capture-changed", () => void refresh()); const restored = listen("dataset-restored", () => void refresh()); return () => { void changed.then((dispose) => dispose()); void restored.then((dispose) => dispose()); }; }, [refresh]);
  useEffect(() => { const handler = (event: KeyboardEvent) => { if (event.ctrlKey && event.key.toLowerCase() === "k") { event.preventDefault(); setSearchOpen(true); } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, []);

  function setUndo(next: UndoAction) {
    setUndoState(next);
    if (undoTimer.current) window.clearTimeout(undoTimer.current);
    undoTimer.current = window.setTimeout(() => setUndoState(null), 6_000);
  }

  const content = useMemo(() => {
    if (page === "home") return <HomePage recent={recent} onSaved={() => void refresh()} openInbox={() => setPage("inbox")} />;
    if (page === "inbox") return <InboxPage captures={inbox} onMutation={refresh} setUndo={setUndo} />;
    return <DataPage view={dataView} setView={setDataView} archived={archived} trashed={trashed} status={status} refresh={refresh} />;
  }, [page, recent, inbox, archived, trashed, status, dataView, refresh]);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="wordmark">M/OS</div>
        <nav aria-label="Navegação principal">
          <button aria-current={page === "home"} onClick={() => setPage("home")}><Home size={18} /> Home</button>
          <button aria-current={page === "inbox"} onClick={() => setPage("inbox")}><Inbox size={18} /> Inbox {inbox.length ? <span>{inbox.length}</span> : null}</button>
        </nav>
        <button className="settings-nav" aria-current={page === "data"} onClick={() => { setPage("data"); setDataView("settings"); }}><Settings size={18} /> Settings</button>
      </aside>
      <div className="main-column">
        <header className="topbar"><button className="search-command" onClick={() => setSearchOpen(true)}><Search size={17} /><span>Buscar</span><kbd>Ctrl K</kbd></button><button className="primary-button compact-button" onClick={() => void api.showQuickCapture()}>Quick Capture</button></header>
        <main className="content">{content}</main>
      </div>
      {searchOpen ? <SearchSurface close={() => setSearchOpen(false)} openCapture={(capture) => { setSearchOpen(false); setViewedCapture(capture); }} /> : null}
      {viewedCapture ? <CaptureViewer capture={viewedCapture} close={() => setViewedCapture(null)} /> : null}
      {undo ? <div className="undo-toast" role="status"><span>{undo.label}</span><button onClick={() => void undo.run().then(() => { setUndoState(null); return refresh(); })}>Desfazer</button></div> : null}
    </div>
  );
}

export default function App() {
  return getCurrentWindow().label === "quick-capture" ? <QuickCapture /> : <DesktopApp />;
}

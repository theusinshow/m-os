import { FormEvent, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type Capture = {
  id: number;
  content: string;
  createdAt: string;
};

type Receipt = {
  id: number;
  committedInMs: number;
};

type SpikeStatus = {
  shell: string;
  shortcut: string;
  storage: {
    databasePath: string;
    journalMode: string;
    synchronous: string;
    sqliteVersion: string;
  };
};

function QuickCapture() {
  const [content, setContent] = useState("");
  const [feedback, setFeedback] = useState("Pronto para capturar localmente");
  const [saving, setSaving] = useState(false);
  const input = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    input.current?.focus();
    const unlisten = listen("capture-focus", () => input.current?.focus());
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!content.trim() || saving) return;

    setSaving(true);
    try {
      const receipt = await invoke<Receipt>("save_capture", { content });
      setFeedback(`Salva localmente em ${receipt.committedInMs} ms`);
      setContent("");
      window.setTimeout(() => void invoke("hide_quick_capture"), 260);
    } catch (error) {
      setFeedback(String(error));
    } finally {
      setSaving(false);
    }
  }

  return (
    <main className="quick-shell">
      <form className="quick-form" onSubmit={submit}>
        <textarea
          ref={input}
          aria-label="Texto da captura"
          value={content}
          onChange={(event) => setContent(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") void invoke("hide_quick_capture");
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              event.currentTarget.form?.requestSubmit();
            }
          }}
          placeholder="What's on your mind?"
          rows={1}
        />
        <button type="submit" disabled={!content.trim() || saving}>
          {saving ? "Salvando" : "Capturar"}
        </button>
      </form>
      <p className="quick-feedback" aria-live="polite">{feedback}</p>
    </main>
  );
}

function DiagnosticShell() {
  const [status, setStatus] = useState<SpikeStatus | null>(null);
  const [captures, setCaptures] = useState<Capture[]>([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [shortcut, setShortcut] = useState("Ctrl+Shift+Space");

  async function refresh(search = query) {
    try {
      const [nextStatus, nextCaptures] = await Promise.all([
        invoke<SpikeStatus>("get_spike_status"),
        invoke<Capture[]>("list_captures", { query: search || null }),
      ]);
      setStatus(nextStatus);
      setCaptures(nextCaptures);
      setError("");
    } catch (nextError) {
      setError(String(nextError));
    }
  }

  useEffect(() => {
    void refresh("");
    const unlisten = listen("capture-saved", () => void refresh(""));
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  async function updateShortcut(event: FormEvent) {
    event.preventDefault();
    try {
      await invoke("set_capture_shortcut", { shortcut });
      await refresh();
    } catch (nextError) {
      setError(String(nextError));
      await refresh();
    }
  }

  return (
    <main className="diagnostic-shell">
      <header className="shell-header">
        <div>
          <p className="eyebrow">Disposable technical spike</p>
          <h1>M/OS desktop shell</h1>
        </div>
        <button type="button" onClick={() => void invoke("show_quick_capture")}>
          Testar captura
        </button>
      </header>

      <section aria-labelledby="runtime-heading">
        <h2 id="runtime-heading">Runtime</h2>
        <dl className="status-list">
          <div><dt>Shell</dt><dd>{status?.shell ?? "Carregando"}</dd></div>
          <div>
            <dt>Atalho global</dt>
            <dd>
              <form className="shortcut-form" onSubmit={updateShortcut}>
                <input
                  aria-label="Atalho global"
                  value={shortcut}
                  onChange={(event) => setShortcut(event.currentTarget.value)}
                />
                <button type="submit">Aplicar</button>
              </form>
              <small>{status?.shortcut ?? "Carregando"}</small>
            </dd>
          </div>
          <div><dt>SQLite</dt><dd>{status?.storage.sqliteVersion ?? "Carregando"}</dd></div>
          <div><dt>Durabilidade</dt><dd>{status ? `${status.storage.journalMode.toUpperCase()} / ${status.storage.synchronous}` : "Carregando"}</dd></div>
          <div><dt>Banco local</dt><dd className="path-value">{status?.storage.databasePath ?? "Carregando"}</dd></div>
        </dl>
      </section>

      <section aria-labelledby="captures-heading">
        <div className="section-heading">
          <div>
            <h2 id="captures-heading">Capturas persistidas</h2>
            <p>Consulta local pelo indice FTS5.</p>
          </div>
          <form
            className="search-form"
            onSubmit={(event) => {
              event.preventDefault();
              void refresh();
            }}
          >
            <input
              aria-label="Buscar capturas"
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="Buscar no indice local"
            />
            <button type="submit">Buscar</button>
          </form>
        </div>

        {error ? <p className="error" role="alert">{error}</p> : null}
        {captures.length ? (
          <ol className="capture-list">
            {captures.map((capture) => (
              <li key={capture.id}>
                <span>{capture.content}</span>
                <time>{capture.createdAt}</time>
              </li>
            ))}
          </ol>
        ) : (
          <p className="empty-state">Nenhuma captura encontrada.</p>
        )}
      </section>
    </main>
  );
}

function App() {
  return getCurrentWindow().label === "quick-capture" ? <QuickCapture /> : <DiagnosticShell />;
}

export default App;

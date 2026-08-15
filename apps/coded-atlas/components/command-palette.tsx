"use client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import type { ProjectSummary } from "@/lib/storage/list-projects";

type Item = {
  id: string;
  label: string;
  sub?: string;
  hint: string;
  run: () => void;
};

const ACTIONS = [
  { id: "go-generate", label: "Gerar catálogo", href: "/generate", hint: "Ação" },
  { id: "go-projects", label: "Biblioteca de projetos", href: "/projects", hint: "Ação" },
  { id: "go-home", label: "Início", href: "/", hint: "Ação" },
  { id: "go-lab", label: "Laboratório", href: "/lab/coded-atlas", hint: "Ação" },
];

function matches(q: string, ...fields: (string | undefined)[]) {
  if (!q) return true;
  const needle = q.toLowerCase();
  return fields.some((f) => f?.toLowerCase().includes(needle));
}

export function CommandPalette() {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [projects, setProjects] = useState<ProjectSummary[] | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
    setActive(0);
  }, []);

  // Atalho global Cmd/Ctrl+K + gatilho via evento (botão na nav)
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((o) => !o);
      }
    }
    function onOpen() {
      setOpen(true);
    }
    document.addEventListener("keydown", onKey);
    window.addEventListener("atlas:open-command", onOpen);
    return () => {
      document.removeEventListener("keydown", onKey);
      window.removeEventListener("atlas:open-command", onOpen);
    };
  }, []);

  // Carrega projetos na primeira abertura; trava scroll; foca o input
  useEffect(() => {
    if (!open) return;
    if (projects === null) {
      fetch("/api/projects")
        .then((r) => r.json())
        .then((d: { projects: ProjectSummary[] }) => setProjects(d.projects))
        .catch(() => setProjects([]));
    }
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const id = requestAnimationFrame(() => inputRef.current?.focus());
    return () => {
      document.body.style.overflow = prev;
      cancelAnimationFrame(id);
    };
  }, [open, projects]);

  const items: Item[] = useMemo(() => {
    const acts: Item[] = ACTIONS.filter((a) => matches(query, a.label)).map((a) => ({
      id: a.id,
      label: a.label,
      hint: a.hint,
      run: () => {
        router.push(a.href);
        close();
      },
    }));

    const projs: Item[] = (projects ?? [])
      .filter((p) => matches(query, p.name, p.category, p.url, p.client, p.slug))
      .slice(0, 8)
      .map((p) => ({
        id: `proj-${p.slug}`,
        label: p.name,
        sub: `${p.category} · ${p.slug}`,
        hint: "Projeto",
        run: () => {
          router.push(`/projects/${p.slug}`);
          close();
        },
      }));

    return [...acts, ...projs];
  }, [query, projects, router, close]);

  useEffect(() => setActive(0), [query]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => Math.min(i + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      items[active]?.run();
    }
  }

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[70] bg-base/70 backdrop-blur-sm flex items-start justify-center pt-[12vh] px-4"
      onClick={close}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Paleta de comando"
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-xl bg-surface border border-line-soft shadow-2xl"
      >
        <div className="flex items-center gap-3 px-4 border-b border-line">
          <span className="text-zinc-500 text-sm" aria-hidden>⌕</span>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Buscar projeto ou ação..."
            className="flex-1 bg-transparent py-3.5 text-sm text-zinc-100 placeholder:text-zinc-500 focus:outline-none"
          />
          <kbd className="text-[10px] font-mono text-zinc-500 border border-line px-1.5 py-0.5">esc</kbd>
        </div>

        <ul className="max-h-80 overflow-y-auto py-1">
          {items.length === 0 && (
            <li className="px-4 py-6 text-center text-[13px] text-zinc-500">
              {projects === null ? "Carregando..." : "Nada encontrado."}
            </li>
          )}
          {items.map((item, i) => {
            const sel = i === active;
            return (
              <li key={item.id}>
                <button
                  onMouseEnter={() => setActive(i)}
                  onClick={item.run}
                  className={[
                    "w-full flex items-center justify-between gap-3 px-4 py-2.5 text-left transition-colors cursor-pointer",
                    sel ? "bg-surface-2" : "",
                  ].join(" ")}
                >
                  <span className="min-w-0">
                    <span className={["text-sm", sel ? "text-zinc-50" : "text-zinc-200"].join(" ")}>
                      {item.label}
                    </span>
                    {item.sub && (
                      <span className="block text-[11px] font-mono text-zinc-500 truncate">{item.sub}</span>
                    )}
                  </span>
                  <span className={["text-[10px] font-mono uppercase tracking-wider shrink-0", sel ? "text-accent" : "text-zinc-600"].join(" ")}>
                    {item.hint}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}

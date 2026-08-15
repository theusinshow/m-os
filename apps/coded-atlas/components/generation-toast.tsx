"use client";
import { useState, useEffect, useCallback } from "react";
import Link from "next/link";
import {
  GEN_STATUS_EVENT,
  GEN_STATUS_KEY,
  emitGenStatus,
  type GenStatus,
} from "@/lib/gen-status";

function useElapsed(startedAt: number | undefined): string {
  const [sec, setSec] = useState(0);

  useEffect(() => {
    if (!startedAt) return;
    setSec(Math.floor((Date.now() - startedAt) / 1000));
    const id = setInterval(
      () => setSec(Math.floor((Date.now() - startedAt) / 1000)),
      5000
    );
    return () => clearInterval(id);
  }, [startedAt]);

  if (sec < 60) return `${sec}s`;
  return `${Math.floor(sec / 60)}min`;
}

export function GenerationToast() {
  const [status, setStatus] = useState<GenStatus | null>(null);
  const [dismissed, setDismissed] = useState<string>(""); // slug+state key

  useEffect(() => {
    const raw = localStorage.getItem(GEN_STATUS_KEY);
    if (raw) {
      try { setStatus(JSON.parse(raw) as GenStatus); } catch {}
    }

    function handler(e: Event) {
      const evt = e as CustomEvent<GenStatus | null>;
      setStatus(evt.detail);
      setDismissed("");
    }

    window.addEventListener(GEN_STATUS_EVENT, handler);
    return () => window.removeEventListener(GEN_STATUS_EVENT, handler);
  }, []);

  // Auto-dismiss success after 10s
  useEffect(() => {
    if (status?.state === "done") {
      const id = setTimeout(() => dismiss(), 10_000);
      return () => clearTimeout(id);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status?.slug, status?.state]);

  const dismiss = useCallback(() => {
    if (!status) return;
    setDismissed(`${status.slug}:${status.state}`);
    if (status.state !== "generating") emitGenStatus(null);
  }, [status]);

  const elapsed = useElapsed(status?.startedAt);

  const key = status ? `${status.slug}:${status.state}` : "";
  if (!status || dismissed === key) return null;

  const isStale =
    status.state === "generating" &&
    Date.now() - status.startedAt > 8 * 60 * 1000;

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed bottom-5 right-5 z-50 w-80 border border-line bg-surface shadow-2xl"
    >
      {/* status bar top */}
      <div
        className={`h-1 w-full ${
          status.state === "generating"
            ? isStale
              ? "bg-warn"
              : "bg-accent animate-atlas-pulse"
            : status.state === "done"
            ? "bg-ok"
            : "bg-bad"
        }`}
      />

      <div className="px-4 py-3 space-y-2">
        {/* header row */}
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            {status.state === "generating" && !isStale && (
              <span className="shrink-0 w-2 h-2 rounded-full bg-accent animate-atlas-pulse mt-0.5" />
            )}
            {status.state === "generating" && isStale && (
              <span className="shrink-0 w-2 h-2 rounded-full bg-warn mt-0.5" />
            )}
            {status.state === "done" && (
              <span className="shrink-0 text-ok text-sm leading-none">✓</span>
            )}
            {status.state === "error" && (
              <span className="shrink-0 text-bad text-sm leading-none">✗</span>
            )}

            <p className="text-[13px] font-medium text-zinc-100 truncate">
              {status.state === "generating" && !isStale && "Gerando catálogo..."}
              {status.state === "generating" && isStale && "Geração interrompida?"}
              {status.state === "done" && "Catálogo gerado"}
              {status.state === "error" && "Geração falhou"}
            </p>
          </div>

          {status.state !== "generating" && (
            <button
              onClick={dismiss}
              aria-label="Fechar"
              className="shrink-0 text-zinc-400 hover:text-zinc-100 transition-colors text-sm leading-none cursor-pointer"
            >
              ×
            </button>
          )}
        </div>

        {/* project name */}
        <p className="text-[12px] font-mono text-zinc-400 truncate pl-4">
          {status.name} <span className="text-zinc-600">/ {status.slug}</span>
        </p>

        {/* footer row */}
        <div className="pl-4 flex items-center justify-between gap-2">
          {status.state === "generating" && (
            <>
              <span className="text-[11px] font-mono text-zinc-500">
                {isStale ? "Pode ter sido interrompido" : `há ${elapsed}`}
              </span>
              <Link
                href="/projects"
                className="text-[11px] font-medium text-zinc-300 hover:text-zinc-100 transition-colors"
              >
                projetos →
              </Link>
            </>
          )}

          {status.state === "done" && status.projectUrl && (
            <>
              <span className="text-[11px] font-mono text-zinc-500">gerado em {elapsed}</span>
              <Link
                href={status.projectUrl}
                className="text-[11px] font-semibold text-accent hover:text-accent-bright transition-colors"
              >
                ver catálogo →
              </Link>
            </>
          )}

          {status.state === "error" && (
            <span className="text-[11px] text-zinc-400 leading-relaxed">
              {status.errorMessage ?? "Erro desconhecido"}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

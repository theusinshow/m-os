"use client";
import { useState } from "react";
import { ZoomImage } from "@/components/zoom-image";

interface DiffResult {
  percent: number;
  changedPixels: number;
  totalPixels: number;
  url: string;
  before: string;
  after: string;
  diff: string;
  capturedAt: string;
}

interface Props {
  slug: string;
  beforeImage: string;
}

export function VisualDiff({ slug, beforeImage }: Props) {
  const [state, setState] = useState<"idle" | "loading" | "done" | "error">("idle");
  const [result, setResult] = useState<DiffResult | null>(null);
  const [error, setError] = useState("");

  async function run() {
    setState("loading");
    setError("");
    try {
      const res = await fetch(`/api/diff/${slug}`, { method: "POST" });
      const data = (await res.json()) as DiffResult & { error?: string };
      if (!res.ok) throw new Error(data.error ?? "Falha ao verificar.");
      // cache-busting nas imagens recém-escritas
      const bust = `?t=${Date.now()}`;
      setResult({ ...data, after: data.after + bust, diff: data.diff + bust });
      setState("done");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao verificar.");
      setState("error");
    }
  }

  const unchanged = result !== null && result.percent < 0.1;

  return (
    <section aria-label="Monitoramento">
      <div className="flex items-baseline justify-between mb-4">
        <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">Monitoramento</p>
        <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">diff visual</span>
      </div>

      <div className="border border-line bg-surface/30 p-5 space-y-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="text-[13px] text-zinc-300 leading-relaxed max-w-prose">
            Recaptura o site agora e compara com a captura do catálogo, destacando o que mudou.
            Bom para vigiar sites já entregues.
          </p>
          <button
            onClick={run}
            disabled={state === "loading"}
            className="shrink-0 px-4 py-2 border border-line text-zinc-200 text-[13px] font-medium hover:border-accent hover:text-accent-bright transition-colors disabled:opacity-60 cursor-pointer"
          >
            {state === "loading" ? "Verificando..." : "Verificar mudanças"}
          </button>
        </div>

        {state === "error" && <p className="text-bad text-[13px]">{error}</p>}

        {state === "done" && result && (
          <div className="space-y-4">
            <div className="flex items-baseline gap-3">
              <span
                className={[
                  "text-2xl font-semibold tabular-nums",
                  unchanged ? "text-ok" : "text-warn",
                ].join(" ")}
              >
                {result.percent}%
              </span>
              <span className="text-[13px] text-zinc-400">
                {unchanged
                  ? "praticamente sem mudanças"
                  : `da página mudou (${result.changedPixels.toLocaleString("pt-BR")} pixels)`}
              </span>
            </div>

            <div className="grid sm:grid-cols-3 gap-3">
              {[
                { label: "Antes (catálogo)", src: beforeImage },
                { label: "Agora", src: result.after },
                { label: "Diferença", src: result.diff },
              ].map((col) => (
                <div key={col.label} className="space-y-1.5">
                  <p className="text-[10px] font-mono text-zinc-400 uppercase tracking-wider">{col.label}</p>
                  <div className="border border-line bg-surface overflow-hidden">
                    <ZoomImage src={col.src} alt={col.label} className="w-full block" />
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

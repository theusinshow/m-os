"use client";
import { useState } from "react";

interface Props {
  slug: string;
  existingPath?: string;
}

export function CaseDraftSection({ slug, existingPath }: Props) {
  const [state, setState] = useState<"idle" | "generating" | "done" | "error">(
    existingPath ? "done" : "idle"
  );
  const [casePath, setCasePath] = useState<string | null>(existingPath ?? null);
  const [errorMsg, setErrorMsg] = useState("");

  async function handleGenerate() {
    setState("generating");
    setErrorMsg("");
    try {
      const res = await fetch("/api/case", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ slug }),
      });

      const data = (await res.json()) as { path?: string; error?: string };

      if (!res.ok) {
        throw new Error(data.error ?? "Erro desconhecido");
      }

      setCasePath(data.path!);
      setState("done");
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : "Erro desconhecido");
      setState("error");
    }
  }

  return (
    <section aria-label="Rascunho de case">
      <h2 className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-4">
        Rascunho de Case
      </h2>

      {state === "idle" && (
        <div className="border border-line p-5 space-y-4">
          <p className="text-sm text-zinc-300 leading-relaxed">
            Gera um{" "}
            <code className="text-zinc-200 font-mono text-xs bg-surface px-1 py-0.5">
              case-draft.mdx
            </code>{" "}
            com todas as capturas, inspeção e metadados organizados em estrutura de case.
          </p>
          <button
            onClick={handleGenerate}
            className="px-5 py-2.5 border border-line text-zinc-200 text-sm hover:border-line-soft hover:text-zinc-50 transition-colors cursor-pointer"
          >
            Gerar rascunho →
          </button>
        </div>
      )}

      {state === "generating" && (
        <div className="border border-line p-5">
          <p className="text-[11px] font-mono text-accent uppercase tracking-widest animate-atlas-pulse">
            Gerando case-draft.mdx...
          </p>
        </div>
      )}

      {state === "done" && casePath && (
        <div className="border border-line bg-surface/40 p-5 space-y-4">
          <div className="flex items-start gap-3">
            <span className="text-ok text-lg leading-none shrink-0">✓</span>
            <div>
              <p className="text-zinc-50 text-sm font-semibold">Rascunho gerado.</p>
              <p className="text-[11px] font-mono text-zinc-500 mt-0.5 break-all">{casePath}</p>
            </div>
          </div>
          <div className="flex gap-3">
            <a
              href={casePath}
              download
              className="px-5 py-2.5 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors cursor-pointer"
            >
              Baixar .mdx →
            </a>
            <button
              onClick={handleGenerate}
              className="px-5 py-2.5 border border-line text-zinc-300 text-sm hover:border-line-soft hover:text-zinc-100 transition-colors cursor-pointer"
            >
              Regenerar
            </button>
          </div>
        </div>
      )}

      {state === "error" && (
        <div className="border border-line bg-surface/40 p-5 space-y-4">
          <div className="flex items-start gap-3">
            <span className="text-bad text-lg leading-none shrink-0">✗</span>
            <p className="text-zinc-200 text-sm">
              {errorMsg || "Não foi possível gerar o rascunho."}
            </p>
          </div>
          <button
            onClick={handleGenerate}
            className="px-5 py-2.5 border border-line text-zinc-300 text-sm hover:border-line-soft hover:text-zinc-100 transition-colors cursor-pointer"
          >
            Tentar novamente
          </button>
        </div>
      )}
    </section>
  );
}

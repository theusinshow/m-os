"use client";
import { useState } from "react";
import type { PortfolioManifest } from "@/lib/types";

interface Props {
  manifest: PortfolioManifest;
}

export function PortfolioExport({ manifest }: Props) {
  const [copied, setCopied] = useState(false);
  const json = JSON.stringify(manifest, null, 2);

  async function copy() {
    try {
      await navigator.clipboard.writeText(json);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      /* clipboard indisponível — o usuário ainda pode baixar */
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-baseline justify-between gap-4">
        <p className="text-[10px] font-mono text-zinc-500 uppercase tracking-widest">
          Exportar para o portfólio
        </p>
        <span className="text-[10px] font-mono text-accent uppercase tracking-wider">
          → /cases/{manifest.slug}
        </span>
      </div>

      <p className="text-zinc-500 text-xs leading-relaxed max-w-prose">
        Fragmento que o portfólio Coded by M lê para montar o case e a Paisagem
        Digital. Baixe o <span className="font-mono text-zinc-400">portfolio.json</span>{" "}
        (ou copie o conteúdo) e versione junto do{" "}
        <span className="font-mono text-zinc-400">case-draft.mdx</span> no repositório do portfólio.
      </p>

      {/* Resumo dos campos-chave */}
      <div className="grid sm:grid-cols-3 gap-6">
        {/* Acento + paleta */}
        <div className="space-y-2">
          <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">Cor / acento</p>
          {manifest.accent ? (
            <div className="flex items-center gap-2">
              <span
                className="w-6 h-6 border border-line-soft shrink-0"
                style={{ backgroundColor: manifest.accent }}
                title={manifest.accent}
              />
              <span className="text-[11px] font-mono text-zinc-400">{manifest.accent}</span>
            </div>
          ) : (
            <p className="text-[11px] font-mono text-zinc-500">não detectada</p>
          )}
          {manifest.palette.length > 1 && (
            <div className="flex flex-wrap gap-1 pt-1">
              {manifest.palette.map((c) => (
                <span
                  key={c}
                  className="w-4 h-4 border border-line shrink-0"
                  style={{ backgroundColor: c }}
                  title={c}
                />
              ))}
            </div>
          )}
        </div>

        {/* Categoria */}
        <div className="space-y-2">
          <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">Categoria</p>
          <p className="text-sm text-zinc-300">{manifest.category}</p>
        </div>

        {/* Sinais */}
        <div className="space-y-2">
          <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">Sinais</p>
          <div className="flex flex-wrap gap-1.5">
            <span className="text-[11px] font-mono text-zinc-300 border border-line px-1.5 py-0.5">
              {manifest.hasVideo ? "com vídeo" : "sem vídeo"}
            </span>
            {manifest.cover && (
              <span className="text-[11px] font-mono text-zinc-300 border border-line px-1.5 py-0.5">
                capa
              </span>
            )}
            {manifest.techStack.slice(0, 3).map((t) => (
              <span key={t} className="text-[11px] font-mono text-zinc-300 border border-line px-1.5 py-0.5">
                {t}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* JSON */}
      <div className="border border-line bg-surface/60">
        <div className="flex items-center justify-between border-b border-line px-3 py-2">
          <span className="text-[11px] font-mono text-zinc-400">portfolio.json</span>
          <div className="flex items-center gap-3">
            <button
              onClick={copy}
              className="text-[10px] font-mono uppercase tracking-wider text-zinc-500 hover:text-zinc-200 transition-colors cursor-pointer"
            >
              {copied ? "copiado ✓" : "copiar"}
            </button>
            <a
              href={`/api/export/${manifest.slug}?download=1`}
              className="text-[10px] font-mono uppercase tracking-wider text-zinc-500 hover:text-zinc-200 transition-colors cursor-pointer"
            >
              baixar ↓
            </a>
          </div>
        </div>
        <pre className="text-[11px] font-mono text-zinc-400 leading-[1.6] p-4 overflow-x-auto">
          {json}
        </pre>
      </div>
    </div>
  );
}

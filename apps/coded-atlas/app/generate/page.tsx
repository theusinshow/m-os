"use client";
import { useState, useRef, useEffect } from "react";
import type {
  Catalog,
  ProjectInput,
  ProgressEvent,
  ResultEvent,
  AtlasErrorPayload,
  AtlasErrorCode,
} from "@/lib/types";
import { emitGenStatus } from "@/lib/gen-status";
import { UrlInput } from "@/components/url-input";
import { GenerationStatus } from "@/components/generation-status";

type GenState = "idle" | "generating" | "done" | "error";

/** Orientação prática por código de erro (o quê fazer a seguir). */
const ERROR_HELP: Partial<Record<AtlasErrorCode, string>> = {
  INVALID_URL: "Confira se a URL começa com https:// e aponta para uma página real.",
  UNREACHABLE: "Confirme se o site está no ar e acessível publicamente (sem VPN ou login).",
  NAV_TIMEOUT: "O site pode estar lento. Aguarde alguns instantes e tente de novo.",
  BLOCKED: "Alguns sites bloqueiam captura automática. Tente outra URL ou rode em ambiente local.",
  RENDER_TIMEOUT: "A página não estabilizou a tempo. Tente novamente.",
  CAPTURE_FAILED: "Falha durante a captura. Geralmente passa ao tentar de novo.",
  STORAGE_FAILED: "Não foi possível salvar os arquivos. Verifique a pasta public/generated.",
  SLUG_CONFLICT: "Já existe um projeto com esse slug. Escolha outro slug.",
  SERVER_DOWN: "O servidor de desenvolvimento não respondeu. Reinicie e tente novamente.",
  UNKNOWN: "Erro inesperado. Veja o terminal do servidor para detalhes.",
};

export default function GeneratePage() {
  const [genState, setGenState] = useState<GenState>("idle");
  const [events, setEvents] = useState<ProgressEvent[]>([]);
  const [result, setResult] = useState<ResultEvent | null>(null);
  const [error, setError] = useState<AtlasErrorPayload | null>(null);
  const [reprocessName, setReprocessName] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const reprocessRef = useRef(false);
  const lastInputRef = useRef<ProjectInput | null>(null);

  async function generate(input: ProjectInput) {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    lastInputRef.current = input;

    const startedAt = Date.now();

    setGenState("generating");
    setEvents([]);
    setResult(null);
    setError(null);

    emitGenStatus({ state: "generating", slug: input.slug, name: input.name, startedAt });

    try {
      const res = await fetch("/api/generate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
        signal: ctrl.signal,
      });

      const reader = res.body!.getReader();
      const decoder = new TextDecoder();
      let buf = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buf += decoder.decode(value, { stream: true });
        const parts = buf.split("\n\n");
        buf = parts.pop() ?? "";
        for (const part of parts) {
          if (!part.startsWith("data: ")) continue;
          const ev = JSON.parse(part.slice(6)) as
            | ProgressEvent
            | ResultEvent
            | AtlasErrorPayload;

          if (ev.step === "done") {
            const r = ev as ResultEvent;
            setResult(r);
            setGenState("done");
            emitGenStatus({
              state: "done",
              slug: input.slug,
              name: input.name,
              projectUrl: r.projectUrl,
              startedAt,
            });
          } else if (ev.step === "error") {
            const e = ev as AtlasErrorPayload;
            setError(e);
            setGenState("error");
            emitGenStatus({
              state: "error",
              slug: input.slug,
              name: input.name,
              errorMessage: e.message,
              startedAt,
            });
          } else {
            setEvents((prev) => [...prev, ev as ProgressEvent]);
          }
        }
      }
    } catch (err: unknown) {
      if (err instanceof Error && err.name === "AbortError") return;
      const isServerDown = err instanceof TypeError;
      const msg = isServerDown
        ? "O servidor de desenvolvimento não respondeu."
        : "Falha de conexão inesperada.";
      setError({ step: "error", code: isServerDown ? "SERVER_DOWN" : "UNKNOWN", message: msg });
      setGenState("error");
      emitGenStatus({ state: "error", slug: input.slug, name: input.name, errorMessage: msg, startedAt });
    }
  }

  function retry() {
    if (lastInputRef.current) generate(lastInputRef.current);
  }

  function cancel() {
    abortRef.current?.abort();
    emitGenStatus(null);
    reset();
  }

  function reset() {
    abortRef.current?.abort();
    setGenState("idle");
    setEvents([]);
    setResult(null);
    setError(null);
    setReprocessName(null);
    if (typeof window !== "undefined" && window.location.search) {
      window.history.replaceState(null, "", window.location.pathname);
    }
  }

  // Reprocessamento: ?reprocess=<slug> relê o input salvo e regera sem o form.
  useEffect(() => {
    if (reprocessRef.current) return;
    const slug = new URLSearchParams(window.location.search).get("reprocess");
    if (!slug) return;
    reprocessRef.current = true;

    (async () => {
      try {
        const res = await fetch(`/generated/${slug}/catalog.json`, { cache: "no-store" });
        if (!res.ok) throw new Error(`catalog ${res.status}`);
        const catalog = (await res.json()) as Catalog;
        setReprocessName(catalog.project.name);
        generate(catalog.project);
      } catch {
        setError({
          step: "error",
          code: "STORAGE_FAILED",
          message: "Não foi possível carregar este projeto para reprocessar. Verifique se ele ainda existe.",
        });
        setGenState("error");
      }
    })();
  }, []);

  return (
    <main className="max-w-xl mx-auto px-6 py-12">
      {/* Header */}
      <header className="mb-9">
        <h1 className="text-2xl font-semibold text-zinc-50 tracking-tight">
          {reprocessName ? "Reprocessar catálogo" : "Gerar catálogo"}
        </h1>
        <p className="text-zinc-400 text-sm mt-1.5 leading-relaxed">
          {reprocessName ? (
            <>
              Regerando as capturas de{" "}
              <span className="text-zinc-200 font-medium">{reprocessName}</span>. Os arquivos atuais
              são substituídos; o rascunho de case é preservado.
            </>
          ) : (
            "Cole a URL, confira os dados e gere o pacote visual completo."
          )}
        </p>
      </header>

      {/* Idle: formulário */}
      {genState === "idle" && <UrlInput onSubmit={generate} />}

      {/* Generating */}
      {genState === "generating" && (
        <div className="space-y-8">
          <GenerationStatus events={events} />
          <div className="border-t border-line pt-5 flex items-center justify-between gap-4">
            <p className="text-[13px] text-zinc-400">
              Pode fechar esta aba: o status fica salvo no canto da tela.
            </p>
            <button
              onClick={cancel}
              className="shrink-0 px-4 py-2 border border-line text-zinc-300 text-[13px] font-medium hover:border-bad/50 hover:text-bad transition-colors cursor-pointer"
            >
              Cancelar
            </button>
          </div>
        </div>
      )}

      {/* Done */}
      {genState === "done" && result && (
        <div className="space-y-8">
          <GenerationStatus events={events} done />

          <div className="border border-line bg-surface/40 p-5 space-y-5">
            <div className="flex items-start gap-3">
              <span className="text-ok shrink-0 mt-0.5 text-lg leading-none" aria-hidden>✓</span>
              <div>
                <p className="text-zinc-50 text-sm font-semibold">Catálogo gerado com sucesso.</p>
                <p className="text-zinc-400 text-[13px] font-mono mt-1">
                  {result.catalog.project.name}{" "}
                  <span className="text-zinc-600">/ {result.catalog.project.slug}</span>
                </p>
              </div>
            </div>

            <div className="flex flex-wrap gap-3">
              <a
                href={result.projectUrl}
                className="px-5 py-2.5 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
              >
                Ver catálogo →
              </a>
              <button
                onClick={reset}
                className="px-5 py-2.5 border border-line text-zinc-300 text-sm hover:border-line-soft hover:text-zinc-100 transition-colors"
              >
                Gerar outro
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Error */}
      {genState === "error" && error && (
        <div className="space-y-5">
          <div className="border border-bad/40 bg-bad/[0.06] p-5">
            <div className="flex items-start gap-3">
              <span className="text-bad shrink-0 mt-0.5 text-lg leading-none" aria-hidden>✗</span>
              <div className="min-w-0">
                <p className="text-zinc-50 text-sm font-semibold">Não foi possível gerar o catálogo.</p>
                <p className="text-zinc-300 text-sm mt-1">{error.message}</p>
                {error.code && ERROR_HELP[error.code] && (
                  <p className="text-zinc-400 text-[13px] mt-2 leading-relaxed">
                    {ERROR_HELP[error.code]}
                  </p>
                )}
                {error.code && (
                  <p className="text-zinc-600 text-[11px] font-mono mt-3">código: {error.code}</p>
                )}
              </div>
            </div>
          </div>

          <div className="flex flex-wrap gap-3">
            {lastInputRef.current && (
              <button
                onClick={retry}
                className="px-5 py-2.5 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
              >
                Tentar novamente
              </button>
            )}
            <button
              onClick={reset}
              className="px-5 py-2.5 border border-line text-zinc-300 text-sm hover:border-line-soft hover:text-zinc-100 transition-colors"
            >
              Editar dados
            </button>
          </div>
        </div>
      )}
    </main>
  );
}

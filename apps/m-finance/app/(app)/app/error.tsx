"use client";

import { useEffect } from "react";
import { TriangleAlert } from "lucide-react";

export default function AppError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <div className="flex min-h-[60vh] items-center justify-center">
      <section className="relative w-full max-w-md rounded-xl border border-border-subtle bg-background-card/95 p-6 shadow-xl shadow-black/15">
        <div className="mb-4 flex h-11 w-11 items-center justify-center rounded-md border border-accent-border bg-accent-soft text-accent">
          <TriangleAlert size={20} />
        </div>
        <h2 className="text-lg font-semibold text-text-primary">Algo deu errado</h2>
        <p className="mt-2 text-sm leading-6 text-text-muted">
          Não foi possível carregar esta página. Tente novamente — se o problema continuar, recarregue
          o app.
        </p>
        {error.digest ? (
          <p className="num mt-3 text-xs text-text-muted">Código: {error.digest}</p>
        ) : null}
        <div className="mt-5 flex flex-wrap gap-3">
          <button
            className="clip-notch sheen group focus-ring relative inline-flex min-h-11 items-center justify-center gap-2 bg-accent-strong px-4 text-sm font-semibold tracking-tight text-text-primary shadow-lg shadow-accent/25 transition duration-200 hover:bg-accent-strong-hover active:scale-[0.985]"
            onClick={reset}
            type="button"
          >
            Tentar novamente
          </button>
          <a
            className="focus-ring inline-flex min-h-11 items-center rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
            href="/app/dashboard"
          >
            Voltar ao dashboard
          </a>
        </div>
      </section>
    </div>
  );
}

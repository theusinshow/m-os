"use client";

import { useEffect, useState } from "react";
import { shouldReload } from "@/lib/version-check";

const RUNNING = process.env.NEXT_PUBLIC_BUILD_ID ?? "dev";
const KEY = "mf-reloaded-for";
const EVERY_MS = 5 * 60_000;

function readReloaded() {
  try {
    return window.sessionStorage.getItem(KEY);
  } catch {
    return null;
  }
}

/**
 * Cada deploy é a versão que vale: uma aba (ou o PWA do iPhone) aberta antes
 * do push troca sozinha para a nova — ao voltar para a tela e a cada cinco
 * minutos (ADR-076).
 */
export function VersionWatch() {
  const [updating, setUpdating] = useState(false);

  useEffect(() => {
    if (RUNNING === "dev") return;

    async function check() {
      let served: string | null = null;
      try {
        const response = await fetch("/api/version", { cache: "no-store" });
        if (response.ok) served = ((await response.json()) as { version?: string }).version ?? null;
      } catch {
        return; // Sem rede: a aba continua como está.
      }
      if (!shouldReload({ running: RUNNING, served, alreadyReloadedFor: readReloaded() })) return;
      try {
        window.sessionStorage.setItem(KEY, served ?? "");
      } catch {
        // Sem onde marcar, o laço é contido pelo intervalo de cinco minutos.
      }
      setUpdating(true);
      window.setTimeout(() => window.location.reload(), 600);
    }

    const onVisible = () => {
      if (document.visibilityState === "visible") void check();
    };
    void check();
    const timer = window.setInterval(() => void check(), EVERY_MS);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, []);

  if (!updating) return null;
  return (
    <div
      aria-live="polite"
      className="fixed inset-0 z-[100] grid place-items-center bg-background-primary/95 backdrop-blur"
      role="status"
    >
      <p className="text-sm font-semibold text-text-primary">Atualizando o M-Finance…</p>
    </div>
  );
}

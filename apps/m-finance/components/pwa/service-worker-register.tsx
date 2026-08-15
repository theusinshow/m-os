"use client";

import { useEffect } from "react";

// Registers the service worker in production only. In dev a SW would cache stale
// build assets and fight the Next.js fast-refresh pipeline.
export function ServiceWorkerRegister() {
  useEffect(() => {
    if (process.env.NODE_ENV !== "production") return;
    if (typeof navigator === "undefined" || !("serviceWorker" in navigator)) return;

    const register = () => {
      navigator.serviceWorker.register("/sw.js").catch(() => {
        // Registration failures are non-fatal — the app still works online.
      });
    };

    window.addEventListener("load", register);
    return () => window.removeEventListener("load", register);
  }, []);

  return null;
}

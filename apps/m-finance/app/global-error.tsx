"use client";

import { useEffect } from "react";
import "@/styles/globals.css";

export default function GlobalError({
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
    <html lang="pt-BR">
      <body>
        <div
          style={{
            minHeight: "100vh",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "1.5rem",
          }}
        >
          <section
            style={{
              width: "100%",
              maxWidth: "28rem",
              borderRadius: "0.75rem",
              border: "1px solid rgba(245, 242, 237, 0.08)",
              background: "#091009",
              padding: "1.5rem",
              color: "#F5F2ED",
            }}
          >
            <h2 style={{ fontSize: "1.125rem", fontWeight: 600, margin: 0 }}>
              Erro inesperado
            </h2>
            <p style={{ marginTop: "0.5rem", fontSize: "0.875rem", color: "#8A8780", lineHeight: 1.5 }}>
              O M Finance encontrou um problema e precisa recarregar.
            </p>
            <button
              onClick={reset}
              type="button"
              style={{
                marginTop: "1.25rem",
                minHeight: "2.75rem",
                padding: "0 1rem",
                background: "#FB3640",
                color: "#020A06",
                fontWeight: 600,
                border: "none",
                borderRadius: "0.375rem",
                cursor: "pointer",
              }}
            >
              Recarregar
            </button>
          </section>
        </div>
      </body>
    </html>
  );
}

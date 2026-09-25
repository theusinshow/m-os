import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./estilo.css";
import "./telas.css";

const raiz = document.getElementById("raiz");
if (raiz) createRoot(raiz).render(<StrictMode><App /></StrictMode>);

// O service worker sobe na abertura, e não só ao ativar notificações: é ele
// que guarda a casca para o app abrir sem rede e a captura ir para a fila do
// aparelho (ADR-077). Com rede, ele nunca serve cópia — ver `sw.js`.
if ("serviceWorker" in navigator) {
  void navigator.serviceWorker.register("/sw.js").catch(() => undefined);
}

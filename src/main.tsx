import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "@/app/App";
import { ReminderWidget } from "@/features/reminder/ReminderWidget";
import { isTauri } from "@/services/tauri";
import "@/styles/global.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Elemento #root nao encontrado no index.html");
}

/** Rotula a janela atual: a janela "reminder" renderiza o widget flutuante. */
async function currentLabel(): Promise<string> {
  if (!isTauri()) return "main";
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

void currentLabel().then((label) => {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      {label === "reminder" ? <ReminderWidget /> : <App />}
    </React.StrictMode>,
  );
});

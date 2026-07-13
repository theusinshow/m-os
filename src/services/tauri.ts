/**
 * Camada de acesso ao runtime Tauri.
 *
 * Centraliza `invoke` e a escuta de eventos com tipagem, e degrada de forma
 * segura quando o app roda em um navegador comum (ex.: `vite dev`/`vite build`
 * sem o backend). Isso permite desenvolver e testar a UI com dados temporarios
 * antes de o backend estar ligado.
 *
 * Nenhum SQL bruto e exposto ao frontend: a comunicacao acontece por comandos
 * especificos do backend (secao 19).
 */

import type {
  TauriEventName,
  TauriEventPayloadMap,
} from "@/types/events";

/** True quando executando dentro do runtime Tauri (WebView), false no browser. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Esconde a janela atual (usado pelo widget de lembrete). No-op no browser. */
export async function hideCurrentWindow(): Promise<void> {
  if (!isTauri()) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().hide();
}

/** Exibe e traz a janela atual para o topo (usado pelo widget). */
export async function showCurrentWindow(): Promise<void> {
  if (!isTauri()) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  await w.show();
  await w.setAlwaysOnTop(true);
  await w.setFocus();
}

/**
 * Invoca um comando Tauri tipado. Lanca erro claro quando fora do Tauri,
 * para que os chamadores decidam usar dados temporarios nesta fase.
 */
export async function invokeCommand<TResult>(
  command: string,
  args?: Record<string, unknown>,
): Promise<TResult> {
  if (!isTauri()) {
    throw new Error(
      `Comando "${command}" indisponivel: runtime Tauri ausente (rodando no navegador).`,
    );
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<TResult>(command, args);
}

/**
 * Escuta um evento Tauri tipado. No navegador, e um no-op que retorna uma
 * funcao de cancelamento vazia.
 */
export async function listenEvent<TName extends TauriEventName>(
  name: TName,
  handler: (payload: TauriEventPayloadMap[TName]) => void,
): Promise<() => void> {
  if (!isTauri()) {
    return () => {};
  }
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen<TauriEventPayloadMap[TName]>(name, (event) => {
    handler(event.payload);
  });
  return unlisten;
}

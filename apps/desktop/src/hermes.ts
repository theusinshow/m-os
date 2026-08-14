import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Fronteira do renderer com a ponte do Hermes.
 *
 * Espelha o padrao de api.ts de proposito: nenhuma chamada de rede em
 * componente React, e nenhuma credencial atravessa para ca. O renderer aprende
 * que existe credencial, nunca qual e.
 */

export type HermesConnectionState = "offline" | "connecting" | "online";

export type HermesStatus = {
  state: HermesConnectionState;
  hasCredentials: boolean;
  baseUrl: string;
  /** Mensagem legivel do ultimo erro. Vazia quando nao ha. */
  detail: string;
};

/** O que a ponte entrega. Espelha `Outcome` do crate mos-hermes. */
export type HermesOutcome =
  | { outcome: "delta"; text: string }
  | { outcome: "complete" }
  | { outcome: "reasoning"; text: string }
  | { outcome: "tool"; name: string; running: boolean }
  | { outcome: "approval"; prompt: string }
  | { outcome: "busy" }
  | { outcome: "failed"; message: string }
  | { outcome: "unknown_frame"; kind: string };

export const hermes = {
  status() {
    return invoke<HermesStatus>("hermes_status");
  },
  /** Preguicosa: so na primeira vez que o modo Hermes e usado. */
  connect() {
    return invoke<void>("hermes_connect");
  },
  disconnect() {
    return invoke<void>("hermes_disconnect");
  },
  send(text: string) {
    return invoke<void>("hermes_send", { text });
  },
  /** Tambem nega todas as aprovacoes pendentes, do lado do servidor. */
  interrupt() {
    return invoke<void>("hermes_interrupt");
  },
  approve(approved: boolean) {
    return invoke<void>("hermes_approve", { approved });
  },
  setCredentials(username: string, password: string) {
    return invoke<void>("hermes_set_credentials", { username, password });
  },
  clearCredentials() {
    return invoke<void>("hermes_clear_credentials");
  },
  setBaseUrl(url: string) {
    return invoke<void>("hermes_set_base_url", { url });
  },
  onEvent(handler: (outcome: HermesOutcome) => void) {
    return listen<HermesOutcome>("hermes-event", (event) => handler(event.payload));
  },
  onState(handler: (status: HermesStatus) => void) {
    return listen<HermesStatus>("hermes-state", (event) => handler(event.payload));
  },
};

/** Texto que o usuario le quando o Hermes nao esta disponivel. */
export function hermesUnavailableLabel(status: HermesStatus) {
  if (!status.hasCredentials) return "Configure usuário e senha do Hermes em Settings.";
  if (status.detail) return status.detail;
  return "Hermes indisponível — o túnel SSH não parece estar aberto.";
}

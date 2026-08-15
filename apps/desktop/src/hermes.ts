import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Fronteira do renderer com a ponte do Hermes.
 *
 * Espelha o padrao de api.ts de proposito: nenhuma chamada de rede em
 * componente React, e nenhuma credencial atravessa para ca. O renderer aprende
 * que existe credencial, nunca qual e.
 */

const BASE_URL_KEY = "m-os-hermes-base-url";

export type HermesConnectionState = "offline" | "connecting" | "online";

export type HermesStatus = {
  state: HermesConnectionState;
  hasCredentials: boolean;
  baseUrl: string;
  /** Mensagem legivel do ultimo erro. Vazia quando nao ha. */
  detail: string;
  /** Online significa socket aceito; isto significa sessao aberta. Perguntar
   *  antes disso falha, e a janela entre os dois e real sobre um tunel. */
  sessionReady: boolean;
};

/** Falha da conexao, com o nome da causa. Espelha `HermesFailure` da ponte. */
export type HermesFailure = {
  kind: "unreachable" | "unauthorized" | "rate_limited" | "protocol" | "missing_credentials" | "gateway";
  message: string;
  /** Só `unreachable` é. Credencial recusada não muda por insistência, e
   *  rate_limited PIORA — repetir foi o que causou o bloqueio. */
  retriable: boolean;
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
  /** Rejeita com `HermesFailure`, nunca com string solta: quem chama precisa
   *  saber SE pode tentar de novo antes de tentar. */
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
  /** O endereco nao e segredo, entao mora no renderer e e reaplicado no boot.
   *  Sem isso ele vivia so em memoria do processo Rust e voltava ao padrao a
   *  cada reinicio — em silencio, para quem usa porta ou prefixo diferente. */
  async setBaseUrl(url: string) {
    await invoke<void>("hermes_set_base_url", { url });
    localStorage.setItem(BASE_URL_KEY, url);
  },
  /** Reaplica o endereco guardado. Chamado uma vez, na abertura do app. */
  async restoreBaseUrl() {
    const saved = localStorage.getItem(BASE_URL_KEY);
    if (!saved) return;
    await invoke<void>("hermes_set_base_url", { url: saved }).catch(() => undefined);
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

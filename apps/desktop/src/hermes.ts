import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * Fronteira do renderer com a camada Jarvis.
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
  /** A conversa a que a sessao aberta pertence. */
  conversationId: string;
};

/** Falha da conexao, com o nome da causa. Espelha `HermesFailure` da ponte. */
export type HermesFailure = {
  kind: "unreachable" | "unauthorized" | "rate_limited" | "protocol" | "missing_credentials" | "gateway";
  message: string;
  /** Só `unreachable` é. Credencial recusada não muda por insistência, e
   *  rate_limited PIORA — repetir foi o que causou o bloqueio. */
  retriable: boolean;
};

export type ToolRunState = "queued" | "running" | "success" | "error" | "cancelled" | "waiting_permission";
export type ContextOrigin = "explicit" | "automatic";
export type ContextEntity = "project" | "task" | "capture" | "resource" | "workspace" | "screen";

/** Corpo de uma parte. Espelha `PartBody` do dominio. */
export type PartBody =
  | { kind: "text"; text: string }
  | { kind: "reasoning"; text: string }
  | { kind: "tool_run"; name: string; state: ToolRunState; detail: string }
  | { kind: "status"; text: string }
  | { kind: "error"; message: string }
  | {
      kind: "action_proposal";
      /** A proposta crua, que identifica qual parte resolver. */
      raw: string;
      preview: ActionPreview;
      status: ProposalStatus;
      outcome: string;
    }
  | {
      kind: "context_ref";
      origin: ContextOrigin;
      entity: ContextEntity;
      id: string;
      label: string;
      /** Campos que entraram no bloco enviado. */
      fields: string[];
      /** Tamanho do que efetivamente saiu, em bytes. */
      bytes: number;
    };

export type ProposalStatus = "pending" | "executed" | "cancelled" | "refused" | "failed";
export type ActionRisk = "low" | "medium" | "high";

/** O que o Hermes propôs, e que só vira efeito depois que você confirma. */
export type ActionPreview = {
  action: string;
  title: string;
  lines: { label: string; value: string }[];
  risk: ActionRisk;
  confirmation: "none" | "explicit";
};

export type MessagePart = { id: string; seq: number; body: PartBody };
export type MessageRole = "user" | "assistant" | "system";
export type MessageStatus = "pending" | "streaming" | "complete" | "interrupted" | "failed";

export type Message = {
  id: string;
  conversationId: string;
  seq: number;
  role: MessageRole;
  status: MessageStatus;
  createdAt: string;
  parts: MessagePart[];
};

export type Conversation = {
  id: string;
  /** Vazio ate o Hermes nomear. O M/OS nao inventa titulo. */
  title: string;
  hermesSessionId: string | null;
  lifecycleState: "active" | "archived" | "trashed";
  createdAt: string;
  updatedAt: string;
};

export type ConversationSummary = {
  id: string;
  title: string;
  updatedAt: string;
  messageCount: number;
  preview: string;
};

/** Contexto anexado antes do envio. Nada sai sem um chip visivel (ADR-027). */
export type ContextInput = {
  origin: ContextOrigin;
  entity: ContextEntity;
  id: string;
  label: string;
};

/** O que a ponte entrega, com o endereco de onde pertence. */
export type HermesOutcome =
  | { outcome: "delta"; text: string }
  | { outcome: "complete" }
  | { outcome: "reasoning"; text: string }
  | { outcome: "tool"; name: string; running: boolean }
  | { outcome: "status"; text: string }
  | { outcome: "approval"; prompt: string }
  | { outcome: "clarify"; requestId: string; question: string; choices: string[] }
  | { outcome: "sudo_refused" }
  | { outcome: "busy" }
  | { outcome: "failed"; message: string }
  | { outcome: "history"; messages: unknown[] }
  | { outcome: "title"; title: string }
  | { outcome: "unknown_frame"; kind: string };

/** Evento de streaming enderecado. Sem isto, duas superficies assinando o mesmo
 *  barramento dividiam a mesma resposta entre si. */
export type TurnEvent = HermesOutcome & { conversationId?: string; messageId?: string };

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
  send(conversationId: string, text: string, contexts: ContextInput[]) {
    return invoke<void>("hermes_send", { conversationId, text, contexts });
  },
  /** Tambem nega todas as aprovacoes pendentes, do lado do servidor, e grava a
   *  resposta parcial que ja chegou. */
  interrupt() {
    return invoke<void>("hermes_interrupt");
  },
  approve(approved: boolean) {
    return invoke<void>("hermes_approve", { approved });
  },
  /** Sem isto o agente fica travado ate o timeout de cinco minutos. */
  clarify(requestId: string, answer: string) {
    return invoke<void>("hermes_clarify", { requestId, answer });
  },
  selectConversation(conversationId: string) {
    return invoke<void>("hermes_select_conversation", { conversationId });
  },
  loadHistory() {
    return invoke<void>("hermes_load_history");
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
  onEvent(handler: (event: TurnEvent) => void) {
    return listen<TurnEvent>("hermes-event", (event) => handler(event.payload));
  },
  onState(handler: (status: HermesStatus) => void) {
    return listen<HermesStatus>("hermes-state", (event) => handler(event.payload));
  },
  /** Mensagem gravada. O renderer troca o buffer de streaming por ela, o que
   *  impede o texto da tela de divergir do texto do banco. */
  onMessage(handler: (message: Message) => void) {
    return listen<Message>("hermes-message", (event) => handler(event.payload));
  },
  onConversation(handler: (conversation: Conversation) => void) {
    return listen<Conversation>("hermes-conversation", (event) => handler(event.payload));
  },
  onHistory(handler: (conversationId: string) => void) {
    return listen<string>("hermes-history", (event) => handler(event.payload));
  },
};

export const conversations = {
  list(includeArchived = false) {
    return invoke<ConversationSummary[]>("conversation_list", { includeArchived });
  },
  current() {
    return invoke<Conversation>("conversation_current");
  },
  create() {
    return invoke<Conversation>("conversation_create");
  },
  messages(id: string) {
    return invoke<Message[]>("conversation_messages", { id });
  },
  rename(id: string, title: string) {
    return invoke<Conversation>("conversation_rename", { id, title });
  },
  setArchived(id: string, archived: boolean) {
    return invoke<Conversation>("conversation_set_archived", { id, archived });
  },
  remove(id: string) {
    return invoke<void>("conversation_delete", { id });
  },
  search(query: string) {
    return invoke<ConversationSummary[]>("conversation_search", { query });
  },
  /** Descarta uma mensagem e tudo depois dela. Regenerate e edicao. */
  truncate(messageId: string) {
    return invoke<void>("conversation_truncate", { messageId });
  },
  /** Executa ou cancela uma proposta. O modelo propõe; isto é o M/OS agindo. */
  resolveAction(messageId: string, raw: string, approved: boolean) {
    return invoke<Message>("action_resolve", { messageId, raw, approved });
  },
};

/** Texto que o usuario le quando o Hermes nao esta disponivel. */
export function hermesUnavailableLabel(status: HermesStatus) {
  if (!status.hasCredentials) return "Configure usuário e senha do Hermes em Settings.";
  if (status.detail) return status.detail;
  return "Hermes indisponível — o túnel SSH não parece estar aberto.";
}

/** Junta as partes de texto. E o que Copy copia e o que a edicao reaproveita. */
export function messageText(message: Message) {
  return message.parts
    .filter((part): part is MessagePart & { body: { kind: "text"; text: string } } => part.body.kind === "text")
    .map((part) => part.body.text)
    .join("");
}

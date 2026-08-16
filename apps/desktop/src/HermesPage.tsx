import { FormEvent, KeyboardEvent, memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import {
  conversations as conversationApi,
  hermes,
  hermesUnavailableLabel,
  messageText,
  type ActionResolution,
  type ContextInput,
  type Conversation,
  type ConversationSummary,
  type HermesStatus,
  type Message,
  type MessagePart,
  type ToolRunState,
} from "./hermes";
import { Icon, SmallIcon } from "./Icon";
import { Markdown } from "./markdown";
import type { Capture, Project, SearchItem, Task } from "./types";

/**
 * Hermes como superficie unica, na direcao "Marginalia"
 * (`M-OS Hermes - Design Direction v1`, §03).
 *
 * O dispositivo: tudo que o sistema FAZ — buscar, ler, citar, executar — mora
 * numa coluna estreita a esquerda. Tudo que ele DIZ mora na coluna de leitura.
 * Nunca se misturam, nunca viram card. E por isso que a atividade de ferramenta
 * nao empurra a prosa para baixo.
 *
 * O reconhecimento e tipografico: voce fala em 21px, ele responde em 15px. Nao
 * ha bolha, avatar, orbe nem gradiente.
 *
 * Ate a auditoria de 2026-08-15 existiam DUAS superficies: esta tela e o modo
 * Hermes do Command. As duas assinavam `hermes-event` no barramento global e o
 * Command montava por cima desta, entao com as duas abertas os deltas da mesma
 * resposta se dividiam entre dois estados.
 */

/** O turno chegando, acumulado fora do estado do React. */
type StreamBuffer = {
  messageId: string;
  text: string;
  reasoning: string;
  tools: { name: string; state: ToolRunState }[];
  status: string[];
  startedAt: number;
};

const EMPTY_BUFFER: StreamBuffer = { messageId: "", text: "", reasoning: "", tools: [], status: [], startedAt: 0 };

/** Marcador textual do estado. Nenhum estado depende so de cor (§20). */
const TOOL_MARK: Record<ToolRunState, string> = {
  queued: "·",
  running: "→",
  success: "✓",
  error: "!",
  cancelled: "×",
  waiting_permission: "?",
};

const TOOL_LABEL: Record<ToolRunState, string> = {
  queued: "na fila",
  running: "executando",
  success: "concluído",
  error: "falhou",
  cancelled: "cancelado",
  waiting_permission: "aguardando permissão",
};

function clockOf(value: string) {
  return new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(new Date(value));
}

/** Agrupamento por tempo. Conversas nao sao arquivos: a maioria morre no mesmo
 *  dia, e a lista precisa refletir isso em vez de virar uma pilha unica (§05). */
function groupOf(value: string) {
  const date = new Date(value);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return "HOJE";
  if (date.toDateString() === yesterday.toDateString()) return "ONTEM";
  return new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "short" }).format(date).toUpperCase();
}

/** Os contextos que uma mensagem registrou. E o que a edicao restaura. */
function contextsOf(message: Message): ContextInput[] {
  return message.parts
    .filter((part) => part.body.kind === "context_ref")
    .map((part) => {
      const body = part.body as Extract<MessagePart["body"], { kind: "context_ref" }>;
      return { origin: body.origin, entity: body.entity, id: body.id, label: body.label };
    });
}

const ENTITY_TAG: Record<ContextInput["entity"], string> = {
  project: "PROJ",
  task: "TASK",
  capture: "CAP",
  resource: "RES",
  workspace: "WS",
  screen: "TELA",
};

/**
 * O estado de hoje, lido do que o M/OS ja sabe.
 *
 * Sao perguntas de verdade sobre dados de verdade — nao sugestoes genericas.
 * Somem depois da primeira mensagem e nao voltam na mesma sessao (§05).
 */
function suggestionsFor(inbox: Capture[], projects: Project[], tasks: Task[]) {
  const suggestions: string[] = [];
  if (inbox.length) {
    const oldest = inbox.reduce((left, right) => (left.capturedAt < right.capturedAt ? left : right));
    const since = new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "long" }).format(new Date(oldest.capturedAt));
    suggestions.push(`${inbox.length} ${inbox.length === 1 ? "item" : "itens"} na Inbox desde ${since} — organizar em lote?`);
  }
  const stale = projects
    .filter((project) => project.lifecycleState === "active")
    .filter((project) => Date.now() - new Date(project.updatedAt).getTime() > 21 * 24 * 60 * 60 * 1000)
    .sort((left, right) => left.updatedAt.localeCompare(right.updatedAt))[0];
  if (stale) suggestions.push(`${stale.name} está parado há semanas — retomar ou arquivar?`);
  const doing = tasks.filter((task) => task.lifecycleState === "active" && task.state === "doing");
  if (doing.length) suggestions.push(`${doing.length} ${doing.length === 1 ? "task está" : "tasks estão"} em andamento — revisar o dia?`);
  return suggestions.slice(0, 3);
}

/** Chip de contexto. Borda solida e manual, tracejada e automatico — sem cor
 *  extra, sem icone, sem legenda (§06). */
function Chip({ entity, label, origin, onRemove, detail }: {
  entity: ContextInput["entity"];
  label: string;
  origin: ContextInput["origin"];
  onRemove?: () => void;
  detail?: string;
}) {
  return (
    <span className="hermes-chip" data-origin={origin} title={detail}>
      <span className="hermes-chip-kind">{ENTITY_TAG[entity]}</span>
      <span className="hermes-chip-label">{label}</span>
      {onRemove ? (
        <button type="button" aria-label={`Remover ${label} do contexto`} onClick={onRemove}>✕</button>
      ) : null}
    </span>
  );
}

/**
 * A margem de uma resposta.
 *
 * Durante: os passos, o corrente pulsando. Depois: um recibo de uma linha que
 * se expande no clique. A prosa nunca e empurrada (§08, decisao A).
 */
function AnswerGutter({ tools, elapsed, live }: {
  tools: { name: string; state: ToolRunState }[];
  elapsed: string;
  live: boolean;
}) {
  const [open, setOpen] = useState(false);
  const settled = !live && tools.length > 0;

  return (
    <div className="hermes-gutter">
      <div className="hermes-gutter-who">HERMES</div>
      {settled && !open ? (
        <button type="button" className="hermes-receipt" onClick={() => setOpen(true)}>
          {tools.length} {tools.length === 1 ? "passo" : "passos"}{elapsed ? ` · ${elapsed}` : ""}
        </button>
      ) : null}
      {(live || open) && tools.length ? (
        <ul className="hermes-steps">
          {tools.map((tool, index) => (
            <li key={index} data-state={tool.state}>
              <span aria-hidden="true">{TOOL_MARK[tool.state]}</span> {tool.name}
              <span className="visually-hidden"> — {TOOL_LABEL[tool.state]}</span>
            </li>
          ))}
        </ul>
      ) : null}
      {live && !tools.length ? <div className="hermes-thinking">pensando</div> : null}
      {settled && open ? (
        <button type="button" className="hermes-receipt" onClick={() => setOpen(false)}>fechar</button>
      ) : null}
    </div>
  );
}

/**
 * O que o Hermes propôs.
 *
 * O cartão é a explicação do que ele entendeu, e por isso aparece para toda
 * proposta — inclusive as de risco baixo. Quem clica "Criar Task" na interface
 * escolheu; quem falou uma frase pode ter sido mal interpretado, e
 * `UX-PRINCIPLES` §19 pede que o sistema mostre o que compreendeu antes de
 * agir. O risco decide o peso da confirmação, não a existência dela.
 */
function ActionCard({ part, messageId, onResolved }: {
  part: Extract<MessagePart["body"], { kind: "action_proposal" }>;
  messageId: string;
  onResolved: (resolution: ActionResolution) => void;
}) {
  const [working, setWorking] = useState(false);

  async function resolve(approved: boolean) {
    setWorking(true);
    const resolution = await conversationApi
      .resolveAction(messageId, part.raw, approved)
      .catch(() => null);
    setWorking(false);
    if (resolution) onResolved(resolution);
  }

  return (
    <div className="hermes-action" data-status={part.status} data-risk={part.preview.risk}>
      <div className="hermes-action-head">
        <span className="micro-label">{part.preview.title}</span>
        {part.preview.risk !== "low" ? (
          <span className="micro-label" data-risk>RISCO {part.preview.risk === "high" ? "ALTO" : "MÉDIO"}</span>
        ) : null}
      </div>
      {part.preview.lines.length ? (
        <dl className="hermes-action-lines">
          {part.preview.lines.map((entry) => (
            <div key={entry.label}>
              <dt>{entry.label}</dt>
              <dd>{entry.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {part.status === "pending" ? (
        <div className="hermes-action-foot">
          <button type="button" disabled={working} onClick={() => void resolve(false)}>CANCELAR</button>
          <button type="button" data-primary disabled={working} onClick={() => void resolve(true)}>
            {part.preview.risk === "high" ? "CONFIRMAR" : "FAZER"}
          </button>
        </div>
      ) : (
        <p className="hermes-action-outcome">{part.outcome}</p>
      )}
    </div>
  );
}

/**
 * Uma mensagem gravada.
 *
 * `memo` nao e otimizacao preventiva: numa thread longa toda mensagem fechada e
 * imutavel, e sem isto o Markdown de todas elas era reparseado a cada token da
 * resposta em curso.
 */
const StoredMessage = memo(function StoredMessage({ message, onCopy, onEdit, onRegenerate, onResolved }: {
  message: Message;
  onCopy: (message: Message) => void;
  onEdit: (message: Message) => void;
  onRegenerate: (message: Message) => void;
  onResolved: (resolution: ActionResolution) => void;
}) {
  const text = messageText(message);
  const chips = message.parts.filter((part) => part.body.kind === "context_ref");
  const tools = message.parts
    .filter((part) => part.body.kind === "tool_run")
    .map((part) => {
      const body = part.body as Extract<MessagePart["body"], { kind: "tool_run" }>;
      return { name: body.name, state: body.state };
    });

  if (message.role === "user") {
    return (
      <article className="hermes-turn" data-role="user">
        <div className="hermes-gutter">
          <div className="hermes-gutter-who">VOCÊ</div>
          <time dateTime={message.createdAt}>{clockOf(message.createdAt)}</time>
        </div>
        <div className="hermes-said">
          <p className="hermes-question">{text}</p>
          {chips.length ? (
            <div className="hermes-chips">
              {chips.map((part) => {
                const body = part.body as Extract<MessagePart["body"], { kind: "context_ref" }>;
                return (
                  <Chip
                    key={part.id}
                    entity={body.entity}
                    label={body.label}
                    origin={body.origin}
                    detail={`Enviado: ${body.fields.join(", ") || "nada"} · ${body.bytes} bytes`}
                  />
                );
              })}
            </div>
          ) : null}
          <div className="hermes-actions">
            <button type="button" onClick={() => onCopy(message)}>COPIAR</button>
            <button type="button" onClick={() => onEdit(message)}>EDITAR PERGUNTA</button>
          </div>
        </div>
      </article>
    );
  }

  const failed = message.status === "failed" || message.status === "interrupted";

  return (
    <article className="hermes-turn" data-role="assistant">
      <AnswerGutter tools={tools} elapsed="" live={false} />
      <div className="hermes-said">
        {message.parts.map((part) => {
          if (part.body.kind === "status") return <p className="hermes-system-line" key={part.id}>{part.body.text}</p>;
          if (part.body.kind === "reasoning") {
            return (
              <details className="hermes-reasoning" key={part.id}>
                <summary>RACIOCÍNIO</summary>
                <p>{part.body.text}</p>
              </details>
            );
          }
          if (part.body.kind === "error") return <p className="hermes-failed" key={part.id}>{part.body.message}</p>;
          if (part.body.kind === "action_proposal") {
            return (
              <ActionCard
                key={part.id}
                part={part.body}
                messageId={message.id}
                onResolved={onResolved}
              />
            );
          }
          if (part.body.kind === "text") return <Markdown key={part.id} source={part.body.text} />;
          return null;
        })}
        {message.status === "interrupted" ? (
          <p className="hermes-system-line">{text ? "Interrompido por você. O texto parcial fica." : "Interrompido antes de começar."}</p>
        ) : null}
        <div className="hermes-actions">
          {text ? <button type="button" onClick={() => onCopy(message)}>COPIAR</button> : null}
          <button type="button" onClick={() => onRegenerate(message)}>{failed ? "TENTAR DE NOVO" : "REFAZER"}</button>
        </div>
      </div>
    </article>
  );
});

export function HermesPage({ inbox, projects, tasks, receipt }: {
  inbox: Capture[];
  projects: Project[];
  tasks: Task[];
  openProject?: (project: Project) => void;
  openResource?: (id: string) => void;
  /** Mesma janela de recibo do resto do app. Tipada pela forma, e não
   *  importada de `App.tsx`, que já importa esta página. */
  receipt?: (action: { message: string; run: () => Promise<unknown> }) => void;
}) {
  const [status, setStatus] = useState<HermesStatus | null>(null);
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [summaries, setSummaries] = useState<ConversationSummary[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [stream, setStream] = useState<StreamBuffer | null>(null);
  const [draft, setDraft] = useState("");
  const [contexts, setContexts] = useState<ContextInput[]>([]);
  const [approval, setApproval] = useState<string | null>(null);
  const [clarify, setClarify] = useState<{ requestId: string; question: string; choices: string[] } | null>(null);
  const [clarifyAnswer, setClarifyAnswer] = useState("");
  const [mentions, setMentions] = useState<SearchItem[]>([]);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [listQuery, setListQuery] = useState("");
  const [railOpen, setRailOpen] = useState(true);
  const [renaming, setRenaming] = useState(false);
  const [announcement, setAnnouncement] = useState("");
  const [pinnedBottom, setPinnedBottom] = useState(true);

  const field = useRef<HTMLTextAreaElement>(null);
  const thread = useRef<HTMLDivElement>(null);
  const surface = useRef<HTMLDivElement>(null);
  const buffer = useRef<StreamBuffer>({ ...EMPTY_BUFFER });
  const frame = useRef(0);
  /** Ate onde a resposta ja foi anunciada. O leitor de tela recebe paragrafo
   *  concluido, nunca token (§20). */
  const announced = useRef(0);

  const conversationId = conversation?.id ?? "";
  const running = stream !== null;

  const flush = useCallback(() => {
    if (frame.current) return;
    frame.current = window.requestAnimationFrame(() => {
      frame.current = 0;
      const current = buffer.current;
      setStream({ ...current, tools: [...current.tools], status: [...current.status] });
      const boundary = current.text.lastIndexOf("\n\n");
      if (boundary > announced.current) {
        setAnnouncement(current.text.slice(announced.current, boundary).trim().slice(-400));
        announced.current = boundary;
      }
    });
  }, []);

  const reloadList = useCallback(async () => {
    setSummaries(await conversationApi.list().catch(() => []));
  }, []);

  useEffect(() => {
    void hermes.status().then(setStatus).catch(() => undefined);
    void conversationApi
      .current()
      .then(async (current) => {
        setConversation(current);
        setMessages(await conversationApi.messages(current.id).catch(() => []));
        await reloadList();
      })
      .catch(() => undefined);
  }, [reloadList]);

  useEffect(() => {
    const subscriptions = [
      hermes.onState(setStatus),
      hermes.onConversation((next) => {
        setConversation((current) => (current && current.id === next.id ? next : current));
        void reloadList();
      }),
      hermes.onHistory((id) => {
        void conversationApi.messages(id).then(setMessages).catch(() => undefined);
      }),
      hermes.onMessage((message) => {
        setMessages((current) => {
          const index = current.findIndex((candidate) => candidate.id === message.id);
          if (index === -1) return [...current, message];
          const next = [...current];
          next[index] = message;
          return next;
        });
        // A mensagem gravada substitui o buffer: o texto da tela deixa de poder
        // divergir do texto do banco.
        if (buffer.current.messageId === message.id && message.status !== "pending") {
          buffer.current = { ...EMPTY_BUFFER };
          announced.current = 0;
          setStream(null);
          setAnnouncement(message.status === "complete" ? "Resposta concluída." : "Resposta interrompida.");
        }
        void reloadList();
      }),
      hermes.onEvent((event) => {
        if (event.outcome === "approval") return setApproval(event.prompt);
        if (event.outcome === "clarify") {
          setClarifyAnswer("");
          setAnnouncement("O Hermes fez uma pergunta.");
          return setClarify({ requestId: event.requestId, question: event.question, choices: event.choices });
        }
        if (!("messageId" in event) || !event.messageId) return;
        if (buffer.current.messageId !== event.messageId) {
          buffer.current = { ...EMPTY_BUFFER, messageId: event.messageId, startedAt: Date.now() };
          announced.current = 0;
        }
        if (event.outcome === "delta") { buffer.current.text += event.text; return flush(); }
        if (event.outcome === "reasoning") { buffer.current.reasoning += event.text; return flush(); }
        if (event.outcome === "status") { buffer.current.status = [...buffer.current.status, event.text]; return flush(); }
        if (event.outcome === "sudo_refused") {
          buffer.current.status = [...buffer.current.status, "O Hermes pediu senha de sudo na VPS. O M/OS não pede senha de root."];
          return flush();
        }
        if (event.outcome === "tool") {
          const tools = [...buffer.current.tools];
          if (event.running) tools.push({ name: event.name, state: "running" });
          else {
            const last = [...tools].reverse().find((tool) => tool.name === event.name);
            if (last) last.state = "success";
          }
          buffer.current.tools = tools;
          return flush();
        }
        if (event.outcome === "busy") setAnnouncement("O Hermes ainda está respondendo.");
      }),
    ];
    return () => { subscriptions.forEach((subscription) => void subscription.then((dispose) => dispose())); };
  }, [flush, reloadList]);

  useEffect(() => () => { if (frame.current) window.cancelAnimationFrame(frame.current); }, []);

  /**
   * Reidrata da VPS quando o M/OS nao tem a conversa, mas a sessao existe.
   *
   * O caso real e restaurar um backup anterior, ou abrir num M/OS que ainda nao
   * tinha conversa local: o vinculo `hermes_session_id` sobrevive e o conteudo
   * esta la. Com mensagens locais nao se pede nada — elas sao a verdade que a
   * tela desenha, e sobrescreve-las com a projecao da VPS perderia as partes
   * que so o M/OS conhece, como o registro de contexto enviado.
   */
  const rehydrated = useRef("");
  useEffect(() => {
    if (!status?.sessionReady || !conversationId) return;
    if (messages.length || rehydrated.current === conversationId) return;
    rehydrated.current = conversationId;
    void hermes.loadHistory().catch(() => undefined);
  }, [status?.sessionReady, conversationId, messages.length]);

  useEffect(() => {
    const node = thread.current;
    if (!node || !pinnedBottom) return;
    node.scrollTop = node.scrollHeight;
  }, [messages, stream, pinnedBottom]);

  useEffect(() => {
    const node = thread.current;
    if (!node) return;
    const measure = () => setPinnedBottom(node.scrollHeight - node.scrollTop - node.clientHeight < 120);
    node.addEventListener("scroll", measure, { passive: true });
    return () => node.removeEventListener("scroll", measure);
  }, []);

  useEffect(() => {
    const at = /@([\wÀ-ú]*)$/.exec(draft);
    if (!at || at[1].length < 2) { setMentions([]); return; }
    let cancelled = false;
    const timeout = window.setTimeout(() => {
      void api.search(at[1], false)
        .then((items) => {
          if (cancelled) return;
          setMentions(items.filter((item) => ["project", "resource", "task", "capture"].includes(item.kind)).slice(0, 5));
          setMentionIndex(0);
        })
        .catch(() => setMentions([]));
    }, 120);
    return () => { cancelled = true; window.clearTimeout(timeout); };
  }, [draft]);

  const suggestions = useMemo(() => suggestionsFor(inbox, projects, tasks), [inbox, projects, tasks]);
  const online = status?.state === "online" && status.sessionReady;

  const grouped = useMemo(() => {
    const filtered = listQuery.trim()
      ? summaries.filter((item) => `${item.title} ${item.preview}`.toLowerCase().includes(listQuery.toLowerCase()))
      : summaries;
    const groups: { label: string; items: ConversationSummary[] }[] = [];
    for (const item of filtered) {
      const label = groupOf(item.updatedAt);
      const bucket = groups.find((group) => group.label === label);
      if (bucket) bucket.items.push(item);
      else groups.push({ label, items: [item] });
    }
    return groups;
  }, [summaries, listQuery]);

  function mentionLabel(item: SearchItem) {
    if (item.kind === "project") return item.project.name;
    if (item.kind === "resource") return item.resource.title;
    if (item.kind === "task") return item.task.title;
    if (item.kind === "capture") return item.capture.content.slice(0, 40);
    return "";
  }

  function mentionId(item: SearchItem) {
    if (item.kind === "project") return item.project.id;
    if (item.kind === "resource") return item.resource.id;
    if (item.kind === "task") return item.task.id;
    if (item.kind === "capture") return item.capture.id;
    return "";
  }

  function pickMention(item: SearchItem) {
    const label = mentionLabel(item);
    setDraft((current) => current.replace(/@([\wÀ-ú]*)$/, `@${label} `));
    setMentions([]);
    field.current?.focus();
    setContexts((current) =>
      current.some((entry) => entry.id === mentionId(item))
        ? current
        : [...current, { origin: "explicit", entity: item.kind as ContextInput["entity"], id: mentionId(item), label }]);
  }

  const ask = useCallback(async (text: string, attached: ContextInput[]) => {
    const question = text.trim();
    if (!question || !conversationId) return;
    setDraft("");
    setContexts([]);
    setAnnouncement("Enviado.");
    await hermes.send(conversationId, question, attached).catch(() => setAnnouncement("Não foi possível enviar."));
  }, [conversationId]);

  function submit(event: FormEvent) {
    event.preventDefault();
    if (running || !online) return;
    void ask(draft, contexts);
  }

  function edit(question: Message) {
    setDraft(messageText(question));
    setContexts(contextsOf(question));
    void conversationApi
      .truncate(question.id)
      .then(() => conversationApi.messages(conversationId))
      .then(setMessages)
      .catch(() => undefined);
    field.current?.focus();
  }

  async function regenerate(answer: Message) {
    const index = messages.findIndex((candidate) => candidate.id === answer.id);
    const question = [...messages.slice(0, index)].reverse().find((candidate) => candidate.role === "user");
    if (!question) return;
    const text = messageText(question);
    const attached = contextsOf(question);
    await conversationApi.truncate(question.id).catch(() => undefined);
    setMessages(await conversationApi.messages(conversationId).catch(() => messages));
    await ask(text, attached);
  }

  function copy(message: Message) {
    void navigator.clipboard.writeText(messageText(message));
    setAnnouncement("Copiado.");
  }

  const newConversation = useCallback(async () => {
    const created = await conversationApi.create().catch(() => null);
    if (!created) return;
    setConversation(created);
    setMessages([]);
    setStream(null);
    buffer.current = { ...EMPTY_BUFFER };
    await hermes.selectConversation(created.id).catch(() => undefined);
    await reloadList();
    field.current?.focus();
  }, [reloadList]);

  async function selectConversation(id: string) {
    if (id === conversationId) return;
    const opened = await conversationApi.messages(id).catch(() => null);
    if (opened === null) return;
    const summary = summaries.find((item) => item.id === id);
    setConversation((current) => (current ? { ...current, id, title: summary?.title ?? "" } : current));
    setMessages(opened);
    setStream(null);
    buffer.current = { ...EMPTY_BUFFER };
    await hermes.selectConversation(id).catch(() => undefined);
  }

  async function removeConversation(id: string) {
    await conversationApi.remove(id).catch(() => undefined);
    await reloadList();
    if (id === conversationId) {
      const current = await conversationApi.current().catch(() => null);
      if (current) {
        setConversation(current);
        setMessages(await conversationApi.messages(current.id).catch(() => []));
      }
    }
  }

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (mentions.length) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        const direction = event.key === "ArrowDown" ? 1 : -1;
        setMentionIndex((current) => (current + direction + mentions.length) % mentions.length);
        return;
      }
      if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); pickMention(mentions[mentionIndex]); return; }
      if (event.key === "Escape") { event.preventDefault(); setMentions([]); return; }
    }
    // Campo vazio, seta para cima: editar a ultima pergunta (§18).
    if (event.key === "ArrowUp" && !draft) {
      const last = [...messages].reverse().find((message) => message.role === "user");
      if (last) { event.preventDefault(); edit(last); return; }
    }
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (!running && online) void ask(draft, contexts);
    }
  }

  /** Atalhos da superficie. `Esc` para de qualquer foco dentro de Hermes —
   *  nunca depende de alcancar um botao (§20). */
  useEffect(() => {
    function handler(event: globalThis.KeyboardEvent) {
      const node = surface.current;
      if (!node || !node.contains(document.activeElement)) return;
      if (event.key === "Escape") {
        if (approval) { setApproval(null); void hermes.approve(false); event.preventDefault(); return; }
        // Limpar so o estado local deixava o agente bloqueado no `_block()` do
        // gateway com a caixa de resposta ja fora da tela — pensando para
        // sempre, sem saida. Desistir precisa responder.
        if (clarify) { setClarify(null); void hermes.clarifyCancel(clarify.requestId); event.preventDefault(); return; }
        if (running) { void hermes.interrupt(); event.preventDefault(); }
        return;
      }
      if (event.ctrlKey && event.key.toLowerCase() === "n") { event.preventDefault(); void newConversation(); }
      if (event.ctrlKey && event.key === "/") { event.preventDefault(); setRailOpen((open) => !open); }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [approval, clarify, running, newConversation]);

  async function renameConversation(title: string) {
    if (!conversationId) return;
    const renamed = await conversationApi.rename(conversationId, title).catch(() => null);
    if (renamed) setConversation(renamed);
    setRenaming(false);
    await reloadList();
  }

  const elapsed = stream?.startedAt ? `${((Date.now() - stream.startedAt) / 1000).toFixed(1)}s` : "";

  return <div className="hermes-page" data-rail={railOpen || undefined} ref={surface}>
    {railOpen ? (
      <aside className="hermes-rail" aria-label="Conversas">
        <div className="hermes-rail-head">
          <span className="micro-label">CONVERSAS</span>
          <button type="button" onClick={() => void newConversation()} title="Nova conversa · Ctrl+N" aria-label="Nova conversa"><SmallIcon name="plus" /></button>
        </div>
        <input
          className="hermes-rail-search"
          value={listQuery}
          onChange={(event) => setListQuery(event.currentTarget.value)}
          placeholder="buscar nas conversas"
          aria-label="Buscar nas conversas"
        />
        <div className="hermes-rail-list">
          {grouped.length ? grouped.map((group) => (
            <div className="hermes-rail-group" key={group.label}>
              <div className="micro-label">{group.label}</div>
              {group.items.map((item) => (
                <div className="hermes-rail-row" key={item.id} data-active={item.id === conversationId || undefined}>
                  <button type="button" className="hermes-rail-open" onClick={() => void selectConversation(item.id)}>
                    <span className="hermes-rail-title">{item.title || item.preview || "Conversa vazia"}</span>
                    <span className="hermes-rail-meta">{clockOf(item.updatedAt)} · {item.messageCount} {item.messageCount === 1 ? "MSG" : "MSGS"}</span>
                  </button>
                  <button type="button" className="hermes-rail-drop" aria-label={`Excluir ${item.title || "conversa"}`} title="Excluir conversa" onClick={() => void removeConversation(item.id)}>
                    <SmallIcon name="trash" />
                  </button>
                </div>
              ))}
            </div>
          )) : <p className="hermes-quiet">{listQuery ? "Nenhuma conversa encontrada." : "Nenhuma conversa ainda."}</p>}
        </div>
      </aside>
    ) : null}

    <div className="hermes-main">
      <div className="hermes-seeing">
        <span className="micro-label">VENDO</span>
        {/* So os chips rolam. Com a regua inteira rolando, o estado da conexao
            saia de vista quando havia contexto demais — e ele e justamente o
            que precisa estar sempre visivel. */}
        <div className="hermes-seeing-chips">
          {contexts.length ? contexts.map((context) => (
            <Chip
              key={context.id}
              entity={context.entity}
              label={context.label}
              origin={context.origin}
              onRemove={() => setContexts((current) => current.filter((entry) => entry.id !== context.id))}
            />
          )) : <span className="hermes-quiet">nada anexado — mencione com <b>@</b></span>}
        </div>
        {renaming ? (
          <form onSubmit={(event) => { event.preventDefault(); void renameConversation(new FormData(event.currentTarget).get("title") as string); }}>
            <input name="title" defaultValue={conversation?.title ?? ""} aria-label="Título da conversa" autoFocus />
          </form>
        ) : (
          // O nome acessível diz a AÇÃO e contém o texto visível. Só com o
          // título, o leitor de tela anunciava o nome da conversa e nada
          // indicava que clicar renomeia; só com "Renomear", o nome deixaria
          // de conter o rótulo visível (WCAG 2.5.3).
          <button type="button" className="hermes-title" onClick={() => setRenaming(true)} aria-label={`Renomear conversa: ${conversation?.title || "sem título"}`} title="Renomear conversa">
            {conversation?.title || "sem título"}
          </button>
        )}
        <span className="micro-label" data-state={status?.state}>
          {status?.state === "online" ? (status.sessionReady ? "ONLINE" : "ABRINDO SESSÃO") : status?.state === "connecting" ? "CONECTANDO" : "OFFLINE"}
        </span>
      </div>

      {/* Paragrafo concluido, nunca token. `aria-live` sobre a thread inteira
          fazia o Narrator ler a resposta caractere a caractere (§20). */}
      <p className="visually-hidden" role="status" aria-live="polite">{announcement}</p>

      <div className="hermes-thread-area" ref={thread}>
        <div className="hermes-thread">
          {!messages.length && !stream ? <div className="hermes-empty">
            <p className="hermes-empty-title">Pergunte, mande fazer,<br />ou jogue alguma coisa aqui.</p>
            {suggestions.length ? <div className="hermes-suggestions">
              {suggestions.map((suggestion, index) => (
                <button key={suggestion} type="button" disabled={!online} onClick={() => void ask(suggestion, [])}>
                  <span>{index + 1}</span>{suggestion}
                </button>
              ))}
            </div> : <p className="hermes-quiet">Nada pedindo atenção agora. Inbox vazia, nenhum Project parado.</p>}
          </div> : null}

          {messages.map((message) => (
            <StoredMessage
              key={message.id}
              message={message}
              onCopy={copy}
              onEdit={edit}
              onRegenerate={(answer) => void regenerate(answer)}
              onResolved={(resolution) => {
                setMessages((current) => current.map((candidate) => candidate.id === resolution.message.id ? resolution.message : candidate));
                // O recibo só aparece quando há caminho de volta. Sem Undo ele
                // não acrescentaria nada: o cartão na conversa já mostra o
                // desfecho, e de forma permanente.
                const step = resolution.undo;
                if (step && resolution.receipt) {
                  receipt?.({ message: resolution.receipt, run: () => conversationApi.undoAction(step) });
                }
              }}
            />
          ))}

          {stream ? (
            <article className="hermes-turn" data-role="assistant">
              <AnswerGutter tools={stream.tools} elapsed={elapsed} live />
              <div className="hermes-said">
                {stream.status.map((text, index) => <p className="hermes-system-line" key={index}>{text}</p>)}
                {stream.reasoning ? (
                  <details className="hermes-reasoning"><summary>RACIOCÍNIO</summary><p>{stream.reasoning}</p></details>
                ) : null}
                {/* Durante o streaming o texto e cru: o Markdown assenta uma vez,
                    no fim. Reparsear a cada quadro faria o bloco de codigo piscar
                    enquanto a cerca nao fecha. */}
                {/* O bloco de proposta some do texto em curso. Ele chega token
                    a token, então ficaria minutos na tela como JSON cru antes
                    de virar cartão — e o cartão é a forma legível da mesma
                    informação. Corta na abertura da cerca, não no fechamento,
                    porque o fechamento pode nunca chegar. */}
                {stream.text.split("```mos-action")[0].trim() ? (
                  <p className="hermes-streaming">
                    {stream.text.split("```mos-action")[0].trimEnd()}
                    <span className="hermes-caret" aria-hidden="true" />
                  </p>
                ) : null}
              </div>
            </article>
          ) : null}

          {clarify ? <div className="hermes-ask" role="group" aria-label="Pergunta do Hermes">
            <p>{clarify.question}</p>
            {clarify.choices.length ? <div className="hermes-ask-choices">
              {clarify.choices.map((choice) => (
                <button key={choice} type="button" onClick={() => { void hermes.clarify(clarify.requestId, choice); setClarify(null); }}>{choice}</button>
              ))}
            </div> : null}
            <form onSubmit={(event) => { event.preventDefault(); void hermes.clarify(clarify.requestId, clarifyAnswer); setClarify(null); }}>
              <input value={clarifyAnswer} onChange={(event) => setClarifyAnswer(event.currentTarget.value)} placeholder="responder" aria-label="Resposta ao Hermes" />
              <button type="submit">RESPONDER</button>
            </form>
          </div> : null}

          {/* O composer segue livre durante a aprovacao; Esc descarta (§17). */}
          {approval ? <div className="hermes-ask" role="alertdialog" aria-label="Aprovação do Hermes">
            <p>{approval}</p>
            <div className="hermes-ask-choices">
              <button type="button" onClick={() => { setApproval(null); void hermes.approve(false); }}>NEGAR</button>
              <button type="button" data-primary onClick={() => { setApproval(null); void hermes.approve(true); }}>APROVAR</button>
            </div>
          </div> : null}
        </div>
      </div>

      {!pinnedBottom ? (
        <button type="button" className="hermes-jump" onClick={() => { setPinnedBottom(true); const node = thread.current; if (node) node.scrollTop = node.scrollHeight; }}>
          novas mensagens abaixo
        </button>
      ) : null}

      <form className="hermes-composer" onSubmit={submit}>
        {mentions.length ? <div className="hermes-mentions" role="listbox" aria-label="Contexto para anexar">
          {mentions.map((item, index) => (
            <button
              key={`${item.kind}-${mentionId(item)}`}
              type="button"
              role="option"
              aria-selected={index === mentionIndex}
              data-active={index === mentionIndex || undefined}
              onClick={() => pickMention(item)}
            >
              <span className="hermes-mention-kind">{ENTITY_TAG[item.kind as ContextInput["entity"]] ?? "ITEM"}</span>
              <span className="hermes-mention-name">{mentionLabel(item)}</span>
            </button>
          ))}
        </div> : null}

        <div className="hermes-trilho" data-running={running || undefined} data-offline={!online || undefined}>
          <span className="hermes-bar" aria-hidden="true" />
          <textarea
            ref={field}
            value={draft}
            rows={1}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={onKeyDown}
            placeholder={running ? "Hermes está escrevendo" : "Pergunte alguma coisa"}
            aria-label="Perguntar ao Hermes"
            disabled={!online}
          />
          {/* O modo e a unica promessa que o sistema faz sobre o que vai
              acontecer com seus dados (§07). Hoje so existe uma promessa
              verdadeira, e ela e garantida pela arquitetura: mos-hermes nao
              compila com acesso ao banco. Quando ACT existir, este slot vira o
              ciclo de quatro. Ver ADR-029. */}
          <span className="hermes-mode" title="O Hermes lê o M/OS, mas ainda não escreve nele. Criar Task, agendar e abrir Issue são fases seguintes.">NÃO ESCREVE</span>
          {running
            ? <button type="button" className="hermes-stop" onClick={() => void hermes.interrupt()}>PARAR ⎋</button>
            : <button type="submit" className="hermes-send" disabled={!draft.trim() || !online} aria-label="Enviar">↑</button>}
        </div>

        {!online ? <p className="hermes-offline">
          {status ? hermesUnavailableLabel(status) : "Verificando o Hermes…"}
          {status?.hasCredentials && status.state === "offline"
            ? <button type="button" onClick={() => void hermes.connect().catch(() => undefined)}>RECONECTAR</button>
            : null}
        </p> : null}
      </form>
    </div>
  </div>;
}

export function HermesRailIcon() {
  return <Icon name="hermes" />;
}

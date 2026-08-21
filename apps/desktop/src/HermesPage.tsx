import { KeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  conversations as conversationApi,
  hermes,
  hermesUnavailableLabel,
  messageText,
  type ContextInput,
  type ContextOrigin,
  type Conversation,
  type ConversationSummary,
  type HermesStatus,
  type Message,
  type MessagePart,
  type ToolRunState,
  type TouchedEntity,
} from "./hermes";
import { Icon } from "./Icon";
import { DecryptedText } from "./motion/DecryptedText";
import { AgentActivity } from "./hermes/AgentActivity";
import { ConversationHeader } from "./hermes/ConversationHeader";
import { ConversationRail } from "./hermes/ConversationRail";
import { EmptyConversation, sugestoesDe } from "./hermes/EmptyConversation";
import { MessageTurn } from "./hermes/MessageTurn";
import { SmartComposer } from "./hermes/SmartComposer";
import { coladoNoFim } from "./hermes/composer";
import { decorridoDe } from "./hermes/atividade";
import type { Capture, Project, Task } from "./types";

/**
 * Hermes: conversa, workspace de agente e interface de comando na mesma tela.
 *
 * # O esqueleto: Marginalia
 *
 * Tudo que o sistema FAZ — buscar, ler, citar, executar — mora numa coluna
 * estreita à esquerda. Tudo que ele DIZ mora na coluna de leitura. Nunca se
 * misturam. É por isso que a atividade de ferramenta não empurra a prosa para
 * baixo enquanto a resposta chega, e é o que separa esta tela de um clone de
 * chatbot: aqui dá para ver o trabalho sem que ele atrapalhe a leitura.
 *
 * # O que o redesign de 2026-08-20 mudou, e por quê
 *
 * O esqueleto ficou. Dentro dele:
 *
 * - **A pergunta ganhou moldura.** Não a bolha do messenger — uma superfície um
 *   degrau acima do fundo, recuada à direita, com a largura do conteúdo. Numa
 *   thread longa, pergunta e resposta em prosa corrida se confundiam.
 * - **O efeito ganhou materialidade.** Proposta executada deixou de ser uma
 *   frase sobre algo invisível e passa a mostrar a entidade que a execução
 *   tocou, com botão de abrir. Os dados vêm do rastro de auditoria, nunca do
 *   texto: um card que aponta para o objeto errado é pior que card nenhum.
 * - **O composer virou o ponto de comando.** Duas linhas mesmo vazio, cresce
 *   até doze, `@` para mencionar, `/` para atalho, `+` para procurar. E o
 *   contexto anexado mudou de lugar: desceu da régua no topo para dentro dele,
 *   porque a pergunta que os chips respondem é sobre o que está prestes a ser
 *   enviado.
 * - **A atividade colapsa falando em fontes**, e não em ferramentas.
 *
 * # A superfície é uma só
 *
 * Até a auditoria de 2026-08-15 existiam DUAS: esta tela e o modo Hermes do
 * Command. As duas assinavam `hermes-event` no barramento global, então com as
 * duas abertas os deltas da mesma resposta se dividiam entre dois estados.
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

const HERMES_COMPACT_MEDIA = "(max-width: 1279px)";

function compactHermesViewport() {
  return typeof window !== "undefined" && window.matchMedia(HERMES_COMPACT_MEDIA).matches;
}

/** O erro cru continua disponível, mas deixa de disputar espaço com a ação que
 *  resolve o problema. A causa provável é inferida apenas para a frase de
 *  interface; o diagnóstico original permanece intacto no disclosure. */
function unavailablePresentation(status: HermesStatus | null) {
  if (!status) return { summary: "Verificando a conexão com o Hermes…", detail: "" };
  if (!status.hasCredentials) {
    return { summary: "Configure usuário e senha do Hermes em Settings.", detail: status.detail };
  }

  const detail = hermesUnavailableLabel(status);
  if (/429|rate.?limit|tentativas demais/i.test(detail)) {
    return { summary: "Muitas tentativas de conexão. Aguarde antes de tentar novamente.", detail };
  }
  if (/401|unauthorized|credencial|autentica|senha/i.test(detail)) {
    return { summary: "O Hermes recusou as credenciais configuradas.", detail };
  }
  if (/t[uú]nel|sending request|api\/status|unreachable|connection|conex[aã]o/i.test(detail)) {
    return { summary: "Hermes indisponível. Abra o túnel SSH e tente novamente.", detail };
  }
  return { summary: "Hermes indisponível. Verifique a conexão e tente novamente.", detail };
}

/**
 * Os contextos que uma mensagem registrou. É o que a edição restaura.
 *
 * Só os EXPLÍCITOS. O contexto automático — a tela aberta e a busca que o M/OS
 * fez sozinho — é recalculado a cada envio contra o estado de agora; restaurar
 * o de ontem anexaria à mão um resultado velho, e o chip passaria a dizer
 * "você anexou isto" sobre algo que o usuário nunca anexou.
 */
function contextsOf(message: Message): ContextInput[] {
  return message.parts
    .filter((part) => part.body.kind === "context_ref")
    .filter((part) => (part.body as { origin: ContextOrigin }).origin === "explicit")
    .map((part) => {
      const body = part.body as Extract<MessagePart["body"], { kind: "context_ref" }>;
      return { origin: body.origin, entity: body.entity, id: body.id, label: body.label };
    });
}



export function HermesPage({ inbox, projects, tasks, receipt, openProject, openResource, openTask }: {
  inbox: Capture[];
  projects: Project[];
  tasks: Task[];
  openProject?: (project: Project) => void;
  openResource?: (id: string) => void;
  openTask?: (id: string) => void;
  /** Mesma janela de recibo do resto do app. Tipada pela forma, e não importada
   *  de `App.tsx`, que já importa esta página. */
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
  const [listQuery, setListQuery] = useState("");
  const [compact, setCompact] = useState(compactHermesViewport);
  const [railOpen, setRailOpen] = useState(() => !compactHermesViewport());
  const [renaming, setRenaming] = useState(false);
  const [announcement, setAnnouncement] = useState("");
  const [pinnedBottom, setPinnedBottom] = useState(true);

  const field = useRef<HTMLTextAreaElement>(null);
  const thread = useRef<HTMLDivElement>(null);
  const surface = useRef<HTMLDivElement>(null);
  const rail = useRef<HTMLElement>(null);
  const railToggle = useRef<HTMLButtonElement>(null);
  const railClose = useRef<HTMLButtonElement>(null);
  const buffer = useRef<StreamBuffer>({ ...EMPTY_BUFFER });
  const frame = useRef(0);
  /** Até onde a resposta já foi anunciada. O leitor de tela recebe parágrafo
   *  concluído, nunca token. */
  const announced = useRef(0);

  const conversationId = conversation?.id ?? "";
  const running = stream !== null;
  const empty = !messages.length && !stream;
  const unavailable = unavailablePresentation(status);

  const closeRail = useCallback((restoreFocus = true) => {
    setRailOpen(false);
    if (restoreFocus) window.requestAnimationFrame(() => railToggle.current?.focus());
  }, []);

  const openRail = useCallback(() => {
    setRailOpen(true);
    window.requestAnimationFrame(() => railClose.current?.focus());
  }, []);

  useEffect(() => {
    const media = window.matchMedia(HERMES_COMPACT_MEDIA);
    const sync = () => {
      setCompact(media.matches);
      setRailOpen(!media.matches);
    };
    sync();
    media.addEventListener("change", sync);
    return () => media.removeEventListener("change", sync);
  }, []);

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
   * Reidrata da VPS quando o M/OS não tem a conversa, mas a sessão existe.
   *
   * O caso real é restaurar um backup anterior, ou abrir num M/OS que ainda não
   * tinha conversa local: o vínculo `hermes_session_id` sobrevive e o conteúdo
   * está lá. Com mensagens locais não se pede nada — elas são a verdade que a
   * tela desenha, e sobrescrevê-las com a projeção da VPS perderia as partes
   * que só o M/OS conhece, como o registro de contexto enviado.
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
    const measure = () => setPinnedBottom(coladoNoFim(node.scrollHeight, node.scrollTop, node.clientHeight));
    node.addEventListener("scroll", measure, { passive: true });
    return () => node.removeEventListener("scroll", measure);
  }, []);

  const suggestions = useMemo(() => sugestoesDe(inbox, projects, tasks), [inbox, projects, tasks]);
  const online = status?.state === "online" && status.sessionReady;

  const ask = useCallback(async (text: string, attached: ContextInput[]) => {
    const question = text.trim();
    if (!question || !conversationId) return;
    setDraft("");
    setContexts([]);
    setAnnouncement("Enviado.");
    await hermes.send(conversationId, question, attached).catch(() => setAnnouncement("Não foi possível enviar."));
  }, [conversationId]);

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

  function editarUltima() {
    const last = [...messages].reverse().find((message) => message.role === "user");
    if (last) edit(last);
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

  /**
   * Como abrir uma entidade que a ação tocou.
   *
   * Devolve `undefined` para o que esta tela não sabe abrir, e o card omite o
   * botão. Um "Abrir" que não abre nada é pior que a ausência dele.
   */
  const aoAbrir = useCallback((entity: TouchedEntity) => {
    if (entity.kind === "project" && openProject) {
      const project = projects.find((candidate) => candidate.id === entity.id);
      if (project) return () => openProject(project);
      return undefined;
    }
    if (entity.kind === "resource" && openResource) return () => openResource(entity.id);
    if (entity.kind === "task" && openTask) return () => openTask(entity.id);
    return undefined;
  }, [openProject, openResource, openTask, projects]);

  const newConversation = useCallback(async () => {
    const created = await conversationApi.create().catch(() => null);
    if (!created) return;
    setConversation(created);
    setMessages([]);
    setStream(null);
    buffer.current = { ...EMPTY_BUFFER };
    await hermes.selectConversation(created.id).catch(() => undefined);
    await reloadList();
    if (compact) setRailOpen(false);
    field.current?.focus();
  }, [compact, reloadList]);

  async function selectConversation(id: string) {
    if (id === conversationId) return;
    const opened = await conversationApi.messages(id).catch(() => null);
    if (opened === null) return;
    const summary = summaries.find((item) => item.id === id);
    setConversation((current) => (current ? { ...current, id, title: summary?.title ?? "" } : current));
    setMessages(opened);
    setStream(null);
    setPinnedBottom(true);
    buffer.current = { ...EMPTY_BUFFER };
    await hermes.selectConversation(id).catch(() => undefined);
    if (compact) setRailOpen(false);
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

  /** Atalhos da superfície. `Esc` para de qualquer foco dentro do Hermes —
   *  nunca depende de alcançar um botão. */
  useEffect(() => {
    function handler(event: globalThis.KeyboardEvent) {
      const node = surface.current;
      if (!node || !node.contains(document.activeElement)) return;

      if (compact && railOpen && event.key === "Tab") {
        const focusable = [...(rail.current?.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ) ?? [])];
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (first && last && event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
          return;
        }
        if (first && last && !event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
          return;
        }
      }

      if (event.key === "Escape") {
        if (compact && railOpen) { event.preventDefault(); closeRail(); return; }
        if (approval) { setApproval(null); void hermes.approve(false); event.preventDefault(); return; }
        // Limpar só o estado local deixava o agente bloqueado no `_block()` do
        // gateway com a caixa de resposta já fora da tela — pensando para
        // sempre, sem saída. Desistir precisa responder.
        if (clarify) { setClarify(null); void hermes.clarifyCancel(clarify.requestId); event.preventDefault(); return; }
        if (running) { void hermes.interrupt(); event.preventDefault(); }
        return;
      }
      if (event.ctrlKey && event.key.toLowerCase() === "n") { event.preventDefault(); void newConversation(); }
      if (event.ctrlKey && event.key === "/") {
        event.preventDefault();
        if (railOpen) closeRail();
        else openRail();
      }
    }
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [approval, clarify, closeRail, compact, newConversation, openRail, railOpen, running]);

  /**
   * Arquiva a conversa aberta e abre outra.
   *
   * Arquivar sem trocar de conversa deixaria a tela mostrando algo que a lista
   * ja nao contem — um fantasma que some no proximo recarregamento e leva junto
   * o que estivesse sendo escrito.
   */
  async function arquivarConversa() {
    if (!conversationId) return;
    await conversationApi.setArchived(conversationId, true).catch(() => undefined);
    await reloadList();
    const current = await conversationApi.current().catch(() => null);
    if (!current) return;
    setConversation(current);
    setMessages(await conversationApi.messages(current.id).catch(() => []));
    setStream(null);
    buffer.current = { ...EMPTY_BUFFER };
  }

  async function renameConversation(title: string) {
    if (!conversationId) return;
    const renamed = await conversationApi.rename(conversationId, title).catch(() => null);
    if (renamed) setConversation(renamed);
    setRenaming(false);
    await reloadList();
  }

  const elapsed = decorridoDe(stream?.startedAt ?? 0, Date.now());

  return <div className="hermes-page" data-rail={railOpen || undefined} ref={surface}>
    {railOpen ? (
      <ConversationRail
        ref={rail}
        summaries={summaries}
        atual={conversationId}
        busca={listQuery}
        setBusca={setListQuery}
        compacto={compact}
        onNova={() => void newConversation()}
        onAbrir={(id) => void selectConversation(id)}
        onExcluir={(id) => void removeConversation(id)}
        onFechar={() => closeRail()}
        fecharRef={railClose}
      />
    ) : null}
    {compact && railOpen ? (
      <button className="hermes-rail-backdrop" type="button" aria-label="Fechar conversas" onClick={() => closeRail()} />
    ) : null}

    <div className="hermes-main">
      <ConversationHeader
        ref={railToggle}
        conversation={conversation}
        status={status}
        railAberto={railOpen}
        renomeando={renaming}
        setRenomeando={setRenaming}
        onRenomear={(title) => void renameConversation(title)}
        onAbrirRail={openRail}
        onArquivar={() => void arquivarConversa()}
        onExcluir={() => void removeConversation(conversationId)}
      />

      {/* Parágrafo concluído, nunca token. `aria-live` sobre a thread inteira
          fazia o Narrator ler a resposta caractere a caractere. */}
      <p className="visually-hidden" role="status" aria-live="polite">{announcement}</p>

      <div className="hermes-thread-area" data-empty={empty || undefined} ref={thread}>
        <div className="hermes-thread">
          {empty ? (
            <EmptyConversation
              sugestoes={suggestions}
              online={Boolean(online)}
              onPerguntar={(texto) => void ask(texto, [])}
            />
          ) : null}

          {messages.map((message) => (
            <MessageTurn
              key={message.id}
              message={message}
              onCopy={copy}
              onEdit={edit}
              onRegenerate={(answer) => void regenerate(answer)}
              aoAbrir={aoAbrir}
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
              <AgentActivity passos={stream.tools} decorrido={elapsed} vivo />
              <div className="hermes-said">
                {stream.status.map((text, index) => (
                  <p className="hermes-system-line" key={index}><DecryptedText text={text} duration={260} /></p>
                ))}
                {stream.reasoning ? (
                  <details className="hermes-reasoning"><summary>Raciocínio</summary><p>{stream.reasoning}</p></details>
                ) : null}
                {/* Durante o streaming o texto é cru: o Markdown assenta uma
                    vez, no fim. Reparsear a cada quadro faria o bloco de código
                    piscar enquanto a cerca não fecha.

                    O bloco de proposta some do texto em curso. Ele chega token
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
            <span className="micro-label">HERMES PERGUNTA</span>
            <p>{clarify.question}</p>
            {clarify.choices.length ? <div className="hermes-ask-choices">
              {clarify.choices.map((choice) => (
                <button key={choice} type="button" onClick={() => { void hermes.clarify(clarify.requestId, choice); setClarify(null); }}>{choice}</button>
              ))}
            </div> : null}
            <form onSubmit={(event) => { event.preventDefault(); void hermes.clarify(clarify.requestId, clarifyAnswer); setClarify(null); }}>
              <input value={clarifyAnswer} onChange={(event) => setClarifyAnswer(event.currentTarget.value)} placeholder="responder" aria-label="Resposta ao Hermes" />
              <button type="submit">Responder</button>
            </form>
          </div> : null}

          {/* O composer segue livre durante a aprovação; Esc descarta. */}
          {approval ? <div className="hermes-ask" role="alertdialog" aria-label="Aprovação do Hermes">
            <span className="micro-label">PRECISA DA SUA APROVAÇÃO</span>
            <p>{approval}</p>
            <div className="hermes-ask-choices">
              <button type="button" onClick={() => { setApproval(null); void hermes.approve(false); }}>Negar</button>
              <button type="button" data-primary onClick={() => { setApproval(null); void hermes.approve(true); }}>Aprovar</button>
            </div>
          </div> : null}
        </div>
      </div>

      {!pinnedBottom ? (
        <button
          type="button"
          className="hermes-jump"
          onClick={() => { setPinnedBottom(true); const node = thread.current; if (node) node.scrollTop = node.scrollHeight; }}
        >
          <span aria-hidden="true">↓</span> Ir para o mais recente
        </button>
      ) : null}

      <SmartComposer
        draft={draft}
        setDraft={setDraft}
        contexts={contexts}
        setContexts={setContexts}
        running={running}
        online={Boolean(online)}
        status={status}
        campo={field}
        onSubmit={() => void ask(draft, contexts)}
        onInterrupt={() => void hermes.interrupt()}
        onEditarUltima={editarUltima}
        offlinePanel={!online ? (
          <div className="hermes-offline" role="status">
            <div className="hermes-offline-head">
              <span>{unavailable.summary}</span>
              {status?.hasCredentials && status.state === "offline"
                ? <button type="button" onClick={() => void hermes.connect().catch(() => undefined)}>Reconectar</button>
                : null}
            </div>
            {unavailable.detail && unavailable.detail !== unavailable.summary ? (
              <details className="hermes-offline-detail">
                <summary>Detalhes técnicos</summary>
                <code>{unavailable.detail}</code>
              </details>
            ) : null}
          </div>
        ) : null}
      />
    </div>
  </div>;
}

export function HermesRailIcon() {
  return <Icon name="hermes" />;
}

export type { KeyboardEvent };

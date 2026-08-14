import { FormEvent, KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { hermes, hermesUnavailableLabel, type HermesStatus } from "./hermes";
import { Icon } from "./Icon";
import { MosSymbol } from "./Symbol";
import type { Capture, Project, SearchItem, Task } from "./types";

/**
 * Hermes como tela inteira, conforme `M-OS Hermes - Chat Completo v0.1`.
 *
 * O desenho e mais largo que a Fase 1 acordada: ele mostra preview de acoes
 * com checkbox, agendamento e Issue no GitHub — tudo isso e escrita do Hermes
 * no M/OS, que foi explicitamente adiada, e agendamento nem existe no dominio.
 *
 * A tela e construida inteira. O que tem lastro funciona de verdade; o que nao
 * tem aparece como ausencia declarada, nunca como controle que finge. Um botao
 * "Criar Task" que nao cria e pior que a sua falta: ensina errado.
 */

type Turn = { question: string; answer: string; reasoning: string };

/** Os tres modos do desenho. So ASK tem lastro hoje — ver o cabecalho. */
const MODES = [
  { id: "ASK", hint: "ASK NUNCA ESCREVE", enabled: true },
  { id: "ACT", hint: "ACT ESCREVE COM PREVIEW", enabled: false },
  { id: "ORGANIZE", hint: "ORGANIZE DEVOLVE UM DIFF", enabled: false },
] as const;

type ModeId = (typeof MODES)[number]["id"];

function appendLast(turns: Turn[], field: "answer" | "reasoning", text: string): Turn[] {
  if (!turns.length) return turns;
  return turns.map((turn, index) => index === turns.length - 1 ? { ...turn, [field]: turn[field] + text } : turn);
}

/**
 * O estado de hoje, lido do que o M/OS ja sabe.
 *
 * Sao perguntas de verdade sobre dados de verdade — nao sugestoes decorativas.
 * Quando nao ha o que apontar, a lista fica vazia e a tela diz isso.
 */
function suggestionsFor(inbox: Capture[], projects: Project[], tasks: Task[]) {
  const suggestions: string[] = [];
  if (inbox.length) {
    const oldest = inbox.reduce((left, right) => left.capturedAt < right.capturedAt ? left : right);
    const since = new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "long" }).format(new Date(oldest.capturedAt));
    suggestions.push(`${inbox.length} ${inbox.length === 1 ? "item" : "itens"} na Inbox desde ${since} — o que faço com eles?`);
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

export function HermesPage({ inbox, projects, tasks, openProject, openResource }: {
  inbox: Capture[];
  projects: Project[];
  tasks: Task[];
  openProject: (project: Project) => void;
  openResource: (id: string) => void;
}) {
  const [status, setStatus] = useState<HermesStatus | null>(null);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [draft, setDraft] = useState("");
  const [running, setRunning] = useState(false);
  const [approval, setApproval] = useState<string | null>(null);
  const [mode, setMode] = useState<ModeId>("ASK");
  const [drawerOpen, setDrawerOpen] = useState(true);
  const [mentions, setMentions] = useState<SearchItem[]>([]);
  const [cited, setCited] = useState<{ id: string; label: string; kind: string }[]>([]);
  const field = useRef<HTMLInputElement>(null);
  const thread = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void hermes.status().then(setStatus).catch(() => undefined);
    const subscriptions = [
      hermes.onState(setStatus),
      hermes.onEvent((event) => {
        if (event.outcome === "delta") return setTurns((current) => appendLast(current, "answer", event.text));
        if (event.outcome === "reasoning") return setTurns((current) => appendLast(current, "reasoning", event.text));
        if (event.outcome === "complete") return setRunning(false);
        if (event.outcome === "approval") return setApproval(event.prompt);
        if (event.outcome === "failed") {
          setRunning(false);
          return setTurns((current) => appendLast(current, "answer", `\n${event.message}`));
        }
      }),
    ];
    return () => { subscriptions.forEach((subscription) => void subscription.then((dispose) => dispose())); };
  }, []);

  // Conexao preguicosa e de tentativa UNICA.
  //
  // O ref impede o loop: uma falha anuncia Offline, Offline muda a dependencia,
  // e sem guarda o efeito se redispararia sozinho — martelando o login, que o
  // gateway responde com 429 e tranca a conta. Reconectar depois de falhar e
  // acao explicita do usuario, no botao abaixo.
  const connectAttempted = useRef(false);
  useEffect(() => {
    if (connectAttempted.current) return;
    if (status?.state === "offline" && status.hasCredentials) {
      connectAttempted.current = true;
      void hermes.connect().catch(() => undefined);
    }
  }, [status?.state, status?.hasCredentials]);

  function reconnect() {
    connectAttempted.current = true;
    void hermes.connect().catch(() => undefined);
  }

  // A thread acompanha o texto chegando, sem roubar o scroll de quem subiu.
  useEffect(() => {
    const node = thread.current;
    if (!node) return;
    const atBottom = node.scrollHeight - node.scrollTop - node.clientHeight < 120;
    if (atBottom) node.scrollTop = node.scrollHeight;
  }, [turns]);

  /**
   * Mencao por `@`: busca de verdade no acervo local. E leitura, entao cabe na
   * fase atual — e e o que torna o campo util antes de o Hermes escrever.
   */
  useEffect(() => {
    const at = /@([\wÀ-ú]*)$/.exec(draft);
    if (!at) { setMentions([]); return; }
    const query = at[1];
    if (query.length < 2) { setMentions([]); return; }
    let cancelled = false;
    const timeout = window.setTimeout(() => {
      void api.search(query, false)
        .then((items) => { if (!cancelled) setMentions(items.filter((item) => item.kind === "project" || item.kind === "resource").slice(0, 4)); })
        .catch(() => setMentions([]));
    }, 120);
    return () => { cancelled = true; window.clearTimeout(timeout); };
  }, [draft]);

  const suggestions = useMemo(() => suggestionsFor(inbox, projects, tasks), [inbox, projects, tasks]);
  const online = status?.state === "online";

  function pickMention(item: SearchItem) {
    const name = item.kind === "project" ? item.project.name : item.kind === "resource" ? item.resource.title : "";
    setDraft((current) => current.replace(/@([\wÀ-ú]*)$/, `@${name} `));
    setMentions([]);
    field.current?.focus();
    if (item.kind === "project") setCited((current) => current.some((entry) => entry.id === item.project.id) ? current : [...current, { id: item.project.id, label: item.project.name, kind: "PROJ" }]);
    if (item.kind === "resource") setCited((current) => current.some((entry) => entry.id === item.resource.id) ? current : [...current, { id: item.resource.id, label: item.resource.title, kind: "RES" }]);
  }

  async function ask(text: string) {
    const question = text.trim();
    if (!question || running || !online) return;
    setTurns((current) => [...current, { question, answer: "", reasoning: "" }]);
    setDraft("");
    setRunning(true);
    try { await hermes.send(question); }
    catch (error) { setRunning(false); setTurns((current) => appendLast(current, "answer", String(error))); }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    void ask(draft);
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Tab") {
      event.preventDefault();
      // Tab troca de modo, como o desenho. So percorre os que tem lastro.
      const enabled = MODES.filter((entry) => entry.enabled);
      const next = enabled[(enabled.findIndex((entry) => entry.id === mode) + 1) % enabled.length];
      setMode(next.id);
      return;
    }
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void ask(draft);
    }
  }

  const hint = MODES.find((entry) => entry.id === mode)?.hint ?? "";

  return <div className="hermes-page">
    <div className="hermes-main">
      <div className="hermes-topline">
        <span className="micro-label">HERMES</span>
        <div className="hermes-topline-end">
          <button type="button" className="filter-label" data-active onClick={() => setDrawerOpen((open) => !open)}>{drawerOpen ? "PAINEL ✕" : "PAINEL"}</button>
          {/* A conversa nao e salva pelo M/OS: o historico vive na VPS e o que
              fica aqui e o que ela criar. Dizer isso e parte do desenho. */}
          <span className="micro-label">EFÊMERA · SESSÃO</span>
        </div>
      </div>

      <div className="hermes-scope">
        <span className="hermes-scope-label">VENDO</span>
        <span className="hermes-scope-empty">tudo — escopo por workspace e período entra com a busca do Hermes (fase 2)</span>
      </div>

      <div className="hermes-thread-area" ref={thread}>
        {!turns.length ? <div className="hermes-empty-thread">
          <span className="micro-label">THREAD VAZIA · O ESTADO DE HOJE</span>
          {suggestions.length ? <div className="hermes-suggestions">
            {suggestions.map((suggestion, index) => <button key={suggestion} type="button" className="hermes-suggestion" disabled={!online} onClick={() => void ask(suggestion)}>
              <span className="hermes-suggestion-index">{index + 1}</span>
              <span>{suggestion}</span>
            </button>)}
          </div> : <p className="hermes-quiet">Nada pedindo atenção agora. Inbox vazia, nenhum Project parado.</p>}
        </div> : null}

        {turns.map((turn, index) => <div className="hermes-turn" key={index}>
          <p className="hermes-question">{turn.question}</p>
          <div className="hermes-answer-row">
            {/* Filete de 2px em sodio: o marcador de autoria do sistema. */}
            <span className="hermes-filet" aria-hidden="true" />
            <div className="hermes-answer-body">
              <p className="hermes-answer">{turn.answer}</p>
              {turn.reasoning ? <details className="hermes-reasoning"><summary className="micro-label">RACIOCÍNIO</summary><p>{turn.reasoning}</p></details> : null}
            </div>
          </div>
        </div>)}

        {approval ? <div className="hermes-approval" role="alertdialog" aria-label="Aprovação do Hermes">
          <p>{approval}</p>
          <div className="hermes-approval-actions">
            <button type="button" className="button ghost" onClick={() => { setApproval(null); void hermes.approve(false); }}>Negar</button>
            <button type="button" className="button primary" onClick={() => { setApproval(null); void hermes.approve(true); }}>Aprovar</button>
          </div>
        </div> : null}
      </div>

      <form className="hermes-composer" onSubmit={submit}>
        {mentions.length ? <div className="hermes-mentions">
          {mentions.map((item) => {
            const name = item.kind === "project" ? item.project.name : item.kind === "resource" ? item.resource.title : "";
            const meta = item.kind === "project" ? "PROJECT" : "RESOURCE";
            return <button key={`${item.kind}-${name}`} type="button" className="hermes-mention" onClick={() => pickMention(item)}>
              <span className="hermes-mention-kind">{meta === "PROJECT" ? "PROJ" : "RES"}</span>
              <span className="hermes-mention-name">{name}</span>
              <span className="hermes-mention-meta">{meta}</span>
            </button>;
          })}
        </div> : null}

        <div className="hermes-modes">
          {MODES.map((entry) => <button key={entry.id} type="button" className="hermes-mode" data-active={entry.id === mode || undefined} disabled={!entry.enabled} title={entry.enabled ? entry.hint : "Escrita do Hermes no M/OS ainda não está ligada."} onClick={() => setMode(entry.id)}>{entry.id}</button>)}
          <span className="hermes-mode-rule" aria-hidden="true">|</span>
          <span className="hermes-mode-hint">TAB TROCA · {hint}</span>
          <span className="hermes-mode-spacer" />
          <span className="hermes-mode-hint">⇧↵ NOVA LINHA</span>
        </div>

        <div className="hermes-field">
          <span className="capture-bar" aria-hidden="true" />
          <div className="hermes-field-input">
            <input ref={field} value={draft} onChange={(event) => setDraft(event.currentTarget.value)} onKeyDown={onKeyDown} placeholder="Pergunte, mande fazer, ou jogue a Inbox aqui" aria-label="Perguntar ao Hermes" disabled={!online} />
            {!draft ? <span className="capture-caret" aria-hidden="true" /> : null}
          </div>
          {running
            ? <button type="button" className="button secondary" onClick={() => { setRunning(false); void hermes.interrupt(); }}>Cancelar</button>
            : <button type="submit" className="button primary" disabled={!draft.trim() || !online}>Enviar ⏎</button>}
        </div>

        {!online ? <div className="hermes-offline-row">
          <p className="hermes-offline">{status ? hermesUnavailableLabel(status) : "Verificando o Hermes…"}</p>
          {status?.hasCredentials && status.state === "offline" ? <button type="button" className="button outline sm" onClick={reconnect}>Reconectar</button> : null}
        </div> : null}
        {running ? <div className="hermes-running"><MosSymbol size={16} spinning /><span className="micro-label">PENSANDO</span></div> : null}
      </form>
    </div>

    {drawerOpen ? <aside className="hermes-drawer">
      <div className="hermes-drawer-head">
        <span className="micro-label">CONTEXTO</span>
        <button type="button" className="filter-label" onClick={() => setDrawerOpen(false)}>✕</button>
      </div>
      <div className="hermes-drawer-body">
        <span className="micro-label">FONTES CITADAS NESTA THREAD</span>
        {cited.length ? <div className="row-list hermes-cited">
          {cited.map((entry) => <button key={entry.id} type="button" className="data-row" onClick={() => { const project = projects.find((candidate) => candidate.id === entry.id); if (project) openProject(project); else openResource(entry.id); }}>
            <span className="hermes-cited-kind">{entry.kind}</span>
            <span className="row-copy"><strong>{entry.label}</strong></span>
          </button>)}
        </div> : <p className="hermes-quiet">Nada citado ainda. Mencione com <b>@</b> para trazer um Project ou Resource para a conversa.</p>}

        <span className="micro-label hermes-drawer-section">THREAD</span>
        <p className="hermes-quiet">Esta conversa não é salva pelo M/OS. O histórico vive na sua VPS, e o que fica aqui é o que ela criar.</p>

        <span className="micro-label hermes-drawer-section">CRIADO NESTA SESSÃO</span>
        {/* Nada aparece aqui porque o Hermes ainda nao escreve no M/OS. Quando
            escrever, e nesta lista que a criacao fica visivel. */}
        <p className="hermes-quiet">Nada ainda. O Hermes lê o M/OS, mas ainda não escreve nele — criar Task, agendar e abrir Issue são as próximas fases.</p>
      </div>
    </aside> : null}
  </div>;
}

export function HermesRailIcon() {
  return <Icon name="hermes" />;
}

import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { CardGravacao } from "./CardGravacao";
import { Icon } from "./Icon";
import { selecaoAoFocar } from "./meetingsSync";
import {
  aguardandoOutros, doTipo, duracao, exigeAtencao, fraseDoCorte, jaCriados, linhaDeContagem,
  relogio, rotuloDaFase, secaoDa, seloDoCartao,
} from "./reuniao";
import {
  DialogoApagar, DialogoCorte, DialogoFollowUp, ListaDeItens, Lixeira, ProgressoDaReuniao,
  RevisaoEmLote, Transcricao, nomeDoProject,
} from "./ReuniaoPartes";
import { ActionMenu, EmptyState, Inspector, PageHeader, PaneHeader, Panel, StateMessage } from "./Surface";
import type {
  Meeting, MeetingAnalysis, MeetingBookmark, MeetingInsight, MeetingOverview,
  MeetingProgressEvent, Project, ProjectInference, TranscriptSegment,
} from "./types";

/**
 * Reuniões: "Você participa da reunião. O M/OS cuida do resto."
 *
 * A V1 era um pipeline com botões — Transcrever, esperar, Analisar, esperar,
 * abrir item por item. A V2 mostra só o que a pessoa precisa: a reunião
 * acontecendo, a reunião sendo organizada, a reunião pronta, e o que exige
 * atenção. Transcrever e analisar deixaram de ser gestos.
 *
 * A página de uma reunião pronta prioriza RESULTADO, nesta ordem:
 *
 * ```
 * Reunião com equipe estrutural
 * Hoje · 48 min · 167-25
 *
 * O que exige sua atenção      ← a revisão em lote
 * Decisões
 * Aguardando outras pessoas
 * Perguntas em aberto
 * Riscos e referências
 * Notas
 * Transcrição
 * ```
 *
 * Spec: `docs/superpowers/specs/2026-09-16-meeting-agent-v2-design.md` §9.
 */

type Receipt = (action: { message: string; run: () => Promise<unknown> }) => void;
type Visao = "resumo" | "transcricao" | "notas";
type Lista = "reunioes" | "lixeira";

const CHAVE_CORTE_DISPENSADO = "mos.reuniao.corte-dispensado";

function lerDispensados(): string[] {
  try { return JSON.parse(localStorage.getItem(CHAVE_CORTE_DISPENSADO) ?? "[]"); } catch { return []; }
}

function quandoFoi(iso: string): string {
  const data = new Date(iso);
  const hoje = new Date();
  const mesmoDia = (a: Date, b: Date) => a.toDateString() === b.toDateString();
  const ontem = new Date(hoje.getFullYear(), hoje.getMonth(), hoje.getDate() - 1);
  const hora = data.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
  if (mesmoDia(data, hoje)) return `Hoje, ${hora}`;
  if (mesmoDia(data, ontem)) return `Ontem, ${hora}`;
  return `${data.toLocaleDateString("pt-BR", { day: "2-digit", month: "short" })}, ${hora}`;
}

export function MeetingsPage({ projects, focus, receipt, refresh, perguntarAoHermes, consentimentoPedido }: {
  projects: Project[];
  /** Muda quando o atalho global pediu o consentimento da primeira gravação. */
  consentimentoPedido?: number;
  /** Abre direto numa reunião — barra de gravação, Home, notificação. */
  focus?: string | null;
  receipt: Receipt;
  refresh: () => Promise<unknown>;
  perguntarAoHermes: (meeting: Meeting) => void;
}) {
  const [linhas, setLinhas] = useState<MeetingOverview[]>([]);
  const [lixeira, setLixeira] = useState<Meeting[]>([]);
  const [lista, setLista] = useState<Lista>("reunioes");
  const [busca, setBusca] = useState("");
  const [mostrarArquivadas, setMostrarArquivadas] = useState(false);
  const [escolhida, setEscolhida] = useState<string | null>(focus ?? null);
  const [painelEstreito, setPainelEstreito] = useState<"list" | "detail">(focus ? "detail" : "list");
  const [carregando, setCarregando] = useState(true);
  const [nota, setNota] = useState("");
  const [flash, setFlash] = useState("");

  const [visao, setVisao] = useState<Visao>("resumo");
  const [trechos, setTrechos] = useState<TranscriptSegment[]>([]);
  const [itens, setItens] = useState<MeetingInsight[]>([]);
  const [analise, setAnalise] = useState<MeetingAnalysis | null>(null);
  const [marcas, setMarcas] = useState<MeetingBookmark[]>([]);
  const [sugestaoDeCorte, setSugestaoDeCorte] = useState<number | null>(null);
  const [sugestaoDeProject, setSugestaoDeProject] = useState<ProjectInference | null>(null);
  const [alvo, setAlvo] = useState<string | null>(null);
  const [progresso, setProgresso] = useState<Record<string, number>>({});
  const [gravandoId, setGravandoId] = useState<string | null>(null);

  const [renomeando, setRenomeando] = useState(false);
  const [rascunhoTitulo, setRascunhoTitulo] = useState("");
  const [notasEditadas, setNotasEditadas] = useState<string | null>(null);
  const [apagando, setApagando] = useState<Meeting | null>(null);
  const [cortando, setCortando] = useState<Meeting | null>(null);
  const [followUp, setFollowUp] = useState<string | null>(null);
  const [pedirConsentimento, setPedirConsentimento] = useState(false);
  const [menuDeContexto, setMenuDeContexto] = useState<{ meeting: Meeting; x: number; y: number } | null>(null);
  const [dispensados, setDispensados] = useState<string[]>(lerDispensados);
  const [tecnico, setTecnico] = useState<Record<string, unknown> | null>(null);

  const carregarLista = useCallback(async () => {
    try {
      const [visao, lixo, tick] = await Promise.all([
        api.meetingOverview(mostrarArquivadas),
        api.meetingTrashed(),
        api.meetingRecording(),
      ]);
      setLinhas(visao);
      setLixeira(lixo);
      setGravandoId(tick?.meetingId ?? null);
    } catch (erro) {
      setNota(erro instanceof Error ? erro.message : String(erro));
    } finally {
      setCarregando(false);
    }
  }, [mostrarArquivadas]);

  useEffect(() => { void carregarLista(); }, [carregarLista]);

  useEffect(() => {
    if (consentimentoPedido) setPedirConsentimento(true);
  }, [consentimentoPedido]);

  useEffect(() => {
    setEscolhida((atual) => selecaoAoFocar(focus, atual));
    if (focus) { setPainelEstreito("detail"); setLista("reunioes"); }
  }, [focus]);

  const linha = useMemo(() => linhas.find((l) => l.meeting.id === escolhida) ?? null, [linhas, escolhida]);
  const escolhidaNaLixeira = useMemo(() => lixeira.find((m) => m.id === escolhida) ?? null, [lixeira, escolhida]);

  const carregarDetalhe = useCallback(async () => {
    if (!escolhida) { setTrechos([]); setItens([]); setAnalise(null); setMarcas([]); return; }
    try {
      const [transcricao, todos, resumo, momentos, corte, project] = await Promise.all([
        api.meetingTranscript(escolhida),
        api.meetingInsights(escolhida),
        api.meetingAnalysis(escolhida),
        api.meetingBookmarks(escolhida),
        api.meetingTrimSuggestion(escolhida).catch(() => null),
        api.meetingProjectSuggestion(escolhida).catch(() => null),
      ]);
      setTrechos(transcricao);
      setItens(todos);
      setAnalise(resumo);
      setMarcas(momentos);
      setSugestaoDeCorte(corte);
      setSugestaoDeProject(project && project.confidence !== "low" ? project : null);
    } catch {
      /* Apagada por outro caminho: a lista recarrega e a seleção some. */
    }
  }, [escolhida]);

  useEffect(() => { void carregarDetalhe(); setNotasEditadas(null); setTecnico(null); }, [carregarDetalhe]);

  const recarregar = useCallback(() => { void carregarLista(); void carregarDetalhe(); }, [carregarLista, carregarDetalhe]);

  useEffect(() => {
    const eventos = [
      "meeting-started", "meeting-stopped", "meeting-transcribed", "meeting-analyzed",
      "meeting-failed", "meeting-waiting", "meeting-ready", "meeting-trashed", "meeting-deleted",
    ];
    const offs = eventos.map((nome) => listen(nome, () => recarregar()));
    offs.push(listen<MeetingProgressEvent>("meeting-progress", (evento) => {
      // O whisper reporta muitas vezes por segundo. A página só re-renderiza
      // quando o NÚMERO que ela mostra muda.
      setProgresso((atual) => {
        const id = evento.payload.meetingId;
        const antes = atual[id];
        if (antes != null && Math.round(antes * 100) === Math.round(evento.payload.overall * 100)) return atual;
        return { ...atual, [id]: evento.payload.overall };
      });
    }));
    offs.push(listen<string>("data-changed", (evento) => {
      if (String(evento.payload).startsWith("meeting")) recarregar();
    }));
    offs.push(listen("meeting-consent-needed", () => setPedirConsentimento(true)));
    return () => { offs.forEach((off) => void off.then((fn) => fn())); };
  }, [recarregar]);

  const mostrarFlash = (texto: string) => {
    setFlash(texto);
    window.setTimeout(() => setFlash(""), 4000);
  };

  const agir = async (acao: () => Promise<unknown>, mensagem?: string) => {
    setNota("");
    try {
      await acao();
      recarregar();
      if (mensagem) mostrarFlash(mensagem);
    } catch (erro) {
      setNota(erro instanceof Error ? erro.message : String(erro));
    }
  };

  const iniciar = useCallback(async () => {
    setNota("");
    try {
      const consentimento = await api.meetingAnalysisConsent();
      if (!consentimento.granted) { setPedirConsentimento(true); return; }
      const meeting = await api.meetingStart("", null);
      setEscolhida(meeting.id);
      setPainelEstreito("detail");
      recarregar();
    } catch (erro) {
      setNota(erro instanceof Error ? erro.message : String(erro));
    }
  }, [recarregar]);

  const apagar = async (meeting: Meeting, pararAntes: boolean) => {
    setApagando(null);
    try {
      await api.meetingTrash(meeting.id, pararAntes);
      if (escolhida === meeting.id) { setEscolhida(null); setPainelEstreito("list"); }
      receipt({
        message: `“${meeting.title}” foi para a lixeira`,
        run: () => api.meetingRestore(meeting.id).then(() => recarregar()),
      });
      recarregar();
      await refresh();
    } catch (erro) {
      setNota(erro instanceof Error ? erro.message : String(erro));
    }
  };

  const saltar = (segmentId: string) => {
    setVisao("transcricao");
    setAlvo(null);
    window.setTimeout(() => setAlvo(segmentId), 30);
  };

  const filtradas = useMemo(() => {
    const agulha = busca.trim().toLowerCase();
    if (!agulha) return linhas;
    return linhas.filter((l) => l.meeting.title.toLowerCase().includes(agulha)
      || (nomeDoProject(projects, l.meeting.projectId) ?? "").toLowerCase().includes(agulha));
  }, [linhas, busca, projects]);

  const secoes = useMemo(() => ({
    em_andamento: filtradas.filter((l) => secaoDa(l) === "em_andamento"),
    atencao: filtradas.filter((l) => secaoDa(l) === "atencao"),
    recentes: filtradas.filter((l) => secaoDa(l) === "recentes"),
  }), [filtradas]);

  // Memoizados: a revisão guarda rascunho por item, e um array novo a cada
  // render faria o rascunho reiniciar.
  const minhas = useMemo(() => exigeAtencao(itens), [itens]);
  const deOutros = useMemo(() => aguardandoOutros(itens), [itens]);
  const criados = useMemo(() => jaCriados(itens), [itens]);

  const meeting = linha?.meeting ?? null;
  const gravandoEsta = meeting ? meeting.status === "recording" || meeting.status === "paused" : false;
  const dispensouCorte = meeting ? dispensados.includes(meeting.id) : true;

  // Funções de desenho, e não componentes: um componente declarado dentro do
  // render nasce com identidade nova a cada render, e o React remontaria os
  // cartões — perdendo o foco de quem navega pelo teclado.
  const cartao = (l: MeetingOverview) => {
    const contagem = linhaDeContagem(l);
    const project = nomeDoProject(projects, l.meeting.projectId);
    return (
      <button
        key={l.meeting.id}
        type="button"
        className="reuniao-cartao"
        aria-current={l.meeting.id === escolhida ? "true" : undefined}
        data-fase={l.phase}
        onClick={() => { setEscolhida(l.meeting.id); setVisao("resumo"); setPainelEstreito("detail"); }}
        onContextMenu={(evento) => {
          evento.preventDefault();
          setMenuDeContexto({ meeting: l.meeting, x: evento.clientX, y: evento.clientY });
        }}
      >
        <span className="reuniao-cartao-titulo">{l.meeting.title}</span>
        <span className="reuniao-cartao-meta">
          {quandoFoi(l.meeting.startedAt)} · {duracao(l.meeting.durationMs)}{project ? ` · ${project}` : ""}
        </span>
        {contagem ? <span className="reuniao-cartao-contagem">{contagem}</span> : null}
        <span className="reuniao-cartao-selo" data-fase={l.phase}>
          {l.phase === "recording" ? "● Gravando" : seloDoCartao(l, progresso[l.meeting.id])}
        </span>
      </button>
    );
  };

  const secao = (rotulo: string, grupo: MeetingOverview[]) => grupo.length ? (
    <div className="meeting-group">
      <span className="micro-label">{rotulo}</span>
      {grupo.map(cartao)}
    </div>
  ) : null;

  return (
    <div className="page meetings-page">
      <PageHeader
        title="Reuniões"
        subtitle="Você participa da reunião. O M/OS cuida do resto."
        actions={gravandoId
          ? <Button variant="outline" onClick={() => { setEscolhida(gravandoId); setPainelEstreito("detail"); }}>Abrir gravação</Button>
          : <Button variant="primary" onClick={() => void iniciar()} title="Ctrl+Alt+M">Iniciar reunião</Button>}
      />

      {nota ? <StateMessage state="error" label="Não foi possível concluir" detail={nota} /> : null}
      {flash ? <StateMessage state="saved" label={flash} /> : null}

      <div className="split-page inspector-page meetings-split">
        <section className="list-pane">
          <PaneHeader segments={["Reuniões"]} meta={lista === "lixeira" ? `${lixeira.length} na lixeira` : `${linhas.length}`} />
          <div className="reuniao-lista-topo">
            <input
              className="reuniao-busca"
              value={busca}
              onChange={(evento) => setBusca(evento.currentTarget.value)}
              placeholder="Buscar reuniões"
              aria-label="Buscar reuniões"
            />
            <div className="segmented" role="tablist" aria-label="Qual lista">
              <button role="tab" aria-selected={lista === "reunioes"} onClick={() => setLista("reunioes")}>Reuniões</button>
              <button role="tab" aria-selected={lista === "lixeira"} onClick={() => setLista("lixeira")}>
                Lixeira{lixeira.length ? ` · ${lixeira.length}` : ""}
              </button>
            </div>
          </div>

          {lista === "lixeira" ? (
            <Lixeira
              reunioes={lixeira}
              restaurar={(m) => void agir(() => api.meetingRestore(m.id), `“${m.title}” restaurada`)}
              apagarDeVez={(m) => void agir(() => api.meetingDelete(m.id), "Apagada de vez")}
              esvaziar={() => void agir(() => api.meetingEmptyTrash(), "Lixeira esvaziada")}
            />
          ) : carregando && linhas.length === 0 ? (
            <div className="meeting-carregando" role="status">
              <span className="processing-trilha" data-indeterminado><span className="processing-preenchimento" /></span>
              <span className="micro-label">CARREGANDO AS REUNIÕES</span>
            </div>
          ) : linhas.length === 0 ? (
            <EmptyState>
              Nenhuma reunião ainda. Quando você entrar numa chamada, o M/OS oferece gravar — ou use Iniciar reunião.
            </EmptyState>
          ) : (
            <div className="meeting-groups">
              {secao("EM ANDAMENTO", secoes.em_andamento)}
              {secao("PRECISA DE ATENÇÃO", secoes.atencao)}
              {secao("RECENTES", secoes.recentes)}
              <label className="meeting-arquivadas">
                <input type="checkbox" checked={mostrarArquivadas} onChange={(evento) => setMostrarArquivadas(evento.currentTarget.checked)} />
                <span className="micro-label">MOSTRAR ARQUIVADAS</span>
              </label>
            </div>
          )}
        </section>

        <Inspector
          label="Reunião"
          open={painelEstreito === "detail"}
          onBack={() => setPainelEstreito("list")}
          onEscape={() => setPainelEstreito("list")}
        >
          {escolhidaNaLixeira && !meeting ? (
            <div className="meeting-detail">
              <h2>{escolhidaNaLixeira.title}</h2>
              <p className="support-copy">Esta reunião está na lixeira.</p>
              <Button onClick={() => void agir(() => api.meetingRestore(escolhidaNaLixeira.id), "Reunião restaurada")}>Restaurar</Button>
            </div>
          ) : !linha || !meeting ? (
            <EmptyState>Escolha uma reunião.</EmptyState>
          ) : (
            <div className="meeting-detail">
              <header className="meeting-head">
                <div className="meeting-head-line">
                  {renomeando ? (
                    <input
                      className="meeting-title-input"
                      aria-label="Nome da reunião"
                      autoFocus
                      value={rascunhoTitulo}
                      onChange={(evento) => setRascunhoTitulo(evento.currentTarget.value)}
                      onBlur={() => {
                        setRenomeando(false);
                        const limpo = rascunhoTitulo.trim();
                        if (limpo && limpo !== meeting.title) void agir(() => api.meetingSetTitle(meeting.id, limpo));
                      }}
                      onKeyDown={(evento) => {
                        if (evento.key === "Enter") (evento.currentTarget as HTMLInputElement).blur();
                        if (evento.key === "Escape") { evento.preventDefault(); setRenomeando(false); }
                      }}
                    />
                  ) : <h2>{meeting.title}</h2>}
                  <ActionMenu
                    trigger={<Icon name="more" />}
                    label="Ações da reunião"
                    items={[
                      { label: "Renomear", onSelect: () => { setRascunhoTitulo(meeting.title); setRenomeando(true); } },
                      {
                        label: "Ajustar início e fim",
                        disabled: gravandoEsta || meeting.durationMs < 2000 || meeting.status === "transcribing" || meeting.status === "analyzing",
                        onSelect: () => setCortando(meeting),
                      },
                      {
                        label: "Preparar follow-up",
                        disabled: !analise && itens.length === 0,
                        onSelect: () => void api.meetingFollowUp(meeting.id).then(setFollowUp).catch((e) => setNota(String(e))),
                      },
                      { label: "Perguntar ao Hermes", disabled: trechos.length === 0, onSelect: () => perguntarAoHermes(meeting) },
                      {
                        label: meeting.lifecycleState === "archived" ? "Desarquivar" : "Arquivar",
                        onSelect: () => void agir(
                          () => api.meetingSetArchived(meeting.id, meeting.lifecycleState !== "archived"),
                          meeting.lifecycleState === "archived" ? "Reunião desarquivada" : "Reunião arquivada",
                        ),
                      },
                      { label: "Apagar reunião", danger: true, onSelect: () => setApagando(meeting) },
                    ]}
                  />
                </div>
                <p className="meeting-head-meta">
                  {quandoFoi(meeting.startedAt)} · {duracao(meeting.durationMs)}
                  {meeting.lifecycleState === "archived" ? " · arquivada" : ""}
                  {linha.phase !== "ready" && linha.phase !== "recording" ? ` · ${rotuloDaFase(linha.phase)}` : ""}
                </p>
                <label className="meeting-field meeting-head-project">
                  <span className="micro-label">PROJECT</span>
                  <select
                    value={meeting.projectId ?? ""}
                    onChange={(evento) => {
                      const valor = evento.currentTarget.value;
                      void agir(() => api.meetingSetProject(meeting.id, valor || null));
                    }}
                  >
                    <option value="">Sem Project</option>
                    {projects
                      .filter((p) => p.lifecycleState === "active" || p.id === meeting.projectId)
                      .map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
                  </select>
                </label>
              </header>

              {sugestaoDeProject && !meeting.projectId ? (
                <div className="reuniao-aviso">
                  <p>Parece ser do Project <b>{nomeDoProject(projects, sugestaoDeProject.projectId)}</b>.</p>
                  <Button variant="ghost" size="sm" onClick={() => void agir(() => api.meetingSetProject(meeting.id, sugestaoDeProject.projectId), "Project associado")}>Associar</Button>
                </div>
              ) : null}

              {gravandoEsta ? (
                <CardGravacao
                  meeting={meeting}
                  onMudou={() => recarregar()}
                  onEncerrou={(m) => { setEscolhida(m.id); recarregar(); }}
                />
              ) : null}

              {linha.phase === "processing" || linha.phase === "finalizing" ? (
                <ProgressoDaReuniao linha={linha} fracaoAoVivo={progresso[meeting.id] ?? null} />
              ) : null}

              {linha.phase === "needs_attention" || linha.phase === "failed_recoverable" || linha.phase === "recovered" ? (
                <div className="reuniao-aviso" data-tom="atencao">
                  <p>
                    {linha.phase === "failed_recoverable" || meeting.status === "failed" ? "A gravação está segura. " : ""}
                    {linha.attention || (linha.phase === "recovered"
                      ? `O M/OS fechou durante a gravação. ${duracao(meeting.durationMs)} foram recuperados.`
                      : "Esta reunião precisa de você.")}
                  </p>
                  {meeting.durationMs > 0 ? (
                    <Button size="sm" onClick={() => void agir(() => api.meetingRetry(meeting.id), "Processando de novo")}>
                      {linha.job?.lastErrorCode === "manual_start" ? "Processar agora" : "Tentar de novo"}
                    </Button>
                  ) : (
                    <Button variant="ghost" size="sm" onClick={() => setApagando(meeting)}>Apagar</Button>
                  )}
                </div>
              ) : null}

              {linha.phase === "partially_ready" ? (
                <div className="reuniao-aviso">
                  <p>
                    <b>Transcrição pronta.</b>{" "}
                    {linha.job?.lastErrorCode === "consent_missing"
                      ? "A organização com o Hermes espera a autorização em Settings."
                      : trechos.length === 0
                        ? "Nenhuma fala foi encontrada nesta gravação."
                        : "Organização inteligente pendente — tenta de novo sozinha."}
                  </p>
                  {trechos.length > 0 && linha.job?.lastErrorCode !== "consent_missing" ? (
                    <Button variant="ghost" size="sm" onClick={() => void agir(() => api.meetingRetry(meeting.id), "Tentando de novo")}>Tentar agora</Button>
                  ) : null}
                </div>
              ) : null}

              {meeting.trimOrigin === "auto" && meeting.trimEndMs != null ? (
                <div className="reuniao-aviso">
                  <p>Ignoramos {duracao(meeting.durationMs - meeting.trimEndMs)} depois do fim da reunião.</p>
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={!!meeting.audioDeletedAt}
                    onClick={() => void agir(() => api.meetingClearTrim(meeting.id), "O trecho volta a contar")}
                  >Incluir de volta</Button>
                </div>
              ) : sugestaoDeCorte != null && !dispensouCorte && (linha.phase === "ready" || linha.phase === "partially_ready") ? (
                <div className="reuniao-aviso">
                  <p>{fraseDoCorte(meeting.durationMs, sugestaoDeCorte)} Ignorar esse trecho?</p>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void agir(() => api.meetingSetTrim(meeting.id, 0, sugestaoDeCorte, "suggested"), "Trecho ignorado")}
                  >Ignorar</Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      const proximos = [...dispensados, meeting.id].slice(-200);
                      setDispensados(proximos);
                      try { localStorage.setItem(CHAVE_CORTE_DISPENSADO, JSON.stringify(proximos)); } catch { /* sem storage */ }
                    }}
                  >Manter</Button>
                </div>
              ) : null}

              {!gravandoEsta ? (
                <div className="segmented" role="tablist" aria-label="Visão da reunião">
                  <button role="tab" aria-selected={visao === "resumo"} onClick={() => setVisao("resumo")}>Resumo</button>
                  <button role="tab" aria-selected={visao === "transcricao"} onClick={() => setVisao("transcricao")}>
                    Transcrição{trechos.length ? ` · ${trechos.length}` : ""}
                  </button>
                  <button role="tab" aria-selected={visao === "notas"} onClick={() => setVisao("notas")}>
                    Notas{meeting.notes.trim() ? " ·" : ""}
                  </button>
                </div>
              ) : null}

              {gravandoEsta ? null : visao === "resumo" ? (
                <div className="meeting-overview">
                  <RevisaoEmLote
                    titulo={minhas.length === 1 ? "1 ITEM PRECISA DA SUA ATENÇÃO" : `O QUE EXIGE SUA ATENÇÃO`}
                    linhas={minhas}
                    trechos={trechos}
                    projectId={meeting.projectId}
                    saltar={saltar}
                    receipt={receipt}
                    depois={() => { recarregar(); void refresh(); }}
                  />

                  {analise?.summary ? (
                    <Panel label="RESUMO">
                      <p className="meeting-summary">{analise.summary}</p>
                      {analise.windows > 1 ? (
                        <p className="meeting-windows">A transcrição foi organizada em {analise.windows} partes.</p>
                      ) : null}
                    </Panel>
                  ) : null}

                  <ListaDeItens titulo="DECISÕES" itens={doTipo(itens, ["decision"])} trechos={trechos} saltar={saltar} depois={recarregar} />

                  <RevisaoEmLote
                    titulo="AGUARDANDO OUTRAS PESSOAS"
                    linhas={deOutros}
                    trechos={trechos}
                    projectId={meeting.projectId}
                    saltar={saltar}
                    receipt={receipt}
                    depois={() => { recarregar(); void refresh(); }}
                    externa
                  />

                  <ListaDeItens titulo="PERGUNTAS EM ABERTO" itens={doTipo(itens, ["open_question"])} trechos={trechos} saltar={saltar} depois={recarregar} />
                  <ListaDeItens titulo="RISCOS" itens={doTipo(itens, ["risk"])} trechos={trechos} saltar={saltar} depois={recarregar} />
                  <ListaDeItens titulo="REFERÊNCIAS" itens={doTipo(itens, ["reference"])} trechos={trechos} saltar={saltar} depois={recarregar} />
                  <ListaDeItens titulo="TÓPICOS" itens={doTipo(itens, ["topic"])} trechos={trechos} saltar={saltar} depois={recarregar} />

                  {criados.length ? (
                    <Panel label="JÁ VIRARAM TASK" count={String(criados.length)}>
                      <ul className="reuniao-itens">
                        {criados.map((item) => <li key={item.id}><span className="reuniao-item-texto">{item.text}</span></li>)}
                      </ul>
                    </Panel>
                  ) : null}

                  {marcas.length ? (
                    <Panel label="MOMENTOS MARCADOS" count={String(marcas.length)}>
                      <div className="reuniao-marcas">
                        {marcas.map((marca) => (
                          <span key={marca.id} className="reuniao-marca">
                            <button
                              type="button"
                              onClick={() => {
                                const trecho = trechos.find((t) => marca.atMs >= t.startMs - 5000 && marca.atMs <= t.endMs + 5000);
                                if (trecho) saltar(trecho.id); else setVisao("transcricao");
                              }}
                            >★ {relogio(marca.atMs)}</button>
                            <button type="button" aria-label="Remover marca" onClick={() => void api.meetingDeleteBookmark(marca.id).then(recarregar)}>×</button>
                          </span>
                        ))}
                      </div>
                    </Panel>
                  ) : null}

                  {!analise && itens.length === 0 && linha.phase === "ready" ? (
                    <EmptyState>Nenhuma decisão ou tarefa foi identificada nesta reunião.</EmptyState>
                  ) : null}

                  {import.meta.env.DEV ? (
                    <details
                      className="reuniao-tecnico"
                      onToggle={(evento) => {
                        if ((evento.currentTarget as HTMLDetailsElement).open) void api.meetingDebug(meeting.id).then(setTecnico).catch(() => undefined);
                      }}
                    >
                      <summary className="micro-label">TÉCNICO (DESENVOLVIMENTO)</summary>
                      <pre>{tecnico ? JSON.stringify(tecnico, null, 2) : "…"}</pre>
                    </details>
                  ) : null}
                </div>
              ) : visao === "transcricao" ? (
                <Transcricao
                  meeting={meeting}
                  trechos={trechos}
                  marcas={marcas}
                  itens={itens}
                  alvo={alvo}
                  audioDisponivel={!meeting.audioDeletedAt}
                  depois={recarregar}
                />
              ) : (
                <div className="meeting-overview">
                  <textarea
                    className="card-gravacao-notas"
                    aria-label="Notas da reunião"
                    value={notasEditadas ?? meeting.notes}
                    placeholder="Nada foi anotado. !task, !decision e !question viram itens."
                    onChange={(evento) => setNotasEditadas(evento.currentTarget.value)}
                    onBlur={() => {
                      if (notasEditadas != null && notasEditadas !== meeting.notes) {
                        void agir(() => api.meetingSetNotes(meeting.id, notasEditadas));
                      }
                    }}
                  />
                  <p className="support-copy">Linhas com !task, !decision ou !question aparecem no Resumo como itens escritos por você.</p>
                </div>
              )}
            </div>
          )}
        </Inspector>
      </div>

      {menuDeContexto ? (
        <div className="reuniao-contexto-scrim" onClick={() => setMenuDeContexto(null)} onContextMenu={(e) => { e.preventDefault(); setMenuDeContexto(null); }}>
          <div className="reuniao-contexto" role="menu" style={{ left: menuDeContexto.x, top: menuDeContexto.y }}>
            <button role="menuitem" type="button" onClick={() => { setEscolhida(menuDeContexto.meeting.id); setPainelEstreito("detail"); setMenuDeContexto(null); }}>Abrir</button>
            <button role="menuitem" type="button" data-perigo onClick={() => { setApagando(menuDeContexto.meeting); setMenuDeContexto(null); }}>Apagar reunião</button>
          </div>
        </div>
      ) : null}

      {apagando ? (
        <DialogoApagar
          meeting={apagando}
          gravando={apagando.id === gravandoId}
          fechar={() => setApagando(null)}
          apagar={(pararAntes) => apagar(apagando, pararAntes)}
        />
      ) : null}

      {cortando ? (
        <DialogoCorte
          meeting={cortando}
          fechar={() => setCortando(null)}
          salvar={async (inicio, fim) => {
            const resultado = await api.meetingSetTrim(cortando.id, inicio, fim, "manual");
            recarregar();
            mostrarFlash(resultado.requeued ? "Ajustado. A reunião vai ser organizada de novo." : "Início e fim ajustados");
          }}
        />
      ) : null}

      {followUp != null ? <DialogoFollowUp texto={followUp} fechar={() => setFollowUp(null)} /> : null}

      {pedirConsentimento ? (
        <DialogoConsentimento
          fechar={() => setPedirConsentimento(false)}
          autorizado={() => { setPedirConsentimento(false); void iniciar(); }}
        />
      ) : null}
    </div>
  );
}

/**
 * O consentimento. **Uma vez, e não a cada reunião** (`UX-PRINCIPLES` §21):
 * confirmações constantes ensinam a clicar sem ler.
 */
function DialogoConsentimento({ fechar, autorizado }: { fechar: () => void; autorizado: () => void }) {
  const [ocupado, setOcupado] = useState(false);
  return (
    <div className="meeting-scrim" onClick={fechar}>
      <div className="meeting-dialog meeting-consent" role="dialog" aria-modal="true" aria-label="Reuniões gravam áudio" onClick={(e) => e.stopPropagation()}>
        <header><h2>Reuniões gravam áudio</h2></header>
        <p>Enquanto estiver gravando, o M/OS captura o seu microfone e o áudio que sai pelo computador — o que inclui a voz das outras pessoas na chamada.</p>
        <p>O áudio fica neste computador e é apagado depois de processado. A transcrição é feita aqui. Para organizar decisões e tarefas, a transcrição é enviada ao Hermes; dá para desligar isso em Settings.</p>
        <p><b>Obter o consentimento dos outros participantes, quando necessário, é responsabilidade sua.</b></p>
        <footer className="form-actions">
          <Button variant="ghost" onClick={fechar}>Cancelar</Button>
          <Button
            autoFocus
            disabled={ocupado}
            onClick={() => {
              setOcupado(true);
              void api.meetingSetAnalysisConsent(true).then(autorizado).catch(() => setOcupado(false));
            }}
          >Entendi, gravar</Button>
        </footer>
      </div>
    </div>
  );
}

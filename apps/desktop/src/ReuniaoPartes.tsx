import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { conversations } from "./hermes";
import { EmptyState, Panel, StateMessage } from "./Surface";
import {
  dataDoInput, duracao, filtrarTranscricao, instanteDoInput, limitarCorte, marcaDoPasso,
  passosVisiveis, pedacosDoTrecho, relogio, rotuloDaOrigem, rotuloDePrazo, rotuloDoLote,
  textoDoTrecho, trechoMarcado, type FiltroDaTranscricao, type LinhaDeRevisao,
} from "./reuniao";
import type {
  Meeting, MeetingBookmark, MeetingInsight, MeetingOverview, Project, TranscriptSegment,
} from "./types";

/**
 * As partes da página de uma reunião.
 *
 * Separadas da página porque cada uma responde a UMA pergunta — o que falta
 * fazer, o que foi dito, onde a organização está — e porque a página já é
 * longa o bastante só de coordenar estado.
 */

type Receipt = (action: { message: string; run: () => Promise<unknown> }) => void;

// ---------------------------------------------------------------------------
// Progresso
// ---------------------------------------------------------------------------

/**
 * `✓ Gravação salva · ● Transcrevendo · ○ Finalizando · 72%`.
 *
 * O número é o medido: fração do whisper e janelas do Hermes, ponderadas. Sem
 * medida, sem número.
 */
export function ProgressoDaReuniao({ linha, fracaoAoVivo }: { linha: MeetingOverview; fracaoAoVivo: number | null }) {
  const passos = passosVisiveis(linha.progress);
  const fracao = fracaoAoVivo ?? linha.progress.fraction;
  return (
    <section className="reuniao-progresso" aria-label="Organizando a reunião">
      <header>
        <span className="micro-label">ORGANIZANDO REUNIÃO</span>
        {fracao != null ? <span className="reuniao-progresso-porcento">{Math.round(fracao * 100)}%</span> : null}
      </header>
      {fracao != null ? (
        <span className="processing-trilha"><span className="processing-preenchimento" style={{ width: `${Math.round(fracao * 100)}%` }} /></span>
      ) : null}
      <ol className="reuniao-passos">
        {passos.map((passo) => (
          <li key={passo.chave} data-estado={passo.estado}>
            <span aria-hidden="true">{marcaDoPasso(passo.estado)}</span>
            <span>{passo.texto}</span>
          </li>
        ))}
      </ol>
      <p className="support-copy">Pode sair desta tela. O M/OS avisa quando estiver pronta.</p>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Evidência
// ---------------------------------------------------------------------------

export function Evidencias({ item, trechos, saltar }: {
  item: MeetingInsight;
  trechos: TranscriptSegment[];
  saltar: (segmentId: string) => void;
}) {
  if (!item.evidence.length) return null;
  return (
    <span className="reuniao-evidencias">
      {item.evidence.map((evidencia) => {
        const trecho = trechos.find((t) => t.id === evidencia.segmentId);
        if (!trecho) return null;
        return (
          <button
            key={`${evidencia.segmentId}-${evidencia.seq}`}
            type="button"
            className="reuniao-evidencia"
            onClick={() => saltar(evidencia.segmentId)}
            title={textoDoTrecho(trecho)}
          >
            {relogio(trecho.startMs)}
          </button>
        );
      })}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Revisão em lote
// ---------------------------------------------------------------------------

type Rascunho = { marcado: boolean; titulo: string; prazo: string };

/**
 * A revisão em lote: o elemento principal da reunião pronta.
 *
 * ```
 * [x] Revisar prancha 04      sexta
 * [x] Enviar base             amanhã
 * [ ] Conferir IFC            sem prazo
 *                          Criar 2 tarefas
 * ```
 *
 * Alta vem marcada; média desmarcada e sinalizada; baixa não tem caixa — é
 * registro, e não pendência. Nenhum número de confiança aparece: a caixa JÁ é a
 * confiança dita em forma de gesto.
 */
export function RevisaoEmLote({ titulo, linhas, trechos, projectId, saltar, receipt, depois, externa }: {
  titulo: string;
  linhas: LinhaDeRevisao[];
  trechos: TranscriptSegment[];
  projectId: string | null;
  saltar: (segmentId: string) => void;
  receipt: Receipt;
  depois: () => void;
  /** Compromissos de outras pessoas: a Task nasce aguardando. */
  externa?: boolean;
}) {
  const [rascunhos, setRascunhos] = useState<Record<string, Rascunho>>({});
  const [ocupado, setOcupado] = useState(false);
  const [erro, setErro] = useState("");

  // Um rascunho por item, criado na chegada e preservado enquanto a pessoa
  // edita — recarregar a lista não pode desfazer o que ela marcou.
  useEffect(() => {
    setRascunhos((atuais) => {
      const ids = linhas.map((linha) => linha.item.id);
      const iguais = ids.length === Object.keys(atuais).length && ids.every((id) => id in atuais);
      // Mesmo conjunto de itens: devolve o MESMO objeto, e o React nao
      // re-renderiza. Sem isto, um array novo a cada render viraria laco.
      if (iguais) return atuais;
      const proximos: Record<string, Rascunho> = {};
      for (const linha of linhas) {
        proximos[linha.item.id] = atuais[linha.item.id] ?? {
          marcado: linha.marcadaDeInicio && !externa,
          titulo: linha.item.text,
          prazo: dataDoInput(linha.item.dueAt),
        };
      }
      return proximos;
    });
  }, [linhas, externa]);

  if (!linhas.length) return null;

  const marcados = linhas.filter((linha) => linha.selecionavel && rascunhos[linha.item.id]?.marcado);

  const criar = async () => {
    setOcupado(true);
    setErro("");
    try {
      const recibos = await api.meetingAcceptBatch(marcados.map((linha) => {
        const rascunho = rascunhos[linha.item.id];
        return {
          insightId: linha.item.id,
          title: rascunho.titulo.trim() || linha.item.text,
          projectId,
          dueAt: instanteDoInput(rascunho.prazo),
        };
      }));
      receipt({
        message: recibos.length === 1 ? "1 tarefa criada" : `${recibos.length} tarefas criadas`,
        run: async () => {
          for (const recibo of recibos) await conversations.undoAction(recibo.undo);
        },
      });
      depois();
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
    } finally {
      setOcupado(false);
    }
  };

  const alterar = (id: string, parte: Partial<Rascunho>) =>
    setRascunhos((atuais) => ({ ...atuais, [id]: { ...atuais[id], ...parte } }));

  return (
    <Panel label={titulo} count={String(linhas.length)} className="reuniao-revisao">
      <ul className="reuniao-revisao-lista">
        {linhas.map((linha) => {
          const rascunho = rascunhos[linha.item.id];
          if (!rascunho) return null;
          const origem = rotuloDaOrigem(linha.item);
          return (
            <li key={linha.item.id} className="reuniao-revisao-linha" data-selecionavel={linha.selecionavel || undefined}>
              {linha.selecionavel ? (
                <input
                  type="checkbox"
                  aria-label={`Criar tarefa: ${rascunho.titulo}`}
                  checked={rascunho.marcado}
                  onChange={(evento) => alterar(linha.item.id, { marcado: evento.currentTarget.checked })}
                />
              ) : <span className="reuniao-revisao-sem-caixa" aria-hidden="true">·</span>}
              <span className="reuniao-revisao-corpo">
                <input
                  className="reuniao-revisao-titulo"
                  value={rascunho.titulo}
                  aria-label="Título da tarefa"
                  onChange={(evento) => alterar(linha.item.id, { titulo: evento.currentTarget.value })}
                />
                <span className="reuniao-revisao-meta">
                  {externa && linha.item.owner ? <span>{linha.item.owner}</span> : null}
                  {linha.revisar ? <span className="reuniao-revisar">revisar</span> : null}
                  {!linha.selecionavel ? <span>só registro</span> : null}
                  {origem ? <span>{origem}</span> : null}
                  <Evidencias item={linha.item} trechos={trechos} saltar={saltar} />
                </span>
              </span>
              <label className="reuniao-revisao-prazo">
                <span className="visually-hidden">Prazo</span>
                <input
                  type="date"
                  value={rascunho.prazo}
                  onChange={(evento) => alterar(linha.item.id, { prazo: evento.currentTarget.value })}
                />
                <span className="reuniao-prazo-rotulo">
                  {rascunho.prazo ? rotuloDePrazo(instanteDoInput(rascunho.prazo)?.toISOString() ?? null) : "sem prazo"}
                </span>
              </label>
              <button
                type="button"
                className="reuniao-descartar"
                aria-label="Descartar item"
                title="Descartar"
                onClick={() => void api.meetingDismissInsight(linha.item.id).then(depois)}
              >×</button>
            </li>
          );
        })}
      </ul>
      {erro ? <StateMessage state="error" label="Não foi possível criar" detail={erro} /> : null}
      <footer className="reuniao-revisao-rodape">
        {externa ? <span className="support-copy">Viram Tasks aguardando a pessoa responsável.</span> : null}
        <Button variant="primary" size="sm" disabled={ocupado || marcados.length === 0} onClick={() => void criar()}>
          {ocupado ? "Criando…" : rotuloDoLote(marcados.length)}
        </Button>
      </footer>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Listas simples de itens
// ---------------------------------------------------------------------------

export function ListaDeItens({ titulo, itens, trechos, saltar, depois }: {
  titulo: string;
  itens: MeetingInsight[];
  trechos: TranscriptSegment[];
  saltar: (segmentId: string) => void;
  depois: () => void;
}) {
  if (!itens.length) return null;
  return (
    <Panel label={titulo} count={String(itens.length)}>
      <ul className="reuniao-itens">
        {itens.map((item) => (
          <li key={item.id} data-confianca={item.confidence}>
            <span className="reuniao-item-texto">{item.text}</span>
            <span className="reuniao-revisao-meta">
              {item.owner ? <span>{item.owner}</span> : null}
              {item.dueAt ? <span>{rotuloDePrazo(item.dueAt)}</span> : item.dueHint ? <span>{item.dueHint}</span> : null}
              {rotuloDaOrigem(item) ? <span>{rotuloDaOrigem(item)}</span> : null}
              {item.status === "accepted" ? <span>virou Task</span> : null}
              <Evidencias item={item} trechos={trechos} saltar={saltar} />
            </span>
            {item.status === "proposed" ? (
              <button
                type="button"
                className="reuniao-descartar"
                aria-label="Descartar item"
                title="Descartar"
                onClick={() => void api.meetingDismissInsight(item.id).then(depois)}
              >×</button>
            ) : null}
          </li>
        ))}
      </ul>
    </Panel>
  );
}

// ---------------------------------------------------------------------------
// Transcrição
// ---------------------------------------------------------------------------

type ModoDeOuvir = "both" | "mic" | "system";

/**
 * A transcrição como timeline: busca, filtros, horário, ouvir, copiar, marcar
 * momento, criar Task e marcar decisão a partir de um trecho.
 *
 * O texto lido é o NORMALIZADO quando existe — com as trocas do vocabulário
 * grifadas e o original no `title`. A troca incerta aparece como **[Criciúma?]**.
 * O cru nunca muda.
 */
export function Transcricao({ meeting, trechos, marcas, itens, alvo, audioDisponivel, depois }: {
  meeting: Meeting;
  trechos: TranscriptSegment[];
  marcas: MeetingBookmark[];
  itens: MeetingInsight[];
  /** O trecho para o qual a evidência saltou. */
  alvo: string | null;
  audioDisponivel: boolean;
  depois: () => void;
}) {
  const [filtro, setFiltro] = useState<FiltroDaTranscricao>({ busca: "", canal: "todos", soMarcados: false, soComItens: false });
  const [modo, setModo] = useState<ModoDeOuvir>("both");
  const [tocando, setTocando] = useState<{ segmentId: string; src: string } | null>(null);
  const [aviso, setAviso] = useState("");
  const lista = useRef<HTMLDivElement>(null);

  const visiveis = useMemo(() => filtrarTranscricao(trechos, filtro, marcas, itens), [trechos, filtro, marcas, itens]);

  useEffect(() => {
    if (!alvo) return;
    const no = lista.current?.querySelector(`[data-segment="${alvo}"]`);
    no?.scrollIntoView({ block: "center", behavior: "smooth" });
    no?.classList.add("is-target");
    const timer = window.setTimeout(() => no?.classList.remove("is-target"), 2000);
    return () => window.clearTimeout(timer);
  }, [alvo, visiveis.length]);

  const avisar = (texto: string) => {
    setAviso(texto);
    window.setTimeout(() => setAviso(""), 2500);
  };

  const ouvir = async (trecho: TranscriptSegment) => {
    try {
      const inicio = Math.max(0, trecho.startMs - 1500);
      const base64 = await api.meetingClip(meeting.id, inicio, modo, Math.min(60_000, trecho.endMs - inicio + 3000));
      setTocando({ segmentId: trecho.id, src: `data:audio/wav;base64,${base64}` });
    } catch (causa) {
      avisar(causa instanceof Error ? causa.message : String(causa));
    }
  };

  if (!trechos.length) {
    return <EmptyState>A transcrição aparece aqui quando a organização terminar.</EmptyState>;
  }

  return (
    <div className="reuniao-transcricao">
      <div className="reuniao-transcricao-filtros">
        <input
          className="reuniao-busca"
          value={filtro.busca}
          onChange={(evento) => setFiltro({ ...filtro, busca: evento.currentTarget.value })}
          placeholder="Buscar na transcrição"
          aria-label="Buscar na transcrição"
        />
        <div className="segmented" role="group" aria-label="Quem falou">
          {(["todos", "mic", "system"] as const).map((canal) => (
            <button key={canal} type="button" aria-pressed={filtro.canal === canal} onClick={() => setFiltro({ ...filtro, canal })}>
              {canal === "todos" ? "Todos" : canal === "mic" ? "Você" : "Remoto"}
            </button>
          ))}
        </div>
        <label className="check-control">
          <input type="checkbox" checked={filtro.soMarcados} onChange={(evento) => setFiltro({ ...filtro, soMarcados: evento.currentTarget.checked })} />
          <span>Marcados</span>
        </label>
        <label className="check-control">
          <input type="checkbox" checked={filtro.soComItens} onChange={(evento) => setFiltro({ ...filtro, soComItens: evento.currentTarget.checked })} />
          <span>Com itens</span>
        </label>
        {audioDisponivel ? (
          <label className="reuniao-ouvir-modo">
            <span className="micro-label">OUVIR</span>
            <select value={modo} onChange={(evento) => setModo(evento.currentTarget.value as ModoDeOuvir)}>
              <option value="both">Ambos</option>
              <option value="mic">Você</option>
              <option value="system">Remoto</option>
            </select>
          </label>
        ) : null}
      </div>

      <p className="micro-label">{visiveis.length} de {trechos.length} trechos{aviso ? ` · ${aviso}` : ""}</p>

      {tocando ? (
        <audio className="reuniao-player" src={tocando.src} controls autoPlay onEnded={() => setTocando(null)} />
      ) : null}

      <div className="reuniao-linhas" ref={lista}>
        {visiveis.map((trecho) => (
          <div
            key={trecho.id}
            className="meeting-line reuniao-linha"
            data-segment={trecho.id}
            data-channel={trecho.channel}
            data-marcado={trechoMarcado(trecho, marcas) || undefined}
          >
            <span className="meeting-line-time">{relogio(trecho.startMs)}</span>
            <span className="meeting-line-who">{trecho.channel === "mic" ? "VOCÊ" : "REMOTO"}</span>
            <span className="meeting-line-text">
              {pedacosDoTrecho(trecho).map((pedaco, indice) => pedaco.original ? (
                <mark
                  key={indice}
                  className="reuniao-correcao"
                  data-incerta={pedaco.incerto || undefined}
                  title={`Transcrito como “${pedaco.original}”`}
                >
                  {pedaco.incerto ? `[${pedaco.texto}?]` : pedaco.texto}
                </mark>
              ) : <span key={indice}>{pedaco.texto}</span>)}
            </span>
            <span className="reuniao-linha-acoes">
              {audioDisponivel ? (
                <button type="button" title="Ouvir este trecho" aria-label="Ouvir" onClick={() => void ouvir(trecho)}>▶</button>
              ) : null}
              <button
                type="button"
                title="Copiar trecho"
                aria-label="Copiar"
                onClick={() => void navigator.clipboard.writeText(`[${relogio(trecho.startMs)}] ${trecho.channel === "mic" ? "Você" : "Remoto"}: ${textoDoTrecho(trecho)}`).then(() => avisar("copiado"))}
              >⧉</button>
              <button
                type="button"
                title="Marcar momento"
                aria-label="Marcar momento"
                onClick={() => void api.meetingAddBookmark(meeting.id, trecho.startMs).then(depois)}
              >★</button>
              <button
                type="button"
                title="Criar Task a partir deste trecho"
                aria-label="Criar Task"
                onClick={() => void api.meetingAddInsight(meeting.id, trecho.id, "my_action").then(() => { avisar("vai para a revisão"); depois(); })}
              >＋</button>
              <button
                type="button"
                title="Marcar como decisão"
                aria-label="Marcar decisão"
                onClick={() => void api.meetingAddInsight(meeting.id, trecho.id, "decision").then(() => { avisar("decisão registrada"); depois(); })}
              >◆</button>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Diálogos
// ---------------------------------------------------------------------------

function Dialogo({ rotulo, fechar, children, alerta }: {
  rotulo: string;
  fechar: () => void;
  children: React.ReactNode;
  alerta?: boolean;
}) {
  return (
    <div className="meeting-scrim" onClick={fechar}>
      <div
        className="meeting-dialog"
        role={alerta ? "alertdialog" : "dialog"}
        aria-modal="true"
        aria-label={rotulo}
        onClick={(evento) => evento.stopPropagation()}
        onKeyDown={(evento) => { if (evento.key === "Escape") fechar(); }}
      >
        {children}
      </div>
    </div>
  );
}

/**
 * Apagar reunião — a lixeira, com desfazer.
 *
 * Diz o que sai, diz o que FICA (as Tasks), e diz a política da lixeira. Se a
 * reunião ainda grava, a pergunta muda: não se apaga silenciosamente o que
 * está gravando.
 */
export function DialogoApagar({ meeting, gravando, fechar, apagar }: {
  meeting: Meeting;
  gravando: boolean;
  fechar: () => void;
  apagar: (pararAntes: boolean) => Promise<void>;
}) {
  const [ocupado, setOcupado] = useState(false);
  const ir = async () => {
    setOcupado(true);
    try { await apagar(gravando); } finally { setOcupado(false); }
  };
  return (
    <Dialogo rotulo="Apagar reunião" fechar={fechar} alerta>
      <header>
        <span className="micro-label">APAGAR REUNIÃO</span>
        <h2>{gravando ? "Esta reunião ainda está sendo gravada." : `Apagar “${meeting.title}”?`}</h2>
      </header>
      <p>
        {gravando ? "Encerrar a gravação e mandar a reunião para a lixeira? " : ""}
        Saem a gravação, a transcrição, o resumo, os itens, os momentos marcados e as notas.
      </p>
      <p><b>As Tasks já criadas serão mantidas.</b></p>
      <p className="support-copy">Fica na lixeira por 30 dias, e dá para desfazer. Depois disso é apagada de vez, com os arquivos.</p>
      <footer className="form-actions">
        <Button variant="ghost" onClick={fechar}>Cancelar</Button>
        <Button variant="danger" disabled={ocupado} onClick={() => void ir()}>
          {gravando ? "Encerrar e apagar" : "Apagar"}
        </Button>
      </footer>
    </Dialogo>
  );
}

/** "Ajustar início e fim": duas alças sobre a duração. Não é editor de áudio. */
export function DialogoCorte({ meeting, fechar, salvar }: {
  meeting: Meeting;
  fechar: () => void;
  salvar: (inicio: number, fim: number) => Promise<void>;
}) {
  const total = meeting.durationMs;
  const [inicio, setInicio] = useState(meeting.trimStartMs ?? 0);
  const [fim, setFim] = useState(meeting.trimEndMs ?? total);
  const [ocupado, setOcupado] = useState(false);
  const [erro, setErro] = useState("");
  const passo = total > 60 * 60_000 ? 10_000 : 1_000;
  const [a, b] = limitarCorte(inicio, fim, total);

  const ir = async () => {
    setOcupado(true);
    setErro("");
    try {
      await salvar(a, b);
      fechar();
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
    } finally {
      setOcupado(false);
    }
  };

  return (
    <Dialogo rotulo="Ajustar início e fim" fechar={fechar}>
      <header>
        <span className="micro-label">AJUSTAR INÍCIO E FIM</span>
        <h2>O que conta desta gravação</h2>
      </header>
      <div className="reuniao-corte" aria-hidden="true">
        <span className="reuniao-corte-util" style={{ left: `${(a / total) * 100}%`, width: `${((b - a) / total) * 100}%` }} />
      </div>
      <label className="meeting-field">
        <span>Início · {relogio(a)}</span>
        <input type="range" min={0} max={total} step={passo} value={a} onChange={(evento) => setInicio(Number(evento.currentTarget.value))} />
      </label>
      <label className="meeting-field">
        <span>Fim · {relogio(b)}</span>
        <input type="range" min={0} max={total} step={passo} value={b} onChange={(evento) => setFim(Number(evento.currentTarget.value))} />
      </label>
      <p className="support-copy">
        Conta {duracao(b - a)} de {duracao(total)}. O áudio fica guardado enquanto a retenção permitir: dá para voltar atrás.
      </p>
      {erro ? <StateMessage state="error" label="Não foi possível ajustar" detail={erro} /> : null}
      <footer className="form-actions">
        <Button variant="ghost" onClick={fechar}>Cancelar</Button>
        <Button variant="primary" disabled={ocupado} onClick={() => void ir()}>Aplicar</Button>
      </footer>
    </Dialogo>
  );
}

/** O follow-up pronto para copiar. Nunca é enviado. */
export function DialogoFollowUp({ texto, fechar }: { texto: string; fechar: () => void }) {
  const [copiado, setCopiado] = useState(false);
  return (
    <Dialogo rotulo="Follow-up" fechar={fechar}>
      <header>
        <span className="micro-label">PREPARAR FOLLOW-UP</span>
        <h2>Resumo para enviar</h2>
      </header>
      <textarea className="reuniao-followup" readOnly value={texto} rows={14} />
      <p className="support-copy">O M/OS não envia nada. Copie e cole onde quiser.</p>
      <footer className="form-actions">
        <Button variant="ghost" onClick={fechar}>Fechar</Button>
        <Button
          variant="primary"
          onClick={() => void navigator.clipboard.writeText(texto).then(() => { setCopiado(true); window.setTimeout(() => setCopiado(false), 2000); })}
        >{copiado ? "Copiado" : "Copiar"}</Button>
      </footer>
    </Dialogo>
  );
}

/** A lixeira: restaurar, apagar de vez, esvaziar. */
export function Lixeira({ reunioes, restaurar, apagarDeVez, esvaziar }: {
  reunioes: Meeting[];
  restaurar: (meeting: Meeting) => void;
  apagarDeVez: (meeting: Meeting) => void;
  esvaziar: () => void;
}) {
  const [confirmar, setConfirmar] = useState<Meeting | "todas" | null>(null);
  if (!reunioes.length) {
    return <EmptyState>A lixeira está vazia.</EmptyState>;
  }
  return (
    <div className="reuniao-lixeira">
      <p className="support-copy">Reuniões na lixeira são apagadas de vez depois de 30 dias. As Tasks criadas a partir delas ficam.</p>
      <ul>
        {reunioes.map((meeting) => (
          <li key={meeting.id} className="reuniao-lixeira-linha">
            <span>
              <strong>{meeting.title}</strong>
              <span className="micro-label">
                {new Date(meeting.startedAt).toLocaleDateString("pt-BR", { day: "2-digit", month: "short" })} · {duracao(meeting.durationMs)}
              </span>
            </span>
            <Button variant="ghost" size="sm" onClick={() => restaurar(meeting)}>Restaurar</Button>
            <Button variant="ghost" size="sm" onClick={() => setConfirmar(meeting)}>Apagar de vez</Button>
          </li>
        ))}
      </ul>
      <Button variant="ghost" size="sm" onClick={() => setConfirmar("todas")}>Esvaziar lixeira</Button>
      {confirmar ? (
        <Dialogo rotulo="Exclusão definitiva" fechar={() => setConfirmar(null)} alerta>
          <header>
            <span className="micro-label">EXCLUSÃO DEFINITIVA</span>
            <h2>{confirmar === "todas" ? "Esvaziar a lixeira?" : `Apagar “${confirmar.title}” de vez?`}</h2>
          </header>
          <p>Isto apaga do banco e do disco. Não há desfazer. As Tasks já criadas continuam.</p>
          <footer className="form-actions">
            <Button variant="ghost" onClick={() => setConfirmar(null)}>Cancelar</Button>
            <Button
              variant="danger"
              onClick={() => {
                if (confirmar === "todas") esvaziar(); else apagarDeVez(confirmar);
                setConfirmar(null);
              }}
            >Apagar de vez</Button>
          </footer>
        </Dialogo>
      ) : null}
    </div>
  );
}

export function nomeDoProject(projects: Project[], id: string | null): string | null {
  if (!id) return null;
  return projects.find((project) => project.id === id)?.name ?? null;
}

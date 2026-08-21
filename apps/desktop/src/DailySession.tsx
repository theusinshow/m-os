import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LazyMotion, m } from "framer-motion";
import { api, appError } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { ActionMenu, EmptyState, StateMessage } from "./Surface";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import {
  HUMOR_ROTULO,
  SECUNDARIOS_SUGERIDOS,
  dataPorExtenso,
  emOrdem,
  estadoDoDia,
  horaDe,
  linhaDeHistorico,
  linhaDeObjetivo,
  moverObjetivo,
  progresso,
  resumoDoDia,
  saudacao,
  vagasRestantes,
} from "./daily";
import type { DailySessionSummary, DailyContext, DailyObjective, DailyToday, ObjectiveLink } from "./types";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/**
 * A Daily Session na Home, e a sessão inteira quando alguém quer vê-la.
 *
 * # Por que o foco do dia é um WIDGET
 *
 * A Home do M/OS é um quadro de widgets arrumável (`homeLayout.ts`), e tudo que
 * mora nela é um deles. Um bloco fixo acima do quadro seria a única coisa da
 * Home que não se pode mover, esconder ou redimensionar — e o pedido diz que a
 * feature tem de parecer camada estrutural, não página colada.
 *
 * Ele nasce na faixa "Agora", primeiro do catálogo. Quem nunca arrumou a Home
 * vê o dia no topo; quem já arrumou recebe o widget no FIM da faixa dele, pela
 * regra do `arrangeHome` — widget novo não se enfia no meio de um arranjo que a
 * pessoa montou.
 */

// ===========================================================================
// A âncora do dia, na Home
// ===========================================================================

export function DailyFocusWidget({
  dia,
  contexto,
  carregando,
  erro,
  iniciar,
  abrirSessao,
  encerrarAntigo,
  concluirObjetivo,
  abrirVinculo,
}: {
  dia: DailyToday | null;
  contexto: DailyContext | null;
  carregando: boolean;
  erro: string;
  iniciar: () => void;
  abrirSessao: () => void;
  encerrarAntigo: () => void;
  concluirObjetivo: (id: string) => void;
  abrirVinculo: (link: ObjectiveLink) => void;
}) {
  const estado = estadoDoDia(dia);
  const agora = useMemo(() => new Date(), []);

  if (carregando && !dia) return <StateMessage state="loading" label="Lendo o dia..." />;
  if (erro && !dia) return <StateMessage state="error" label="O dia não pôde ser lido." detail={erro} />;

  if (estado.tipo === "nao_iniciado" || estado.tipo === "ontem_aberto") {
    const linhas = resumoDoDia(contexto).slice(0, 3);
    return (
      <div className="daily-widget" data-state="idle">
        <p className="daily-greeting">{saudacao(agora)}</p>
        {linhas.length ? (
          <ul className="daily-widget-context">
            {linhas.map((linha) => (
              <li key={linha.chave}>
                <strong>{linha.valor}</strong>
                <span>{linha.texto.replace(`${linha.valor} `, "")}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="daily-widget-quiet">Nada atrasado, nada vencendo.</p>
        )}

        {/* A porta de ontem, discreta e SEM travar nada. O pedido é explícito:
            não bloquear o usuário. Começar hoje fecha a de ontem sozinho, do
            lado do banco, sem decidir o destino de nenhum objetivo. */}
        {estado.tipo === "ontem_aberto" ? (
          <p className="daily-stale" role="status">
            Você ainda não encerrou {dataPorExtenso(estado.dia)}.
            {estado.pendentes ? ` ${estado.pendentes} ${estado.pendentes === 1 ? "objetivo" : "objetivos"} sem desfecho.` : ""}
            <Button size="sm" variant="ghost" onClick={encerrarAntigo}>Encerrar</Button>
          </p>
        ) : null}

        <Button variant="primary" onClick={iniciar}>Iniciar meu dia</Button>
      </div>
    );
  }

  const objetivos = emOrdem(dia?.objectives ?? []);

  if (estado.tipo === "encerrado") {
    return (
      <div className="daily-widget" data-state="closed">
        <p className="daily-greeting">Dia encerrado</p>
        <p className="daily-widget-score">
          <strong>{estado.feitos}</strong> de {estado.total} {estado.total === 1 ? "objetivo concluído" : "objetivos concluídos"}.
        </p>
        {dia?.reflection?.mood ? (
          <p className="daily-widget-quiet">Dia {HUMOR_ROTULO[dia.reflection.mood]}.</p>
        ) : null}
        {/* "Ver resumo" e não "Reabrir": o pedido pede para não incentivar a
            reabertura do ciclo. Reabrir continua existindo, uma camada adentro,
            para quem precisa. */}
        <Button variant="outline" size="sm" onClick={abrirSessao}>Ver resumo</Button>
      </div>
    );
  }

  return (
    <div className="daily-widget" data-state="active">
      <p className="daily-widget-score">
        <strong>{estado.feitos}</strong> de {estado.total} {estado.total === 1 ? "objetivo" : "objetivos"}
      </p>
      <ul className="daily-objectives">
        {objetivos.map((objetivo) => (
          <ObjectiveRow
            key={objetivo.id}
            objetivo={objetivo}
            compacto
            concluir={() => concluirObjetivo(objetivo.id)}
            abrirVinculo={abrirVinculo}
          />
        ))}
      </ul>
      <Button variant="ghost" size="sm" onClick={abrirSessao}>Ver sessão do dia</Button>
    </div>
  );
}

// ===========================================================================
// Uma linha de objetivo
// ===========================================================================

function ObjectiveRow({
  objetivo,
  compacto = false,
  arrastavel = false,
  concluir,
  abrirVinculo,
  acoes,
  aoArrastar,
  aoSoltar,
}: {
  objetivo: DailyObjective;
  compacto?: boolean;
  arrastavel?: boolean;
  concluir?: () => void;
  abrirVinculo: (link: ObjectiveLink) => void;
  acoes?: React.ReactNode;
  aoArrastar?: () => void;
  aoSoltar?: () => void;
}) {
  const linha = linhaDeObjetivo(objetivo);
  return (
    <li
      className="daily-objective"
      data-main={linha.principal || undefined}
      data-status={objetivo.status}
      data-compact={compacto || undefined}
      draggable={arrastavel || undefined}
      onDragStart={aoArrastar}
      onDragOver={arrastavel ? (evento) => evento.preventDefault() : undefined}
      onDrop={aoSoltar}
    >
      {/* O marcador é botão só quando dá para concluir. Um marcador clicável que
          não faz nada ensina a não confiar nos outros. */}
      {linha.concluivel && concluir ? (
        <button type="button" className="daily-mark" aria-label={`Concluir ${linha.titulo}`} onClick={concluir}>
          <span aria-hidden="true">{linha.marcador}</span>
        </button>
      ) : (
        <span className="daily-mark" aria-hidden="true">{linha.marcador}</span>
      )}

      {linha.link ? (
        <button type="button" className="daily-objective-title" onClick={() => abrirVinculo(linha.link!)} title={`Abrir ${linha.link.kind}`}>
          {linha.titulo}
        </button>
      ) : (
        <span className="daily-objective-title">{linha.titulo}</span>
      )}

      {linha.principal && !compacto ? <span className="micro-label">PRINCIPAL</span> : null}
      {linha.estado ? <span className="daily-objective-state micro-label">{linha.estado}</span> : null}
      {acoes}
    </li>
  );
}

// ===========================================================================
// A sessão inteira
// ===========================================================================

type Aba = "hoje" | "historico";

export function DailySessionView({
  dia,
  close,
  atualizado,
  encerrar,
  abrirVinculo,
}: {
  dia: DailyToday;
  close: () => void;
  atualizado: (proximo: DailyToday) => void;
  encerrar: () => void;
  abrirVinculo: (link: ObjectiveLink) => void;
}) {
  const [aba, setAba] = useState<Aba>("hoje");
  const [erro, setErro] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [adicionando, setAdicionando] = useState(false);
  const [novo, setNovo] = useState("");
  const [arrastando, setArrastando] = useState<string | null>(null);
  const painel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    painel.current?.focus();
    function aoTeclar(evento: KeyboardEvent) {
      if (evento.key === "Escape") {
        evento.preventDefault();
        close();
      }
    }
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [close]);

  /** Toda mutação devolve o dia inteiro: a tela nunca recalcula o progresso. */
  const agir = useCallback(
    async (executar: () => Promise<DailyToday>) => {
      setOcupado(true);
      try {
        atualizado(await executar());
        setErro("");
      } catch (falha) {
        setErro(appError(falha).message);
      } finally {
        setOcupado(false);
      }
    },
    [atualizado],
  );

  const objetivos = useMemo(() => emOrdem(dia.objectives), [dia.objectives]);
  const { feitos, total } = progresso(dia.objectives);
  const vagas = vagasRestantes(dia.objectives);
  const ativo = dia.status === "active";

  function soltarSobre(alvo: string) {
    if (!arrastando || arrastando === alvo) return;
    const ordem = moverObjetivo(objetivos, arrastando, alvo).map((objetivo) => objetivo.id);
    setArrastando(null);
    if (!dia.session) return;
    void agir(() => api.dailyReorder(dia.session!.id, ordem));
  }

  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <m.button
        aria-hidden="true"
        className="attention-scrim"
        onClick={close}
        tabIndex={-1}
        type="button"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: MOTION_DURATIONS.enter }}
      />
      <m.div
        aria-label="Sessão do dia"
        className="daily-session"
        ref={painel}
        role="dialog"
        tabIndex={-1}
        initial={{ opacity: 0, x: 24 }}
        animate={{ opacity: 1, x: 0 }}
        exit={{ opacity: 0, x: 20 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <header className="daily-session-head">
          <div className="daily-session-when">
            <strong>{dataPorExtenso(dia.day)}</strong>
            {dia.session ? (
              <span className="micro-label">
                {horaDe(dia.session.startedAt)}
                {dia.session.endedAt ? ` — ${horaDe(dia.session.endedAt)}` : ""}
              </span>
            ) : null}
          </div>
          <button type="button" className="icon-button" aria-label="Fechar" onClick={close}>
            <Icon name="close" />
          </button>
        </header>

        <div className="daily-tabs" role="tablist" aria-label="Sessão do dia">
          <button type="button" role="tab" aria-selected={aba === "hoje"} onClick={() => setAba("hoje")}>Sessão</button>
          <button type="button" role="tab" aria-selected={aba === "historico"} onClick={() => setAba("historico")}>Histórico</button>
        </div>

        {aba === "historico" ? (
          <DailySessionHistory abrirVinculo={abrirVinculo} />
        ) : (
          <div className="daily-session-body" data-busy={ocupado || undefined}>
            <p className="daily-widget-score">
              <strong>{feitos}</strong> de {total} {total === 1 ? "objetivo" : "objetivos"}
            </p>

            {/* A justificativa do Hermes, quando ele montou o dia. Só aparece
                quando existe: um rótulo permanente vazio ensinaria que falta
                algo. */}
            {dia.session?.note ? <p className="daily-note">{dia.session.note}</p> : null}

            {objetivos.length ? (
              <ul className="daily-objectives" data-editable={ativo || undefined}>
                {objetivos.map((objetivo) => (
                  <ObjectiveRow
                    key={objetivo.id}
                    objetivo={objetivo}
                    arrastavel={ativo}
                    aoArrastar={() => setArrastando(objetivo.id)}
                    aoSoltar={() => soltarSobre(objetivo.id)}
                    concluir={ativo ? () => void agir(() => api.dailySetObjectiveStatus(objetivo.id, "completed")) : undefined}
                    abrirVinculo={abrirVinculo}
                    acoes={
                      ativo ? (
                        <ActionMenu
                          trigger={<Icon name="more" />}
                          label={`Ações de ${objetivo.title}`}
                          items={[
                            ...(objetivo.priority === "secondary"
                              ? [{ label: "Tornar principal", onSelect: () => void agir(() => api.dailySetMain(objetivo.id)) }]
                              : []),
                            ...(objetivo.status === "completed"
                              ? [{ label: "Devolver a pendente", onSelect: () => void agir(() => api.dailySetObjectiveStatus(objetivo.id, "pending")) }]
                              : [{ label: "Concluir", onSelect: () => void agir(() => api.dailySetObjectiveStatus(objetivo.id, "completed")) }]),
                            { label: "Levar para amanhã", onSelect: () => void agir(() => api.dailySetObjectiveStatus(objetivo.id, "carried_over")) },
                            { label: "Abandonar", onSelect: () => void agir(() => api.dailySetObjectiveStatus(objetivo.id, "dropped")) },
                            // Remover APAGA, e por isso é a última e marcada
                            // como perigosa: `dropped` é a saída que mantém o
                            // registro, e ela está logo acima.
                            { label: "Remover do dia", danger: true, onSelect: () => void agir(() => api.dailyRemoveObjective(objetivo.id)) },
                          ]}
                        />
                      ) : undefined
                    }
                  />
                ))}
              </ul>
            ) : (
              <EmptyState>Este dia não tem objetivos. Um objetivo é o que faz o dia ter uma resposta.</EmptyState>
            )}

            {ativo ? (
              adicionando ? (
                <form
                  className="daily-add"
                  onSubmit={(evento) => {
                    evento.preventDefault();
                    if (!novo.trim()) return;
                    const titulo = novo.trim();
                    setNovo("");
                    setAdicionando(false);
                    void agir(() => api.dailyAddObjective({ title: titulo, linkKind: "", linkId: "" }, "secondary"));
                  }}
                >
                  <input autoFocus value={novo} aria-label="Novo objetivo" placeholder="Outro objetivo para hoje" onChange={(evento) => setNovo(evento.currentTarget.value)} />
                  <Button type="submit" size="sm" variant="primary" disabled={!novo.trim()}>Adicionar</Button>
                  <Button size="sm" variant="ghost" onClick={() => { setNovo(""); setAdicionando(false); }}>Cancelar</Button>
                </form>
              ) : (
                <Button size="sm" variant="outline" onClick={() => setAdicionando(true)}>
                  Adicionar objetivo{vagas > 0 ? ` · ${vagas} ${vagas === 1 ? "vaga" : "vagas"}` : ""}
                </Button>
              )
            ) : null}

            {/* Passar do conselho não é erro: o banco aceita, e a frase é um
                lembrete e não uma barreira. */}
            {ativo && vagas === 0 ? (
              <p className="daily-slot-hint">{SECUNDARIOS_SUGERIDOS} secundários é o que costuma caber num dia.</p>
            ) : null}

            {dia.reflection ? (
              <section className="daily-reflection-read">
                <span className="micro-label">COMO FOI O DIA</span>
                {dia.reflection.mood ? <p className="daily-mood">Dia {HUMOR_ROTULO[dia.reflection.mood]}.</p> : null}
                {dia.reflection.summary ? <p>{dia.reflection.summary}</p> : null}
              </section>
            ) : null}

            {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}

            <div className="form-actions">
              {ativo ? (
                <Button variant="primary" onClick={encerrar}>Encerrar meu dia</Button>
              ) : dia.session ? (
                <Button
                  variant="outline"
                  size="sm"
                  disabled={ocupado}
                  onClick={() => void agir(() => api.dailyReopen(dia.session!.id))}
                >
                  Reabrir o dia
                </Button>
              ) : null}
            </div>
          </div>
        )}
      </m.div>
    </LazyMotion>
  );
}

// ===========================================================================
// O histórico
// ===========================================================================

export function DailySessionHistory({ abrirVinculo }: { abrirVinculo: (link: ObjectiveLink) => void }) {
  const [dias, setDias] = useState<DailySessionSummary[] | null>(null);
  const [aberto, setAberto] = useState<DailyToday | null>(null);
  const [erro, setErro] = useState("");

  useEffect(() => {
    let cancelado = false;
    void api
      .dailyHistory()
      .then((achados) => { if (!cancelado) { setDias(achados); setErro(""); } })
      .catch((falha) => { if (!cancelado) setErro(appError(falha).message); });
    return () => { cancelado = true; };
  }, []);

  if (erro) return <StateMessage state="error" label="O histórico não pôde ser lido." detail={erro} />;
  if (!dias) return <StateMessage state="loading" label="Lendo o histórico..." />;
  if (!dias.length) {
    return (
      <div className="daily-session-body">
        <EmptyState>Nenhum dia registrado ainda. O primeiro aparece aqui depois que você iniciar um.</EmptyState>
      </div>
    );
  }

  if (aberto) {
    const objetivos = emOrdem(aberto.objectives);
    return (
      <div className="daily-session-body">
        <button type="button" className="daily-back" onClick={() => setAberto(null)}>← Todos os dias</button>
        <div className="daily-session-when">
          <strong>{dataPorExtenso(aberto.day)}</strong>
          {aberto.session ? (
            <span className="micro-label">
              {horaDe(aberto.session.startedAt)}
              {aberto.session.endedAt ? ` — ${horaDe(aberto.session.endedAt)}` : " · em aberto"}
            </span>
          ) : null}
        </div>
        {aberto.session?.note ? <p className="daily-note">{aberto.session.note}</p> : null}
        <ul className="daily-objectives">
          {objetivos.map((objetivo) => (
            <ObjectiveRow key={objetivo.id} objetivo={objetivo} abrirVinculo={abrirVinculo} />
          ))}
        </ul>
        {aberto.reflection ? (
          <section className="daily-reflection-read">
            <span className="micro-label">COMO FOI O DIA</span>
            {aberto.reflection.mood ? <p className="daily-mood">Dia {HUMOR_ROTULO[aberto.reflection.mood]}.</p> : null}
            {aberto.reflection.summary ? <p>{aberto.reflection.summary}</p> : null}
          </section>
        ) : null}
      </div>
    );
  }

  return (
    <div className="daily-session-body">
      <ul className="daily-history">
        {dias.map((resumo) => {
          const linha = linhaDeHistorico(resumo);
          return (
            <li key={resumo.session.id}>
              <button
                type="button"
                onClick={() => {
                  void api
                    .dailySession(resumo.session.id)
                    .then(setAberto)
                    .catch((falha) => setErro(appError(falha).message));
                }}
              >
                <strong>{linha.data}</strong>
                <span className="daily-history-score">{linha.placar}</span>
                {resumo.mainTitle ? <span className="daily-history-main">{resumo.mainTitle}</span> : null}
                {resumo.session.status === "active" ? <span className="micro-label">EM ABERTO</span> : null}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

// ===========================================================================
// O carregamento
// ===========================================================================

/**
 * Lê o dia e o contexto, e mantém os dois vivos.
 *
 * **Fora do `refresh()` do shell, de propósito.** Aquele é o caminho de boot do
 * aplicativo inteiro, e um erro ao ler o dia não pode ser motivo para a Home
 * não abrir — é a mesma decisão que o `useTrackedTime` já tomou, e pelo mesmo
 * motivo. O widget mostra o próprio erro; o resto da Home continua de pé.
 *
 * **O fuso é publicado ANTES da primeira leitura, e isto não é cerimônia.** Quem
 * decide que dia é hoje é o backend, lendo o offset que a tela publicou em
 * `surface.rs`. Sem esta espera, a primeira leitura de um dia que começa às 21h
 * em UTC-3 acontece com offset zero — e o M/OS responderia sobre AMANHÃ, criando
 * a sessão na data errada. Publicar é idempotente e custa uma chamada.
 */
export function useDaily() {
  const [dia, setDia] = useState<DailyToday | null>(null);
  const [contexto, setContexto] = useState<DailyContext | null>(null);
  const [carregando, setCarregando] = useState(true);
  const [erro, setErro] = useState("");

  const recarregar = useCallback(async () => {
    try {
      await api.surfaceSetLocale();
      const [proximoDia, proximoContexto] = await Promise.all([api.dailyToday(), api.dailyContext()]);
      setDia(proximoDia);
      setContexto(proximoContexto);
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    } finally {
      setCarregando(false);
    }
  }, []);

  useEffect(() => {
    void recarregar();
  }, [recarregar]);

  return { dia, contexto, carregando, erro, recarregar, setDia };
}

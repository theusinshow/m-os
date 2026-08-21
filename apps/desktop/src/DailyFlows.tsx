import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LazyMotion, m } from "framer-motion";
import { api, appError } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { StateMessage } from "./Surface";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import {
  DESTINOS,
  HUMORES,
  SECUNDARIOS_SUGERIDOS,
  avisoDeCarregado,
  carryOverEmOrdem,
  destinos,
  emOrdem,
  linhaDeObjetivo,
  podeIniciar,
  rascunho,
  rascunhoValido,
  resumoDoDia,
} from "./daily";
import type {
  DailyContext,
  DailyToday,
  DayMood,
  ObjectiveDraft,
  ObjectiveLink,
  ObjectiveStatus,
} from "./types";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/**
 * Os dois fluxos do ciclo do dia: **Start My Day** e **End My Day**.
 *
 * # O que estes componentes NÃO fazem
 *
 * Eles não decidem nada. Progresso, carry-over, unicidade do dia e conclusão
 * automática vivem em `mos-core::daily`; o que é apresentação — em que estado a
 * Home está, o que o resumo diz, quantas vagas sobraram — vive em `daily.ts`,
 * que é puro e tem teste. Aqui só há tela.
 *
 * # Por que duas etapas, e não uma nem sete
 *
 * Uma tela só obrigaria a rolar entre "o que existe hoje" e "o que eu escolho",
 * e a segunda decisão depende da primeira ter sido lida. Um wizard de cinco
 * passos transformaria trinta segundos em burocracia. Duas etapas separam
 * exatamente as duas perguntas: **o que há** e **o que importa**.
 */

// ===========================================================================
// O seletor de objetivo
// ===========================================================================

type Sugestao = { titulo: string; detalhe: string; link: ObjectiveLink | null };

/**
 * Escolhe uma Task, um Project — ou escreve uma intenção livre.
 *
 * A busca é a MESMA do Command (`api.search`), e não uma lista própria: um
 * segundo índice de Tasks daria dois resultados diferentes para a mesma palavra.
 * Com o campo vazio, o que aparece são as sugestões que o contexto do dia já
 * trouxe — Tasks em andamento primeiro, que é a resposta a "o que eu estava
 * fazendo".
 *
 * O texto digitado que não casa com nada continua valendo: **um objetivo pode
 * não existir em lugar nenhum do M/OS**, e é isso que separa a Daily Session de
 * mais uma base de tarefas.
 */
function DailyObjectivePicker({
  contexto,
  valor,
  aoEscolher,
  aoLimpar,
  autoFoco = false,
  rotulo,
  dica,
}: {
  contexto: DailyContext | null;
  valor: ObjectiveDraft | null;
  aoEscolher: (draft: ObjectiveDraft) => void;
  aoLimpar: () => void;
  autoFoco?: boolean;
  rotulo: string;
  dica: string;
}) {
  const [texto, setTexto] = useState("");
  const [achados, setAchados] = useState<Sugestao[]>([]);
  const [aberto, setAberto] = useState(false);
  const campo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (autoFoco) campo.current?.focus();
  }, [autoFoco]);

  /* As sugestões do contexto, quando o campo está vazio. Elas já chegam
     ordenadas pelo domínio — `doing` antes do resto —, então a tela não
     reordena: fazer isso aqui criaria uma segunda regra de relevância. */
  const sugestoes = useMemo<Sugestao[]>(() => {
    if (!contexto) return [];
    return [
      ...contexto.suggestedTasks.map((task) => ({
        titulo: task.title,
        detalhe: task.project ? `task · ${task.project}` : "task",
        link: { kind: "task", id: task.id } as ObjectiveLink,
      })),
      ...contexto.suggestedProjects.map((project) => ({
        titulo: project.name,
        detalhe: project.openTasks ? `project · ${project.openTasks} abertas` : "project",
        link: { kind: "project", id: project.id } as ObjectiveLink,
      })),
    ];
  }, [contexto]);

  /* A busca é adiada em 160ms. Sem isso, cada tecla vira uma varredura de FTS —
     e o campo de um fluxo que promete ser rápido não pode ser o que engasga. */
  useEffect(() => {
    const termo = texto.trim();
    if (termo.length < 2) {
      setAchados([]);
      return;
    }
    let cancelado = false;
    const timer = window.setTimeout(() => {
      void api
        .search(termo, false)
        .then((itens) => {
          if (cancelado) return;
          setAchados(
            itens
              .flatMap((item): Sugestao[] => {
                if (item.kind === "task") {
                  return [{
                    titulo: item.task.title,
                    detalhe: item.project ? `task · ${item.project.name}` : "task",
                    link: { kind: "task", id: item.task.id },
                  }];
                }
                if (item.kind === "project") {
                  return [{ titulo: item.project.name, detalhe: "project", link: { kind: "project", id: item.project.id } }];
                }
                return [];
              })
              .slice(0, 6),
          );
        })
        /* Busca que falha vira lista vazia, e não erro na tela: o texto livre
           continua funcionando, e o fluxo inteiro não pode cair porque o índice
           não respondeu. */
        .catch(() => setAchados([]));
    }, 160);
    return () => {
      cancelado = true;
      window.clearTimeout(timer);
    };
  }, [texto]);

  if (valor && rascunhoValido(valor)) {
    return (
      <div className="daily-picked" data-kind={valor.linkKind || "free"}>
        <span className="daily-picked-title">{valor.title}</span>
        <span className="micro-label">{valor.linkKind || "intenção"}</span>
        <button type="button" className="icon-button" aria-label={`Trocar ${rotulo}`} onClick={() => { setTexto(""); aoLimpar(); }}>
          <Icon name="close" />
        </button>
      </div>
    );
  }

  const lista = texto.trim().length >= 2 ? achados : sugestoes.slice(0, 6);

  function escolherTexto() {
    if (!texto.trim()) return;
    aoEscolher(rascunho(texto, null));
    setTexto("");
    setAberto(false);
  }

  return (
    <div className="daily-picker">
      <input
        ref={campo}
        value={texto}
        placeholder={dica}
        aria-label={rotulo}
        onFocus={() => setAberto(true)}
        onChange={(event) => { setTexto(event.currentTarget.value); setAberto(true); }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            escolherTexto();
          }
          if (event.key === "Escape" && aberto) {
            event.stopPropagation();
            setAberto(false);
          }
        }}
        /* O blur fecha com atraso: fechar na hora mataria a lista antes de o
           clique nela chegar a acontecer. */
        onBlur={() => window.setTimeout(() => setAberto(false), 120)}
      />
      {aberto && (lista.length > 0 || texto.trim().length > 0) ? (
        <div className="daily-picker-list" role="listbox" aria-label={`Sugestões para ${rotulo}`}>
          {texto.trim() ? (
            <button type="button" role="option" aria-selected="false" className="daily-picker-free" onMouseDown={(event) => event.preventDefault()} onClick={escolherTexto}>
              <strong>{texto.trim()}</strong>
              <span className="micro-label">INTENÇÃO LIVRE</span>
            </button>
          ) : null}
          {lista.map((sugestao) => (
            <button
              key={`${sugestao.link?.kind}-${sugestao.link?.id}-${sugestao.titulo}`}
              type="button"
              role="option"
              aria-selected="false"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => { aoEscolher(rascunho(sugestao.titulo, sugestao.link)); setTexto(""); setAberto(false); }}
            >
              <strong>{sugestao.titulo}</strong>
              <span className="micro-label">{sugestao.detalhe}</span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

// ===========================================================================
// Start My Day
// ===========================================================================

/** O resumo do que já existe hoje. Números, e nunca adjetivos. */
function DailyContextSummary({ contexto }: { contexto: DailyContext | null }) {
  const linhas = resumoDoDia(contexto);
  if (!linhas.length) {
    return (
      <p className="daily-context-empty">
        Nada vencendo, nada atrasado, nada esperando. O dia está aberto.
      </p>
    );
  }
  return (
    <ul className="daily-context">
      {linhas.map((linha) => (
        <li key={linha.chave}>
          <strong>{linha.valor}</strong>
          <span>{linha.texto.replace(`${linha.valor} `, "")}</span>
        </li>
      ))}
    </ul>
  );
}

export function StartMyDayFlow({
  close,
  concluido,
}: {
  close: () => void;
  concluido: (dia: DailyToday) => void;
}) {
  const [etapa, setEtapa] = useState<"hoje" | "objetivos">("hoje");
  const [contexto, setContexto] = useState<DailyContext | null>(null);
  const [carregando, setCarregando] = useState(true);
  const [erro, setErro] = useState("");
  const [salvando, setSalvando] = useState(false);
  const [principal, setPrincipal] = useState<ObjectiveDraft | null>(null);
  const [secundarios, setSecundarios] = useState<(ObjectiveDraft | null)[]>([null, null, null]);
  const painel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelado = false;
    void api
      .dailyContext()
      .then((achado) => { if (!cancelado) { setContexto(achado); setErro(""); } })
      /* O contexto é o que ORIENTA a escolha, e não o que a permite. Se ele
         falhar, o fluxo continua: montar o dia com texto livre é melhor que não
         poder começar o dia porque uma contagem não pôde ser lida. */
      .catch((falha) => { if (!cancelado) setErro(appError(falha).message); })
      .finally(() => { if (!cancelado) setCarregando(false); });
    return () => { cancelado = true; };
  }, []);

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

  const carryOver = useMemo(() => carryOverEmOrdem(contexto?.carryOver ?? []), [contexto]);

  /* Quais carry-overs estão marcados NÃO é um estado próprio: é o que os
     rascunhos já dizem, pelo `carriedFrom` que cada um carrega. Um `Set`
     paralelo seria uma segunda fonte de verdade sobre a mesma pergunta, e ela
     divergiria no primeiro momento em que a pessoa trocasse uma vaga à mão. */
  const carregados = useMemo(() => {
    const vagas = [principal, ...secundarios];
    return new Set(vagas.flatMap((vaga) => (vaga?.carriedFrom ? [vaga.carriedFrom] : [])));
  }, [principal, secundarios]);

  /** Um carry-over marcado entra na primeira vaga livre — principal, se houver. */
  const alternarCarregado = useCallback(
    (objectiveId: string) => {
      const item = carryOver.find((candidato) => candidato.objectiveId === objectiveId);
      if (!item) return;

      if (carregados.has(objectiveId)) {
        // Desmarcar tira o rascunho de onde ele estiver.
        setPrincipal((atual) => (atual?.carriedFrom === objectiveId ? null : atual));
        setSecundarios((atual) => atual.map((vaga) => (vaga?.carriedFrom === objectiveId ? null : vaga)));
        return;
      }

      const draft = rascunho(item.title, item.link, item.objectiveId);
      if (!principal) {
        setPrincipal(draft);
        return;
      }
      const livre = secundarios.findIndex((vaga) => !vaga);
      // Sem vaga livre, marcar não faz nada — e o botão fica sem resposta de
      // propósito: encher o dia por cima de uma escolha que a pessoa acabou de
      // fazer seria pior que não reagir.
      if (livre < 0) return;
      setSecundarios((atual) => atual.map((vaga, at) => (at === livre ? draft : vaga)));
    },
    [carryOver, carregados, principal, secundarios],
  );

  async function confirmar() {
    const escolhidos = secundarios.filter((vaga): vaga is ObjectiveDraft => Boolean(vaga && rascunhoValido(vaga)));
    if (!podeIniciar(principal, escolhidos) || salvando) return;
    setSalvando(true);
    try {
      const dia = await api.dailyStart({
        main: principal && rascunhoValido(principal) ? principal : null,
        secondaries: escolhidos,
      });
      concluido(dia);
      close();
    } catch (falha) {
      setErro(appError(falha).message);
      setSalvando(false);
    }
  }

  const escolhidos = secundarios.filter((vaga): vaga is ObjectiveDraft => Boolean(vaga && rascunhoValido(vaga)));
  const pronto = podeIniciar(principal, escolhidos);

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
        aria-label="Iniciar meu dia"
        className="daily-flow"
        ref={painel}
        role="dialog"
        tabIndex={-1}
        initial={{ opacity: 0, scale: 0.985, y: -6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.99, y: -4 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <header className="daily-flow-head">
          <span className="micro-label">{etapa === "hoje" ? "HOJE" : "O QUE IMPORTA HOJE"}</span>
          <button type="button" className="icon-button" aria-label="Fechar" onClick={close}>
            <Icon name="close" />
          </button>
        </header>

        {carregando ? <StateMessage state="loading" label="Lendo o dia..." /> : null}

        {etapa === "hoje" ? (
          <div className="daily-flow-body">
            <DailyContextSummary contexto={contexto} />

            {carryOver.length ? (
              <section className="daily-carry" aria-labelledby="daily-carry-head">
                <span className="micro-label" id="daily-carry-head">
                  DE {contexto?.carryOverDay ? contexto.carryOverDay.split("-").reverse().slice(0, 2).join("/") : "ONTEM"}
                </span>
                {/* O que sobrou não é imposto: a pessoa decide se ainda é
                    prioridade. Marcar aqui só preenche uma vaga da etapa
                    seguinte, e ela continua editável lá. */}
                {carryOver.map((item) => {
                  const marcado = carregados.has(item.objectiveId);
                  const aviso = avisoDeCarregado(item.timesCarried);
                  return (
                    <button
                      key={item.objectiveId}
                      type="button"
                      className="daily-carry-item"
                      aria-pressed={marcado}
                      data-selected={marcado || undefined}
                      onClick={() => alternarCarregado(item.objectiveId)}
                    >
                      <span aria-hidden="true">{marcado ? "●" : "○"}</span>
                      <strong>{item.title}</strong>
                      {aviso ? <span className="micro-label">{aviso}</span> : null}
                    </button>
                  );
                })}
              </section>
            ) : null}

            {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}

            <div className="form-actions">
              <Button onClick={close} variant="ghost">Agora não</Button>
              <Button onClick={() => setEtapa("objetivos")} variant="primary">Escolher objetivos</Button>
            </div>
          </div>
        ) : (
          <div className="daily-flow-body">
            <section className="daily-slot" data-main="true">
              <span className="micro-label">PRINCIPAL</span>
              <p className="daily-slot-hint">O que faria o dia valer a pena mesmo sozinho.</p>
              <DailyObjectivePicker
                contexto={contexto}
                valor={principal}
                autoFoco
                rotulo="objetivo principal"
                dica="Finalizar detalhamento do 063-26"
                aoEscolher={setPrincipal}
                aoLimpar={() => setPrincipal(null)}
              />
            </section>

            <section className="daily-slot">
              <span className="micro-label">SECUNDÁRIOS</span>
              <p className="daily-slot-hint">Até {SECUNDARIOS_SUGERIDOS} coisas que também devem avançar.</p>
              {secundarios.map((vaga, indice) => (
                <DailyObjectivePicker
                  key={indice}
                  contexto={contexto}
                  valor={vaga}
                  rotulo={`objetivo secundário ${indice + 1}`}
                  dica={indice === 0 ? "Revisar memorial" : "Opcional"}
                  aoEscolher={(draft) => setSecundarios((atual) => atual.map((item, at) => (at === indice ? draft : item)))}
                  aoLimpar={() => setSecundarios((atual) => atual.map((item, at) => (at === indice ? null : item)))}
                />
              ))}
            </section>

            {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}

            <div className="form-actions">
              <Button onClick={() => setEtapa("hoje")} variant="ghost">Voltar</Button>
              <Button disabled={!pronto || salvando} onClick={() => void confirmar()} variant="primary">
                {salvando ? "Começando" : "Começar o dia"}
              </Button>
            </div>
          </div>
        )}
      </m.div>
    </LazyMotion>
  );
}

// ===========================================================================
// End My Day
// ===========================================================================

/**
 * A reflexão. Uma pergunta, e um campo.
 *
 * Quatro campos rotulados — resumo, vitórias, bloqueios, notas — é um formulário
 * de journaling, e o pedido recusa isso por nome. O que sobrou responde a mesma
 * coisa: três botões para o que dá para responder em um clique, e um campo para
 * o que não dá.
 */
function DailyReflectionInput({
  humor,
  resumo,
  aoMudarHumor,
  aoMudarResumo,
}: {
  humor: DayMood | "";
  resumo: string;
  aoMudarHumor: (proximo: DayMood | "") => void;
  aoMudarResumo: (proximo: string) => void;
}) {
  return (
    <section className="daily-reflection" aria-labelledby="daily-reflection-head">
      <span className="micro-label" id="daily-reflection-head">COMO FOI O DIA?</span>
      <div className="daily-moods" role="group" aria-label="Como foi o dia">
        {HUMORES.map((opcao) => (
          <Button
            key={opcao.valor}
            size="sm"
            aria-pressed={humor === opcao.valor}
            variant={humor === opcao.valor ? "primary" : "ghost"}
            /* Clicar de novo desmarca: "não quero responder" tem de continuar
               alcançável depois de um clique acidental. */
            onClick={() => aoMudarHumor(humor === opcao.valor ? "" : opcao.valor)}
          >
            {opcao.rotulo}
          </Button>
        ))}
      </div>
      <textarea
        rows={3}
        value={resumo}
        aria-label="Resumo do dia"
        placeholder="Opcional"
        onChange={(evento) => aoMudarResumo(evento.currentTarget.value)}
      />
    </section>
  );
}

export function EndMyDayFlow({
  dia,
  sessaoAntiga = null,
  close,
  concluido,
}: {
  dia: DailyToday;
  /** Quando presente, o fluxo encerra ESTA sessão — o "encerrar ontem". */
  sessaoAntiga?: string | null;
  close: () => void;
  concluido: (dia: DailyToday) => void;
}) {
  const objetivos = useMemo(() => emOrdem(dia.objectives), [dia.objectives]);
  const [escolhas, setEscolhas] = useState<Map<string, ObjectiveStatus>>(() => new Map());
  const [humor, setHumor] = useState<DayMood | "">("");
  const [resumo, setResumo] = useState("");
  const [erro, setErro] = useState("");
  const [salvando, setSalvando] = useState(false);
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

  const pendentes = objetivos.filter((objetivo) => objetivo.status === "pending");

  async function confirmar() {
    if (salvando) return;
    setSalvando(true);
    try {
      const fechado = await api.dailyEnd(
        { resolutions: destinos(escolhas), mood: humor, summary: resumo },
        sessaoAntiga,
      );
      concluido(fechado);
      close();
    } catch (falha) {
      setErro(appError(falha).message);
      setSalvando(false);
    }
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
        aria-label="Encerrar meu dia"
        className="daily-flow"
        ref={painel}
        role="dialog"
        tabIndex={-1}
        initial={{ opacity: 0, scale: 0.985, y: -6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.99, y: -4 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <header className="daily-flow-head">
          <span className="micro-label">ENCERRAR {sessaoAntiga ? "O DIA EM ABERTO" : "O DIA"}</span>
          <button type="button" className="icon-button" aria-label="Fechar" onClick={close}>
            <Icon name="close" />
          </button>
        </header>

        <div className="daily-flow-body">
          {objetivos.length ? (
            <ul className="daily-close-list">
              {objetivos.map((objetivo) => {
                const linha = linhaDeObjetivo(objetivo);
                const escolhido = escolhas.get(objetivo.id);
                return (
                  <li key={objetivo.id} data-main={linha.principal || undefined}>
                    <div className="daily-close-title">
                      <span aria-hidden="true">{escolhido === "completed" ? "✓" : linha.marcador}</span>
                      <strong>{linha.titulo}</strong>
                      {linha.principal ? <span className="micro-label">PRINCIPAL</span> : null}
                    </div>
                    {/* Só o pendente pede decisão. Um objetivo já concluído
                        durante o dia aparece aqui como registro, e não como
                        pergunta — reperguntar seria desfazer o que já foi
                        respondido. */}
                    {objetivo.status === "pending" ? (
                      <div className="daily-close-choices" role="group" aria-label={`Destino de ${linha.titulo}`}>
                        {DESTINOS.map((destino) => (
                          <Button
                            key={destino.valor}
                            size="sm"
                            title={destino.explica}
                            aria-pressed={escolhido === destino.valor}
                            variant={escolhido === destino.valor ? "primary" : "ghost"}
                            onClick={() =>
                              setEscolhas((atual) => {
                                const proximo = new Map(atual);
                                if (proximo.get(objetivo.id) === destino.valor) proximo.delete(objetivo.id);
                                else proximo.set(objetivo.id, destino.valor);
                                return proximo;
                              })
                            }
                          >
                            {destino.rotulo}
                          </Button>
                        ))}
                      </div>
                    ) : (
                      <span className="daily-close-state micro-label">{linha.estado.toUpperCase()}</span>
                    )}
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="daily-context-empty">Este dia não teve objetivos registrados.</p>
          )}

          {/* Nada de confete e nada de streak. O dia todo concluído recebe uma
              frase, e o resto do fluxo continua igual. */}
          {pendentes.length === 0 && objetivos.length > 0 ? (
            <p className="daily-close-all" role="status">Tudo o que você escolheu hoje está resolvido.</p>
          ) : null}

          <DailyReflectionInput humor={humor} resumo={resumo} aoMudarHumor={setHumor} aoMudarResumo={setResumo} />

          {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}

          <div className="form-actions">
            <Button onClick={close} variant="ghost">Voltar</Button>
            <Button disabled={salvando} onClick={() => void confirmar()} variant="primary">
              {salvando ? "Encerrando" : "Encerrar o dia"}
            </Button>
          </div>
          {/* O que ficou sem destino continua pendente, e volta amanhã. Dizer
              isso em voz alta é o que impede a pessoa de achar que precisa
              responder tudo para poder sair. */}
          {pendentes.some((objetivo) => !escolhas.has(objetivo.id)) ? (
            <p className="daily-close-note">
              O que você não decidir fica pendente, e reaparece amanhã.
            </p>
          ) : null}
        </div>
      </m.div>
    </LazyMotion>
  );
}

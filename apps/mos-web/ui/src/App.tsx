import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import {
  api,
  pedeAtencao,
  SemSessao,
  type AlvoDoLembrete,
  type Capture,
  type EstadoDoAparelho,
  type Lembrete,
  type Task,
} from "./api";
import { ativar, situacao, type Situacao } from "./notificacoes";
import { Porta } from "./Porta";
import { Quando } from "./Quando";
import { daquiA, porExtenso } from "./instantes";

type Aba = "capturar" | "inbox" | "tasks" | "lembretes";

/** O que a folha de *quando* está agendando, enquanto ela está aberta. */
type Agendamento = {
  titulo: string;
  descricao: string;
  alvo?: AlvoDoLembrete;
};

/** Os estados de Task, na palavra que a tela usa. */
const ESTADO_DA_TASK: Record<Task["state"], string> = {
  inbox: "Inbox",
  backlog: "Backlog",
  planned: "Planejada",
  doing: "Em andamento",
  review: "Revisão",
  done: "Feita",
};

/**
 * O M/OS de bolso.
 *
 * # A decisão de layout que manda em todas as outras
 *
 * O compositor mora EMBAIXO, no alcance do polegar, e não no topo. Capturar é a
 * única coisa que este app existe para fazer sem atrito, e no topo a caixa fica
 * onde a mão não chega sem reposicionar o aparelho.
 *
 * # Por que não há tela de carregando
 *
 * Uma lista que ainda não chegou aparece vazia com uma frase, e não com um
 * spinner: o app abre já podendo capturar, e a inbox é o que se olha depois. Um
 * spinner na frente do compositor atrasaria a única coisa urgente.
 *
 * # Por que a aba de notificações virou a aba de LEMBRETES
 *
 * Ela mostrava configuração e nada mais — dois botões e uma frase sobre a Tela
 * de Início. Configuração não é conteúdo, e uma das quatro abas do app ficava
 * gasta com uma decisão que se toma uma vez na vida. Agora ela mostra os
 * lembretes, que é a coisa que a notificação existe para entregar, e o canal
 * aparece embaixo: em destaque enquanto está desligado, numa linha depois que
 * liga.
 */
export function App() {
  const [aba, setAba] = useState<Aba>("capturar");
  const [texto, setTexto] = useState("");
  const [capturas, setCapturas] = useState<Capture[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [lembretes, setLembretes] = useState<Lembrete[]>([]);
  const [estado, setEstado] = useState<EstadoDoAparelho | null>(null);
  const [recado, setRecado] = useState("");
  const [erro, setErro] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  const [avisos, setAvisos] = useState<Situacao | null>(null);
  const [agendando, setAgendando] = useState<Agendamento | null>(null);
  /** `true` enquanto o servidor recusar por falta de sessão. */
  const [fechado, setFechado] = useState(false);

  const atualizar = useCallback(async () => {
    // O `estado` é o que decide se há sessão: ele é a chamada mais barata, e um
    // 401 aqui vale por todas — pedir inbox e tasks para depois descobrir que
    // ninguém entrou seriam dois 401 a mais para nada.
    let proximoEstado: EstadoDoAparelho | null = null;
    try {
      proximoEstado = await api.estado();
      setFechado(false);
    } catch (causa) {
      if (causa instanceof SemSessao) {
        setFechado(true);
        return;
      }
    }
    const [proximaInbox, proximasTasks, proximosLembretes] = await Promise.all([
      api.inbox().catch(() => [] as Capture[]),
      api.tasks().catch(() => [] as Task[]),
      api.lembretes().catch(() => [] as Lembrete[]),
    ]);
    if (proximoEstado) setEstado(proximoEstado);
    setCapturas(proximaInbox);
    setTasks(proximasTasks);
    setLembretes(proximosLembretes);
    // A situação das notificações é recalculada junto: ela muda por fora do app
    // — instalar na tela de início, mexer em Ajustes —, e uma tela que só olha
    // uma vez ficaria dizendo "instale" depois de você já ter instalado.
    setAvisos(await situacao(proximoEstado?.chavePush ?? null));
  }, []);

  useEffect(() => {
    void atualizar();
    // O servidor sincroniza sozinho a cada minuto; a tela olha de novo com a
    // mesma cadência para a fila não mentir enquanto o app está aberto.
    const relogio = window.setInterval(() => void atualizar(), 30_000);
    return () => window.clearInterval(relogio);
  }, [atualizar]);

  function contar(mensagem: string, falhou = false) {
    setRecado(mensagem);
    setErro(falhou);
  }

  /** O que a última ação falhou dizendo, na frase que o servidor mandou. */
  function reclamar(causa: unknown) {
    contar(causa instanceof Error ? causa.message : String(causa), true);
  }

  async function capturar(evento: FormEvent) {
    evento.preventDefault();
    const conteudo = texto.trim();
    if (!conteudo) return;
    setOcupado(true);
    try {
      await api.capturar(conteudo);
      // O campo esvazia só depois do sucesso. Esvaziar antes e falhar depois
      // apagaria o que a pessoa acabou de escrever — e ela não tem cópia.
      setTexto("");
      contar("Guardado.");
      await atualizar();
    } catch (causa) {
      reclamar(causa);
    }
    setOcupado(false);
  }

  async function novaTask(evento: FormEvent) {
    evento.preventDefault();
    const titulo = texto.trim();
    if (!titulo) return;
    setOcupado(true);
    try {
      await api.criarTask(titulo);
      setTexto("");
      contar("Task criada.");
      await atualizar();
    } catch (causa) {
      reclamar(causa);
    }
    setOcupado(false);
  }

  /**
   * O compositor da aba de lembretes não cria nada sozinho: ele abre a folha de
   * *quando*.
   *
   * Um lembrete sem hora não é um lembrete — seria uma Task com outro nome —, e
   * inventar uma hora padrão para não perguntar produziria a pior falha
   * possível: um lembrete que toca em hora que ninguém escolheu.
   */
  function agendarSolto(evento: FormEvent) {
    evento.preventDefault();
    const titulo = texto.trim();
    if (!titulo) return;
    setAgendando({ titulo, descricao: "NOVO LEMBRETE" });
  }

  async function alternar(task: Task) {
    const destino = task.state === "done" ? "doing" : "done";
    // Otimista: a marca muda antes da resposta. O gesto precisa parecer
    // instantâneo, e o servidor grava local — se falhar, o `atualizar` do
    // `catch` devolve a verdade.
    setTasks((atuais) =>
      atuais.map((t) => (t.id === task.id ? { ...t, state: destino } : t)),
    );
    try {
      await api.mudarEstado(task.id, destino);
      await atualizar();
    } catch (causa) {
      reclamar(causa);
      await atualizar();
    }
  }

  /** A folha respondeu. Cria, avisa e volta para a lista de onde ela saiu. */
  async function criarLembrete(quando: Date) {
    if (!agendando) return;
    const pedido = agendando;
    setOcupado(true);
    try {
      await api.criarLembrete(pedido.titulo, quando, "", pedido.alvo);
      setAgendando(null);
      // O texto só some quando o lembrete existe, pela mesma razão da captura.
      // Um lembrete preso a uma Task não veio do compositor, e apagá-lo ali
      // jogaria fora o que a pessoa estava escrevendo em outra aba.
      if (!pedido.alvo) setTexto("");
      // A confirmação repete a HORA, e não "criado". A única forma de descobrir
      // que se agendou para o dia errado é ler o dia — e depois que a folha
      // fecha não há mais onde ler.
      contar(`Lembrete para ${porExtenso(quando)}.`);
      await atualizar();
    } catch (causa) {
      reclamar(causa);
      setAgendando(null);
    }
    setOcupado(false);
  }

  async function resolverLembrete(lembrete: Lembrete, como: "concluir" | "cancelar") {
    setOcupado(true);
    try {
      if (como === "concluir") await api.concluirLembrete(lembrete.id);
      else await api.cancelarLembrete(lembrete.id);
      contar(como === "concluir" ? "Lembrete concluído." : "Lembrete cancelado.");
      await atualizar();
    } catch (causa) {
      reclamar(causa);
    }
    setOcupado(false);
  }

  async function ativarAvisos() {
    if (!estado?.chavePush) return;
    setOcupado(true);
    try {
      await ativar(estado.chavePush);
      contar("Notificações ativadas.");
      await atualizar();
    } catch (causa) {
      reclamar(causa);
    }
    setOcupado(false);
  }

  async function testarAvisos() {
    setOcupado(true);
    try {
      const { enviadas } = await api.testarPush();
      contar(
        enviadas > 0
          ? "Mandei. Se não chegar em alguns segundos, algo está errado."
          : "Nenhum aparelho assinado para receber.",
        enviadas === 0,
      );
    } catch (causa) {
      reclamar(causa);
    }
    setOcupado(false);
  }

  const pendentes = estado?.pendentes ?? 0;

  /**
   * Quais Tasks já têm lembrete vivo.
   *
   * Serve para o sino não oferecer um segundo lembrete para a mesma coisa sem
   * dizer que já existe um — o jeito mais fácil de acabar com três notificações
   * idênticas às nove da manhã.
   */
  const tasksLembradas = useMemo(() => {
    const encontradas = new Set<string>();
    for (const lembrete of lembretes) {
      if (lembrete.target?.type === "task") encontradas.add(lembrete.target.id);
    }
    return encontradas;
  }, [lembretes]);

  /** Só o que realmente espera uma ação: é o número que o badge pode mostrar. */
  const cobrando = useMemo(() => lembretes.filter(pedeAtencao).length, [lembretes]);

  // A porta ocupa a tela inteira, e não um cartaz por cima do app: um app
  // visível atrás de um aviso de login convida a tocar no que não responde.
  if (fechado) {
    return <Porta aoEntrar={() => void atualizar()} />;
  }

  const canal = avisos?.estado ?? null;

  return (
    <div className="app">
      <header className="topo">
        <span className="marca">M/OS</span>
        <span className="fila" data-estado={sinalDaFila(estado, pendentes)}>
          <i aria-hidden="true" />
          {estado?.sincroniza === false
            ? "SEM HUB"
            : pendentes > 0
              ? `${pendentes} NA FILA`
              : "EM DIA"}
        </span>
      </header>

      <nav className="abas" aria-label="Seções">
        {(
          [
            ["capturar", "Capturar", 0],
            ["inbox", "Inbox", capturas.length],
            ["tasks", "Tasks", tasks.length],
            ["lembretes", "Lembretes", cobrando],
          ] as const
        ).map(([valor, rotulo, conta]) => (
          <button
            key={valor}
            type="button"
            aria-current={aba === valor ? "page" : undefined}
            onClick={() => setAba(valor)}
          >
            <span>{rotulo}</span>
            {/* O badge de lembretes conta só o que cobra ação — `scheduled` não
                entra. Um badge que sobe com coisa que ainda não é hora é um
                badge que se aprende a ignorar (`ATTENTION-SYSTEM.md` §21.1). */}
            {conta > 0 ? (
              <b className="conta" data-urgente={valor === "lembretes" || undefined}>
                {conta}
              </b>
            ) : null}
          </button>
        ))}
      </nav>

      <main className="conteudo">
        {/* A aba de capturar mostra o que ACABOU de entrar.
            Ela era uma frase de instrução e trezentos pixels de nada, e o vazio
            fazia a tela parecer quebrada logo depois de guardar algo. As três
            últimas capturas custam o mesmo pedido que a inbox já faz e
            respondem à única pergunta de quem acabou de escrever: entrou? */}
        {aba === "capturar" ? (
          capturas.length === 0 ? (
            <p className="vazio">
              O que estiver na cabeça vai para a Inbox. Organizar é depois.
            </p>
          ) : (
            <>
              <p className="rotulo">ÚLTIMAS</p>
              <ul className="lista">
                {capturas.slice(0, 3).map((capture) => (
                  <li className="item" key={capture.id}>
                    <div className="item-corpo">
                      <p>{capture.content}</p>
                      <small>{quando(capture.capturedAt)}</small>
                    </div>
                  </li>
                ))}
              </ul>
            </>
          )
        ) : null}

        {aba === "inbox" ? (
          capturas.length === 0 ? (
            <p className="vazio">Inbox vazia.</p>
          ) : (
            <ul className="lista">
              {capturas.map((capture) => (
                <li className="item" key={capture.id}>
                  <div className="item-corpo">
                    <p>{capture.content}</p>
                    <small>{quando(capture.capturedAt)}</small>
                  </div>
                </li>
              ))}
            </ul>
          )
        ) : null}

        {aba === "tasks" ? (
          tasks.length === 0 ? (
            <p className="vazio">Nenhuma task ativa.</p>
          ) : (
            <ul className="lista">
              {tasks.map((task) => {
                const lembrada = tasksLembradas.has(task.id);
                return (
                  <li className="item" key={task.id}>
                    <button
                      className="marcar"
                      type="button"
                      aria-pressed={task.state === "done"}
                      aria-label={
                        task.state === "done"
                          ? `Reabrir ${task.title}`
                          : `Concluir ${task.title}`
                      }
                      onClick={() => void alternar(task)}
                    >
                      <span aria-hidden="true" />
                    </button>
                    <div className="item-corpo">
                      <p>{task.title}</p>
                      <small>
                        {ESTADO_DA_TASK[task.state]}
                        {lembrada ? " · com lembrete" : ""}
                      </small>
                    </div>
                    {/* O sino, e não um menu de três pontos: é a única ação que
                        esta linha oferece além de marcar, e escondê-la atrás de
                        um menu custaria dois toques para ganhar nada. */}
                    <button
                      className="sino"
                      type="button"
                      data-ligado={lembrada || undefined}
                      aria-label={
                        lembrada
                          ? `Outro lembrete para ${task.title}`
                          : `Lembrar de ${task.title}`
                      }
                      onClick={() =>
                        setAgendando({
                          // O título do lembrete é o da Task: quem toca no sino
                          // quer ser lembrado DELA, e pedir um texto novo aqui
                          // seria pedir para escrever de novo o que já está na
                          // linha acima do dedo.
                          titulo: task.title,
                          descricao: lembrada
                            ? "JÁ HÁ UM LEMBRETE PARA ESTA TASK"
                            : "LEMBRAR DESTA TASK",
                          alvo: { type: "task", id: task.id },
                        })
                      }
                    >
                      <SinoIcone />
                    </button>
                  </li>
                );
              })}
            </ul>
          )
        ) : null}

        {aba === "lembretes" ? (
          <div className="lembretes">
            {lembretes.length === 0 ? (
              <p className="vazio">
                Nenhum lembrete esperando. Escreva embaixo, ou toque no sino de
                uma Task.
              </p>
            ) : (
              <ul className="lista">
                {lembretes.map((lembrete) => {
                  const cobra = pedeAtencao(lembrete);
                  return (
                    <li className="item" key={lembrete.id} data-cobra={cobra || undefined}>
                      <div className="item-corpo">
                        <p>{lembrete.title}</p>
                        <small>
                          {daquiA(lembrete.nextDueAt)}
                          {lembrete.target?.type === "task" ? " · task" : ""}
                          {lembrete.snoozeCount > 0
                            ? ` · adiado ${lembrete.snoozeCount}×`
                            : ""}
                        </small>
                      </div>
                      {/* Concluir e cancelar, e não adiar: adiar mexe na hora do
                          vencimento, que é a coluna que o agendador do PC lê —
                          ver `api.rs`. As duas daqui levam o lembrete para
                          estado terminal, e depois delas não há o que disputar. */}
                      <div className="item-acoes">
                        <button
                          className="acao"
                          type="button"
                          disabled={ocupado}
                          aria-label={`Concluir ${lembrete.title}`}
                          onClick={() => void resolverLembrete(lembrete, "concluir")}
                        >
                          Feito
                        </button>
                        <button
                          className="acao"
                          type="button"
                          data-variante="quieto"
                          disabled={ocupado}
                          aria-label={`Cancelar ${lembrete.title}`}
                          onClick={() => void resolverLembrete(lembrete, "cancelar")}
                        >
                          Cancelar
                        </button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}

            {/* O CANAL.
                Ele fica embaixo dos lembretes, e não em cima: enquanto está
                desligado ele é um bloco que explica o passo que falta; depois
                que liga, vira uma linha. Configuração resolvida não merece
                ocupar o topo de uma tela todo dia. */}
            <section className="canal" data-estado={canal ?? "conferindo"}>
              {canal === "ativo" ? (
                <>
                  <p className="canal-linha">
                    <i aria-hidden="true" />
                    Notificações ativas ·{" "}
                    {estado?.aparelhosAvisados === 1
                      ? "1 aparelho"
                      : `${estado?.aparelhosAvisados ?? 0} aparelhos`}
                  </p>
                  <button
                    className="botao"
                    data-variante="quieto"
                    type="button"
                    disabled={ocupado}
                    onClick={() => void testarAvisos()}
                  >
                    Enviar um teste agora
                  </button>
                </>
              ) : (
                <>
                  <p className="rotulo">NOTIFICAÇÃO</p>
                  {/* A frase vem antes do botão de propósito: no iPhone o botão
                      só funciona depois de instalar na tela de início, e um
                      botão que falha calado é pior que um botão ausente. */}
                  <p className="explicacao">{porQueNaoAtivo(avisos)}</p>

                  {/* O que ele vai avisar, dito antes de você decidir ativar.
                      Permissão de notificação é um sim ou não sem volta fácil no
                      iPhone, e concedê-la sem saber o que vai chegar é o começo
                      do app que a pessoa silencia na semana seguinte. */}
                  {canal === "impossivel" ? null : (
                    <ul className="promessas">
                      <li>Lembretes, na hora em que vencerem.</li>
                      <li>Quando o computador mandar coisa nova.</li>
                    </ul>
                  )}

                  {canal === "pronto" ? (
                    <button
                      className="botao"
                      type="button"
                      disabled={ocupado}
                      onClick={() => void ativarAvisos()}
                    >
                      Ativar notificações
                    </button>
                  ) : null}
                </>
              )}
            </section>
          </div>
        ) : null}
      </main>

      <form
        className="compositor"
        onSubmit={
          aba === "tasks" ? novaTask : aba === "lembretes" ? agendarSolto : capturar
        }
      >
        <textarea
          value={texto}
          onChange={(evento) => setTexto(evento.currentTarget.value)}
          placeholder={
            aba === "tasks"
              ? "O que precisa ser feito?"
              : aba === "lembretes"
                ? "Lembrar de…"
                : "O que está na cabeça?"
          }
          aria-label={
            aba === "tasks"
              ? "Nova task"
              : aba === "lembretes"
                ? "Novo lembrete"
                : "Nova captura"
          }
        />
        <div className="linha-de-botoes">
          <button className="botao" type="submit" disabled={ocupado || !texto.trim()}>
            {aba === "tasks"
              ? "Criar task"
              : aba === "lembretes"
                ? "Escolher quando"
                : "Guardar"}
          </button>
        </div>
        <p className="recado" data-estado={erro ? "erro" : "ok"} aria-live="polite">
          {recado}
        </p>
      </form>

      {agendando ? (
        <Quando
          titulo={agendando.titulo}
          descricao={agendando.descricao}
          ocupado={ocupado}
          aoEscolher={(escolhido: Date) => void criarLembrete(escolhido)}
          aoFechar={() => setAgendando(null)}
        />
      ) : null}
    </div>
  );
}

/**
 * A frase do canal desligado.
 *
 * Ela existe como funcao para o `motivo` continuar amarrado ao braco que o tem:
 * `pronto` nao carrega motivo nenhum — nao ha o que explicar quando so falta
 * tocar —, e ler o campo com `?.` faria o compilador aceitar um estado a mais
 * do que o tipo descreve.
 */
function porQueNaoAtivo(avisos: Situacao | null): string {
  if (!avisos) return "Conferindo…";
  switch (avisos.estado) {
    case "pronto":
      return "Ative para receber aqui os lembretes que vencerem, mesmo com o app fechado.";
    case "falta":
    case "impossivel":
      return avisos.motivo;
    case "ativo":
      // Inalcancavel: com o canal ativo esta secao nao e desenhada. Devolver
      // uma frase honesta e melhor que um `throw` numa tela que ja abriu.
      return "Notificações ativas.";
  }
}

/** Verde, âmbar ou apagado — o que a etiqueta da fila está dizendo. */
function sinalDaFila(estado: EstadoDoAparelho | null, pendentes: number): string {
  if (estado?.sincroniza === false) return "sem-hub";
  return pendentes > 0 ? "fila" : "em-dia";
}

/**
 * O sino, desenhado e não importado.
 *
 * Uma biblioteca de ícones para um glifo só custaria mais bytes no 4G do que a
 * tela inteira — e este app abre na rua.
 */
function SinoIcone() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" focusable="false">
      <path
        d="M12 3a5.5 5.5 0 0 0-5.5 5.5c0 3.2-.7 5-1.5 6.1-.4.6 0 1.4.8 1.4h12.4c.8 0 1.2-.8.8-1.4-.8-1.1-1.5-2.9-1.5-6.1A5.5 5.5 0 0 0 12 3Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <path
        d="M10 19a2 2 0 0 0 4 0"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}

/** "há 3 min", "ontem". O relógio exato não ajuda a decidir nada aqui. */
function quando(iso: string): string {
  const momento = new Date(iso).getTime();
  if (Number.isNaN(momento)) return "";
  const minutos = Math.round((Date.now() - momento) / 60_000);
  if (minutos < 1) return "agora";
  if (minutos < 60) return `há ${minutos} min`;
  const horas = Math.round(minutos / 60);
  if (horas < 24) return `há ${horas} h`;
  const dias = Math.round(horas / 24);
  return dias === 1 ? "ontem" : `há ${dias} dias`;
}

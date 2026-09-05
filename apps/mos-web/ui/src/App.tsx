import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import {
  api,
  pedeAtencao,
  SemSessao,
  type AlvoDoLembrete,
  type Capture,
  type EstadoDoAparelho,
  type Lembrete,
  type CompromissoDaLista,
  type HorasDeProjeto,
  type ItemDaAgenda,
  type Panorama,
  type Task,
} from "./api";
import { ativar, situacao, type Situacao } from "./notificacoes";
import { Porta } from "./Porta";
import { Quando } from "./Quando";
import { porExtenso } from "./instantes";
import { Barra } from "./componentes/Barra";
import { Marca } from "./componentes/Marca";
import type { Pagina } from "./navegacao";
import { Home } from "./paginas/Home";
import { gravarArranjo, lerArranjo, type Arranjo } from "./paginas/arranjo";
import { Capturar } from "./paginas/Capturar";
import { Fazer } from "./paginas/Fazer";
import { Lembretes } from "./paginas/Lembretes";
import { Mais } from "./paginas/Mais";
import { Agenda } from "./paginas/Agenda";
import { Horas } from "./paginas/Horas";
import { Academico } from "./paginas/Academico";

/** O que a folha de *quando* está agendando, enquanto ela está aberta. */
type Agendamento = {
  titulo: string;
  descricao: string;
  alvo?: AlvoDoLembrete;
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
 * spinner: o app abre já podendo capturar, e a inbox é o que se olha depois.
 *
 * # Este arquivo é a casca
 *
 * Ele guarda o estado, fala com o servidor e escolhe a página. O desenho de cada
 * tela mora em `paginas/`, e o que elas têm em comum em `componentes/` — antes
 * disso tudo vivia aqui, em 666 linhas, e a decisão de cada tela era JSX que
 * nenhum teste alcançava.
 */
export function App() {
  const [pagina, setPagina] = useState<Pagina>("home");
  const [texto, setTexto] = useState("");
  const [capturas, setCapturas] = useState<Capture[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [lembretes, setLembretes] = useState<Lembrete[]>([]);
  const [estado, setEstado] = useState<EstadoDoAparelho | null>(null);
  const [panorama, setPanorama] = useState<Panorama | null>(null);
  const [agenda, setAgenda] = useState<ItemDaAgenda[]>([]);
  const [horas, setHoras] = useState<HorasDeProjeto[]>([]);
  const [janelaDasHoras, setJanelaDasHoras] = useState<"semana" | "mes">("semana");
  const [academico, setAcademico] = useState<CompromissoDaLista[]>([]);
  const [recado, setRecado] = useState("");
  const [erro, setErro] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  const [avisos, setAvisos] = useState<Situacao | null>(null);
  const [agendando, setAgendando] = useState<Agendamento | null>(null);
  // Lido uma vez, na montagem: o arranjo vive no `localStorage` deste aparelho,
  // e reler a cada render custaria uma ida ao disco por causa de nada.
  const [arranjo, setArranjo] = useState<Arranjo>(lerArranjo);
  /** A Home está sendo arrumada. Mora aqui, e não na Home, porque o modo TROCA
   *  a barra do topo — e a barra do topo é desta casca. */
  const [arrumando, setArrumando] = useState(false);
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
    const [
      proximaInbox,
      proximasTasks,
      proximosLembretes,
      proximoPanorama,
      proximaAgenda,
      proximoAcademico,
    ] = await Promise.all([
      api.inbox().catch(() => [] as Capture[]),
      api.tasks().catch(() => [] as Task[]),
      api.lembretes().catch(() => [] as Lembrete[]),
      // Nulo em vez de erro: um servidor sem a rota ainda serve a Home inteira,
      // só sem os dois cartões novos.
      api.panorama().catch(() => null),
      // A janela: de ontem a uma semana. Para tras um dia porque "o que eu fiz
      // ontem" e pergunta de manha; para a frente sete porque prova marcada
      // para daqui a duas semanas nao muda o que se faz hoje.
      api
        .agenda(diasDaqui(-1), diasDaqui(7))
        .catch(() => [] as ItemDaAgenda[]),
      api.academico().catch(() => [] as CompromissoDaLista[]),
    ]);
    if (proximoEstado) setEstado(proximoEstado);
    setCapturas(proximaInbox);
    setTasks(proximasTasks);
    setLembretes(proximosLembretes);
    setPanorama(proximoPanorama);
    setAgenda(proximaAgenda);
    setAcademico(proximoAcademico);
    // A situação das notificações é recalculada junto: ela muda por fora do app
    // — instalar na tela de início, mexer em Ajustes —, e uma tela que só olha
    // uma vez ficaria dizendo "instale" depois de você já ter instalado.
    setAvisos(await situacao(proximoEstado?.chavePush ?? null));
  }, []);

  useEffect(() => {
    // As horas seguem a janela escolhida, e nao o laco geral: elas so importam
    // com a pagina aberta, e recarrega-las a cada trinta segundos seria uma ida
    // a rede por um numero que ninguem esta olhando.
    const inicio = janelaDasHoras === "semana" ? inicioDaSemana() : inicioDoMes();
    void api
      .horas(inicio, new Date())
      .then(setHoras)
      .catch(() => setHoras([]));
  }, [janelaDasHoras, pagina]);

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
   * O compositor da página de lembretes não cria nada sozinho: ele abre a folha
   * de *quando*.
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
      // jogaria fora o que a pessoa estava escrevendo em outra página.
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

  const dados = { capturas, tasks, lembretes };
  // O compositor não existe onde não há o que compor. Na Home ele roubaria o
  // lugar do panorama, e em Mais não há nada para escrever.
  const compoe = pagina === "capturar" || pagina === "fazer" || pagina === "lembretes";

  return (
    <div className="app">
      {arrumando ? (
        /* A barra inteira em sódio, e não um aviso flutuante: o modo muda o que
           cada toque faz na tela toda, e um estado assim tem que ser visível
           sem ser procurado. */
        <header className="topo" data-arrumando="">
          <span className="rotulo">ARRUMANDO</span>
          <span className="topo-dica">segure e arraste</span>
          <button type="button" className="topo-concluir" onClick={() => setArrumando(false)}>
            Concluir
          </button>
        </header>
      ) : (
        <header className="topo">
          <Marca tamanho={18} girando={ocupado} />
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
      )}

      {/* A `key` remonta o conteúdo a cada troca, que é o que faz a animação de
          entrada rodar. Sem ela o React reaproveita o nó e a página nova aparece
          sem transição nenhuma, como um corte. */}
      <main className="conteudo" key={pagina}>
        {pagina === "home" ? (
          <Home
            estado={estado}
            dados={dados}
            panorama={panorama}
            arranjo={arranjo}
            arrumando={arrumando}
            aoArrumando={setArrumando}
            aoArranjar={(proximo) => {
              setArranjo(proximo);
              gravarArranjo(proximo);
            }}
            aoIr={setPagina}
          />
        ) : null}
        {pagina === "capturar" ? <Capturar capturas={capturas} /> : null}
        {pagina === "fazer" ? (
          <Fazer
            capturas={capturas}
            tasks={tasks}
            tasksLembradas={tasksLembradas}
            aoCapturar={() => setPagina("capturar")}
            aoAlternar={(task) => void alternar(task)}
            aoLembrar={(task, jaTem) =>
              setAgendando({
                // O título do lembrete é o da Task: quem toca no sino quer ser
                // lembrado DELA, e pedir um texto novo aqui seria pedir para
                // escrever de novo o que já está na linha acima do dedo.
                titulo: task.title,
                descricao: jaTem
                  ? "JÁ HÁ UM LEMBRETE PARA ESTA TASK"
                  : "LEMBRAR DESTA TASK",
                alvo: { type: "task", id: task.id },
              })
            }
          />
        ) : null}
        {pagina === "lembretes" ? (
          <Lembretes
            lembretes={lembretes}
            ocupado={ocupado}
            aoResolver={(lembrete, como) => void resolverLembrete(lembrete, como)}
          />
        ) : null}
        {pagina === "agenda" ? <Agenda itens={agenda} agora={new Date()} /> : null}
        {pagina === "horas" ? (
          <Horas linhas={horas} janela={janelaDasHoras} aoTrocarJanela={setJanelaDasHoras} />
        ) : null}
        {pagina === "academico" ? <Academico compromissos={academico} /> : null}
        {pagina === "mais" ? (
          <Mais
            estado={estado}
            avisos={avisos}
            ocupado={ocupado}
            cobrando={cobrando}
            aoAtivar={() => void ativarAvisos()}
            aoTestar={() => void testarAvisos()}
            aoAbrirLembretes={() => setPagina("lembretes")}
            aoAbrirHoras={() => setPagina("horas")}
            aoAbrirAcademico={() => setPagina("academico")}
          />
        ) : null}
      </main>

      {compoe ? (
        <form
          className="compositor"
          onSubmit={
            pagina === "fazer" ? novaTask : pagina === "lembretes" ? agendarSolto : capturar
          }
        >
          <textarea
            value={texto}
            onChange={(evento) => setTexto(evento.currentTarget.value)}
            placeholder={
              pagina === "fazer"
                ? "O que precisa ser feito?"
                : pagina === "lembretes"
                  ? "Lembrar de…"
                  : "O que está na cabeça?"
            }
            aria-label={
              pagina === "fazer"
                ? "Nova task"
                : pagina === "lembretes"
                  ? "Novo lembrete"
                  : "Nova captura"
            }
          />
          <div className="linha-de-botoes">
            <button className="botao" type="submit" disabled={ocupado || !texto.trim()}>
              {pagina === "fazer"
                ? "Criar task"
                : pagina === "lembretes"
                  ? "Escolher quando"
                  : "Guardar"}
            </button>
          </div>
          <p className="recado" data-estado={erro ? "erro" : "ok"} aria-live="polite">
            {recado}
          </p>
        </form>
      ) : null}

      <Barra atual={pagina} dados={dados} aoIr={setPagina} />

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

/** Verde, âmbar ou apagado — o que a etiqueta da fila está dizendo. */
function sinalDaFila(estado: EstadoDoAparelho | null, pendentes: number): string {
  if (estado?.sincroniza === false) return "sem-hub";
  return pendentes > 0 ? "fila" : "em-dia";
}

/** O instante de N dias a partir de agora. Negativo volta no tempo. */
function diasDaqui(dias: number): Date {
  return new Date(Date.now() + dias * 86_400_000);
}

/** A segunda-feira desta semana, à meia-noite local. */
function inicioDaSemana(): Date {
  const inicio = new Date();
  const desdeSegunda = (inicio.getDay() + 6) % 7;
  inicio.setDate(inicio.getDate() - desdeSegunda);
  inicio.setHours(0, 0, 0, 0);
  return inicio;
}

/** O dia 1 deste mês, à meia-noite local. */
function inicioDoMes(): Date {
  const inicio = new Date();
  inicio.setDate(1);
  inicio.setHours(0, 0, 0, 0);
  return inicio;
}

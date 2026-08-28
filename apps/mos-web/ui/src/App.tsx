import { useCallback, useEffect, useState, type FormEvent } from "react";
import { api, type Capture, type EstadoDoAparelho, type Task } from "./api";
import { ativar, situacao, type Situacao } from "./notificacoes";

type Aba = "capturar" | "inbox" | "tasks" | "avisos";

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
 */
export function App() {
  const [aba, setAba] = useState<Aba>("capturar");
  const [texto, setTexto] = useState("");
  const [capturas, setCapturas] = useState<Capture[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [estado, setEstado] = useState<EstadoDoAparelho | null>(null);
  const [recado, setRecado] = useState("");
  const [erro, setErro] = useState(false);
  const [ocupado, setOcupado] = useState(false);
  const [avisos, setAvisos] = useState<Situacao | null>(null);

  const atualizar = useCallback(async () => {
    const [proximoEstado, proximaInbox, proximasTasks] = await Promise.all([
      api.estado().catch(() => null),
      api.inbox().catch(() => [] as Capture[]),
      api.tasks().catch(() => [] as Task[]),
    ]);
    if (proximoEstado) setEstado(proximoEstado);
    setCapturas(proximaInbox);
    setTasks(proximasTasks);
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
      contar(causa instanceof Error ? causa.message : String(causa), true);
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
      contar(causa instanceof Error ? causa.message : String(causa), true);
    }
    setOcupado(false);
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
      contar(causa instanceof Error ? causa.message : String(causa), true);
      await atualizar();
    }
  }

  async function ativarAvisos() {
    if (!estado?.chavePush) return;
    setOcupado(true);
    try {
      await ativar(estado.chavePush);
      contar("Notificações ativadas.");
      await atualizar();
    } catch (causa) {
      contar(causa instanceof Error ? causa.message : String(causa), true);
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
      contar(causa instanceof Error ? causa.message : String(causa), true);
    }
    setOcupado(false);
  }

  const pendentes = estado?.pendentes ?? 0;

  return (
    <div className="app">
      <header className="topo">
        <span className="marca">M/OS</span>
        <span className="fila" data-pendente={pendentes > 0 ? "sim" : "nao"}>
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
            ["capturar", "Capturar"],
            ["inbox", `Inbox${capturas.length ? ` ${capturas.length}` : ""}`],
            ["tasks", `Tasks${tasks.length ? ` ${tasks.length}` : ""}`],
            ["avisos", "Avisos"],
          ] as const
        ).map(([valor, rotulo]) => (
          <button
            key={valor}
            type="button"
            aria-current={aba === valor ? "page" : undefined}
            onClick={() => setAba(valor)}
          >
            {rotulo}
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
                    <div>
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
                  <div>
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
              {tasks.map((task) => (
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
                  <div>
                    <p>{task.title}</p>
                    <small>{task.state.toUpperCase()}</small>
                  </div>
                </li>
              ))}
            </ul>
          )
        ) : null}
        {aba === "avisos" ? (
          <div className="avisos">
            {/* A frase vem antes do botão de propósito: no iPhone o botão só
                funciona depois de instalar na tela de início, e um botão que
                falha calado é pior que um botão ausente. */}
            <p className="explicacao">
              {avisos?.estado === "ativo"
                ? "Este aparelho recebe os lembretes que vencem e avisa quando o computador manda coisa nova."
                : avisos?.estado === "pronto"
                  ? "Ative para receber aqui os lembretes que vencerem, mesmo com o app fechado."
                  : (avisos?.motivo ?? "Conferindo…")}
            </p>

            {/* O que ele vai avisar, dito antes de você decidir ativar.
                Permissão de notificação é um sim ou não sem volta fácil no
                iPhone, e concedê-la sem saber o que vai chegar é o começo do
                app que a pessoa silencia na semana seguinte. */}
            {avisos?.estado === "impossivel" ? null : (
              <ul className="promessas">
                <li>Lembretes, na hora em que vencerem.</li>
                <li>Quando o computador mandar coisa nova.</li>
              </ul>
            )}

            {avisos?.estado === "pronto" ? (
              <button
                className="botao"
                type="button"
                disabled={ocupado}
                onClick={() => void ativarAvisos()}
              >
                Ativar notificações
              </button>
            ) : null}

            {avisos?.estado === "ativo" ? (
              <>
                <button
                  className="botao"
                  type="button"
                  disabled={ocupado}
                  onClick={() => void testarAvisos()}
                >
                  Enviar um teste agora
                </button>
                <p className="rotulo">
                  {estado?.aparelhosAvisados === 1
                    ? "1 APARELHO AVISADO"
                    : `${estado?.aparelhosAvisados ?? 0} APARELHOS AVISADOS`}
                </p>
              </>
            ) : null}
          </div>
        ) : null}
      </main>

      {/* Sem compositor na aba de avisos: não há nada para escrever ali, e um
          campo de texto sob um botão de configuração convida ao engano. */}
      {aba === "avisos" ? (
        <p className="recado" data-estado={erro ? "erro" : "ok"} aria-live="polite">
          {recado}
        </p>
      ) : (
      <form
        className="compositor"
        onSubmit={aba === "tasks" ? novaTask : capturar}
      >
        <textarea
          value={texto}
          onChange={(evento) => setTexto(evento.currentTarget.value)}
          placeholder={
            aba === "tasks" ? "O que precisa ser feito?" : "O que está na cabeça?"
          }
          aria-label={aba === "tasks" ? "Nova task" : "Nova captura"}
        />
        <div className="linha-de-botoes">
          <button className="botao" type="submit" disabled={ocupado || !texto.trim()}>
            {aba === "tasks" ? "Criar task" : "Guardar"}
          </button>
        </div>
        <p className="recado" data-estado={erro ? "erro" : "ok"} aria-live="polite">
          {recado}
        </p>
      </form>
      )}
    </div>
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

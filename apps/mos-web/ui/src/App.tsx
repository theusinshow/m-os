import { useCallback, useEffect, useState, type FormEvent } from "react";
import { api, type Capture, type EstadoDoAparelho, type Task } from "./api";

type Aba = "capturar" | "inbox" | "tasks";

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

  const atualizar = useCallback(async () => {
    const [proximoEstado, proximaInbox, proximasTasks] = await Promise.all([
      api.estado().catch(() => null),
      api.inbox().catch(() => [] as Capture[]),
      api.tasks().catch(() => [] as Task[]),
    ]);
    if (proximoEstado) setEstado(proximoEstado);
    setCapturas(proximaInbox);
    setTasks(proximasTasks);
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
      </main>

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

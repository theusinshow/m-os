import type { CSSProperties } from "react";
import type { Capture, Task } from "../api";
import { Vazio } from "../componentes/Vazio";
import { idade } from "./idade";

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
 * O que está aberto: o que falta triar, e o que falta fazer.
 *
 * # Por que as duas listas numa tela só
 *
 * Inbox e Tasks eram dois dos cinco lugares da barra, e respondiam a mesma
 * pergunta duas vezes. A diferença entre elas não é de assunto — é de estágio:
 * a captura é o pensamento cru, a task é o pensamento que já virou compromisso.
 * Ver as duas juntas é o que permite mover uma para a outra sem trocar de tela.
 *
 * A ordem é essa e não a inversa: **o que chegou vem primeiro**. Uma captura
 * parada é uma decisão que ainda não foi tomada, e enterrá-la embaixo da lista
 * de tasks é o jeito mais fácil de nunca mais tomá-la.
 */
export function Fazer({
  capturas,
  tasks,
  tasksLembradas,
  aoCapturar,
  aoAlternar,
  aoAbrir,
  aoLembrar,
}: {
  capturas: Capture[];
  tasks: Task[];
  tasksLembradas: Set<string>;
  aoCapturar: () => void;
  aoAlternar: (task: Task) => void;
  aoAbrir: (task: Task) => void;
  aoLembrar: (task: Task, jaTem: boolean) => void;
}) {
  const abertas = tasks.filter((task) => task.state !== "done");

  if (capturas.length === 0 && tasks.length === 0) {
    return (
      <Vazio
        frase="Nada aberto. O que você capturar e as tasks que criar aparecem aqui."
        acao={{ rotulo: "Capturar agora", aoTocar: aoCapturar }}
      />
    );
  }

  return (
    <div className="fazer">
      {capturas.length > 0 ? (
        <section>
          <h2 className="secao">
            <span>POR TRIAR</span>
            <b>{capturas.length}</b>
          </h2>
          <ul className="lista">
            {capturas.map((captura, indice) => (
              <li
                className="item"
                key={captura.id}
                // A escada de entrada é por posição, e para no oitavo: passado
                // isso a soma dos atrasos vira espera, e uma lista que demora a
                // aparecer não parece animada — parece lenta.
                style={escada(indice)}
              >
                <div className="item-corpo">
                  <p>{captura.content}</p>
                  <small>{idade(captura.capturedAt)}</small>
                </div>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {tasks.length > 0 ? (
        <section>
          <h2 className="secao">
            <span>TASKS</span>
            <b>{abertas.length}</b>
          </h2>
          <ul className="lista">
            {tasks.map((task, indice) => {
              const lembrada = tasksLembradas.has(task.id);
              return (
                <li
                  className="item"
                  key={task.id}
                  data-feita={task.state === "done" || undefined}
                  style={escada(indice)}
                >
                  <button
                    className="marcar"
                    type="button"
                    aria-pressed={task.state === "done"}
                    aria-label={
                      task.state === "done"
                        ? `Reabrir ${task.title}`
                        : `Concluir ${task.title}`
                    }
                    onClick={() => aoAlternar(task)}
                  >
                    <span aria-hidden="true" />
                  </button>
                  {/* A linha abre a task. O que fica fora dela sao os dois
                      alvos que agem sem abrir: marcar e lembrar. */}
                  <button
                    className="linha-destino"
                    type="button"
                    onClick={() => aoAbrir(task)}
                  >
                    <div className="item-corpo">
                      <p>{task.title}</p>
                      <small>
                        {ESTADO_DA_TASK[task.state]}
                        {lembrada ? " · com lembrete" : ""}
                      </small>
                    </div>
                  </button>
                  {/* O sino, e nao um menu de tres pontos: e a unica acao que
                      esta linha oferece alem de marcar, e esconde-la atras de um
                      menu custaria dois toques para ganhar nada. */}
                  <button
                    className="sino"
                    type="button"
                    data-ligado={lembrada || undefined}
                    aria-label={
                      lembrada
                        ? `Outro lembrete para ${task.title}`
                        : `Lembrar de ${task.title}`
                    }
                    onClick={() => aoLembrar(task, lembrada)}
                  >
                    <SinoIcone />
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      ) : null}
    </div>
  );
}

/** O atraso de entrada da enésima linha, com teto. */
function escada(indice: number) {
  return { "--degrau": `${Math.min(indice, 8) * 30}ms` } as CSSProperties;
}

/**
 * O sino, desenhado e nao importado.
 *
 * Uma biblioteca de icones para um glifo so custaria mais bytes no 4G do que a
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

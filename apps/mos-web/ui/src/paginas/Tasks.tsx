import type { Task } from "../api";

/** Os estados de Task, na palavra que a tela usa. */
const ESTADO_DA_TASK: Record<Task["state"], string> = {
  inbox: "Inbox",
  backlog: "Backlog",
  planned: "Planejada",
  doing: "Em andamento",
  review: "Revisão",
  done: "Feita",
};

export function Tasks({
  tasks,
  tasksLembradas,
  aoAlternar,
  aoLembrar,
}: {
  tasks: Task[];
  tasksLembradas: Set<string>;
  aoAlternar: (task: Task) => void;
  aoLembrar: (task: Task, jaTem: boolean) => void;
}) {
  if (tasks.length === 0) {
    return (
      <div className="vazio">
        <p>Nenhuma task aberta. Escreva embaixo para criar a primeira.</p>
      </div>
    );
  }
  return (
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
                task.state === "done" ? `Reabrir ${task.title}` : `Concluir ${task.title}`
              }
              onClick={() => aoAlternar(task)}
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
            {/* O sino, e nao um menu de tres pontos: e a unica acao que esta
                linha oferece alem de marcar, e esconde-la atras de um menu
                custaria dois toques para ganhar nada. */}
            <button
              className="sino"
              type="button"
              data-ligado={lembrada || undefined}
              aria-label={
                lembrada ? `Outro lembrete para ${task.title}` : `Lembrar de ${task.title}`
              }
              onClick={() => aoLembrar(task, lembrada)}
            >
              <SinoIcone />
            </button>
          </li>
        );
      })}
    </ul>
  );
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

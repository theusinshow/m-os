import { useState, type CSSProperties } from "react";
import type { Capture, Task } from "../api";
import { Esqueleto } from "../componentes/Esqueleto";
import { Vazio } from "../componentes/Vazio";
import { Progresso } from "./Checklist";
import { idade } from "./idade";
import { dominioDe, enderecoEm } from "./links";

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
  aoTriar,
  carregando,
}: {
  capturas: Capture[];
  tasks: Task[];
  tasksLembradas: Set<string>;
  aoCapturar: () => void;
  aoAlternar: (task: Task) => void;
  aoAbrir: (task: Task) => void;
  aoLembrar: (task: Task, jaTem: boolean) => void;
  aoTriar: (captura: Capture, como: "task" | "referencia" | "arquivar") => void;
  /** O primeiro carregamento ainda não voltou. */
  carregando?: boolean;
}) {
  const abertas = tasks.filter((task) => task.state !== "done");

  // Vazio só depois de uma resposta: antes disso, a tela não sabe se está
  // vazio — ela sabe que ainda não perguntou.
  if (carregando && capturas.length === 0 && tasks.length === 0) {
    return <Esqueleto linhas={4} />;
  }

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
              <Captura
                key={captura.id}
                captura={captura}
                // A escada de entrada é por posição, e para no oitavo: passado
                // isso a soma dos atrasos vira espera, e uma lista que demora a
                // aparecer não parece animada — parece lenta.
                estilo={escada(indice)}
                aoTriar={(como) => aoTriar(captura, como)}
              />
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
                  data-atrasada={atrasada(task) || undefined}
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
                      {/* O progresso vem ANTES da linha de estado: `3/6` é a
                          resposta a "o que falta nessa task", e o estado é
                          contexto. Só aparece quando há checklist — uma barra
                          vazia diria "começou e não andou" numa task que não
                          tem passos. */}
                      {task.checklistTotal ? (
                        <Progresso feitos={task.checklistDone} total={task.checklistTotal} />
                      ) : null}
                      <small>
                        {ESTADO_DA_TASK[task.state]}
                        {task.dueAt ? ` · ${prazo(task.dueAt)}` : ""}
                        {task.waitingFor ? ` · aguardando ${task.waitingFor}` : ""}
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

/**
 * Uma captura por triar.
 *
 * # Por que ela se anuncia como link
 *
 * A queixa era exata: *"clico em algo dentro de Fazer e aparece um link que eu
 * havia salvo apenas como referência"*. A Capture não tem tipo — ela é o
 * registro cru, e isso é decisão do domínio, não esquecimento —, mas o texto
 * dela DIZ o que ela provavelmente é. Um endereço no meio dela é um sinal forte
 * de que aquilo se consulta, e não se faz.
 *
 * Então a linha mostra o domínio, e a ação de virar referência vem PRIMEIRO
 * quando há link. O sistema continua não adivinhando: ele oferece.
 */
function Captura({
  captura,
  estilo,
  aoTriar,
}: {
  captura: Capture;
  estilo: CSSProperties;
  aoTriar: (como: "task" | "referencia" | "arquivar") => void;
}) {
  const [aberta, setAberta] = useState(false);
  const link = enderecoEm(captura.content);
  const dominio = link ? dominioDe(link) : null;

  return (
    <li className="item" data-link={link ? "" : undefined} style={estilo}>
      <button className="linha-destino" type="button" onClick={() => setAberta(!aberta)}>
        <div className="item-corpo">
          <p>{captura.content}</p>
          <small>
            {idade(captura.capturedAt)}
            {dominio ? ` · ${dominio}` : ""}
          </small>
        </div>
      </button>

      {aberta ? (
        // As três saídas de uma captura, e não mais: virar tarefa, virar
        // referência, ou não ser nada. Uma quarta opção aqui seria uma decisão
        // a mais no momento em que a pessoa só quer esvaziar a inbox.
        <div className="triagem">
          {link ? (
            <>
              <button type="button" onClick={() => aoTriar("referencia")}>
                Guardar referência
              </button>
              <button type="button" onClick={() => aoTriar("task")}>
                Virar task
              </button>
            </>
          ) : (
            <>
              <button type="button" onClick={() => aoTriar("task")}>
                Virar task
              </button>
              <button type="button" onClick={() => aoTriar("referencia")}>
                Guardar nota
              </button>
            </>
          )}
          <button type="button" onClick={() => aoTriar("arquivar")}>
            Arquivar
          </button>
        </div>
      ) : null}
    </li>
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

/**
 * O prazo escrito curto, para caber na linha de apoio.
 *
 * "Hoje 17:00" e não "quinta-feira, 8 de setembro": a linha já carrega estado,
 * lembrete e quem está segurando a task, e um prazo por extenso empurraria
 * todos eles para fora numa tela de 393px.
 */
function prazo(iso: string): string {
  const data = new Date(iso);
  if (Number.isNaN(data.getTime())) return "";
  const dia = (valor: Date) =>
    new Date(valor.getFullYear(), valor.getMonth(), valor.getDate()).getTime();
  const distancia = Math.round((dia(data) - dia(new Date())) / 86_400_000);
  const p = (valor: number) => String(valor).padStart(2, "0");
  const hora = data.getHours() || data.getMinutes() ? ` ${p(data.getHours())}:${p(data.getMinutes())}` : "";
  if (distancia === 0) return `hoje${hora}`;
  if (distancia === 1) return `amanhã${hora}`;
  if (distancia === -1) return `ontem${hora}`;
  const mes = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"][data.getMonth()];
  return `${data.getDate()} ${mes}${hora}`;
}

/**
 * Passou do prazo e ainda não foi feita.
 *
 * A segunda metade é a que importa: cobrar prazo de trabalho já entregue é o
 * comportamento que faz as pessoas pararem de olhar para os avisos do sistema.
 */
function atrasada(task: Task): boolean {
  if (!task.dueAt || task.state === "done" || task.completedAt) return false;
  const prazoEm = new Date(task.dueAt);
  return !Number.isNaN(prazoEm.getTime()) && prazoEm.getTime() < Date.now();
}

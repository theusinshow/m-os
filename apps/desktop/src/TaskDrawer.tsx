import { LazyMotion, m } from "framer-motion";
import { useEffect, useRef, useState, type FormEvent } from "react";

import { api } from "./api";
import { Button } from "./Button";
import { Checklist } from "./Checklist";
import { Icon } from "./Icon";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import {
  doCampoLocal,
  edicaoDe,
  ESTIMATIVAS_RAPIDAS,
  estimativaCurta,
  ORDEM_DOS_ESTADOS,
  paraCampoLocal,
  prazoCurto,
  ROTULO_DE_PRIORIDADE,
  situacaoDoPrazo,
} from "./tasks";
import type { Capture, Project, Task, TaskDetail, TaskPriority, TaskState } from "./types";

/* Dinâmico, como nas outras telas: um `import` estático puxa o `framer-motion`
   inteiro para o bundle principal e desfaz o code-splitting que o resto do app
   já tem. */
const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

const ROTULO_DO_ESTADO: Record<TaskState, string> = {
  inbox: "Inbox",
  backlog: "Backlog",
  planned: "Planned",
  doing: "Doing",
  review: "Review",
  done: "Done",
};

/**
 * A folha de uma Task.
 *
 * # A hierarquia, e por que ela é essa
 *
 * De cima para baixo: **título, estado, checklist, notas, metadados, mais
 * opções.** A ordem é a do princípio *contexto antes de metadado, execução
 * antes de administração* — quem abre uma Task quase sempre abre para EXECUTAR
 * um passo, não para trocar a prioridade. O que se usa toda vez fica onde o
 * olho cai; o que se usa uma vez por Task desce.
 *
 * # Por que ela salva sozinha, campo por campo
 *
 * A versão anterior era um formulário com um botão Salvar: mexer em qualquer
 * coisa exigia confirmar, e fechar sem confirmar perdia tudo. Aqui cada campo
 * grava quando muda — marcar um passo, escolher a prioridade, escrever a nota
 * ao sair do campo. É o mesmo comportamento do checklist, e ter DOIS modelos de
 * gravação na mesma folha (um salva na hora, o outro espera um botão) é a
 * receita para perder o que foi digitado.
 *
 * O título e a nota gravam no `blur` e no Enter, e não a cada tecla: uma
 * operação de sync por caractere encheria a fila com trinta versões da mesma
 * frase.
 */
export function TaskDrawer({
  task,
  projects,
  tasks,
  close,
  refresh,
  receipt,
  openCapture,
  openTask,
  remind,
}: {
  task: Task;
  projects: Project[];
  /** Para escolher a Task que bloqueia esta. */
  tasks: Task[];
  close: () => void;
  refresh: () => Promise<void>;
  receipt: (action: { message: string; run: () => Promise<unknown> }) => void;
  openCapture: (capture: Capture) => void;
  openTask: (task: Task) => void;
  remind: () => void;
}) {
  const [detalhe, setDetalhe] = useState<TaskDetail | null>(null);
  const [titulo, setTitulo] = useState(task.title);
  const [nota, setNota] = useState(task.description);
  const [origem, setOrigem] = useState<Capture | null>(null);
  const [mais, setMais] = useState(false);
  const [erro, setErro] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const tituloInput = useRef<HTMLInputElement>(null);
  const voltarFoco = useRef<HTMLElement | null>(
    document.activeElement instanceof HTMLElement ? document.activeElement : null,
  );

  const atual = detalhe?.task ?? task;

  useEffect(() => {
    setDetalhe(null);
    setTitulo(task.title);
    setNota(task.description);
    void api.taskDetail(task.id).then(setDetalhe).catch(() => setDetalhe(null));
    if (task.sourceCaptureId) void api.getCapture(task.sourceCaptureId).then(setOrigem);
    /* "Mais opções" abre já expandido quando há algo lá dentro. Progressive
       disclosure esconde o que está VAZIO; esconder o que está preenchido é
       esconder informação, e aí a pessoa não sabe que a Task tem um bloqueio. */
    setMais(
      Boolean(task.estimateMinutes) ||
        Boolean(task.waitingFor) ||
        Boolean(task.blockedByTaskId) ||
        Boolean(task.parentTaskId),
    );
  }, [task.id]);

  useEffect(() => {
    tituloInput.current?.focus();
    return () => {
      const alvo = voltarFoco.current;
      if (alvo?.isConnected) requestAnimationFrame(() => alvo.focus());
    };
  }, []);

  /** Grava uma mudança de campo e recarrega a folha. */
  async function gravar(mudanca: Partial<ReturnType<typeof edicaoDe>>) {
    setErro("");
    setOcupado(true);
    try {
      await api.updateTask({ ...edicaoDe(atual), ...mudanca });
      setDetalhe(await api.taskDetail(task.id));
      await refresh();
    } catch (falha) {
      setErro(falha instanceof Error ? falha.message : String(falha));
    } finally {
      setOcupado(false);
    }
  }

  async function mudarEstado(estado: TaskState) {
    setErro("");
    try {
      await api.setTaskState(task.id, estado);
      setDetalhe(await api.taskDetail(task.id));
      await refresh();
    } catch (falha) {
      setErro(falha instanceof Error ? falha.message : String(falha));
    }
  }

  async function arquivar() {
    setErro("");
    try {
      await api.setTaskArchived(task.id, true);
      receipt({ message: "Task arquivada.", run: () => api.setTaskArchived(task.id, false) });
      await refresh();
      close();
    } catch (falha) {
      setErro(falha instanceof Error ? falha.message : String(falha));
    }
  }

  const situacao = situacaoDoPrazo(atual);
  const feito = atual.state === "done";
  const tudoMarcado = atual.checklistTotal > 0 && atual.checklistDone === atual.checklistTotal && !feito;

  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <m.aside
        className="task-drawer"
        aria-label="Detalhe da Task"
        aria-busy={ocupado}
        tabIndex={-1}
        initial={{ opacity: 0, x: 24 }}
        animate={{ opacity: 1, x: 0 }}
        exit={{ opacity: 0, x: 24 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
        onKeyDown={(evento) => {
          if (evento.key === "Escape" && !ocupado) close();
        }}
      >
        <header>
          <span className="micro-label">TASK</span>
          <IconeBotao label="Fechar" onClick={close} />
        </header>

        {/* ---------------------------------------------------- identidade */}
        <form
          className="task-titulo"
          onSubmit={(evento: FormEvent) => {
            evento.preventDefault();
            tituloInput.current?.blur();
          }}
        >
          <input
            ref={tituloInput}
            value={titulo}
            aria-label="Título da Task"
            onChange={(evento) => setTitulo(evento.currentTarget.value)}
            onBlur={() => {
              const limpo = titulo.trim();
              if (!limpo) {
                setTitulo(atual.title);
                return;
              }
              if (limpo !== atual.title) void gravar({ title: limpo });
            }}
          />
        </form>

        {/* O estado é a coisa que mais muda nesta folha; um `<select>` o
            esconderia atrás de um clique para abrir a lista. */}
        <div className="task-estados" role="group" aria-label="Estado">
          {ORDEM_DOS_ESTADOS.map((estado) => (
            <button
              key={estado}
              type="button"
              aria-pressed={atual.state === estado}
              onClick={() => void mudarEstado(estado)}
            >
              {ROTULO_DO_ESTADO[estado]}
            </button>
          ))}
        </div>

        {atual.blockedByTaskId && detalhe?.blockedBy ? (
          <p className="task-bloqueio">
            Bloqueada por{" "}
            <button type="button" onClick={() => openTask(detalhe.blockedBy as Task)}>
              {detalhe.blockedBy.title}
            </button>
          </p>
        ) : null}

        {/* ----------------------------------------------------- checklist */}
        {detalhe ? (
          <Checklist
            taskId={task.id}
            itens={detalhe.checklist}
            aoMudar={(atualizada) => {
              void api.taskDetail(task.id).then(setDetalhe);
              void refresh();
              void atualizada;
            }}
          />
        ) : (
          <p className="checklist-carregando">Carregando…</p>
        )}

        {/* A oferta, e não a ação. Uma Task que se fecha sozinha é o sistema
            afirmando algo que ninguém disse. */}
        {tudoMarcado ? (
          <div className="task-tudo-feito">
            <span>Todos os itens concluídos.</span>
            <Button variant="primary" size="sm" onClick={() => void mudarEstado("done")}>
              Concluir Task
            </Button>
          </div>
        ) : null}

        {/* --------------------------------------------------------- notas */}
        <section className="task-bloco">
          <span className="micro-label">NOTAS</span>
          <textarea
            className="task-nota"
            value={nota}
            rows={3}
            placeholder="O contexto que você vai esquecer."
            onChange={(evento) => setNota(evento.currentTarget.value)}
            onBlur={() => {
              if (nota !== atual.description) void gravar({ description: nota });
            }}
          />
        </section>

        {/* ---------------------------------------------------- metadados */}
        <dl className="task-campos">
          <div>
            <dt>Project</dt>
            <dd>
              <select
                value={atual.projectId ?? ""}
                aria-label="Project"
                onChange={(evento) => void gravar({ projectId: evento.currentTarget.value || null })}
              >
                <option value="">Sem Project</option>
                {projects
                  .filter((project) => project.lifecycleState === "active")
                  .map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name}
                    </option>
                  ))}
              </select>
            </dd>
          </div>

          <div>
            <dt>Prazo</dt>
            <dd className="task-prazo" data-situacao={situacao}>
              <input
                type="datetime-local"
                value={paraCampoLocal(atual.dueAt)}
                aria-label="Prazo"
                onChange={(evento) => void gravar({ dueAt: doCampoLocal(evento.currentTarget.value) })}
              />
              {atual.dueAt ? <span className="task-prazo-lido">{prazoCurto(atual.dueAt)}</span> : null}
            </dd>
          </div>

          <div>
            <dt>Prioridade</dt>
            <dd>
              <div className="task-prioridade" role="group" aria-label="Prioridade">
                {(Object.keys(ROTULO_DE_PRIORIDADE) as TaskPriority[]).map((nivel) => (
                  <button
                    key={nivel}
                    type="button"
                    data-nivel={nivel}
                    aria-pressed={atual.priority === nivel}
                    onClick={() => void gravar({ priority: nivel })}
                  >
                    {ROTULO_DE_PRIORIDADE[nivel]}
                  </button>
                ))}
              </div>
            </dd>
          </div>

          <div>
            <dt>Lembrete</dt>
            <dd className="task-lembretes">
              {/* O lembrete NÃO é o prazo, e os dois aparecem juntos aqui de
                  propósito: o prazo diz quando vence, o lembrete diz quando o
                  M/OS interrompe. Ver a ADR-066. */}
              {detalhe?.reminders.length ? (
                detalhe.reminders.map((lembrete) => (
                  <span key={lembrete.id}>
                    {lembrete.nextDueAt ? prazoCurto(lembrete.nextDueAt) : "sem hora"}
                    {lembrete.snoozeCount > 0 ? ` · adiado ${lembrete.snoozeCount}×` : ""}
                  </span>
                ))
              ) : (
                <span className="task-vazio">nenhum</span>
              )}
              <Button variant="ghost" size="sm" onClick={remind}>
                {detalhe?.reminders.length ? "Outro" : "Lembrar"}
              </Button>
            </dd>
          </div>
        </dl>

        {/* ------------------------------------------------- referências */}
        {detalhe?.references.length ? (
          <section className="task-bloco">
            <span className="micro-label">REFERÊNCIAS</span>
            <ul className="task-referencias">
              {detalhe.references.map((resource) => (
                <li key={resource.id}>
                  <span>{resource.title}</span>
                  <button
                    type="button"
                    aria-label={`Desvincular ${resource.title}`}
                    onClick={() =>
                      void api.setTaskReference(task.id, resource.id, false).then(setDetalhe)
                    }
                  >
                    <Icon name="close" />
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {/* --------------------------------------------------- subtasks */}
        {detalhe?.subtasks.length ? (
          <section className="task-bloco">
            <span className="micro-label">SUBTASKS</span>
            <ul className="task-subtasks">
              {detalhe.subtasks.map((filha) => (
                <li key={filha.id}>
                  <button type="button" onClick={() => openTask(filha)} data-feita={filha.state === "done" || undefined}>
                    <span>{filha.title}</span>
                    <small>
                      {ROTULO_DO_ESTADO[filha.state]}
                      {filha.checklistTotal ? ` · ${filha.checklistDone}/${filha.checklistTotal}` : ""}
                    </small>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {origem ? (
          <div className="provenance">
            <span className="micro-label">ORIGEM</span>
            <button type="button" onClick={() => openCapture(origem)}>
              {origem.content}
            </button>
          </div>
        ) : null}

        {/* ------------------------------------------------ mais opções */}
        <button
          type="button"
          className="task-mais"
          aria-expanded={mais}
          onClick={() => setMais((aberto) => !aberto)}
        >
          {mais ? "Menos opções" : "Mais opções"}
        </button>

        {mais ? (
          <dl className="task-campos">
            <div>
              <dt>Estimativa</dt>
              <dd>
                <div className="task-estimativa" role="group" aria-label="Estimativa">
                  {ESTIMATIVAS_RAPIDAS.map((minutos) => (
                    <button
                      key={minutos}
                      type="button"
                      aria-pressed={atual.estimateMinutes === minutos}
                      onClick={() =>
                        void gravar({ estimateMinutes: atual.estimateMinutes === minutos ? null : minutos })
                      }
                    >
                      {estimativaCurta(minutos)}
                    </button>
                  ))}
                  <input
                    type="number"
                    min={1}
                    step={5}
                    value={atual.estimateMinutes ?? ""}
                    aria-label="Estimativa em minutos"
                    placeholder="min"
                    onChange={(evento) => {
                      const valor = Number(evento.currentTarget.value);
                      void gravar({ estimateMinutes: Number.isFinite(valor) && valor > 0 ? valor : null });
                    }}
                  />
                </div>
              </dd>
            </div>

            <div>
              <dt>Aguardando</dt>
              <dd>
                {/* Texto, e não uma entidade Pessoa: não existe cadastro de
                    pessoas no M/OS, e criar um para escrever um nome seria
                    construir um CRM por engano. */}
                <input
                  defaultValue={atual.waitingFor}
                  placeholder="quem está segurando"
                  aria-label="Aguardando"
                  onBlur={(evento) => {
                    const valor = evento.currentTarget.value.trim();
                    if (valor !== atual.waitingFor) void gravar({ waitingFor: valor });
                  }}
                />
              </dd>
            </div>

            {atual.waitingFor ? (
              <div>
                <dt>Cobrar em</dt>
                <dd>
                  <input
                    type="datetime-local"
                    value={paraCampoLocal(atual.followUpAt)}
                    aria-label="Cobrar em"
                    onChange={(evento) => void gravar({ followUpAt: doCampoLocal(evento.currentTarget.value) })}
                  />
                </dd>
              </div>
            ) : null}

            <div>
              <dt>Bloqueada por</dt>
              <dd>
                <select
                  value={atual.blockedByTaskId ?? ""}
                  aria-label="Bloqueada por"
                  onChange={(evento) => void gravar({ blockedByTaskId: evento.currentTarget.value || null })}
                >
                  <option value="">Nada</option>
                  {tasks
                    .filter((outra) => outra.id !== atual.id && outra.lifecycleState === "active")
                    .slice(0, 60)
                    .map((outra) => (
                      <option key={outra.id} value={outra.id}>
                        {outra.title}
                      </option>
                    ))}
                </select>
              </dd>
            </div>

            <div>
              <dt>Subtask de</dt>
              <dd>
                <select
                  value={atual.parentTaskId ?? ""}
                  aria-label="Subtask de"
                  onChange={(evento) => void gravar({ parentTaskId: evento.currentTarget.value || null })}
                >
                  <option value="">Nenhuma</option>
                  {tasks
                    .filter(
                      (outra) =>
                        outra.id !== atual.id &&
                        outra.lifecycleState === "active" &&
                        /* Um nível só na interface: a Task que já é filha não
                           aparece como mãe possível. O esquema aceita mais
                           fundo, e o dia em que a tela souber desenhar isso a
                           regra muda aqui, num lugar só. */
                        !outra.parentTaskId,
                    )
                    .slice(0, 60)
                    .map((outra) => (
                      <option key={outra.id} value={outra.id}>
                        {outra.title}
                      </option>
                    ))}
                </select>
              </dd>
            </div>
          </dl>
        ) : null}

        {erro ? <p className="task-erro" role="alert">{erro}</p> : null}

        <div className="form-actions spread">
          <Button variant="danger" onClick={() => void arquivar()}>
            Arquivar
          </Button>
        </div>
      </m.aside>
    </LazyMotion>
  );
}

function IconeBotao({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button className="icon-button" type="button" aria-label={label} title={label} onClick={onClick}>
      <Icon name="close" />
    </button>
  );
}

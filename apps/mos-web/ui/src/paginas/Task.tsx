import { useEffect, useState } from "react";
import type {
  DetalheDaTask,
  EdicaoDeTask,
  EstadoDaTask,
  Lembrete,
  PrioridadeDaTask,
  Projeto,
  Task as Item,
} from "../api";
import { daquiA } from "../instantes";
import { Checklist } from "./Checklist";

/**
 * Os estados, na ordem em que o trabalho anda.
 *
 * São os seis do Kanban do desktop, e não um subconjunto: uma Task que o PC
 * mandou para `review` precisa poder ser lida — e devolvida — do celular. Um
 * bolso que só conhecesse "aberta" e "feita" mostraria a Task em revisão como
 * se fosse mais uma aberta, e o estado que o desktop escolheu sumiria no
 * primeiro toque daqui.
 */
const PRIORIDADES: { valor: PrioridadeDaTask; rotulo: string }[] = [
  { valor: "low", rotulo: "Baixa" },
  { valor: "normal", rotulo: "Normal" },
  { valor: "high", rotulo: "Alta" },
  { valor: "urgent", rotulo: "Urgente" },
];

const ESTADOS: { valor: EstadoDaTask; rotulo: string }[] = [
  { valor: "inbox", rotulo: "Inbox" },
  { valor: "backlog", rotulo: "Backlog" },
  { valor: "planned", rotulo: "Planejada" },
  { valor: "doing", rotulo: "Fazendo" },
  { valor: "review", rotulo: "Revisão" },
  { valor: "done", rotulo: "Feita" },
];

/**
 * Uma Task, inteira.
 *
 * # A queixa que originou esta tela
 *
 * *"Abro a Home no celular e aparece algo que já fiz, mas não consigo resolver
 * isso direto dali."* A lista marcava e desmarcava, e mais nada: o estado
 * intermediário — planejada, fazendo, em revisão — era do PC, e o celular
 * achatava tudo em feito/não feito.
 *
 * # Por que os seis estados, e não um interruptor
 *
 * Marcar como feita é um caso do movimento, não o movimento inteiro. O
 * interruptor resolve o dia em que a Task acabou; ele não resolve o dia em que
 * ela saiu do backlog e virou o que se está fazendo agora — que é a informação
 * que faz a Home responder *o que importa para mim agora*.
 */
export function Task({
  task,
  detalhe,
  projeto,
  projetos,
  lembrete,
  ocupado,
  aoSalvar,
  aoMudarEstado,
  aoArquivar,
  aoLembrar,
  aoVoltar,
  aoMarcarItem,
  aoCriarItem,
  aoApagarItem,
}: {
  task: Item;
  /** A folha inteira: checklist, subtasks, referências. `null` enquanto carrega. */
  detalhe: DetalheDaTask | null;
  projeto: Projeto | null;
  projetos: Projeto[];
  /** O lembrete que aponta para esta Task, se houver. NÃO é o prazo — desde a
   *  ADR-066 a Task tem os dois, e eles respondem perguntas diferentes. */
  lembrete: Lembrete | null;
  ocupado: boolean;
  aoSalvar: (mudanca: EdicaoDeTask) => void;
  aoMudarEstado: (estado: EstadoDaTask) => void;
  aoArquivar: () => void;
  aoLembrar: () => void;
  aoVoltar: () => void;
  aoMarcarItem: (id: string, feito: boolean) => void;
  aoCriarItem: (texto: string) => void;
  aoApagarItem: (id: string) => void;
}) {
  const [titulo, setTitulo] = useState(task.title);
  const [descricao, setDescricao] = useState(task.description);
  const [confirmando, setConfirmando] = useState(false);

  useEffect(() => {
    setTitulo(task.title);
    setDescricao(task.description);
    setConfirmando(false);
  }, [task.id]);

  const mexeu = titulo !== task.title || descricao !== task.description;

  function salvar() {
    const mudanca: EdicaoDeTask = {};
    if (titulo !== task.title) mudanca.titulo = titulo;
    if (descricao !== task.description) mudanca.descricao = descricao;
    aoSalvar(mudanca);
  }

  return (
    <div className="detalhe">
      <header className="detalhe-topo">
        <button type="button" className="voltar" onClick={aoVoltar}>
          ← Fazer
        </button>
        {projeto ? <span className="etiqueta">{projeto.name}</span> : null}
      </header>

      <label className="campo">
        <span>TASK</span>
        <input
          value={titulo}
          onChange={(evento) => setTitulo(evento.currentTarget.value)}
          enterKeyHint="done"
        />
      </label>

      <label className="campo">
        <span>DESCRIÇÃO</span>
        <textarea
          value={descricao}
          rows={3}
          placeholder="o que precisa ser feito, com o detalhe que você vai esquecer"
          onChange={(evento) => setDescricao(evento.currentTarget.value)}
        />
      </label>

      {mexeu ? (
        <button
          type="button"
          className="botao"
          disabled={ocupado || !titulo.trim()}
          onClick={salvar}
        >
          Salvar
        </button>
      ) : null}

      <section className="detalhe-bloco">
        <h3>ESTADO</h3>
        {/* Uma grade de seis, e não um menu: o estado é a coisa que mais muda
            nesta tela, e escondê-lo atrás de um toque para abrir a lista faria
            o gesto mais frequente ser o mais caro. */}
        <div className="task-estados">
          {ESTADOS.map((opcao) => (
            <button
              key={opcao.valor}
              type="button"
              aria-pressed={task.state === opcao.valor}
              disabled={ocupado}
              onClick={() => aoMudarEstado(opcao.valor)}
            >
              {opcao.rotulo}
            </button>
          ))}
        </div>
      </section>

      {/* O CHECKLIST vem antes de tudo que é metadado.
          Quem abre uma task no celular quase sempre abre para marcar um passo,
          e não para trocar a prioridade. */}
      {detalhe ? (
        <Checklist
          itens={detalhe.checklist}
          ocupado={ocupado}
          aoMarcar={aoMarcarItem}
          aoCriar={aoCriarItem}
          aoApagar={aoApagarItem}
        />
      ) : null}

      {/* Todos marcados e a task ainda aberta: a tela OFERECE concluir. Ela
          nunca conclui sozinha — isso seria o sistema afirmando algo que
          ninguém disse. */}
      {detalhe && task.checklistTotal > 0 && task.checklistDone === task.checklistTotal && task.state !== "done" ? (
        <section className="detalhe-bloco">
          <p className="detalhe-nota">Todos os itens concluídos.</p>
          <button type="button" className="botao" disabled={ocupado} onClick={() => aoMudarEstado("done")}>
            Concluir task
          </button>
        </section>
      ) : null}

      <section className="detalhe-bloco">
        <h3>QUANDO</h3>
        {/* PRAZO e LEMBRETE, nesta ordem e separados.
            O prazo diz quando o trabalho vence; o lembrete diz quando o M/OS
            interrompe. Até 2026-09-08 a Task não tinha prazo — a decisão D-1 o
            mantinha fora, e esta tela dizia isso em voz alta. A ADR-066 mudou a
            decisão, e agora a tela mostra os dois. */}
        <label className="campo">
          <span>PRAZO</span>
          <input
            type="datetime-local"
            value={paraCampo(task.dueAt)}
            onChange={(evento) => aoSalvar({ prazo: doCampo(evento.currentTarget.value) })}
          />
        </label>

        {lembrete ? (
          <p className="detalhe-nota">
            Lembrete {daquiA(lembrete.nextDueAt)}
            {lembrete.snoozeCount > 0 ? ` · adiado ${lembrete.snoozeCount}×` : ""}
          </p>
        ) : (
          <button type="button" className="botao" data-variante="quieto" onClick={aoLembrar}>
            Criar lembrete
          </button>
        )}
      </section>

      <section className="detalhe-bloco">
        <h3>PRIORIDADE</h3>
        <div className="task-estados">
          {PRIORIDADES.map((opcao) => (
            <button
              key={opcao.valor}
              type="button"
              aria-pressed={task.priority === opcao.valor}
              disabled={ocupado}
              onClick={() => aoSalvar({ prioridade: opcao.valor })}
            >
              {opcao.rotulo}
            </button>
          ))}
        </div>
      </section>

      {task.waitingFor ? (
        <section className="detalhe-bloco">
          <h3>AGUARDANDO</h3>
          <p className="detalhe-nota">
            {task.waitingFor}
            {task.followUpAt ? ` · cobrar ${daquiA(task.followUpAt)}` : ""}
          </p>
        </section>
      ) : null}

      {detalhe?.subtasks.length ? (
        <section className="detalhe-bloco">
          <h3>SUBTASKS</h3>
          <ul className="lista">
            {detalhe.subtasks.map((filha) => (
              <li className="item" key={filha.id} data-feita={filha.state === "done" || undefined}>
                <div className="item-corpo">
                  <p>{filha.title}</p>
                  <small>
                    {ESTADOS.find((e) => e.valor === filha.state)?.rotulo}
                    {filha.checklistTotal ? ` · ${filha.checklistDone}/${filha.checklistTotal}` : ""}
                  </small>
                </div>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {projetos.length > 0 ? (
        <section className="detalhe-bloco">
          <h3>PROJETO</h3>
          <div className="task-projetos">
            <button
              type="button"
              aria-pressed={task.projectId === null}
              disabled={ocupado}
              onClick={() => aoSalvar({ projectId: null })}
            >
              Nenhum
            </button>
            {projetos.map((p) => (
              <button
                key={p.id}
                type="button"
                aria-pressed={task.projectId === p.id}
                disabled={ocupado}
                onClick={() => aoSalvar({ projectId: p.id })}
              >
                {p.name}
              </button>
            ))}
          </div>
        </section>
      ) : null}

      <section className="detalhe-bloco">
        {confirmando ? (
          <div className="detalhe-acoes">
            <button
              type="button"
              className="botao"
              data-variante="perigo"
              disabled={ocupado}
              onClick={aoArquivar}
            >
              Excluir mesmo
            </button>
            <button
              type="button"
              className="botao"
              data-variante="quieto"
              onClick={() => setConfirmando(false)}
            >
              Deixa
            </button>
          </div>
        ) : (
          <button
            type="button"
            className="detalhe-excluir"
            onClick={() => setConfirmando(true)}
          >
            Excluir
          </button>
        )}
      </section>
    </div>
  );
}

/**
 * O `<input type="datetime-local">` fala hora LOCAL sem fuso; o M/OS grava UTC.
 *
 * As duas conversões ficam juntas de propósito: separadas, uma delas ganharia
 * um fuso a mais na volta e o prazo andaria três horas por edição.
 */
function paraCampo(iso: string | null): string {
  if (!iso) return "";
  const data = new Date(iso);
  if (Number.isNaN(data.getTime())) return "";
  const p = (valor: number) => String(valor).padStart(2, "0");
  return `${data.getFullYear()}-${p(data.getMonth() + 1)}-${p(data.getDate())}T${p(data.getHours())}:${p(data.getMinutes())}`;
}

/** Campo vazio é AUSÊNCIA de prazo, e não data inválida: é assim que se tira. */
function doCampo(valor: string): string | null {
  if (!valor.trim()) return null;
  const data = new Date(valor);
  return Number.isNaN(data.getTime()) ? null : data.toISOString();
}

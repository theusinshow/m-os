import { useEffect, useState } from "react";
import type { EdicaoDeTask, EstadoDaTask, Lembrete, Projeto, Task as Item } from "../api";
import { daquiA } from "../instantes";

/**
 * Os estados, na ordem em que o trabalho anda.
 *
 * São os seis do Kanban do desktop, e não um subconjunto: uma Task que o PC
 * mandou para `review` precisa poder ser lida — e devolvida — do celular. Um
 * bolso que só conhecesse "aberta" e "feita" mostraria a Task em revisão como
 * se fosse mais uma aberta, e o estado que o desktop escolheu sumiria no
 * primeiro toque daqui.
 */
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
  projeto,
  projetos,
  lembrete,
  ocupado,
  aoSalvar,
  aoMudarEstado,
  aoArquivar,
  aoLembrar,
  aoVoltar,
}: {
  task: Item;
  projeto: Projeto | null;
  projetos: Projeto[];
  /** O lembrete que aponta para esta Task, se houver. É como o M/OS dá hora a
   *  uma Task — ela não tem campo de prazo, e isso é decisão de domínio. */
  lembrete: Lembrete | null;
  ocupado: boolean;
  aoSalvar: (mudanca: EdicaoDeTask) => void;
  aoMudarEstado: (estado: EstadoDaTask) => void;
  aoArquivar: () => void;
  aoLembrar: () => void;
  aoVoltar: () => void;
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

      <section className="detalhe-bloco">
        <h3>QUANDO</h3>
        {lembrete ? (
          <p className="detalhe-nota">
            {/* A Task não tem prazo: quem dá hora a ela é um lembrete apontado
                para ela. Dizer isso na tela é o que impede a pessoa de procurar
                um campo de data que não existe. */}
            Lembrete {daquiA(lembrete.nextDueAt)}
            {lembrete.snoozeCount > 0 ? ` · adiado ${lembrete.snoozeCount}×` : ""}
          </p>
        ) : (
          <>
            <p className="detalhe-aviso">
              Esta task não tem hora. No M/OS, quem dá hora a uma task é um
              lembrete apontado para ela.
            </p>
            <button type="button" className="botao" data-variante="quieto" onClick={aoLembrar}>
              Criar lembrete
            </button>
          </>
        )}
      </section>

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

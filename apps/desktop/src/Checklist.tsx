import { useEffect, useRef, useState, type DragEvent, type KeyboardEvent } from "react";

import { api } from "./api";
import { Icon } from "./Icon";
import { linhasColadas } from "./tasks";
import type { ChecklistItem, Task } from "./types";

/**
 * O checklist de uma Task — a lista de passos, e o que se faz com ela.
 *
 * # Por que ele é um componente, e não um bloco dentro da gaveta
 *
 * Ele aparece em dois lugares com o mesmo comportamento: dentro da gaveta e,
 * recolhido, dentro do cartão do Kanban. Escrito duas vezes, o Enter criaria
 * item num lugar e não no outro na primeira vez que alguém mexesse num deles.
 *
 * # As regras que este arquivo existe para manter
 *
 * **Sem modal.** Criar, editar, marcar, apagar e reordenar acontecem na própria
 * lista. Um diálogo para renomear três palavras custa dois cliques e um
 * deslocamento de olho por palavra.
 *
 * **A escrita é otimista, e o servidor é a verdade.** Marcar um item risca o
 * texto na hora e só depois espera a resposta — porque a resposta demora um
 * quadro e a pessoa clicou. Se a escrita falhar, a lista volta ao que o
 * servidor diz, e o erro aparece; ela nunca fica mostrando algo que não foi
 * gravado.
 *
 * **O checkbox é um `<input type="checkbox">` de verdade.** Não um `<div>` com
 * `role`. Leitor de tela, teclado, estado indeterminado e o gesto de arrastar
 * sobre vários vêm de graça, e nenhum deles se reimplementa direito.
 */
export function Checklist({
  taskId,
  itens,
  compacto = false,
  aoMudar,
}: {
  taskId: string;
  itens: ChecklistItem[];
  /** No cartão do Kanban a lista é mais apertada e não aceita reordenar. */
  compacto?: boolean;
  /** A Task com o progresso novo, mais a lista recarregada. */
  aoMudar: (task: Task) => void;
}) {
  const [rascunho, setRascunho] = useState("");
  const [editando, setEditando] = useState<string | null>(null);
  const [texto, setTexto] = useState("");
  const [erro, setErro] = useState("");
  /* O que a tela mostra ANTES de o servidor responder. Um mapa e não uma cópia
     da lista: assim uma resposta que chega no meio de dois cliques não desfaz o
     segundo. */
  const [otimista, setOtimista] = useState<Record<string, boolean>>({});
  const [arrastando, setArrastando] = useState<string | null>(null);
  const [sobre, setSobre] = useState<string | null>(null);
  const novo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setOtimista({});
  }, [itens]);

  const feito = (item: ChecklistItem) => otimista[item.id] ?? item.completedAt !== null;

  async function guardar(acao: () => Promise<Task>) {
    setErro("");
    try {
      aoMudar(await acao());
    } catch (falha) {
      setOtimista({});
      setErro(falha instanceof Error ? falha.message : String(falha));
    }
  }

  function alternar(item: ChecklistItem) {
    const proximo = !feito(item);
    setOtimista((atual) => ({ ...atual, [item.id]: proximo }));
    void guardar(() => api.setChecklistItemDone(item.id, proximo));
  }

  function criar() {
    const conteudo = rascunho.trim();
    if (!conteudo) return;
    setRascunho("");
    /* O campo fica FOCADO depois de criar, e é isso que faz escrever seis
       passos ser seis linhas e um Enter cada, em vez de seis cliques no "+". */
    void guardar(() => api.addChecklistItem(taskId, conteudo)).then(() => novo.current?.focus());
  }

  function renomear(item: ChecklistItem) {
    const conteudo = texto.trim();
    setEditando(null);
    if (!conteudo || conteudo === item.label) return;
    void guardar(async () => {
      await api.renameChecklistItem(item.id, conteudo);
      return api.task(taskId);
    });
  }

  function teclaDoItem(evento: KeyboardEvent<HTMLInputElement>, item: ChecklistItem) {
    if (evento.key === "Enter") {
      evento.preventDefault();
      renomear(item);
      /* Enter no último item abre o campo de criação: continuar a lista é o
         gesto seguinte mais provável, e obrigar a pegar o mouse para isso é o
         que faz uma lista de dez itens custar dez viagens. */
      if (item.id === itens[itens.length - 1]?.id) {
        window.requestAnimationFrame(() => novo.current?.focus());
      }
    }
    if (evento.key === "Escape") {
      evento.preventDefault();
      setEditando(null);
    }
  }

  function soltar(alvo: ChecklistItem) {
    const origem = arrastando;
    setArrastando(null);
    setSobre(null);
    if (!origem || origem === alvo.id) return;
    const ordem = itens.map((item) => item.id).filter((id) => id !== origem);
    const posicao = ordem.indexOf(alvo.id);
    ordem.splice(posicao < 0 ? ordem.length : posicao, 0, origem);
    void guardar(async () => {
      await api.reorderChecklist(taskId, ordem);
      return api.task(taskId);
    });
  }

  const total = itens.length;
  const concluidos = itens.filter(feito).length;

  return (
    <section className="checklist" data-compacto={compacto || undefined} aria-label="Checklist">
      {!compacto ? (
        <header className="checklist-topo">
          <span className="micro-label">CHECKLIST</span>
          {total ? (
            <span className="checklist-contagem" aria-label={`${concluidos} de ${total} concluídos`}>
              {concluidos}/{total}
            </span>
          ) : null}
        </header>
      ) : null}

      {total ? (
        <ul className="checklist-itens">
          {itens.map((item) => (
            <li
              key={item.id}
              className="checklist-item"
              data-feito={feito(item) || undefined}
              data-arrastando={arrastando === item.id || undefined}
              data-sobre={sobre === item.id || undefined}
              draggable={!compacto && editando !== item.id}
              onDragStart={(evento: DragEvent<HTMLLIElement>) => {
                /* PARA no `li`: sem isto o arrasto do item sobe para o cartão do
                   Kanban, e reordenar um passo movia a Task de coluna. */
                evento.stopPropagation();
                evento.dataTransfer.effectAllowed = "move";
                evento.dataTransfer.setData("text/checklist-item", item.id);
                setArrastando(item.id);
              }}
              onDragOver={(evento) => {
                if (!arrastando) return;
                evento.preventDefault();
                evento.stopPropagation();
                setSobre(item.id);
              }}
              onDrop={(evento) => {
                evento.preventDefault();
                evento.stopPropagation();
                soltar(item);
              }}
              onDragEnd={() => {
                setArrastando(null);
                setSobre(null);
              }}
            >
              {/* O checkbox do design system, e não um desenhado aqui: ele já
                  tem os cinco estados da folha, o foco visível e o glifo. Um
                  segundo checkbox no mesmo app divergiria na primeira mudança
                  de tema.

                  `aria-label` e não um `<label>` em volta do texto: o texto
                  entra em modo de edição, e um label que aponta para um campo
                  de texto não descreve mais o checkbox. */}
              <input
                type="checkbox"
                className="checklist-marca"
                checked={feito(item)}
                aria-label={feito(item) ? `Reabrir ${item.label}` : `Concluir ${item.label}`}
                onChange={() => alternar(item)}
                onClick={(evento) => evento.stopPropagation()}
              />

              {editando === item.id ? (
                <input
                  className="checklist-edicao"
                  value={texto}
                  autoFocus
                  onChange={(evento) => setTexto(evento.currentTarget.value)}
                  onBlur={() => renomear(item)}
                  onKeyDown={(evento) => teclaDoItem(evento, item)}
                />
              ) : (
                <button
                  type="button"
                  className="checklist-texto"
                  onClick={(evento) => {
                    evento.stopPropagation();
                    if (compacto) {
                      alternar(item);
                      return;
                    }
                    setEditando(item.id);
                    setTexto(item.label);
                  }}
                >
                  {item.label}
                </button>
              )}

              {!compacto ? (
                <button
                  type="button"
                  className="checklist-remover"
                  aria-label={`Remover ${item.label}`}
                  onClick={(evento) => {
                    evento.stopPropagation();
                    void guardar(() => api.deleteChecklistItem(item.id));
                  }}
                >
                  <Icon name="close" />
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}

      {!compacto ? (
        <div className="checklist-novo">
          <input
            ref={novo}
            value={rascunho}
            placeholder={total ? "Próximo passo" : "Adicionar primeiro item"}
            aria-label="Novo item de checklist"
            onChange={(evento) => setRascunho(evento.currentTarget.value)}
            onKeyDown={(evento) => {
              if (evento.key === "Enter") {
                evento.preventDefault();
                criar();
              }
              if (evento.key === "Escape") setRascunho("");
            }}
            onPaste={(evento) => {
              /* Colar VÁRIAS linhas cria vários itens, sem perguntar.
                 A alternativa — um diálogo "transformar 4 linhas em checklist?"
                 — pergunta o que a ação já disse: quem colou quatro linhas num
                 campo de checklist quis quatro passos. Quem quis um item só cola
                 uma linha, e o caminho dele não muda.
                 Quem divide é o domínio, no backend; aqui só se conta. */
              const colado = evento.clipboardData.getData("text");
              if (linhasColadas(colado) < 2) return;
              evento.preventDefault();
              void guardar(() => api.addChecklistItem(taskId, colado)).then(() => novo.current?.focus());
            }}
          />
          {rascunho.trim() ? (
            <button type="button" className="checklist-confirmar" onClick={criar} aria-label="Adicionar item">
              <Icon name="plus" />
            </button>
          ) : null}
        </div>
      ) : null}

      {erro ? <p className="checklist-erro" role="alert">{erro}</p> : null}
    </section>
  );
}

/**
 * A barra de progresso do cartão: `4/7` e um traço que enche.
 *
 * Sem percentual escrito. O número já responde "quanto falta" com precisão, e o
 * "57%" ao lado dele seria a mesma informação duas vezes ocupando a linha em
 * que o Project e o prazo precisam caber.
 */
export function ProgressoDoChecklist({ feitos, total }: { feitos: number; total: number }) {
  if (!total) return null;
  return (
    <span className="checklist-progresso" title={`${feitos} de ${total} concluídos`}>
      <span className="checklist-progresso-numero">
        {feitos}/{total}
      </span>
      <span className="checklist-barra" aria-hidden="true">
        <span style={{ transform: `scaleX(${feitos / total})` }} />
      </span>
    </span>
  );
}

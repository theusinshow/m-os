import { forwardRef, useMemo } from "react";
import { SmallIcon } from "../Icon";
import type { ConversationSummary } from "../hermes";

/**
 * A lista de conversas.
 *
 * # Por que agrupada por tempo
 *
 * Conversas não são arquivos: a maioria morre no mesmo dia. Uma pilha única
 * ordenada por data faz o usuário procurar a de dez minutos atrás no meio de
 * seis semanas de histórico. HOJE / ONTEM / a data é como a memória arquiva.
 *
 * # O que cada linha mostra, e o que ela recusa mostrar
 *
 * Título — ou o começo da primeira mensagem, enquanto o Hermes não nomeia — e
 * a hora. Contagem de mensagens saiu: ela nunca respondeu nenhuma pergunta que
 * alguém fizesse a essa lista, e disputava a linha com o título, que é a única
 * coisa que faz alguém reconhecer a conversa.
 */

function grupoDe(valor: string) {
  const data = new Date(valor);
  const hoje = new Date();
  const ontem = new Date(hoje);
  ontem.setDate(hoje.getDate() - 1);
  if (data.toDateString() === hoje.toDateString()) return "HOJE";
  if (data.toDateString() === ontem.toDateString()) return "ONTEM";
  return new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "short" }).format(data).toUpperCase();
}

function relogio(valor: string) {
  return new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(new Date(valor));
}

export const ConversationRail = forwardRef<HTMLElement, {
  summaries: ConversationSummary[];
  atual: string;
  busca: string;
  setBusca: (next: string) => void;
  compacto: boolean;
  onNova: () => void;
  onAbrir: (id: string) => void;
  onExcluir: (id: string) => void;
  onFechar: () => void;
  fecharRef: React.RefObject<HTMLButtonElement | null>;
}>(function ConversationRail(
  { summaries, atual, busca, setBusca, compacto, onNova, onAbrir, onExcluir, onFechar, fecharRef },
  ref,
) {
  const grupos = useMemo(() => {
    const termo = busca.trim().toLowerCase();
    const filtradas = termo
      ? summaries.filter((item) => `${item.title} ${item.preview}`.toLowerCase().includes(termo))
      : summaries;
    const saida: { label: string; items: ConversationSummary[] }[] = [];
    for (const item of filtradas) {
      const label = grupoDe(item.updatedAt);
      const balde = saida.find((grupo) => grupo.label === label);
      if (balde) balde.items.push(item);
      else saida.push({ label, items: [item] });
    }
    return saida;
  }, [summaries, busca]);

  return (
    <aside
      className="hermes-rail"
      aria-label="Conversas"
      aria-modal={compacto || undefined}
      ref={ref}
      role={compacto ? "dialog" : undefined}
    >
      <div className="hermes-rail-head">
        <span className="micro-label">CONVERSAS</span>
        <button type="button" onClick={onNova} title="Nova conversa · Ctrl+N" aria-label="Nova conversa">
          <SmallIcon name="plus" />
        </button>
        {compacto ? (
          <button ref={fecharRef} type="button" onClick={onFechar} title="Fechar conversas · Esc" aria-label="Fechar conversas">
            <SmallIcon name="close" />
          </button>
        ) : null}
      </div>

      <input
        className="hermes-rail-search"
        value={busca}
        onChange={(evento) => setBusca(evento.currentTarget.value)}
        placeholder="Buscar conversas"
        aria-label="Buscar nas conversas"
      />

      <div className="hermes-rail-list">
        {grupos.length ? grupos.map((grupo) => (
          <div className="hermes-rail-group" key={grupo.label}>
            <div className="micro-label">{grupo.label}</div>
            {grupo.items.map((item) => (
              <div className="hermes-rail-row" key={item.id} data-active={item.id === atual || undefined}>
                <button type="button" className="hermes-rail-open" onClick={() => onAbrir(item.id)}>
                  <span className="hermes-rail-title">{item.title || item.preview || "Conversa vazia"}</span>
                  <span className="hermes-rail-meta">{relogio(item.updatedAt)}</span>
                </button>
                <button
                  type="button"
                  className="hermes-rail-drop"
                  aria-label={`Excluir ${item.title || "conversa"}`}
                  title="Excluir conversa"
                  onClick={() => onExcluir(item.id)}
                >
                  <SmallIcon name="trash" />
                </button>
              </div>
            ))}
          </div>
        )) : (
          <p className="hermes-quiet">{busca ? "Nenhuma conversa encontrada." : "Nenhuma conversa ainda."}</p>
        )}
      </div>
    </aside>
  );
});

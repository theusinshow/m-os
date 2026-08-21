import type { Capture, Project, Task } from "../types";

/**
 * A conversa vazia.
 *
 * # Por que as sugestões são perguntas de verdade
 *
 * Uma lista de exemplos genéricos — "resuma um texto", "escreva um e-mail" —
 * ensina o que um chatbot faz, e não o que ESTE sistema sabe. As três linhas
 * abaixo são lidas do M/OS de agora: quantos itens estão parados na Inbox, qual
 * Project não recebe toque há semanas, quantas Tasks estão em andamento. Se não
 * houver nada, a tela diz isso — e o silêncio é a informação.
 *
 * # Por que não são cards
 *
 * O redesign pede silêncio visual, e três cards grandes numa tela vazia são o
 * oposto disso. São linhas de texto clicáveis, numeradas, com a numeração
 * servindo de alvo. Ocupam a largura de uma frase, não a da tela.
 */

/** Sugestões lidas do estado de agora. Somem depois da primeira mensagem. */
export function sugestoesDe(inbox: Capture[], projects: Project[], tasks: Task[]): string[] {
  const sugestoes: string[] = [];

  if (inbox.length) {
    const antiga = inbox.reduce((esquerda, direita) => (esquerda.capturedAt < direita.capturedAt ? esquerda : direita));
    const desde = new Intl.DateTimeFormat("pt-BR", { day: "numeric", month: "long" }).format(new Date(antiga.capturedAt));
    sugestoes.push(`${inbox.length} ${inbox.length === 1 ? "item" : "itens"} na Inbox desde ${desde} — organizar em lote?`);
  }

  const parado = projects
    .filter((project) => project.lifecycleState === "active")
    .filter((project) => Date.now() - new Date(project.updatedAt).getTime() > 21 * 24 * 60 * 60 * 1000)
    .sort((esquerda, direita) => esquerda.updatedAt.localeCompare(direita.updatedAt))[0];
  if (parado) sugestoes.push(`${parado.name} está parado há semanas — retomar ou arquivar?`);

  const fazendo = tasks.filter((task) => task.lifecycleState === "active" && task.state === "doing");
  if (fazendo.length) {
    sugestoes.push(`${fazendo.length} ${fazendo.length === 1 ? "task está" : "tasks estão"} em andamento — revisar o dia?`);
  }

  return sugestoes.slice(0, 3);
}

export function EmptyConversation({ sugestoes, online, onPerguntar }: {
  sugestoes: string[];
  online: boolean;
  onPerguntar: (texto: string) => void;
}) {
  return (
    <div className="hermes-empty-state">
      <p className="hermes-empty-title">
        Pergunte, mande fazer
        <br />
        ou jogue alguma coisa aqui.
      </p>

      {sugestoes.length ? (
        <div className="hermes-suggestions">
          <span className="micro-label">AGORA NO M/OS</span>
          {sugestoes.map((sugestao, indice) => (
            <button key={sugestao} type="button" disabled={!online} onClick={() => onPerguntar(sugestao)}>
              <span aria-hidden="true">{indice + 1}</span>
              {sugestao}
            </button>
          ))}
        </div>
      ) : (
        <p className="hermes-quiet">Nada pedindo atenção agora. Inbox vazia, nenhum Project parado.</p>
      )}
    </div>
  );
}

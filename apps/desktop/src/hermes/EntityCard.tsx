import type { ReactNode } from "react";
import type { TouchedEntity } from "../hermes";

/**
 * O que uma ação do Hermes alcançou, com materialidade.
 *
 * # Por que card, se a direção Marginalia recusa cards
 *
 * Ela recusa card para o que o Hermes **diz** — prosa emoldurada vira slide, e
 * a coluna de leitura existe justamente para a prosa correr solta. Mas o que
 * ele **fez** não é prosa: é um objeto do M/OS que passou a existir, e um
 * objeto precisa de arestas para poder ser apontado, aberto e conferido.
 *
 * A regra continua a mesma de antes, só aplicada com mais precisão: texto sem
 * moldura, efeito com moldura.
 *
 * # Por que os dados vêm do rastro, e não do texto
 *
 * `TouchedEntity` é o que a execução resolveu em id — não o que o modelo
 * escreveu. Montar o card a partir da prosa exigiria adivinhar qual "task do
 * Victor" ele quis dizer, e um card que aponta para o objeto errado é pior que
 * card nenhum. Sem rastro, não há card.
 */

const RÓTULO: Record<string, string> = {
  task: "TASK",
  project: "PROJECT",
  capture: "CAPTURE",
  resource: "RESOURCE",
  reminder: "LEMBRETE",
  meeting: "REUNIÃO",
  timeEntry: "SESSÃO",
  workspace: "WORKSPACE",
};

/** O nome do tipo, para um `kind` que este renderer ainda não conhece.
 *  Devolver o cru é honesto; devolver "ITEM" apagaria o que o backend sabia. */
export function rotuloDaEntidade(kind: string): string {
  return RÓTULO[kind] ?? kind.toUpperCase();
}

export function EntityCard({ entity, marca, linhas, onOpen }: {
  entity: TouchedEntity;
  /** O sinal à esquerda do tipo. `✓` para feito, vazio para referência. */
  marca?: string;
  /** Metadados já resolvidos. Nada é inferido aqui dentro. */
  linhas?: string[];
  onOpen?: () => void;
}) {
  return (
    <article className="hermes-entidade" data-kind={entity.kind}>
      <header>
        {marca ? <span className="hermes-entidade-marca" aria-hidden="true">{marca}</span> : null}
        <span className="micro-label">{rotuloDaEntidade(entity.kind)}</span>
      </header>
      <p className="hermes-entidade-titulo">{entity.label}</p>
      {linhas?.length ? (
        <p className="hermes-entidade-meta">{linhas.filter(Boolean).join(" · ")}</p>
      ) : null}
      {onOpen ? (
        <button type="button" className="hermes-entidade-abrir" onClick={onOpen}>
          Abrir
        </button>
      ) : null}
    </article>
  );
}

/**
 * O bloco de resultado de uma proposta executada.
 *
 * Uma ação costuma tocar mais de uma coisa — criar Task a partir de Capture
 * toca as duas —, e o desfecho só é compreensível quando as duas aparecem. O
 * texto do recibo fica ACIMA dos cards porque é ele que diz o que aconteceu;
 * os cards dizem com o quê.
 */
export function AgentResult({ resumo, entities, aoAbrir, extra }: {
  resumo: string;
  entities: TouchedEntity[];
  aoAbrir?: (entity: TouchedEntity) => (() => void) | undefined;
  extra?: ReactNode;
}) {
  return (
    <div className="hermes-resultado">
      {resumo ? <p className="hermes-resultado-resumo">{resumo}</p> : null}
      {entities.length ? (
        <div className="hermes-resultado-cards">
          {entities.map((entity) => (
            <EntityCard
              key={`${entity.kind}-${entity.id}`}
              entity={entity}
              marca="✓"
              onOpen={aoAbrir?.(entity)}
            />
          ))}
        </div>
      ) : null}
      {extra}
    </div>
  );
}

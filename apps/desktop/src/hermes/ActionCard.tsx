import { useState } from "react";
import {
  conversations as conversationApi,
  type ActionResolution,
  type MessagePart,
  type TouchedEntity,
} from "../hermes";
import { AgentResult } from "./EntityCard";

type Proposta = Extract<MessagePart["body"], { kind: "action_proposal" }>;

/**
 * O que o Hermes propôs, e o que aconteceu depois.
 *
 * # Por que o cartão aparece mesmo para risco baixo
 *
 * Ele é a explicação do que o Hermes ENTENDEU. Quem clica "Criar Task" na
 * interface escolheu; quem falou uma frase pode ter sido mal interpretado. O
 * risco decide o peso da confirmação, não a existência dela.
 *
 * # Por que o desfecho vira card de entidade
 *
 * Antes, uma proposta executada virava uma frase — "Task criada" — e a coisa
 * criada não existia na tela. O rastro da execução já sabia qual objeto foi
 * tocado; agora ele aparece com arestas, e o resultado deixa de ser uma notícia
 * sobre algo invisível.
 */
export function ActionCard({ part, messageId, onResolved, aoAbrir }: {
  part: Proposta;
  messageId: string;
  onResolved: (resolution: ActionResolution) => void;
  aoAbrir?: (entity: TouchedEntity) => (() => void) | undefined;
}) {
  const [trabalhando, setTrabalhando] = useState(false);
  const [falha, setFalha] = useState("");

  async function resolver(aprovado: boolean) {
    setTrabalhando(true);
    setFalha("");
    const resolucao = await conversationApi
      .resolveAction(messageId, part.raw, aprovado)
      .catch((causa) => {
        // O erro fica NO cartão. Mandar procurar o motivo em outro lugar desfaz
        // o motivo de o cartão existir, e uma ação que falha em silêncio é a
        // pior coisa que um agente pode fazer.
        setFalha(causa instanceof Error ? causa.message : String(causa));
        return null;
      });
    setTrabalhando(false);
    if (resolucao) onResolved(resolucao);
  }

  const executada = part.status === "executed";
  const entidades = part.audit?.entities ?? [];

  return (
    <div className="hermes-action" data-status={part.status} data-risk={part.preview.risk}>
      <div className="hermes-action-head">
        <span className="hermes-action-mark" aria-hidden="true">
          {executada ? "✓" : part.status === "failed" ? "!" : "▸"}
        </span>
        <span className="micro-label">{part.preview.title}</span>
        {part.preview.risk !== "low" ? (
          <span className="micro-label" data-risk>RISCO {part.preview.risk === "high" ? "ALTO" : "MÉDIO"}</span>
        ) : null}
      </div>

      {part.preview.lines.length ? (
        <dl className="hermes-action-lines">
          {part.preview.lines.map((entrada) => (
            <div key={entrada.label}>
              <dt>{entrada.label}</dt>
              <dd>{entrada.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}

      {part.status === "pending" ? (
        <div className="hermes-action-foot">
          <button type="button" disabled={trabalhando} onClick={() => void resolver(false)}>Cancelar</button>
          <button type="button" data-primary disabled={trabalhando} onClick={() => void resolver(true)}>
            {trabalhando ? "Executando…" : part.preview.risk === "high" ? "Confirmar" : "Fazer"}
          </button>
        </div>
      ) : (
        <AgentResult resumo={part.outcome} entities={entidades} aoAbrir={aoAbrir} />
      )}

      {falha ? (
        <p className="hermes-action-falha" role="alert">
          <span aria-hidden="true">! </span>{falha}
          <button type="button" onClick={() => void resolver(true)}>Tentar de novo</button>
        </p>
      ) : null}
    </div>
  );
}

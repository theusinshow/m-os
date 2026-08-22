import { memo } from "react";
import { Markdown } from "../markdown";
import {
  messageText,
  type ActionResolution,
  type ContextOrigin,
  type Message,
  type MessagePart,
  type TouchedEntity,
} from "../hermes";
import { ActionCard } from "./ActionCard";
import { AgentActivity } from "./AgentActivity";
import { ContextChip } from "./ContextChip";
import { explicaOFim, type Passo } from "./atividade";

function relogio(valor: string) {
  return new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" }).format(new Date(valor));
}

function passosDe(message: Message): Passo[] {
  return message.parts
    .filter((part) => part.body.kind === "tool_run")
    .map((part) => {
      const body = part.body as Extract<MessagePart["body"], { kind: "tool_run" }>;
      return { name: body.name, state: body.state };
    });
}

/**
 * A pergunta.
 *
 * # Por que ela ganhou moldura, se a direção recusa bolha
 *
 * O que a direção recusa é a bolha do messenger: cantos de 18px, cor de marca,
 * cauda apontando para um avatar. O problema que ela resolve, porém, é real —
 * numa thread longa, pergunta e resposta em prosa corrida se confundem, e o
 * olho perde onde um turno começa.
 *
 * A resposta aqui é estrutural e não decorativa: a pergunta recua para a
 * direita, ganha uma superfície um degrau acima do fundo, uma borda de 1px e o
 * raio padrão do sistema — o MESMO de todo card do M/OS. Ela não é maior que o
 * texto do Hermes nem colorida; só está claramente em outro plano. E a largura
 * segue o conteúdo, então uma pergunta de três palavras não vira um retângulo
 * de trinta centímetros.
 */
const UserTurn = memo(function UserTurn({ message, onCopy, onEdit }: {
  message: Message;
  onCopy: (message: Message) => void;
  onEdit: (message: Message) => void;
}) {
  const texto = messageText(message);
  const chips = message.parts.filter((part) => part.body.kind === "context_ref");

  return (
    <article className="hermes-turn" data-role="user">
      <div className="hermes-gutter">
        <div className="hermes-gutter-who">VOCÊ</div>
        <time dateTime={message.createdAt}>{relogio(message.createdAt)}</time>
      </div>
      <div className="hermes-said">
        <div className="hermes-question-wrap">
          <p className="hermes-question">{texto}</p>
        </div>
        {chips.length ? (
          <div className="hermes-chips">
            {chips.map((part) => {
              const body = part.body as Extract<MessagePart["body"], { kind: "context_ref" }>;
              return (
                <ContextChip
                  key={part.id}
                  entity={body.entity}
                  label={body.label}
                  origin={body.origin as ContextOrigin}
                  detail={`Enviado: ${body.fields.join(", ") || "nada"} · ${body.bytes} bytes`}
                />
              );
            })}
          </div>
        ) : null}
        <div className="hermes-actions">
          <button type="button" onClick={() => onCopy(message)}>Copiar</button>
          <button type="button" onClick={() => onEdit(message)}>Editar pergunta</button>
        </div>
      </div>
    </article>
  );
});

/**
 * A resposta.
 *
 * `memo` não é otimização preventiva: numa thread longa toda mensagem fechada é
 * imutável, e sem isto o Markdown de todas elas era reparseado a cada token da
 * resposta em curso.
 */
const AssistantTurn = memo(function AssistantTurn({ message, onCopy, onRegenerate, onResolved, aoAbrir }: {
  message: Message;
  onCopy: (message: Message) => void;
  onRegenerate: (message: Message) => void;
  onResolved: (resolution: ActionResolution) => void;
  aoAbrir?: (entity: TouchedEntity) => (() => void) | undefined;
}) {
  const texto = messageText(message);
  const falhou = message.status === "failed" || message.status === "interrupted";

  return (
    <article className="hermes-turn" data-role="assistant">
      <AgentActivity passos={passosDe(message)} decorrido="" vivo={false} />
      <div className="hermes-said">
        {message.parts.map((part) => {
          if (part.body.kind === "status") {
            return <p className="hermes-system-line" key={part.id}>{part.body.text}</p>;
          }
          if (part.body.kind === "reasoning") {
            return (
              <details className="hermes-reasoning" key={part.id}>
                <summary>Raciocínio</summary>
                <p>{part.body.text}</p>
              </details>
            );
          }
          if (part.body.kind === "error") {
            return <p className="hermes-failed" key={part.id}><span aria-hidden="true">! </span>{part.body.message}</p>;
          }
          if (part.body.kind === "action_proposal") {
            return (
              <ActionCard
                key={part.id}
                part={part.body}
                messageId={message.id}
                onResolved={onResolved}
                aoAbrir={aoAbrir}
              />
            );
          }
          if (part.body.kind === "text") return <Markdown key={part.id} source={part.body.text} />;
          return null;
        })}

        {/* O motivo do fim é GRAVADO como parte de status pelo `settle_turn`, e
            desenhado ali em cima como qualquer outra linha de sistema. Esta
            linha aqui é só o fallback para as mensagens antigas, gravadas antes
            de 2026-08-22, que não têm a parte.

            Ela não diz "por você": os quatro finais precoces compartilham o
            mesmo `interrupted`, e afirmar autoria a partir dele era o defeito —
            uma queda de túnel aparecia na tela como decisão do usuário. */}
        {message.status === "interrupted" && !explicaOFim(message) ? (
          <p className="hermes-system-line">Interrompido.</p>
        ) : null}

        <div className="hermes-actions">
          {texto ? <button type="button" onClick={() => onCopy(message)}>Copiar</button> : null}
          <button type="button" onClick={() => onRegenerate(message)}>
            {falhou ? "Tentar de novo" : "Refazer"}
          </button>
        </div>
      </div>
    </article>
  );
});

export function MessageTurn(props: {
  message: Message;
  onCopy: (message: Message) => void;
  onEdit: (message: Message) => void;
  onRegenerate: (message: Message) => void;
  onResolved: (resolution: ActionResolution) => void;
  aoAbrir?: (entity: TouchedEntity) => (() => void) | undefined;
}) {
  if (props.message.role === "user") {
    return <UserTurn message={props.message} onCopy={props.onCopy} onEdit={props.onEdit} />;
  }
  return (
    <AssistantTurn
      message={props.message}
      onCopy={props.onCopy}
      onRegenerate={props.onRegenerate}
      onResolved={props.onResolved}
      aoAbrir={props.aoAbrir}
    />
  );
}

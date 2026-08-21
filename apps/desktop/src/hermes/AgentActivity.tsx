import { useState } from "react";
import { DecryptedText } from "../motion/DecryptedText";
import { MARCA_DO_PASSO, NOME_DO_ESTADO, reciboAlerta, reciboDosPassos, type Passo } from "./atividade";

/**
 * A margem de uma resposta: quem fala, e o que o sistema fez para responder.
 *
 * Durante o turno, os passos aparecem um a um com o corrente pulsando. Depois
 * que assenta, tudo colapsa para uma linha — "3 fontes consultadas" — que se
 * abre no clique.
 *
 * O dispositivo inteiro existe para que a atividade NUNCA empurre a prosa para
 * baixo: ela mora na coluna estreita, ao lado, e a coluna de leitura fica
 * parada enquanto o texto chega. Uma lista de ferramentas crescendo acima do
 * texto faria a resposta pular a cada ferramenta.
 *
 * Nada aqui expõe raciocínio. Ferramenta que rodou é fato operacional; o que o
 * modelo pensou continua atrás de um `<details>` fechado, na coluna de leitura.
 */
export function AgentActivity({ passos, decorrido, vivo }: {
  passos: Passo[];
  decorrido: string;
  vivo: boolean;
}) {
  const [aberto, setAberto] = useState(false);
  const assentou = !vivo && passos.length > 0;
  const recibo = reciboDosPassos(passos, decorrido);

  return (
    <div className="hermes-gutter">
      <div className="hermes-gutter-who">HERMES</div>

      {assentou && !aberto ? (
        <button
          type="button"
          className="hermes-receipt"
          data-alerta={reciboAlerta(passos) || undefined}
          onClick={() => setAberto(true)}
          aria-expanded={false}
        >
          {recibo}
        </button>
      ) : null}

      {(vivo || aberto) && passos.length ? (
        <ul className="hermes-steps">
          {passos.map((passo, indice) => (
            <li key={`${passo.name}-${indice}`} data-state={passo.state}>
              <span className="hermes-step-mark" aria-hidden="true">{MARCA_DO_PASSO[passo.state]}</span>
              <span className="hermes-step-name">{passo.name}</span>
              <span className="visually-hidden"> — {NOME_DO_ESTADO[passo.state]}</span>
            </li>
          ))}
        </ul>
      ) : null}

      {vivo && !passos.length ? (
        <div className="hermes-thinking">
          <DecryptedText text="pensando…" duration={240} />
        </div>
      ) : null}

      {assentou && aberto ? (
        <button type="button" className="hermes-receipt" onClick={() => setAberto(false)} aria-expanded>
          fechar
        </button>
      ) : null}
    </div>
  );
}

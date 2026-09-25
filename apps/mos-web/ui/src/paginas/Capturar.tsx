import type { Capture } from "../api";
import type { Pendente } from "../filaDeCaptura";
import { idade } from "./idade";
import { Vazio } from "../componentes/Vazio";

/**
 * A pagina de capturar mostra o que ACABOU de entrar.
 *
 * Ela era uma frase de instrucao e trezentos pixels de nada, e o vazio fazia a
 * tela parecer quebrada logo depois de guardar algo. As tres ultimas capturas
 * custam o mesmo pedido que a inbox ja faz e respondem a unica pergunta de quem
 * acabou de escrever: entrou?
 */
export function Capturar({ capturas, naFila = [] }: { capturas: Capture[]; naFila?: Pendente[] }) {
  // O que está no aparelho aparece ANTES e dito como tal: "guardado aqui,
  // aguardando envio" é diferente de "recebido pelo M/OS" (F3.3).
  const fila = naFila.length ? (
    <>
      <p className="rotulo">NESTE APARELHO · AGUARDANDO ENVIO</p>
      <ul className="lista">
        {naFila.map((item) => (
          <li className="item" key={item.chave} data-pendente>
            <div className="item-corpo">
              <p>{item.texto}</p>
              <small>{item.recusa ? `O M/OS recusou: ${item.recusa}` : `${idade(item.capturadaEm)} · vai quando a conexão voltar`}</small>
            </div>
          </li>
        ))}
      </ul>
    </>
  ) : null;
  if (capturas.length === 0) {
    return fila ?? <Vazio frase="O que estiver na cabeça vai para a Inbox. Organizar é depois." />;
  }
  return (
    <>
      {fila}
      <p className="rotulo">ÚLTIMAS</p>
      <ul className="lista">
        {capturas.slice(0, 3).map((capture) => (
          <li className="item" key={capture.id}>
            <div className="item-corpo">
              <p>{capture.content}</p>
              <small>{idade(capture.capturedAt)}</small>
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}

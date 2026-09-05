import type { Capture } from "../api";
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
export function Capturar({ capturas }: { capturas: Capture[] }) {
  if (capturas.length === 0) {
    return <Vazio frase="O que estiver na cabeça vai para a Inbox. Organizar é depois." />;
  }
  return (
    <>
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

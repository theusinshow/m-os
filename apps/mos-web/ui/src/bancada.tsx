import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "./estilo.css";
import { FALSO } from "./falso";

/**
 * A bancada.
 *
 * O `mos-web` de verdade exige sessao — `/app/*` sem passkey cai na porta —,
 * entao fotografar o app real custa a cerimonia inteira antes de cada olhada. E
 * o que custa caro se faz pouco. Aqui as telas nascem com dado falso e o CSS de
 * verdade, lado a lado, nas duas larguras que importam.
 *
 * O que ela NAO prova: nada que dependa do servidor. Comportamento continua
 * sendo assunto dos testes, e do app de verdade no aparelho.
 */
const LARGURAS = [390, 430];

function Moldura({
  titulo,
  largura,
  children,
}: {
  titulo: string;
  largura: number;
  children: ReactNode;
}) {
  return (
    <figure className="bancada-moldura">
      <figcaption>
        {titulo} · {largura}px
      </figcaption>
      <div className="bancada-tela" style={{ width: largura, height: 780 }}>
        {children}
      </div>
    </figure>
  );
}

function Bancada() {
  return (
    <div className="bancada">
      {LARGURAS.map((largura) => (
        <Moldura key={largura} titulo="Nada ainda" largura={largura}>
          <p style={{ padding: 20 }}>
            As paginas entram aqui conforme as tasks seguintes as criarem. Capturas
            falsas: {FALSO.capturas.length}.
          </p>
        </Moldura>
      ))}
    </div>
  );
}

createRoot(document.getElementById("raiz")!).render(<Bancada />);

import type { EstadoDoAparelho } from "../api";
import { Cartao } from "../componentes/Cartao";
import type { Dados, Pagina } from "../navegacao";
import { cartoesDaHome } from "./cartoes";

/**
 * O hub.
 *
 * O app abria no compositor e nunca dizia como estao as coisas. A Home responde
 * isso antes de qualquer lista, com o dado que o servidor JA devolve — horas do
 * CronoCAD e academico entram quando existir a rota de resumo.
 */
export function Home({
  estado,
  dados,
  aoIr,
}: {
  estado: EstadoDoAparelho | null;
  dados: Dados;
  aoIr: (pagina: Pagina) => void;
}) {
  const cartoes = cartoesDaHome(estado, dados);
  return (
    <div className="home">
      {cartoes.map((cartao) => (
        <Cartao
          key={cartao.chave}
          rotulo={cartao.rotulo}
          numero={cartao.numero}
          legenda={cartao.legenda}
          urgente={cartao.urgente}
          largo={cartao.chave === "ultima"}
          aoTocar={() => aoIr(cartao.destino)}
        />
      ))}
    </div>
  );
}

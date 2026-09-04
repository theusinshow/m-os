import { useState } from "react";
import type { EstadoDoAparelho, Panorama } from "../api";
import { Cartao } from "../componentes/Cartao";
import type { Dados, Pagina } from "../navegacao";
import { aplicarArranjo, mostrar, mover, ocultar, type Arranjo } from "./arranjo";
import { cartoesDaHome } from "./cartoes";

/**
 * O hub.
 *
 * O app abria no compositor e nunca dizia como estao as coisas. A Home responde
 * isso antes de qualquer lista, com o dado que o servidor JA devolve — horas do
 * CronoCAD e academico entram quando existir a rota de resumo.
 *
 * # Por que arrumar é um MODO, e não arrastar
 *
 * A Home inteira é feita de alvos de toque, e num aparelho na mão o gesto de
 * arrastar um cartão é o mesmo gesto de rolar a tela: sem um modo, metade das
 * tentativas de mover viraria uma rolagem, e metade das rolagens viraria um
 * cartão fora de lugar. Dentro do modo, o toque no cartão não navega — o que se
 * pode fazer ali é mover e esconder, e nada mais.
 */
export function Home({
  estado,
  dados,
  panorama,
  arranjo,
  aoArranjar,
  aoIr,
}: {
  estado: EstadoDoAparelho | null;
  dados: Dados;
  /** Nulo enquanto não chegou, ou quando o servidor é antigo demais para ter a
   *  rota. A Home continua inteira nos dois casos. */
  panorama: Panorama | null;
  arranjo: Arranjo;
  aoArranjar: (arranjo: Arranjo) => void;
  aoIr: (pagina: Pagina) => void;
}) {
  const [arrumando, setArrumando] = useState(false);

  const todos = cartoesDaHome(estado, dados, new Date(), panorama);
  const cartoes = aplicarArranjo(todos, arranjo);
  const visiveis = cartoes.map((cartao) => cartao.chave);

  // Um cartão escondido pode não estar mais na Home — a inbox esvaziou, a
  // semana zerou. Ele continua listado em ESCONDIDOS pela chave, senão a única
  // forma de trazê-lo de volta seria fazer o dado voltar primeiro.
  const escondidos = arranjo.ocultos.map((chave) => ({
    chave,
    rotulo: todos.find((cartao) => cartao.chave === chave)?.rotulo ?? chave.toUpperCase(),
  }));

  return (
    <div className="home" data-arrumando={arrumando || undefined}>
      <div className="home-topo">
        <button type="button" className="home-arrumar" onClick={() => setArrumando(!arrumando)}>
          {arrumando ? "Pronto" : "Arrumar"}
        </button>
      </div>

      {cartoes.map((cartao) => (
        <div
          className="home-slot"
          key={cartao.chave}
          data-largo={cartao.chave === "ultima" || undefined}
        >
          <Cartao
            rotulo={cartao.rotulo}
            numero={cartao.numero}
            legenda={cartao.legenda}
            urgente={cartao.urgente}
            largo={cartao.chave === "ultima"}
            // Arrumando, o cartão não leva a lugar nenhum: quem está mexendo na
            // ordem erra o alvo o tempo todo, e sair da Home a cada erro faria
            // recomeçar do zero.
            aoTocar={() => (arrumando ? undefined : aoIr(cartao.destino))}
          />
          {arrumando ? (
            <div className="home-controles">
              <button
                type="button"
                aria-label={`Subir ${cartao.rotulo}`}
                disabled={visiveis[0] === cartao.chave}
                onClick={() => aoArranjar(mover(arranjo, visiveis, cartao.chave, "cima"))}
              >
                ↑
              </button>
              <button
                type="button"
                aria-label={`Descer ${cartao.rotulo}`}
                disabled={visiveis[visiveis.length - 1] === cartao.chave}
                onClick={() => aoArranjar(mover(arranjo, visiveis, cartao.chave, "baixo"))}
              >
                ↓
              </button>
              <button
                type="button"
                className="home-esconder"
                onClick={() => aoArranjar(ocultar(arranjo, cartao.chave))}
              >
                Esconder
              </button>
            </div>
          ) : null}
        </div>
      ))}

      {/* Só dentro do modo: fora dele, uma lista do que você mandou sumir é
          exatamente o que você pediu para não ver. */}
      {arrumando && escondidos.length > 0 ? (
        <section className="home-escondidos">
          <h2>ESCONDIDOS</h2>
          <div>
            {escondidos.map((cartao) => (
              <button
                type="button"
                key={cartao.chave}
                onClick={() => aoArranjar(mostrar(arranjo, cartao.chave))}
              >
                {cartao.rotulo}
              </button>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}

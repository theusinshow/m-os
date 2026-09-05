import { CENTRAL, contagemDe, DESTINOS, type Dados, type Pagina } from "../navegacao";
import { Icone } from "./icones";
import { Marca } from "./Marca";

/**
 * A barra de baixo.
 *
 * Ela mora no alcance do polegar, e cada alvo tem `--toque` de altura: abaixo de
 * 44px o dedo erra, e errar aqui significa abrir a pagina errada com o aparelho
 * na mao no meio da rua.
 *
 * # O botao do meio nao e um destino
 *
 * Capturar e a razao de existir do app, e como quinto texto igual aos outros ele
 * pedia a mesma mira que "Mais". Agora e um alvo de 52px em sodio, no centro
 * exato da barra — o unico lugar que o polegar acha sem olhar — e carrega a
 * barra da marca em vez de um lapis generico.
 *
 * O icone vem antes do rotulo, e o rotulo fica: icone sozinho e adivinhacao, e
 * um app que se abre uma vez por dia nao tem quantas repeticoes um icone precisa
 * para virar palavra.
 */
export function Barra({
  atual,
  dados,
  aoIr,
}: {
  atual: Pagina;
  dados: Dados;
  aoIr: (pagina: Pagina) => void;
}) {
  return (
    <nav className="barra" aria-label="Seções">
      {DESTINOS.map(({ pagina, rotulo }) => {
        if (pagina === CENTRAL) {
          return (
            <button
              key={pagina}
              type="button"
              className="barra-central"
              aria-label={rotulo}
              aria-current={atual === pagina ? "page" : undefined}
              onClick={() => aoIr(pagina)}
            >
              <Marca tamanho={22} />
            </button>
          );
        }
        const conta = contagemDe(pagina, dados);
        return (
          <button
            key={pagina}
            type="button"
            aria-current={atual === pagina ? "page" : undefined}
            onClick={() => aoIr(pagina)}
          >
            <Icone pagina={pagina} />
            <span>{rotulo}</span>
            {conta > 0 ? (
              <b className="conta" data-urgente={pagina === "mais" || undefined}>
                {conta}
              </b>
            ) : null}
          </button>
        );
      })}
    </nav>
  );
}

import { contagemDe, DESTINOS, type Dados, type Pagina } from "../navegacao";

/**
 * A barra de baixo.
 *
 * Ela mora no alcance do polegar, e cada alvo tem `--toque` de altura: abaixo de
 * 44px o dedo erra, e errar aqui significa abrir a pagina errada com o aparelho
 * na mao no meio da rua.
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
        const conta = contagemDe(pagina, dados);
        return (
          <button
            key={pagina}
            type="button"
            aria-current={atual === pagina ? "page" : undefined}
            onClick={() => aoIr(pagina)}
          >
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

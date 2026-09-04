import type { HorasDeProjeto } from "../api";
import { Vazio } from "../componentes/Vazio";
import { emHoras, emReais } from "./numeros";

/**
 * As horas por projeto, na janela escolhida.
 *
 * # Por que a barra, e não só o número
 *
 * A pergunta que traz alguém aqui não é "quantas horas no Rancho" — é "onde foi
 * o meu tempo". Uma lista de números obriga a comparar de cabeça; a barra
 * responde antes de ler, e o número fica para quem quiser conferir.
 *
 * A barra é proporcional ao MAIOR da lista, e não ao total: contra o total, uma
 * semana espalhada em seis projetos vira seis tracinhos indistinguíveis.
 */
export function Horas({
  linhas,
  janela,
  aoTrocarJanela,
}: {
  linhas: HorasDeProjeto[];
  janela: "semana" | "mes";
  aoTrocarJanela: (janela: "semana" | "mes") => void;
}) {
  const maior = Math.max(...linhas.map((linha) => linha.segundos), 1);
  const totalSegundos = linhas.reduce((soma, linha) => soma + linha.segundos, 0);
  const totalValor = linhas.reduce((soma, linha) => soma + linha.valorCents, 0);

  return (
    <div className="horas">
      <div className="horas-janela">
        {(["semana", "mes"] as const).map((opcao) => (
          <button
            key={opcao}
            type="button"
            aria-pressed={janela === opcao}
            onClick={() => aoTrocarJanela(opcao)}
          >
            {opcao === "semana" ? "Semana" : "Mês"}
          </button>
        ))}
      </div>

      {linhas.length === 0 ? (
        <Vazio frase="Nenhuma hora nesta janela. O que você registrar no CronoCAD aparece aqui." />
      ) : (
        <>
          <p className="horas-total">
            <strong>{emHoras(totalSegundos)}</strong>
            <span>{emReais(totalValor)}</span>
          </p>
          <ul className="lista">
            {linhas.map((linha) => (
              <li className="item" key={linha.projeto}>
                <div className="item-corpo">
                  <p>{linha.projeto}</p>
                  <div
                    className="horas-barra"
                    style={{ width: `${Math.max(4, (linha.segundos / maior) * 100)}%` }}
                  />
                  <small>
                    {emHoras(linha.segundos)} · {emReais(linha.valorCents)} ·{" "}
                    {linha.lancamentos}
                    {linha.lancamentos === 1 ? " lançamento" : " lançamentos"}
                  </small>
                </div>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

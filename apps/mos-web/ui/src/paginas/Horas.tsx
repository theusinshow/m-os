import type { HorasDeProjeto } from "../api";
import { Esqueleto } from "../componentes/Esqueleto";
import { Vazio } from "../componentes/Vazio";
import { emHoras, emReais } from "./numeros";
import { JANELAS, pontaCurta, type Janela } from "./janelas";
import { paraCampoLocal } from "../instantes";

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
  periodo,
  aoTrocarJanela,
  aoEscolherPeriodo,
  carregando,
}: {
  linhas: HorasDeProjeto[];
  janela: Janela;
  /** As duas pontas em uso, para a tela poder DIZER o que está somando. */
  periodo: [Date, Date];
  aoTrocarJanela: (janela: Janela) => void;
  aoEscolherPeriodo: (de: Date, ate: Date) => void;
  carregando?: boolean;
}) {
  const maior = Math.max(...linhas.map((linha) => linha.segundos), 1);
  const totalSegundos = linhas.reduce((soma, linha) => soma + linha.segundos, 0);
  const totalValor = linhas.reduce((soma, linha) => soma + linha.valorCents, 0);

  return (
    <div className="horas">
      {/* Cinco janelas prontas e uma livre. As prontas cobrem o que se pergunta
          quase sempre — esta semana, a passada, o mês —, e a livre existe para
          o resto: fechar uma fatura de um período que não é nenhum deles. */}
      <div className="horas-janela">
        {JANELAS.map((opcao) => (
          <button
            key={opcao.chave}
            type="button"
            aria-pressed={janela === opcao.chave}
            onClick={() => aoTrocarJanela(opcao.chave)}
          >
            {opcao.rotulo}
          </button>
        ))}
        <button
          type="button"
          aria-pressed={janela === "personalizado"}
          onClick={() => aoTrocarJanela("personalizado")}
        >
          Período
        </button>
      </div>

      {janela === "personalizado" ? (
        <div className="horas-periodo">
          <label className="campo">
            <span>DE</span>
            <input
              type="date"
              value={paraCampoLocal(periodo[0]).slice(0, 10)}
              onChange={(evento) =>
                aoEscolherPeriodo(
                  new Date(`${evento.currentTarget.value}T00:00:00`),
                  periodo[1],
                )
              }
            />
          </label>
          <label className="campo">
            <span>ATÉ</span>
            <input
              type="date"
              value={paraCampoLocal(periodo[1]).slice(0, 10)}
              onChange={(evento) =>
                aoEscolherPeriodo(
                  periodo[0],
                  // Até o FIM do dia escolhido: `T00:00` cortaria fora tudo o
                  // que foi lançado no próprio dia que a pessoa pediu.
                  new Date(`${evento.currentTarget.value}T23:59:59`),
                )
              }
            />
          </label>
        </div>
      ) : (
        // A tela DIZ o que está somando. Sem isto, "15h38" é um número sem
        // pergunta — e duas janelas diferentes produzem o mesmo número com
        // significados diferentes.
        <p className="horas-faixa">
          {pontaCurta(periodo[0])} — {pontaCurta(periodo[1])}
        </p>
      )}

      {carregando && linhas.length === 0 ? (
        <Esqueleto />
      ) : linhas.length === 0 ? (
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

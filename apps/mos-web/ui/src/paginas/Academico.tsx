import type { CompromissoDaLista } from "../api";
import { Vazio } from "../componentes/Vazio";
import { daquiA } from "../instantes";

/**
 * O que vem por aí na faculdade.
 *
 * # A ordem não é cronológica, e isso é de propósito
 *
 * O atrasado vem primeiro. Ele já falhou, e enterrá-lo no meio da lista por data
 * o esconderia justamente de quem precisa agir — que é o oposto do que uma lista
 * de prazos existe para fazer.
 *
 * Depois vem o de hoje, e só então o que ainda vai acontecer.
 */
export function Academico({ compromissos }: { compromissos: CompromissoDaLista[] }) {
  if (compromissos.length === 0) {
    return (
      <Vazio frase="Nada por aqui. Provas e entregas cadastradas no M/OS aparecem nesta lista." />
    );
  }

  return (
    <ul className="lista">
      {compromissos.map((compromisso, indice) => (
        <li
          className="item"
          key={`${compromisso.titulo}-${compromisso.quando}-${indice}`}
          // O mesmo filete de sódio das outras listas, e pela mesma regra: só
          // marca o que cobra ação. Uma prova daqui a dez dias não cobra nada
          // hoje, e pintá-la ensinaria a ignorar a cor.
          data-cobra={
            compromisso.urgencia === "atrasado" || compromisso.urgencia === "hoje" || undefined
          }
        >
          <div className="item-corpo">
            <p>{compromisso.titulo}</p>
            <small>
              {compromisso.disciplina} · {compromisso.tipo === "exam" ? "prova" : "entrega"} ·{" "}
              {compromisso.urgencia === "atrasado"
                ? "atrasado"
                : daquiA(compromisso.quando)}
            </small>
          </div>
        </li>
      ))}
    </ul>
  );
}

import { pedeAtencao, type Capture, type Lembrete, type Task } from "./api";

export type Pagina =
  | "home"
  | "agenda"
  | "capturar"
  | "fazer"
  | "mais"
  | "horas"
  | "academico"
  | "lembretes";

/**
 * Os cinco lugares da barra de baixo.
 *
 * # O que mudou, e por que doia
 *
 * A barra antiga era `Home · Capturar · Inbox · Tasks · Mais`, e escondia a
 * Agenda dentro de "Mais" — quem nao soubesse que o calendario existe nunca
 * descobriria. Tres decisoes consertam isso:
 *
 * - **Capturar deixa de ser um destino igual aos outros** e vira o botao
 *   central. E a razao de existir do app; em pe de igualdade com "Mais" ele
 *   pedia a mesma mira que uma pagina de ajustes.
 * - **Inbox e Tasks fundem em FAZER.** Sao a mesma pergunta — *o que esta
 *   aberto?* — e ocupavam dois dos cinco lugares para responde-la duas vezes.
 * - **A Agenda sobe para a barra**, no lugar que sobrou.
 *
 * Lembretes continua fora, e a ausencia continua sendo a decisao: ele e destino
 * de notificacao — chega-se nele pelo aviso que tocou, ou pelo cartao da Home.
 */
export const DESTINOS: ReadonlyArray<{ pagina: Pagina; rotulo: string }> = [
  { pagina: "home", rotulo: "Home" },
  { pagina: "agenda", rotulo: "Agenda" },
  { pagina: "capturar", rotulo: "Capturar" },
  { pagina: "fazer", rotulo: "Fazer" },
  { pagina: "mais", rotulo: "Mais" },
];

/** Qual dos cinco e o botao central em sodio, e nao um alvo de texto. */
export const CENTRAL: Pagina = "capturar";

export type Dados = {
  capturas: Capture[];
  tasks: Task[];
  lembretes: Lembrete[];
};

/** O numero do badge. Zero significa "nao desenhe nada". */
export function contagemDe(pagina: Pagina, dados: Dados): number {
  switch (pagina) {
    // O badge de FAZER soma as duas metades da tela, porque a tela e uma so: o
    // que esta na barra tem que corresponder ao que se ve ao tocar nela.
    case "fazer":
      return (
        dados.capturas.length + dados.tasks.filter((task) => task.state !== "done").length
      );
    case "mais":
    case "lembretes":
      // So o que cobra acao. `scheduled` nao entra: um badge que sobe com coisa
      // que ainda nao e hora e um badge que se aprende a ignorar
      // (`ATTENTION-SYSTEM.md` §21.1).
      return dados.lembretes.filter(pedeAtencao).length;
    default:
      return 0;
  }
}

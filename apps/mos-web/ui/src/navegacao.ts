import { pedeAtencao, type Capture, type Lembrete, type Task } from "./api";

export type Pagina = "home" | "capturar" | "inbox" | "tasks" | "lembretes" | "mais";

/**
 * Os cinco destinos da barra de baixo.
 *
 * Lembretes NAO esta aqui, e a ausencia e a decisao: ele e destino de
 * notificacao — chega-se nele pelo aviso que tocou, ou pelo cartao da Home —, e
 * a barra guarda os cinco alvos que o polegar procura sem motivo externo.
 */
export const DESTINOS: ReadonlyArray<{ pagina: Pagina; rotulo: string }> = [
  { pagina: "home", rotulo: "Home" },
  { pagina: "capturar", rotulo: "Capturar" },
  { pagina: "inbox", rotulo: "Inbox" },
  { pagina: "tasks", rotulo: "Tasks" },
  { pagina: "mais", rotulo: "Mais" },
];

export type Dados = {
  capturas: Capture[];
  tasks: Task[];
  lembretes: Lembrete[];
};

/** O numero do badge. Zero significa "nao desenhe nada". */
export function contagemDe(pagina: Pagina, dados: Dados): number {
  switch (pagina) {
    case "inbox":
      return dados.capturas.length;
    case "tasks":
      return dados.tasks.filter((task) => task.state !== "done").length;
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

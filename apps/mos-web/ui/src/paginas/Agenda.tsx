import type { ItemDaAgenda } from "../api";
import { Vazio } from "../componentes/Vazio";
import { cobraAtencao, horaDoItem, porDia } from "./dias";

/** `1h30`, para o total do dia. */
function emHoras(segundos: number): string {
  const minutos = Math.round(segundos / 60);
  const horas = Math.floor(minutos / 60);
  return `${horas}h${String(minutos % 60).padStart(2, "0")}`;
}

/**
 * A palavra que diz o que aquele item É.
 *
 * O tipo cru (`exam_scheduled`) não serve para ler; e a cor sozinha também não,
 * porque ela não diz nada a quem não distingue âmbar de cinza. Tipo que este
 * binário não conhece vira string vazia — a linha continua legível pelo título,
 * que é o que importa.
 */
const PALAVRA: Record<string, string> = {
  session: "hora",
  task_done: "feita",
  task_created: "task",
  capture: "captura",
  day_started: "dia começou",
  day_ended: "dia encerrado",
  objective_done: "objetivo",
  assignment_due: "entrega",
  exam_scheduled: "prova",
  academic_planned: "planejado",
  meeting: "reunião",
};

/**
 * A agenda do bolso.
 *
 * # Por que ela mostra o passado junto com o futuro
 *
 * Um calendário que só olha para a frente responde "o que vem" e cala sobre "o
 * que eu fiz" — e no M/OS as duas perguntas moram na mesma linha do tempo. A
 * hora trabalhada ontem e a prova de quinta são fatos do mesmo dia útil.
 *
 * O que separa as duas não é a posição: é o marcador. Prova e prazo ganham a
 * barra em sódio; o rastro do que já passou fica em traço apagado.
 */
export function Agenda({ itens, agora }: { itens: ItemDaAgenda[]; agora: Date }) {
  const dias = porDia(itens, agora);

  if (dias.length === 0) {
    return (
      <Vazio frase="Nada nesta janela. O que você registrar — horas, capturas, provas — aparece aqui no dia em que cai." />
    );
  }

  return (
    <div className="agenda">
      {dias.map((dia) => (
        <section key={dia.data}>
          <header className="agenda-dia">
            <span className="rotulo">{dia.titulo}</span>
            {/* O total só aparece quando houve hora: um "0h00" em todo dia sem
                trabalho ensinaria a ignorar o número. */}
            {dia.segundos > 0 ? (
              <span className="agenda-total">{emHoras(dia.segundos)}</span>
            ) : null}
          </header>
          <ul className="lista">
            {dia.itens.map((item, indice) => (
              <li
                className="item"
                key={`${item.kind}-${item.at}-${indice}`}
                data-cobra={cobraAtencao(item) || undefined}
              >
                <span className="agenda-hora">{horaDoItem(item)}</span>
                <div className="item-corpo">
                  <p>{item.title}</p>
                  <small>
                    {PALAVRA[item.kind] ?? ""}
                    {item.seconds > 0 ? ` · ${emHoras(item.seconds)}` : ""}
                  </small>
                </div>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

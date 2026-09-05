import { useRef, useState } from "react";
import type { ItemDaAgenda } from "../api";
import { Vazio } from "../componentes/Vazio";
import {
  cobraAtencao,
  faixaDeDias,
  horaDoItem,
  nomeDoMes,
  porDia,
  porMes,
  siglaDoDia,
  type CelulaDoMes,
} from "./dias";
import { emHoras } from "./numeros";

export type VistaDaAgenda = "lista" | "mes";

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
 *
 * # Duas vistas, e não uma
 *
 * A lista responde *o que vem agora*; o mês responde *como está o mês*. São
 * perguntas diferentes o bastante para merecerem desenhos diferentes, e
 * próximas o bastante para não merecerem duas telas.
 */
export function Agenda({
  itens,
  agora,
  vista,
  aoTrocarVista,
}: {
  itens: ItemDaAgenda[];
  agora: Date;
  vista: VistaDaAgenda;
  aoTrocarVista: (vista: VistaDaAgenda) => void;
}) {
  return (
    <div className="agenda">
      <header className="agenda-topo">
        <h2>{vista === "mes" ? nomeDoMes(agora) : "Agenda"}</h2>
        <div className="agenda-vista">
          {(["lista", "mes"] as const).map((opcao) => (
            <button
              key={opcao}
              type="button"
              aria-pressed={vista === opcao}
              onClick={() => aoTrocarVista(opcao)}
            >
              {opcao === "lista" ? "LISTA" : "MÊS"}
            </button>
          ))}
        </div>
      </header>

      {vista === "mes" ? (
        <Mes itens={itens} agora={agora} />
      ) : (
        <Lista itens={itens} agora={agora} />
      )}
    </div>
  );
}

/**
 * A lista, com a faixa de dias no topo.
 *
 * A faixa não é decoração: ela responde *tem coisa na sexta?* sem rolar até
 * sexta, e tocar num dia leva a lista até ele. Nove dias porque é a janela que
 * o servidor entrega — de ontem a uma semana — e mostrar caixas para dias sem
 * dado seria prometer conteúdo que não existe.
 */
function Lista({ itens, agora }: { itens: ItemDaAgenda[]; agora: Date }) {
  const dias = porDia(itens, agora);
  const secoes = useRef(new Map<string, HTMLElement>());

  if (dias.length === 0) {
    return (
      <Vazio frase="Nada nesta janela. O que você registrar — horas, capturas, provas — aparece aqui no dia em que cai." />
    );
  }

  return (
    <>
      <div className="agenda-faixa">
        {faixaDeDias(agora, itens).map((celula) => (
          <button
            key={celula.data}
            type="button"
            data-hoje={celula.hoje || undefined}
            data-vazio={celula.registros === 0 || undefined}
            onClick={() =>
              secoes.current
                .get(celula.data)
                ?.scrollIntoView({ behavior: "smooth", block: "start" })
            }
          >
            <b>{siglaDoDia(new Date(`${celula.data}T12:00:00`))}</b>
            <b>{celula.numero}</b>
            <i data-cobra={celula.cobra || undefined} aria-hidden="true" />
          </button>
        ))}
      </div>

      {dias.map((dia) => (
        <section
          key={dia.data}
          ref={(no) => {
            if (no) secoes.current.set(dia.data, no);
            else secoes.current.delete(dia.data);
          }}
          // O que já passou fica em traço apagado: rastro não pede atenção.
          data-passado={dia.data < diaDe(agora) || undefined}
        >
          <header className="agenda-dia">
            <span className="rotulo">{dia.titulo}</span>
            {/* O total só aparece quando houve hora: um "0h00" em todo dia sem
                trabalho ensinaria a ignorar o número. */}
            {dia.segundos > 0 ? (
              <span className="agenda-total">{emHoras(dia.segundos)}</span>
            ) : null}
          </header>
          <Linhas itens={dia.itens} />
        </section>
      ))}
    </>
  );
}

/**
 * O mês em grade, e o dia escolhido embaixo.
 *
 * A grade responde de relance onde o mês está cheio e onde ele cobra; a lista
 * de baixo é o detalhe do dia em que o dedo tocou. Sem ela a grade seria um
 * mapa bonito que não leva a lugar nenhum.
 */
function Mes({ itens, agora }: { itens: ItemDaAgenda[]; agora: Date }) {
  const celulas = porMes(itens, agora, agora);
  const [escolhido, setEscolhido] = useState(diaDe(agora));
  const doDia = porDia(itens, agora).find((dia) => dia.data === escolhido);

  return (
    <>
      <div className="agenda-grade">
        {["S", "T", "Q", "Q", "S", "S", "D"].map((sigla, indice) => (
          <span className="agenda-cabecalho" key={indice}>
            {sigla}
          </span>
        ))}
        {celulas.map((celula, indice) => (
          <Celula
            key={celula.data || `folga-${indice}`}
            celula={celula}
            escolhido={celula.data === escolhido}
            aoEscolher={() => setEscolhido(celula.data)}
          />
        ))}
      </div>

      <div className="agenda-legenda">
        <span>
          <i aria-hidden="true" />
          REGISTRO
        </span>
        <span>
          <i data-cobra aria-hidden="true" />
          COBRA
        </span>
      </div>

      <section>
        <header className="agenda-dia">
          <span className="rotulo">{doDia?.titulo ?? tituloSolto(escolhido)}</span>
          {doDia && doDia.segundos > 0 ? (
            <span className="agenda-total">{emHoras(doDia.segundos)}</span>
          ) : null}
        </header>
        {doDia ? (
          <Linhas itens={doDia.itens} />
        ) : (
          <p className="agenda-nada">Nada neste dia.</p>
        )}
      </section>
    </>
  );
}

function Celula({
  celula,
  escolhido,
  aoEscolher,
}: {
  celula: CelulaDoMes;
  escolhido: boolean;
  aoEscolher: () => void;
}) {
  if (celula.numero === 0) return <span className="agenda-celula" data-folga="" />;
  return (
    <button
      type="button"
      className="agenda-celula"
      // Hoje e escolhido são estados diferentes: hoje é um fato, escolhido é uma
      // decisão. Quando coincidem, vence o anel de sódio do escolhido — senão a
      // grade ficaria com dois anéis dizendo a mesma coisa.
      data-hoje={(celula.hoje && !escolhido) || undefined}
      data-escolhido={escolhido || undefined}
      onClick={aoEscolher}
    >
      <b>{celula.numero}</b>
      <span className="agenda-pontos">
        {/* Até três pontos: acima disso a diferença entre quatro e nove deixa de
            ser lida e a célula vira uma mancha. */}
        {Array.from({ length: Math.min(celula.registros, 3) }).map((_, i) => (
          <i key={i} aria-hidden="true" />
        ))}
        {celula.cobra ? <i data-cobra aria-hidden="true" /> : null}
      </span>
    </button>
  );
}

function Linhas({ itens }: { itens: ItemDaAgenda[] }) {
  return (
    <ul className="lista">
      {itens.map((item, indice) => (
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
  );
}

/** `2026-09-05` no fuso local. */
function diaDe(quando: Date): string {
  const mes = String(quando.getMonth() + 1).padStart(2, "0");
  const dia = String(quando.getDate()).padStart(2, "0");
  return `${quando.getFullYear()}-${mes}-${dia}`;
}

/** O título de um dia que não tem nada: `SÁB · 12 DE SET`, sem passar pela
 *  lista — que só conhece dias com item dentro. */
function tituloSolto(data: string): string {
  const quando = new Date(`${data}T12:00:00`);
  const dias = ["DOM", "SEG", "TER", "QUA", "QUI", "SEX", "SÁB"];
  const meses = [
    "JAN", "FEV", "MAR", "ABR", "MAI", "JUN",
    "JUL", "AGO", "SET", "OUT", "NOV", "DEZ",
  ];
  return `${dias[quando.getDay()]} · ${quando.getDate()} DE ${meses[quando.getMonth()]}`;
}

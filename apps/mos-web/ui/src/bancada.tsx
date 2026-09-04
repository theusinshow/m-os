import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "./estilo.css";
import "./telas.css";
import { FALSO } from "./falso";
import { Barra } from "./componentes/Barra";
import { Marca } from "./componentes/Marca";
import type { Pagina } from "./navegacao";
import { Home } from "./paginas/Home";
import { Capturar } from "./paginas/Capturar";
import { Inbox } from "./paginas/Inbox";
import { Tasks } from "./paginas/Tasks";
import { Lembretes } from "./paginas/Lembretes";
import { Mais } from "./paginas/Mais";

/**
 * A bancada.
 *
 * O `mos-web` de verdade exige sessao — `/app/*` sem passkey cai na porta —,
 * entao fotografar o app real custa a cerimonia inteira antes de cada olhada. E
 * o que custa caro se faz pouco. Aqui as telas nascem com dado falso e o CSS de
 * verdade, lado a lado, nas duas larguras que importam.
 *
 * O que ela NAO prova: nada que dependa do servidor, e nada de comportamento —
 * os botoes daqui nao fazem nada. Isso continua sendo assunto dos testes e do
 * app de verdade no aparelho.
 */
const LARGURAS = [390, 430];

const NADA = () => {};

function Moldura({
  titulo,
  largura,
  pagina,
  compoe,
  children,
}: {
  titulo: string;
  largura: number;
  pagina: Pagina;
  compoe?: boolean;
  children: ReactNode;
}) {
  return (
    <figure className="bancada-moldura">
      <figcaption>
        {titulo} · {largura}px
      </figcaption>
      <div className="bancada-tela" style={{ width: largura, height: 760 }}>
        <div className="app">
          <header className="topo">
            <Marca tamanho={18} />
            <span className="marca">M/OS</span>
            <span className="fila" data-estado="fila">
              <i aria-hidden="true" />3 NA FILA
            </span>
          </header>
          <main className="conteudo">{children}</main>
          {compoe ? (
            <form className="compositor" onSubmit={(e) => e.preventDefault()}>
              <textarea placeholder="O que está na cabeça?" aria-label="Nova captura" />
              <div className="linha-de-botoes">
                <button className="botao" type="button">
                  Guardar
                </button>
              </div>
              <p className="recado" data-estado="ok" />
            </form>
          ) : null}
          <Barra atual={pagina} dados={FALSO} aoIr={NADA} />
        </div>
      </div>
    </figure>
  );
}

const TELAS: { titulo: string; pagina: Pagina; compoe?: boolean; corpo: ReactNode }[] = [
  {
    titulo: "Home",
    pagina: "home",
    corpo: (
      <Home
        estado={FALSO.estado}
        dados={FALSO}
        panorama={{
          horas: { semanaSegundos: 32_880, semanaValorCents: 27_400, hojeSegundos: 3_600 },
          proximos: [
            {
              titulo: "Prova de Cálculo III",
              disciplina: "Cálculo III",
              quando: "2026-09-06T14:00:00Z",
              tipo: "exam",
            },
          ],
        }}
        aoIr={NADA}
      />
    ),
  },
  {
    titulo: "Capturar",
    pagina: "capturar",
    compoe: true,
    corpo: <Capturar capturas={FALSO.capturas} />,
  },
  {
    titulo: "Inbox",
    pagina: "inbox",
    corpo: <Inbox capturas={FALSO.capturas} aoCapturar={NADA} />,
  },
  {
    titulo: "Inbox vazia",
    pagina: "inbox",
    corpo: <Inbox capturas={[]} aoCapturar={NADA} />,
  },
  {
    titulo: "Tasks",
    pagina: "tasks",
    compoe: true,
    corpo: (
      <Tasks
        tasks={FALSO.tasks}
        tasksLembradas={new Set(["t3"])}
        aoAlternar={NADA}
        aoLembrar={NADA}
      />
    ),
  },
  {
    titulo: "Lembretes",
    pagina: "lembretes",
    compoe: true,
    corpo: <Lembretes lembretes={FALSO.lembretes} ocupado={false} aoResolver={NADA} />,
  },
  {
    titulo: "Mais",
    pagina: "mais",
    corpo: (
      <Mais
        estado={FALSO.estado}
        avisos={{ estado: "pronto" }}
        ocupado={false}
        cobrando={1}
        aoAtivar={NADA}
        aoTestar={NADA}
        aoAbrirLembretes={NADA}
      />
    ),
  },
];

function Bancada() {
  return (
    <div className="bancada">
      {LARGURAS.map((largura) =>
        TELAS.map((tela) => (
          <Moldura
            key={`${tela.titulo}-${largura}`}
            titulo={tela.titulo}
            largura={largura}
            pagina={tela.pagina}
            compoe={tela.compoe}
          >
            {tela.corpo}
          </Moldura>
        )),
      )}
    </div>
  );
}

createRoot(document.getElementById("raiz")!).render(<Bancada />);

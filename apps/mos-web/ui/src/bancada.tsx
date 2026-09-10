import { useState, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import "./estilo.css";
import "./telas.css";
import { FALSO } from "./falso";
import { Barra } from "./componentes/Barra";
import { Marca } from "./componentes/Marca";
import type { Pagina } from "./navegacao";
import { Home } from "./paginas/Home";
import { ARRANJO_VAZIO, type Arranjo } from "./paginas/arranjo";
import { Capturar } from "./paginas/Capturar";
import { Fazer } from "./paginas/Fazer";
import { Task } from "./paginas/Task";
import { Lembrete } from "./paginas/Lembrete";
import { Lembretes } from "./paginas/Lembretes";
import { Mais } from "./paginas/Mais";
import { Agenda } from "./paginas/Agenda";
import { TUDO } from "./paginas/categorias";
import { Horas } from "./paginas/Horas";
import { Academico } from "./paginas/Academico";

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

const O_DIA = {
  status: "active" as const,
  objetivos: [
    { id: "o1", titulo: "Fechar o quantitativo de aço", status: "done" as const, prioridade: "high" },
    { id: "o2", titulo: "Visitar a obra da Ratones", status: "done" as const, prioridade: "normal" },
    { id: "o3", titulo: "Estudar para a prova de Cálculo", status: "pending" as const, prioridade: "high" },
  ],
  resolvidos: 2,
  feitasHoje: 3,
  sequencia: 6,
};

const PANORAMA = {
  horas: {
    semanaSegundos: 32_880,
    semanaValorCents: 27_400,
    hojeSegundos: 3_600,
    diasSegundos: [12_600, 23_400, 17_200, 32_880, 0, 0, 0],
  },
  proximos: [
    {
      titulo: "Prova de Cálculo III",
      disciplina: "Cálculo III",
      quando: "2026-09-06T14:00:00Z",
      tipo: "exam" as const,
    },
  ],
};

/**
 * A Home já dentro do modo arrumar.
 *
 * O modo mora no App, e não na Home, porque ele troca a barra do topo. Aqui a
 * bancada o liga direto — e por isso desenha também a barra de sódio à mão, que
 * no app de verdade é responsabilidade da casca.
 */
function HomeArrumando() {
  const [arranjo, setArranjo] = useState<Arranjo>({ ordem: [], ocultos: ["inbox"] });
  return (
    <Home
      estado={FALSO.estado}
      dados={FALSO}
      panorama={PANORAMA}
      dia={O_DIA}
      arranjo={arranjo}
      arrumando
      aoArrumando={NADA}
      aoArranjar={setArranjo}
      aoIr={NADA}
    />
  );
}

const NADA = () => {};

/** Um instante de hoje (ou de daqui a N dias) na hora pedida. */
function comHora(hora: number, minuto: number, daquiA = 0): string {
  const quando = new Date();
  quando.setDate(quando.getDate() + daquiA);
  quando.setHours(hora, minuto, 0, 0);
  return quando.toISOString();
}

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

/** Um mês espalhado, para a grade ter o que desenhar em mais de uma semana. */
const AGENDA_DO_MES = [-9, -8, -8, -5, -3, -3, -3, -1, 0, 0, 2, 4, 9, 11].map((quantos, i) => ({
  kind: i % 5 === 0 ? "exam_scheduled" : i % 3 === 0 ? "capture" : "session",
  at: comHora(9 + (i % 8), 0, quantos),
  endsAt: null,
  title: ["046 - Ratones · desenho", "Prova de Cálculo III", "Conferir o radier"][i % 3],
  projectId: null,
  seconds: i % 3 === 0 ? 0 : 3_600 * (1 + (i % 3)),
  amountCents: 0,
}));

const TELAS: { titulo: string; pagina: Pagina; compoe?: boolean; corpo: ReactNode }[] = [
  {
    titulo: "Home",
    pagina: "home",
    corpo: (
      <Home
        estado={FALSO.estado}
        dados={FALSO}
        panorama={PANORAMA}
        dia={O_DIA}
        arranjo={ARRANJO_VAZIO}
        arrumando={false}
        aoArrumando={NADA}
        aoArranjar={NADA}
        aoIr={NADA}
      />
    ),
  },
  {
    titulo: "Home arrumando",
    pagina: "home",
    corpo: <HomeArrumando />,
  },
  {
    titulo: "Capturar",
    pagina: "capturar",
    compoe: true,
    corpo: <Capturar capturas={FALSO.capturas} />,
  },
  {
    titulo: "Fazer",
    pagina: "fazer",
    compoe: true,
    corpo: (
      <Fazer
        capturas={[
          {
            id: "c0",
            content: "tabela de aço CA-50 https://exemplo.com/ca50 boa pra consultar",
            capturedAt: new Date(Date.now() - 900_000).toISOString(),
          },
          ...FALSO.capturas,
        ]}
        tasks={FALSO.tasks}
        tasksLembradas={new Set(["t3"])}
        aoCapturar={NADA}
        aoAbrir={NADA}
        aoAlternar={NADA}
        aoLembrar={NADA}
        aoTriar={NADA}
      />
    ),
  },
  {
    titulo: "Fazer vazio",
    pagina: "fazer",
    corpo: (
      <Fazer
        capturas={[]}
        tasks={FALSO.tasks}
        tasksLembradas={new Set(["t1"])}
        aoCapturar={NADA}
        aoAbrir={NADA}
        aoAlternar={NADA}
        aoLembrar={NADA}
        aoTriar={NADA}
      />
    ),
  },
  {
    titulo: "Task · detalhe",
    pagina: "fazer",
    corpo: (
      <Task
        task={FALSO.tasks[0]}
        detalhe={{
          task: FALSO.tasks[0],
          /* Cinco passos com dois feitos: é a proporção que mostra ao mesmo
             tempo o texto riscado, o texto vivo e a barra a meio caminho. Uma
             lista toda marcada ou toda aberta esconderia dois dos três. */
          checklist: [
            { id: "i1", taskId: "t1", label: "Conferir o nível do radier", position: 0, completedAt: "2026-09-08T12:00:00Z" },
            { id: "i2", taskId: "t1", label: "Atualizar o corte AA", position: 1, completedAt: "2026-09-08T13:00:00Z" },
            { id: "i3", taskId: "t1", label: "Revisar a armadura da caixa 01, incluindo o cobrimento de 5 cm combinado na reunião", position: 2, completedAt: null },
            { id: "i4", taskId: "t1", label: "Gerar o PDF", position: 3, completedAt: null },
            { id: "i5", taskId: "t1", label: "Enviar para o Victor", position: 4, completedAt: null },
          ],
          subtasks: [FALSO.tasks[1]],
          blockedBy: null,
          references: [],
          reminders: [],
        }}
        projeto={{ id: "p1", name: "046 - Ratones" }}
        projetos={[
          { id: "p1", name: "046 - Ratones" },
          { id: "p2", name: "JABOTICATUBA" },
        ]}
        lembrete={null}
        ocupado={false}
        aoSalvar={NADA}
        aoMudarEstado={NADA}
        aoArquivar={NADA}
        aoLembrar={NADA}
        aoVoltar={NADA}
        aoMarcarItem={NADA}
        aoCriarItem={NADA}
        aoApagarItem={NADA}
      />
    ),
  },
  {
    titulo: "Lembretes",
    pagina: "lembretes",
    compoe: true,
    corpo: (
      <Lembretes
        lembretes={FALSO.lembretes}
        resolvidos={[]}
        vista="abertos"
        ocupado={false}
        aoTrocarVista={NADA}
        aoAbrir={NADA}
        aoResolver={NADA}
      />
    ),
  },
  {
    titulo: "Lembrete · detalhe",
    pagina: "lembretes",
    corpo: (
      <Lembrete
        lembrete={FALSO.lembretes[0]}
        ocupado={false}
        aoSalvar={NADA}
        aoConcluir={NADA}
        aoCancelar={NADA}
        aoAdiar={NADA}
        aoArquivar={NADA}
        aoVoltar={NADA}
      />
    ),
  },
  {
    titulo: "Agenda",
    pagina: "agenda",
    corpo: (
      <Agenda
        vista="lista"
        filtro={TUDO}
        aoTrocarVista={NADA}
        aoFiltrar={NADA}
        agora={new Date()}
        itens={[
          {
            kind: "day_started",
            at: comHora(8, 30),
            endsAt: null,
            title: "fechar o Rancho Queimado",
            projectId: null,
            seconds: 0,
            amountCents: 0,
          },
          {
            kind: "session",
            at: comHora(9, 15),
            endsAt: null,
            title: "046 - Ratones · desenho",
            projectId: "p1",
            seconds: 7_200,
            amountCents: 6_000,
          },
          {
            kind: "capture",
            at: comHora(11, 40),
            endsAt: null,
            title: "Ligar para o cliente sobre a prancha 04",
            projectId: null,
            seconds: 0,
            amountCents: 0,
          },
          {
            kind: "exam_scheduled",
            at: comHora(14, 0, 1),
            endsAt: null,
            title: "Prova de Cálculo III",
            projectId: null,
            seconds: 0,
            amountCents: 0,
          },
          {
            kind: "assignment_due",
            at: comHora(0, 0, 2),
            endsAt: null,
            title: "Lista 04 · Estática dos Corpos",
            projectId: null,
            seconds: 0,
            amountCents: 0,
          },
        ]}
      />
    ),
  },
  {
    titulo: "Agenda · mês",
    pagina: "agenda",
    corpo: (
      <Agenda
        vista="mes"
        filtro={TUDO}
        aoTrocarVista={NADA}
        aoFiltrar={NADA}
        agora={new Date()}
        itens={AGENDA_DO_MES}
      />
    ),
  },
  {
    titulo: "Horas",
    pagina: "horas",
    corpo: (
      <Horas
        janela="semana"
        periodo={[new Date(2026, 8, 1), new Date()]}
        aoTrocarJanela={NADA}
        aoEscolherPeriodo={NADA}
        linhas={[
          { projeto: "046 - Ratones", segundos: 32_880, valorCents: 27_400, lancamentos: 8 },
          { projeto: "JABOTICATUBA", segundos: 18_000, valorCents: 15_000, lancamentos: 5 },
          { projeto: "043 - Rancho Queimado", segundos: 5_400, valorCents: 4_500, lancamentos: 1 },
        ]}
      />
    ),
  },
  {
    titulo: "Acadêmico",
    pagina: "academico",
    corpo: (
      <Academico
        compromissos={[
          {
            titulo: "Lista 03 · Estática dos Corpos",
            disciplina: "Estática dos Corpos",
            quando: comHora(23, 59, -2),
            tipo: "assignment",
            urgencia: "atrasado",
          },
          {
            titulo: "Relatório de ensaio",
            disciplina: "Materiais",
            quando: comHora(18, 0),
            tipo: "assignment",
            urgencia: "hoje",
          },
          {
            titulo: "Prova de Cálculo III",
            disciplina: "Cálculo III",
            quando: comHora(14, 0, 3),
            tipo: "exam",
            urgencia: "",
          },
        ]}
      />
    ),
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
        aoAbrirHoras={NADA}
        aoAbrirAcademico={NADA}
        aoAbrirBiblioteca={NADA}
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

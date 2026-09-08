import { createRoot } from "react-dom/client";
import { useState, type ReactElement } from "react";

import "@fontsource/schibsted-grotesk/400.css";
import "@fontsource/schibsted-grotesk/500.css";
import "@fontsource/jetbrains-mono/500.css";
import "../../../packages/design-system/tokens.css";
import "./App.css";

import { Checklist, ProgressoDoChecklist } from "./Checklist";
import { prazoCurto, estimativaCurta, ROTULO_DE_PRIORIDADE, situacaoDoPrazo } from "./tasks";
import type { ChecklistItem, Task } from "./types";

/**
 * A bancada da Task: todos os estados numa página, com os componentes reais.
 *
 * # Por que ela existe
 *
 * A lista de estados que precisam ser conferidos é longa — quadro vazio, quadro
 * cheio, Task sem checklist, com um item, com vinte, título longo, nome de
 * Project longo, prioridade alta, prazo vencido, Task concluída — e chegar a
 * cada um deles no app de verdade exige montar o dado antes. Aqui eles existem
 * todos ao mesmo tempo, e um olhar varre a coluna inteira.
 *
 * # O que ela NÃO prova
 *
 * Comportamento. Nada aqui fala com o Rust: as escritas são de mentira. O que
 * ela responde é FORMA — o que quebra, o que transborda, o que some no tema
 * claro. Comportamento continua sendo a foto da janela de verdade.
 */

const AGORA = new Date();
const daqui = (horas: number) => new Date(AGORA.getTime() + horas * 3_600_000).toISOString();

function task(mudanca: Partial<Task> = {}): Task {
  return {
    id: Math.random().toString(36).slice(2),
    title: "Ajustes reunião — Caixa 01",
    description: "",
    projectId: "p1",
    sourceCaptureId: null,
    state: "doing",
    lifecycleState: "active",
    dueAt: null,
    priority: "normal",
    estimateMinutes: null,
    parentTaskId: null,
    blockedByTaskId: null,
    waitingFor: "",
    followUpAt: null,
    checklistTotal: 0,
    checklistDone: 0,
    createdAt: AGORA.toISOString(),
    updatedAt: AGORA.toISOString(),
    completedAt: null,
    ...mudanca,
  };
}

function itens(quantos: number, feitos: number): ChecklistItem[] {
  const textos = [
    "Corrigir nível",
    "Revisar cobrimento",
    "Atualizar corte",
    "Atualizar prancha",
    "Gerar PDF",
    "Enviar para o Victor",
  ];
  return Array.from({ length: quantos }, (_, indice) => ({
    id: `i${indice}`,
    taskId: "t",
    label: textos[indice % textos.length] + (indice >= textos.length ? ` (${indice})` : ""),
    position: indice,
    completedAt: indice < feitos ? AGORA.toISOString() : null,
    createdAt: AGORA.toISOString(),
    updatedAt: AGORA.toISOString(),
  }));
}

/** Uma cópia do cartão do quadro, com os mesmos nomes de classe do `TaskCard`. */
function Cartao({ item, projeto, dias = 0 }: { item: Task; projeto?: string; dias?: number }) {
  /* `?expandido=1` abre as linguetas para o headless fotografar o checklist do
     cartão sem precisar clicar. */
  const [aberto, setAberto] = useState(new URLSearchParams(location.search).has("expandido"));
  const situacao = situacaoDoPrazo(item, AGORA);
  const estimativa = estimativaCurta(item.estimateMinutes);
  return (
    <article
      className="task-card"
      data-completed={item.state === "done" || undefined}
      data-stale={dias > 0 || undefined}
      data-prioridade={item.priority !== "normal" ? item.priority : undefined}
    >
      <button type="button" className="task-card-corpo">
        <strong>{item.title}</strong>
        {item.checklistTotal ? (
          <ProgressoDoChecklist feitos={item.checklistDone} total={item.checklistTotal} />
        ) : null}
        {projeto || item.dueAt || estimativa || dias > 0 || item.waitingFor ? (
          <span className="task-card-meta">
            {projeto ? <span className="task-card-projeto">{projeto}</span> : null}
            {item.dueAt ? (
              <span className="task-card-prazo" data-situacao={situacao}>
                {prazoCurto(item.dueAt, AGORA)}
              </span>
            ) : null}
            {estimativa ? <span>{estimativa}</span> : null}
            {item.waitingFor ? <span className="task-card-aguardando">aguardando {item.waitingFor}</span> : null}
            {dias > 0 ? <span className="task-card-parado">{dias} DIAS</span> : null}
          </span>
        ) : null}
        {item.priority !== "normal" ? (
          <span className="task-card-prioridade">{ROTULO_DE_PRIORIDADE[item.priority]}</span>
        ) : null}
      </button>
      {item.checklistTotal ? (
        <button type="button" className="task-card-lingueta" aria-expanded={aberto} onClick={() => setAberto(!aberto)}>
          <span aria-hidden="true">{aberto ? "▾" : "▸"}</span>
          <span>Checklist</span>
        </button>
      ) : null}
      {aberto ? (
        <div className="task-card-checklist">
          <Checklist taskId="t" itens={itens(item.checklistTotal, item.checklistDone)} compacto aoMudar={() => {}} />
        </div>
      ) : null}
    </article>
  );
}

const COLUNAS: { estado: string; rotulo: string; cartoes: ReactElement[] }[] = [
  {
    estado: "inbox",
    rotulo: "Inbox",
    cartoes: [],
  },
  {
    estado: "backlog",
    rotulo: "Backlog",
    cartoes: [
      <Cartao key="a" item={task({ title: "Comprar cabo HDMI", state: "backlog", projectId: null })} />,
      <Cartao
        key="b"
        item={task({
          title:
            "Compatibilizar a estrutura do bloco C com o projeto hidrossanitário revisado depois da reunião de quinta",
          state: "backlog",
          checklistTotal: 20,
          checklistDone: 7,
          priority: "high",
          estimateMinutes: 240,
        })}
        projeto="167-25 — Residencial Rancho Queimado, bloco C"
      />,
    ],
  },
  {
    estado: "doing",
    rotulo: "Doing",
    cartoes: [
      <Cartao
        key="c"
        item={task({ checklistTotal: 7, checklistDone: 4, dueAt: daqui(5), estimateMinutes: 30 })}
        projeto="167-25"
      />,
      <Cartao
        key="d"
        item={task({
          title: "Gerar o PDF do memorial",
          checklistTotal: 3,
          checklistDone: 3,
          dueAt: daqui(-30),
          priority: "urgent",
        })}
        projeto="043"
        dias={9}
      />,
      <Cartao
        key="e"
        item={task({ title: "Revisar a planta do quiosque", waitingFor: "Victor", checklistTotal: 1, checklistDone: 0 })}
      />,
    ],
  },
  {
    estado: "done",
    rotulo: "Done",
    cartoes: [
      <Cartao
        key="f"
        item={task({
          title: "Mandar a fatura de agosto",
          state: "done",
          checklistTotal: 4,
          checklistDone: 4,
          completedAt: AGORA.toISOString(),
          dueAt: daqui(-200),
        })}
        projeto="043"
      />,
    ],
  },
];

function Bancada() {
  /* O tema entra pela URL para o headless conseguir fotografar os dois sem
     clicar: `?tema=light`. O botão continua existindo para quem abre à mão. */
  const inicial = new URLSearchParams(location.search).get("tema") === "light" ? "light" : "dark";
  const [tema, setTema] = useState<"dark" | "light">(inicial);
  return (
    <div data-theme={tema} style={{ minHeight: "100vh", background: "var(--canvas)" }}>
      <div style={{ padding: "20px 32px", display: "flex", gap: 12, alignItems: "center" }}>
        <span className="micro-label" style={{ color: "var(--text-system)" }}>
          BANCADA DA TASK
        </span>
        <button className="button secondary sm" type="button" onClick={() => setTema(tema === "dark" ? "light" : "dark")}>
          {tema === "dark" ? "Tema claro" : "Tema escuro"}
        </button>
      </div>

      <div className="page board-page">
        <div className="kanban" aria-label="Kanban de Tasks">
          {COLUNAS.map((coluna) => (
            <section key={coluna.estado} className="kanban-column" data-kanban-state={coluna.estado}>
              <header>
                <h2>{coluna.rotulo}</h2>
                <span>{coluna.cartoes.length}</span>
              </header>
              {coluna.cartoes.length ? coluna.cartoes : <p className="kanban-empty">Vazio</p>}
            </section>
          ))}
        </div>
      </div>

      {/* A gaveta, fora da posição fixa dela, para caber na mesma página. */}
      <div style={{ padding: "32px", display: "flex", gap: 32, flexWrap: "wrap" }}>
        <Folha titulo="COM CHECKLIST" itensDoTeste={itens(6, 3)} />
        <Folha titulo="VAZIA" itensDoTeste={[]} />
        <Folha titulo="UM ITEM SÓ, TEXTO LONGO" itensDoTeste={[
          {
            id: "x",
            taskId: "t",
            label:
              "Revisar a armadura da caixa 01 incluindo o cobrimento de 5 cm que ficou combinado na reunião de quinta com o Victor",
            position: 0,
            completedAt: null,
            createdAt: AGORA.toISOString(),
            updatedAt: AGORA.toISOString(),
          },
        ]} />
      </div>
    </div>
  );
}

function Folha({ titulo, itensDoTeste }: { titulo: string; itensDoTeste: ChecklistItem[] }) {
  return (
    <section
      style={{
        width: 380,
        padding: "20px 32px 52px",
        background: "var(--surface-raised)",
        border: "1px solid var(--border-strong)",
      }}
    >
      <span className="micro-label" style={{ color: "var(--text-system)" }}>
        {titulo}
      </span>
      <div className="task-titulo">
        <input defaultValue="Ajustes reunião — Caixa 01" aria-label="Título" />
      </div>
      <div className="task-estados" role="group">
        {["Inbox", "Backlog", "Planned", "Doing", "Review", "Done"].map((rotulo) => (
          <button key={rotulo} type="button" aria-pressed={rotulo === "Doing"}>
            {rotulo}
          </button>
        ))}
      </div>
      <Checklist taskId="t" itens={itensDoTeste} aoMudar={() => {}} />
      {itensDoTeste.length && itensDoTeste.every((item) => item.completedAt) ? (
        <div className="task-tudo-feito">
          <span>Todos os itens concluídos.</span>
          <button className="button primary sm" type="button">
            Concluir Task
          </button>
        </div>
      ) : null}
      <section className="task-bloco">
        <span className="micro-label">NOTAS</span>
        <textarea className="task-nota" rows={3} defaultValue="Conforme alinhado na reunião, manter cobrimento de 5 cm." />
      </section>
      <dl className="task-campos">
        <div>
          <dt>Project</dt>
          <dd>
            <select defaultValue="">
              <option value="">167-25</option>
            </select>
          </dd>
        </div>
        <div>
          <dt>Prazo</dt>
          <dd className="task-prazo" data-situacao="hoje">
            <input type="datetime-local" aria-label="Prazo" />
            <span className="task-prazo-lido">Hoje 17:00</span>
          </dd>
        </div>
        <div>
          <dt>Prioridade</dt>
          <dd>
            <div className="task-prioridade" role="group">
              {(["low", "normal", "high", "urgent"] as const).map((nivel) => (
                <button key={nivel} type="button" data-nivel={nivel} aria-pressed={nivel === "high"}>
                  {ROTULO_DE_PRIORIDADE[nivel]}
                </button>
              ))}
            </div>
          </dd>
        </div>
        <div>
          <dt>Lembrete</dt>
          <dd className="task-lembretes">
            <span>Hoje 16:30</span>
            <button className="button ghost sm" type="button">
              Outro
            </button>
          </dd>
        </div>
      </dl>
      <button type="button" className="task-mais" aria-expanded={false}>
        Mais opções
      </button>
    </section>
  );
}

createRoot(document.getElementById("raiz") as HTMLElement).render(<Bancada />);


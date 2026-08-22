/**
 * O M/Academic.
 *
 * Duas telas num arquivo: o painel do semestre e o cockpit de uma disciplina. A
 * navegação entre elas é estado local, e não rota — é a mesma escolha da Library
 * e do CronoCAD, e o que ela compra é o botão "voltar" continuar significando
 * "sair do Academic" em vez de "desfazer meu último clique".
 *
 * **A decisão vem pronta.** O que é "chegando", como a média pondera peso e
 * escala, o que é atraso e qual o semestre corrente vivem em
 * `mos-core::academic`. A apresentação testável — faixas, frases de data,
 * duração, cronômetro — vive em `academic.ts`. O que sobra aqui é desenho.
 */
import { FormEvent, useCallback, useEffect, useRef, useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { ActionMenu, ContextPath, EmptyState, PageHeader, Panel } from "./Surface";
import {
  avaliadoDe,
  campoDoInstante,
  cronometroDe,
  decorridoDe,
  duracaoDe,
  instanteDoCampo,
  mediaDe,
  planoDe,
  quandoDe,
  situacaoDe,
  STATUS_ATIVIDADE,
  STATUS_AVALIACAO,
  STATUS_SEMESTRE,
} from "./academic";
import type {
  AcademicDashboard,
  Assignment,
  AssignmentStatus,
  Compromisso,
  Exam,
  ExamStatus,
  ReminderPriority,
  Resource,
  Semester,
  StudySession,
  Subject,
  Decision,
  ProviderSubjectFact,
  SubjectOverview,
} from "./types";

const ACCENTS = ["", "trigo", "cobre", "musgo", "lilas", "argila", "ceu"] as const;

/** O ponto colorido da disciplina. `aria-hidden`: a cor não carrega informação
 *  que o texto ao lado já não diga. */
function SubjectDot({ accent }: { accent: string }) {
  return <span className="academic-dot" data-accent={accent || undefined} aria-hidden="true" />;
}

// ===========================================================================
// Painel
// ===========================================================================

export function AcademicPage({ refresh }: { refresh: () => Promise<void> }) {
  const [dashboard, setDashboard] = useState<AcademicDashboard | null>(null);
  const [semesters, setSemesters] = useState<Semester[]>([]);
  const [subjects, setSubjects] = useState<Subject[]>([]);
  const [facts, setFacts] = useState<Record<string, ProviderSubjectFact>>({});
  const [erro, setErro] = useState("");
  const [aberta, setAberta] = useState<string | null>(null);
  const [criandoSemestre, setCriandoSemestre] = useState(false);
  const [criandoDisciplina, setCriandoDisciplina] = useState(false);
  // O recibo das decisoes, com Undo. Toast e nao dialogo: a decisao e
  // reversivel, e confirmar cada uma transformaria a operacao mais comum da
  // tela em dois cliques.
  const [recibo, setRecibo] = useState<{ texto: string; desfazer?: () => Promise<void> } | null>(null);

  const carregar = useCallback(async () => {
    try {
      const [painel, periodos, materias, fatos] = await Promise.all([
        api.academicDashboard(),
        api.academicSemesters(true),
        api.academicSubjects(false),
        // A media oficial da instituicao. Falha em silencio: quem nunca
        // conectou um AVA nao pode perder o painel por causa disso.
        api.univirtusSubjectFacts().catch(() => [] as ProviderSubjectFact[]),
      ]);
      setDashboard(painel);
      setSemesters(periodos);
      setSubjects(materias);
      setFacts(Object.fromEntries(fatos.map((f) => [f.subjectId, f])));
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }, []);

  useEffect(() => {
    void carregar();
  }, [carregar]);

  const avisar = useCallback((texto: string, desfazer?: () => Promise<void>) => {
    setRecibo({ texto, desfazer });
  }, []);

  const recarregar = useCallback(async () => {
    await carregar();
    await refresh();
  }, [carregar, refresh]);

  if (aberta) {
    return (
      <SubjectPage
        subjectId={aberta}
        voltar={() => setAberta(null)}
        recarregar={recarregar}
      />
    );
  }

  const semestre = dashboard?.semester ?? null;

  return (
    <div className="page academic-page">
      <ContextPath segments={["M", "ACADEMIC"]} />
      <PageHeader
        title="Academic"
        subtitle={
          semestre
            ? `${semestre.name}${semestre.institution ? ` · ${semestre.institution}` : ""} · ${
                STATUS_SEMESTRE[dashboard?.semesterStatus ?? "active"]
              }`
            : "A faculdade dentro do M/OS."
        }
        actions={
          // Sem semestre a acao mora no empty state, e mora la sozinha: o header
          // repetindo "Novo semestre" em primario punha dois botoes de sodio na
          // mesma tela abrindo o mesmo formulario, e a primeira tela que alguem
          // ve e justamente essa.
          semestre ? (
            <>
              <Button onClick={() => setCriandoDisciplina(true)}>Nova disciplina</Button>
              <Button variant="ghost" onClick={() => setCriandoSemestre(true)}>
                Novo semestre
              </Button>
            </>
          ) : undefined
        }
      />

      {erro ? <p className="form-error">{erro}</p> : null}

      {recibo ? (
        <div className="academic-recibo" role="status" aria-live="polite">
          <span>{recibo.texto}</span>
          {recibo.desfazer ? (
            <Button
              variant="ghost"
              onClick={() => {
                const desfazer = recibo.desfazer;
                setRecibo(null);
                if (desfazer) void desfazer();
              }}
            >
              Desfazer
            </Button>
          ) : null}
          <Button variant="ghost" onClick={() => setRecibo(null)}>
            Fechar
          </Button>
        </div>
      ) : null}

      {criandoSemestre ? (
        <SemesterForm
          fechar={() => setCriandoSemestre(false)}
          salvo={async () => {
            setCriandoSemestre(false);
            await recarregar();
          }}
        />
      ) : null}

      {criandoDisciplina && semestre ? (
        <SubjectForm
          semesterId={semestre.id}
          fechar={() => setCriandoDisciplina(false)}
          salvo={async () => {
            setCriandoDisciplina(false);
            await recarregar();
          }}
        />
      ) : null}

      {!semestre && !criandoSemestre ? (
        <section className="academic-empty">
          <span className="micro-label">NENHUM SEMESTRE</span>
          <h1>Comece pelo período.</h1>
          <p>
            Um semestre guarda as disciplinas, e é ele que decide o que o M/OS mostra como
            &ldquo;agora&rdquo;. Depois dele vêm as matérias, as provas e as entregas.
          </p>
          <Button variant="primary" onClick={() => setCriandoSemestre(true)}>
            Criar semestre
          </Button>
        </section>
      ) : null}

      {semestre && dashboard ? (
        <>
          {/* A hierarquia e a resposta a "o que exige minha atencao agora":
              decisao primeiro, prazo depois, estudo e disciplinas em seguida, e
              historico por ultimo — fechado, para nao competir com urgencia. */}
          <AtencaoPainel
            dashboard={dashboard}
            abrir={setAberta}
            recarregar={recarregar}
            avisar={avisar}
          />
          <FaixaPainel
            label="ESTA SEMANA"
            itens={dashboard.thisWeek}
            vazio="Nenhum prazo nos próximos sete dias."
            abrir={setAberta}
            recarregar={recarregar}
            avisar={avisar}
          />
          <FaixaPainel
            label="DEPOIS"
            itens={dashboard.later}
            vazio="Nada marcado além desta semana."
            abrir={setAberta}
            recarregar={recarregar}
            avisar={avisar}
            fechada
          />
          <EstudoPainel dashboard={dashboard} subjects={subjects} recarregar={recarregar} />
          <DisciplinasPainel dashboard={dashboard} abrir={setAberta} facts={facts} />
          <HistoricoPainel
            dashboard={dashboard}
            abrir={setAberta}
            recarregar={recarregar}
            avisar={avisar}
          />
          <SemestresPainel
            semesters={semesters}
            atual={semestre.id}
            recarregar={recarregar}
          />
        </>
      ) : null}
    </div>
  );
}

/** O QUE PRECISA DE MIM — a primeira viewport, e a razão de a tela existir.
 *
 *  Ela não mostra "o que existe na faculdade": mostra o que ainda pede uma
 *  decisão. O que foi entregue, descartado ou é resto de calendário antigo cai
 *  no Histórico, e o critério vive em `mos_core::academic_decision` — derivado,
 *  nunca gravado, porque "precisa de atenção" muda sozinho toda madrugada. */
function AtencaoPainel({
  dashboard,
  abrir,
  recarregar,
  avisar,
}: {
  dashboard: AcademicDashboard;
  abrir: (id: string) => void;
  recarregar: () => Promise<void>;
  avisar: (texto: string, desfazer?: () => Promise<void>) => void;
}) {
  const itens = dashboard.needsAttention;
  return (
    <Panel
      label="PRECISA DE MIM"
      value={String(itens.length)}
      unit={itens.length === 1 ? "compromisso" : "compromissos"}
      action={
        dashboard.overdue ? (
          <span className="academic-alerta">
            {dashboard.overdue} {dashboard.overdue === 1 ? "atrasado" : "atrasados"}
          </span>
        ) : undefined
      }
    >
      {itens.length ? (
        <ul className="academic-lista">
          {itens.map((item) => (
            <CompromissoRow
              key={`${item.kind}-${item.id}`}
              item={item}
              abrir={abrir}
              recarregar={recarregar}
              avisar={avisar}
              destaque
            />
          ))}
        </ul>
      ) : (
        <EmptyState>
          Nada pede decisão agora.
          {dashboard.thisWeek.length
            ? ` ${dashboard.thisWeek.length} ${dashboard.thisWeek.length === 1 ? "compromisso" : "compromissos"} esta semana, logo abaixo.`
            : ""}
        </EmptyState>
      )}
    </Panel>
  );
}

/** Uma faixa secundária: esta semana, ou depois. */
function FaixaPainel({
  label,
  itens,
  vazio,
  abrir,
  recarregar,
  avisar,
  fechada = false,
}: {
  label: string;
  itens: Compromisso[];
  vazio: string;
  abrir: (id: string) => void;
  recarregar: () => Promise<void>;
  avisar: (texto: string, desfazer?: () => Promise<void>) => void;
  fechada?: boolean;
}) {
  return (
    <Panel label={label} value={String(itens.length)} unit={itens.length === 1 ? "item" : "itens"}>
      {itens.length ? (
        <details className="disclosure" open={!fechada}>
          <summary>{itens.length === 1 ? "1 compromisso" : `${itens.length} compromissos`}</summary>
          <ul className="academic-lista">
            {itens.map((item) => (
              <CompromissoRow
                key={`${item.kind}-${item.id}`}
                item={item}
                abrir={abrir}
                recarregar={recarregar}
                avisar={avisar}
              />
            ))}
          </ul>
        </details>
      ) : (
        <EmptyState>{vazio}</EmptyState>
      )}
    </Panel>
  );
}

/** Uma linha de compromisso, com as ações que a tornam operacional.
 *
 *  Uma ação primária visível — **Planejar** — e o resto num menu. Sete botões
 *  expostos por linha transformariam uma lista de doze itens em oitenta e quatro
 *  alvos de clique, e a pessoa pararia de ler os títulos. */
function CompromissoRow({
  item,
  abrir,
  recarregar,
  avisar,
  destaque = false,
}: {
  item: Compromisso;
  abrir: (id: string) => void;
  recarregar: () => Promise<void>;
  avisar: (texto: string, desfazer?: () => Promise<void>) => void;
  destaque?: boolean;
}) {
  const [ocupado, setOcupado] = useState(false);
  const [planejando, setPlanejando] = useState(false);
  const ehAtividade = item.kind === "assignment";

  async function planejar(quando: string | null, minutos: number) {
    setOcupado(true);
    try {
      if (ehAtividade) await api.academicPlanAssignment(item.id, quando, minutos);
      else await api.academicPlanExam(item.id, quando, minutos);
      setPlanejando(false);
      await recarregar();
      avisar(quando ? "Planejado." : "Plano desfeito.");
    } catch (erro) {
      avisar(appError(erro).message);
    } finally {
      setOcupado(false);
    }
  }

  async function decidir(decision: Decision, frase: string) {
    setOcupado(true);
    const anterior = item.decision;
    try {
      if (ehAtividade) await api.academicSetAssignmentDecision(item.id, decision);
      else await api.academicSetExamDecision(item.id, decision);
      await recarregar();
      // Undo em vez de confirmação: a decisão é reversível, e perguntar "tem
      // certeza?" a cada item transformaria a operação mais comum da tela em
      // dois cliques.
      avisar(frase, async () => {
        if (ehAtividade) await api.academicSetAssignmentDecision(item.id, anterior);
        else await api.academicSetExamDecision(item.id, anterior);
        await recarregar();
      });
    } catch (erro) {
      avisar(appError(erro).message);
    } finally {
      setOcupado(false);
    }
  }

  const acoes = [
    {
      label: item.decision === "done" ? "Desmarcar entregue" : "Já entreguei",
      onSelect: () =>
        void decidir(item.decision === "done" ? "none" : "done", "Marcado como entregue."),
    },
    {
      label: item.decision === "skipped" ? "Voltar a considerar" : "Não vou fazer",
      onSelect: () =>
        void decidir(
          item.decision === "skipped" ? "none" : "skipped",
          "Marcado como não vou fazer.",
        ),
    },
    ...(item.plannedAt
      ? [{ label: "Desfazer o plano", onSelect: () => void planejar(null, 0) }]
      : []),
    { label: "Abrir a disciplina", onSelect: () => abrir(item.subjectId) },
  ];

  return (
    <li
      className="academic-row"
      data-kind={item.kind}
      data-horizonte={item.horizonte}
      data-decision={item.decision !== "none" ? item.decision : undefined}
      data-destaque={destaque || undefined}
    >
      <SubjectDot accent={item.subjectAccent} />
      <button type="button" className="academic-row-main" onClick={() => abrir(item.subjectId)}>
        <strong>{item.title}</strong>
        <small>
          {item.subject}
          {item.location ? ` · ${item.location}` : ""}
          {item.plannedAt ? ` · faço ${planoDe(item.plannedAt, item.plannedMinutes)}` : ""}
        </small>
      </button>
      <span className="academic-quando">{quandoDe(item.at, item.horizonte)}</span>
      {item.kind === "exam" ? <span className="academic-tag">Prova</span> : null}
      {item.decision === "none" ? (
        <Button
          variant={destaque && !item.plannedAt ? "secondary" : "ghost"}
          onClick={() => setPlanejando((aberto) => !aberto)}
          disabled={ocupado}
        >
          {item.plannedAt ? "Replanejar" : "Planejar"}
        </Button>
      ) : (
        <span className="academic-tag" data-decision={item.decision}>
          {item.decision === "done" ? "Entregue" : "Não vou fazer"}
        </span>
      )}
      <ActionMenu trigger={<span aria-hidden="true">···</span>} items={acoes} />
      {planejando ? (
        <PlanoForm
          item={item}
          fechar={() => setPlanejando(false)}
          planejar={planejar}
          ocupado={ocupado}
        />
      ) : null}
    </li>
  );
}

/** Quando vou fazer, e por quanto tempo.
 *
 *  Atalhos em vez de só um seletor de data: a resposta quase sempre é "hoje à
 *  noite" ou "amanhã", e abrir um calendário completo para escolher amanhã é o
 *  tipo de precisão que custa mais do que entrega. O campo exato continua ali
 *  para quem precisa dele. */
function PlanoForm({
  item,
  fechar,
  planejar,
  ocupado,
}: {
  item: Compromisso;
  fechar: () => void;
  planejar: (quando: string | null, minutos: number) => Promise<void>;
  ocupado: boolean;
}) {
  const [quando, setQuando] = useState(() => campoDoInstante(item.plannedAt ?? sugestaoDeQuando()));
  const [minutos, setMinutos] = useState(item.plannedMinutes || 60);

  return (
    <form
      className="academic-plano"
      onSubmit={(evento) => {
        evento.preventDefault();
        void planejar(instanteDoCampo(quando), minutos);
      }}
    >
      <span className="micro-label">QUANDO VOU FAZER</span>
      <p className="support-copy">
        O prazo é {quandoDe(item.at, item.horizonte)}. Isto é outra coisa: a hora em que você senta
        para fazer.
      </p>
      <div className="academic-plano-atalhos">
        {atalhosDePlano().map((atalho) => (
          <Button
            key={atalho.label}
            variant="ghost"
            onClick={() => setQuando(campoDoInstante(atalho.iso))}
          >
            {atalho.label}
          </Button>
        ))}
      </div>
      <div className="academic-form-grid">
        <label className="field">
          <span>Início</span>
          <input
            type="datetime-local"
            value={quando}
            onChange={(evento) => setQuando(evento.target.value)}
          />
        </label>
        <label className="field">
          <span>Duração</span>
          <select value={minutos} onChange={(evento) => setMinutos(Number(evento.target.value))}>
            <option value={0}>sem duração</option>
            <option value={30}>30 min</option>
            <option value={60}>1h</option>
            <option value={120}>2h</option>
            <option value={180}>3h</option>
          </select>
        </label>
      </div>
      <div className="academic-form-acoes">
        <Button variant="ghost" onClick={fechar}>
          Cancelar
        </Button>
        <Button variant="primary" type="submit" disabled={ocupado || !quando}>
          Planejar
        </Button>
      </div>
    </form>
  );
}

/** Hoje às 19h, se ainda der; senão amanhã às 19h. */
function sugestaoDeQuando(): string {
  const agora = new Date();
  const alvo = new Date(agora);
  alvo.setHours(19, 0, 0, 0);
  if (alvo.getTime() <= agora.getTime()) alvo.setDate(alvo.getDate() + 1);
  return alvo.toISOString();
}

function atalhosDePlano(): { label: string; iso: string }[] {
  const noite = (dias: number) => {
    const alvo = new Date();
    alvo.setDate(alvo.getDate() + dias);
    alvo.setHours(19, 0, 0, 0);
    return alvo.toISOString();
  };
  const agora = new Date();
  return [
    ...(agora.getHours() < 19 ? [{ label: "Hoje, 19h", iso: noite(0) }] : []),
    { label: "Amanhã, 19h", iso: noite(1) },
    { label: "Depois de amanhã", iso: noite(2) },
  ];
}

/** O que já foi resolvido, descartado, ou é resto de calendário antigo.
 *
 *  Fechado por padrão: histórico que compete com urgência deixa de ser
 *  histórico e vira ruído. */
function HistoricoPainel({
  dashboard,
  abrir,
  recarregar,
  avisar,
}: {
  dashboard: AcademicDashboard;
  abrir: (id: string) => void;
  recarregar: () => Promise<void>;
  avisar: (texto: string, desfazer?: () => Promise<void>) => void;
}) {
  if (!dashboard.history.length) return null;
  return (
    <Panel label="HISTÓRICO" value={String(dashboard.history.length)} unit="resolvidos">
      <details className="disclosure">
        <summary>O que saiu da frente</summary>
        <ul className="academic-lista">
          {dashboard.history.map((item) => (
            <CompromissoRow
              key={`${item.kind}-${item.id}`}
              item={item}
              abrir={abrir}
              recarregar={recarregar}
              avisar={avisar}
            />
          ))}
        </ul>
      </details>
    </Panel>
  );
}

/** O cronômetro de estudo, e quanto já foi hoje e na semana. */
function EstudoPainel({
  dashboard,
  subjects,
  recarregar,
}: {
  dashboard: AcademicDashboard;
  subjects: Subject[];
  recarregar: () => Promise<void>;
}) {
  const [escolhida, setEscolhida] = useState("");
  const [topico, setTopico] = useState("");
  const [erro, setErro] = useState("");

  const running = dashboard.running;

  async function comecar(event: FormEvent) {
    event.preventDefault();
    const alvo = escolhida || dashboard.subjects[0]?.id;
    if (!alvo) return;
    try {
      await api.academicStartStudy(alvo, topico);
      setTopico("");
      setErro("");
      await recarregar();
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }

  return (
    <Panel
      label="ESTUDO"
      value={duracaoDe(dashboard.studySecondsToday)}
      unit="hoje"
      action={<span className="row-meta">{duracaoDe(dashboard.studySecondsWeek)} na semana</span>}
    >
      {running ? (
        <StudyRunning session={running} subjects={subjects} recarregar={recarregar} />
      ) : (
        <form className="academic-study-form" onSubmit={comecar}>
          <label className="field">
            <span>Disciplina</span>
            <select value={escolhida} onChange={(event) => setEscolhida(event.target.value)}>
              {dashboard.subjects.map((subject) => (
                <option key={subject.id} value={subject.id}>
                  {subject.name}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Tópico (opcional)</span>
            <input
              value={topico}
              onChange={(event) => setTopico(event.target.value)}
              placeholder="O que vai estudar?"
            />
          </label>
          <Button variant="primary" type="submit" disabled={!dashboard.subjects.length}>
            Começar
          </Button>
          {erro ? <p className="form-error">{erro}</p> : null}
        </form>
      )}
    </Panel>
  );
}

/** A sessão em curso, com o relógio andando. */
function StudyRunning({
  session,
  subjects,
  recarregar,
}: {
  session: StudySession;
  subjects: Subject[];
  recarregar: () => Promise<void>;
}) {
  const [decorrido, setDecorrido] = useState(() => decorridoDe(session.startedAt));
  const [notas, setNotas] = useState("");

  useEffect(() => {
    setDecorrido(decorridoDe(session.startedAt));
    const timer = window.setInterval(() => setDecorrido(decorridoDe(session.startedAt)), 1000);
    return () => window.clearInterval(timer);
  }, [session.startedAt, session.id]);

  const materia = subjects.find((item) => item.id === session.subjectId);

  return (
    <div className="academic-running">
      <p className="academic-cronometro" aria-live="off">
        {cronometroDe(decorrido)}
      </p>
      <p className="academic-running-alvo">
        {materia?.name ?? "Disciplina"}
        {session.topic ? ` · ${session.topic}` : ""}
      </p>
      <label className="field">
        <span>Como foi (opcional)</span>
        <input value={notas} onChange={(event) => setNotas(event.target.value)} />
      </label>
      <div className="academic-running-acoes">
        <Button
          variant="primary"
          onClick={async () => {
            await api.academicFinishStudy(session.id, decorrido, notas);
            await recarregar();
          }}
        >
          Encerrar
        </Button>
        {/* Descartar existe para o cronômetro esquecido: uma sessão de oito
            horas registrada por engano estraga o histórico da semana inteira. */}
        <Button
          variant="ghost"
          onClick={async () => {
            await api.academicDiscardStudy(session.id);
            await recarregar();
          }}
        >
          Descartar
        </Button>
      </div>
    </div>
  );
}

function DisciplinasPainel({
  dashboard,
  abrir,
  facts,
}: {
  dashboard: AcademicDashboard;
  abrir: (id: string) => void;
  facts: Record<string, ProviderSubjectFact>;
}) {
  return (
    <Panel
      label="DISCIPLINAS"
      value={String(dashboard.subjects.length)}
      unit={dashboard.subjects.length === 1 ? "disciplina" : "disciplinas"}
      action={
        dashboard.semesterProgress !== null ? (
          <span className="row-meta">
            {Math.round(dashboard.semesterProgress * 100)}% do período
          </span>
        ) : undefined
      }
    >
      <div className="academic-grid">
        {dashboard.subjects.map((subject) => (
          <SubjectCard key={subject.id} subject={subject} abrir={abrir} fact={facts[subject.id]} />
        ))}
      </div>
      {!dashboard.subjects.length ? (
        <EmptyState>
          Nenhuma disciplina neste semestre. Adicione a primeira para começar a organizar as
          entregas e as provas.
        </EmptyState>
      ) : null}
    </Panel>
  );
}

function SubjectCard({
  subject,
  abrir,
  fact,
}: {
  subject: SubjectOverview;
  abrir: (id: string) => void;
  fact?: ProviderSubjectFact;
}) {
  const media = mediaDe(subject.media);
  return (
    <button
      type="button"
      className="academic-card"
      data-alerta={subject.overdue ? "true" : undefined}
      onClick={() => abrir(subject.id)}
    >
      <header>
        <SubjectDot accent={subject.accent} />
        <strong>{subject.name}</strong>
        {subject.code ? <span className="row-meta">{subject.code}</span> : null}
      </header>
      <p className="academic-card-situacao">{situacaoDe(subject)}</p>
      {subject.next ? (
        <p className="academic-card-next">
          <span>{subject.next.title}</span>
          <span className="row-meta">{quandoDe(subject.next.at, subject.next.horizonte)}</span>
        </p>
      ) : null}
      <footer>
        {media ? (
          <span className="academic-media" title={avaliadoDe(subject.pesoAvaliado)}>
            {media}
          </span>
        ) : (
          <span className="row-meta">sem nota</span>
        )}
        {/* A media da INSTITUICAO, ao lado da nossa e nunca no lugar dela.
            As duas discordam de proposito — a faculdade conta exame e
            recuperacao que o M/OS nao modela —, e esconder uma delas obrigaria
            a pessoa a abrir o portal para conferir a que importa na secretaria. */}
        {fact?.officialGrade !== undefined && fact?.officialGrade !== null ? (
          <span className="row-meta" title={`Média oficial da instituição${fact.situation ? ` · ${fact.situation}` : ""}`}>
            oficial {fact.officialGrade.toFixed(1).replace(".", ",")}
          </span>
        ) : null}
        {subject.studySecondsWeek ? (
          <span className="row-meta">{duracaoDe(subject.studySecondsWeek)} na semana</span>
        ) : null}
      </footer>
    </button>
  );
}

function SemestresPainel({
  semesters,
  atual,
  recarregar,
}: {
  semesters: Semester[];
  atual: string;
  recarregar: () => Promise<void>;
}) {
  const outros = semesters.filter((item) => item.id !== atual);
  if (!outros.length) return null;
  return (
    <Panel label="HISTÓRICO" count={String(outros.length)}>
      <ul className="academic-lista">
        {outros.map((semester) => (
          <li key={semester.id} className="academic-row">
            <span className="row-copy">
              <strong>{semester.name}</strong>
              <small>
                {semester.startsOn} — {semester.endsOn}
                {semester.institution ? ` · ${semester.institution}` : ""}
              </small>
            </span>
            <Button
              variant="ghost"
              onClick={async () => {
                await api.academicArchiveSemester(
                  semester.id,
                  semester.lifecycleState === "active",
                );
                await recarregar();
              }}
            >
              {semester.lifecycleState === "active" ? "Arquivar" : "Restaurar"}
            </Button>
          </li>
        ))}
      </ul>
    </Panel>
  );
}

// ===========================================================================
// Cockpit da disciplina
// ===========================================================================

function SubjectPage({
  subjectId,
  voltar,
  recarregar,
}: {
  subjectId: string;
  voltar: () => void;
  recarregar: () => Promise<void>;
}) {
  const [subject, setSubject] = useState<Subject | null>(null);
  const [assignments, setAssignments] = useState<Assignment[]>([]);
  const [exams, setExams] = useState<Exam[]>([]);
  const [materials, setMaterials] = useState<Resource[]>([]);
  const [overview, setOverview] = useState<SubjectOverview | null>(null);
  const [erro, setErro] = useState("");
  const [novaAtividade, setNovaAtividade] = useState(false);
  const [novaProva, setNovaProva] = useState(false);
  const [editando, setEditando] = useState(false);

  const carregar = useCallback(async () => {
    try {
      const [materias, atividades, provas, recursos, painel] = await Promise.all([
        api.academicSubjects(true),
        api.academicAssignments(false),
        api.academicExams(false),
        api.academicMaterials(subjectId),
        api.academicDashboard(),
      ]);
      setSubject(materias.find((item) => item.id === subjectId) ?? null);
      setAssignments(atividades.filter((item) => item.subjectId === subjectId));
      setExams(provas.filter((item) => item.subjectId === subjectId));
      setMaterials(recursos);
      setOverview(painel.subjects.find((item) => item.id === subjectId) ?? null);
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }, [subjectId]);

  useEffect(() => {
    void carregar();
  }, [carregar]);

  const atualizar = useCallback(async () => {
    await carregar();
    await recarregar();
  }, [carregar, recarregar]);

  if (!subject) {
    return (
      <div className="page academic-page">
        <ContextPath segments={["M", "ACADEMIC"]} />
        {erro ? <p className="form-error">{erro}</p> : <EmptyState>Carregando…</EmptyState>}
      </div>
    );
  }

  return (
    <div className="page academic-page">
      <ContextPath segments={["M", "ACADEMIC", subject.name.toUpperCase()]} />
      <PageHeader
        title={subject.name}
        subtitle={[subject.code, subject.teacher].filter(Boolean).join(" · ") || undefined}
        actions={
          <>
            <Button onClick={() => setNovaAtividade(true)}>Nova atividade</Button>
            <Button onClick={() => setNovaProva(true)}>Nova avaliação</Button>
            <Button variant="ghost" onClick={() => setEditando(true)}>
              Editar
            </Button>
            <Button variant="ghost" onClick={voltar}>
              Voltar
            </Button>
          </>
        }
      />

      {erro ? <p className="form-error">{erro}</p> : null}

      {editando ? (
        <SubjectForm
          semesterId={subject.semesterId}
          existente={subject}
          fechar={() => setEditando(false)}
          salvo={async () => {
            setEditando(false);
            await atualizar();
          }}
        />
      ) : null}

      {novaAtividade ? (
        <AssignmentForm
          subjectId={subjectId}
          fechar={() => setNovaAtividade(false)}
          salvo={async () => {
            setNovaAtividade(false);
            await atualizar();
          }}
        />
      ) : null}

      {novaProva ? (
        <ExamForm
          subjectId={subjectId}
          fechar={() => setNovaProva(false)}
          salvo={async () => {
            setNovaProva(false);
            await atualizar();
          }}
        />
      ) : null}

      {overview ? (
        <Panel
          label="DESEMPENHO"
          value={mediaDe(overview.media) || "—"}
          unit={overview.media !== null ? "média" : "sem nota ainda"}
          action={
            overview.pesoAvaliado !== null ? (
              <span className="row-meta">{avaliadoDe(overview.pesoAvaliado)}</span>
            ) : undefined
          }
        >
          <p className="academic-situacao">{situacaoDe(overview)}</p>
          {overview.studySecondsWeek ? (
            <p className="row-meta">{duracaoDe(overview.studySecondsWeek)} estudados nesta semana</p>
          ) : null}
        </Panel>
      ) : null}

      <Panel label="AVALIAÇÕES" count={String(exams.length)}>
        <ul className="academic-lista">
          {exams.map((exam) => (
            <ExamRow key={exam.id} exam={exam} atualizar={atualizar} />
          ))}
        </ul>
        {!exams.length ? (
          <EmptyState>Nenhuma avaliação marcada. Você está livre por enquanto.</EmptyState>
        ) : null}
      </Panel>

      <Panel label="ATIVIDADES" count={String(assignments.length)}>
        <ul className="academic-lista">
          {assignments.map((assignment) => (
            <AssignmentRow key={assignment.id} assignment={assignment} atualizar={atualizar} />
          ))}
        </ul>
        {!assignments.length ? (
          <EmptyState>Nada a entregar. Quando aparecer uma lista ou trabalho, registre aqui.</EmptyState>
        ) : null}
      </Panel>

      <MateriaisPainel subjectId={subjectId} materials={materials} atualizar={atualizar} />
    </div>
  );
}

function AssignmentRow({
  assignment,
  atualizar,
}: {
  assignment: Assignment;
  atualizar: () => Promise<void>;
}) {
  const [editando, setEditando] = useState(false);
  const [ocupado, setOcupado] = useState(false);

  async function mudar(status: AssignmentStatus) {
    setOcupado(true);
    try {
      await api.academicSetAssignmentStatus(assignment.id, status);
      await atualizar();
    } finally {
      setOcupado(false);
    }
  }

  if (editando) {
    return (
      <li className="academic-row-form">
        <AssignmentForm
          subjectId={assignment.subjectId}
          existente={assignment}
          fechar={() => setEditando(false)}
          salvo={async () => {
            setEditando(false);
            await atualizar();
          }}
        />
      </li>
    );
  }

  const concluida = assignment.status === "submitted" || assignment.status === "graded";

  return (
    <li className="academic-row" data-concluida={concluida || undefined}>
      <span className="row-copy">
        <strong>{assignment.title}</strong>
        <small>
          {STATUS_ATIVIDADE[assignment.status]}
          {assignment.dueAt ? ` · ${new Date(assignment.dueAt).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" })}` : " · sem prazo"}
          {assignment.score !== null && assignment.maxScore !== null
            ? ` · ${assignment.score}/${assignment.maxScore}`
            : ""}
          {assignment.weight ? ` · peso ${assignment.weight}` : ""}
        </small>
      </span>
      {assignment.taskId ? (
        <span className="academic-tag" title="Esta atividade tem uma Task no quadro">
          Task
        </span>
      ) : (
        <Button
          variant="ghost"
          disabled={ocupado}
          onClick={async () => {
            setOcupado(true);
            try {
              await api.academicCreateTask(assignment.id);
              await atualizar();
            } finally {
              setOcupado(false);
            }
          }}
        >
          Criar Task
        </Button>
      )}
      {!concluida ? (
        <Button variant="ghost" disabled={ocupado} onClick={() => mudar("submitted")}>
          Entreguei
        </Button>
      ) : (
        <Button variant="ghost" disabled={ocupado} onClick={() => mudar("pending")}>
          Reabrir
        </Button>
      )}
      <Button variant="ghost" onClick={() => setEditando(true)}>Editar</Button>
    </li>
  );
}

function ExamRow({ exam, atualizar }: { exam: Exam; atualizar: () => Promise<void> }) {
  const [editando, setEditando] = useState(false);

  if (editando) {
    return (
      <li className="academic-row-form">
        <ExamForm
          subjectId={exam.subjectId}
          existente={exam}
          fechar={() => setEditando(false)}
          salvo={async () => {
            setEditando(false);
            await atualizar();
          }}
        />
      </li>
    );
  }

  return (
    <li className="academic-row">
      <span className="row-copy">
        <strong>{exam.name}</strong>
        <small>
          {new Date(exam.at).toLocaleString("pt-BR", { dateStyle: "short", timeStyle: "short" })}
          {exam.location ? ` · ${exam.location}` : ""}
          {` · ${STATUS_AVALIACAO[exam.status]}`}
          {exam.score !== null && exam.maxScore !== null ? ` · ${exam.score}/${exam.maxScore}` : ""}
          {exam.weight ? ` · peso ${exam.weight}` : ""}
        </small>
      </span>
      {exam.topics ? <span className="row-meta academic-topicos">{exam.topics}</span> : null}
      <Button variant="ghost" onClick={() => setEditando(true)}>Editar</Button>
    </li>
  );
}

function MateriaisPainel({
  subjectId,
  materials,
  atualizar,
}: {
  subjectId: string;
  materials: Resource[];
  atualizar: () => Promise<void>;
}) {
  const [disponiveis, setDisponiveis] = useState<Resource[]>([]);
  const [escolhido, setEscolhido] = useState("");
  const [abrindo, setAbrindo] = useState(false);

  useEffect(() => {
    if (!abrindo) return;
    void api.resources(false).then(setDisponiveis).catch(() => setDisponiveis([]));
  }, [abrindo]);

  const jaLigados = new Set(materials.map((item) => item.id));
  const candidatos = disponiveis.filter((item) => !jaLigados.has(item.id));

  return (
    <Panel
      label="MATERIAIS"
      count={String(materials.length)}
      action={
        <Button variant="ghost" onClick={() => setAbrindo((atual) => !atual)}>
          {abrindo ? "Fechar" : "Adicionar"}
        </Button>
      }
    >
      {abrindo ? (
        <div className="academic-material-add">
          <label className="field">
            <span>Do acervo</span>
            <select value={escolhido} onChange={(event) => setEscolhido(event.target.value)}>
              <option value="">Escolha um recurso…</option>
              {candidatos.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.title}
                </option>
              ))}
            </select>
          </label>
          <Button
            disabled={!escolhido}
            onClick={async () => {
              await api.academicLinkMaterial(subjectId, escolhido, true);
              setEscolhido("");
              setAbrindo(false);
              await atualizar();
            }}
          >
            Vincular
          </Button>
          {/* O material É um Resource do M/OS, e não um arquivo próprio do
              Academic: o mesmo PDF serve a duas disciplinas, e a Library
              continua sendo o único lugar onde ele existe. */}
          <p className="row-meta">
            O material vem da Library. Salve o arquivo ou o link lá, e vincule aqui.
          </p>
        </div>
      ) : null}

      <ul className="academic-lista">
        {materials.map((item) => (
          <li key={item.id} className="academic-row">
            <span className="row-copy">
              <strong>{item.title}</strong>
              <small>{item.kind}</small>
            </span>
            <Button
              variant="ghost"
              onClick={async () => {
                await api.academicLinkMaterial(subjectId, item.id, false);
                await atualizar();
              }}
            >
              Desvincular
            </Button>
          </li>
        ))}
      </ul>
      {!materials.length && !abrindo ? (
        <EmptyState>Nenhum material vinculado. Salve na Library e conecte aqui.</EmptyState>
      ) : null}
    </Panel>
  );
}

// ===========================================================================
// Formulários
// ===========================================================================

function SemesterForm({ fechar, salvo }: { fechar: () => void; salvo: () => Promise<void> }) {
  const [name, setName] = useState("");
  const [institution, setInstitution] = useState("");
  const [startsOn, setStartsOn] = useState("");
  const [endsOn, setEndsOn] = useState("");
  const [erro, setErro] = useState("");
  const primeiro = useRef<HTMLInputElement>(null);

  useEffect(() => primeiro.current?.focus(), []);

  async function submeter(event: FormEvent) {
    event.preventDefault();
    try {
      await api.academicCreateSemester(name, institution, startsOn, endsOn);
      await salvo();
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }

  return (
    <form className="academic-form" onSubmit={submeter} aria-label="Novo semestre">
      <span className="micro-label">NOVO SEMESTRE</span>
      <div className="academic-form-grid">
        <label className="field">
          <span>Nome</span>
          <input ref={primeiro} value={name} onChange={(e) => setName(e.target.value)} placeholder="2026.2" required />
        </label>
        <label className="field">
          <span>Instituição (opcional)</span>
          <input value={institution} onChange={(e) => setInstitution(e.target.value)} />
        </label>
        <label className="field">
          <span>Começa em</span>
          <input type="date" value={startsOn} onChange={(e) => setStartsOn(e.target.value)} required />
        </label>
        <label className="field">
          <span>Termina em</span>
          <input type="date" value={endsOn} onChange={(e) => setEndsOn(e.target.value)} required />
        </label>
      </div>
      {erro ? <p className="form-error">{erro}</p> : null}
      <div className="academic-form-acoes">
        <Button variant="primary" type="submit">Criar</Button>
        <Button variant="ghost" onClick={fechar}>Cancelar</Button>
      </div>
    </form>
  );
}

function SubjectForm({
  semesterId,
  existente,
  fechar,
  salvo,
}: {
  semesterId: string;
  existente?: Subject;
  fechar: () => void;
  salvo: () => Promise<void>;
}) {
  const [name, setName] = useState(existente?.name ?? "");
  const [code, setCode] = useState(existente?.code ?? "");
  const [teacher, setTeacher] = useState(existente?.teacher ?? "");
  const [accent, setAccent] = useState(existente?.accent ?? "");
  const [notes, setNotes] = useState(existente?.notes ?? "");
  const [erro, setErro] = useState("");

  async function submeter(event: FormEvent) {
    event.preventDefault();
    try {
      if (existente) {
        await api.academicUpdateSubject(existente.id, name, code, teacher, accent, notes);
      } else {
        await api.academicCreateSubject(semesterId, name, code, teacher, accent, notes);
      }
      await salvo();
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }

  return (
    <form className="academic-form" onSubmit={submeter} aria-label={existente ? "Editar disciplina" : "Nova disciplina"}>
      <span className="micro-label">{existente ? "EDITAR DISCIPLINA" : "NOVA DISCIPLINA"}</span>
      <div className="academic-form-grid">
        <label className="field">
          <span>Nome</span>
          <input value={name} onChange={(e) => setName(e.target.value)} required autoFocus />
        </label>
        <label className="field">
          <span>Código (opcional)</span>
          <input value={code} onChange={(e) => setCode(e.target.value)} placeholder="EMC5132" />
        </label>
        <label className="field">
          <span>Professor (opcional)</span>
          <input value={teacher} onChange={(e) => setTeacher(e.target.value)} />
        </label>
        <label className="field">
          <span>Cor</span>
          <select value={accent} onChange={(e) => setAccent(e.target.value)}>
            {ACCENTS.map((item) => (
              <option key={item || "padrao"} value={item}>
                {item || "padrão"}
              </option>
            ))}
          </select>
        </label>
      </div>
      <label className="field">
        <span>Observações (opcional)</span>
        <textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} />
      </label>
      {erro ? <p className="form-error">{erro}</p> : null}
      <div className="academic-form-acoes">
        <Button variant="primary" type="submit">{existente ? "Salvar" : "Criar"}</Button>
        <Button variant="ghost" onClick={fechar}>Cancelar</Button>
      </div>
    </form>
  );
}

function AssignmentForm({
  subjectId,
  existente,
  fechar,
  salvo,
}: {
  subjectId: string;
  existente?: Assignment;
  fechar: () => void;
  salvo: () => Promise<void>;
}) {
  const [title, setTitle] = useState(existente?.title ?? "");
  const [description, setDescription] = useState(existente?.description ?? "");
  const [dueAt, setDueAt] = useState(campoDoInstante(existente?.dueAt ?? null));
  const [priority, setPriority] = useState<ReminderPriority>(existente?.priority ?? "normal");
  const [weight, setWeight] = useState(String(existente?.weight ?? 0));
  const [score, setScore] = useState(existente?.score !== null && existente?.score !== undefined ? String(existente.score) : "");
  const [maxScore, setMaxScore] = useState(existente?.maxScore !== null && existente?.maxScore !== undefined ? String(existente.maxScore) : "");
  const [status, setStatus] = useState<AssignmentStatus>(existente?.status ?? "pending");
  const [erro, setErro] = useState("");

  async function submeter(event: FormEvent) {
    event.preventDefault();
    const numero = (valor: string) => (valor.trim() === "" ? null : Number(valor));
    try {
      if (existente) {
        await api.academicUpdateAssignment({
          id: existente.id,
          title,
          description,
          dueAt: instanteDoCampo(dueAt),
          priority,
          weight: Number(weight) || 0,
          score: numero(score),
          maxScore: numero(maxScore),
          status,
        });
      } else {
        await api.academicCreateAssignment({
          subjectId,
          title,
          description,
          dueAt: instanteDoCampo(dueAt),
          priority,
          weight: Number(weight) || 0,
          score: numero(score),
          maxScore: numero(maxScore),
        });
      }
      await salvo();
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }

  return (
    <form className="academic-form" onSubmit={submeter} aria-label={existente ? "Editar atividade" : "Nova atividade"}>
      <span className="micro-label">{existente ? "EDITAR ATIVIDADE" : "NOVA ATIVIDADE"}</span>
      <div className="academic-form-grid">
        <label className="field">
          <span>Título</span>
          <input value={title} onChange={(e) => setTitle(e.target.value)} required autoFocus />
        </label>
        <label className="field">
          <span>Prazo (opcional)</span>
          <input type="datetime-local" value={dueAt} onChange={(e) => setDueAt(e.target.value)} />
        </label>
        <label className="field">
          <span>Prioridade</span>
          <select value={priority} onChange={(e) => setPriority(e.target.value as ReminderPriority)}>
            <option value="low">Baixa</option>
            <option value="normal">Normal</option>
            <option value="high">Alta</option>
            <option value="urgent">Urgente</option>
          </select>
        </label>
        <label className="field">
          <span>Peso na média</span>
          <input type="number" min="0" step="0.5" value={weight} onChange={(e) => setWeight(e.target.value)} />
        </label>
        <label className="field">
          <span>Nota</span>
          <input type="number" min="0" step="0.1" value={score} onChange={(e) => setScore(e.target.value)} />
        </label>
        <label className="field">
          <span>Nota máxima</span>
          <input type="number" min="0" step="0.1" value={maxScore} onChange={(e) => setMaxScore(e.target.value)} placeholder="10" />
        </label>
        {existente ? (
          <label className="field">
            <span>Estado</span>
            <select value={status} onChange={(e) => setStatus(e.target.value as AssignmentStatus)}>
              {Object.entries(STATUS_ATIVIDADE).map(([valor, rotulo]) => (
                <option key={valor} value={valor}>{rotulo}</option>
              ))}
            </select>
          </label>
        ) : null}
      </div>
      <label className="field">
        <span>Descrição (opcional)</span>
        <textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={2} />
      </label>
      {erro ? <p className="form-error">{erro}</p> : null}
      <div className="academic-form-acoes">
        <Button variant="primary" type="submit">{existente ? "Salvar" : "Criar"}</Button>
        <Button variant="ghost" onClick={fechar}>Cancelar</Button>
      </div>
    </form>
  );
}

function ExamForm({
  subjectId,
  existente,
  fechar,
  salvo,
}: {
  subjectId: string;
  existente?: Exam;
  fechar: () => void;
  salvo: () => Promise<void>;
}) {
  const [name, setName] = useState(existente?.name ?? "");
  const [at, setAt] = useState(campoDoInstante(existente?.at ?? null));
  const [location, setLocation] = useState(existente?.location ?? "");
  const [topics, setTopics] = useState(existente?.topics ?? "");
  const [weight, setWeight] = useState(String(existente?.weight ?? 0));
  const [score, setScore] = useState(existente?.score !== null && existente?.score !== undefined ? String(existente.score) : "");
  const [maxScore, setMaxScore] = useState(existente?.maxScore !== null && existente?.maxScore !== undefined ? String(existente.maxScore) : "");
  const [status, setStatus] = useState<ExamStatus>(existente?.status ?? "scheduled");
  const [erro, setErro] = useState("");

  async function submeter(event: FormEvent) {
    event.preventDefault();
    const instante = instanteDoCampo(at);
    if (!instante) {
      // Prova sem data é um plano, e o M/OS já tem Task para plano. O que faz
      // uma avaliação ser avaliação é ela ocupar um instante.
      setErro("Uma avaliação precisa de data e hora.");
      return;
    }
    const numero = (valor: string) => (valor.trim() === "" ? null : Number(valor));
    try {
      if (existente) {
        await api.academicUpdateExam({
          id: existente.id,
          name,
          at: instante,
          location,
          topics,
          weight: Number(weight) || 0,
          score: numero(score),
          maxScore: numero(maxScore),
          status,
        });
      } else {
        await api.academicCreateExam({
          subjectId,
          name,
          at: instante,
          location,
          topics,
          weight: Number(weight) || 0,
          score: numero(score),
          maxScore: numero(maxScore),
        });
      }
      await salvo();
    } catch (falha) {
      setErro(appError(falha).message);
    }
  }

  return (
    <form className="academic-form" onSubmit={submeter} aria-label={existente ? "Editar avaliação" : "Nova avaliação"}>
      <span className="micro-label">{existente ? "EDITAR AVALIAÇÃO" : "NOVA AVALIAÇÃO"}</span>
      <div className="academic-form-grid">
        <label className="field">
          <span>Nome</span>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="P1" required autoFocus />
        </label>
        <label className="field">
          <span>Quando</span>
          <input type="datetime-local" value={at} onChange={(e) => setAt(e.target.value)} required />
        </label>
        <label className="field">
          <span>Local (opcional)</span>
          <input value={location} onChange={(e) => setLocation(e.target.value)} />
        </label>
        <label className="field">
          <span>Peso na média</span>
          <input type="number" min="0" step="0.5" value={weight} onChange={(e) => setWeight(e.target.value)} />
        </label>
        <label className="field">
          <span>Nota</span>
          <input type="number" min="0" step="0.1" value={score} onChange={(e) => setScore(e.target.value)} />
        </label>
        <label className="field">
          <span>Nota máxima</span>
          <input type="number" min="0" step="0.1" value={maxScore} onChange={(e) => setMaxScore(e.target.value)} placeholder="10" />
        </label>
        {existente ? (
          <label className="field">
            <span>Estado</span>
            <select value={status} onChange={(e) => setStatus(e.target.value as ExamStatus)}>
              {Object.entries(STATUS_AVALIACAO).map(([valor, rotulo]) => (
                <option key={valor} value={valor}>{rotulo}</option>
              ))}
            </select>
          </label>
        ) : null}
      </div>
      <label className="field">
        <span>Conteúdo (opcional)</span>
        <textarea value={topics} onChange={(e) => setTopics(e.target.value)} rows={2} placeholder="Equilíbrio, momentos, treliças" />
      </label>
      {erro ? <p className="form-error">{erro}</p> : null}
      <div className="academic-form-acoes">
        <Button variant="primary" type="submit">{existente ? "Salvar" : "Criar"}</Button>
        <Button variant="ghost" onClick={fechar}>Cancelar</Button>
      </div>
    </form>
  );
}

/** O widget do Academic na Home. Mora aqui porque desenha as mesmas linhas. */
export function AcademicWidget({
  dashboard,
  abrir,
}: {
  dashboard: AcademicDashboard | null;
  abrir: () => void;
}) {
  if (!dashboard?.semester) {
    return <EmptyState>Nenhum semestre. Abra o Academic para começar.</EmptyState>;
  }
  const proximos = dashboard.upcoming.slice(0, 4);
  return (
    <>
      {proximos.map((item) => (
        <button
          key={`${item.kind}-${item.id}`}
          type="button"
          className="data-row"
          data-stale={item.horizonte === "overdue" || undefined}
          onClick={abrir}
        >
          <SubjectDot accent={item.subjectAccent} />
          <span className="row-copy">
            <strong>{item.title}</strong>
            <small>{item.subject}</small>
          </span>
          <span className="row-meta">{quandoDe(item.at, item.horizonte)}</span>
        </button>
      ))}
      {!proximos.length ? (
        <EmptyState>Nada marcado na faculdade.</EmptyState>
      ) : null}
    </>
  );
}

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Card, EmptyState, PageHeader } from "./Surface";
import { durationOf, hoursOf, moneyOf } from "./TempoShared";
import type { Client, Project, ProjectTracking, Totals, TrackingStatus } from "./types";

const STATUS: { value: TrackingStatus; label: string }[] = [
  { value: "active", label: "ativo" },
  { value: "paused", label: "pausado" },
  { value: "completed", label: "concluído" },
  { value: "archived", label: "arquivado" },
];

const STATUS_LABEL: Record<TrackingStatus, string> =
  Object.fromEntries(STATUS.map((item) => [item.value, item.label])) as Record<TrackingStatus, string>;

function emptyTracking(projectId: string): ProjectTracking {
  return {
    projectId,
    hourlyRateCents: 0,
    code: "",
    color: "",
    trackingStatus: "active",
    clientId: null,
    budgetMinutes: 0,
    paidAt: null,
  };
}

/**
 * Quanto do orçamento de horas já foi gasto.
 *
 * Sem meta, mostra só o trabalhado — uma barra vazia contra um alvo inexistente
 * sugeriria que falta muito para uma meta que ninguém definiu.
 */
function Budget({ workedSeconds, budgetMinutes }: { workedSeconds: number; budgetMinutes: number }) {
  if (budgetMinutes <= 0) {
    return <span className="tempo-budget-plain">{durationOf(workedSeconds)}</span>;
  }
  const budgetSeconds = budgetMinutes * 60;
  const percent = Math.min(100, (workedSeconds / budgetSeconds) * 100);
  const over = workedSeconds > budgetSeconds;
  return (
    <div className="tempo-budget">
      <div className="tempo-budget-numbers">
        <span className={over ? "over" : undefined}>{durationOf(workedSeconds)}</span>
        <span>{durationOf(budgetSeconds)}</span>
      </div>
      <div className="tempo-budget-track">
        {/* Mínimo de 2%: uma barra de largura zero some, e "comecei" não é a
            mesma informação que "não comecei". */}
        <div
          className="tempo-budget-fill"
          data-over={over || undefined}
          style={{ width: `${Math.max(2, percent)}%` }}
        />
      </div>
    </div>
  );
}

/**
 * Os Projects sob a ótica de quem cobra: valor/hora, cliente, meta e acumulado.
 *
 * Edita só a COBRANÇA. Nome e descrição continuam sendo assunto da página de
 * Projects — dois lugares editando o mesmo nome viram duas versões dele, e a
 * que aparece na fatura seria a última que alguém salvou por acaso.
 */
export function TempoProjects({ projects, totals, openProject, openClients }: {
  projects: Project[];
  totals: Record<string, Totals>;
  openProject: (project: Project) => void;
  /* Clientes moram em Configuracoes, e sao usados AQUI: e nesta tabela que se
     descobre que um Project esta sem cliente e que a fatura por isso nao sai.
     O botao e a ponte entre o lugar onde a falta aparece e o lugar onde ela se
     resolve. */
  openClients: () => void;
}) {
  const [tracking, setTracking] = useState<Record<string, ProjectTracking>>({});
  const [clients, setClients] = useState<Client[]>([]);
  const [query, setQuery] = useState("");
  const [note, setNote] = useState("");
  const [editing, setEditing] = useState<ProjectTracking | null>(null);
  const [editingName, setEditingName] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);

  const load = useCallback(async () => {
    const [rows, people] = await Promise.all([
      api.projectTracking().catch(() => [] as ProjectTracking[]),
      api.clients().catch(() => [] as Client[]),
    ]);
    setTracking(Object.fromEntries(rows.map((row) => [row.projectId, row])));
    setClients(people);
  }, []);

  useEffect(() => { void load(); }, [load]);

  const listed = useMemo(() => {
    const term = query.trim().toLowerCase();
    const rows = projects.map((project) => ({
      project,
      billing: tracking[project.id] ?? emptyTracking(project.id),
      total: totals[project.id],
    }));
    if (!term) return rows;
    return rows.filter(({ project, billing }) =>
      project.name.toLowerCase().includes(term) || billing.code.toLowerCase().includes(term));
  }, [projects, tracking, totals, query]);

  /* Duas somas, e nao uma.
   *
   * O rodape dizia "R$ 780,00" misturando o que ja entrou com o que ainda nao.
   * Um numero que soma dinheiro recebido com dinheiro a receber nao responde
   * nenhuma das duas perguntas — e a que importa e a segunda. */
  const sum = listed.reduce(
    (acc, row) => {
      const cents = row.total?.amountCents ?? 0;
      const billable = row.total?.billableSeconds ?? 0;
      if (row.billing.paidAt) {
        acc.paidCents += cents;
        acc.paidBillable += billable;
      } else {
        acc.cents += cents;
        acc.billable += billable;
      }
      return acc;
    },
    { billable: 0, cents: 0, paidBillable: 0, paidCents: 0 },
  );

  const clientName = (id: string | null) => clients.find((client) => client.id === id)?.name ?? "";

  function openEdit(billing: ProjectTracking, name: string) {
    setEditing({ ...billing });
    setEditingName(name);
    dialog.current?.showModal();
  }

  async function save() {
    if (!editing) return;
    setNote("");
    try {
      await api.setProjectTracking(editing);
      dialog.current?.close();
      setEditing(null);
      await load();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  return (
    <>
      {/* Acima da tabela: o erro de salvar uma cobrança precisa ser visto sem
          rolar por vinte Projects. */}
      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      <PageHeader
        title="Projetos"
        subtitle="Projects, valor/hora e estado. Dados guardados localmente."
        actions={
          <>
            <input
              className="tempo-search"
              value={query}
              placeholder="Pesquisar projeto ou código…"
              aria-label="Pesquisar Project"
              onChange={(event) => setQuery(event.currentTarget.value)}
            />
            <Button variant="outline" size="sm" onClick={openClients}>Clientes</Button>
          </>
        }
      />

      <Card className="flush">
        {listed.length ? (
          <table className="tempo-table tempo-table-projects">
            <thead>
              <tr>
                <th scope="col">Project</th>
                <th scope="col">Estado</th>
                <th scope="col">Progresso</th>
                <th scope="col">Valor/hora</th>
                <th scope="col">Acumulado</th>
                <th scope="col"><span className="visually-hidden">Ações</span></th>
              </tr>
            </thead>
            <tbody>
              {listed.map(({ project, billing, total }) => (
                <tr key={project.id}>
                  <th scope="row">
                    {/* O ponto le o ESTADO, e nao a cor do Project.

                        A cor por Project existe no dado, mas nenhum deles tem
                        uma definida: sairiam cinco pontos cinzas iguais, que e
                        decoracao — exatamente o que o §16 recusa. Amarrado ao
                        estado ele carrega dado, e some quando o Project esta
                        ativo, que e o caso normal e nao precisa de marca.

                        A palavra continua na coluna ao lado: o ponto acelera a
                        varredura, nao substitui o rotulo. */}
                    <span className="tempo-dot" data-status={billing.trackingStatus} aria-hidden="true" />
                    <button type="button" onClick={() => openProject(project)}>{project.name}</button>
                    <small>{[billing.code, clientName(billing.clientId)].filter(Boolean).join(" · ") || "sem código"}</small>
                  </th>
                  <td>{STATUS_LABEL[billing.trackingStatus]}</td>
                  <td>
                    <Budget workedSeconds={total?.grossSeconds ?? 0} budgetMinutes={billing.budgetMinutes} />
                  </td>
                  <td>{billing.hourlyRateCents ? `${moneyOf(billing.hourlyRateCents)}/h` : "—"}</td>
                  <td>
                    <strong>{moneyOf(total?.amountCents ?? 0)}</strong>
                    {/* "pago em 14/07" no lugar de "cobraveis": a linha ja mostra
                        o valor, e o que muda com o pagamento nao e quanto — e se
                        ainda esta na rua. */}
                    <small>
                      {billing.paidAt
                        ? `pago em ${new Date(billing.paidAt).toLocaleDateString("pt-BR")}`
                        : `${hoursOf(total?.billableSeconds ?? 0)} cobráveis`}
                    </small>
                  </td>
                  <td>
                    <Button variant="ghost" size="sm" onClick={() => openEdit(billing, project.name)}>Cobrança</Button>
                  </td>
                </tr>
              ))}
            </tbody>
            <tfoot>
              <tr>
                <th scope="row" colSpan={4}>
                  {listed.length === projects.length ? "Todos os Projects" : `${listed.length} Projects filtrados`}
                </th>
                <td>
                  <strong>{moneyOf(sum.cents)}</strong>
                  <small>{hoursOf(sum.billable)} a receber</small>
                </td>
                <td>
                  {/* So aparece quando ha o que mostrar: um "R$ 0,00 pago" fixo
                      ocuparia a coluna todo dia para dizer nada. */}
                  {sum.paidCents ? (
                    <span className="tempo-paid-total">
                      <strong>{moneyOf(sum.paidCents)}</strong>
                      <small>{hoursOf(sum.paidBillable)} já pagos</small>
                    </span>
                  ) : null}
                </td>
              </tr>
            </tfoot>
          </table>
        ) : (
          <EmptyState>
            {projects.length
              ? "Nenhum Project bate com o filtro."
              : "Projects nascem na página de Projects — aqui eles ganham valor/hora, cliente e meta."}
          </EmptyState>
        )}
      </Card>

      <dialog ref={dialog} className="restore-dialog" onCancel={() => { dialog.current?.close(); setEditing(null); }}>
        <span className="micro-label">COBRANÇA</span>
        <h2>{editingName}</h2>
        {/* Dito antes de o usuário mudar o número, e não depois: reajustar taxa é
            das poucas edições que parecem retroativas e não são. */}
        <p className="support-copy">
          O valor/hora vale <strong>daqui para frente</strong>. Cada sessão já guarda a taxa que valia quando o
          trabalho aconteceu, então reajustar aqui não reescreve o que já foi cobrado.
        </p>
        {editing ? (
          <form className="tempo-form tempo-form-stack" onSubmit={(event) => { event.preventDefault(); void save(); }}>
            <div className="tempo-field">
              <label htmlFor="billing-code">Código da obra</label>
              <input
                id="billing-code"
                value={editing.code}
                placeholder="043"
                onChange={(event) => setEditing({ ...editing, code: event.currentTarget.value })}
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="billing-client">Cliente</label>
              <select
                id="billing-client"
                value={editing.clientId ?? ""}
                onChange={(event) => setEditing({ ...editing, clientId: event.currentTarget.value || null })}
              >
                <option value="">Sem cliente</option>
                {clients.map((client) => <option key={client.id} value={client.id}>{client.name}</option>)}
              </select>
            </div>
            <div className="tempo-field">
              <label htmlFor="billing-rate">Valor/hora (R$)</label>
              {/* Em reais na tela e centavos no banco: dinheiro fracionário em
                  ponto flutuante acumula erro, e aqui ele viraria fatura. */}
              <input
                id="billing-rate"
                type="number"
                min={0}
                step={1}
                value={editing.hourlyRateCents / 100}
                onChange={(event) => setEditing({
                  ...editing,
                  hourlyRateCents: Math.round(Math.max(0, Number(event.currentTarget.value) || 0) * 100),
                })}
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="billing-budget">Meta de horas</label>
              <input
                id="billing-budget"
                type="number"
                min={0}
                step={1}
                value={editing.budgetMinutes / 60}
                onChange={(event) => setEditing({
                  ...editing,
                  budgetMinutes: Math.round(Math.max(0, Number(event.currentTarget.value) || 0) * 60),
                })}
              />
            </div>
            {/* Governa a coluna "a receber" da tabela e os dois numeros do
                Painel, entao vem com a data do dia — quem marca sabe QUE pagou,
                e a data e o que responde "quando" tres meses depois sem obrigar
                ninguem a digita-la. */}
            <label className="tempo-check tempo-check-governa">
              <input
                type="checkbox"
                checked={Boolean(editing.paidAt)}
                onChange={(event) => setEditing({
                  ...editing,
                  paidAt: event.currentTarget.checked ? new Date().toISOString() : null,
                })}
              />
              Já pago{editing.paidAt ? ` · ${new Date(editing.paidAt).toLocaleDateString("pt-BR")}` : ""}
            </label>
            <div className="tempo-field">
              <label htmlFor="billing-status">Estado</label>
              <select
                id="billing-status"
                value={editing.trackingStatus}
                onChange={(event) => setEditing({
                  ...editing,
                  trackingStatus: event.currentTarget.value as TrackingStatus,
                })}
              >
                {STATUS.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
              </select>
            </div>
            <div className="form-actions">
              <Button variant="ghost" onClick={() => { dialog.current?.close(); setEditing(null); }}>Cancelar</Button>
              <Button variant="primary" type="submit">Salvar</Button>
            </div>
          </form>
        ) : null}
      </dialog>
    </>
  );
}

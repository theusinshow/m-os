import { useEffect, useMemo, useState } from "react";
import {
  Archive,
  CheckCircle2,
  Pencil,
  Plus,
  Search,
  StickyNote,
  Users,
} from "lucide-react";
import type { Project } from "@/types/domain";
import { EMPTY_BILLING, useCatalogStore } from "@/stores/catalogStore";
import { useNotesStore } from "@/stores/notesStore";
import { formatCurrency, formatDuration } from "@/lib/format";
import { PROJECT_STATUS_LABELS } from "@/lib/labels";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { Input } from "@/components/ui/Field";
import { ProjectForm } from "./ProjectForm";
import { ProjectNotesModal } from "./ProjectNotesModal";
import { ClientsModal } from "@/features/clients/ClientsModal";

/**
 * Lista de projetos com dados reais (persistencia SQLite via comandos Tauri).
 * Clientes ficam integrados a esta tela, mas separados no banco (secao 13).
 */
export function ProjectsPage() {
  const {
    clients,
    projects,
    projectBilling,
    loading,
    loaded,
    error,
    loadAll,
    loadTotals,
    setProjectStatus,
  } = useCatalogStore();

  const [query, setQuery] = useState("");
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Project | null>(null);
  const [clientsOpen, setClientsOpen] = useState(false);
  // Guarda o id (nao o objeto): assim o modal sempre le a versao mais recente
  // do projeto na lista, mesmo depois de salvar as anotacoes.
  const [notesForId, setNotesForId] = useState<string | null>(null);

  const loadTodos = useNotesStore((s) => s.load);
  const todosLoaded = useNotesStore((s) => s.loaded);

  useEffect(() => {
    // Os acumulados sao recarregados a cada visita (nao so na primeira): o
    // valor muda ao encerrar sessoes, editar o historico ou alterar o
    // arredondamento, e um total desatualizado aqui e um total errado.
    if (loaded) void loadTotals();
    else void loadAll();
    if (!todosLoaded) void loadTodos();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return projects;
    return projects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        (p.code ?? "").toLowerCase().includes(q),
    );
  }, [projects, query]);

  const clientName = (id: string | null) =>
    clients.find((c) => c.id === id)?.name ?? "Sem cliente";

  /** Soma dos projetos visiveis — o "valor completo do trabalho". */
  const grandTotal = useMemo(
    () =>
      filtered.reduce(
        (acc, p) => {
          const b = projectBilling[p.id] ?? EMPTY_BILLING;
          acc.billableSeconds += b.billableSeconds;
          acc.amountCents += b.amountCents;
          return acc;
        },
        { billableSeconds: 0, amountCents: 0 },
      ),
    [filtered, projectBilling],
  );

  const notesFor = projects.find((p) => p.id === notesForId) ?? null;

  function openNew() {
    setEditing(null);
    setFormOpen(true);
  }
  function openEdit(project: Project) {
    setEditing(project);
    setFormOpen(true);
  }

  return (
    <div>
      <PageHeader
        title="Projetos"
        description="Projetos, valor/hora e status. Dados persistidos localmente."
        action={
          <div className="flex gap-2">
            <Button
              variant="secondary"
              onClick={() => setClientsOpen(true)}
              icon={<Users size={16} strokeWidth={1.75} />}
            >
              Clientes
            </Button>
            <Button
              variant="primary"
              onClick={openNew}
              icon={<Plus size={16} strokeWidth={2} />}
            >
              Novo projeto
            </Button>
          </div>
        }
      />

      <div className="mb-4 flex items-center gap-2">
        <div className="relative w-full max-w-xs">
          <Search
            size={15}
            strokeWidth={1.75}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-text-subtle"
          />
          <Input
            placeholder="Pesquisar projeto ou codigo…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="pl-8"
          />
        </div>
      </div>

      {error && (
        <Panel className="mb-4 border-danger/40 p-4">
          <p className="text-sm text-danger">Nao foi possivel carregar: {error}</p>
          <p className="mt-1 text-xs text-text-muted">
            Os cadastros exigem o backend (rode com <code>npm run tauri:dev</code>).
          </p>
        </Panel>
      )}

      {loading && !loaded ? (
        <Panel className="p-6 text-sm text-text-muted">Carregando…</Panel>
      ) : filtered.length === 0 ? (
        <EmptyState
          title={
            projects.length === 0
              ? "Nenhum projeto cadastrado"
              : "Nenhum projeto encontrado"
          }
          description={
            projects.length === 0
              ? "Crie seu primeiro projeto para comecar a registrar horas."
              : "Ajuste a pesquisa para ver outros projetos."
          }
          action={
            projects.length === 0 ? (
              <Button variant="primary" onClick={openNew} icon={<Plus size={16} />}>
                Novo projeto
              </Button>
            ) : undefined
          }
        />
      ) : (
        <Panel>
          {/*
            Duas concessoes para o acumulado caber na janela minima (960px, que
            deixa 680px para a tabela) sem virar rolagem horizontal:

            - o cliente vive na celula do projeto, nao em coluna propria;
            - "Valor/hora" so aparece a partir de `xl` — e dado secundario e
              esta no formulario de edicao, enquanto o acumulado e o motivo de
              se abrir esta tela.
          */}
          {/*
            `table-fixed` com larguras declaradas nos cabecalhos: em tabela de
            layout automatico o `truncate` da celula do projeto nao tem efeito
            (a celula cresce ate caber o texto) e empurrava a coluna de
            acumulado para fora da tela. Fixando as demais, sobra o resto para
            o nome do projeto, que ai sim trunca.
          */}
          <div className="overflow-x-auto">
            <table className="w-full min-w-[810px] table-fixed text-sm xl:min-w-[920px]">
              <thead>
                <tr className="border-b border-border text-left text-2xs uppercase tracking-wide text-text-subtle">
                  <th className="px-4 py-2 font-medium">Projeto</th>
                  <th className="w-[80px] px-4 py-2 font-medium">Status</th>
                  <th className="w-[160px] px-4 py-2 font-medium">Progresso</th>
                  <th className="hidden w-[110px] px-4 py-2 text-right font-medium xl:table-cell">
                    Valor/hora
                  </th>
                  <th className="w-[150px] px-4 py-2 text-right font-medium">
                    Acumulado
                  </th>
                  {/* 4 botoes de 39px (h-8 px-3 + icone) + gaps + padding. */}
                  <th className="w-[200px] px-4 py-2 text-right font-medium">Acoes</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {filtered.map((project) => {
                  const billing = projectBilling[project.id] ?? EMPTY_BILLING;
                  return (
                    <tr key={project.id} className="hover:bg-surface-hover">
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <span
                            className="h-2.5 w-2.5 shrink-0 rounded-full"
                            style={{ background: project.color ?? "var(--color-accent)" }}
                            aria-hidden
                          />
                          <div className="min-w-0">
                            <p className="truncate text-text">{project.name}</p>
                            <p className="truncate text-xs text-text-muted">
                              {[project.code, clientName(project.clientId)]
                                .filter(Boolean)
                                .join(" · ")}
                            </p>
                          </div>
                        </div>
                      </td>
                      <td className="px-4 py-3 text-text-muted">
                        {PROJECT_STATUS_LABELS[project.status]}
                      </td>
                      <td className="px-4 py-3">
                        <BudgetProgress
                          workedSeconds={billing.grossSeconds}
                          budgetMinutes={project.budgetMinutes}
                        />
                      </td>
                      <td className="tabular hidden whitespace-nowrap px-4 py-3 text-right text-text-muted xl:table-cell">
                        {formatCurrency(project.hourlyRateCents)}
                      </td>
                      <td
                        className="whitespace-nowrap px-4 py-3 text-right"
                        title={`${formatDuration(billing.billableSeconds)} faturaveis de ${formatDuration(billing.grossSeconds)} registrados`}
                      >
                        <p className="tabular font-medium text-text">
                          {formatCurrency(billing.amountCents)}
                        </p>
                        <p className="tabular text-2xs text-text-muted">
                          {formatDuration(billing.billableSeconds)}
                        </p>
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => setNotesForId(project.id)}
                            aria-label={`Anotacoes de ${project.name}`}
                            icon={<StickyNote size={15} strokeWidth={1.75} />}
                          />
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => openEdit(project)}
                            aria-label={`Editar ${project.name}`}
                            icon={<Pencil size={15} strokeWidth={1.75} />}
                          />
                          {project.status !== "completed" && (
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => void setProjectStatus(project.id, "completed")}
                              aria-label={`Concluir ${project.name}`}
                              icon={<CheckCircle2 size={15} strokeWidth={1.75} />}
                            />
                          )}
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => void setProjectStatus(project.id, "archived")}
                            aria-label={`Arquivar ${project.name}`}
                            icon={<Archive size={15} strokeWidth={1.75} />}
                          />
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
              {/*
                O rodape repete a estrutura de celulas das linhas em vez de usar
                colSpan: com "Valor/hora" oculto por breakpoint, um colSpan fixo
                desalinharia o total da sua coluna.
              */}
              <tfoot>
                <tr className="border-t border-border">
                  <td className="px-4 py-3 text-2xs uppercase tracking-wide text-text-subtle">
                    {filtered.length === projects.length
                      ? "Total de todos os projetos"
                      : `Total dos ${filtered.length} projetos filtrados`}
                  </td>
                  <td />
                  <td />
                  <td className="hidden xl:table-cell" />
                  <td className="whitespace-nowrap px-4 py-3 text-right">
                    <p className="tabular text-sm font-semibold text-text">
                      {formatCurrency(grandTotal.amountCents)}
                    </p>
                    <p className="tabular text-2xs text-text-muted">
                      {formatDuration(grandTotal.billableSeconds)} faturaveis
                    </p>
                  </td>
                  <td />
                </tr>
              </tfoot>
            </table>
          </div>
        </Panel>
      )}

      <ProjectForm
        open={formOpen}
        project={editing}
        onClose={() => setFormOpen(false)}
      />
      <ClientsModal open={clientsOpen} onClose={() => setClientsOpen(false)} />
      <ProjectNotesModal
        project={notesFor}
        onClose={() => setNotesForId(null)}
      />
    </div>
  );
}

function BudgetProgress({
  workedSeconds,
  budgetMinutes,
}: {
  workedSeconds: number;
  budgetMinutes: number;
}) {
  const worked = formatDuration(workedSeconds);
  if (budgetMinutes <= 0) {
    return <span className="tabular text-xs text-text-muted">{worked}</span>;
  }
  const budgetSeconds = budgetMinutes * 60;
  const pct = Math.min(100, (workedSeconds / budgetSeconds) * 100);
  const over = workedSeconds > budgetSeconds;
  return (
    <div>
      <div className="tabular flex justify-between gap-2 whitespace-nowrap text-2xs text-text-muted">
        <span className={over ? "text-danger" : ""}>{worked}</span>
        <span>{formatDuration(budgetSeconds)}</span>
      </div>
      <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-surface-raised">
        <div
          className={`h-full rounded-full ${over ? "bg-danger" : "bg-accent"}`}
          style={{ width: `${Math.max(2, pct)}%` }}
        />
      </div>
    </div>
  );
}

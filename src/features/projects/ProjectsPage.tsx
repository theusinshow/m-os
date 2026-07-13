import { useEffect, useMemo, useState } from "react";
import { Archive, CheckCircle2, Pencil, Plus, Search, Users } from "lucide-react";
import type { Project } from "@/types/domain";
import { useCatalogStore } from "@/stores/catalogStore";
import { formatCurrency, formatDuration } from "@/lib/format";
import { PROJECT_STATUS_LABELS } from "@/lib/labels";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { Input } from "@/components/ui/Field";
import { ProjectForm } from "./ProjectForm";
import { ClientsModal } from "@/features/clients/ClientsModal";

/**
 * Lista de projetos com dados reais (persistencia SQLite via comandos Tauri).
 * Clientes ficam integrados a esta tela, mas separados no banco (secao 13).
 */
export function ProjectsPage() {
  const {
    clients,
    projects,
    projectTotals,
    loading,
    loaded,
    error,
    loadAll,
    setProjectStatus,
  } = useCatalogStore();

  const [query, setQuery] = useState("");
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Project | null>(null);
  const [clientsOpen, setClientsOpen] = useState(false);

  useEffect(() => {
    if (!loaded) void loadAll();
  }, [loaded, loadAll]);

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
    clients.find((c) => c.id === id)?.name ?? "—";

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
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-2xs uppercase tracking-wide text-text-subtle">
                <th className="px-4 py-2 font-medium">Projeto</th>
                <th className="px-4 py-2 font-medium">Cliente</th>
                <th className="px-4 py-2 font-medium">Status</th>
                <th className="px-4 py-2 font-medium">Progresso</th>
                <th className="px-4 py-2 text-right font-medium">Valor/hora</th>
                <th className="px-4 py-2 text-right font-medium">Acoes</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {filtered.map((project) => (
                <tr key={project.id} className="hover:bg-surface-hover">
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-2">
                      <span
                        className="h-2.5 w-2.5 shrink-0 rounded-full"
                        style={{ background: project.color ?? "var(--color-accent)" }}
                        aria-hidden
                      />
                      <div>
                        <p className="text-text">{project.name}</p>
                        {project.code && (
                          <p className="text-xs text-text-muted">{project.code}</p>
                        )}
                      </div>
                    </div>
                  </td>
                  <td className="px-4 py-3 text-text-muted">
                    {clientName(project.clientId)}
                  </td>
                  <td className="px-4 py-3 text-text-muted">
                    {PROJECT_STATUS_LABELS[project.status]}
                  </td>
                  <td className="px-4 py-3">
                    <BudgetProgress
                      workedSeconds={projectTotals[project.id] ?? 0}
                      budgetMinutes={project.budgetMinutes}
                    />
                  </td>
                  <td className="tabular px-4 py-3 text-right text-text">
                    {formatCurrency(project.hourlyRateCents)}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex justify-end gap-1">
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
              ))}
            </tbody>
          </table>
        </Panel>
      )}

      <ProjectForm
        open={formOpen}
        project={editing}
        onClose={() => setFormOpen(false)}
      />
      <ClientsModal open={clientsOpen} onClose={() => setClientsOpen(false)} />
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
    <div className="w-32">
      <div className="tabular flex justify-between text-2xs text-text-muted">
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

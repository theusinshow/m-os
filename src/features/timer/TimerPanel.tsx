import { useEffect, useMemo, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { Clock, Pause, Play, Square, Zap } from "lucide-react";
import type { ActivityType, Project } from "@/types/domain";
import { useTimerStore } from "@/stores/timerStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useEntriesStore } from "@/stores/entriesStore";
import { ACTIVITY_TYPE_OPTIONS } from "@/lib/labels";
import { recentProjectIds } from "@/lib/projects";
import { Panel } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { Field, Input, Select } from "@/components/ui/Field";
import { QuickTimeModal } from "@/features/history/QuickTimeModal";
import { TimerCard } from "./TimerCard";
import { StopConfirmModal } from "./StopConfirmModal";

/**
 * Painel interativo do cronometro. Sem cronometro ativo, exibe o formulario de
 * inicio (projeto + atividade + descricao). Com cronometro ativo, exibe o
 * TimerCard e os controles de pausar/continuar/encerrar.
 */
export function TimerPanel() {
  const { activeTimer, start, pause, resume, stop } = useTimerStore();
  const projects = useCatalogStore((s) => s.projects);
  const entries = useEntriesStore((s) => s.entries);

  const [projectId, setProjectId] = useState("");
  const [activityType, setActivityType] = useState<ActivityType>("drawing");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmingStop, setConfirmingStop] = useState(false);
  const [quickOpen, setQuickOpen] = useState(false);

  const activeProject =
    projects.find((p) => p.id === activeTimer?.projectId) ?? null;

  // Projetos usados mais recentemente (para inicio em 1 clique).
  const recentProjects = useMemo<Project[]>(() => {
    return recentProjectIds(entries, projects)
      .map((id) => projects.find((p) => p.id === id))
      .filter((p): p is Project => p !== undefined);
  }, [entries, projects]);

  // Pre-seleciona o projeto mais recente (ou o primeiro) no formulario.
  useEffect(() => {
    if (projectId || projects.length === 0) return;
    setProjectId(recentProjects[0]?.id ?? projects[0]?.id ?? "");
  }, [projectId, projects, recentProjects]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(typeof err === "string" ? err : "Operacao falhou.");
    } finally {
      setBusy(false);
    }
  }

  async function handleStart(e: FormEvent) {
    e.preventDefault();
    await run(() =>
      start({ projectId, activityType, description: description || null }),
    );
    setDescription("");
  }

  if (activeTimer) {
    const running = activeTimer.status === "running";
    return (
      <Panel className="border-l-2 border-l-accent p-6">
        <TimerCard timer={activeTimer} project={activeProject} />
        <div className="mt-6 flex gap-2">
          {running ? (
            <Button
              variant="primary"
              onClick={() => void run(pause)}
              disabled={busy}
              icon={<Pause size={16} strokeWidth={2} />}
            >
              Pausar
            </Button>
          ) : (
            <Button
              variant="primary"
              onClick={() => void run(resume)}
              disabled={busy}
              icon={<Play size={16} strokeWidth={2} />}
            >
              Continuar
            </Button>
          )}
          <Button
            variant="danger"
            onClick={() => setConfirmingStop(true)}
            disabled={busy}
            icon={<Square size={16} strokeWidth={2} />}
          >
            Encerrar
          </Button>
        </div>
        {error && <p className="mt-3 text-sm text-danger">{error}</p>}

        <StopConfirmModal
          open={confirmingStop}
          timer={activeTimer}
          project={activeProject}
          busy={busy}
          onCancel={() => setConfirmingStop(false)}
          onPause={() => {
            setConfirmingStop(false);
            void run(pause);
          }}
          onStop={() => {
            setConfirmingStop(false);
            void run(stop);
          }}
        />
      </Panel>
    );
  }

  return (
    <Panel className="p-6">
      <p className="text-sm font-medium text-text">Iniciar trabalho</p>
      <p className="mt-0.5 text-xs text-text-muted">
        Escolha o projeto e comece a registrar as horas.
      </p>

      {projects.length === 0 ? (
        <div className="mt-4 rounded border border-dashed border-border px-4 py-6 text-center">
          <p className="text-sm text-text-muted">
            Nenhum projeto ativo. Crie um projeto para comecar.
          </p>
          <Link
            to="/projetos"
            className="mt-2 inline-block text-sm text-accent hover:underline"
          >
            Ir para Projetos
          </Link>
        </div>
      ) : (
        <form onSubmit={handleStart} className="mt-4 space-y-4">
          {recentProjects.length > 0 && (
            <div>
              <p className="mb-1.5 text-2xs uppercase tracking-wide text-text-subtle">
                Iniciar rapido
              </p>
              <div className="flex flex-wrap gap-2">
                {recentProjects.map((p) => (
                  <Button
                    key={p.id}
                    type="button"
                    variant="secondary"
                    size="sm"
                    disabled={busy}
                    onClick={() =>
                      void run(() =>
                        start({
                          projectId: p.id,
                          activityType,
                          description: null,
                        }),
                      )
                    }
                    icon={<Zap size={14} strokeWidth={1.75} />}
                  >
                    {p.code ?? p.name}
                  </Button>
                ))}
              </div>
            </div>
          )}
          <div className="grid gap-3 sm:grid-cols-2">
            <Field label="Projeto" htmlFor="t-project" required>
              <Select
                id="t-project"
                value={projectId}
                onChange={(e) => setProjectId(e.target.value)}
                required
              >
                <option value="">Selecione…</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.code ? `${p.code} · ${p.name}` : p.name}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Atividade" htmlFor="t-activity">
              <Select
                id="t-activity"
                value={activityType}
                onChange={(e) => setActivityType(e.target.value as ActivityType)}
              >
                {ACTIVITY_TYPE_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </Select>
            </Field>
          </div>
          <Field label="Descricao (opcional)" htmlFor="t-desc">
            <Input
              id="t-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="O que voce vai fazer?"
            />
          </Field>
          <Button
            variant="primary"
            type="submit"
            disabled={busy || !projectId}
            icon={<Play size={16} strokeWidth={2} />}
          >
            {busy ? "Iniciando…" : "Iniciar"}
          </Button>
          {error && <p className="text-sm text-danger">{error}</p>}
        </form>
      )}

      {projects.length > 0 && (
        <div className="mt-5 border-t border-border pt-4">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setQuickOpen(true)}
            icon={<Clock size={14} strokeWidth={1.75} />}
          >
            Esqueceu de registrar? Adicionar tempo
          </Button>
        </div>
      )}

      <QuickTimeModal
        open={quickOpen}
        defaultProjectId={projectId}
        onClose={() => setQuickOpen(false)}
      />
    </Panel>
  );
}

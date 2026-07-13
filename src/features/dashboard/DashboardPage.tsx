import { Link } from "react-router-dom";
import { CalendarClock } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Stat } from "@/components/ui/Stat";
import { EmptyState } from "@/components/ui/EmptyState";
import { TimerPanel } from "@/features/timer/TimerPanel";
import { RecoveryModal } from "@/features/timer/RecoveryModal";
import { useTimerStore } from "@/stores/timerStore";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { elapsedSeconds } from "@/lib/duration";
import { formatCurrency, formatDuration } from "@/lib/format";
import { amountForDuration } from "@/lib/money";
import { ACTIVITY_TYPE_LABELS } from "@/lib/labels";
import { useNow } from "@/hooks/useNow";
import { isSameLocalDay } from "@/lib/datetime";
import { ROUTES } from "@/app/routes";

function isWithinDays(iso: string, days: number, ref: Date): boolean {
  const diff = ref.getTime() - new Date(iso).getTime();
  return diff >= 0 && diff <= days * 24 * 60 * 60 * 1000;
}

/**
 * Painel inicial (secao 13). Cronometro interativo (fonte real), resumo do dia
 * e sessoes recentes reais. A "Linha do tempo detectada" e ilustrativa ate a
 * deteccao de processos (Fases 4 e 7).
 */
export function DashboardPage() {
  const activeTimer = useTimerStore((s) => s.activeTimer);
  const entries = useEntriesStore((s) => s.entries);
  const projects = useCatalogStore((s) => s.projects);
  const catalogLoaded = useCatalogStore((s) => s.loaded);
  const now = useNow(1000);
  const today = new Date(now);

  const activeElapsed = activeTimer ? elapsedSeconds(activeTimer, now) : 0;
  const todaySeconds =
    entries
      .filter((e) => isSameLocalDay(e.startedAt, today))
      .reduce((sum, e) => sum + e.durationSeconds, 0) + activeElapsed;

  const weekAmount = entries
    .filter((e) => isWithinDays(e.startedAt, 7, today))
    .reduce(
      (sum, e) =>
        sum + amountForDuration(e.durationSeconds, e.hourlyRateSnapshotCents),
      0,
    );

  const projectName = (id: string) =>
    projects.find((p) => p.id === id)?.name ?? "Projeto";
  const recent = entries.slice(0, 5);

  return (
    <div>
      <PageHeader
        title="Painel"
        description="Cronometro atual, resumo do dia e sessoes recentes."
      />

      {catalogLoaded && projects.length === 0 && <WelcomeOnboarding />}

      <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
        <TimerPanel />

        <Panel className="grid grid-cols-2 gap-6 p-6">
          <Stat label="Trabalhado hoje" value={formatDuration(todaySeconds)} />
          <Stat label="Estimado (7 dias)" value={formatCurrency(weekAmount)} />
          <Stat label="Sessoes registradas" value={String(entries.length)} />
          <Stat label="Projetos ativos" value={String(projects.length)} />
        </Panel>
      </div>

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <Panel>
          <PanelHeader title="Sessoes recentes" />
          {recent.length === 0 ? (
            <div className="p-4">
              <EmptyState
                title="Nenhuma sessao ainda"
                description="Inicie o cronometro para registrar sua primeira sessao."
              />
            </div>
          ) : (
            <ul className="divide-y divide-border">
              {recent.map((entry) => (
                <li
                  key={entry.id}
                  className="flex items-center justify-between px-4 py-3"
                >
                  <div className="min-w-0">
                    <p className="truncate text-sm text-text">
                      {projectName(entry.projectId)}
                    </p>
                    <p className="text-xs text-text-muted">
                      {ACTIVITY_TYPE_LABELS[entry.activityType]}
                      {entry.description ? ` · ${entry.description}` : ""}
                    </p>
                  </div>
                  <span className="tabular ml-3 shrink-0 text-sm text-text-muted">
                    {formatDuration(entry.durationSeconds)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel>
          <PanelHeader title="Linha do tempo detectada" />
          <div className="flex items-start gap-3 px-4 py-4">
            <CalendarClock
              size={18}
              strokeWidth={1.75}
              className="mt-0.5 shrink-0 text-text-subtle"
            />
            <div className="text-sm">
              <p className="text-text">
                Veja os programas monitorados abertos hoje e transforme periodos
                sem registro em sessoes.
              </p>
              <Link
                to={ROUTES.timeline}
                className="mt-1.5 inline-block text-sm text-accent hover:underline"
              >
                Abrir linha do tempo
              </Link>
            </div>
          </div>
        </Panel>
      </div>

      <RecoveryModal />
    </div>
  );
}

function WelcomeOnboarding() {
  const steps = [
    "Crie um cliente e um projeto (com valor/hora).",
    "No Painel, escolha o projeto e clique em Iniciar.",
    "Ao encerrar, veja horas e valores em Historico e Relatorios.",
  ];
  return (
    <Panel className="mb-4 border-accent/40 p-6">
      <p className="text-sm font-semibold text-text">Bem-vindo ao CronoCAD 👋</p>
      <p className="mt-1 text-sm text-text-muted">
        Vamos configurar em 3 passos rapidos:
      </p>
      <ol className="mt-3 space-y-1.5">
        {steps.map((step, i) => (
          <li key={step} className="flex gap-2.5 text-sm text-text">
            <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-accent-muted text-2xs font-semibold text-accent">
              {i + 1}
            </span>
            {step}
          </li>
        ))}
      </ol>
      <Link
        to={ROUTES.projects}
        className="mt-4 inline-block rounded bg-accent px-3 py-1.5 text-sm font-medium text-accent-contrast hover:bg-accent-hover"
      >
        Criar meu primeiro projeto
      </Link>
    </Panel>
  );
}

import { useEffect, useMemo, useState } from "react";
import { PenTool } from "lucide-react";
import type { ActivityType, Project } from "@/types/domain";
import {
  hideCurrentWindow,
  listenEvent,
  showCurrentWindow,
} from "@/services/tauri";
import { TauriEvent } from "@/types/events";
import { getPendingReminder } from "@/services/app";
import { listProjects } from "@/services/projects";
import { listTimeEntries } from "@/services/timeEntries";
import { startTimer } from "@/services/timer";
import { suppressAppReminderToday } from "@/services/monitoredApps";
import { Button } from "@/components/ui/Button";
import { Select } from "@/components/ui/Field";

interface OpenInfo {
  processName: string;
  displayName: string;
}

/**
 * Widget flutuante (janela `reminder`) exibido sobre o programa CAD ao abri-lo
 * sem cronometro ativo (secao 10). Renderizado em sua propria janela; conversa
 * com o backend apenas por eventos e comandos. Nao monta o app principal.
 */
export function ReminderWidget() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [recentIds, setRecentIds] = useState<string[]>([]);
  const [projectId, setProjectId] = useState("");
  const [info, setInfo] = useState<OpenInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [activityType] = useState<ActivityType>("drawing");

  // Aplica o tema salvo (a janela abre em dark por padrao).
  useEffect(() => {
    const saved =
      typeof localStorage !== "undefined"
        ? localStorage.getItem("cronocad.theme")
        : null;
    if (saved === "light" || saved === "dark") {
      document.documentElement.setAttribute("data-theme", saved);
    }
  }, []);

  // Carrega projetos e recentes uma vez.
  useEffect(() => {
    void listProjects().then(setProjects).catch(() => setProjects([]));
    void listTimeEntries(50)
      .then((entries) => {
        const seen = new Set<string>();
        const ids: string[] = [];
        for (const e of entries) {
          if (!seen.has(e.projectId)) {
            seen.add(e.projectId);
            ids.push(e.projectId);
          }
        }
        setRecentIds(ids);
      })
      .catch(() => setRecentIds([]));
  }, []);

  // Recupera um lembrete pendente ao carregar (robusto a corrida de startup) e
  // escuta novas aberturas enquanto a janela existe.
  useEffect(() => {
    void getPendingReminder().then((pending) => {
      if (pending) {
        setInfo(pending);
        void showCurrentWindow();
      }
    });

    let unlisten: (() => void) | undefined;
    void listenEvent(TauriEvent.monitoredAppOpened, (payload) => {
      if (!payload.hasActiveTimer) {
        setInfo({
          processName: payload.processName,
          displayName: payload.displayName,
        });
        void showCurrentWindow();
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  // Pre-seleciona o projeto mais recente (ou o primeiro) quando abre.
  const defaultProjectId = useMemo(() => {
    const recent = recentIds.find((id) => projects.some((p) => p.id === id));
    return recent ?? projects[0]?.id ?? "";
  }, [recentIds, projects]);

  useEffect(() => {
    if (info && !projectId) setProjectId(defaultProjectId);
  }, [info, projectId, defaultProjectId]);

  async function close() {
    setInfo(null);
    setProjectId("");
    await hideCurrentWindow();
  }

  async function iniciar() {
    if (!projectId) return;
    setBusy(true);
    try {
      await startTimer({ projectId, activityType, description: null });
      await close();
    } finally {
      setBusy(false);
    }
  }

  async function naoLembrar() {
    if (info) await suppressAppReminderToday(info.processName);
    await close();
  }

  if (!info) {
    // Sem contexto (janela aberta sem evento): mostra um estado neutro.
    return (
      <div className="flex h-screen items-center justify-center bg-surface text-sm text-text-muted">
        CronoCAD
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col bg-surface">
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-border px-4 py-2.5"
      >
        <PenTool size={15} strokeWidth={1.75} className="text-accent" />
        <span className="text-xs font-semibold text-text">
          {info.displayName} foi aberto
        </span>
      </div>

      <div className="flex flex-1 flex-col justify-center gap-3 px-4">
        {projects.length === 0 ? (
          <p className="text-sm text-text-muted">
            Nenhum projeto cadastrado. Abra o CronoCAD para criar um.
          </p>
        ) : (
          <>
            <p className="text-sm text-text">Iniciar cronometro em qual projeto?</p>
            <Select
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
              aria-label="Projeto"
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.code ? `${p.code} · ${p.name}` : p.name}
                </option>
              ))}
            </Select>
          </>
        )}
      </div>

      <div className="flex items-center justify-between gap-2 border-t border-border px-4 py-2.5">
        <Button variant="ghost" size="sm" onClick={() => void naoLembrar()}>
          Nao lembrar hoje
        </Button>
        <div className="flex gap-2">
          <Button variant="secondary" size="sm" onClick={() => void close()}>
            Ignorar
          </Button>
          {projects.length > 0 && (
            <Button
              variant="primary"
              size="sm"
              onClick={() => void iniciar()}
              disabled={busy || !projectId}
            >
              Iniciar
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

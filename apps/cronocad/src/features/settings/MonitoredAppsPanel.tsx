import { useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import type { MonitoredApp } from "@/types/domain";
import { useSettingsStore } from "@/stores/settingsStore";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { MonitoredAppForm } from "./MonitoredAppForm";

/** Gestao dos programas monitorados (secao 10): habilitar, editar, remover. */
export function MonitoredAppsPanel() {
  const apps = useSettingsStore((s) => s.apps);
  const editApp = useSettingsStore((s) => s.editApp);
  const removeApp = useSettingsStore((s) => s.removeApp);

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<MonitoredApp | null>(null);

  function toInput(app: MonitoredApp, enabled: boolean) {
    return {
      displayName: app.displayName,
      processName: app.processName,
      enabled,
      remindOnOpen: app.remindOnOpen,
      remindOnClose: app.remindOnClose,
    };
  }

  return (
    <Panel>
      <PanelHeader
        title="Programas monitorados"
        action={
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              setEditing(null);
              setFormOpen(true);
            }}
            icon={<Plus size={15} strokeWidth={1.75} />}
          >
            Adicionar
          </Button>
        }
      />
      {apps.length === 0 ? (
        <p className="px-4 py-6 text-sm text-text-muted">
          Nenhum programa cadastrado. Adicione, por exemplo, o AutoCAD
          (acad.exe).
        </p>
      ) : (
        <ul className="divide-y divide-border">
          {apps.map((app) => (
            <li key={app.id} className="flex items-center justify-between px-4 py-3">
              <div className="flex items-center gap-3">
                <Checkbox
                  label=""
                  ariaLabel={`Monitorar ${app.displayName}`}
                  checked={app.enabled}
                  onChange={(enabled) =>
                    void editApp(app.id, toInput(app, enabled))
                  }
                />
                <div>
                  <p className="text-sm text-text">{app.displayName}</p>
                  <p className="tabular text-xs text-text-muted">
                    {app.processName}
                  </p>
                </div>
              </div>
              <div className="flex shrink-0 gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setEditing(app);
                    setFormOpen(true);
                  }}
                  aria-label={`Editar ${app.displayName}`}
                  icon={<Pencil size={15} strokeWidth={1.75} />}
                />
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void removeApp(app.id)}
                  aria-label={`Remover ${app.displayName}`}
                  icon={<Trash2 size={15} strokeWidth={1.75} />}
                />
              </div>
            </li>
          ))}
        </ul>
      )}

      <MonitoredAppForm
        open={formOpen}
        app={editing}
        onClose={() => setFormOpen(false)}
      />
    </Panel>
  );
}

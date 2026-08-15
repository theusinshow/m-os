import { useState, type FormEvent } from "react";
import type { MonitoredApp } from "@/types/domain";
import { useSettingsStore } from "@/stores/settingsStore";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Field, Input } from "@/components/ui/Field";
import { Checkbox } from "@/components/ui/Checkbox";

interface MonitoredAppFormProps {
  open: boolean;
  app: MonitoredApp | null;
  onClose: () => void;
}

/** Formulario de criacao/edicao de programa monitorado. */
export function MonitoredAppForm({ open, app, onClose }: MonitoredAppFormProps) {
  const addApp = useSettingsStore((s) => s.addApp);
  const editApp = useSettingsStore((s) => s.editApp);

  const [displayName, setDisplayName] = useState("");
  const [processName, setProcessName] = useState("");
  const [remindOnOpen, setRemindOnOpen] = useState(true);
  const [remindOnClose, setRemindOnClose] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [initializedFor, setInitializedFor] = useState<string | null>(null);
  const key = app?.id ?? "new";
  if (open && initializedFor !== key) {
    setDisplayName(app?.displayName ?? "");
    setProcessName(app?.processName ?? "");
    setRemindOnOpen(app?.remindOnOpen ?? true);
    setRemindOnClose(app?.remindOnClose ?? true);
    setError(null);
    setInitializedFor(key);
  }
  if (!open && initializedFor !== null) setInitializedFor(null);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const input = {
        displayName,
        processName,
        enabled: app?.enabled ?? true,
        remindOnOpen,
        remindOnClose,
      };
      if (app) {
        await editApp(app.id, input);
      } else {
        await addApp(input);
      }
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : "Falha ao salvar.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      title={app ? "Editar programa" : "Adicionar programa"}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} type="button">
            Cancelar
          </Button>
          <Button
            variant="primary"
            type="submit"
            form="app-form"
            disabled={saving}
          >
            {saving ? "Salvando…" : "Salvar"}
          </Button>
        </>
      }
    >
      <form id="app-form" onSubmit={handleSubmit} className="space-y-4">
        <Field label="Nome de exibicao" htmlFor="a-name" required>
          <Input
            id="a-name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="AutoCAD"
            autoFocus
            required
          />
        </Field>
        <Field
          label="Executavel"
          htmlFor="a-proc"
          hint="Nome do processo no Windows (ex.: acad.exe)."
          required
        >
          <Input
            id="a-proc"
            value={processName}
            onChange={(e) => setProcessName(e.target.value)}
            placeholder="acad.exe"
            required
          />
        </Field>
        <Checkbox
          label="Lembrar ao abrir"
          checked={remindOnOpen}
          onChange={setRemindOnOpen}
        />
        <Checkbox
          label="Lembrar ao fechar"
          checked={remindOnClose}
          onChange={setRemindOnClose}
        />
        {error && <p className="text-sm text-danger">{error}</p>}
      </form>
    </Modal>
  );
}

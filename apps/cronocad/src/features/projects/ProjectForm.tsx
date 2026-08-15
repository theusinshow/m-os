import { useState, type FormEvent } from "react";
import type { Project } from "@/types/domain";
import { useCatalogStore } from "@/stores/catalogStore";
import { fromCents, toCents } from "@/lib/money";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Field, Input, Select, Textarea } from "@/components/ui/Field";

interface ProjectFormProps {
  open: boolean;
  project: Project | null;
  onClose: () => void;
}

/** Formulario de criacao/edicao de projeto. */
export function ProjectForm({ open, project, onClose }: ProjectFormProps) {
  const clients = useCatalogStore((s) => s.clients);
  const createProject = useCatalogStore((s) => s.createProject);
  const updateProject = useCatalogStore((s) => s.updateProject);

  const [name, setName] = useState("");
  const [code, setCode] = useState("");
  const [clientId, setClientId] = useState("");
  const [rate, setRate] = useState("0");
  const [budgetHours, setBudgetHours] = useState("0");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [initializedFor, setInitializedFor] = useState<string | null>(null);
  const key = project?.id ?? "new";
  if (open && initializedFor !== key) {
    setName(project?.name ?? "");
    setCode(project?.code ?? "");
    setClientId(project?.clientId ?? "");
    setRate(project ? String(fromCents(project.hourlyRateCents)) : "0");
    setBudgetHours(
      project && project.budgetMinutes > 0
        ? String(project.budgetMinutes / 60)
        : "0",
    );
    setDescription(project?.description ?? "");
    setError(null);
    setInitializedFor(key);
  }
  if (!open && initializedFor !== null) {
    setInitializedFor(null);
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const parsedRate = Number(rate.replace(",", "."));
      if (Number.isNaN(parsedRate) || parsedRate < 0) {
        throw "Valor/hora invalido.";
      }
      const parsedBudget = Math.max(0, Number(budgetHours.replace(",", ".")) || 0);
      const input = {
        clientId: clientId || null,
        name,
        code: code || null,
        description: description || null,
        hourlyRateCents: toCents(parsedRate),
        budgetMinutes: Math.round(parsedBudget * 60),
        color: project?.color ?? null,
      };
      if (project) {
        await updateProject(project.id, input);
      } else {
        await createProject(input);
      }
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : "Falha ao salvar o projeto.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      title={project ? "Editar projeto" : "Novo projeto"}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} type="button">
            Cancelar
          </Button>
          <Button
            variant="primary"
            type="submit"
            form="project-form"
            disabled={saving}
          >
            {saving ? "Salvando…" : "Salvar"}
          </Button>
        </>
      }
    >
      <form id="project-form" onSubmit={handleSubmit} className="space-y-4">
        <Field label="Nome" htmlFor="p-name" required>
          <Input
            id="p-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            required
          />
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Codigo" htmlFor="p-code" hint="Ex.: 083-22">
            <Input
              id="p-code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
            />
          </Field>
          <Field label="Valor/hora (R$)" htmlFor="p-rate" required>
            <Input
              id="p-rate"
              inputMode="decimal"
              value={rate}
              onChange={(e) => setRate(e.target.value)}
            />
          </Field>
        </div>
        <Field
          label="Meta de horas"
          htmlFor="p-budget"
          hint="Opcional. 0 = sem meta. Usada para acompanhamento e alerta."
        >
          <Input
            id="p-budget"
            inputMode="decimal"
            value={budgetHours}
            onChange={(e) => setBudgetHours(e.target.value)}
          />
        </Field>
        <Field label="Cliente" htmlFor="p-client">
          <Select
            id="p-client"
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
          >
            <option value="">Sem cliente</option>
            {clients.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Descricao" htmlFor="p-desc">
          <Textarea
            id="p-desc"
            rows={3}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </Field>
        {error && <p className="text-sm text-danger">{error}</p>}
      </form>
    </Modal>
  );
}

import { useState, type FormEvent } from "react";
import type { Client } from "@/types/domain";
import { useCatalogStore } from "@/stores/catalogStore";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Field, Input, Textarea } from "@/components/ui/Field";

interface ClientFormProps {
  open: boolean;
  client: Client | null;
  onClose: () => void;
}

/** Formulario de criacao/edicao de cliente. */
export function ClientForm({ open, client, onClose }: ClientFormProps) {
  const createClient = useCatalogStore((s) => s.createClient);
  const updateClient = useCatalogStore((s) => s.updateClient);

  const [name, setName] = useState("");
  const [companyName, setCompanyName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Reinicia os campos quando o modal abre (para criar ou editar).
  const [initializedFor, setInitializedFor] = useState<string | null>(null);
  const key = client?.id ?? "new";
  if (open && initializedFor !== key) {
    setName(client?.name ?? "");
    setCompanyName(client?.companyName ?? "");
    setEmail(client?.email ?? "");
    setPhone(client?.phone ?? "");
    setNotes(client?.notes ?? "");
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
      const input = {
        name,
        companyName: companyName || null,
        email: email || null,
        phone: phone || null,
        notes: notes || null,
      };
      if (client) {
        await updateClient(client.id, input);
      } else {
        await createClient(input);
      }
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : "Falha ao salvar o cliente.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      title={client ? "Editar cliente" : "Novo cliente"}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} type="button">
            Cancelar
          </Button>
          <Button
            variant="primary"
            type="submit"
            form="client-form"
            disabled={saving}
          >
            {saving ? "Salvando…" : "Salvar"}
          </Button>
        </>
      }
    >
      <form id="client-form" onSubmit={handleSubmit} className="space-y-4">
        <Field label="Nome" htmlFor="c-name" required>
          <Input
            id="c-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            required
          />
        </Field>
        <Field label="Empresa" htmlFor="c-company">
          <Input
            id="c-company"
            value={companyName}
            onChange={(e) => setCompanyName(e.target.value)}
          />
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="E-mail" htmlFor="c-email">
            <Input
              id="c-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
          </Field>
          <Field label="Telefone" htmlFor="c-phone">
            <Input
              id="c-phone"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
            />
          </Field>
        </div>
        <Field label="Observacoes" htmlFor="c-notes">
          <Textarea
            id="c-notes"
            rows={3}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
          />
        </Field>
        {error && <p className="text-sm text-danger">{error}</p>}
      </form>
    </Modal>
  );
}

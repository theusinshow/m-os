import { useState } from "react";
import { Archive, Pencil, Plus } from "lucide-react";
import type { Client } from "@/types/domain";
import { useCatalogStore } from "@/stores/catalogStore";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { ClientForm } from "./ClientForm";

interface ClientsModalProps {
  open: boolean;
  onClose: () => void;
}

/** Gestao de clientes (lista + criar/editar/arquivar). */
export function ClientsModal({ open, onClose }: ClientsModalProps) {
  const clients = useCatalogStore((s) => s.clients);
  const archiveClient = useCatalogStore((s) => s.archiveClient);

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Client | null>(null);

  function openNew() {
    setEditing(null);
    setFormOpen(true);
  }
  function openEdit(client: Client) {
    setEditing(client);
    setFormOpen(true);
  }

  return (
    <>
      <Modal
        open={open && !formOpen}
        title="Clientes"
        onClose={onClose}
        footer={
          <Button
            variant="primary"
            onClick={openNew}
            icon={<Plus size={16} strokeWidth={2} />}
          >
            Novo cliente
          </Button>
        }
      >
        {clients.length === 0 ? (
          <EmptyState
            title="Nenhum cliente cadastrado"
            description="Cadastre um cliente para associar aos projetos."
          />
        ) : (
          <ul className="divide-y divide-border">
            {clients.map((client) => (
              <li
                key={client.id}
                className="flex items-center justify-between py-2.5"
              >
                <div className="min-w-0">
                  <p className="truncate text-sm text-text">{client.name}</p>
                  <p className="truncate text-xs text-text-muted">
                    {client.companyName ?? client.email ?? "—"}
                  </p>
                </div>
                <div className="flex shrink-0 gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => openEdit(client)}
                    aria-label={`Editar ${client.name}`}
                    icon={<Pencil size={15} strokeWidth={1.75} />}
                  />
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => archiveClient(client.id)}
                    aria-label={`Arquivar ${client.name}`}
                    icon={<Archive size={15} strokeWidth={1.75} />}
                  />
                </div>
              </li>
            ))}
          </ul>
        )}
      </Modal>

      <ClientForm
        open={formOpen}
        client={editing}
        onClose={() => setFormOpen(false)}
      />
    </>
  );
}

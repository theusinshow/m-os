import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Card, EmptyState } from "./Surface";
import type { Client, ClientInput } from "./types";

const EMPTY: ClientInput = { name: "", companyName: "", email: "", phone: "", notes: "" };

/**
 * Clientes.
 *
 * Existe para a fatura: é daqui que sai o cabeçalho do PDF e o agrupamento de
 * horas por quem paga. Project pessoal não tem cliente, e essa ausência é o caso
 * comum — por isso a tela não empurra o cadastro, ela espera.
 */
export function TempoClients({ onChanged }: { onChanged?: () => void }) {
  const [clients, setClients] = useState<Client[]>([]);
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState<ClientInput>(EMPTY);
  const [open, setOpen] = useState(false);
  const [note, setNote] = useState("");

  const load = useCallback(async () => {
    setClients(await api.clients(showArchived).catch(() => []));
  }, [showArchived]);

  useEffect(() => { void load(); }, [load]);

  async function guard(run: () => Promise<unknown>) {
    setNote("");
    try {
      await run();
      await load();
      onChanged?.();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  function startNew() {
    setEditing(null);
    setDraft(EMPTY);
    setOpen(true);
  }

  function startEdit(client: Client) {
    setEditing(client.id);
    setDraft({
      name: client.name,
      companyName: client.companyName,
      email: client.email,
      phone: client.phone,
      notes: client.notes,
    });
    setOpen(true);
  }

  return (
    <Card
      label="CLIENTES"
      count={clients.length ? String(clients.length) : undefined}
      action={<Button variant="ghost" size="sm" onClick={startNew}>Novo</Button>}
    >
      {open ? (
        <form
          className="tempo-form"
          onSubmit={(event) => {
            event.preventDefault();
            void guard(async () => {
              await api.saveClient(editing, draft);
              setOpen(false);
            });
          }}
        >
          <div className="tempo-field">
            <label htmlFor="client-name">Nome</label>
            <input
              id="client-name"
              value={draft.name}
              autoFocus
              onChange={(event) => setDraft({ ...draft, name: event.currentTarget.value })}
            />
          </div>
          <div className="tempo-field">
            <label htmlFor="client-company">Empresa</label>
            <input
              id="client-company"
              value={draft.companyName}
              placeholder="opcional"
              onChange={(event) => setDraft({ ...draft, companyName: event.currentTarget.value })}
            />
          </div>
          <div className="tempo-field">
            <label htmlFor="client-email">E-mail</label>
            <input
              id="client-email"
              type="email"
              value={draft.email}
              placeholder="opcional"
              onChange={(event) => setDraft({ ...draft, email: event.currentTarget.value })}
            />
          </div>
          <div className="tempo-field">
            <label htmlFor="client-phone">Telefone</label>
            <input
              id="client-phone"
              value={draft.phone}
              placeholder="opcional"
              onChange={(event) => setDraft({ ...draft, phone: event.currentTarget.value })}
            />
          </div>
          <div className="form-actions">
            <Button variant="ghost" onClick={() => setOpen(false)}>Cancelar</Button>
            <Button variant="primary" type="submit" disabled={!draft.name.trim()}>
              {editing ? "Salvar" : "Criar"}
            </Button>
          </div>
        </form>
      ) : null}

      {clients.length ? (
        <div className="tempo-sessions">
          {clients.map((client) => (
            <div className="tempo-session" key={client.id} data-archived={client.archived || undefined}>
              <span>
                <strong>{client.name}</strong>
                <small>
                  {[client.companyName, client.email, client.phone].filter(Boolean).join(" · ") || "sem contato"}
                  {client.archived ? " · arquivado" : ""}
                </small>
              </span>
              <span className="tempo-session-actions">
                <Button variant="ghost" size="sm" onClick={() => startEdit(client)}>Editar</Button>
                {/* Arquivar e não excluir: o cliente pode estar em faturas já
                    emitidas, e removê-lo deixaria horas apontando para um
                    pagador que sumiu. */}
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void guard(() => api.setClientArchived(client.id, !client.archived))}
                >
                  {client.archived ? "Restaurar" : "Arquivar"}
                </Button>
              </span>
            </div>
          ))}
        </div>
      ) : (
        <EmptyState>Nenhum cliente. Eles existem para a fatura — um Project pessoal não precisa de um.</EmptyState>
      )}

      <label className="tempo-check">
        <input
          type="checkbox"
          checked={showArchived}
          onChange={(event) => setShowArchived(event.currentTarget.checked)}
        />
        Mostrar arquivados
      </label>
      {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
    </Card>
  );
}

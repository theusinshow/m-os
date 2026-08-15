/**
 * Servico de clientes: wrappers tipados sobre os comandos Tauri.
 * Nenhum SQL no frontend — apenas chamadas a comandos especificos (secao 19).
 */

import type { Client } from "@/types/domain";
import { invokeCommand } from "./tauri";

export interface ClientInput {
  name: string;
  companyName?: string | null;
  email?: string | null;
  phone?: string | null;
  notes?: string | null;
}

export function listClients(includeArchived = false): Promise<Client[]> {
  return invokeCommand<Client[]>("list_clients", { includeArchived });
}

export function getClient(id: string): Promise<Client> {
  return invokeCommand<Client>("get_client", { id });
}

export function createClient(input: ClientInput): Promise<Client> {
  return invokeCommand<Client>("create_client", { input });
}

export function updateClient(id: string, input: ClientInput): Promise<Client> {
  return invokeCommand<Client>("update_client", { id, input });
}

export function archiveClient(id: string): Promise<Client> {
  return invokeCommand<Client>("archive_client", { id });
}

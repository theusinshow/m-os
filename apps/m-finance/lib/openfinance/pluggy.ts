import { PluggyClient } from "pluggy-sdk";
import type { Account, AccountType, Transaction } from "pluggy-sdk";
import { env, isPluggyConfigured } from "@/lib/env";

/**
 * Thin wrapper around the official Pluggy SDK for the Open Finance spike.
 *
 * The SDK handles authentication (client credentials -> API key) and refresh
 * internally, so we only expose the few calls our sync needs. Sandbox uses the
 * same API — the sandbox *connectors* are the fake banks.
 *
 * Docs: https://docs.pluggy.ai
 */

export type PluggyAccount = Account;
export type PluggyTransaction = Transaction;

let client: PluggyClient | null = null;

function getClient(): PluggyClient {
  if (!isPluggyConfigured()) {
    throw new Error("Pluggy não configurado (PLUGGY_CLIENT_ID/SECRET).");
  }
  if (!client) {
    client = new PluggyClient({
      clientId: env.pluggyClientId,
      clientSecret: env.pluggyClientSecret,
    });
  }
  return client;
}

/**
 * Mints a connect token consumed by the Pluggy Connect widget on the frontend.
 * - `itemId` reopens an existing connection for re-consent/update.
 * - `clientUserId` ties the item to our app user (shown in the dashboard and
 *   useful to avoid duplicate connections per user).
 * - `webhookUrl` registers where Pluggy posts item/transaction events.
 */
export async function createConnectToken(options?: {
  itemId?: string;
  clientUserId?: string;
  webhookUrl?: string;
}): Promise<string> {
  const { accessToken } = await getClient().createConnectToken(options?.itemId, {
    clientUserId: options?.clientUserId,
    webhookUrl: options?.webhookUrl,
  });
  return accessToken;
}

/** Lists the accounts of a connected item, optionally filtered by type. */
export async function listAccounts(itemId: string, type?: AccountType) {
  const { results } = await getClient().fetchAccounts(itemId, type);
  return results;
}

/** Reads transactions for an account since `from` (ISO yyyy-mm-dd), paginated. */
export async function listTransactions(accountId: string, from: string) {
  const pluggy = getClient();
  const out: Transaction[] = [];
  let page = 1;
  // pageSize caps at 500; loop until we drain all pages.
  for (;;) {
    const data = await pluggy.fetchTransactions(accountId, { from, page, pageSize: 500 });
    out.push(...data.results);
    if (page >= data.totalPages || data.results.length === 0) break;
    page += 1;
  }
  return out;
}

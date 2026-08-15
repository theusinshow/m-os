"use client";

import { useState, useTransition } from "react";
import dynamic from "next/dynamic";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { linkPluggyAccountToCard, registerPluggyItem } from "@/app/actions/openfinance";

// The widget touches `window`, so load it client-only.
const PluggyConnect = dynamic(
  () => import("react-pluggy-connect").then((m) => m.PluggyConnect),
  { ssr: false },
);

// Pluggy "Pluggy Bank" sandbox connector — exposes a fake credit card with
// transactions. A trial Pluggy plan can only create items for sandbox
// connectors (real banks return TRIAL_CLIENT_ITEM_CREATE_NOT_ALLOWED), so we
// pin the widget to it. Remove this on a production plan to list real banks.
const SANDBOX_CONNECTOR_IDS = [2];

type CardOption = { id: string; name: string };
type PendingAccount = { id: string; name: string };

export function ConnectBank({ cards }: { cards: CardOption[] }) {
  const router = useRouter();
  const { addToast } = useToast();
  const [pending, startTransition] = useTransition();

  const [token, setToken] = useState<string | null>(null);
  const [loadingToken, setLoadingToken] = useState(false);
  // After a successful connection we collect the accounts to map to cards.
  const [linking, setLinking] = useState<{ itemId: string; accounts: PendingAccount[] } | null>(
    null,
  );
  const [choice, setChoice] = useState<Record<string, string>>({});

  async function openWidget() {
    setLoadingToken(true);
    try {
      const res = await fetch("/api/openfinance/connect-token", { method: "POST" });
      const data = (await res.json()) as { accessToken?: string; error?: string };
      if (!res.ok || !data.accessToken) {
        throw new Error(data.error ?? "Falha ao gerar token.");
      }
      setToken(data.accessToken);
    } catch (error) {
      addToast(error instanceof Error ? error.message : "Erro ao conectar.", "error");
    } finally {
      setLoadingToken(false);
    }
  }

  function handleSuccess(itemId: string) {
    setToken(null);
    addToast("Conexão recebida, lendo as contas…", "info");
    startTransition(async () => {
      const result = await registerPluggyItem(itemId);
      if (!result.ok) {
        addToast(result.error, "error");
        return;
      }
      setLinking({ itemId, accounts: result.accounts });
      addToast("Banco conectado. Vincule cada conta a um cartão.", "success");
    });
  }

  function linkAccount(accountId: string) {
    if (!linking) return;
    const cardId = choice[accountId];
    if (!cardId) {
      addToast("Escolha um cartão.", "error");
      return;
    }
    startTransition(async () => {
      const result = await linkPluggyAccountToCard(linking.itemId, accountId, cardId);
      if (!result.ok) {
        addToast(result.error, "error");
        return;
      }
      const imported = result.results?.reduce((n, r) => n + r.imported, 0) ?? 0;
      addToast(`Vinculado. ${imported} compra(s) importada(s).`, "success");
      const rest = linking.accounts.filter((a) => a.id !== accountId);
      if (rest.length === 0) {
        setLinking(null);
        router.refresh();
      } else {
        setLinking({ ...linking, accounts: rest });
      }
    });
  }

  if (cards.length === 0) {
    return (
      <p className="text-sm leading-6 text-text-muted">
        Cadastre um cartão antes de conectar um banco — as compras importadas precisam de um cartão
        de destino.
      </p>
    );
  }

  return (
    <div className="space-y-4">
      {linking ? (
        <div className="space-y-3">
          <p className="text-sm text-text-secondary">
            Vincule cada conta de cartão à sua versão no app:
          </p>
          {linking.accounts.map((account) => (
            <div
              className="flex flex-wrap items-center gap-2 rounded-lg border border-border-subtle bg-background-elevated px-4 py-2.5"
              key={account.id}
            >
              <span className="min-w-0 flex-1 truncate text-sm font-medium text-text-primary">
                {account.name}
              </span>
              <select
                className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary"
                onChange={(e) => setChoice((c) => ({ ...c, [account.id]: e.target.value }))}
                value={choice[account.id] ?? ""}
              >
                <option value="">Escolha o cartão…</option>
                {cards.map((card) => (
                  <option key={card.id} value={card.id}>
                    {card.name}
                  </option>
                ))}
              </select>
              <Button disabled={pending} onClick={() => linkAccount(account.id)} type="button">
                {pending ? "Importando…" : "Vincular"}
              </Button>
            </div>
          ))}
        </div>
      ) : (
        <Button
          className="w-fit"
          disabled={loadingToken || pending}
          onClick={openWidget}
          type="button"
        >
          {loadingToken ? "Abrindo…" : "Conectar banco"}
        </Button>
      )}

      {token ? (
        <PluggyConnect
          connectToken={token}
          connectorIds={SANDBOX_CONNECTOR_IDS}
          includeSandbox
          theme="dark"
          onClose={() => setToken(null)}
          onError={(error) => {
            setToken(null);
            addToast(error.message ?? "Erro na conexão.", "error");
          }}
          onSuccess={(data) => handleSuccess(data.item.id)}
        />
      ) : null}
    </div>
  );
}

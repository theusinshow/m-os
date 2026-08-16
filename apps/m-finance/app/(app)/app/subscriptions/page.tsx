import { cancelSubscription, deleteSubscription } from "@/app/actions/subscriptions";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { PageHeading } from "@/components/page-heading";
import { SubscriptionFormDrawer } from "@/components/subscriptions/subscription-form-drawer";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getSubscriptionsForUser } from "@/lib/subscriptions";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";

const cycleLabel: Record<string, string> = {
  once: "Cobrança única",
  monthly: "Mensal",
  yearly: "Anual",
};

const statusLabel: Record<string, { text: string; className: string }> = {
  trial: { text: "Teste grátis", className: "text-status-fair" },
  active: { text: "Ativa", className: "text-status-positive" },
  canceled: { text: "Cancelada", className: "text-text-muted" },
};

export default async function SubscriptionsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const subscriptions = appUser ? await getSubscriptionsForUser(appUser.id) : [];
  const live = subscriptions.filter((s) => s.status !== "canceled");
  const canceled = subscriptions.filter((s) => s.status === "canceled");

  // Custo mensal equivalente: anuais divididos por 12, cobranças únicas ficam de fora.
  const monthlyCostCents = live.reduce((total, sub) => {
    if (sub.cycle === "monthly") return total + sub.amountCents;
    if (sub.cycle === "yearly") return total + Math.round(sub.amountCents / 12);
    return total;
  }, 0);

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Assinaturas" title="Assinaturas & testes grátis">
        <SubscriptionFormDrawer />
      </PageHeading>

      <DashboardCard
        action={
          live.length > 0 ? (
            <p className="text-right">
              <span className="block text-[11px] font-semibold uppercase tracking-[0.14em] text-text-muted">
                Custo mensal
              </span>
              <span className="num text-sm font-semibold text-text-primary">
                {formatCurrency(monthlyCostCents)}
              </span>
            </p>
          ) : undefined
        }
        title="Suas assinaturas"
      >
        {live.length === 0 ? (
          <InlineEmpty>
            Nenhuma assinatura cadastrada. Adicione streaming, software e testes grátis para ser
            avisado antes de cada cobrança.
          </InlineEmpty>
        ) : (
          <div className="space-y-2">
            {live.map((sub) => {
              const status = statusLabel[sub.status];
              return (
                <div
                  className="flex flex-wrap items-center gap-3 rounded-md border border-border-subtle bg-background-elevated px-4 py-3"
                  key={sub.id}
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-semibold text-text-primary">
                        {sub.name}
                      </span>
                      <span
                        className={`rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] ${status.className}`}
                      >
                        {status.text}
                      </span>
                    </div>
                    <p className="mt-1 text-xs text-text-muted">
                      <span className="num">{formatCurrency(sub.amountCents)}</span> ·{" "}
                      {cycleLabel[sub.cycle]} ·{" "}
                      {sub.status === "trial" ? "cobra em " : "próxima em "}
                      {formatShortDate(sub.nextChargeDate)} · avisa {sub.reminderDaysBefore}d antes
                    </p>
                  </div>
                  <ToastForm action={cancelSubscription} successMessage="Assinatura cancelada.">
                    <input name="id" type="hidden" value={sub.id} />
                    <FormSubmitButton pendingLabel="Cancelando..." variant="secondary">
                      Cancelar
                    </FormSubmitButton>
                  </ToastForm>
                </div>
              );
            })}
          </div>
        )}

        {canceled.length > 0 ? (
          <div className="mt-5 space-y-2 border-t border-border-subtle pt-4">
            <p className="text-xs font-semibold uppercase tracking-[0.12em] text-text-muted">
              Canceladas
            </p>
            {canceled.map((sub) => (
              <div
                className="flex items-center justify-between gap-3 rounded-md border border-border-subtle px-4 py-2.5"
                key={sub.id}
              >
                <span className="truncate text-sm text-text-muted line-through">{sub.name}</span>
                <ToastForm action={deleteSubscription} successMessage="Removida.">
                  <input name="id" type="hidden" value={sub.id} />
                  <ConfirmDeleteButton confirmMessage="Excluir de vez?">Excluir</ConfirmDeleteButton>
                </ToastForm>
              </div>
            ))}
          </div>
        ) : null}
      </DashboardCard>
    </div>
  );
}

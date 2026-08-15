import { addSubscription, cancelSubscription, deleteSubscription } from "@/app/actions/subscriptions";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getSubscriptionsForUser } from "@/lib/subscriptions";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";

const inputClass =
  "focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted";
const labelClass = "mb-2 block text-sm font-medium text-text-secondary";

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

  return (
    <div className="space-y-6">
      <PageHeading
        eyebrow="Assinaturas"
        title="Assinaturas & testes grátis"
      />

      <div className="grid gap-4 xl:grid-cols-[0.8fr_1fr]">
        <DashboardCard title="Nova assinatura / teste grátis">
          <ValidatedForm
            action={addSubscription}
            successMessage="Salvo."
            resetOnSuccess
            className="space-y-4"
          >
            <div>
              <label className={labelClass} htmlFor="sub-name">
                Serviço
              </label>
              <ValidatedInput
                className={inputClass}
                id="sub-name"
                name="name"
                placeholder="Netflix, Spotify, ChatGPT…"
                required
              />
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label className={labelClass} htmlFor="sub-amount">
                  Valor da cobrança
                </label>
                <ValidatedInput
                  className={inputClass}
                  id="sub-amount"
                  inputMode="decimal"
                  name="amount"
                  placeholder="39,90"
                  required
                />
              </div>
              <div>
                <label className={labelClass} htmlFor="sub-date">
                  Data da 1ª cobrança
                </label>
                <ValidatedInput
                  className={inputClass}
                  id="sub-date"
                  name="nextChargeDate"
                  required
                  type="date"
                />
              </div>
            </div>

            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <label className={labelClass} htmlFor="sub-cycle">
                  Recorrência
                </label>
                <ValidatedSelect className={inputClass} defaultValue="monthly" id="sub-cycle" name="cycle">
                  <option value="monthly">Mensal</option>
                  <option value="yearly">Anual</option>
                  <option value="once">Cobrança única</option>
                </ValidatedSelect>
              </div>
              <div>
                <label className={labelClass} htmlFor="sub-reminder">
                  Avisar quantos dias antes
                </label>
                <ValidatedInput
                  className={inputClass}
                  defaultValue={1}
                  id="sub-reminder"
                  inputMode="numeric"
                  max={30}
                  min={0}
                  name="reminderDaysBefore"
                  type="number"
                />
              </div>
            </div>

            <label className="flex items-center gap-2 text-sm text-text-secondary">
              <input className="h-4 w-4 accent-accent" name="isTrial" type="checkbox" />
              É um teste grátis (vai começar a cobrar nessa data)
            </label>

            <FormSubmitButton pendingLabel="Salvando...">Salvar</FormSubmitButton>
          </ValidatedForm>
        </DashboardCard>

        <DashboardCard title="Suas assinaturas">
          {live.length === 0 ? (
            <InlineEmpty>Nenhuma assinatura cadastrada ainda.</InlineEmpty>
          ) : (
            <div className="space-y-2">
              {live.map((sub) => {
                const status = statusLabel[sub.status];
                return (
                  <div
                    className="flex flex-wrap items-center gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
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
                        {formatCurrency(sub.amountCents)} · {cycleLabel[sub.cycle]} ·{" "}
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
                  className="flex items-center justify-between gap-3 rounded-lg border border-border-subtle px-4 py-2.5"
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
    </div>
  );
}

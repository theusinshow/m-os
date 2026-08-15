"use client";

import { Repeat } from "lucide-react";
import { createBill } from "@/app/actions/bills";
import { CategoryChips } from "@/components/bills/category-chips";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { formatCurrency } from "@/lib/formatters/currency";

type Category = { id: string; name: string };

export function QuickAddExpense({
  categories,
  totalPendingCents,
  pendingCount,
  paidCount,
}: {
  categories: Category[];
  totalPendingCents: number;
  pendingCount: number;
  paidCount: number;
}) {
  return (
    <div>
      <div className="flex flex-wrap items-end justify-between gap-x-6 gap-y-2">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.16em] text-text-muted">
            A pagar neste mês
          </p>
          <p className="num mt-2 text-4xl font-semibold text-text-primary">
            {formatCurrency(totalPendingCents)}
          </p>
        </div>
        <p className="pb-1 text-sm text-text-muted">
          {pendingCount} em aberto · {paidCount} paga{paidCount === 1 ? "" : "s"}
        </p>
      </div>

      <ValidatedForm
        action={createBill}
        successMessage="Despesa adicionada."
        resetOnSuccess
        className="mt-6 border-t border-border-subtle pt-6"
      >
        {/* Amount is the hero: oversized tabular figures with a quiet R$ lead-in. */}
        <div className="grid gap-4 md:grid-cols-[minmax(0,14rem)_1fr] md:items-start">
          <div>
            <label
              className="mb-2 block text-xs font-semibold uppercase tracking-[0.14em] text-text-muted"
              htmlFor="quick-amount"
            >
              Valor
            </label>
            <div className="relative">
              <span className="num pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-lg font-semibold text-text-muted">
                R$
              </span>
              <ValidatedInput
                autoComplete="off"
                className="num focus-ring h-14 w-full rounded-lg border border-border-subtle bg-background-elevated pl-11 pr-3 text-2xl font-semibold text-text-primary placeholder:text-text-muted/60"
                id="quick-amount"
                inputMode="decimal"
                name="amount"
                placeholder="0,00"
                required
              />
            </div>
          </div>

          <div>
            <label
              className="mb-2 block text-xs font-semibold uppercase tracking-[0.14em] text-text-muted"
              htmlFor="quick-name"
            >
              Despesa
            </label>
            <ValidatedInput
              autoComplete="off"
              className="focus-ring h-14 w-full rounded-lg border border-border-subtle bg-background-elevated px-3.5 text-base text-text-primary placeholder:text-text-muted/60"
              id="quick-name"
              name="name"
              placeholder="Internet, aluguel, cartão…"
              required
            />
          </div>
        </div>

        {categories.length > 0 ? (
          <div className="mt-5">
            <p className="mb-2 text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
              Categoria
            </p>
            <CategoryChips categories={categories} />
          </div>
        ) : null}

        <div className="mt-5 flex flex-wrap items-center gap-3">
          {/* Recurring as a pill toggle, not a stray checkbox. */}
          <label className="group focus-within:[&>span]:ring-2 focus-within:[&>span]:ring-accent/40">
            <input className="peer sr-only" name="isRecurring" type="checkbox" />
            <span className="inline-flex min-h-11 cursor-pointer items-center gap-2 rounded-full border border-border-subtle bg-background-elevated px-4 text-sm font-medium text-text-muted transition duration-150 hover:border-border-default peer-checked:border-accent-border peer-checked:bg-accent-soft peer-checked:text-accent">
              <Repeat size={15} aria-hidden="true" />
              Recorrente
            </span>
          </label>

          <div className="flex items-center gap-2">
            <label className="text-sm text-text-muted" htmlFor="quick-due-day">
              Vence dia
            </label>
            <ValidatedInput
              aria-label="Dia do vencimento"
              className="focus-ring num h-11 w-20 rounded-lg border border-border-subtle bg-background-elevated px-3 text-center text-sm text-text-primary placeholder:text-text-muted/60"
              id="quick-due-day"
              inputMode="numeric"
              max={31}
              min={1}
              name="dueDay"
              placeholder="00"
              type="number"
            />
          </div>

          <div className="ml-auto">
            <FormSubmitButton pendingLabel="Adicionando...">Adicionar despesa</FormSubmitButton>
          </div>
        </div>

        <p className="mt-3 text-xs leading-5 text-text-muted">
          Sem dia informado, a despesa vence no fim do mês.
        </p>
      </ValidatedForm>
    </div>
  );
}

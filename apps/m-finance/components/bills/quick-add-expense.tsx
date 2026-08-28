"use client";

import { useState } from "react";
import { createBill } from "@/app/actions/bills";
import { CategoryChips } from "@/components/bills/category-chips";
import { FormSubmitButton } from "@/components/form-submit-button";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";
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
  const [scheduleType, setScheduleType] = useState<"once" | "fixed" | "ongoing">("once");

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
        successMessage="Conta adicionada."
        resetOnSuccess
        // `scheduleType` mora aqui em cima, fora do alcance do sinal de reset.
        // Sem isto, uma conta "Recorrente, sem fim" deixava a proxima recorrente
        // tambem — e a proxima nao gera uma linha, gera doze.
        onSuccess={() => setScheduleType("once")}
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
              Conta
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
          <div className="flex items-center gap-2">
            <label className="text-sm text-text-muted" htmlFor="quick-schedule">
              Repetição
            </label>
            <ValidatedSelect
              className="field-input"
              id="quick-schedule"
              name="scheduleType"
              onChange={(event) =>
                setScheduleType(event.target.value as "once" | "fixed" | "ongoing")
              }
              value={scheduleType}
            >
              <option value="once">Somente este mês</option>
              <option value="fixed">Por alguns meses</option>
              <option value="ongoing">Recorrente, sem fim</option>
            </ValidatedSelect>
          </div>

          {scheduleType === "fixed" ? (
            <div className="flex items-center gap-2">
              <label className="text-sm text-text-muted" htmlFor="quick-repeat-months">
                Durante
              </label>
              <ValidatedInput
                aria-label="Quantidade de meses"
                className="focus-ring num h-11 w-20 rounded-lg border border-border-subtle bg-background-elevated px-3 text-center text-sm text-text-primary"
                defaultValue={2}
                id="quick-repeat-months"
                inputMode="numeric"
                max={60}
                min={2}
                name="repeatMonths"
                required
                type="number"
              />
              <span className="text-sm text-text-muted">meses</span>
            </div>
          ) : null}

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
            <FormSubmitButton pendingLabel="Adicionando...">Adicionar conta</FormSubmitButton>
          </div>
        </div>

        <p className="mt-3 text-xs leading-5 text-text-muted">
          Sem dia informado, a conta vence no fim do mês. Séries começam no mês selecionado.
        </p>
      </ValidatedForm>
    </div>
  );
}

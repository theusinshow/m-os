"use client";

import { useState } from "react";
import { createSimulation } from "@/app/actions/simulations";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { monthName } from "@/lib/calculations/simulator";

const fieldClass =
  "focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted";

export function SimulatorForm({
  defaultMonth,
  defaultYear,
}: {
  defaultMonth: number;
  defaultYear: number;
}) {
  const [paymentType, setPaymentType] = useState<"cash" | "installment">("installment");
  const years = [defaultYear, defaultYear + 1];

  return (
    <DashboardCard
      description="Informe a compra e veja o impacto projetado nos próximos meses."
      title="Simular compra"
    >
      <ValidatedForm
        action={createSimulation}
        successMessage="Simulação salva."
        resetOnSuccess
        className="grid gap-4 lg:grid-cols-2"
      >
        <div className="lg:col-span-2">
          <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="sim-name">
            O que você quer comprar
          </label>
          <ValidatedInput
            className={fieldClass}
            id="sim-name"
            name="name"
            placeholder="Notebook novo"
            required
          />
        </div>

        <div>
          <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="sim-amount">
            Valor total
          </label>
          <ValidatedInput
            className={fieldClass}
            id="sim-amount"
            inputMode="decimal"
            name="amount"
            placeholder="3500,00"
            required
          />
        </div>

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="sim-payment-type"
          >
            Forma de pagamento
          </label>
          <select
            className={fieldClass}
            id="sim-payment-type"
            name="paymentType"
            onChange={(event) => setPaymentType(event.target.value as "cash" | "installment")}
            value={paymentType}
          >
            <option value="installment">Parcelado</option>
            <option value="cash">À vista</option>
          </select>
        </div>

        {paymentType === "installment" ? (
          <div>
            <label
              className="mb-2 block text-sm font-medium text-text-secondary"
              htmlFor="sim-installments"
            >
              Número de parcelas
            </label>
            <ValidatedInput
              className={fieldClass}
              defaultValue={4}
              id="sim-installments"
              inputMode="numeric"
              max={60}
              min={2}
              name="installments"
              type="number"
            />
          </div>
        ) : (
          <div className="hidden lg:block" />
        )}

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="sim-start-month"
          >
            Mês de início
          </label>
          <select className={fieldClass} defaultValue={defaultMonth} id="sim-start-month" name="startMonth">
            {Array.from({ length: 12 }, (_, index) => index + 1).map((month) => (
              <option key={month} value={month}>
                {monthName(month).charAt(0).toUpperCase() + monthName(month).slice(1)}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="sim-start-year"
          >
            Ano de início
          </label>
          <select className={fieldClass} defaultValue={defaultYear} id="sim-start-year" name="startYear">
            {years.map((year) => (
              <option key={year} value={year}>
                {year}
              </option>
            ))}
          </select>
        </div>

        <div className="lg:col-span-2">
          <FormSubmitButton pendingLabel="Simulando...">Simular e salvar</FormSubmitButton>
        </div>
      </ValidatedForm>
    </DashboardCard>
  );
}

"use client";

import { useState } from "react";
import { addCardExpense } from "@/app/actions/card-expenses";
import { FormSubmitButton } from "@/components/form-submit-button";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";

const fieldClass = "field-input";

export function CardExpenseForm({ cardId }: { cardId: string }) {
  const [paymentType, setPaymentType] = useState<"cash" | "installment">("cash");

  return (
    <ValidatedForm
      action={addCardExpense}
      successMessage="Compra lançada."
      resetOnSuccess
      className="grid gap-4 md:grid-cols-2 xl:grid-cols-[minmax(0,1fr)_150px_150px_110px_150px_auto] xl:items-end"
    >
      <input name="cardId" type="hidden" value={cardId} />
      <div className="md:col-span-2 xl:col-span-1">
        <label
          className="mb-2 block text-sm font-medium text-text-secondary"
          htmlFor="expense-description"
        >
          Descrição
        </label>
        <ValidatedInput
          autoComplete="off"
          className={fieldClass}
          id="expense-description"
          name="description"
          placeholder="Notebook, assinatura, manutenção…"
          required
        />
      </div>
      <div>
        <label
          className="mb-2 block text-sm font-medium text-text-secondary"
          htmlFor="expense-amount"
        >
          {paymentType === "installment" ? "Valor total" : "Valor"}
        </label>
        <ValidatedInput
          autoComplete="off"
          className={fieldClass}
          id="expense-amount"
          inputMode="decimal"
          name="amount"
          placeholder="120,00"
          required
        />
      </div>
      <div>
        <label
          className="mb-2 block text-sm font-medium text-text-secondary"
          htmlFor="expense-payment-type"
        >
          Pagamento
        </label>
        <ValidatedSelect
          className={fieldClass}
          id="expense-payment-type"
          name="paymentType"
          onChange={(event) =>
            setPaymentType(event.target.value as "cash" | "installment")
          }
          value={paymentType}
        >
          <option value="cash">À vista</option>
          <option value="installment">Parcelado</option>
        </ValidatedSelect>
      </div>
      {paymentType === "installment" ? (
        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="expense-installments"
          >
            Parcelas
          </label>
          <ValidatedInput
            className={fieldClass}
            defaultValue={2}
            id="expense-installments"
            inputMode="numeric"
            max={60}
            min={2}
            name="installments"
            required
            type="number"
          />
        </div>
      ) : (
        <input name="installments" type="hidden" value="1" />
      )}
      <div>
        <label
          className="mb-2 block text-sm font-medium text-text-secondary"
          htmlFor="expense-date"
        >
          Data (opcional)
        </label>
        <ValidatedInput
          className={fieldClass}
          id="expense-date"
          name="purchaseDate"
          type="date"
        />
      </div>
      <FormSubmitButton pendingLabel="Lançando...">Lançar</FormSubmitButton>
      {paymentType === "installment" ? (
        <p className="text-xs leading-5 text-text-muted md:col-span-2 xl:col-span-6">
          A primeira parcela entra no mês selecionado. O total é dividido sem perder centavos.
        </p>
      ) : null}
    </ValidatedForm>
  );
}

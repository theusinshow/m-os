"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { addSubscription } from "@/app/actions/subscriptions";
import { FormSubmitButton } from "@/components/form-submit-button";
import { Button } from "@/components/ui/button";
import { Drawer } from "@/components/ui/drawer";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";

export function SubscriptionFormDrawer() {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button onClick={() => setOpen(true)} type="button">
        <Plus size={16} aria-hidden="true" />
        Nova assinatura
      </Button>
      <Drawer
        description="Cadastre a cobrança recorrente ou o teste grátis. O app avisa antes de cobrar."
        onClose={() => setOpen(false)}
        open={open}
        title="Nova assinatura / teste grátis"
      >
        <ValidatedForm
          action={addSubscription}
          className="space-y-4"
          onSuccess={() => setOpen(false)}
          resetOnSuccess
          successMessage="Assinatura salva."
        >
          <div>
            <label className="field-label" htmlFor="sub-name">
              Serviço
            </label>
            <ValidatedInput
              className="field-input"
              id="sub-name"
              name="name"
              placeholder="Netflix, Spotify, ChatGPT…"
              required
            />
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="sub-amount">
                Valor da cobrança
              </label>
              <ValidatedInput
                className="field-input"
                id="sub-amount"
                inputMode="decimal"
                name="amount"
                placeholder="39,90"
                required
              />
            </div>
            <div>
              <label className="field-label" htmlFor="sub-date">
                Data da 1ª cobrança
              </label>
              <ValidatedInput
                className="field-input"
                id="sub-date"
                name="nextChargeDate"
                required
                type="date"
              />
            </div>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="sub-cycle">
                Recorrência
              </label>
              <ValidatedSelect className="field-input" defaultValue="monthly" id="sub-cycle" name="cycle">
                <option value="monthly">Mensal</option>
                <option value="yearly">Anual</option>
                <option value="once">Cobrança única</option>
              </ValidatedSelect>
            </div>
            <div>
              <label className="field-label" htmlFor="sub-reminder">
                Avisar quantos dias antes
              </label>
              <ValidatedInput
                className="field-input"
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

          <FormSubmitButton pendingLabel="Salvando...">Salvar assinatura</FormSubmitButton>
        </ValidatedForm>
      </Drawer>
    </>
  );
}

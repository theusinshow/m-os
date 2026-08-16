"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { createCard } from "@/app/actions/cards";
import { FormSubmitButton } from "@/components/form-submit-button";
import { Button } from "@/components/ui/button";
import { Drawer } from "@/components/ui/drawer";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";

export function CardFormDrawer() {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button onClick={() => setOpen(true)} type="button" variant="secondary">
        <Plus size={16} aria-hidden="true" />
        Novo cartão
      </Button>
      <Drawer
        description="O cartão serve para agrupar faturas. Inativar não apaga o histórico."
        onClose={() => setOpen(false)}
        open={open}
        title="Novo cartão"
      >
        <ValidatedForm
          action={createCard}
          className="space-y-4"
          onSuccess={() => setOpen(false)}
          resetOnSuccess
          successMessage="Cartão adicionado."
        >
          <div>
            <label className="field-label" htmlFor="card-name">
              Nome
            </label>
            <ValidatedInput
              className="field-input"
              id="card-name"
              name="name"
              placeholder="Nubank Pessoal"
              required
            />
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="card-type">
                Tipo
              </label>
              <ValidatedSelect
                className="field-input"
                defaultValue="personal"
                id="card-type"
                name="cardType"
                required
              >
                <option value="personal">Pessoal</option>
                <option value="business">PJ</option>
              </ValidatedSelect>
            </div>
            <div>
              <label className="field-label" htmlFor="card-due-day">
                Dia de vencimento
              </label>
              <ValidatedInput
                className="field-input"
                id="card-due-day"
                inputMode="numeric"
                max={31}
                min={1}
                name="dueDay"
                placeholder="10"
                required
                type="number"
              />
            </div>
          </div>

          <FormSubmitButton pendingLabel="Adicionando...">Adicionar cartão</FormSubmitButton>
        </ValidatedForm>
      </Drawer>
    </>
  );
}

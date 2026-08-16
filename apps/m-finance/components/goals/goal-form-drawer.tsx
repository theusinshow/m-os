"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { createGoal } from "@/app/actions/goals";
import { FormSubmitButton } from "@/components/form-submit-button";
import { Button } from "@/components/ui/button";
import { Drawer } from "@/components/ui/drawer";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";

export function GoalFormDrawer() {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button onClick={() => setOpen(true)} type="button">
        <Plus size={16} aria-hidden="true" />
        Nova meta
      </Button>
      <Drawer
        description="Metas são acompanhadas à parte: não entram no cálculo de contas nem na sobra do mês."
        onClose={() => setOpen(false)}
        open={open}
        title="Nova meta"
      >
        <ValidatedForm
          action={createGoal}
          className="space-y-4"
          onSuccess={() => setOpen(false)}
          resetOnSuccess
          successMessage="Meta criada."
        >
          <div>
            <label className="field-label" htmlFor="goal-name">
              Nome da meta
            </label>
            <ValidatedInput
              className="field-input"
              id="goal-name"
              name="name"
              placeholder="Reserva de emergência"
              required
            />
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="goal-target">
                Valor alvo
              </label>
              <ValidatedInput
                className="field-input"
                id="goal-target"
                inputMode="decimal"
                name="targetAmount"
                placeholder="10000,00"
                required
              />
            </div>
            <div>
              <label className="field-label" htmlFor="goal-current">
                Já guardado (opcional)
              </label>
              <ValidatedInput
                className="field-input"
                id="goal-current"
                inputMode="decimal"
                name="currentAmount"
                placeholder="0,00"
              />
            </div>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label className="field-label" htmlFor="goal-priority">
                Prioridade
              </label>
              <ValidatedSelect className="field-input" defaultValue="medium" id="goal-priority" name="priority">
                <option value="low">Baixa</option>
                <option value="medium">Média</option>
                <option value="high">Alta</option>
              </ValidatedSelect>
            </div>
            <div>
              <label className="field-label" htmlFor="goal-deadline">
                Prazo (opcional)
              </label>
              <ValidatedInput className="field-input" id="goal-deadline" name="deadline" type="date" />
            </div>
          </div>

          <FormSubmitButton pendingLabel="Criando...">Criar meta</FormSubmitButton>
        </ValidatedForm>
      </Drawer>
    </>
  );
}

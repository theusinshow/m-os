"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { createBudget } from "@/app/actions/budgets";
import { FormSubmitButton } from "@/components/form-submit-button";
import { Button } from "@/components/ui/button";
import { Drawer } from "@/components/ui/drawer";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";
type CategoryOption = { id: string; name: string };
type CardOption = { id: string; name: string; cardType: "personal" | "business" };

type BudgetType = "total" | "category" | "card";

export function BudgetFormDrawer({
  categories,
  cards,
}: {
  categories: CategoryOption[];
  cards: CardOption[];
}) {
  const [open, setOpen] = useState(false);
  const [budgetType, setBudgetType] = useState<BudgetType>("total");

  return (
    <>
      <Button onClick={() => setOpen(true)} type="button">
        <Plus size={16} aria-hidden="true" />
        Novo orçamento
      </Button>
      <Drawer
        description="Defina tetos de gasto por categoria, cartão ou para o mês inteiro."
        onClose={() => setOpen(false)}
        open={open}
        title="Novo orçamento"
      >
        <ValidatedForm
          action={createBudget}
          className="space-y-4"
          onSuccess={() => setOpen(false)}
          resetOnSuccess
          successMessage="Orçamento criado."
        >
          <div>
            <label className="field-label" htmlFor="budget-type">
              Tipo
            </label>
            <ValidatedSelect
              className="field-input"
              id="budget-type"
              name="budgetType"
              onChange={(event) => setBudgetType(event.target.value as BudgetType)}
              value={budgetType}
            >
              <option value="total">Gasto total do mês</option>
              <option value="category">Por categoria</option>
              <option value="card">Por cartão</option>
            </ValidatedSelect>
          </div>

          <div>
            <label className="field-label" htmlFor="budget-limit">
              Limite (R$)
            </label>
            <ValidatedInput
              className="field-input"
              id="budget-limit"
              inputMode="decimal"
              name="limit"
              placeholder="2000,00"
              required
            />
          </div>

          {budgetType === "category" ? (
            <div>
              <label className="field-label" htmlFor="budget-category">
                Categoria
              </label>
              <ValidatedSelect className="field-input" id="budget-category" name="categoryId">
                {categories.map((cat) => (
                  <option key={cat.id} value={cat.id}>
                    {cat.name}
                  </option>
                ))}
              </ValidatedSelect>
            </div>
          ) : null}

          {budgetType === "card" ? (
            <div>
              <label className="field-label" htmlFor="budget-card">
                Cartão
              </label>
              <ValidatedSelect className="field-input" id="budget-card" name="cardId">
                {cards.map((card) => (
                  <option key={card.id} value={card.id}>
                    {card.name} ({card.cardType === "business" ? "PJ" : "pessoal"})
                  </option>
                ))}
              </ValidatedSelect>
            </div>
          ) : null}

          <FormSubmitButton pendingLabel="Criando...">Criar orçamento</FormSubmitButton>
        </ValidatedForm>
      </Drawer>
    </>
  );
}

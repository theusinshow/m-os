"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { createInvoice } from "@/app/actions/invoices";
import { FormSubmitButton } from "@/components/form-submit-button";
import { Button } from "@/components/ui/button";
import { Drawer } from "@/components/ui/drawer";
import {
  ValidatedForm,
  ValidatedInput,
  ValidatedSelect,
} from "@/components/ui/validated-form";

type Card = {
  id: string;
  name: string;
  cardType: "personal" | "business";
};

export function InvoiceFormDrawer({ cards }: { cards: Card[] }) {
  const [open, setOpen] = useState(false);

  if (cards.length === 0) return null;

  return (
    <>
      <Button onClick={() => setOpen(true)} type="button">
        <Plus size={16} aria-hidden="true" />
        Nova fatura
      </Button>
      <Drawer
        description="Lance só o total do mês. O vencimento usa o dia do próprio cartão."
        onClose={() => setOpen(false)}
        open={open}
        title="Nova fatura"
      >
        <ValidatedForm
          action={createInvoice}
          className="space-y-4"
          onSuccess={() => setOpen(false)}
          resetOnSuccess
          successMessage="Fatura adicionada."
        >
          <div>
            <label className="field-label" htmlFor="invoice-card">
              Cartão
            </label>
            <ValidatedSelect className="field-input" id="invoice-card" name="cardId" required>
              {cards.map((card) => (
                <option key={card.id} value={card.id}>
                  {card.name}
                  {card.cardType === "business" ? " PJ" : ""}
                </option>
              ))}
            </ValidatedSelect>
          </div>
          <div>
            <label className="field-label" htmlFor="invoice-amount">
              Valor da fatura
            </label>
            <ValidatedInput
              className="field-input"
              id="invoice-amount"
              inputMode="decimal"
              name="amount"
              placeholder="900,00"
              required
            />
          </div>
          <FormSubmitButton pendingLabel="Salvando...">Salvar fatura</FormSubmitButton>
        </ValidatedForm>
      </Drawer>
    </>
  );
}

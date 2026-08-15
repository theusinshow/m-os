import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { createCard, setCardActive, updateCard } from "@/app/actions/cards";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import { CardBrandMark } from "@/components/cards/card-brand-mark";
import { InlineEmpty } from "@/components/ui/inline-empty";

type ManagedCard = {
  id: string;
  name: string;
  cardType: "personal" | "business";
  dueDay: number;
  isActive: boolean;
};

const cardTypeLabel = {
  personal: "Pessoal",
  business: "PJ",
};

export function CardManager({ cards }: { cards: ManagedCard[] }) {
  return (
    <DashboardCard
      description="Cartões usados para controlar faturas. Inative sem perder o histórico."
      title="Gerenciar cartões"
    >
      <div className="grid gap-5 xl:grid-cols-[0.85fr_1fr]">
        <ValidatedForm action={createCard} successMessage="Cartão adicionado." resetOnSuccess className="space-y-4">
          <div>
            <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="card-name">
              Nome
            </label>
            <ValidatedInput
              className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted"
              id="card-name"
              name="name"
              placeholder="Nubank Pessoal"
              required
            />
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div>
              <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="card-type">
                Tipo
              </label>
              <ValidatedSelect
                className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary"
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
              <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="card-due-day">
                Dia de vencimento
              </label>
              <ValidatedInput
                className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted"
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

        <div className="space-y-3">
          {cards.length === 0 ? (
            <InlineEmpty>Nenhum cartão cadastrado ainda.</InlineEmpty>
          ) : (
            cards.map((card) => (
              <div
                className="rounded-lg border border-border-subtle bg-background-elevated p-4"
                key={card.id}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-start gap-3">
                    <CardBrandMark name={card.name} />
                    <div>
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="font-semibold text-text-primary">{card.name}</p>
                        {card.cardType === "business" ? (
                          <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                            PJ
                          </span>
                        ) : null}
                        {!card.isActive ? (
                          <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                            Inativo
                          </span>
                        ) : null}
                      </div>
                      <p className="mt-1 text-sm text-text-muted">
                        {cardTypeLabel[card.cardType]} · vence dia {card.dueDay}
                      </p>
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <Link
                      className="focus-ring inline-flex min-h-10 items-center gap-1.5 rounded-md border border-border-default bg-background-card px-3 text-xs font-semibold text-text-secondary transition duration-200 hover:border-border-strong hover:bg-background-hover hover:text-text-primary"
                      href={`/app/cards/${card.id}`}
                    >
                      Abrir
                      <ArrowRight size={14} aria-hidden="true" />
                    </Link>
                    <ToastForm
                      action={setCardActive}
                      successMessage={card.isActive ? "Cartão inativado." : "Cartão reativado."}
                    >
                      <input name="cardId" type="hidden" value={card.id} />
                      <input name="isActive" type="hidden" value={card.isActive ? "false" : "true"} />
                      <FormSubmitButton
                        pendingLabel={card.isActive ? "Inativando..." : "Reativando..."}
                        variant="secondary"
                      >
                        {card.isActive ? "Inativar" : "Reativar"}
                      </FormSubmitButton>
                    </ToastForm>
                  </div>
                </div>

                <EditDisclosure className="mt-4">
                  <ValidatedForm action={updateCard} successMessage="Cartão atualizado." className="grid gap-3">
                    <input name="cardId" type="hidden" value={card.id} />
                    <ValidatedInput
                      className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                      defaultValue={card.name}
                      name="name"
                      required
                    />
                    <div className="grid gap-3 sm:grid-cols-2">
                      <ValidatedSelect
                        className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                        defaultValue={card.cardType}
                        name="cardType"
                      >
                        <option value="personal">Pessoal</option>
                        <option value="business">PJ</option>
                      </ValidatedSelect>
                      <ValidatedInput
                        className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                        defaultValue={card.dueDay}
                        inputMode="numeric"
                        max={31}
                        min={1}
                        name="dueDay"
                        required
                        type="number"
                      />
                    </div>
                    <FormSubmitButton pendingLabel="Salvando...">Salvar cartão</FormSubmitButton>
                  </ValidatedForm>
                </EditDisclosure>
              </div>
            ))
          )}
        </div>
      </div>
    </DashboardCard>
  );
}

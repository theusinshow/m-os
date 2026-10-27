import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { createInvoice, markInvoiceAsPending } from "@/app/actions/invoices";
import { setCardActive, updateCard } from "@/app/actions/cards";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { FormSubmitButton } from "@/components/form-submit-button";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
import { StatusBadge } from "@/components/status-badge";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import { CardBrandMark } from "@/components/cards/card-brand-mark";
import { EstimateBadge } from "@/components/cards/estimate-badge";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { Badge } from "@/components/ui/badge";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";
import { centsToInput } from "@/lib/money";
import type { CardMonthLine, ForecastCard } from "@/lib/calculations/forecast";

const linkClass =
  "focus-ring inline-flex min-h-10 items-center justify-center gap-1.5 rounded-md border border-border-default bg-background-card px-3 text-xs font-semibold text-text-secondary transition duration-200 hover:border-border-strong hover:bg-background-hover hover:text-text-primary";

function basisText(line: CardMonthLine) {
  if (line.source !== "estimated") return null;
  if (line.basisCount === 0) return "pelas parcelas já lançadas neste mês";
  return line.basisCount === 1
    ? "pela última fatura do cartão"
    : `pela média das últimas ${line.basisCount} faturas`;
}

/**
 * Confirmar o valor que o banco fechou. Nasce preenchido com a estimativa:
 * na maior parte dos meses basta corrigir os centavos.
 */
function ConfirmInvoiceForm({ line }: { line: CardMonthLine }) {
  const estimated = line.source === "estimated";

  return (
    <ValidatedForm
      action={createInvoice}
      className="grid gap-2 sm:grid-cols-[1fr_auto] sm:items-start"
      successMessage="Fatura confirmada."
    >
      <input name="cardId" type="hidden" value={line.card.id} />
      <ValidatedInput
        aria-label={`Valor da fatura do ${line.card.name}`}
        className="field-input"
        defaultValue={estimated ? centsToInput(line.amountCents) : ""}
        inputMode="decimal"
        name="amount"
        placeholder="Valor que o banco cobrou"
        required
      />
      <FormSubmitButton pendingLabel="Salvando..." variant="secondary">
        {estimated ? "Confirmar valor" : "Lançar fatura"}
      </FormSubmitButton>
    </ValidatedForm>
  );
}

function CardEditForms({ card }: { card: ForecastCard }) {
  return (
    <EditDisclosure className="mt-4" label="Cartão">
      <ValidatedForm action={updateCard} successMessage="Cartão atualizado." className="grid gap-3">
        <input name="cardId" type="hidden" value={card.id} />
        <ValidatedInput className="field-input" defaultValue={card.name} name="name" required />
        <div className="grid gap-3 sm:grid-cols-2">
          <ValidatedSelect className="field-input" defaultValue={card.cardType} name="cardType">
            <option value="personal">Pessoal</option>
            <option value="business">PJ</option>
          </ValidatedSelect>
          <ValidatedInput
            aria-label="Dia de vencimento"
            className="field-input"
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
      <ToastForm
        action={setCardActive}
        className="mt-3"
        successMessage={card.isActive ? "Cartão inativado." : "Cartão reativado."}
      >
        <input name="cardId" type="hidden" value={card.id} />
        <input name="isActive" type="hidden" value={card.isActive ? "false" : "true"} />
        <FormSubmitButton
          pendingLabel={card.isActive ? "Inativando..." : "Reativando..."}
          variant="secondary"
        >
          {card.isActive ? "Inativar cartão" : "Reativar cartão"}
        </FormSubmitButton>
      </ToastForm>
    </EditDisclosure>
  );
}

function CardLine({ line }: { line: CardMonthLine }) {
  const { card, invoice } = line;
  const basis = basisText(line);

  return (
    <div className="rounded-lg border border-border-subtle bg-background-elevated p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <CardBrandMark name={card.name} />
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <p className="truncate font-semibold text-text-primary">{card.name}</p>
              {card.cardType === "business" ? (
                <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                  PJ
                </span>
              ) : null}
            </div>
            <p className="mt-1 text-sm text-text-muted">Vence {formatShortDate(line.dueDate)}</p>
          </div>
        </div>
        {invoice ? (
          <StatusBadge status={invoice.status} />
        ) : line.source === "estimated" ? (
          <EstimateBadge />
        ) : (
          <Badge
            className="border-border-subtle bg-transparent text-text-muted"
            label="Sem fatura"
          />
        )}
      </div>

      <div className="mt-4 border-t border-border-subtle pt-4">
        {line.source === "none" ? (
          <p className="text-sm text-text-muted">
            Nenhuma fatura lançada e nenhum histórico para estimar.
          </p>
        ) : (
          <>
            <p
              className={`num text-2xl font-semibold ${
                line.source === "estimated" ? "text-text-secondary" : "text-text-primary"
              }`}
            >
              {line.source === "estimated" ? "≈ " : ""}
              {formatCurrency(line.amountCents)}
            </p>
            {basis ? <p className="mt-1 text-xs text-text-muted">Estimada {basis}.</p> : null}
            {line.installmentsCents > 0 ? (
              <p className="mt-1 text-xs text-text-muted">
                {formatCurrency(line.installmentsCents)} em parcelas já lançadas neste mês.
              </p>
            ) : null}
          </>
        )}

        <div className="mt-4 space-y-2">
          {!invoice ? (
            <ConfirmInvoiceForm line={line} />
          ) : invoice.status !== "paid" ? (
            <MarkPaidButton payableId={invoice.id} payableType="invoice" variant="success">
              Marcar fatura como paga
            </MarkPaidButton>
          ) : (
            <ToastForm action={markInvoiceAsPending} successMessage="Fatura reaberta.">
              <input name="invoiceId" type="hidden" value={invoice.id} />
              <FormSubmitButton pendingLabel="Reabrindo..." variant="secondary">
                Reabrir fatura
              </FormSubmitButton>
            </ToastForm>
          )}
          <Link className={`${linkClass} w-full sm:w-auto`} href={`/app/cards/${card.id}`}>
            Compras e histórico
            <ArrowRight size={14} aria-hidden="true" />
          </Link>
        </div>
      </div>

      <CardEditForms card={card} />
    </div>
  );
}

/**
 * Cada cartão no mês selecionado, com uma ação principal só: confirmar o valor
 * enquanto é estimativa, marcar como paga depois.
 */
export function CardManager({
  lines,
  inactiveCards,
}: {
  lines: CardMonthLine[];
  inactiveCards: ForecastCard[];
}) {
  return (
    <DashboardCard title="Faturas do mês">
      {lines.length === 0 ? (
        <InlineEmpty>
          Nenhum cartão cadastrado. Adicione um cartão para começar a controlar faturas.
        </InlineEmpty>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {lines.map((line) => (
            <CardLine key={line.card.id} line={line} />
          ))}
        </div>
      )}

      {inactiveCards.length > 0 ? (
        <details className="group mt-5 border-t border-border-subtle pt-4">
          <summary className="focus-ring cursor-pointer rounded-md text-sm font-medium text-text-muted [&::-webkit-details-marker]:hidden">
            {inactiveCards.length === 1
              ? "1 cartão inativo"
              : `${inactiveCards.length} cartões inativos`}
          </summary>
          <ul className="mt-3 space-y-2">
            {inactiveCards.map((card) => (
              <li
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
                key={card.id}
              >
                <span className="flex items-center gap-3 text-sm text-text-secondary">
                  <CardBrandMark name={card.name} />
                  {card.name}
                  {card.cardType === "business" ? " · PJ" : ""}
                </span>
                <ToastForm action={setCardActive} successMessage="Cartão reativado.">
                  <input name="cardId" type="hidden" value={card.id} />
                  <input name="isActive" type="hidden" value="true" />
                  <FormSubmitButton pendingLabel="Reativando..." variant="secondary">
                    Reativar
                  </FormSubmitButton>
                </ToastForm>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </DashboardCard>
  );
}

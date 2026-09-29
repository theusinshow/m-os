import { createIncome } from "@/app/actions/incomes";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { centsToInput } from "@/lib/money";

/**
 * Lançar a nota fiscal de um mês em um passo: o mês já vem escolhido e o
 * valor já vem com o palpite. A NF é a receita principal; o nome e o tipo não
 * precisam ser perguntados de novo todo mês.
 */
export function NfQuickForm({
  monthValue,
  monthLabel,
  suggestedCents,
}: {
  /** `yyyy-mm` do mês em que a NF entra. */
  monthValue: string;
  monthLabel: string;
  suggestedCents: number;
}) {
  const inputId = `nf-amount-${monthValue}`;

  return (
    <ValidatedForm action={createIncome} className="grid gap-3" successMessage="Nota fiscal lançada.">
      <input name="name" type="hidden" value="Nota fiscal" />
      <input name="incomeType" type="hidden" value="main" />
      <input name="targetMonth" type="hidden" value={monthValue} />
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor={inputId}>
            Valor da NF de {monthLabel}
          </label>
          <ValidatedInput
            className="field-input"
            defaultValue={suggestedCents > 0 ? centsToInput(suggestedCents) : ""}
            id={inputId}
            inputMode="decimal"
            name="amount"
            placeholder="5700,00"
            required
          />
        </div>
        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor={`nf-date-${monthValue}`}
          >
            Dia que cai na conta
          </label>
          <input
            className="field-input"
            id={`nf-date-${monthValue}`}
            max={`${monthValue}-31`}
            min={`${monthValue}-01`}
            name="expectedDate"
            type="date"
          />
        </div>
      </div>
      <label className="flex items-center gap-2 text-sm text-text-secondary">
        <input className="h-4 w-4 accent-accent" name="received" type="checkbox" />
        Já caiu na conta
      </label>
      <FormSubmitButton pendingLabel="Lançando...">Lançar NF</FormSubmitButton>
    </ValidatedForm>
  );
}

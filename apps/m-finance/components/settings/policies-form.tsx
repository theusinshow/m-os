import { savePolicies } from "@/app/actions/intelligence";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import type { ResolvedPolicies } from "@/lib/finance-intelligence/policies";
import { centsToInput } from "@/lib/money";

function Field({
  id,
  label,
  hint,
  children,
}: {
  id: string;
  label: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor={id}>
        {label}
      </label>
      {children}
      <p className="mt-2 text-xs leading-5 text-text-muted">{hint}</p>
    </div>
  );
}

/**
 * As regras que o Safe-to-Spend, os cenários e o Observer obedecem. Campo
 * vazio volta ao padrão — e o padrão é dito no próprio campo, para ninguém
 * achar que "vazio" significa zero quando não significa.
 */
export function PoliciesForm({ policies }: { policies: ResolvedPolicies }) {
  const configured = new Set(policies.configured);
  const value = <T,>(key: ResolvedPolicies["configured"][number], shown: T) =>
    configured.has(key) ? shown : undefined;

  return (
    <ValidatedForm action={savePolicies} successMessage="Políticas salvas." className="space-y-5">
      <div className="grid gap-5 lg:grid-cols-2">
        <Field
          id="policy-buffer"
          label="Reserva mínima ao fechar o mês (R$)"
          hint="Sai do Safe-to-Spend todo mês. Vazio: nenhuma margem."
        >
          <ValidatedInput
            className="field-input"
            defaultValue={value("minimum_month_end_buffer", centsToInput(policies.minimumMonthEndBufferCents))}
            id="policy-buffer"
            inputMode="decimal"
            name="minimumBuffer"
            placeholder="0,00"
          />
        </Field>

        <Field
          id="policy-installments"
          label="Máximo em parcelas por mês (R$)"
          hint="Vira aviso nos cenários e no Observer. Vazio: sem teto."
        >
          <ValidatedInput
            className="field-input"
            defaultValue={value(
              "max_installment_commitment",
              policies.maxInstallmentCommitmentCents === null ? "" : centsToInput(policies.maxInstallmentCommitmentCents),
            )}
            id="policy-installments"
            inputMode="decimal"
            name="maxInstallments"
            placeholder="sem teto"
          />
        </Field>
      </div>

      <fieldset>
        <legend className="mb-2 text-sm font-medium text-text-secondary">
          Quanto de cada receita a receber é confiável (%)
        </legend>
        <div className="grid gap-3 sm:grid-cols-3">
          {(
            [
              ["reliableMain", "Principal (NF)", policies.reliableIncomeRules.main],
              ["reliableFreelance", "Freelance", policies.reliableIncomeRules.freelance],
              ["reliableExtra", "Extra", policies.reliableIncomeRules.extra],
            ] as const
          ).map(([name, label, weight]) => (
            <div key={name}>
              <label className="mb-1 block text-xs text-text-muted" htmlFor={`policy-${name}`}>
                {label}
              </label>
              <ValidatedInput
                className="field-input"
                defaultValue={value("reliable_income_rules", String(Math.round(weight * 100)))}
                id={`policy-${name}`}
                inputMode="numeric"
                max={100}
                min={0}
                name={name}
                placeholder={String(Math.round(weight * 100))}
                type="number"
              />
            </div>
          ))}
        </div>
        <p className="mt-2 text-xs leading-5 text-text-muted">
          Receita já recebida sempre conta 100%. Padrão: principal 100, freelance 50, extra 0.
        </p>
      </fieldset>

      <div className="grid gap-5 lg:grid-cols-3">
        <Field id="policy-horizon" label="Horizonte de projeção (meses)" hint="Padrão: 6.">
          <ValidatedInput
            className="field-input"
            defaultValue={value("forecast_horizon_months", String(policies.forecastHorizonMonths))}
            id="policy-horizon"
            inputMode="numeric"
            max={24}
            min={1}
            name="horizonMonths"
            placeholder="6"
            type="number"
          />
        </Field>

        <Field id="policy-lookahead" label="Safe-to-Spend olha quantos meses à frente" hint="Padrão: 2.">
          <ValidatedInput
            className="field-input"
            defaultValue={value("safe_to_spend_policy", String(policies.safeToSpend.lookaheadMonths))}
            id="policy-lookahead"
            inputMode="numeric"
            max={12}
            min={0}
            name="lookaheadMonths"
            placeholder="2"
            type="number"
          />
        </Field>

        <Field id="policy-sensitivity" label="Sensibilidade dos alertas" hint="Alta avisa mais cedo.">
          <ValidatedSelect
            className="field-input"
            defaultValue={value("observer_sensitivity", policies.observerSensitivity) ?? ""}
            id="policy-sensitivity"
            name="sensitivity"
          >
            <option value="">Padrão (normal)</option>
            <option value="low">Baixa</option>
            <option value="normal">Normal</option>
            <option value="high">Alta</option>
          </ValidatedSelect>
        </Field>
      </div>

      <label className="flex items-center gap-3 text-sm text-text-secondary">
        <input
          className="h-4 w-4 accent-accent"
          defaultChecked={policies.safeToSpend.protectGoals}
          name="protectGoals"
          type="checkbox"
        />
        Proteger o ritmo das metas no Safe-to-Spend
      </label>

      <FormSubmitButton pendingLabel="Salvando...">Salvar políticas</FormSubmitButton>
    </ValidatedForm>
  );
}

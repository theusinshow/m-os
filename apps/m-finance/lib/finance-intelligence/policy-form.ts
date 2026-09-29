import { parseCurrencyToCents } from "@/lib/money";
import { parsePolicyValue, type PolicyKey } from "@/lib/finance-intelligence/policies";

type FormLike = { get(name: string): FormDataEntryValue | null };

export type PolicyFormResult = {
  set: { key: PolicyKey; value: unknown }[];
  /** Campo vazio = volta ao padrão. */
  reset: PolicyKey[];
  errors: Record<string, string>;
};

function percent(form: FormLike, name: string) {
  const raw = String(form.get(name) ?? "").trim();
  if (!raw) return null;
  const value = Number(raw.replace(",", "."));
  return Number.isFinite(value) ? value / 100 : Number.NaN;
}

function integer(form: FormLike, name: string) {
  const raw = String(form.get(name) ?? "").trim();
  if (!raw) return null;
  const value = Number(raw);
  return Number.isInteger(value) ? value : Number.NaN;
}

/**
 * A tela de políticas em políticas. Cada chave passa pelo MESMO schema que a
 * Action API usa quando o Hermes propõe a mudança — a tela não tem regra
 * própria.
 */
export function policiesFromForm(form: FormLike): PolicyFormResult {
  const result: PolicyFormResult = { set: [], reset: [], errors: {} };
  const push = (key: PolicyKey, value: unknown, field: string, message: string) => {
    const parsed = parsePolicyValue(key, value);
    if (parsed.success) result.set.push({ key, value: parsed.data });
    else result.errors[field] = message;
  };

  const buffer = String(form.get("minimumBuffer") ?? "").trim();
  if (buffer) push("minimum_month_end_buffer", { amountCents: parseCurrencyToCents(buffer) }, "minimumBuffer", "Informe um valor válido.");
  else result.reset.push("minimum_month_end_buffer");

  const main = percent(form, "reliableMain");
  const freelance = percent(form, "reliableFreelance");
  const extra = percent(form, "reliableExtra");
  if (main === null && freelance === null && extra === null) {
    result.reset.push("reliable_income_rules");
  } else {
    push(
      "reliable_income_rules",
      { main: main ?? 1, freelance: freelance ?? 0.5, extra: extra ?? 0 },
      "reliableFreelance",
      "Use percentuais entre 0 e 100.",
    );
  }

  const cap = String(form.get("maxInstallments") ?? "").trim();
  if (cap) push("max_installment_commitment", { amountCents: parseCurrencyToCents(cap) }, "maxInstallments", "Informe um valor válido.");
  else result.reset.push("max_installment_commitment");

  const horizon = integer(form, "horizonMonths");
  if (horizon === null) result.reset.push("forecast_horizon_months");
  else push("forecast_horizon_months", { months: horizon }, "horizonMonths", "Entre 1 e 24 meses.");

  const level = String(form.get("sensitivity") ?? "").trim();
  if (!level) result.reset.push("observer_sensitivity");
  else push("observer_sensitivity", { level }, "sensitivity", "Escolha baixa, normal ou alta.");

  const lookahead = integer(form, "lookaheadMonths");
  const protectGoals = form.get("protectGoals") === "on";
  if (lookahead === null && !protectGoals) result.reset.push("safe_to_spend_policy");
  else push("safe_to_spend_policy", { lookaheadMonths: lookahead ?? 2, protectGoals }, "lookaheadMonths", "Entre 0 e 12 meses.");

  return result;
}

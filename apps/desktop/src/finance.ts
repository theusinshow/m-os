import { invoke } from "@tauri-apps/api/core";

/** O que uma ferramenta de leitura do M-Finance devolveu (ADR-073). */
export type FinanceReading = { tool: string; asOf: string; data: unknown };

/**
 * Fronteira do renderer com o modulo `finance` do lado Rust.
 *
 * Mesmo padrao de `hermes.ts`: nenhuma chamada de rede em componente React, e
 * o secret nunca atravessa de volta para ca depois de guardado — o renderer
 * so aprende que existe (booleano), nunca qual e.
 */
export const finance = {
  setActionSecret(secret: string) {
    return invoke<void>("finance_set_action_secret", { secret });
  },
  clearActionSecret() {
    return invoke<void>("finance_clear_action_secret");
  },
  actionSecretConfigured() {
    return invoke<boolean>("finance_action_secret_configured");
  },
  /** O secret de LEITURA: lê o M-Finance e não escreve nada. Opcional — sem
   *  ele a leitura usa o de ação. */
  setReadSecret(secret: string) {
    return invoke<void>("finance_set_read_secret", { secret });
  },
  clearReadSecret() {
    return invoke<void>("finance_clear_read_secret");
  },
  readSecretConfigured() {
    return invoke<boolean>("finance_read_secret_configured");
  },
  /** O context pack, para a Home. Rejeita com `"not_configured"` sem secret. */
  homeSummary() {
    return invoke<FinanceReading>("finance_home_summary");
  },
};

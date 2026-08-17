import { invoke } from "@tauri-apps/api/core";

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
};

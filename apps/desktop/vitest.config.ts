import { defineConfig } from "vitest/config";

/**
 * Só as funções puras do renderer.
 *
 * Nada de DOM: o que precisa de tela não é verificável daqui de qualquer jeito
 * — a janela do Tauri não é legível para quem escreve o teste —, e um runner
 * com jsdom convidaria a escrever teste de componente que passa verde enquanto
 * a janela real mostra outra coisa. Pior que não ter teste é ter teste que
 * mente.
 */
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});

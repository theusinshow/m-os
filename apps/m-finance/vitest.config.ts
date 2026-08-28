import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    // Espelha o `paths` do tsconfig.json ("@/*" -> "./*").
    alias: {
      "@": fileURLToPath(new URL(".", import.meta.url)),
    },
  },
  test: {
    environment: "node",
    // `.tsx` entra para os testes de componente. O ambiente segue `node` por
    // padrao — quem precisa de DOM pede jsdom no proprio arquivo, com
    // `// @vitest-environment jsdom`, para nao cobrar o custo do jsdom dos 110
    // testes que nao encostam em tela.
    include: ["**/*.test.ts", "**/*.test.tsx"],
    exclude: ["node_modules/**", ".next/**"],
  },
});

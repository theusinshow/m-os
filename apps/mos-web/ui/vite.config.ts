import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// A saida vai para `../static`, que e o que o binario Rust serve. Um caminho
// so, decidido aqui: se o front e o servidor discordarem sobre onde os
// arquivos estao, o sintoma e uma pagina em branco sem erro nenhum.
export default defineConfig({
  plugins: [react()],
  build: { outDir: "../static", emptyOutDir: true },
  // Em `npm run dev` o front roda separado e fala com o Rust por proxy, para o
  // ciclo de UI nao precisar recompilar Rust.
  server: {
    port: 9131,
    proxy: { "/api": "http://127.0.0.1:9130" },
  },
});

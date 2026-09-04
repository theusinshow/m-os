import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// O fuso do teste é declarado, e não herdado.
//
// A agenda corta o dia no fuso DO APARELHO — é a regra que impede a noite de
// trabalho de cair em amanhã —, e testar isso exige um fuso conhecido. Sem esta
// linha os testes passam nesta máquina (UTC-3) e falham no CI (UTC), que é o
// pior dos dois mundos: a máquina de quem escreve mente, e a que diz a verdade
// vira barulho.
process.env.TZ = "America/Sao_Paulo";

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

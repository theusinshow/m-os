// Coded Atlas — gerador de sessão autenticada.
//
// Abre um navegador VISÍVEL para você logar manualmente no app (inclusive com
// MFA / login social), e salva a sessão em auth/<slug>.json. Depois disso, a
// engine de captura reusa essa sessão e fotografa as páginas internas.
//
// Uso:
//   node scripts/login.mjs <url> <slug>
//
// O <slug> precisa ser EXATAMENTE o mesmo usado ao gerar o projeto no Atlas.
// Ex.: node scripts/login.mjs https://meu-app.com/login meu-app

import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import readline from "node:readline";

const [, , url, slug] = process.argv;

if (!url || !slug) {
  console.error("Uso: node scripts/login.mjs <url> <slug>");
  process.exit(1);
}

const authDir = path.join(process.cwd(), "auth");
await mkdir(authDir, { recursive: true });
const out = path.join(authDir, `${slug}.json`);

const browser = await chromium.launch({ headless: false }); // janela visível
const context = await browser.newContext();
const page = await context.newPage();

try {
  await page.goto(url);
} catch (err) {
  console.error(`\n✖ Não foi possível abrir ${url}: ${err}`);
  await browser.close();
  process.exit(1);
}

console.log("\n▶ Faça login na janela do navegador que abriu.");
console.log("▶ Quando estiver dentro do app (já logado), volte aqui e pressione ENTER.\n");

await new Promise((resolve) => {
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  rl.question("", () => {
    rl.close();
    resolve();
  });
});

await context.storageState({ path: out });
await browser.close();

console.log(`\n✔ Sessão salva em ${out}`);
console.log("  Agora gere o projeto no Atlas usando o mesmo slug:", slug);

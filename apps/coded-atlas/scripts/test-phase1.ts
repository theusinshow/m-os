/**
 * Script de verificação da Fase 1.
 * Execute com: npx tsx scripts/test-phase1.ts
 */
import { validateUrl } from "../lib/validation/validate-url";
import { validateProjectInput } from "../lib/validation/validate-project-input";
import { slugify } from "../lib/validation/slugify";
import { AtlasError } from "../lib/errors";

let passed = 0;
let failed = 0;

function ok(name: string, fn: () => void): void {
  try {
    fn();
    console.log(`  ✓  ${name}`);
    passed++;
  } catch (err) {
    const msg = err instanceof AtlasError ? `AtlasError(${err.code})` : String(err);
    console.error(`  ✗  ${name} — esperava sucesso, recebeu ${msg}`);
    failed++;
  }
}

function throws(name: string, fn: () => void, expectedCode?: string): void {
  try {
    fn();
    console.error(`  ✗  ${name} — esperava erro, mas não lançou`);
    failed++;
  } catch (err) {
    if (!(err instanceof AtlasError)) {
      console.error(`  ✗  ${name} — lançou erro não-AtlasError: ${err}`);
      failed++;
      return;
    }
    if (expectedCode && err.code !== expectedCode) {
      console.error(`  ✗  ${name} — código esperado "${expectedCode}", recebeu "${err.code}"`);
      failed++;
      return;
    }
    console.log(`  ✓  ${name}  →  AtlasError(${err.code}): ${err.userMessage}`);
    passed++;
  }
}

// ─── validateUrl ────────────────────────────────────────────────────────────
console.log("\n── validateUrl ──────────────────────────────────────");

throws("URL vazia",              () => validateUrl(""),                     "INVALID_URL");
throws("URL sem protocolo",      () => validateUrl("example.com"),          "INVALID_URL");
throws("Protocolo ftp:// ",      () => validateUrl("ftp://example.com"),    "INVALID_URL");
throws("https:// sem hostname",  () => validateUrl("https://"),             "INVALID_URL");
ok("URL http válida",            () => validateUrl("http://example.com"));
ok("URL https válida",           () => validateUrl("https://machadoplataformas.com.br"));
ok("URL com path e query",       () => validateUrl("https://site.com/path?foo=bar#hash"));

// ─── slugify ─────────────────────────────────────────────────────────────────
console.log("\n── slugify ──────────────────────────────────────────");

const slugCases: [string, string][] = [
  ["Machado Plataformas",   "machado-plataformas"],
  ["Café Especial",         "cafe-especial"],
  ["My  Project---Here",    "my-project-here"],
  ["  leading trailing  ",  "leading-trailing"],
  ["Área 51",               "area-51"],
  ["Ação & Reação",         "acao-reacao"],
];

for (const [input, expected] of slugCases) {
  const result = slugify(input);
  if (result === expected) {
    console.log(`  ✓  slugify("${input}") → "${result}"`);
    passed++;
  } else {
    console.error(`  ✗  slugify("${input}") → "${result}" (esperado "${expected}")`);
    failed++;
  }
}

// ─── validateProjectInput ────────────────────────────────────────────────────
console.log("\n── validateProjectInput ─────────────────────────────");

ok("Input válido completo", () =>
  validateProjectInput({
    url: "https://machadoplataformas.com.br",
    name: "Machado Plataformas",
    slug: "machado-plataformas",
    category: "Site Institucional",
    client: "Machado Plataformas",
    description: "Site institucional premium.",
  })
);

throws("URL inválida no input", () =>
  validateProjectInput({ url: "not-a-url", name: "T", slug: "t", category: "Site" }),
  "INVALID_URL"
);

throws("Slug com espaços", () =>
  validateProjectInput({ url: "https://ex.com", name: "T", slug: "meu slug", category: "Site" })
);

throws("Slug com maiúsculas", () =>
  validateProjectInput({ url: "https://ex.com", name: "T", slug: "MeuSlug", category: "Site" })
);

throws("Nome vazio", () =>
  validateProjectInput({ url: "https://ex.com", name: "", slug: "t", category: "Site" })
);

throws("Categoria vazia", () =>
  validateProjectInput({ url: "https://ex.com", name: "T", slug: "t", category: "" })
);

// ─── Resumo ──────────────────────────────────────────────────────────────────
console.log(`\n${"─".repeat(52)}`);
console.log(`  ${passed} passou  |  ${failed} falhou\n`);
if (failed > 0) process.exit(1);

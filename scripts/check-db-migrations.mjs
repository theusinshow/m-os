/**
 * Compara as migrations do M-Finance com as que o banco realmente aplicou.
 *
 * Existe porque o problema que ele checa ja aconteceu, e passou meses calado:
 * `db/schema.ts` ganhou a coluna `whatsapp_pending_action_id` em 03/07/2026,
 * a migration `0012` foi gerada, e ela nunca rodou no banco. Como o Drizzle
 * lista TODAS as colunas do schema em todo INSERT, qualquer criacao de conta
 * passa a estourar `42703: column does not exist` — em runtime, dentro de uma
 * request, longe de onde a causa esta. Nada acusa: `npm run build` compila,
 * `npm test` passa (o banco esta mockado) e o app sobe normalmente.
 *
 * O journal (`db/migrations/meta/_journal.json`) e a lista do que deveria
 * estar aplicado; `drizzle.__drizzle_migrations` e a lista do que esta. A
 * diferenca entre as duas e a resposta.
 *
 *   DATABASE_URL=... node scripts/check-db-migrations.mjs
 *   DATABASE_URL=... node scripts/check-db-migrations.mjs --dir ../outro/db/migrations
 *
 * Sai com codigo 1 se faltar migration, para o CI poder barrar.
 */

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const AQUI = dirname(fileURLToPath(import.meta.url));
const indiceDir = process.argv.indexOf("--dir");
const migrationsDir = resolve(
  indiceDir !== -1 && process.argv[indiceDir + 1]
    ? process.argv[indiceDir + 1]
    : join(AQUI, "..", "apps", "m-finance", "db", "migrations"),
);

if (!process.env.DATABASE_URL) {
  console.error("Falta DATABASE_URL no ambiente.");
  process.exit(1);
}

/**
 * O driver vive no `node_modules` do app, nao na raiz do monorepo, e a pasta
 * de migrations pode ser apontada por `--dir` de qualquer lugar. Entao sobe
 * procurando, em vez de assumir uma profundidade fixa.
 */
function acharDriver(partindoDe) {
  let atual = partindoDe;
  for (;;) {
    const candidato = join(atual, "node_modules", "postgres", "src", "index.js");
    if (existsSync(candidato)) return candidato;
    const acima = dirname(atual);
    if (acima === atual) return null;
    atual = acima;
  }
}

const caminhoDoDriver = acharDriver(migrationsDir);
if (!caminhoDoDriver) {
  console.error("Nao achei o pacote `postgres`. Rode `npm install` em apps/m-finance antes.");
  process.exit(1);
}

// `pathToFileURL`: no Windows o import() dinamico recusa caminho absoluto.
const { default: postgres } = await import(pathToFileURL(caminhoDoDriver).href);

const journal = JSON.parse(readFileSync(join(migrationsDir, "meta", "_journal.json"), "utf8"));

const sql = postgres(process.env.DATABASE_URL, { prepare: false, max: 1 });

let aplicadas;
try {
  aplicadas = await sql`select created_at from drizzle.__drizzle_migrations order by created_at`;
} catch (error) {
  console.error(`Nao consegui ler drizzle.__drizzle_migrations: ${error.message}`);
  await sql.end();
  process.exit(1);
}

await sql.end();

// Mesmo criterio do migrator do Drizzle: ele nao compara hashes para decidir
// o que rodar, compara o `when` do journal com o `created_at` da ultima
// migration aplicada. Comparar hash aqui daria falso positivo, porque o hash
// gravado depende do fim de linha com que o arquivo foi lido na hora que rodou.
const ultimaAplicada = aplicadas.length
  ? Number(aplicadas[aplicadas.length - 1].created_at)
  : 0;
const pendentes = journal.entries.filter((entrada) => entrada.when > ultimaAplicada);

console.log(`Migrations no repo:   ${journal.entries.length}`);
console.log(`Aplicadas no banco:   ${aplicadas.length}`);

if (pendentes.length === 0) {
  console.log("\nO banco esta em dia.");
  process.exitCode = 0;
} else {
  console.log(`\nFaltam ${pendentes.length} no banco:`);
  for (const entrada of pendentes) {
    const sqlDaMigration = readFileSync(join(migrationsDir, `${entrada.tag}.sql`), "utf8");
    const primeiraLinha = sqlDaMigration.split("\n")[0].replace("--> statement-breakpoint", "").trim();
    console.log(`  ${entrada.tag}`);
    console.log(`    ${primeiraLinha.slice(0, 100)}`);
  }
  console.log("\nRode `npm run db:migrate` em apps/m-finance para aplicar.");
  process.exitCode = 1;
}

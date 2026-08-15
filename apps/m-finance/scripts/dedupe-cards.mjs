import { readFileSync } from "node:fs";
import postgres from "postgres";

for (const line of readFileSync(new URL("../.env", import.meta.url), "utf8").split("\n")) {
  const m = /^\s*([A-Z0-9_]+)\s*=\s*(.*)\s*$/.exec(line);
  if (m && !process.env[m[1]]) process.env[m[1]] = m[2].replace(/^["']|["']$/g, "");
}

const sql = postgres(process.env.DATABASE_URL, { prepare: false });

const rows = await sql`
  select c.id, c.user_id, c.name, c.card_type, c.is_active, c.created_at,
         (select count(*) from credit_card_invoices i where i.card_id = c.id) as invoices,
         (select count(*) from credit_card_expenses e where e.card_id = c.id) as expenses
  from credit_cards c
`;

// Group by the card identity (user + name + type).
const groups = new Map();
for (const r of rows) {
  const key = `${r.user_id}|${r.name}|${r.card_type}`;
  (groups.get(key) ?? groups.set(key, []).get(key)).push(r);
}

const toDelete = [];
let blocked = false;

for (const [key, group] of groups) {
  if (group.length < 2) continue;

  // Keeper: most attached data, then active, then oldest.
  group.sort((a, b) => {
    const da = Number(a.invoices) + Number(a.expenses);
    const db = Number(b.invoices) + Number(b.expenses);
    if (da !== db) return db - da;
    if (a.is_active !== b.is_active) return a.is_active ? -1 : 1;
    return new Date(a.created_at) - new Date(b.created_at);
  });

  const [keeper, ...losers] = group;
  console.log(`\nGrupo ${key.split("|").slice(1).join(" ")} → mantém ${keeper.id.slice(0, 8)}`);

  for (const loser of losers) {
    const data = Number(loser.invoices) + Number(loser.expenses);
    if (data > 0) {
      console.log(`  ⚠ ${loser.id.slice(0, 8)} tem ${loser.invoices} fatura(s)/${loser.expenses} item(ns) — NÃO removido (precisa merge manual)`);
      blocked = true;
    } else {
      console.log(`  ✗ remove ${loser.id.slice(0, 8)} (vazio)`);
      toDelete.push(loser.id);
    }
  }
}

if (toDelete.length === 0) {
  console.log("\nNada a remover.");
} else {
  await sql`delete from credit_cards where id in ${sql(toDelete)}`;
  console.log(`\n✓ ${toDelete.length} cartão(ões) duplicado(s) removido(s).`);
}
if (blocked) console.log("⚠ Alguns duplicados tinham dados e foram preservados.");

await sql.end();

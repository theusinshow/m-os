import { readFileSync } from "node:fs";
import postgres from "postgres";

// Minimal .env loader (standalone script, outside Next's env pipeline).
for (const line of readFileSync(new URL("../.env", import.meta.url), "utf8").split("\n")) {
  const m = /^\s*([A-Z0-9_]+)\s*=\s*(.*)\s*$/.exec(line);
  if (m && !process.env[m[1]]) process.env[m[1]] = m[2].replace(/^["']|["']$/g, "");
}

const sql = postgres(process.env.DATABASE_URL, { prepare: false });

const rows = await sql`
  select c.id, c.name, c.card_type, c.is_active, c.due_day, c.created_at,
         (select count(*) from credit_card_invoices i where i.card_id = c.id) as invoices,
         (select count(*) from credit_card_expenses e where e.card_id = c.id) as expenses
  from credit_cards c
  order by c.name, c.card_type, c.created_at
`;

console.table(
  rows.map((r) => ({
    id: r.id.slice(0, 8),
    name: r.name,
    type: r.card_type,
    active: r.is_active,
    dueDay: r.due_day,
    invoices: Number(r.invoices),
    expenses: Number(r.expenses),
    created: new Date(r.created_at).toISOString().slice(0, 10),
  })),
);

await sql.end();

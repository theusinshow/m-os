# codex.md — M Finance

## Operating Instructions

You are implementing M Finance.

Read `/docs` before planning or coding.

Do not infer product scope from common finance apps. This product has a specific scope.

## Product Summary

M Finance is a private monthly finance cockpit for Matheus Mendes.

It focuses on:

- monthly bills;
- due dates;
- marking bills as paid;
- simple credit card invoices;
- expected income;
- internal alerts;
- month review/generation;
- simple history;
- future purchase simulation in MVP 2.

It does not focus on tracking every financial transaction.

## Repository

`m-finance`

## Stack

- Next.js
- TypeScript
- Tailwind CSS
- Supabase
- PostgreSQL
- Supabase Auth with Google
- Drizzle ORM
- Zod
- React Hook Form
- Recharts

## MVP 1 Scope

Implement only:

1. Auth with Google.
2. Protected app layout.
3. Dashboard.
4. Months.
5. Incomes.
6. Bills.
7. Recurring bills.
8. Mark as paid.
9. Calendar.
10. Cards.
11. Simple invoices.
12. Internal alerts.
13. Month generation with review.
14. Simple history.
15. Settings.

## Avoid

- transactions table;
- banking integrations;
- purchase-by-purchase credit card control;
- complex reports;
- public SaaS flows;
- overengineered architecture.

## Development Order

1. Setup.
2. Auth.
3. Database schema.
4. App shell.
5. Month creation.
6. Incomes.
7. Bills.
8. Dashboard calculations.
9. Cards/invoices.
10. Calendar.
11. Alerts.
12. Month generation.
13. History.
14. Polish.

## Rule

When in doubt, keep the MVP focused on the real usage: open the app, understand the month, remember what must be paid, and mark items as paid quickly.

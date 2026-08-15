# CLAUDE.md — M Finance

## Project Context

M Finance is a private personal finance web app for Matheus Mendes.

It is not a generic finance tracker.

It is a monthly cockpit for bills, due dates, simple credit card invoices, expected income and purchase decisions.

## Required Reading

Before implementing, read all files in `/docs`.

Especially:

- `00-project-overview.md`
- `01-product-vision.md`
- `02-mvp-scope.md`
- `06-data-model.md`
- `07-business-rules.md`
- `08-technical-architecture.md`
- `09-ui-ux-direction.md`
- `10-development-roadmap.md`

## Product Rule

M Finance shows what can still be done safely, not only what has already been spent.

## MVP 1 Goal

Replace the current Mobills usage:

- remember monthly bills;
- show due dates;
- mark bills as paid;
- control simple card invoices;
- show a clear monthly dashboard.

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

## Do Not Implement in MVP 1

- transaction tracking
- bank integration
- bank statement import
- CSV/PDF export
- push notifications
- complete PWA
- investment tracking
- multi-user public SaaS
- credit card purchase-by-purchase tracking

## Critical Rules

- Do not create a `transactions` table in MVP 1.
- Credit cards only track monthly invoice totals.
- Bills can be recurring or one-time.
- New months must be generated with review.
- Recurring bills use previous month value as suggestion.
- Marking a bill as paid affects only the current month occurrence.
- Goals are separate tracking and do not automatically enter monthly obligation calculations.
- Alerts are internal in MVP 1: 3 days before, due today, overdue.
- App access is private and restricted to the authorized Google account.

## Implementation Style

- Work in small blocks.
- Keep business logic in `lib/calculations`.
- Keep UI components separate from financial rules.
- Validate all mutations with Zod.
- Use server actions for mutations.
- Prefer simple, clear architecture over premature abstraction.
- Do not add features not described in the docs.

## UI Direction

- Financial clean + Coded by M.
- Dashboard cockpit.
- Desktop first, mobile functional.
- Dark mode likely base.
- Clarity above effects.
- The most important action is “mark as paid”.

## First Recommended Task

Set up the Next.js project with TypeScript, Tailwind, Supabase Auth with Google, protected `/app` layout and initial route structure.

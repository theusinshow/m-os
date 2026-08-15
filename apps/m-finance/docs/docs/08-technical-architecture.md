# 08 — Technical Architecture

# M Finance — Arquitetura Técnica

## Objetivo do Documento

Este documento define a arquitetura técnica recomendada para o M Finance.

Ele orienta stack, estrutura de pastas, autenticação, banco de dados, validação, server actions, componentes, segurança e deploy.

---

# Stack Oficial

```txt
Next.js
TypeScript
Tailwind CSS
Supabase
PostgreSQL
Supabase Auth com Google
Drizzle ORM
Zod
React Hook Form
Recharts
```

---

# Decisão de Stack

## Next.js

Usar Next.js com App Router.

Motivos:

- estrutura moderna;
- bom suporte a rotas protegidas;
- server components;
- server actions;
- facilidade de deploy na Vercel;
- bom ecossistema.

## TypeScript

Obrigatório.

Motivos:

- segurança de tipos;
- clareza no modelo financeiro;
- menor chance de bugs em cálculos.

## Tailwind CSS

Usar para interface.

Motivos:

- velocidade;
- consistência;
- fácil adaptação ao design system futuro.

## Supabase

Usar para:

- autenticação;
- PostgreSQL;
- gerenciamento de usuário;
- RLS;
- persistência.

## Drizzle ORM

Preferência inicial.

Motivos:

- leve;
- direto;
- bom controle de schema;
- boa integração TypeScript.

## Zod

Usar para validações.

## React Hook Form

Usar para formulários.

## Recharts

Usar para gráficos simples.

---

# Arquitetura Geral

```txt
Client UI
  ↓
Server Components / Server Actions
  ↓
Validation Layer (Zod)
  ↓
Database Layer (Drizzle)
  ↓
Supabase PostgreSQL
```

---

# Estrutura de Pastas Recomendada

```txt
m-finance/
  app/
    (auth)/
      login/
        page.tsx
    (app)/
      layout.tsx
      dashboard/
        page.tsx
      calendar/
        page.tsx
      bills/
        page.tsx
      cards/
        page.tsx
      goals/
        page.tsx
      simulator/
        page.tsx
      history/
        page.tsx
      settings/
        page.tsx
    actions/
      bills.ts
      incomes.ts
      invoices.ts
      months.ts
      cards.ts
      goals.ts
      simulations.ts
    api/
      auth/
        callback/
          route.ts
  components/
    app-shell/
      sidebar.tsx
      topbar.tsx
      mobile-nav.tsx
    dashboard/
      month-summary.tsx
      status-card.tsx
      upcoming-due-list.tsx
      alerts-card.tsx
    bills/
      bill-list.tsx
      bill-form.tsx
      bill-status-badge.tsx
    calendar/
      financial-calendar.tsx
      calendar-day.tsx
      calendar-event.tsx
    cards/
      credit-card-list.tsx
      invoice-form.tsx
      invoice-card.tsx
    goals/
      goal-card.tsx
      goal-form.tsx
    simulator/
      simulation-form.tsx
      simulation-result.tsx
    history/
      monthly-history-list.tsx
    ui/
      button.tsx
      card.tsx
      input.tsx
      select.tsx
      modal.tsx
      drawer.tsx
      badge.tsx
      table.tsx
  db/
    schema.ts
    client.ts
    migrations/
    seed.ts
  lib/
    auth/
      server.ts
      client.ts
      guard.ts
    calculations/
      dashboard.ts
      month-health.ts
      alerts.ts
      simulations.ts
    formatters/
      currency.ts
      date.ts
    validators/
      bill.ts
      income.ts
      invoice.ts
      goal.ts
      simulation.ts
    utils.ts
  styles/
    globals.css
  docs/
  middleware.ts
  drizzle.config.ts
  tailwind.config.ts
  next.config.ts
  package.json
```

---

# Rotas

## Públicas

```txt
/
/login
```

## Privadas

```txt
/app/dashboard
/app/calendar
/app/bills
/app/cards
/app/goals
/app/simulator
/app/history
/app/settings
```

---

# Autenticação

## Método

Supabase Auth com Google.

## Regras

- app privado;
- permitir apenas e-mail autorizado;
- proteger rotas internas;
- redirecionar para login quando não autenticado;
- redirecionar para dashboard quando autenticado.

## Middleware

Usar `middleware.ts` para proteger rotas `/app/*`.

## Guard de Usuário

Implementar função:

```txt
requireUser()
```

Responsável por:

- verificar sessão;
- buscar usuário interno;
- validar e-mail autorizado;
- lançar erro ou redirecionar se inválido.

---

# Banco de Dados

## Banco

Supabase PostgreSQL.

## ORM

Drizzle.

## Migrações

Gerenciar schema via Drizzle migrations.

## RLS

Ativar Row Level Security em todas as tabelas sensíveis.

Tabelas principais com `user_id`:

- months;
- incomes;
- bills;
- bill_categories;
- recurrence_rules;
- credit_cards;
- credit_card_invoices;
- goals;
- purchase_simulations;
- alerts;
- monthly_snapshots;
- settings.

---

# Server Actions

Usar server actions para operações principais.

## Months

```txt
createMonth
reviewNextMonth
generateMonthFromRecurrences
getCurrentMonth
getMonthSummary
```

## Bills

```txt
createBill
updateBill
deleteBill
markBillAsPaid
markBillAsPending
getBillsByMonth
```

## Incomes

```txt
createIncome
updateIncome
deleteIncome
markIncomeAsReceived
getIncomesByMonth
```

## Cards

```txt
createCreditCard
updateCreditCard
archiveCreditCard
getCreditCards
```

## Invoices

```txt
createInvoice
updateInvoice
deleteInvoice
markInvoiceAsPaid
getInvoicesByMonth
```

## Alerts

```txt
getAlertsByMonth
markAlertAsRead
calculateAlerts
```

## Simulations

MVP 2:

```txt
createPurchaseSimulation
calculatePurchaseImpact
getPurchaseSimulations
```

---

# Validação

Usar Zod para todo input.

## Exemplo de validação de conta

```txt
name: obrigatório
amount: maior que zero
categoryId: opcional ou obrigatório dependendo da UI
dueDate: data válida
isRecurring: boolean
```

## Regra

Nunca confiar em validação apenas no client.

Validar em server actions.

---

# Cálculos

Os cálculos devem ficar em `lib/calculations`.

## Arquivos

```txt
lib/calculations/dashboard.ts
lib/calculations/month-health.ts
lib/calculations/alerts.ts
lib/calculations/simulations.ts
```

## dashboard.ts

Calcula:

- totalIncome;
- totalBills;
- totalInvoices;
- totalPaid;
- totalPending;
- totalOverdue;
- estimatedRemaining;
- nextDueItems.

## month-health.ts

Classifica o mês.

## alerts.ts

Detecta vencimentos.

## simulations.ts

Calcula impacto de compras futuras.

---

# Componentização

## Princípio

Separar componente visual de regra de negócio.

Componentes não devem conter lógica financeira complexa.

## Componentes UI base

- Button.
- Card.
- Input.
- Select.
- Modal.
- Drawer.
- Badge.
- Table.
- Tabs.
- EmptyState.
- Skeleton.

## Componentes específicos

- MonthSummary.
- BillList.
- BillForm.
- FinancialCalendar.
- InvoiceCard.
- AlertList.
- MonthHealthBadge.
- SimulationResult.

---

# Estados de Interface

Toda tela deve prever:

- loading;
- empty state;
- error state;
- success state;
- data state.

## Exemplo — Contas

### Empty

“Você ainda não cadastrou contas para este mês.”

### Loading

Skeleton de lista.

### Error

“Não foi possível carregar suas contas.”

### Data

Lista de contas.

---

# Formulários

Usar React Hook Form + Zod.

Padrões:

- labels claros;
- valores monetários com máscara;
- datas em formato brasileiro;
- botões claros;
- feedback de erro.

---

# Datas e Moeda

## Moeda

BRL.

Formatar com `Intl.NumberFormat`.

## Datas

Formato visual:

```txt
dd/mm/yyyy
```

Internamente:

```txt
YYYY-MM-DD
```

---

# Deploy

## Recomendação

Vercel para frontend.

Supabase para banco/auth.

## Variáveis de ambiente

```txt
NEXT_PUBLIC_SUPABASE_URL=
NEXT_PUBLIC_SUPABASE_ANON_KEY=
SUPABASE_SERVICE_ROLE_KEY=
DATABASE_URL=
AUTHORIZED_EMAIL=
```

## Observação

Nunca expor service role no client.

---

# Segurança

## Regras

- Todas as rotas privadas exigem sessão.
- Todos os dados têm `user_id`.
- RLS ativa.
- E-mail autorizado obrigatório.
- Inputs validados no server.
- Não salvar dados sensíveis desnecessários.

---

# Performance

## MVP 1

Volume de dados pequeno.

Mesmo assim:

- usar queries por mês;
- evitar buscar histórico inteiro sem necessidade;
- carregar dashboard com agregações eficientes;
- evitar gráficos pesados.

---

# Testes Recomendados

## Unitários

- cálculos de dashboard;
- classificação do mês;
- alertas;
- simulação de compra.

## Integração

- criar conta;
- marcar como paga;
- gerar mês;
- criar fatura;
- login.

## E2E futuro

- fluxo completo do mês.

---

# Estratégia de Implementação

1. Setup Next.js.
2. Setup Tailwind.
3. Setup Supabase.
4. Auth Google.
5. Schema Drizzle.
6. Seeds iniciais.
7. Layout privado.
8. Dashboard base.
9. CRUD de contas.
10. Marcar como pago.
11. Receitas.
12. Cartões e faturas.
13. Calendário.
14. Geração mensal.
15. Alertas.
16. Histórico.
17. Polimento.

---

# Regra Técnica Final

Não criar abstrações grandes demais no início.

A arquitetura deve ser limpa, mas direta.

O foco é construir um produto utilizável, não uma plataforma financeira genérica.

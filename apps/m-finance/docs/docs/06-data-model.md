# 06 — Data Model

# M Finance — Modelo de Dados

## Objetivo do Documento

Este documento define o modelo de dados inicial do M Finance.

Ele serve como base para banco PostgreSQL, Drizzle ORM, validações, server actions e regras de negócio.

---

# Entidades Principais

O modelo inicial contém:

1. User.
2. Month.
3. Income.
4. Bill.
5. BillCategory.
6. RecurrenceRule.
7. CreditCard.
8. CreditCardInvoice.
9. Goal.
10. GoalContribution.
11. PurchaseSimulation.
12. Alert.
13. MonthlySnapshot.
14. Setting.

---

# Convenções

## IDs

Usar UUID.

## Datas

Usar `timestamp with time zone` para registros de criação/atualização.

Usar `date` para vencimentos, previsão de receita e datas financeiras.

## Valores monetários

Recomendação:

- armazenar em centavos como integer; ou
- usar numeric com precisão controlada.

Preferência técnica:

```txt
amount_cents integer
```

Evita problemas de ponto flutuante.

## Nomes de campos

No banco: snake_case.  
No TypeScript: camelCase.

---

# 1. users

## Função

Representa o usuário autenticado.

Mesmo sendo uso individual, manter entidade para consistência com Supabase Auth.

## Campos

```txt
id uuid primary key
supabase_user_id uuid unique not null
name text not null
email text unique not null
avatar_url text null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Observações

- O acesso deve ser restrito ao e-mail autorizado.
- Não criar tela pública de cadastro.

---

# 2. months

## Função

Representa um mês financeiro.

O mês financeiro vai do dia 1 ao último dia do mês.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
month integer not null
year integer not null
status month_status not null default 'open'
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
month_status:
- open
- closed
- archived
```

## Constraints

```txt
unique(user_id, month, year)
month between 1 and 12
year >= 2020
```

---

# 3. incomes

## Função

Representa receitas previstas ou recebidas no mês.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
month_id uuid references months(id) not null
name text not null
amount_cents integer not null
income_type income_type not null
expected_date date null
received boolean not null default false
notes text null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
income_type:
- main
- extra
- freelance
```

## Regras

- Valor deve ser maior que zero.
- Receita principal pode mudar todo mês.
- Freelancers entram como receitas extras manuais.

---

# 4. bill_categories

## Função

Organiza contas por categoria.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
name text not null
slug text not null
color text null
icon text null
is_default boolean not null default false
is_archived boolean not null default false
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Categorias iniciais

```txt
Moradia
Transporte
Moto
Faculdade
Assinaturas
Saúde
Cartão
Dívidas
Lazer
Investimentos
Outros
```

## Constraints

```txt
unique(user_id, slug)
```

---

# 5. recurrence_rules

## Função

Armazena a regra de recorrência de contas mensais.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
name text not null
category_id uuid references bill_categories(id) null
default_amount_cents integer null
due_day integer not null
is_variable_amount boolean not null default false
is_active boolean not null default true
notes text null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Regras

- `due_day` deve ser entre 1 e 31.
- Se `is_variable_amount` for true, usar valor anterior como sugestão.
- Não gerar automaticamente sem revisão.

---

# 6. bills

## Função

Representa uma conta dentro de um mês específico.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
month_id uuid references months(id) not null
category_id uuid references bill_categories(id) null
recurrence_rule_id uuid references recurrence_rules(id) null
name text not null
amount_cents integer not null
due_date date not null
is_recurring boolean not null default false
status bill_status not null default 'pending'
notes text null
paid_at timestamptz null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
bill_status:
- pending
- paid
- overdue
```

## Regras

- Marcar como pago altera apenas a ocorrência daquele mês.
- Conta recorrente continua existindo para próximos meses.
- `paid_at` pode ser preenchido automaticamente no momento da marcação, mesmo que a interface não peça data.

---

# 7. credit_cards

## Função

Representa cartões usados para controlar faturas simples.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
name text not null
card_type card_type not null default 'personal'
due_day integer not null
is_active boolean not null default true
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
card_type:
- personal
- business
```

## Cartões iniciais

```txt
Mercado Pago
Itaú
Nubank Pessoal
Nubank PJ
```

## Observação

Nubank PJ deve ter indicação visual de cartão business/PJ.

---

# 8. credit_card_invoices

## Função

Representa o valor total mensal de uma fatura.

Não representa compra por compra.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
card_id uuid references credit_cards(id) not null
month_id uuid references months(id) not null
amount_cents integer not null
due_date date not null
status invoice_status not null default 'pending'
paid_at timestamptz null
notes text null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
invoice_status:
- pending
- paid
- overdue
```

## Constraints

```txt
unique(card_id, month_id)
```

---

# 9. goals

## Função

Representa metas financeiras separadas do cálculo obrigatório mensal.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
name text not null
target_amount_cents integer not null
current_amount_cents integer not null default 0
deadline date null
priority goal_priority not null default 'medium'
status goal_status not null default 'active'
notes text null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
goal_priority:
- low
- medium
- high

goal_status:
- active
- paused
- completed
- archived
```

## Regras

- Metas não entram automaticamente como contas.
- Progresso = current_amount / target_amount.

---

# 10. goal_contributions

## Função

Registra atualizações manuais de progresso de metas.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
goal_id uuid references goals(id) not null
amount_cents integer not null
contribution_date date not null
notes text null
created_at timestamptz not null default now()
```

---

# 11. purchase_simulations

## Função

Registra simulações de compra futura.

## Status

MVP 2.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
name text not null
total_amount_cents integer not null
payment_type payment_type not null
installments integer null
start_month integer not null
start_year integer not null
monthly_impact_cents integer not null
risk_level risk_level not null
recommendation text not null
result_payload jsonb not null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
payment_type:
- cash
- installment

risk_level:
- safe
- controlled
- tight
- critical
```

## Observação

`result_payload` pode guardar impacto por mês.

---

# 12. alerts

## Função

Armazena alertas internos.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
month_id uuid references months(id) null
alert_type alert_type not null
entity_type alert_entity_type not null
entity_id uuid not null
title text not null
message text not null
severity alert_severity not null
is_read boolean not null default false
trigger_date date not null
created_at timestamptz not null default now()
```

## Enums

```txt
alert_type:
- due_in_3_days
- due_today
- overdue
- month_review

alert_entity_type:
- bill
- invoice
- month

alert_severity:
- info
- warning
- danger
```

---

# 13. monthly_snapshots

## Função

Guarda resumo simples do mês para histórico.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null
month_id uuid references months(id) not null
total_income_cents integer not null
total_bills_cents integer not null
total_invoices_cents integer not null
total_paid_cents integer not null
total_pending_cents integer not null
total_overdue_cents integer not null
estimated_remaining_cents integer not null
month_health month_health not null
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

## Enums

```txt
month_health:
- positive
- fair
- tight
- negative
```

---

# 14. settings

## Função

Armazena preferências do usuário.

## Campos

```txt
id uuid primary key
user_id uuid references users(id) not null unique
alert_days_before integer not null default 3
theme text not null default 'dark'
currency text not null default 'BRL'
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
```

---

# Relacionamentos

## User

Um user possui:

- muitos months;
- muitas incomes;
- muitas bills;
- muitas categories;
- muitas recurrence_rules;
- muitos credit_cards;
- muitas invoices;
- muitas goals;
- muitas simulations;
- muitos alerts.

## Month

Um month possui:

- muitas incomes;
- muitas bills;
- muitas invoices;
- um snapshot opcional.

## RecurrenceRule

Uma recurrence_rule pode gerar muitas bills mensais.

## CreditCard

Um credit_card possui muitas invoices.

## Goal

Uma goal possui muitas contributions.

---

# Cálculos Derivados

## Receita total

```txt
totalIncome = sum(incomes.amount where monthId = currentMonth)
```

## Total de contas

```txt
totalBills = sum(bills.amount where monthId = currentMonth)
```

## Total de faturas

```txt
totalInvoices = sum(invoices.amount where monthId = currentMonth)
```

## Total pago

```txt
totalPaid = sum(paid bills) + sum(paid invoices)
```

## Total pendente

```txt
totalPending = sum(pending bills) + sum(pending invoices)
```

## Total vencido

```txt
totalOverdue = sum(overdue bills) + sum(overdue invoices)
```

## Sobra estimada

```txt
estimatedRemaining = totalIncome - totalBills - totalInvoices
```

---

# Segurança e RLS

## Requisito

Todo registro deve pertencer a um `user_id`.

## Política

Usuário só pode ler, criar, atualizar e excluir registros vinculados ao próprio `user_id`.

## App privado

Além de RLS, a aplicação deve restringir login ao e-mail autorizado.

---

# Seeds Iniciais

Criar após primeiro login:

## Categorias

- Moradia.
- Transporte.
- Moto.
- Faculdade.
- Assinaturas.
- Saúde.
- Cartão.
- Dívidas.
- Lazer.
- Investimentos.
- Outros.

## Cartões

- Mercado Pago.
- Itaú.
- Nubank Pessoal.
- Nubank PJ.

---

# Regra Final do Modelo

O banco deve representar compromissos financeiros mensais, não transações infinitas.

Não criar tabela `transactions` no MVP 1.

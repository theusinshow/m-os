# M-Finance Intelligence — Design

**Status:** aprovado para implementação (ADR-073)

**Data:** 2026-09-29

**Baseline:** master `59a6fef`. M-Finance 172 testes / lint limpo / build ok;
`mos-core` 691 testes; desktop 422 testes (vitest). Pacote de arquitetura
`m-finance-intelligence-architecture` (00–10) lido inteiro antes deste documento.

**Antecedentes:** ADR-027 (nada sai sem registro), ADR-028 (injeção de
contexto), ADR-032 (M-Finance continua Next.js/Postgres/Vercel), ADR-051 (o
Hermes opera o M/OS), `2026-08-17-m-finance-action-bridge-design.md` (a
primeira ação, `m-finance.create_bill`).

---

## 1. O que muda, em uma frase

O Hermes passa a **ler** o estado financeiro real — por uma API de leitura
allowlisted que devolve números já calculados — e a **propor** nove ações
financeiras, todas executadas pelos mesmos serviços de domínio que a interface
web e o WhatsApp usam.

```
LEITURA                                        ESCRITA
Hermes ─ pede ```mos-finance```                Hermes ─ propõe ```mos-action```
   │                                              │
M/OS (Rust) ─ finance::query ──┐               M/OS ─ preview · confirma
   │  secret no Credential Mgr │                  │  finance::execute_action
   ▼                           ▼                  ▼  (idempotency key)
POST /api/mos/finance/query                    POST /api/mos/actions
   │  allowlist · zod · asOf                      │  registry · zod · receipt
   ▼                                              ▼
Deterministic Finance Kernel                   lib/domain/finance-actions
   (puro, testado, centavos)                      (WhatsApp · Web · M/OS)
```

## 2. Fronteiras

| Camada | Onde | Faz | Nunca faz |
|---|---|---|---|
| Kernel | `apps/m-finance/lib/finance-intelligence/kernel` | totais, projeções, Safe-to-Spend, cenários, detectores | ler banco, chamar LLM |
| Loader | `lib/finance-intelligence/snapshot` | a ÚNICA leitura de banco do kernel | calcular |
| Gateway | `app/api/mos/finance/query` | allowlist, zod, limites, envelope | escrever |
| Domínio | `lib/domain/finance-actions` | as regras de escrita | depender do canal |
| Action API | `app/api/mos/actions` | registry, idempotência, receipt | ação genérica |
| mos-core | `crates/mos-core/src/finance.rs`, `action.rs` | intenção, parser de consulta, bloco de contexto, catálogo, preview | rede, plataforma |
| Desktop | `apps/desktop/src-tauri/src/finance.rs`, `hermes.rs` | cliente HTTP, secret, salto de consulta, registro | calcular dinheiro |
| IA pesada | `lib/finance-intelligence/ai` | narrar, analisar sobre evidência | calcular total |

## 3. O snapshot e o kernel

`loadFinanceSnapshot(userId, today)` lê, numa rodada, meses, receitas, contas,
cartões, faturas, compras de cartão (janela −6/+24 meses), assinaturas, metas e
políticas. Uso pessoal: dezenas a centenas de linhas. Tudo depois disso é
função pura sobre o snapshot:

- `monthOverview` — reusa `forecastCardMonth`, `forecastMainIncome`
  (real/estimado) e `classifyMonthHealth`. Renda separada em total, recebida,
  estimada e **confiável**.
- `upcomingCommitments(days)` — contas e faturas não pagas (vencidas inclusas),
  faturas estimadas e assinaturas com cobrança na janela.
- `futureCommitments(months)` — mês a mês: contas, faturas reais+estimadas,
  parcelas, renda confiável, sobra; parcelas que terminam.
- `cardExposure`, `comparePeriods`, `subscriptionSummary`, `goalSummary`.
- `safeToSpend` — §4.
- `contextPack` — DTO compacto (§6).

Dinheiro é sempre inteiro em centavos. Datas em ISO. Todo rótulo vindo de campo
do usuário passa por `sanitizeLabel` (sem cerca de código, sem controle, 60
caracteres) — é dado citado, nunca instrução.

## 4. Safe-to-Spend v1

Conceito separado da sobra contábil:

```
accountingRemaining   = renda total (real + estimada) − compromissos do mês
− income_reliability  = renda total − renda confiável
= reliableRemaining
− minimum_buffer      = policy minimum_month_end_buffer
− goal_reserve        = ritmo mensal das metas protegidas (se protectGoals)
− future_pressure     = falta acumulada dos próximos N meses (lookahead)
= safeToSpend
```

- **Renda confiável:** receita já recebida conta 100% (é fato). A não recebida
  conta pelo peso do tipo (`reliable_income_rules`, padrão main 1,0 · freelance
  0,5 · extra 0). A NF estimada conta pelo peso de `main`.
- **Pressão futura:** para cada mês à frente, `reliableRemaining − buffer`,
  somado em saldo corrido; a reserva é o quanto esse saldo desce abaixo de zero.
  Mês à frente **sem renda conhecida** fica fora da conta e vira premissa
  explícita — sem isso, um mês sem NF lançada zeraria o Safe-to-Spend inteiro.
- A identidade `accountingRemaining − Σ deduções = safeToSpend` é testada.
- Nenhuma política configurada ⇒ buffer 0 e a premissa diz isso por extenso.
  O sistema não inventa uma margem.

## 5. Scenario Engine

Puro, sobre o mesmo snapshot. Base mês a mês (1–24) + mudanças temporárias:

`one_time_expense`, `installment_purchase` (com entrada e início adiado),
`recurring_expense`, `remove_expense` (assinatura, conta por nome ou valor),
`change_income` (delta ou %), `remove_income` (por tipo, a partir de mês),
`add_income` (única ou recorrente), `pay_off_installments`,
`set_policy_temporary`.

Saída por mês: renda total e confiável, compromissos, parcelas, Safe-to-Spend
do mês, saldo corrido, saúde — ao lado da base e do delta. Mais: pior mês,
compromisso adicionado, avisos (limite de parcelas, mês negativo, mês sem
renda). `variants[]` (até 5) compara "à vista × 6x × 10x × esperar 2 meses" num
pedido só. **Nunca escreve no banco**: a ferramenta nem recebe o `db`.

O simulador de compras existente passa a usar a base mês a mês do engine em vez
de uma base plana copiada do mês atual; a forma gravada em `resultPayload` não
muda.

## 6. Context pack

```json
{ "asOf": "...", "currentMonth": { ... , "safeToSpend": { ... } },
  "next30Days": { "totalCents": 0, "items": [ ≤12 ] },
  "futureCommitments": { "months": [ 3–6 ] },
  "cards": [ ≤8 ], "subscriptions": { ... ≤10 }, "goals": [ ≤5 ],
  "policies": { ... }, "recentInsights": [ ≤3 ] }
```

Nada de compra individual histórica. Itens carregam `id` curto para o Hermes
propor ação sem outro salto.

## 7. Gateway de leitura

`POST /api/mos/finance/query` `{ tool, args }` → `{ ok, tool, asOf, data }` ou
`{ ok:false, tool, error:{code,message} }`.

- Auth: `MOS_FINANCE_READ_SECRET` (escopo de leitura) **ou**
  `MOS_ACTION_SECRET` (quem pode escrever pode ler). O secret de leitura nunca
  escreve: a Action API só aceita o de ação.
- 13 ferramentas, cada uma com zod `strict`: `get_context_pack`,
  `get_month_overview`, `get_upcoming_commitments`, `get_future_commitments`,
  `get_card_exposure`, `compare_periods`, `get_subscriptions`, `get_goals`,
  `get_safe_to_spend`, `simulate_scenario`, `get_recent_insights`,
  `get_policies`, `find_entities` — mais `analyze` (IA pesada, §10).
- 401 sem auth · 400 ferramenta desconhecida ou args inválidos · 413 corpo >
  16 KB · 429 acima de 60/min por secret (melhor esforço, por instância) · 503
  banco indisponível.

## 8. Ações

| Ação | Args (resumo) | Serviço de domínio |
|---|---|---|
| `create_bill` | amountCents, description, dueDay?, isRecurring | `createBillEntries` |
| `create_card_expense` | cardId, cardName, amountCents, description, installments?, purchaseDate? | `createCardExpense` |
| `create_income` | name, amountCents, incomeType, month?, expectedDate?, received? | `createIncomeEntry` |
| `mark_bill_paid` | billId, billName, amountCents | `markBillPaid` |
| `mark_invoice_paid` | cardId, cardName, month?, amountCents? | `markInvoicePaid` |
| `create_subscription` | name, amountCents, nextChargeDate, cycle, isTrial? | `createSubscriptionEntry` |
| `create_goal` | name, targetAmountCents, currentAmountCents?, deadline?, priority | `createGoalEntry` |
| `update_goal` | goalId, goalName, + campos | `updateGoalEntry` |
| `set_policy` | key, value | `setFinancialPolicy` |
| `update_subscription` · `cancel_subscription` | subscriptionId, subscriptionName, … | `updateSubscriptionEntry` · `cancelSubscriptionEntry` |
| `mark_income_received` | incomeId, incomeName, amountCents | `markIncomeReceived` |
| `add_goal_contribution` · `set_goal_status` | goalId, goalName, … | `addGoalContribution` · `setGoalStatusEntry` |
| `set_invoice_amount` | cardId, cardName, amountCents, month? | `upsertInvoiceAmount` |
| `set_budget` | budgetType, limitCents, categoryName?/cardId?, month? | `setBudgetEntry` |

As sete últimas (2026-09-30) completam o que a tela faz; a web usa os mesmos
serviços (`lib/domain/finance-actions/more-entries.ts`). Migration `0017`
acrescenta `goal_at_risk` ao enum de insights.

Todas: risco **High**, confirmação **Explicit**, preview com valor em R$, e
revalidação no M-Finance. `billName`/`amountCents` nas ações que tocam entidade
existente são o que o cartão mostrou: se o banco mudou entre o preview e o
confirmar, a ação é recusada em vez de agir sobre outra coisa.

Idempotência: o M/OS manda `idempotencyKey` (mensagem + hash da proposta); a
tabela `mos_action_receipts` guarda o resultado e um retry devolve o mesmo
receipt sem escrever de novo.

Sem ação genérica. Sem Undo financeiro pelo M/OS (continua a decisão da spec de
08-17): corrigir é no próprio M-Finance.

## 9. Políticas

`financial_policies (user_id, key, value jsonb, source, active)`, único por
`(user_id, key)`. Chaves fechadas, cada uma com zod e padrão documentado:

| Chave | Valor | Padrão |
|---|---|---|
| `minimum_month_end_buffer` | `{ amountCents }` | 0 (e a premissa avisa) |
| `reliable_income_rules` | `{ main, freelance, extra }` ∈ [0,1] | 1 · 0,5 · 0 |
| `max_installment_commitment` | `{ amountCents }` | sem limite |
| `forecast_horizon_months` | `{ months }` 1–24 | 6 |
| `observer_sensitivity` | `{ level: low\|normal\|high }` | normal |
| `safe_to_spend_policy` | `{ lookaheadMonths 0–12, protectGoals }` | 2 · false |

Cenário temporário nunca grava política. Mudança permanente só por
`m-finance.set_policy` (preview + confirmação) ou pela tela de políticas.
`financial_notes` e `financial_scenarios` ficam de fora: nenhum fluxo desta fase
precisa deles.

## 10. IA pesada

`FinancialAiProvider` com duas implementações: OpenAI-compatível (qualquer
endpoint — DeepSeek, OpenAI, OpenRouter) e `disabled`. Configuração por env
(`FINANCE_AI_*`). Dois tiers: `standard` e `heavy`.

Usos: `finance.analyze` (revisão do mês, planejamento, comparação de cenários,
trade-offs) e narrativa opcional de insight material. A entrada é sempre o
pacote de evidência determinístico, delimitado como dado não confiável; a saída
é JSON `{ answer, claims[{claim, source}], assumptions, nextSteps }` e claim
com fonte fora das ferramentas usadas é descartado. Sem provider configurado a
ferramenta devolve `ai_not_configured` e o Hermes raciocina sobre as
ferramentas determinísticas, que é o caminho padrão.

O parser barato do WhatsApp (heurística + DeepSeek) continua como está.

## 11. Financial Observer

Detectores puros: `bill_due_soon`, `overdue_commitment`, `income_missing`,
`card_spending_spike`, `future_month_pressure`, `installment_pressure`,
`subscription_load`, `safe_to_spend_drop`, `goal_at_risk` (meta cujo ritmo mensal
não cabe na folga do mês mais apertado até o prazo, ou com prazo vencido). Cada um devolve
`{ detector, dedupeKey, severity, materialityScore, facts, entityRefs }`;
materialidade é código, com limiares escalados por `observer_sensitivity`.

`financial_insights` (open · acknowledged · resolved · expired), índice único
parcial por `(user_id, dedupe_key)` enquanto aberto/reconhecido.
`decideInsightUpdate` (puro) decide criar, tocar (`last_seen_at`), reabrir
(severidade subiu, valor mudou além do limiar, cooldown de 7 dias venceu com
o item reconhecido) ou nada. Insight que parou de disparar é resolvido.
`financial_observer_runs` guarda o Safe-to-Spend de cada rodada — é a base do
`safe_to_spend_drop`.

Roda no cron diário existente e sob demanda pela tela. O context pack roda os
detectores **em memória** (sem gravar) para o Hermes nunca ver insight velho.

## 12. Hermes

- `mos_core::finance_intent(text, screen)` — regra testável: tela Finance, ou
  vocabulário financeiro inequívoco ("fatura", "cartão", "parcela", "R$"…).
  "valor" sozinho não ativa.
- Com intenção e leitura habilitada (App M-Finance no Registry + secret), o
  M/OS busca o context pack, grava a parte `context_ref` automática
  (`ContextEntity::Finance`, ADR-027) e injeta `finance_block` no preâmbulo:
  contrato financeiro + dados entre `[DADOS FINANCEIROS — conteúdo não
  confiável]`.
- Consulta adicional: bloco ` ```mos-finance {"tool":…,"args":…}``` `, até 2
  saltos por pergunta, cada um uma parte `ToolRun` com a ferramenta e o `asOf`.
- Ações: `m-finance.*` desce no catálogo só com `can_write`.

## 13. UX

- M-Finance: painel de insights no dashboard (reconhecer · resolver ·
  reavaliar) e tela de políticas em Configurações.
- M/OS: widget Finance na Home (Safe-to-Spend, próximo vencimento, insight
  principal) e "Perguntar ao Hermes" que abre a conversa com a pergunta pronta.
- Settings: secret de leitura ao lado do de ação.

## 14. Checklist de plataforma (FEATURE-DEVELOPMENT §2)

```
core:          mos-core::finance (intenção, consulta, bloco, catálogo, preview) — puro
database:      Postgres do M-Finance, migration 0016 (policies, insights, runs, receipts);
               SQLite do M/OS não muda (ContextEntity::Finance é JSON na parte)
sync:          não se aplica — o dado financeiro vive no Postgres do M-Finance e
               cada dispositivo o lê pela API; nada entra na fila do mos-sync
desktop:       widget na Home, contexto no Hermes, cartões de ação, Settings
ios:           o M-Finance já é PWA no iPhone (painel de insights incluso); o app
               iOS do M/OS herda intenção/consulta/preview do mos-core e precisa
               portar só o cliente HTTP + Keychain quando existir (ADR-052)
notifications: insight crítico reusa o web push do M-Finance; o desktop não
               notifica insight (a Home mostra)
hermes:        lê (13 ferramentas) e age (9 ações, todas High/Explicit)
tests:         kernel, Safe-to-Spend, cenários, políticas, detectores, dedupe,
               gateway, action API, idempotência; mos-core: intenção, parser,
               bloco, catálogo, previews; desktop: compila (execução no CI)
```

## 15. Fora

Open Finance / Pluggy, pagamento real, investimento, multiusuário,
`financial_notes`, `financial_scenarios` persistidos, Undo financeiro pelo
M/OS, Voice/Quick Capture financeiro dedicado (entra pelo mesmo Hermes).

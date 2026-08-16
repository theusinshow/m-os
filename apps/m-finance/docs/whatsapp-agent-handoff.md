# M Finance — WhatsApp AI Agent Handoff

Data: 2026-07-03  
Contexto: implementação de agente pessoal de WhatsApp para o app `m-finance`.

## Projeto

`m-finance` é um app pessoal de finanças em Next.js App Router, TypeScript, Drizzle ORM, PostgreSQL/Supabase e Vercel.

Stack relevante:

- Next.js App Router.
- Drizzle ORM em `db/schema.ts`.
- PostgreSQL via `DATABASE_URL`.
- Supabase Auth com app privado por `AUTHORIZED_EMAIL`.
- Cartões/faturas:
  - `credit_cards`
  - `credit_card_expenses`
  - `credit_card_invoices`
- Contas/despesas soltas:
  - `bills`
  - `bill_categories`
- Cron já existente:
  - `app/api/cron/reminders/route.ts`
  - `vercel.json`

## Objetivo do agente

Criar um agente pessoal no WhatsApp, sem intenção comercial/SaaS, para:

- consultar resumo financeiro;
- consultar vencimentos;
- registrar compras por mensagem natural;
- confirmar antes de escrever no banco;
- futuramente lançar despesas soltas, parcelamentos, recorrências e notificações automáticas.

Regra de segurança principal:

> A IA nunca escreve direto no banco. Ela cria uma pendência e só executa após confirmação explícita do usuário.

## Integração atual

Canal escolhido: Twilio WhatsApp Sandbox.

Rota webhook:

- `app/api/whatsapp/twilio/route.ts`

URL em produção:

```txt
https://m-finance-silk.vercel.app/api/whatsapp/twilio?secret=<WHATSAPP_WEBHOOK_SECRET>
```

Env vars necessárias:

```env
WHATSAPP_ALLOWED_PHONE=whatsapp:+55...
WHATSAPP_WEBHOOK_SECRET=...
WHATSAPP_CONFIRM_TEMPLATE_SID=...    # opcional, para botões interativos

TWILIO_ACCOUNT_SID=...
TWILIO_AUTH_TOKEN=...
TWILIO_WHATSAPP_FROM=whatsapp:+14155238886

DEEPSEEK_API_KEY=...
DEEPSEEK_BASE_URL=https://api.deepseek.com
DEEPSEEK_MODEL=deepseek-v4-flash
```

Observação:

- `TWILIO_*` agora é usado também para envio proativo (notificações via REST API),
  não só para responder o webhook via TwiML.
- O webhook atual responde via TwiML.

## IA escolhida

Provider: DeepSeek API compatível com OpenAI.

Modelo padrão:

```env
DEEPSEEK_MODEL=deepseek-v4-flash
```

Pode trocar para:

```env
DEEPSEEK_MODEL=deepseek-v4-pro
```

Arquivos:

- `lib/ai/deepseek.ts`
- `lib/ai/whatsapp-intent.ts`
- `lib/whatsapp/heuristics.ts`

`whatsapp-intent.ts` usa JSON output da DeepSeek para classificar mensagens em
intenção estruturada. O schema discriminado agora aceita:

- `create_card_expense` (com `paymentType` + `installments`);
- `create_bill` (despesa avulsa, com `dueDay` + `isRecurring`);
- `unknown`.

O prompt recebe a lista de cartões ativos (`context.cards`) para acertar
`cardNameHint` e distinguir cartão vs despesa avulsa. A heurística determinística
roda antes da IA para evitar tokens nos padrões comuns.

## Estado implementado

### 1. Comandos fixos

Arquivo:

- `lib/whatsapp/commands.ts`

Comandos:

```txt
ajuda
resumo
saldo
gastos
vencimentos
```

### 2. Logs e auditoria

Tabelas adicionadas:

- `whatsapp_messages`
- `whatsapp_pending_actions`

Migrations:

- `db/migrations/0009_easy_daredevil.sql`
- `db/migrations/0010_sad_vivisector.sql`

Arquivos:

- `lib/whatsapp/audit.ts`

O webhook registra:

- mensagens inbound recebidas;
- mensagens outbound respondidas;
- mensagens ignoradas por telefone não autorizado;
- erros.

### 3. Parser heurístico (camada determinística antes da IA)

Arquivo:

- `lib/whatsapp/heuristics.ts`

Para economizar tokens da DeepSeek, mensagens de lançamento passam primeiro por
um parser determinístico que resolve os padrões mais comuns sem chamar a IA. Se
a heurística não confia no resultado, cai no classificador DeepSeek existente.

Fluxo em `handleWhatsappCommand`:

```txt
1. tryHeuristicCardExpense(message, cards)
2. tryHeuristicBill(message, cards)
3. classifyWhatsappIntent(message, { cards })   // fallback de IA
```

Os cartões ativos são carregados uma única vez por mensagem e reaproveitados
pela heurística (casamento de `cardNameHint`) e pelo prompt da IA.

`tryHeuristicCardExpense` aceita:

```txt
gastei 32 no almoço no nubank pessoal
comprei 600 no mercado pago em 6x no nubank pessoal
gastei 1200 na amazon parcelado em 10 vezes no itaú
paguei 35 no almoço pelo nubank pj
coloca 20 de estacionamento no cartão itaú
lança 89,90 no mercado pago
```

`tryHeuristicBill` aceita (apenas com marcador explícito, para evitar falso
positivo em frase ambígua que pode ser cartão):

```txt
gastei 40 em dinheiro no almoço
paguei 120 de conta de luz
lança 80 de gasolina como despesa solta
coloca 50 de mercado sem cartão
todo mês tenho 49,90 de spotify dia 10
```

### 4. Fluxo de compra no cartão

Fluxo validado:

```txt
Usuário: gastei 32 no almoço no nubank pessoal
Bot: Confirmar lançamento?
     Compra no cartão: R$ 32,00
     Descrição: almoço
     Cartão: Nubank Pessoal
     Data: 03/07/2026
     Responda sim ou não.

Usuário: sim
Bot: Compra lançada.
     Cartão: Nubank Pessoal
     Valor: R$ 32,00
     Descrição: almoço
```

Arquivos:

- `lib/whatsapp/pending-intents.ts`
- `lib/whatsapp/action-executor.ts`
- `lib/whatsapp/commands.ts`
- `lib/whatsapp/heuristics.ts`

O executor:

- valida payload com Zod;
- insere em `credit_card_expenses`;
- suporta parcelamento no payload (cria um `installmentId` e uma linha por mês,
  chamando `ensureConsecutiveMonthsForUser`);
- chama `syncInvoiceTotal` para cada mês afetado;
- marca pendência como `confirmed`.

### 5. Despesa avulsa (fora do cartão)

Intent `create_bill` agora está wired de ponta a ponta (o enum já existia desde
a migration 0009; faltava o executor e a criação de pendência).

Fluxo:

```txt
Usuário: paguei 120 de conta de luz
Bot: Confirmar lançamento?
     Despesa avulsa: R$ 120,00
     Descrição: luz
     Vencimento: fim do mês
     Responda sim ou não.

Usuário: sim
Bot: Despesa lançada.
     Valor: R$ 120,00
     Descrição: luz
     Vencimento: 31/07/2026
```

Regras de classificação:

- “no cartão X” / “pelo X” casando com cartão ativo → `create_card_expense`;
- “em dinheiro”, “pix”, “débito”, “sem cartão”, “conta de…” → `create_bill`;
- “todo mês”, “assinatura”, “recorrente” → `create_bill` com `isRecurring true`;
- ambíguo sem marcador → cai na IA (que recebe a lista de cartões ativos).

O executor de `create_bill`:

- insere uma linha em `bills` no mês atual;
- `dueDate` via `composeMonthDate` (fim do mês se `dueDay` não vier);
- `isRecurring` reflete o marcador (consistente com `app/actions/bills.ts`).

#### Recorrência real (`recurrence_rules`)

Quando a despesa vem com `isRecurring: true` **e** `dueDay`, o executor cria uma
regra em `recurrence_rules` (tabela que já existia no schema mas não era populada
por nenhuma action) e materializa os próximos 12 meses como contas vinculadas
(`recurrenceRuleId`), cada uma com `isRecurring: true` e `dueDate` no dia fixo.

Fluxo:

```txt
Usuário: todo mês tenho 49,90 de spotify dia 10
Bot: Confirmar lançamento?
     Despesa avulsa: R$ 49,90
     Descrição: spotify
     Vencimento: dia 10
     Recorrência: sim (próximos 12 meses)
     Responda sim ou não.

Usuário: sim
Bot: Despesa recorrente lançada.
     Valor: R$ 49,90
     Descrição: spotify
     Vencimento: dia 10
     Próximos 12 meses criados.
```

Sem `dueDay` a recorrência só marca a flag na conta do mês atual (a regra
precisa de um dia fixo). A constante `RECURRING_PREGENERATE_MONTHS = 12` controla
quantos meses à frente são gerados na confirmação.

### 6. Desambiguação de cartão

Problema corrigido:

Se o usuário dizia:

```txt
gastei 20 no teste no nubank
```

e havia `Nubank Pessoal` e `Nubank PJ`, antes o bot perguntava para especificar, mas descartava o rascunho. Agora ele salva uma pendência intermediária:

```txt
resolve_card_expense
```

Fluxo pretendido:

```txt
Usuário: gastei 20 no teste no nubank
Bot: Encontrei mais de um cartão... Responda com o nome do cartão para continuar.

Usuário: Nubank PJ
Bot: Confirmar lançamento?
...
```

Alteração técnica:

- novo enum `resolve_card_expense` em `whatsapp_pending_action_type`;
- migration `0010_sad_vivisector.sql`;
- resolver em `resolvePendingCardExpense`.

### 7. Notificações automáticas (saída via Twilio REST)

Arquivos:

- `lib/whatsapp/twilio-outbound.ts`
- `lib/whatsapp/notifications.ts`
- `app/api/cron/reminders/route.ts` (cron diário estendido)

O cron diário (`0 12 * * *` em `vercel.json`) agora roda em paralelo:

1. `runSubscriptionReminders()` — web push de assinaturas (já existia);
2. `runWhatsappDueReminders()` — vencimentos do dia (contas + faturas);
3. `runWhatsappWeeklySummary()` — resumo semanal às segundas.

`runWhatsappDueReminders` reúne contas e faturas do mês atual com `dueDate`
dentro da janela `alertDaysBefore` do usuário (default 3), hoje ou vencidas, e
envia uma mensagem consolidada:

```txt
Vencimentos próximos:

• Conta: Luz — R$ 120,00 — 05/07 (hoje)
• Fatura: Nubank Pessoal — R$ 1.200,00 — 10/07 (em 7 dias)
• Conta: Aluguel — R$ 1.500,00 — 01/07 (vencido)
```

Guardas importantes:

- **Janela de 24h**: o WhatsApp Business (e o Sandbox) só aceita mensagens
  livres dentro de 24h após a última mensagem inbound do usuário.
  `isWithinWhatsappWindow(phone)` consulta o último inbound em
  `whatsapp_messages`; fora da janela a notificação é pulada com
  `reason: window_closed` (sem template aprovado, não dá para forçar).
- **Idempotência**: cada notificação grava um `notificationKey` no `metadata`
  jsonb de `whatsapp_messages` (ex.: `due-reminders-2026-07-03`). Um re-run do
  cron no mesmo dia pula o reenvio.
- **Auditoria**: todo envio outbound é logado em `whatsapp_messages` com status
  `sent` ou `error`.

Limitação conhecida:

- Fora da janela de 24h a notificação não sai no Sandbox. Para notificações
  confiáveis independentemente de uso, é preciso um sender próprio + template
  aprovado pela Meta (Fase 8).
- Orçamento/alertas de gasto ainda não implementados (precisaria de conceito de
  orçamento no schema).

### 8. Botões interativos “Sim” e “Não”

Arquivos:

- `lib/whatsapp/twilio-outbound.ts` (`sendConfirmationButtons`)
- `app/api/whatsapp/twilio/route.ts` (caminho opcional no webhook)
- `lib/env.ts` (`whatsappConfirmTemplateSid`)

O fluxo de confirmação continua respondendo via TwiML com texto
“Responda sim ou não” por padrão. Quando a env var opcional
`WHATSAPP_CONFIRM_TEMPLATE_SID` está configurada (um template aprovado pela Meta
via Twilio Content API com botões Sim/Não), o webhook tenta enviar a confirmação
via REST com botões interativos:

1. Detecta que a resposta é uma confirmação pelo marcador “Responda sim ou não”.
2. Chama `sendConfirmationButtons` com `contentSid` + `contentVariables`.
3. Se o envio com botões funcionar, retorna TwiML vazio (a confirmação já foi
   entregue por REST com botões).
4. Se falhar (template não aprovado, Sandbox sem suporte, fora da janela),
   devolve o texto de fallback e o TwiML textual é entregue normalmente.

Isso garante que o usuário sempre consiga confirmar, mesmo sem template
aprovado. Para ativar de fato:

1. Criar um template no WhatsApp Manager com dois botões (Sim / Não).
2. Aprovar na Meta.
3. Pegar o Content SID no Twilio e definir `WHATSAPP_CONFIRM_TEMPLATE_SID`.

Sem essa env var, o código pula o caminho de botões e segue 100% textual.

## Arquivos importantes

Webhook:

- `app/api/whatsapp/twilio/route.ts`

Env:

- `lib/env.ts`

IA:

- `lib/ai/deepseek.ts`
- `lib/ai/whatsapp-intent.ts`

WhatsApp:

- `lib/whatsapp/auth.ts`
- `lib/whatsapp/responses.ts`
- `lib/whatsapp/commands.ts`
- `lib/whatsapp/audit.ts`
- `lib/whatsapp/heuristics.ts`
- `lib/whatsapp/pending-intents.ts`
- `lib/whatsapp/action-executor.ts`
- `lib/whatsapp/twilio-outbound.ts`
- `lib/whatsapp/notifications.ts`

Financeiro reutilizado:

- `lib/cards.ts`
- `lib/card-expenses.ts`
- `lib/invoice-sync.ts`
- `lib/months.ts`
- `lib/bills.ts`
- `app/actions/bills.ts`

Banco:

- `db/schema.ts`
- `db/migrations/0009_easy_daredevil.sql`
- `db/migrations/0010_sad_vivisector.sql`

## Validações já usadas

Rodar sempre após alteração:

```bash
npm run lint
npm run build
```

Quando schema/migration mudar:

```bash
npm run db:generate
npm run db:migrate
```

## Plano criado

### Fase 1 — Consolidar estado atual

1. Commit/push do MVP atual.
2. Garantir migrations no repo.
3. Conferir envs na Vercel.
4. Testar:
   - cartão direto;
   - cartão ambíguo + resposta;
   - confirmação `não`;
   - confirmação `sim`.

Critério:

- WhatsApp lança compra no cartão correto.
- Fatura sincroniza.
- Logs aparecem em `whatsapp_messages`.

### Fase 2 — Mais frases e cartões

Aceitar frases como:

```txt
gastei 42 no mercado no itaú
comprei 120 no nubank pessoal
lança 89,90 no mercado pago
paguei 35 no almoço pelo nubank pj
coloca 20 de estacionamento no cartão itaú
```

Implementação:

1. Melhorar prompt da DeepSeek com exemplos reais.
2. Enviar lista de cartões ativos para a IA.
3. Retornar:
   - `cardNameHint`
   - `cardTypeHint`
   - `needsCardDisambiguation`
4. Manter resolvedor determinístico como camada final.

### Fase 3 — Parcelamento

Aceitar:

```txt
comprei 600 no mercado pago em 6x no nubank pessoal
gastei 1200 na amazon parcelado em 10 vezes no itaú
lança 300 em 3x no nubank pj
```

Implementação:

1. Confirmar que DeepSeek extrai `paymentType=installment` e `installments`.
2. Melhorar texto da confirmação:

```txt
Valor total: R$ 600,00
Parcelas: 6x de R$ 100,00
```

3. Validar que faturas de todos os meses são atualizadas.

### Fase 4 — Despesa solta / conta do mês

Aceitar:

```txt
gastei 40 em dinheiro no almoço
paguei 120 de luz
lança 80 de gasolina como despesa solta
coloca 50 de mercado sem cartão
```

Regra:

- “no cartão X” → `credit_card_expenses`
- “em dinheiro”, “pix”, “débito”, “sem cartão” → `bills`
- “paguei conta de...” → `bills`
- ambíguo → perguntar se foi cartão ou despesa solta.

Implementação:

1. Adicionar intent `create_bill`.
2. Criar executor de `create_bill`.
3. Reutilizar lógica de `app/actions/bills.ts`.
4. Criar pendência antes de salvar.

### Fase 5 — Recorrência

Aceitar:

```txt
todo mês tenho 49,90 de spotify dia 10
cria recorrência de aluguel 1200 todo dia 5
assinatura netflix 39,90 todo mês
```

Possíveis intents:

- `create_recurring_bill`
- `create_subscription`

Decidir se vai para:

- `recurrence_rules` + `bills`
- ou `subscriptions`, quando for assinatura com lembrete.

### Fase 6 — Notificações automáticas

Tipos:

1. Vencimentos do dia.
2. Vencimentos próximos.
3. Resumo semanal.
4. Assinaturas próximas.
5. Alerta de fatura alta.
6. Lembrete de revisar mês.

Implementação:

1. Criar `lib/whatsapp/notifications.ts`.
2. Reaproveitar `app/api/cron/reminders/route.ts`.
3. Enviar via Twilio REST API.
4. Considerar templates aprovados para mensagens fora da janela de 24h.

Observação:

- Fora da janela de 24h, WhatsApp exige template aprovado.
- Mensagem livre só dentro da janela de 24h após interação iniciada pelo usuário.

### Fase 7 — Botões “Sim” e “Não”

É possível via mensagens interativas do WhatsApp Business, mas:

- sandbox pode ter limitações;
- produção pode exigir formato específico;
- fora da janela de 24h pode depender de template;
- deve ter fallback textual.

Plano:

1. Manter `Responda sim ou não`.
2. Criar envio via Twilio REST.
3. Implementar `sendConfirmationButtons()`.
4. Fallback para texto se Twilio/sandbox não suportar.

### Fase 8 — Nome e foto do número

No Sandbox não dá controle real.

Para personalizar:

1. Criar WhatsApp Sender próprio na Twilio.
2. Conectar/validar WhatsApp Business.
3. Configurar perfil:
   - Display name: `M - Finance Agent`
   - Foto: ícone do app
   - Categoria/descrição/site
4. Passar aprovação Meta/Twilio.

## Ordem recomendada

```txt
1. Commit/push do MVP atual
2. Mais frases + cartões
3. Parcelamento
4. Despesa solta
5. Notificações automáticas
6. Botões interativos
7. Recorrência
8. Sender próprio com nome/foto
```

## Próxima tarefa recomendada

Fases 2, 3, 4, 5, 6 e 7 já estão implementadas (mais frases + parcelamento +
despesa avulsa + recorrência real via `recurrence_rules` + notificações de
vencimento/resumo semanal + botões interativos com fallback textual).
Próximos passos sugeridos:

1. **Testar em produção**: validar lançamentos (cartão, parcelado, despesa
   avulsa, recorrência) e notificações pelo WhatsApp. Conferir envs na Vercel.
2. **Orçamento/alertas de gasto**: precisa de conceito de orçamento no schema.
3. **Fase 8 — Sender próprio**: nome/foto do número + templates aprovados para
   notificações e botões fora da janela de 24h.

Antes de cada fase, rode `git status`, preserve mudanças existentes e mantenha
`npm run lint` + `npm run build` passando.


# Open Finance via Pluggy — Spike

Importa automaticamente as compras dos cartões de crédito usando a [Pluggy](https://pluggy.ai)
como agregador (TPP) autorizado no Open Finance, via SDK oficial **`pluggy-sdk`**. Este spike
usa os **connectors sandbox**, então dá pra testar o fluxo inteiro com bancos fake, sem custo e
sem homologação.

## Por que um agregador

Open Finance no Brasil não é uma API pública: para receber dados de terceiros você precisa ser
instituição autorizada pelo Bacen. A Pluggy já é autorizada e expõe uma API REST simples; nós só
consumimos. O que vem do cartão é a **lista de transações da fatura** (estabelecimento, valor,
data) — não um cronograma de parcelas padronizado.

## Variáveis de ambiente

Adicione ao `.env` (credenciais em Dashboard Pluggy → Aplicações):

```
PLUGGY_CLIENT_ID=...
PLUGGY_CLIENT_SECRET=...
PLUGGY_WEBHOOK_SECRET=uma-string-aleatoria      # protege a URL do webhook
```

O SDK autentica e descobre a base URL sozinho — sandbox e produção usam a mesma API; o que
muda são os connectors. Dependência: `pluggy-sdk` (já instalada).

## Migration

`db/migrations/0005_*.sql` adiciona:

- `credit_card_expenses.source` (`manual` | `openfinance`) e `external_id` (id da transação no
  provedor) + unique `(user_id, external_id)` para dedupe/upsert.
- Tabela `pluggy_items`: mapeia uma conexão (item) e uma conta de cartão Pluggy a um cartão local.

Aplique com `npm run db:migrate`.

## Fluxo

1. **Token** — o frontend chama `POST /api/openfinance/connect-token` → `{ accessToken }`. A rota
   já injeta `clientUserId` (id do usuário interno) e registra o `webhookUrl` no token.
2. **Consentimento** — o widget [Pluggy Connect](https://docs.pluggy.ai/docs/pluggy-connect)
   (`react-pluggy-connect`) abre com esse token; o usuário escolhe o banco e consente. O widget
   retorna um `itemId`.
3. **Registro** — chame a server action `registerPluggyItem(itemId)`: lê as contas de crédito do
   item e cria as linhas em `pluggy_items` (ainda sem cartão).
4. **Vínculo** — `linkPluggyAccountToCard(itemId, accountId, cardId)` liga a conta a um cartão
   local e dispara o primeiro sync.
5. **Sync** — para cada conta mapeada, busca transações (`from` = último sync ou 90 dias),
   faz upsert em `credit_card_expenses` (dedup por `external_id`) e recalcula a fatura do mês
   afetado via `syncInvoiceTotal`. Re-rodar é seguro: atualiza no lugar.
6. **Automático** — a rota `connect-token` registra o webhook no token **apenas quando o host é
   https** (a Pluggy recusa URLs http/localhost com `400 "Webhook url must be a https secured
   url"`). Em produção (https) o sync roda sozinho em `transactions/updated` etc.; em
   localhost use o botão "Sincronizar agora".

## Plano trial só conecta sandbox

Uma aplicação Pluggy **trial/gratuita não cria itens para bancos reais** (Nubank, Mercado Pago,
etc.) — a API responde `TRIAL_CLIENT_ITEM_CREATE_NOT_ALLOWED`. Só funcionam os **conectores de
teste**. Por isso o widget está fixado no conector **"Pluggy Bank"** (id 2) via `connectorIds`
em `components/openfinance/connect-bank.tsx`, que expõe um cartão de crédito fake com transações.

Credenciais de teste do Pluggy Bank: usuário **`user-ok`**, senha **`password-ok`**.

Para conectar bancos reais é preciso um **plano pago/produção** na Pluggy; aí remova o
`connectorIds` para listar os bancos de verdade.

## Decisões do spike (e limites)

- **Só outflows viram compra.** Transações com valor positivo (pagamento de fatura, estorno,
  cashback) são ignoradas — ver `expenseCents()` em `lib/openfinance/sync.ts`.
- **Mês derivado da data da transação**, criando o mês se não existir (`ensureMonthForUser`).
- **Parcelas**: importadas como o lançamento que o banco reporta na fatura; não há explosão de
  "parcela 3/10" futura (o Open Finance não padroniza isso de forma confiável).
- **API key** da Pluggy é cacheada em processo (~2h). Em serverless com múltiplas instâncias cada
  uma renova a sua — ok para o spike.

## Arquivos

| Arquivo | Papel |
|---|---|
| `lib/openfinance/pluggy.ts` | Wrapper do `pluggy-sdk` (connect token, accounts, transactions) |
| `lib/openfinance/sync.ts` | Importa transações → expenses → recalcula faturas |
| `lib/invoice-sync.ts` | `syncInvoiceTotal` compartilhado (manual + Open Finance) |
| `app/actions/openfinance.ts` | `registerPluggyItem`, `linkPluggyAccountToCard` |
| `app/api/openfinance/connect-token/route.ts` | Token do widget |
| `app/api/openfinance/sync/route.ts` | "Sincronizar agora" manual |
| `app/api/openfinance/webhook/route.ts` | Webhook Pluggy → sync automático |
| `lib/openfinance/items.ts` | Lista as conexões do usuário (com nome do cartão) |
| `components/openfinance/connect-bank.tsx` | Widget `react-pluggy-connect` + vínculo conta↔cartão |
| `app/(app)/app/settings/page.tsx` | Seção "Open Finance" (conectar, sincronizar, desvincular) |

## UI (implementada)

Seção **Open Finance (Pluggy)** em `/app/settings`: botão "Conectar banco" (widget
`react-pluggy-connect`, `includeSandbox`), vínculo conta↔cartão logo após conectar, e por
conexão os botões "Sincronizar agora" e "Desvincular". Contas conectadas mas não vinculadas
aparecem como "sem cartão vinculado".

## Falta para virar produção

- Re-vincular contas que ficaram sem cartão a partir da lista (hoje só no fluxo logo após
  conectar).
- Sair do sandbox: cadastro/produção na Pluggy (custo por conexão/mês), revisão LGPD do
  consentimento e retenção dos dados importados.

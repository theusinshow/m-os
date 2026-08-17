# Hermes cria conta no M-Finance (Feature B) — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-17

**Baseline:** M/OS `v0.2.11`; Feature A (M-Finance embutido, `docs/superpowers/specs/2026-08-17-m-finance-embed-design.md`) já implementada. M-Finance em `apps/m-finance`, deploy em produção `https://m-finance-silk.vercel.app`.

**Origem:** implementa a Fase 3 de `docs/SPEC-ACOES-ENTRE-APPS.md` ("M-Finance Action API"), restrita a uma única ação (`create_bill`) em vez do catálogo genérico completo desenhado naquele documento.

## 1. Objetivo

Usuário escreve no Hermes algo como "adiciona conta de luz, R$180, vence dia 10, recorrente"; o M/OS interpreta, mostra um preview, e — só com confirmação explícita — cria a conta de verdade no M-Finance, via uma API nova que o M-Finance expõe. O Hermes nunca segura credencial nem escreve direto no banco do M-Finance.

## 2. Escopo

**Dentro:**
- injeção de contexto (descrição da ação `create_bill`) antes da mensagem do usuário, condicionada a `m-finance` ter `can_write=true` no App Registry;
- parsing do bloco ` ```mos-action ``` ` na resposta do Hermes;
- card de preview/confirmação no chat do Hermes;
- comando Tauri + cliente HTTP que chama a nova Action API do M-Finance;
- credencial via Windows Credential Manager (`keyring`), configurada manualmente numa tela de Settings;
- endpoint novo em `apps/m-finance` (`app/api/mos/actions/route.ts`) reaproveitando a lógica de `create_bill` já existente no executor do bot de WhatsApp.

**Fora:**
- qualquer ação além de `create_bill` (`mark_bill_paid`, `mark_invoice_paid`, etc.);
- catálogo genérico multi-App/multi-ação (só a estrutura mínima que uma segunda ação poderá reaproveitar depois);
- multiusuário no M-Finance;
- provisionamento automático do secret (é manual: Vercel env var + colar no M/OS);
- Undo de uma conta criada — correção é manual dentro do M-Finance, como hoje;
- Feature A (já implementada) e qualquer mudança nela.

## 3. Fluxo

```
1. Usuário digita no Hermes: "adiciona conta de luz, R$ 180, vence dia 10, recorrente"
2. M/OS injeta, antes do texto do usuário, um bloco compacto descrevendo a
   ação create_bill (nome + schema de argumentos) — só quando M-Finance está
   registrado como App com can_write=true.
3. Hermes responde em prosa e, se decidir propor a ação, inclui um bloco
   ```mos-action``` (JSON) na própria resposta de texto.
4. M/OS (Rust) faz o parsing desse bloco na resposta completa (não durante o
   streaming), valida contra o schema declarado do create_bill.
5. Se válido: card de preview no chat (valor, descrição, vencimento,
   recorrência) com Confirmar/Cancelar. Dinheiro = sempre preview, sempre
   confirmação explícita, nunca execução silenciosa.
6. Ao confirmar: M/OS chama a Action API do M-Finance (HTTPS, secret no
   header) e mostra o resultado (sucesso/erro) como uma nova linha no chat —
   sem promessa de Undo.
```

## 4. Peças por lado

### M/OS · Rust (`crates/mos-core`)

- Novo módulo (`mos_actions.rs`, nome provisório): `ActionCatalogEntry { id, app_id, name, description, arg_schema, risk }` e `finance_action_catalog()` retornando só `m_finance.create_bill`. Mesmo espírito de `functions.rs` (`FunctionDefinition`/`function_registry`), mas para ações externas a Apps, não locais — os dois catálogos não se fundem.
- Parser do bloco ` ```mos-action ``` `: recebe o texto completo da resposta do Hermes (não o stream), extrai o primeiro bloco fenced com essa linguagem, `serde_json::from_str`, valida `action` contra o catálogo e os campos de `args` contra o schema declarado (checagem de tipo/presença simples, não um validador de JSON Schema completo).
- Cliente HTTP (`reqwest`) para a Action API: `POST` com header `Authorization: Bearer <secret>`.

### M/OS · Tauri (`apps/desktop/src-tauri`)

- Comando `finance_execute_action(action_id: String, args: serde_json::Value) -> Result<ActionReceipt, String>`.
- Leitura/escrita do secret via `keyring` — mesmo `service` `"m-os"` já usado em `mos-hermes/src/auth.rs`, `account` novo (`"finance-action-secret"`).
- Comandos `finance_set_action_secret(secret: String)` e `finance_action_secret_configured() -> bool`, espelhando `hermes_set_credentials`/`hermes_clear_credentials`.

### M/OS · React (`apps/desktop/src`)

- Injeção do bloco de contexto antes de `bridge.submit`, só quando o `RegisteredApp` `m-finance` tem `can_write` marcado — reaproveita a capacidade que já existe no App Registry (hoje só informativa).
- Parsing do bloco ` ```mos-action ``` ` na mensagem completa do Hermes (mensagem fechada, não durante streaming), renderizando um card de preview em vez do bloco de código cru.
- Card de preview/confirmação (componente novo): campos formatados (valor em R$, data), botões Confirmar/Cancelar. Ao confirmar, chama `finance_execute_action`; o card vira um resultado (sucesso com o que foi criado, ou erro com a mensagem).
- Settings: campo para colar o secret (mesmo padrão visual das credenciais do Hermes), com indicador configurado/não configurado.

### M-Finance (`apps/m-finance`)

- `app/api/mos/actions/route.ts`: `POST`, `runtime = "nodejs"`, autentica via `Authorization: Bearer` comparado a `env.mosActionSecret` (mesmo padrão de `app/api/cron/reminders/route.ts`).
- Body: `{ actionId: "m_finance.create_bill", args: { amountCents, description, dueDay, isRecurring } }`.
- Novo `lib/mos/action-bridge.ts`: função `executeMosAction(actionId, args, userId)` que reaproveita o schema Zod (`billPayloadSchema`) e a lógica de escrita (Drizzle) hoje em `lib/whatsapp/action-executor.ts`, desacoplada da tabela `PendingAction` do WhatsApp (não exige linha pendente prévia). `userId` fixo do único usuário autorizado (`env.authorizedEmail`, mesmo padrão do webhook do WhatsApp) — sem multiusuário.
- Resposta: `{ ok: true, billId, ... }` ou `{ ok: false, error }`.

## 5. Segurança e risco

- `create_bill` é sempre `risk: High` — todo dinheiro é. Preview sempre aparece; confirmação é sempre explícita; sem promessa de Undo.
- O Hermes nunca recebe a credencial — só propõe texto/JSON; quem autentica e executa é sempre o M/OS, pelo mesmo caminho que a UI usaria se o formulário fosse preenchido manualmente.
- Ação rejeitada se o JSON não bater no schema declarado — sem tentativa de correção automática de argumento malformado.
- Auditoria: a mensagem do chat já persiste a proposta (bloco `mos-action`), o card de confirmação e o resultado — isso é o registro do lado M/OS. Do lado M-Finance, a escrita usa o mesmo caminho de dados de qualquer outra criação de conta, sem tabela de auditoria nova.

## 6. QA / gate de conclusão

- `npm run build` (`apps/desktop`) e `cargo build`/`cargo test` (`crates/mos-core`, `apps/desktop/src-tauri`);
- `npm run build` (`apps/m-finance`, local, sem deploy automático);
- teste manual de ponta a ponta: pedir pro Hermes real (conectado) criar uma conta de teste, confirmar o card, checar que ela aparece no M-Finance;
- teste de rejeição: secret errado/ausente → M/OS mostra erro claro, não trava a conversa;
- confirmar que, sem `can_write` no App Registry do M-Finance, o bloco de contexto não é injetado e o Hermes não propõe a ação;
- nenhuma regra de negócio, API, banco ou contrato de domínio pré-existente alterado — só peças novas.

## 7. Fora de escopo / decisões futuras

- Uma segunda ação (ex. `mark_bill_paid`) reabre a questão de generalizar o catálogo — não decidida aqui.
- Rotação de secret, multiusuário no M-Finance e Undo de ações financeiras ficam para quando (e se) houver necessidade real.

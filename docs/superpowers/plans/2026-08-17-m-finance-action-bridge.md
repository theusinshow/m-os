# Hermes cria conta no M-Finance (Feature B) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Usuário pede ao Hermes para lançar uma conta ("adiciona conta de luz, R$180, vence dia 10"); o Hermes propõe a ação, o M/OS mostra um preview e só executa — chamando uma API nova do M-Finance — com confirmação explícita.

**Architecture:** A infraestrutura de propose→preview→confirm→executar **já existe** no M/OS para ações locais (`mos.task.create` etc. — `crates/mos-core/src/action.rs`, `apps/desktop/src-tauri/src/jarvis.rs`, `ActionCard` em `HermesPage.tsx`). Este plano estende esse mecanismo genérico com um novo `ActionKind::MFinanceCreateBill`, cuja execução (em vez de chamar um serviço local) faz uma chamada HTTP autenticada para um endpoint novo no M-Finance. O catálogo de ações já desce automaticamente em todo prompt (`action_contract()`); a novidade é só condicioná-lo à capacidade `can_write` do App M-Finance.

**Tech Stack:** Rust (`crates/mos-core`, `apps/desktop/src-tauri`, Tauri 2), React/TypeScript (`apps/desktop/src`), Next.js + Drizzle + Zod (`apps/m-finance`).

## Global Constraints

- `crates/mos-core` **não pode depender de rede** (regra arquitetural já documentada no topo de `jarvis.rs`: "`mos-core` continua sem SQLite... sem saber que existe rede"). Toda chamada HTTP para o M-Finance vive em `apps/desktop/src-tauri`, nunca em `mos-core`.
- `crates/mos-hermes` é exclusivamente a ponte com o gateway do Hermes — não ganha nenhuma responsabilidade de M-Finance. A credencial e o cliente HTTP do M-Finance são um módulo novo e separado em `apps/desktop/src-tauri`.
- Nomes de ação seguem o padrão já estabelecido em `action.rs` (comentário do próprio arquivo: "o catálogo vai crescer com `m-finance.*`") — hífen, não underscore: `"m-finance.create_bill"`.
- `apps/desktop` **não tem testes de componente/DOM** (`vitest.config.ts` roda só `src/**/*.test.ts`, `environment: "node"`). Nenhuma tarefa deste plano cria `*.test.tsx`. `crates/mos-core` **tem** testes reais (`cargo test`) — use-os onde a tarefa mexe nesse crate.
- `apps/m-finance` não tem framework de teste configurado hoje — nenhuma tarefa deste plano introduz um framework de teste novo lá; verificação é `npm run build` + QA manual.
- Nenhuma regra de negócio, API, banco ou contrato de domínio pré-existente é alterada — só peças novas, e a mudança de assinatura de `action_contract()`/`run_action()` (que passam a receber um parâmetro novo / virar `async`) é mecânica, sem mudar o comportamento das 8 ações locais existentes.
- Design aprovado em `docs/superpowers/specs/2026-08-17-m-finance-action-bridge-design.md`.

---

### Task 1: `mos-core` — `ActionKind`/`ActionArgs` para `m-finance.create_bill`

**Files:**
- Modify: `crates/mos-core/src/action.rs`

**Interfaces:**
- Produz: `ActionKind::MFinanceCreateBill` (`as_str() == "m-finance.create_bill"`), `ActionArgs::MFinanceCreateBill { amount_cents: i64, description: String, due_day: Option<u8>, is_recurring: bool }`, `action_contract(finance_enabled: bool) -> String` (assinatura muda de `action_contract()` — sem argumento — para aceitar o flag).
- Consome: nada novo além do que o arquivo já importa.

- [ ] **Step 1: Escrever o teste de parsing (falha esperada)**

Adicionar ao módulo de testes no final de `crates/mos-core/src/action.rs` (dentro de `mod tests`):

```rust
#[test]
fn parses_m_finance_create_bill() {
    let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":18000,"description":"Conta de luz","dueDay":10,"isRecurring":true}}"#;
    assert_eq!(
        parse_action(raw).unwrap(),
        ActionArgs::MFinanceCreateBill {
            amount_cents: 18000,
            description: "Conta de luz".into(),
            due_day: Some(10),
            is_recurring: true,
        }
    );
}

#[test]
fn refuses_a_bill_with_zero_or_negative_amount() {
    let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":0,"description":"X","isRecurring":false}}"#;
    assert!(parse_action(raw).is_err());
}

#[test]
fn refuses_a_due_day_outside_the_month() {
    let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":100,"description":"X","dueDay":32,"isRecurring":false}}"#;
    assert!(parse_action(raw).is_err());
}

#[test]
fn the_m_finance_preview_shows_currency_and_due_day() {
    let args = ActionArgs::MFinanceCreateBill {
        amount_cents: 18000,
        description: "Conta de luz".into(),
        due_day: Some(10),
        is_recurring: true,
    };
    let preview = preview_of(&args);
    assert_eq!(preview.title, "CRIAR CONTA NO M-FINANCE");
    assert!(preview.lines.iter().any(|l| l.value.contains("180")));
    assert!(preview.lines.iter().any(|l| l.value.contains("10")));
    assert_eq!(preview.risk, FunctionRisk::High);
}

#[test]
fn the_contract_hides_m_finance_when_not_enabled() {
    assert!(!action_contract(false).contains("m-finance.create_bill"));
    assert!(action_contract(true).contains("m-finance.create_bill"));
}
```

- [ ] **Step 2: Rodar os testes e confirmar a falha**

Run: `cd crates/mos-core && cargo test action`
Expected: FAIL — `ActionArgs::MFinanceCreateBill` não existe, `action_contract` não aceita argumento (erros de compilação, não de asserção).

- [ ] **Step 3: Adicionar `MFinanceCreateBill` a `ActionKind`**

Em `crates/mos-core/src/action.rs`, dentro do enum `ActionKind` (linhas ~24-33), adicionar o variante:

```rust
pub enum ActionKind {
    CaptureCreate,
    TaskCreate,
    TaskSetState,
    ProjectCreate,
    ResourceCreate,
    TimeStart,
    TimeStop,
    TimeRecord,
    MFinanceCreateBill,
}
```

Em `as_str()` (dentro do `impl ActionKind`):

```rust
Self::TimeRecord => "mos.time.record",
Self::MFinanceCreateBill => "m-finance.create_bill",
```

Em `parse()`:

```rust
"mos.time.record" => Some(Self::TimeRecord),
"m-finance.create_bill" => Some(Self::MFinanceCreateBill),
```

Em `function_id()`:

```rust
Self::TimeRecord => "time.record",
Self::MFinanceCreateBill => "m-finance.create_bill",
```

Em `all()`, atualizar o tamanho do array e adicionar o item:

```rust
pub fn all() -> [ActionKind; 9] {
    [
        Self::CaptureCreate,
        Self::TaskCreate,
        Self::TaskSetState,
        Self::ProjectCreate,
        Self::ResourceCreate,
        Self::TimeStart,
        Self::TimeStop,
        Self::TimeRecord,
        Self::MFinanceCreateBill,
    ]
}
```

Em `signature()`:

```rust
Self::TimeRecord => "{ project, minutes, day?: AAAA-MM-DD, activity?, description? }",
Self::MFinanceCreateBill => "{ amountCents, description, dueDay?: 1-31, isRecurring }",
```

- [ ] **Step 4: Adicionar o variante a `ActionArgs` e ao `parse_action`**

Em `ActionArgs` (linhas ~125-166), adicionar:

```rust
MFinanceCreateBill {
    /// Centavos. Sempre positivo — zero ou negativo nao e uma conta.
    amount_cents: i64,
    description: String,
    /// Dia do mes, 1-31. Ausente quando a conta nao tem vencimento fixo.
    due_day: Option<u8>,
    is_recurring: bool,
},
```

No `impl ActionArgs { pub fn kind(&self) ... }`, adicionar:

```rust
Self::MFinanceCreateBill { .. } => ActionKind::MFinanceCreateBill,
```

Em `parse_action`, dentro do `match kind { ... }`, adicionar o braço (depois de `ActionKind::TimeRecord => ...`):

```rust
ActionKind::MFinanceCreateBill => {
    let amount_cents = args
        .get("amountCents")
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| {
            CoreError::new(
                ErrorCode::InvalidInput,
                "A proposta de `m-finance.create_bill` veio sem `amountCents` valido.".to_owned(),
                false,
            )
        })?;
    let due_day = args
        .get("dueDay")
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as u8);
    if let Some(day) = due_day {
        if !(1..=31).contains(&day) {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                format!("`{day}` nao e um dia valido de vencimento."),
                false,
            ));
        }
    }
    ActionArgs::MFinanceCreateBill {
        amount_cents,
        description: required(&args, "description", kind)?,
        due_day,
        is_recurring: args
            .get("isRecurring")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    }
}
```

- [ ] **Step 5: Adicionar o preview em `preview_of`**

Antes do fechamento do `match args { ... }` em `preview_of` (depois do braço `ActionArgs::TimeRecord`), adicionar:

```rust
ActionArgs::MFinanceCreateBill {
    amount_cents,
    description,
    due_day,
    is_recurring,
} => {
    let mut lines = vec![
        line("Valor", &format_cents(*amount_cents)),
        line("Descrição", description),
    ];
    if let Some(day) = due_day {
        lines.push(line("Vencimento", &format!("dia {day}")));
    }
    lines.push(line("Recorrente", if *is_recurring { "sim" } else { "não" }));
    ("CRIAR CONTA NO M-FINANCE", lines)
}
```

E adicionar a função auxiliar `format_cents` perto de `line()` (antes de `preview_of`):

```rust
/// R$ a partir de centavos. Nao e formatacao de moeda completa (sem milhar) —
/// e so o preview; o M-Finance formata de verdade na tela dele.
fn format_cents(cents: i64) -> String {
    format!("R$ {:.2}", cents as f64 / 100.0)
}
```

- [ ] **Step 6: Mudar a assinatura de `action_contract` para aceitar o flag**

Trocar:

```rust
pub fn action_contract() -> String {
    let catalog = ActionKind::all()
        .iter()
        .map(|kind| format!("- {} {}", kind.as_str(), kind.signature()))
        .collect::<Vec<_>>()
        .join("\n");
```

por:

```rust
/// `finance_enabled` decide se `m-finance.create_bill` desce no catalogo.
/// Sem a capacidade `can_write` no App M-Finance, o Hermes nunca aprende que
/// a acao existe — a mesma logica que impede a UI de oferecer uma acao que o
/// usuario nao habilitou.
pub fn action_contract(finance_enabled: bool) -> String {
    let catalog = ActionKind::all()
        .iter()
        .filter(|kind| finance_enabled || **kind != ActionKind::MFinanceCreateBill)
        .map(|kind| format!("- {} {}", kind.as_str(), kind.signature()))
        .collect::<Vec<_>>()
        .join("\n");
```

O resto do corpo da função (o `format!(...)` que monta o bloco final) não muda.

- [ ] **Step 7: Rodar os testes e confirmar que passam**

Run: `cd crates/mos-core && cargo test action`
Expected: PASS — todos os testes de `action.rs`, incluindo os quatro novos.

- [ ] **Step 8: Rodar a suíte inteira do crate**

Run: `cd crates/mos-core && cargo test`
Expected: PASS — nenhum teste existente quebrou (em especial `the_preview_reads_risk_from_the_function_registry`, que itera `ActionKind::all()` e vai precisar achar `"m-finance.create_bill"` em `function_registry()` — ver Task 2, que precisa ser feita **antes** deste `cargo test` passar limpo).

- [ ] **Step 9: Commit**

```bash
git add crates/mos-core/src/action.rs
git commit -m "feat(mos-core): adiciona ActionKind m-finance.create_bill"
```

---

### Task 2: `mos-core` — registrar `m-finance.create_bill` em `functions.rs`

**Files:**
- Modify: `crates/mos-core/src/functions.rs`

**Interfaces:**
- Consome: nenhum símbolo novo.
- Produz: uma entrada em `function_registry()` com `id: "m-finance.create_bill"`, `risk: High`, `confirmation: Explicit` — consumida por `ActionKind::function_id()` (Task 1) via `preview_of`.

- [ ] **Step 1: Escrever o teste (falha esperada)**

Em `crates/mos-core/src/functions.rs`, dentro de `mod tests`, adicionar:

```rust
#[test]
fn m_finance_create_bill_is_registered_as_high_risk() {
    let entry = function_registry()
        .into_iter()
        .find(|item| item.id == "m-finance.create_bill")
        .expect("m-finance.create_bill deveria estar registrada");
    assert_eq!(entry.risk, FunctionRisk::High);
    assert_eq!(entry.confirmation, FunctionConfirmation::Explicit);
}
```

- [ ] **Step 2: Rodar e confirmar a falha**

Run: `cd crates/mos-core && cargo test m_finance_create_bill_is_registered`
Expected: FAIL — `expect` dispara, a entrada não existe.

- [ ] **Step 3: Adicionar a entrada ao registro**

Em `crates/mos-core/src/functions.rs`, dentro do `vec![...]` de `function_registry()`, depois da entrada `"app.open"` (linhas ~201-208):

```rust
function(
    "m-finance.create_bill",
    "Criar conta no M-Finance",
    "Propõe uma conta (valor, descrição, vencimento) para o M-Finance, App externo. Dinheiro é sempre risco alto.",
    FunctionCategory::App,
    FunctionRisk::High,
    FunctionConfirmation::Explicit,
),
```

- [ ] **Step 4: Rodar e confirmar que passa**

Run: `cd crates/mos-core && cargo test`
Expected: PASS — o teste novo e todos os testes existentes de `functions.rs` e `action.rs` (inclusive `the_preview_reads_risk_from_the_function_registry` da Task 1).

- [ ] **Step 5: Commit**

```bash
git add crates/mos-core/src/functions.rs
git commit -m "feat(mos-core): registra m-finance.create_bill como acao de risco alto"
```

---

### Task 3: dependências novas em `apps/desktop/src-tauri`

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`

**Interfaces:**
- Produz: os crates `reqwest` e `keyring` disponíveis para a Task 4.

- [ ] **Step 1: Adicionar `reqwest` e `keyring` às dependências**

Em `apps/desktop/src-tauri/Cargo.toml`, na seção `[dependencies]`, adicionar (mesmas versões já usadas em `crates/mos-hermes/Cargo.toml`, sem o feature `cookies` que o M-Finance não precisa):

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
keyring = { version = "3", features = ["windows-native"] }
```

- [ ] **Step 2: Confirmar que o workspace resolve as dependências**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: compila (pode demorar na primeira vez por baixar os crates); sem erros. Avisos de "unused" são esperados até a Task 4 usar os crates — não são falha.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.lock
git commit -m "chore(desktop): adiciona reqwest e keyring para a Action API do M-Finance"
```

---

### Task 4: módulo `finance.rs` — credencial e cliente HTTP

**Files:**
- Create: `apps/desktop/src-tauri/src/finance.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs` (declarar o módulo)

**Interfaces:**
- Produz: comandos Tauri `finance_set_action_secret(secret: String) -> Result<(), String>`, `finance_clear_action_secret() -> Result<(), String>`, `finance_action_secret_configured() -> bool`; função `pub async fn execute_create_bill(amount_cents: i64, description: &str, due_day: Option<u8>, is_recurring: bool) -> Result<String, String>` (usada pela Task 5).
- Consome: `keyring::Entry` (Task 3), `reqwest::Client` (Task 3).

- [ ] **Step 1: Criar `finance.rs`**

```rust
//! Credencial e cliente HTTP para a Action API do M-Finance.
//!
//! Mesmo padrao de `mos-hermes/src/auth.rs`: o segredo vive so no Windows
//! Credential Manager, nunca na memoria do renderer nem em disco em texto
//! claro. Diferente do Hermes, aqui nao ha sessao — cada chamada manda o
//! segredo no header `Authorization`, como o proprio M-Finance ja faz para o
//! cron do Vercel (`app/api/cron/reminders`).

use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "m-os";
const ACCOUNT: &str = "finance-action-secret";
const ACTION_API_URL: &str = "https://m-finance-silk.vercel.app/api/mos/actions";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT)
        .map_err(|error| format!("Credential Manager indisponivel: {error}"))
}

#[tauri::command]
pub fn finance_set_action_secret(secret: String) -> Result<(), String> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err("O secret nao pode ficar vazio.".into());
    }
    entry()?
        .set_password(trimmed)
        .map_err(|error| format!("Nao foi possivel guardar: {error}"))
}

#[tauri::command]
pub fn finance_clear_action_secret() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Nao foi possivel remover: {error}")),
    }
}

#[tauri::command]
pub fn finance_action_secret_configured() -> bool {
    entry().and_then(|e| e.get_password().map_err(|error| error.to_string())).is_ok()
}

#[derive(Serialize)]
struct ActionRequest {
    #[serde(rename = "actionId")]
    action_id: &'static str,
    args: serde_json::Value,
}

#[derive(Deserialize)]
struct ActionResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default, rename = "billId")]
    bill_id: Option<String>,
}

/// Chama a Action API do M-Finance. Erros de rede, autenticacao e recusa de
/// negocio viram a MESMA `Result<_, String>` — quem chama (jarvis::run_action)
/// converte para `CoreError` e o texto vai direto para o recibo da conversa.
pub async fn execute_create_bill(
    amount_cents: i64,
    description: &str,
    due_day: Option<u8>,
    is_recurring: bool,
) -> Result<String, String> {
    let secret = entry()?.get_password().map_err(|_| {
        "Secret do M-Finance nao configurado. Cole-o em Settings antes de confirmar.".to_owned()
    })?;

    let args = serde_json::json!({
        "amountCents": amount_cents,
        "description": description,
        "dueDay": due_day,
        "isRecurring": is_recurring,
    });

    let response = reqwest::Client::new()
        .post(ACTION_API_URL)
        .bearer_auth(secret)
        .json(&ActionRequest {
            action_id: "m-finance.create_bill",
            args,
        })
        .send()
        .await
        .map_err(|error| format!("Nao foi possivel falar com o M-Finance: {error}"))?;

    let body: ActionResponse = response
        .json()
        .await
        .map_err(|error| format!("Resposta inesperada do M-Finance: {error}"))?;

    if body.ok {
        Ok(body
            .bill_id
            .map(|id| format!("Conta criada no M-Finance (id {id})."))
            .unwrap_or_else(|| "Conta criada no M-Finance.".to_owned()))
    } else {
        Err(body
            .error
            .unwrap_or_else(|| "O M-Finance recusou a acao.".to_owned()))
    }
}
```

- [ ] **Step 2: Declarar o módulo em `lib.rs`**

Em `apps/desktop/src-tauri/src/lib.rs`, junto às demais declarações de módulo (linhas ~25-30):

```rust
mod calendar;
mod finance;
mod hermes;
mod jarvis;
mod monitor;
mod pdf;
mod tracking;
```

- [ ] **Step 3: Registrar os três comandos no `invoke_handler`**

Em `apps/desktop/src-tauri/src/lib.rs`, dentro de `tauri::generate_handler![...]`, adicionar (perto dos comandos de `hermes::`, por proximidade de assunto):

```rust
finance::finance_set_action_secret,
finance::finance_clear_action_secret,
finance::finance_action_secret_configured,
```

- [ ] **Step 4: Compilar**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: compila sem erros. `execute_create_bill` ainda não é chamada em lugar nenhum — isso é esperado (fica sem uso até a Task 5) e não gera erro de compilação (só de aviso `dead_code` no máximo, aceitável nesta etapa intermediária).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/finance.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): adiciona modulo finance com credencial e cliente da Action API"
```

---

### Task 5: `jarvis.rs` — executar `MFinanceCreateBill`

**Files:**
- Modify: `apps/desktop/src-tauri/src/jarvis.rs`

**Interfaces:**
- Consome: `mos_core::ActionArgs::MFinanceCreateBill` (Task 1), `crate::finance::execute_create_bill` (Task 4).
- Produz: `run_action` passa a ser `async fn`; `action_resolve` (já `async`) passa a dar `.await` na chamada.

- [ ] **Step 1: Tornar `run_action` assíncrona**

Em `apps/desktop/src-tauri/src/jarvis.rs`, trocar a assinatura (linha ~254):

```rust
fn run_action<R: Runtime>(
    app: &AppHandle<R>,
    args: &mos_core::ActionArgs,
) -> Result<mos_core::ActionEffect, CoreError> {
```

por:

```rust
async fn run_action<R: Runtime>(
    app: &AppHandle<R>,
    args: &mos_core::ActionArgs,
) -> Result<mos_core::ActionEffect, CoreError> {
```

Nenhum dos braços existentes do `match args { ... }` precisa de `.await` — eles continuam síncronos e continuam compilando dentro de uma função `async` sem alteração.

- [ ] **Step 2: Adicionar o braço `MFinanceCreateBill`**

Dentro do mesmo `match args { ... }`, depois do braço `ActionArgs::TimeRecord { .. } => { ... }` (antes do fechamento `}` do match, linha ~428), adicionar:

```rust
mos_core::ActionArgs::MFinanceCreateBill {
    amount_cents,
    description,
    due_day,
    is_recurring,
} => {
    let message = crate::finance::execute_create_bill(
        *amount_cents,
        description,
        *due_day,
        *is_recurring,
    )
    .await
    .map_err(|error| CoreError::new(mos_core::ErrorCode::Io, error, true))?;
    Ok(mos_core::ActionEffect {
        message,
        // Sem desfazer: o M/OS nao tem um comando de "apagar conta" no
        // M-Finance, e inventar um so para o Undo seria dar ao Hermes um
        // poder que a Action API (Fase 3 da spec) nao expoe. Corrigir uma
        // conta criada por engano e manual, dentro do proprio M-Finance —
        // igual e como as outras contas de la sempre foram corrigidas.
        undo: None,
    })
}
```

- [ ] **Step 3: Atualizar o ponto de chamada em `action_resolve`**

Em `action_resolve` (linha ~620), trocar:

```rust
match mos_core::parse_action(&raw).and_then(|args| run_action(&app, &args)) {
```

por:

```rust
let resolved = match mos_core::parse_action(&raw) {
    Ok(args) => run_action(&app, &args).await,
    Err(error) => Err(error),
};
match resolved {
```

(A chamada anterior encadeava `parse_action` e `run_action` num `and_then` síncrono; como `run_action` agora é `async`, o `and_then` não serve mais — o `match` explícito faz a mesma coisa com `.await`.)

- [ ] **Step 4: Compilar e rodar os testes do crate `apps/desktop/src-tauri`**

Run: `cd apps/desktop/src-tauri && cargo check && cargo test`
Expected: compila; os testes existentes de `jarvis.rs` (`split_proposal`, `proposal_part`, `TurnRecorder`, `project_history`, etc.) continuam passando — nenhum deles exercita `run_action`/`action_resolve` diretamente (são testes de unidade nas funções puras do arquivo), então não precisam de ajuste.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/jarvis.rs
git commit -m "feat(desktop): jarvis executa m-finance.create_bill via Action API"
```

---

### Task 6: gate de `can_write` e catálogo no prompt do Hermes

**Files:**
- Modify: `apps/desktop/src-tauri/src/hermes.rs`

**Interfaces:**
- Consome: `state.apps.app("m-finance")` (já existe em `mos-core::service::AppService`), `mos_core::action_contract(bool)` (Task 1).

- [ ] **Step 1: Calcular o flag antes de montar o prompt**

Em `apps/desktop/src-tauri/src/hermes.rs`, dentro de `hermes_send` (linha ~487, logo antes da linha que monta `prompt`), adicionar:

```rust
// So desce no catalogo quando o App M-Finance tem can_write marcado no
// Registry — a mesma capacidade que ja existe, so passando a ter efeito
// real pela primeira vez (SPEC-ACOES-ENTRE-APPS.md).
let finance_enabled = app
    .state::<AppState>()
    .apps
    .app("m-finance")
    .map(|entry| entry.can_write)
    .unwrap_or(false);
```

(`AppState` já está importado neste arquivo — é o mesmo `app.state::<AppState>()` usado duas linhas acima, no início da função, para pegar `conversations`.)

- [ ] **Step 2: Passar o flag para `action_contract`**

Trocar:

```rust
let prompt = format!("{}{}{}", mos_core::action_contract(), assembled.block, text);
```

por:

```rust
let prompt = format!(
    "{}{}{}",
    mos_core::action_contract(finance_enabled),
    assembled.block,
    text
);
```

- [ ] **Step 3: Compilar**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: compila sem erros.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/hermes.rs
git commit -m "feat(desktop): condiciona m-finance.create_bill a can_write do App"
```

---

### Task 7: React — secret de Settings

**Files:**
- Create: `apps/desktop/src/finance.ts`
- Modify: `apps/desktop/src/App.tsx` (novo componente `FinanceActionSettings`, montado dentro de `SettingsPage`)

**Interfaces:**
- Produz: `finance.setActionSecret(secret: string)`, `finance.clearActionSecret()`, `finance.actionSecretConfigured(): Promise<boolean>`.
- Consome: comandos Tauri da Task 4; `StateMessage`, `Panel`, `Button` (já importados em `App.tsx`).

- [ ] **Step 1: Criar o wrapper `finance.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";

/**
 * Fronteira do renderer com o modulo `finance` do lado Rust.
 *
 * Mesmo padrao de `hermes.ts`: nenhuma chamada de rede em componente React, e
 * o secret nunca atravessa de volta para ca depois de guardado — o renderer
 * so aprende que existe (booleano), nunca qual e.
 */
export const finance = {
  setActionSecret(secret: string) {
    return invoke<void>("finance_set_action_secret", { secret });
  },
  clearActionSecret() {
    return invoke<void>("finance_clear_action_secret");
  },
  actionSecretConfigured() {
    return invoke<boolean>("finance_action_secret_configured");
  },
};
```

- [ ] **Step 2: Importar em `App.tsx`**

Em `apps/desktop/src/App.tsx`, junto às demais importações de módulo local (perto de `import { hermes } from "./hermes";`, se existir uma linha assim — senão, junto de `import { FinancePage } from "./FinancePage";`):

```tsx
import { finance } from "./finance";
```

- [ ] **Step 3: Criar o componente `FinanceActionSettings`**

Em `apps/desktop/src/App.tsx`, logo depois da função `HermesSettings` (linha ~2111, mesmo arquivo, mesmo padrão):

```tsx
function FinanceActionSettings() {
  const [configured, setConfigured] = useState(false);
  const [secret, setSecret] = useState("");
  const [message, setMessage] = useState("");
  const [messageState, setMessageState] = useState<"saving" | "saved" | "error">("saved");

  useEffect(() => {
    void finance.actionSecretConfigured().then(setConfigured).catch(() => undefined);
  }, []);

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!secret.trim()) return;
    setMessageState("saving");
    setMessage("Salvando secret...");
    try {
      await finance.setActionSecret(secret);
      setSecret("");
      setConfigured(true);
      setMessage("Secret guardado no Windows Credential Manager.");
      setMessageState("saved");
    } catch (error) {
      setMessageState("error");
      setMessage(String(error));
    }
  }

  async function clear() {
    await finance.clearActionSecret().catch(() => undefined);
    setConfigured(false);
  }

  return (
    <Panel label="AÇÕES DO HERMES NO M-FINANCE">
      <p className="support-copy">
        O Hermes pode propor criar contas no M-Finance quando você pedir — nunca sem confirmação
        explícita. Isto guarda o secret que autoriza o M/OS a chamar a Action API do M-Finance
        (mesmo secret configurado como variável de ambiente lá, do lado do M-Finance).
      </p>
      <form className="stack-form" onSubmit={save}>
        <label><span>SECRET</span><input type="password" value={secret} onChange={(event) => setSecret(event.currentTarget.value)} autoComplete="off" /></label>
        <div className="form-actions">
          <Button variant="ghost" onClick={() => void clear()}>Remover secret</Button>
          <Button variant="primary" type="submit">Salvar</Button>
        </div>
      </form>
      <dl className="fact-grid">
        <div><dt>SECRET</dt><dd>{configured ? "Configurado" : <span className="fact-empty">Não configurado</span>}</dd></div>
      </dl>
      {message ? <StateMessage state={messageState} label={message} /> : null}
    </Panel>
  );
}
```

- [ ] **Step 4: Montar dentro de `SettingsPage`**

Em `apps/desktop/src/App.tsx`, dentro de `SettingsPage`, na seção `settings-connection` (mesma seção onde `<HermesSettings />` já é renderizado — ver o JSX denso perto de `<section className="settings-section" aria-labelledby="settings-connection">`), adicionar `<FinanceActionSettings />` logo depois de `<HermesSettings />`.

- [ ] **Step 5: Checar tipos e build**

Run: `cd apps/desktop && npm run build`
Expected: build conclui sem erros (tsc + vite build).

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/finance.ts apps/desktop/src/App.tsx
git commit -m "feat(desktop): Settings ganha secret da Action API do M-Finance"
```

---

### Task 8: M-Finance — Action API

**Files:**
- Modify: `apps/m-finance/lib/env.ts`
- Create: `apps/m-finance/lib/mos/action-bridge.ts`
- Create: `apps/m-finance/app/api/mos/actions/route.ts`

**Interfaces:**
- Produz: `POST /api/mos/actions` — body `{ actionId: "m-finance.create_bill", args: {...} }`, resposta `{ ok: true, billId } | { ok: false, error }`.
- Consome: `env.mosActionSecret`, `env.authorizedEmail` (`lib/env.ts`), `db`, `bills`, `recurrenceRules`, `composeMonthDate`, `getCurrentMonthForUser`, `ensureConsecutiveMonthsForUser`, `formatCurrency` (já existentes em `lib/whatsapp/action-executor.ts`, reaproveitados sem modificar esse arquivo).

- [ ] **Step 1: Adicionar `mosActionSecret` ao `env`**

Em `apps/m-finance/lib/env.ts`, no objeto `env`, logo depois de `cronSecret`:

```ts
  cronSecret: process.env.CRON_SECRET ?? "",
  // Secret que autoriza o M/OS a chamar a Action API (Hermes propondo acoes).
  mosActionSecret: process.env.MOS_ACTION_SECRET ?? "",
```

- [ ] **Step 2: Criar `lib/mos/action-bridge.ts`**

Reaproveita o schema Zod e a lógica de escrita já existentes em `action-executor.ts` (`billPayloadSchema`, a leitura do mês atual, a criação de `recurrenceRules`/`bills`), mas **desacoplado** da tabela `whatsappPendingActions` — sem `PendingAction`, sem `whatsappPendingActionId`, sem `updateWhatsappPendingActionStatus`:

```ts
import { z } from "zod";
import { db } from "@/db/client";
import { bills, recurrenceRules } from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { ensureConsecutiveMonthsForUser, getCurrentMonthForUser } from "@/lib/months";

const RECURRING_PREGENERATE_MONTHS = 12;

const billPayloadSchema = z.object({
  amountCents: z.number().int().positive(),
  description: z.string().trim().min(1),
  dueDay: z.number().int().min(1).max(31).nullable(),
  isRecurring: z.boolean(),
});

export type MosActionResult =
  | { ok: true; billId: string }
  | { ok: false; error: string };

/**
 * Cria uma conta a partir de uma acao proposta pelo Hermes e confirmada no
 * M/OS. Espelha `executeCreateBill` de `lib/whatsapp/action-executor.ts`,
 * sem o acoplamento com `whatsappPendingActions` — esta acao nao nasceu de
 * uma mensagem de WhatsApp, e forcar uma linha pendente so para satisfazer a
 * foreign key seria inventar um registro que nao existe.
 */
export async function createBillFromMosAction(
  userId: string,
  rawArgs: unknown,
): Promise<MosActionResult> {
  if (!db) {
    return { ok: false, error: "Banco de dados indisponível no momento." };
  }

  const parsed = billPayloadSchema.safeParse(rawArgs);
  if (!parsed.success) {
    return { ok: false, error: "Os argumentos da ação não batem com o esperado." };
  }

  const payload = parsed.data;
  const month = await getCurrentMonthForUser(userId);
  if (!month) {
    return { ok: false, error: "Crie o mês atual no app antes de lançar despesas por aqui." };
  }

  if (payload.isRecurring && payload.dueDay) {
    const [rule] = await db
      .insert(recurrenceRules)
      .values({
        userId,
        name: payload.description,
        defaultAmountCents: payload.amountCents,
        dueDay: payload.dueDay,
        isVariableAmount: false,
        isActive: true,
      })
      .returning();

    if (!rule) {
      return { ok: false, error: "Não consegui criar a regra de recorrência agora." };
    }

    const targetMonths = await ensureConsecutiveMonthsForUser(
      userId,
      month.month,
      month.year,
      RECURRING_PREGENERATE_MONTHS,
    );
    const recurringDueDay = payload.dueDay;

    const created = await db
      .insert(bills)
      .values(
        targetMonths.map((targetMonth) => ({
          userId,
          monthId: targetMonth.id,
          recurrenceRuleId: rule.id,
          name: payload.description,
          amountCents: payload.amountCents,
          dueDate: composeMonthDate(targetMonth.year, targetMonth.month, recurringDueDay),
          isRecurring: true,
          status: "pending" as const,
        })),
      )
      .returning({ id: bills.id });

    return { ok: true, billId: created[0]?.id ?? rule.id };
  }

  const dueDay = payload.dueDay ?? 31;
  const dueDate = composeMonthDate(month.year, month.month, dueDay);

  const [created] = await db
    .insert(bills)
    .values({
      userId,
      monthId: month.id,
      name: payload.description,
      amountCents: payload.amountCents,
      dueDate,
      isRecurring: payload.isRecurring,
      status: "pending",
    })
    .returning({ id: bills.id });

  if (!created) {
    return { ok: false, error: "Não consegui gravar a conta agora." };
  }

  return { ok: true, billId: created.id };
}
```

- [ ] **Step 3: Criar a rota `app/api/mos/actions/route.ts`**

Segue exatamente o padrão de autenticação de `app/api/cron/reminders/route.ts` (`Authorization: Bearer`), mas exigindo o header — nada de `?secret=` na query aqui, já que quem chama é o M/OS, não um agendador externo:

```ts
import { env } from "@/lib/env";
import { getWhatsappOwnerUser } from "@/lib/whatsapp/auth";
import { createBillFromMosAction } from "@/lib/mos/action-bridge";

// Node.js runtime: Drizzle/pg precisam dele, nao do edge runtime.
export const runtime = "nodejs";

const KNOWN_ACTIONS = new Set(["m-finance.create_bill"]);

/**
 * Executa UMA acao ja proposta pelo Hermes e confirmada no M/OS.
 *
 * O modelo nunca chega aqui direto — quem chama e sempre o M/OS, depois que o
 * usuario confirmou o preview. Autenticacao por secret compartilhado, mesmo
 * padrao do cron do Vercel (`app/api/cron/reminders`).
 */
export async function POST(request: Request) {
  const auth = request.headers.get("authorization");
  const authorized = Boolean(env.mosActionSecret) && auth === `Bearer ${env.mosActionSecret}`;

  if (!authorized) {
    return Response.json({ ok: false, error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => null);
  const actionId = typeof body?.actionId === "string" ? body.actionId : "";

  if (!KNOWN_ACTIONS.has(actionId)) {
    return Response.json({ ok: false, error: `Ação desconhecida: ${actionId}` }, { status: 400 });
  }

  const owner = await getWhatsappOwnerUser();
  if (!owner) {
    return Response.json({ ok: false, error: "Usuário autorizado não configurado." }, { status: 500 });
  }

  if (actionId === "m-finance.create_bill") {
    const result = await createBillFromMosAction(owner.id, body?.args);
    return Response.json(result, { status: result.ok ? 200 : 422 });
  }

  return Response.json({ ok: false, error: "Ação sem execução implementada." }, { status: 400 });
}
```

Nota: `getWhatsappOwnerUser` (de `lib/whatsapp/auth.ts`) já resolve o único usuário autorizado via `env.authorizedEmail` — reaproveitado tal como está, sem modificação (é o mesmo padrão de usuário único já usado pelo webhook do WhatsApp).

- [ ] **Step 4: Build**

Run: `cd apps/m-finance && npm run build`
Expected: build conclui sem erros de tipo.

- [ ] **Step 5: Commit**

```bash
git add apps/m-finance/lib/env.ts apps/m-finance/lib/mos/action-bridge.ts apps/m-finance/app/api/mos/actions/route.ts
git commit -m "feat(m-finance): adiciona Action API para o M/OS criar contas"
```

---

### Task 9: provisionamento manual + QA de ponta a ponta

**Files:** nenhum (configuração manual + verificação; sem alteração de código).

**Interfaces:** consome tudo das Tasks 1–8 completas.

- [ ] **Step 1: Gerar o secret**

Gerar um valor aleatório (ex.: `node -e "console.log(require('crypto').randomUUID())"`) — este é o secret compartilhado.

- [ ] **Step 2: Configurar no Vercel**

No painel do projeto M-Finance na Vercel, adicionar a variável de ambiente `MOS_ACTION_SECRET` com o valor gerado, para produção. Fazer redeploy (ou aguardar o próximo deploy) para a env var entrar em vigor.

- [ ] **Step 3: Configurar no M/OS**

Rodar `npm run tauri dev` (`apps/desktop`), abrir Settings, colar o mesmo secret no campo novo "AÇÕES DO HERMES NO M-FINANCE" e salvar. Confirmar que o indicador muda para "Configurado".

- [ ] **Step 4: Habilitar `can_write` no App M-Finance**

Na página Apps do M/OS, abrir o registro de "M Finance" e marcar a capacidade `WRITE` (checkbox já existente no formulário de App, `RegisteredAppForm`).

- [ ] **Step 5: Testar a proposta e confirmação**

No Hermes, pedir algo como "adiciona conta de teste, R$ 1, vence dia 15, não recorrente". Confirmar que:
- a resposta do Hermes mostra um card de preview (não o JSON cru);
- o card mostra valor, descrição e vencimento formatados;
- clicar `CONFIRMAR` executa e mostra o recibo ("Conta criada no M-Finance...");
- a conta aparece de fato no M-Finance (abrir a página Finance embutida — Feature A — ou o M-Finance direto).

- [ ] **Step 6: Testar cancelamento**

Pedir outra conta de teste e clicar `CANCELAR` no card. Expected: nada é criado no M-Finance, o card mostra "Cancelado por você.".

- [ ] **Step 7: Testar rejeição por secret ausente/errado**

Remover o secret em Settings (M/OS), pedir uma nova conta e confirmar. Expected: o recibo mostra um erro claro ("Secret do M-Finance nao configurado..."), a conversa não trava, nenhuma exceção não tratada aparece no console.

- [ ] **Step 8: Testar o gate de `can_write`**

Desmarcar `WRITE` no App M-Finance. Pedir ao Hermes para criar uma conta. Expected: o Hermes não propõe a ação (porque `m-finance.create_bill` não está mais no catálogo que desce no prompt) — a conversa segue só em texto, sem card.

- [ ] **Step 9: Rodar as suítes automatizadas uma última vez**

Run:
```bash
cd crates/mos-core && cargo test
cd ../../apps/desktop/src-tauri && cargo test
cd ../.. && npm run build && npm test -- --run
cd ../m-finance && npm run build
```
Expected: tudo passa/compila limpo.

- [ ] **Step 10: Registrar a evidência**

Sem documento de trilha dedicado para esta feature (não faz parte de `UI-UX-REFINEMENT.md`) — a evidência dos passos 5–9 fica relatada diretamente ao usuário ao final da execução do plano.

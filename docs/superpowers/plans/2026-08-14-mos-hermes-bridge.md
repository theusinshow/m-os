# Ponte com o Hermes — plano de implementação (Spec B)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tornar o M/OS mais uma superfície do Hermes que já roda na VPS, com sessão própria e modo Hermes dentro do Command.

**Architecture:** Um crate novo `mos-hermes` fala JSON-RPC 2.0 sobre WebSocket com o gateway em `127.0.0.1:9119` (túnel SSH iniciado externamente). O crate não depende de `mos-storage-sqlite`, então "Hermes nunca escreve no SQLite" é impossibilidade de compilação. O renderer nunca vê credencial: fala com a ponte por comandos Tauri e recebe streaming por eventos.

**Tech Stack:** Rust (tokio, tokio-tungstenite, reqwest, keyring, serde_json), React 19, TypeScript.

**Spec:** `docs/superpowers/specs/2026-08-13-mos-hermes-bridge-design.md`
**Contrato:** `docs/HERMES-GATEWAY-CONTRACT.md` — verificado ao vivo, é a fonte de verdade do protocolo.

## Global Constraints

- **O contrato manda.** Nenhum endpoint, método ou campo pode ser inventado. Se algo não estiver em `HERMES-GATEWAY-CONTRACT.md`, verificar contra o código do Hermes em `%LOCALAPPDATA%\hermes\hermes-agent` antes de escrever.
- **Credencial nunca no renderer.** Nem senha, nem cookie, nem ticket. Windows Credential Manager, lido só no Rust (`ARCHITECTURE.md:556`).
- **`mos-hermes` não declara `mos-storage-sqlite`.** Se alguém precisar dessa dependência, a arquitetura está errada, não o Cargo.toml.
- **Nada de escrita no M/OS.** Sem `mos_create_task`, `mos_create_capture`, automações ou contexto automático de tela. Leitura antes de escrita, sempre.
- **Ticket é efêmero:** 30s, uso único, cunhado imediatamente antes de abrir o socket. Guardar ticket é bug.
- **Sem bolha de chat, sidebar de suporte, orb, gradiente "AI" ou sparkle.** Hermes é capacidade nativa, e a autoria do sistema é marcada pela barra de 2px em sódio que o design já define.
- **Zero literal de cor** fora de `mos-tokens.css`, como na Spec A.
- **CI é o bar:** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `npm run build`.

## Dependências novas (declaradas, não silenciosas)

A Spec A proibiu dependência nova, mas aquela regra enumera UI, estilo, animação e ícone — é sobre front-end. Uma ponte WebSocket não existe sem biblioteca.

| Crate | Por quê | Alternativa recusada |
|---|---|---|
| `tokio` | runtime assíncrono; o Tauri já o usa internamente | — |
| `tokio-tungstenite` | cliente WebSocket | `tauri-plugin-websocket` rodaria no renderer, colocando o ticket dentro do WebView |
| `reqwest` (feature `cookies`, `json`) | HTTP com cookie jar persistente, exigido pela rotação de access token | escrever cookie jar à mão |
| `futures-util` | combinadores de stream para o socket | — |
| `keyring` | Windows Credential Manager | FFI direto com `windows-sys` (já presente): ~80 linhas de `unsafe` guardando uma senha. Trocar biblioteca auditada por FFI à mão num caminho de credencial é a troca errada |

---

### Task 1: Crate `mos-hermes` — contrato e tipos

**Files:**
- Create: `crates/mos-hermes/Cargo.toml`, `crates/mos-hermes/src/lib.rs`, `crates/mos-hermes/src/protocol.rs`
- Modify: `Cargo.toml` (workspace members e workspace.dependencies)

**Interfaces:**
- Consumes: nada
- Produces: `HermesEvent` (enum dos eventos de streaming), `HermesError`, `ConnectionState`, `Request::{session_create, prompt_submit, session_interrupt, session_close, session_resume, approval_respond}`.

- [ ] **Step 1: Criar o crate e registrá-lo no workspace**

`crates/mos-hermes/Cargo.toml`:

```toml
[package]
name = "mos-hermes"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
serde.workspace = true
serde_json = "1"
thiserror.workspace = true
tokio = { version = "1", features = ["rt", "sync", "time", "macros"] }
tokio-tungstenite = "0.24"
futures-util = "0.3"
reqwest = { version = "0.12", default-features = false, features = ["json", "cookies", "rustls-tls"] }
keyring = "3"
```

**Nunca** acrescentar `mos-storage-sqlite` aqui.

No `Cargo.toml` da raiz, em `members`, acrescentar `"crates/mos-hermes"`.

- [ ] **Step 2: Escrever o teste do vocabulário de eventos**

`crates/mos-hermes/src/protocol.rs`, módulo `tests`. Os frames abaixo são o formato real verificado no contrato (§4):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_ready_frame() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"gateway.ready","payload":{"skin":"default"}}}"#;
        assert!(matches!(HermesEvent::parse(frame).unwrap(), HermesEvent::Ready));
    }

    #[test]
    fn parses_a_message_delta() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"message.delta","payload":{"text":"ola"}}}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::MessageDelta { text } => assert_eq!(text, "ola"),
            other => panic!("esperava MessageDelta, veio {other:?}"),
        }
    }

    #[test]
    fn parses_an_approval_request() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"approval.request","payload":{"prompt":"rodar git push?"}}}"#;
        assert!(matches!(
            HermesEvent::parse(frame).unwrap(),
            HermesEvent::ApprovalRequest { .. }
        ));
    }

    /// Metodo desconhecido e erro de contrato explicito, nomeando o metodo —
    /// nunca falha generica. O contrato foi lido de um checkout local, e a VPS
    /// pode divergir numa atualizacao futura.
    #[test]
    fn unknown_event_names_itself() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"quantum.flux","payload":{}}}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::Unknown { kind } => assert_eq!(kind, "quantum.flux"),
            other => panic!("esperava Unknown, veio {other:?}"),
        }
    }

    #[test]
    fn parses_a_busy_error_response() {
        let frame = r#"{"jsonrpc":"2.0","error":{"code":4009,"message":"session busy"},"id":7}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::Rejected { code, message, .. } => {
                assert_eq!(code, 4009);
                assert_eq!(message, "session busy");
            }
            other => panic!("esperava Rejected, veio {other:?}"),
        }
    }
}
```

- [ ] **Step 3: Rodar e ver falhar**

```bash
cargo test -p mos-hermes
```

Esperado: FALHA na compilação — `HermesEvent` não existe.

- [ ] **Step 4: Implementar o protocolo**

Ver `protocol.rs` completo na implementação. Pontos que não podem ser negociados:

- `HermesEvent::Unknown { kind }` preserva o nome do tipo desconhecido;
- `Rejected` carrega `code` e `message` verbatim, porque `4009 session busy` tem tratamento de UI próprio;
- os construtores de request seguem os nomes do contrato exatamente: `session.create`, `prompt.submit`, `session.interrupt`, `session.close`, `session.resume`, `approval.respond`.

- [ ] **Step 5: Rodar e ver passar**

```bash
cargo test -p mos-hermes
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/mos-hermes
git commit -m "feat(hermes): crate da ponte e vocabulario do protocolo"
```

---

### Task 2: Autenticação e transporte

**Files:**
- Create: `crates/mos-hermes/src/auth.rs`, `crates/mos-hermes/src/transport.rs`

**Interfaces:**
- Consumes: `protocol.rs` da Task 1.
- Produces: `Credentials::{load, store, clear}` (Credential Manager); `Gateway::connect(base_url) -> Result<Session>`; trait `Transport` com duplo de teste.

- [ ] **Step 1: Credencial no Credential Manager**

`keyring::Entry::new("m-os", "hermes-gateway")`. Guarda `username:password`. O renderer nunca recebe nem um nem outro — só um booleano `hasCredentials`.

- [ ] **Step 2: Fluxo de login verificado**

Exatamente a ordem do contrato §3.1, sem atalho:

1. `GET /api/status` → ler `auth_required` e `auth_providers`;
2. `POST /auth/password-login` com `{provider, username, password}`;
3. `POST /api/auth/ws-ticket` → `{ticket, ttl_seconds}`;
4. `ws://.../api/ws?ticket=<ticket>` **imediatamente**;
5. esperar `gateway.ready`.

Os códigos de falha do login têm mensagens distintas e `429` **não** dispara retry.

- [ ] **Step 3: Trait `Transport` para testabilidade sem rede**

```rust
pub trait Transport: Send {
    fn send(&mut self, frame: String) -> Result<(), HermesError>;
    fn recv(&mut self) -> Result<Option<String>, HermesError>;
}
```

O duplo de teste reproduz os frames reais capturados do contrato. Todos os cenários da spec §10 rodam sem tocar a rede.

- [ ] **Step 4: Testes dos seis cenários**

Túnel fechado → `Offline` com causa nomeada. Socket cai no meio do turno → estado degrada sem perder o recebido. `session.resume` com id morto → recuperação por título. `approval.request` → render e resposta. Cancelamento → `session.interrupt`. Método desconhecido → erro nomeando o método.

- [ ] **Step 5: Verificar e commitar**

```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

---

### Task 3: Ponte, sessão e máquina de estados

**Files:**
- Create: `crates/mos-hermes/src/bridge.rs`

**Interfaces:**
- Produces: `HermesBridge::{connect, send, interrupt, respond_approval, close, state}`.

- [ ] **Step 1: Máquina de estados**

`Offline → Connecting → Online`, com `Error` por turno e não por conexão. **Sem retry automático em `Offline` frio**: túnel fechado não é falha transitória, e reconectar em loop contra socket recusado é ruído. Reconexão automática só depois de ter estado `Online` uma vez.

- [ ] **Step 2: Sessão persistente**

`session.create` com `title: "M/OS"` na primeira vez; `session_id` guardado localmente; `session.resume` ao reabrir; recuperação por título quando o id morreu; `session.close` ao sair.

O histórico **não** é guardado no M/OS: vive no `state.db` da VPS.

- [ ] **Step 3: Testes da máquina de estados e commit**

---

### Task 4: Fronteira Tauri e `hermes.ts`

**Files:**
- Create: `apps/desktop/src/hermes.ts`
- Modify: `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/Cargo.toml`

**Interfaces:**
- Produces: comandos `hermes_status`, `hermes_connect`, `hermes_send`, `hermes_interrupt`, `hermes_approve`, `hermes_set_credentials`; eventos `hermes-event` e `hermes-state`.

- [ ] **Step 1: Comandos e eventos**

Deltas de streaming vão por `emit`, reusando o caminho que `listen("capture-changed")` já usa. Nenhuma chamada de rede em componente React.

- [ ] **Step 2: `hermes.ts` espelhando o padrão de `api.ts`**

- [ ] **Step 3: Verificar e commitar**

---

### Task 5: Modo Hermes no Command

**Files:**
- Modify: `apps/desktop/src/App.tsx` (`CommandSurface`), `apps/desktop/src/App.css`

- [ ] **Step 1: Alternador `Search | Hermes`**

No campo, com `Tab` alternando e o modo **visível** — não folclore. O rodapé passa a anunciar `TAB HERMES` porque agora o atalho existe.

- [ ] **Step 2: Render da conversa**

Resposta em texto na tipografia do sistema. Raciocínio e tool calls acumulados e **escondidos** por padrão, reveláveis por ação discreta. Autoria do sistema marcada pela barra de 2px em sódio.

Enquanto o turno roda, o campo não aceita novo envio — oferece cancelar (`4009 session busy` é o que o servidor responderia).

- [ ] **Step 3: Estado de conexão na topbar**

Reusa o slot `.system-state` que a Spec A já construiu.

- [ ] **Step 4: Aprovação**

Renderiza o pedido e responde com `approval.respond`. **Fechar sem escolher é negar** — o servidor também tem `deny` como default, e aprovar por omissão seria o pior erro possível aqui.

- [ ] **Step 5: Verificação com o túnel aberto**

Os quatro cenários que só existem com sessão autenticada: `gateway.ready`, sequência de `message.delta`, `approval.request` → `approval.respond`, e `4009` em envio concorrente.

- [ ] **Step 6: Commit**

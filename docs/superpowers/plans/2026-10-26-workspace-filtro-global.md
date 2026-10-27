# Workspace como filtro global — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Estar num Workspace mostra só o que é dele — Tasks, Inbox, Projects, Biblioteca e o contexto do Hermes —, com TUDO para ver o conjunto.

**Architecture:** Uma coluna `workspace_id` opcional em `tasks`, `captures` e `conversations` (migration 0043), sincronizada como campo. A regra `pertence` mora em `mos-core`; o core devolve o **conjunto de ids** visíveis num Workspace (`EscopoDoWorkspace`) e o front só testa pertinência — nenhuma cópia da regra em TypeScript.

**Tech Stack:** Rust (mos-core, mos-storage-sqlite, Tauri 2), SQLite, React/TypeScript, vitest.

**Spec:** `docs/superpowers/specs/2026-10-26-workspace-filtro-global-design.md`

## Global Constraints

- Nada de plataforma em `mos-core` nem `mos-sync`.
- Migration nunca destrói dado: só `ADD COLUMN`, nullable.
- `workspace_id` vazio/NULL = sem Workspace = aparece em todos.
- Workspace arquivado não conta como declarado; item só com declarados arquivados aparece só em TUDO.
- `""` no front significa TUDO (mesma convenção de `m-os-current-workspace`).
- Fora do filtro: Calendário, Reuniões, Faculdade, Finance, Tempo, lembretes, atenção.
- Testar só o que cobre a mudança (memória do dono); `cargo fmt`, clippy `-D warnings` e `tsc` limpos antes de cada commit.
- Commits terminam com `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.

---

### Task 1: A coluna — migration, modelo, repositório e sync

**Files:**
- Create: `crates/mos-storage-sqlite/migrations/0043_workspace_dos_itens.sql`
- Modify: `crates/mos-core/src/work.rs` (`Task`), `crates/mos-core/src/capture.rs` (`Capture`, `NewCapture`), `crates/mos-core/src/conversation.rs` (`Conversation`)
- Modify: `crates/mos-storage-sqlite/src/repository.rs` (`CAPTURE_COLUMNS`, row → Capture, `insert_capture`, emissão do create)
- Modify: `crates/mos-storage-sqlite/src/work_repository.rs` (insert/update/select de Task, emissão)
- Modify: `crates/mos-storage-sqlite/src/conversation_repository.rs`
- Modify: `crates/mos-storage-sqlite/src/sync_projecao.rs` (mapas `task`, `capture`, `conversation`)
- Modify: registro de migrations em `crates/mos-storage-sqlite/src/lib.rs`

**Interfaces:**
- Produces: `Task.workspace_id: Option<WorkspaceId>`, `Capture.workspace_id: Option<WorkspaceId>`, `Conversation.workspace_id: Option<WorkspaceId>`, `NewCapture.workspace_id: Option<WorkspaceId>`; campo de sync `"workspaceId"` nos três tipos.

- [ ] **Step 1: Migration**

```sql
-- O Workspace dos itens soltos (ADR-093).
--
-- Task com Project ja herda os Workspaces dele por project_workspaces. O que
-- faltava era a Task SOLTA, a Capture e a conversa do Hermes poderem nascer
-- num Workspace. Uma coluna, e nao uma juncao: cada uma nasce em no maximo um.
-- NULL e "sem Workspace", que aparece em todos — e o estado de tudo o que ja
-- existe, entao nada some depois da migration.
BEGIN IMMEDIATE;
ALTER TABLE tasks ADD COLUMN workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE captures ADD COLUMN workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE conversations ADD COLUMN workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL;
CREATE INDEX tasks_workspace ON tasks(workspace_id) WHERE workspace_id IS NOT NULL;
CREATE INDEX captures_workspace ON captures(workspace_id) WHERE workspace_id IS NOT NULL;
CREATE INDEX conversations_workspace ON conversations(workspace_id) WHERE workspace_id IS NOT NULL;
COMMIT;
```

Registrar como as anteriores (seguir o padrão da 0042 em `lib.rs`).

- [ ] **Step 2: Teste de migration e ida-e-volta** — em `work_repository.rs` (mod tests): criar Workspace, Task com `workspace_id`, ler de volta e conferir; idem Capture em `repository.rs`. Banco com Task antiga continua lendo `workspace_id == None`.
- [ ] **Step 3: Rodar e ver falhar** — `cargo test -p mos-storage-sqlite workspace_dos_itens`
- [ ] **Step 4: Implementar** — campo nos structs (serde camelCase, `#[serde(default)]`), colunas no SELECT/INSERT, `"workspaceId"` na emissão do create e do update de Task/Capture/Conversation, `("workspaceId", "workspace_id")` nos três mapas de `sync_projecao.rs`. FK fora de ordem já é tratada pelas pendentes da projeção.
- [ ] **Step 5: Teste de sync** — duas réplicas (padrão dos testes de `sync_projecao`): Task criada com Workspace em A materializa com o mesmo `workspace_id` em B, inclusive com a op da Task chegando antes da do Workspace.
- [ ] **Step 6: Passar, fmt, clippy, commit** — `feat(workspace): a Task solta, a Capture e a conversa nascem num Workspace`

### Task 2: A regra — `pertence` e `escopo_do_workspace`

**Files:**
- Create: `crates/mos-core/src/escopo.rs`
- Modify: `crates/mos-core/src/lib.rs` (mod + reexport), `crates/mos-core/src/service.rs` (`WorkspaceService::escopo`)
- Modify: `crates/mos-core/src/ports.rs` + `crates/mos-storage-sqlite/src/work_repository.rs` (leitura das junções em lote)

**Interfaces:**
- Produces:
  ```rust
  pub struct Declarados<'a> { pub proprio: Option<WorkspaceId>, pub herdados: &'a [WorkspaceId] }
  pub fn pertence(declarados: &Declarados, alvo: Option<WorkspaceId>, ativos: &HashSet<WorkspaceId>) -> bool;
  #[derive(Serialize)] #[serde(rename_all = "camelCase")]
  pub struct EscopoDoWorkspace { pub tasks: Vec<String>, pub captures: Vec<String>, pub projects: Vec<String>, pub resources: Vec<String> }
  WorkspaceService::escopo(&self, workspace_id: &str) -> Result<EscopoDoWorkspace, CoreError>
  ```
  `alvo == None` é TUDO (sempre `true`).

- [ ] **Step 1: Testes da regra** (em `escopo.rs`):

```rust
#[test] fn tudo_ve_tudo() // alvo None → true, com e sem declarados
#[test] fn sem_declarado_aparece_em_todos()
#[test] fn proprio_decide() // proprio=A: em A sim, em B nao
#[test] fn herdado_do_project_conta() // herdados=[B], proprio=A: em A e em B
#[test] fn arquivado_nao_conta() // proprio=A arquivado, herdados=[B]: em B sim, em C nao
#[test] fn so_arquivados_fica_so_em_tudo() // proprio=A arquivado: em B nao, em None sim
```

- [ ] **Step 2: Ver falhar** — `cargo test -p mos-core escopo`
- [ ] **Step 3: Implementar**

```rust
pub fn pertence(d: &Declarados, alvo: Option<WorkspaceId>, ativos: &HashSet<WorkspaceId>) -> bool {
    let Some(alvo) = alvo else { return true };
    let declarados: Vec<WorkspaceId> = d.proprio.into_iter().chain(d.herdados.iter().copied()).collect();
    if declarados.is_empty() { return true; }
    let vivos: Vec<WorkspaceId> = declarados.into_iter().filter(|w| ativos.contains(w)).collect();
    vivos.contains(&alvo)
}
```

- [ ] **Step 4: `escopo`** — lê Tasks, Captures ativas, Projects, Resources, `project_workspaces`, `resource_workspaces` e os Workspaces ativos (um método de port que devolve as junções em lote, sem N+1), e aplica `pertence` a cada item. Task herda os Workspaces do Project. Teste de serviço com storage real: um cenário com PROSUL/DEV cobrindo cada tipo.
- [ ] **Step 5: Comando Tauri** `workspace_scope(workspace_id: String) -> EscopoDoWorkspace` em `apps/desktop/src-tauri/src/lib.rs`, ao lado dos comandos de Workspace.
- [ ] **Step 6: Passar, fmt, clippy, commit** — `feat(workspace): pertence no core — o escopo de um Workspace e um conjunto de ids`

### Task 3: Nascer e mover

**Files:**
- Modify: `crates/mos-core/src/service.rs` (`CreateCaptureInput.workspace_id`, `CreateTaskInput.workspace_id`, `UpdateTaskInput.workspace_id` + `from_task`, Capture → Task herda da Capture), `CaptureService::set_workspace`
- Modify: ports/repositório da Capture (`set_capture_workspace`, com emissão de update)
- Modify: `apps/desktop/src-tauri/src/lib.rs` (comandos de criação passam o campo; `set_capture_workspace`)

**Interfaces:**
- Consumes: Task 1.
- Produces: `CreateCaptureInput { content, source, #[serde(default)] workspace_id: Option<String> }`; `CreateTaskInput.workspace_id: Option<String>` (`#[serde(default)]`); `UpdateTaskInput.workspace_id: Option<String>`; comando `set_capture_workspace(id, workspace_id: Option<String>)`.

- [ ] **Step 1: Testes** — criar Capture/Task com Workspace grava o campo; `UpdateTaskInput::from_task` preserva `workspace_id` (o teste que impede o apagamento silencioso descrito no comentário de `from_task`); Capture de PROSUL triada para Task sem `workspace_id` explícito nasce em PROSUL; Workspace inexistente → `InvalidInput`.
- [ ] **Step 2: Ver falhar, implementar, passar.** Todos os chamadores existentes de `CreateCaptureInput { .. }` e `CreateTaskInput { .. }` (mos-web, agente_exec, ingest, reunião) ganham `workspace_id: None` ou `..Default::default()` — o bolso continua mandando sem Workspace.
- [ ] **Step 3: fmt, clippy, commit** — `feat(workspace): criar herda o Workspace atual, e a Task triada herda o da Capture`

### Task 4: O Hermes enxerga o Workspace da conversa

**Files:**
- Modify: `crates/mos-core/src/conversation.rs` (`ConversationService::create(workspace_id)`, `conversations(workspace)` filtrado por `pertence`)
- Modify: `crates/mos-core/src/agente_exec.rs` (`Contexto.workspace_id: Option<String>`; `CaptureCreate`, `TaskCreate`, `CaptureToTask` passam adiante)
- Modify: `apps/desktop/src-tauri/src/hermes.rs` (candidatos e contexto filtrados pelo escopo da conversa), `apps/desktop/src-tauri/src/jarvis.rs` (Contexto com o Workspace da conversa da proposta), `apps/mos-web/src` (Contexto com `None`)

**Interfaces:**
- Consumes: Tasks 2 e 3.
- Produces: comando `conversation_create(workspace_id: Option<String>)`; `conversations_list(workspace_id: Option<String>)`.

- [ ] **Step 1: Testes** — em `agente_exec`: `TaskCreate` com `Contexto.workspace_id = Some(A)` grava A; em `conversation`: listar em A mostra conversas de A e as sem Workspace, não as de B.
- [ ] **Step 2: Implementar.** Filtro dos candidatos: depois da busca, descartar o que `escopo` não contém (Task, Capture, Project, Resource); o teto de candidatos vale depois do filtro.
- [ ] **Step 3: fmt, clippy, commit** — `feat(hermes): em PROSUL ele enxerga PROSUL — contexto e conversas pelo Workspace`

### Task 5: O front — seletor global, filtro e triagem

**Files:**
- Create: `apps/desktop/src/escopoDoWorkspace.ts` + `escopoDoWorkspace.test.ts` (filtro puro: `filtrarPorEscopo(itens, escopo, semWorkspace)`)
- Modify: `apps/desktop/src/api.ts`, `apps/desktop/src/types.ts`
- Modify: `apps/desktop/src/App.tsx` (seletor sobe para o topo; estado do escopo; Tarefas, Inbox, Projects, Biblioteca filtram; criação passa `currentWorkspaceId || null`; campo WORKSPACE no `TaskDrawer` e no `CaptureViewer`; filtro SEM WORKSPACE), `apps/desktop/src/HermesPage.tsx`, janela de captura rápida
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `workspace_scope`, `set_capture_workspace`, `conversation_create(workspace_id)`, campos novos.
- Produces: `type EscopoDoWorkspace = { tasks: string[]; captures: string[]; projects: string[]; resources: string[] }`; `filtrarPorEscopo<T extends { id: string }>(itens: T[], escopo: EscopoDoWorkspace | null, chave: keyof EscopoDoWorkspace): T[]` — `escopo === null` é TUDO.

- [ ] **Step 1: Testes do filtro** — null devolve tudo; com escopo devolve só os ids do conjunto, na ordem original; SEM WORKSPACE devolve só itens com `workspaceId` vazio.
- [ ] **Step 2: Implementar filtro e api.** Escopo recarrega quando muda o Workspace atual e no `data-changed`.
- [ ] **Step 3: Telas.** Seletor no topo com TUDO primeiro; páginas filtradas; vazio com escopo oferecendo TUDO; select WORKSPACE nos detalhes (Task com Workspace pelo Project mostra "pelo Project").
- [ ] **Step 4: `tsc`, vitest, ver na tela** com a skill `ver-o-app` (dev app ao lado do instalado, sem ligar sync).
- [ ] **Step 5: Commit** — `feat(desktop): o Workspace vira filtro de todo o app, com TUDO e triagem`

### Task 6: ADR e documentação

**Files:**
- Modify: `docs/DECISIONS.md` (ADR-093 + linha no índice), `docs/TECHNICAL-FOUNDATION-V0.2-WORKSPACES.md` (nota apontando para a ADR), `docs/SYNC.md` (campo novo nos três tipos)

- [ ] **Step 1: Escrever a ADR-093** com contexto, as cinco decisões da spec §1, os oito eixos e os limites (bolso sem seletor; domínios fora do filtro).
- [ ] **Step 2: Commit** — `docs(adr): ADR-093 — o Workspace e filtro global`

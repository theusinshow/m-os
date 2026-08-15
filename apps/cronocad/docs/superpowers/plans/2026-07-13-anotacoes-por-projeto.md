# Anotacoes e Pendencias por Projeto — Plano de Implementacao

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar a cada projeto um bloco de anotacoes livres e uma lista de pendencias, e mostrar as pendencias abertas no Painel — onde elas encontram o usuario, em vez de esperar que ele va atras.

**Architecture:** Migration 0005 acrescenta `projects.notes` (texto livre, 1-para-1) e a tabela `project_todos` (N-para-1, hard delete). Um `repository/notes.rs` novo concentra as consultas; seis comandos Tauri finos as expoem. No frontend, um `notesStore` (Zustand) espelha o backend, um modal em Projetos edita tudo, e um painel novo no Painel lista as pendencias abertas — ordenadas por uma funcao pura testavel (`src/lib/todos.ts`).

**Tech Stack:** Tauri 2 + Rust (sqlx, SQLite), React 18, TypeScript estrito, Zustand, Tailwind, Vitest, `cargo test`.

Spec: `docs/superpowers/specs/2026-07-13-anotacoes-por-projeto-design.md`

## Global Constraints

- **Nunca** editar uma migration ja aplicada. A nova e a `0005`, registrada em `src-tauri/src/database/mod.rs`.
- Rust: **sem `unwrap()`** em caminho de producao; erros como `AppError` (`Database` | `Validation` | `NotFound` | `Conflict` | `Io`).
- Valores em consultas **sempre via `bind`**; nenhum SQL montado por concatenacao de dados; nenhum SQL exposto ao frontend.
- Structs de saida: `#[derive(Debug, Clone, Serialize, sqlx::FromRow)]` + `#[serde(rename_all = "camelCase")]`. Entradas: `#[derive(Debug, Deserialize)]` + camelCase.
- Fluxo obrigatorio: `store -> service (invoke) -> command (valida) -> repository -> SQLite`. **Nao** executar SQL em componente React.
- TypeScript estrito, **`any` proibido**; `npm run lint` com **0 warnings**; alias `@/` -> `src/`.
- Textos da UI em portugues **sem acentos** (padrao do codebase inteiro).
- Timestamps ISO 8601 UTC (`repository::now_iso()`); ids UUID v4 (`repository::new_id()`).
- **Nao tocar** em `time_entries`, `active_timer` nem no motor do cronometro.
- **Sem notificacao, sem data/hora, sem agendamento** nas pendencias: "lembrete" aqui significa apenas "fica visivel".

---

### Task 1: Migration 0005, modelos e repositorio

**Files:**
- Create: `src-tauri/migrations/0005_project_notes.sql`
- Create: `src-tauri/src/repository/notes.rs`
- Modify: `src-tauri/src/database/mod.rs:23-58` (constante + `migrations()`)
- Modify: `src-tauri/src/models.rs` (campo `notes` em `Project`; structs `ProjectTodo`)
- Modify: `src-tauri/src/repository/projects.rs:10-11` (`COLUMNS`)
- Modify: `src-tauri/src/repository/mod.rs:7-13` (`pub mod notes;`)
- Test: `src-tauri/src/repository/tests.rs` (acrescentar testes ao final)

**Interfaces:**
- Consumes: `repository::{new_id, now_iso}`, `AppError`.
- Produces (usado pela Task 2):
  ```rust
  // models.rs
  pub struct ProjectTodo { pub id: String, pub project_id: String, pub text: String,
                           pub done: bool, pub done_at: Option<String>,
                           pub created_at: String, pub updated_at: String }

  // repository/notes.rs
  pub async fn set_notes(pool: &Pool<Sqlite>, project_id: &str, notes: Option<String>) -> Result<Project, AppError>;
  pub async fn list_todos(pool: &Pool<Sqlite>) -> Result<Vec<ProjectTodo>, AppError>;
  pub async fn create_todo(pool: &Pool<Sqlite>, project_id: &str, text: &str) -> Result<ProjectTodo, AppError>;
  pub async fn set_todo_done(pool: &Pool<Sqlite>, id: &str, done: bool) -> Result<ProjectTodo, AppError>;
  pub async fn update_todo_text(pool: &Pool<Sqlite>, id: &str, text: &str) -> Result<ProjectTodo, AppError>;
  pub async fn delete_todo(pool: &Pool<Sqlite>, id: &str) -> Result<(), AppError>;
  ```

**ATENCAO — armadilha real:** o `Project` e lido com `sqlx::FromRow` a partir da lista `COLUMNS` de `repository/projects.rs:10-11`. Ao adicionar o campo `notes` na struct **e obrigatorio** adiciona-lo tambem em `COLUMNS`, senao **todas** as consultas de projeto passam a falhar em runtime.

- [ ] **Step 1: Escrever a migration**

Criar `src-tauri/migrations/0005_project_notes.sql`:

```sql
-- Anotacoes e pendencias por projeto.
--
-- `projects.notes`: bloco de texto livre, 1-para-1 com o projeto (contexto solto
-- que o usuario re-le ao voltar ao trabalho).
--
-- `project_todos`: pendencias curtas. Hard delete de proposito (sem `deleted_at`):
-- uma pendencia nao e registro de tempo nem gera cobranca — a regra de preservar
-- historico protege `time_entries`, nao um item de checklist.

ALTER TABLE projects ADD COLUMN notes TEXT;

CREATE TABLE project_todos (
  id          TEXT PRIMARY KEY,
  project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  text        TEXT NOT NULL,
  done        INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
  done_at     TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE INDEX idx_project_todos_project ON project_todos(project_id);
CREATE INDEX idx_project_todos_done ON project_todos(done);
```

- [ ] **Step 2: Registrar a migration**

Em `src-tauri/src/database/mod.rs`, apos a linha do `MIGRATION_0004` (:27):

```rust
pub const MIGRATION_0005: &str = include_str!("../../migrations/0005_project_notes.sql");
```

E no fim do `vec![...]` de `migrations()`, apos a entrada `version: 4`:

```rust
        Migration {
            version: 5,
            description: "anotacoes e pendencias por projeto",
            sql: MIGRATION_0005,
            kind: MigrationKind::Up,
        },
```

- [ ] **Step 3: Acrescentar o campo `notes` ao `Project` e criar o `ProjectTodo`**

Em `src-tauri/src/models.rs`, na struct `Project` (:94), acrescentar o campo **ao final** da struct (a ordem nao importa para o `FromRow`, que casa por nome):

```rust
    pub notes: Option<String>,
```

E, logo apos o bloco de `ProjectInput` (:111-118), acrescentar:

```rust
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTodo {
    pub id: String,
    pub project_id: String,
    pub text: String,
    pub done: bool,
    pub done_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Incluir `notes` no `COLUMNS` de projetos**

Em `src-tauri/src/repository/projects.rs:10-11`, substituir a constante por:

```rust
const COLUMNS: &str = "id, client_id, name, code, description, hourly_rate_cents, \
     budget_minutes, status, color, created_at, updated_at, archived_at, notes";
```

(Os `INSERT`/`UPDATE` existentes nao listam `notes`, entao continuam corretos: criar um projeto deixa `notes` nulo e editar o projeto **nao apaga** as anotacoes.)

- [ ] **Step 5: Escrever os testes que falham**

Em `src-tauri/src/repository/tests.rs`, atualizar o import da migration (:10) e o array do `setup()` (:23) para incluir a `0005`:

```rust
use crate::database::{
    MIGRATION_0001, MIGRATION_0002, MIGRATION_0003, MIGRATION_0004, MIGRATION_0005,
};
```

```rust
    for migration in [
        MIGRATION_0001,
        MIGRATION_0002,
        MIGRATION_0003,
        MIGRATION_0004,
        MIGRATION_0005,
    ] {
```

Acrescentar `notes` ao import do repositorio (:15):

```rust
use crate::repository::{clients, new_id, notes, now_iso, projects, time_entries, timer};
```

E acrescentar os testes ao final do arquivo:

```rust
#[tokio::test]
async fn pendencia_criada_marcada_e_excluida() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();

    let todo = notes::create_todo(&pool, &project.id, "Revisar cortes")
        .await
        .unwrap();
    assert_eq!(todo.text, "Revisar cortes");
    assert!(!todo.done);
    assert!(todo.done_at.is_none());

    let done = notes::set_todo_done(&pool, &todo.id, true).await.unwrap();
    assert!(done.done);
    assert!(done.done_at.is_some());

    let reopened = notes::set_todo_done(&pool, &todo.id, false).await.unwrap();
    assert!(!reopened.done);
    assert!(reopened.done_at.is_none());

    notes::delete_todo(&pool, &todo.id).await.unwrap();
    assert!(notes::list_todos(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn pendencia_com_texto_vazio_e_rejeitada() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();

    assert!(notes::create_todo(&pool, &project.id, "   ").await.is_err());
}

#[tokio::test]
async fn anotacoes_do_projeto_persistem_e_sobrevivem_a_edicao() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    assert!(project.notes.is_none());

    let saved = notes::set_notes(&pool, &project.id, Some("Esperando topografia".into()))
        .await
        .unwrap();
    assert_eq!(saved.notes.as_deref(), Some("Esperando topografia"));

    // Editar o projeto nao pode apagar as anotacoes.
    let edited = projects::update(
        &pool,
        &project.id,
        project_input("Proj renomeado", 7000).validate().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(edited.notes.as_deref(), Some("Esperando topografia"));

    // Texto em branco limpa as anotacoes.
    let cleared = notes::set_notes(&pool, &project.id, Some("  ".into()))
        .await
        .unwrap();
    assert!(cleared.notes.is_none());
}

#[tokio::test]
async fn excluir_projeto_leva_as_pendencias_junto() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    notes::create_todo(&pool, &project.id, "Enviar PDF")
        .await
        .unwrap();

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM projects WHERE id = ?1")
        .bind(&project.id)
        .execute(&pool)
        .await
        .unwrap();

    assert!(notes::list_todos(&pool).await.unwrap().is_empty());
}
```

- [ ] **Step 6: Rodar os testes e confirmar que falham**

Run: `cd src-tauri && cargo test`
Expected: FAIL na compilacao — `unresolved import crate::repository::notes` (o modulo ainda nao existe).

- [ ] **Step 7: Criar o repositorio**

Criar `src-tauri/src/repository/notes.rs`:

```rust
//! Repositorio de anotacoes e pendencias por projeto.
//!
//! `set_notes` grava o texto livre na coluna `projects.notes` (1-para-1).
//! As pendencias vivem em `project_todos` e usam **hard delete**: nao sao
//! registro de tempo nem geram cobranca, entao nao ha soft delete aqui.

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::{Project, ProjectTodo};

use super::{new_id, now_iso};

const PROJECT_COLUMNS: &str = "id, client_id, name, code, description, hourly_rate_cents, \
     budget_minutes, status, color, created_at, updated_at, archived_at, notes";

const TODO_COLUMNS: &str = "id, project_id, text, done, done_at, created_at, updated_at";

/// Normaliza o texto de uma pendencia; rejeita vazio/em branco.
fn clean_text(text: &str) -> Result<String, AppError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "a pendencia precisa de um texto".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Grava o bloco de anotacoes do projeto. Texto vazio/em branco vira `NULL`.
pub async fn set_notes(
    pool: &Pool<Sqlite>,
    project_id: &str,
    notes: Option<String>,
) -> Result<Project, AppError> {
    let value = notes
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let now = now_iso();
    let project = sqlx::query_as::<_, Project>(&format!(
        "UPDATE projects SET notes = ?2, updated_at = ?3 WHERE id = ?1 \
         RETURNING {PROJECT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&value)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(project)
}

/// Lista todas as pendencias (abertas e concluidas). O Painel filtra as abertas.
pub async fn list_todos(pool: &Pool<Sqlite>) -> Result<Vec<ProjectTodo>, AppError> {
    let rows = sqlx::query_as::<_, ProjectTodo>(&format!(
        "SELECT {TODO_COLUMNS} FROM project_todos ORDER BY created_at"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Cria uma pendencia aberta no projeto.
pub async fn create_todo(
    pool: &Pool<Sqlite>,
    project_id: &str,
    text: &str,
) -> Result<ProjectTodo, AppError> {
    let text = clean_text(text)?;
    let id = new_id();
    let now = now_iso();
    let todo = sqlx::query_as::<_, ProjectTodo>(&format!(
        "INSERT INTO project_todos (id, project_id, text, done, done_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 0, NULL, ?4, ?4) \
         RETURNING {TODO_COLUMNS}"
    ))
    .bind(&id)
    .bind(project_id)
    .bind(&text)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// Marca/desmarca uma pendencia, mantendo `done_at` coerente.
pub async fn set_todo_done(
    pool: &Pool<Sqlite>,
    id: &str,
    done: bool,
) -> Result<ProjectTodo, AppError> {
    let now = now_iso();
    let todo = sqlx::query_as::<_, ProjectTodo>(&format!(
        "UPDATE project_todos SET \
         done = ?2, \
         done_at = CASE WHEN ?2 = 1 THEN ?3 ELSE NULL END, \
         updated_at = ?3 \
         WHERE id = ?1 \
         RETURNING {TODO_COLUMNS}"
    ))
    .bind(id)
    .bind(done)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// Corrige o texto de uma pendencia.
pub async fn update_todo_text(
    pool: &Pool<Sqlite>,
    id: &str,
    text: &str,
) -> Result<ProjectTodo, AppError> {
    let text = clean_text(text)?;
    let now = now_iso();
    let todo = sqlx::query_as::<_, ProjectTodo>(&format!(
        "UPDATE project_todos SET text = ?2, updated_at = ?3 WHERE id = ?1 \
         RETURNING {TODO_COLUMNS}"
    ))
    .bind(id)
    .bind(&text)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// Remove a pendencia definitivamente.
pub async fn delete_todo(pool: &Pool<Sqlite>, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM project_todos WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 8: Declarar o modulo**

Em `src-tauri/src/repository/mod.rs`, na lista de modulos (:7-13), acrescentar em ordem alfabetica (entre `monitored_apps` e `projects`):

```rust
pub mod notes;
```

- [ ] **Step 9: Rodar os testes e confirmar que passam**

Run: `cd src-tauri && cargo test`
Expected: PASS — os testes antigos **e** os 4 novos.

- [ ] **Step 10: Formatar, lintar e commitar**

Run: `cd src-tauri && cargo fmt && cargo clippy -- -D warnings`
Expected: sem erros.

```bash
git add src-tauri/migrations/0005_project_notes.sql src-tauri/src/database/mod.rs src-tauri/src/models.rs src-tauri/src/repository/notes.rs src-tauri/src/repository/mod.rs src-tauri/src/repository/projects.rs src-tauri/src/repository/tests.rs
git commit -m "feat(db): migration 0005 com anotacoes e pendencias por projeto"
```

---

### Task 2: Comandos Tauri

**Files:**
- Modify: `src-tauri/src/commands/mod.rs` (imports + secao nova de comandos)
- Modify: `src-tauri/src/lib.rs:46-84` (`invoke_handler`)

**Interfaces:**
- Consumes: `repository::notes` (assinaturas no bloco "Produces" da Task 1) e `models::ProjectTodo`.
- Produces (nomes exatos dos comandos, usados pelo servico da Task 3):
  `update_project_notes` · `list_todos` · `create_todo` · `set_todo_done` · `update_todo_text` · `delete_todo`

- [ ] **Step 1: Acrescentar os comandos**

Em `src-tauri/src/commands/mod.rs`, incluir `ProjectTodo` no `use crate::models::{...}` (:13-17) e `notes` no `use crate::repository::{...}` (:19-21).

Depois, ao final da secao de Projects (apos `set_project_status`, :134), acrescentar:

```rust
// ---------------------------------------------------------------------------
// Anotacoes e pendencias por projeto
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn update_project_notes(
    db: State<'_, DbInstances>,
    project_id: String,
    notes: Option<String>,
) -> Result<Project, AppError> {
    let pool = database::pool(&db).await?;
    notes::set_notes(&pool, &project_id, notes).await
}

#[tauri::command]
pub async fn list_todos(db: State<'_, DbInstances>) -> Result<Vec<ProjectTodo>, AppError> {
    let pool = database::pool(&db).await?;
    notes::list_todos(&pool).await
}

#[tauri::command]
pub async fn create_todo(
    db: State<'_, DbInstances>,
    project_id: String,
    text: String,
) -> Result<ProjectTodo, AppError> {
    let pool = database::pool(&db).await?;
    notes::create_todo(&pool, &project_id, &text).await
}

#[tauri::command]
pub async fn set_todo_done(
    db: State<'_, DbInstances>,
    id: String,
    done: bool,
) -> Result<ProjectTodo, AppError> {
    let pool = database::pool(&db).await?;
    notes::set_todo_done(&pool, &id, done).await
}

#[tauri::command]
pub async fn update_todo_text(
    db: State<'_, DbInstances>,
    id: String,
    text: String,
) -> Result<ProjectTodo, AppError> {
    let pool = database::pool(&db).await?;
    notes::update_todo_text(&pool, &id, &text).await
}

#[tauri::command]
pub async fn delete_todo(db: State<'_, DbInstances>, id: String) -> Result<(), AppError> {
    let pool = database::pool(&db).await?;
    notes::delete_todo(&pool, &id).await
}
```

- [ ] **Step 2: Registrar os handlers**

Em `src-tauri/src/lib.rs`, dentro de `tauri::generate_handler![...]`, logo apos `commands::set_project_status,` (:58):

```rust
            commands::update_project_notes,
            commands::list_todos,
            commands::create_todo,
            commands::set_todo_done,
            commands::update_todo_text,
            commands::delete_todo,
```

- [ ] **Step 3: Compilar e verificar**

Run: `cd src-tauri && cargo build && cargo clippy -- -D warnings && cargo test`
Expected: compila sem erros; clippy sem warnings; testes passando.

Um comando **nao registrado** no `invoke_handler` compila normalmente e so falha em runtime ("command not found"). Conferir visualmente que as seis linhas estao la.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(commands): expor anotacoes e pendencias por projeto"
```

---

### Task 3: Tipos, servico e store no frontend

**Files:**
- Modify: `src/types/domain.ts` (campo `notes` em `Project`; tipo `ProjectTodo`)
- Create: `src/services/notes.ts`
- Create: `src/stores/notesStore.ts`

**Interfaces:**
- Consumes: os seis comandos da Task 2; `invokeCommand` (`@/services/tauri`).
- Produces (usado pelas Tasks 5 e 6):
  ```ts
  // types/domain.ts
  export interface ProjectTodo {
    id: string; projectId: string; text: string;
    done: boolean; doneAt: string | null;
    createdAt: string; updatedAt: string;
  }
  // Project ganha: notes: string | null;

  // stores/notesStore.ts
  interface NotesState {
    todos: ProjectTodo[];
    loaded: boolean;
    error: string | null;
    load: () => Promise<void>;
    createTodo: (projectId: string, text: string) => Promise<void>;
    setTodoDone: (id: string, done: boolean) => Promise<void>;
    updateTodoText: (id: string, text: string) => Promise<void>;
    deleteTodo: (id: string) => Promise<void>;
  }
  export const useNotesStore: UseBoundStore<StoreApi<NotesState>>;
  ```
  As **anotacoes de texto livre** nao vivem neste store: elas sao um campo do `Project`, entao a acao mora no `catalogStore` (que ja e o dono da lista de projetos):
  ```ts
  // catalogStore ganha:
  updateProjectNotes: (projectId: string, notes: string) => Promise<Project>;
  ```

- [ ] **Step 1: Acrescentar os tipos**

Em `src/types/domain.ts`, na interface `Project`, acrescentar apos `budgetMinutes` (:61):

```ts
  /** Bloco de anotacoes livres do projeto (null quando vazio). */
  notes: string | null;
```

E acrescentar, apos a interface `Project`:

```ts
/** Pendencia de um projeto (checklist). Sem data e sem notificacao: so fica visivel. */
export interface ProjectTodo {
  id: string;
  projectId: string;
  text: string;
  done: boolean;
  doneAt: string | null;
  createdAt: string;
  updatedAt: string;
}
```

- [ ] **Step 2: Criar o servico**

Criar `src/services/notes.ts`:

```ts
/**
 * Servico de anotacoes e pendencias: wrappers tipados sobre os comandos Tauri.
 */

import type { Project, ProjectTodo } from "@/types/domain";
import { invokeCommand } from "./tauri";

export function updateProjectNotes(
  projectId: string,
  notes: string | null,
): Promise<Project> {
  return invokeCommand<Project>("update_project_notes", { projectId, notes });
}

export function listTodos(): Promise<ProjectTodo[]> {
  return invokeCommand<ProjectTodo[]>("list_todos");
}

export function createTodo(
  projectId: string,
  text: string,
): Promise<ProjectTodo> {
  return invokeCommand<ProjectTodo>("create_todo", { projectId, text });
}

export function setTodoDone(id: string, done: boolean): Promise<ProjectTodo> {
  return invokeCommand<ProjectTodo>("set_todo_done", { id, done });
}

export function updateTodoText(id: string, text: string): Promise<ProjectTodo> {
  return invokeCommand<ProjectTodo>("update_todo_text", { id, text });
}

export function deleteTodo(id: string): Promise<void> {
  return invokeCommand<void>("delete_todo", { id });
}
```

- [ ] **Step 3: Criar o store das pendencias**

Criar `src/stores/notesStore.ts`:

```ts
/**
 * Store das pendencias por projeto.
 *
 * O banco e a fonte da verdade: cada acao chama um comando e o estado local
 * reflete o retorno. Guarda **todas** as pendencias (abertas e concluidas); quem
 * filtra e ordena para exibicao e `src/lib/todos.ts`.
 */

import { create } from "zustand";
import type { ProjectTodo } from "@/types/domain";
import {
  createTodo as apiCreateTodo,
  deleteTodo as apiDeleteTodo,
  listTodos,
  setTodoDone as apiSetTodoDone,
  updateTodoText as apiUpdateTodoText,
} from "@/services/notes";

interface NotesState {
  todos: ProjectTodo[];
  loaded: boolean;
  error: string | null;

  load: () => Promise<void>;
  createTodo: (projectId: string, text: string) => Promise<void>;
  setTodoDone: (id: string, done: boolean) => Promise<void>;
  updateTodoText: (id: string, text: string) => Promise<void>;
  deleteTodo: (id: string) => Promise<void>;
}

function messageOf(err: unknown): string {
  return typeof err === "string"
    ? err
    : err instanceof Error
      ? err.message
      : String(err);
}

export const useNotesStore = create<NotesState>((set, get) => ({
  todos: [],
  loaded: false,
  error: null,

  load: async () => {
    try {
      const todos = await listTodos();
      set({ todos, loaded: true, error: null });
    } catch (err) {
      set({ loaded: true, error: messageOf(err) });
    }
  },

  createTodo: async (projectId, text) => {
    const todo = await apiCreateTodo(projectId, text);
    set({ todos: [...get().todos, todo] });
  },

  setTodoDone: async (id, done) => {
    const updated = await apiSetTodoDone(id, done);
    set({ todos: get().todos.map((t) => (t.id === id ? updated : t)) });
  },

  updateTodoText: async (id, text) => {
    const updated = await apiUpdateTodoText(id, text);
    set({ todos: get().todos.map((t) => (t.id === id ? updated : t)) });
  },

  deleteTodo: async (id) => {
    await apiDeleteTodo(id);
    set({ todos: get().todos.filter((t) => t.id !== id) });
  },
}));
```

- [ ] **Step 4: Acrescentar a acao de anotacoes ao `catalogStore`**

Em `src/stores/catalogStore.ts`:

4a. No import de servicos, acrescentar:

```ts
import { updateProjectNotes as apiUpdateProjectNotes } from "@/services/notes";
```

4b. Na interface `CatalogState`, apos `setProjectStatus` (:45):

```ts
  updateProjectNotes: (projectId: string, notes: string) => Promise<Project>;
```

4c. Na implementacao, apos `setProjectStatus` (:130-141):

```ts
  updateProjectNotes: async (projectId, notes) => {
    const updated = await apiUpdateProjectNotes(projectId, notes.trim() || null);
    set({
      projects: get().projects.map((p) => (p.id === projectId ? updated : p)),
    });
    return updated;
  },
```

- [ ] **Step 5: Verificar tipos e lint**

Run: `npm run typecheck && npm run lint`
Expected: sem erros; 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add src/types/domain.ts src/services/notes.ts src/stores/notesStore.ts src/stores/catalogStore.ts
git commit -m "feat(notes): tipos, servico e store de anotacoes e pendencias"
```

---

### Task 4: Regra de exibicao (funcao pura)

**Files:**
- Create: `src/lib/todos.ts`
- Test: `src/lib/todos.test.ts`

**Interfaces:**
- Consumes: `ProjectTodo` e `Project` (`@/types/domain`).
- Produces (usado pela Task 6):
  ```ts
  export interface TodoGroup { project: Project; todos: ProjectTodo[] }
  export function openTodosByProject(
    todos: ProjectTodo[],
    projects: Project[],
    activeProjectId: string | null,
  ): TodoGroup[];
  ```
  Regra: descarta concluidas; descarta pendencias cujo projeto nao esta na lista (ex.: projeto arquivado); agrupa por projeto; o projeto do cronometro ativo vem **primeiro**; os demais em ordem alfabetica; grupos vazios nao aparecem.

Isolar essa regra numa funcao pura e o que torna o Painel testavel sem renderizar nada — mesmo padrao de `src/lib/duration.ts`.

- [ ] **Step 1: Escrever o teste que falha**

Criar `src/lib/todos.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { openTodosByProject } from "./todos";
import type { Project, ProjectTodo } from "@/types/domain";

function project(id: string, name: string): Project {
  return {
    id,
    clientId: null,
    name,
    code: null,
    description: null,
    hourlyRateCents: 9000,
    budgetMinutes: 0,
    status: "active",
    color: null,
    notes: null,
    createdAt: "2026-07-11T08:00:00Z",
    updatedAt: "2026-07-11T08:00:00Z",
    archivedAt: null,
  };
}

function todo(id: string, projectId: string, done = false): ProjectTodo {
  return {
    id,
    projectId,
    text: `Pendencia ${id}`,
    done,
    doneAt: done ? "2026-07-12T10:00:00Z" : null,
    createdAt: "2026-07-11T08:00:00Z",
    updatedAt: "2026-07-11T08:00:00Z",
  };
}

const aurora = project("p1", "Aurora");
const belaVista = project("p2", "Bela Vista");
const projects = [aurora, belaVista];

describe("openTodosByProject", () => {
  it("esconde as pendencias concluidas", () => {
    const groups = openTodosByProject(
      [todo("t1", "p1"), todo("t2", "p1", true)],
      projects,
      null,
    );
    expect(groups).toHaveLength(1);
    expect(groups[0].todos.map((t) => t.id)).toEqual(["t1"]);
  });

  it("agrupa por projeto e ordena por nome quando nao ha cronometro", () => {
    const groups = openTodosByProject(
      [todo("t1", "p2"), todo("t2", "p1")],
      projects,
      null,
    );
    expect(groups.map((g) => g.project.name)).toEqual(["Aurora", "Bela Vista"]);
  });

  it("coloca o projeto do cronometro ativo em primeiro", () => {
    const groups = openTodosByProject(
      [todo("t1", "p1"), todo("t2", "p2")],
      projects,
      "p2",
    );
    expect(groups.map((g) => g.project.name)).toEqual(["Bela Vista", "Aurora"]);
  });

  it("ignora pendencias de projetos fora da lista (ex.: arquivados)", () => {
    const groups = openTodosByProject([todo("t1", "p9")], projects, null);
    expect(groups).toEqual([]);
  });

  it("nao cria grupo para projeto sem pendencias abertas", () => {
    const groups = openTodosByProject([todo("t1", "p1", true)], projects, null);
    expect(groups).toEqual([]);
  });

  it("lista vazia resulta em nenhum grupo", () => {
    expect(openTodosByProject([], projects, null)).toEqual([]);
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npx vitest run src/lib/todos.test.ts`
Expected: FAIL — "Failed to resolve import ./todos".

- [ ] **Step 3: Implementar**

Criar `src/lib/todos.ts`:

```ts
/**
 * Regra de exibicao das pendencias no Painel (funcao pura, testavel isolada).
 *
 * Um lembrete so serve se encontra o usuario: o Painel mostra as pendencias
 * **abertas** de todos os projetos, e o projeto do cronometro ativo vem primeiro,
 * porque e o contexto de quem esta trabalhando agora.
 */

import type { Project, ProjectTodo } from "@/types/domain";

export interface TodoGroup {
  project: Project;
  todos: ProjectTodo[];
}

export function openTodosByProject(
  todos: ProjectTodo[],
  projects: Project[],
  activeProjectId: string | null,
): TodoGroup[] {
  const open = todos.filter((t) => !t.done);

  const groups: TodoGroup[] = [];
  for (const project of projects) {
    const projectTodos = open.filter((t) => t.projectId === project.id);
    // Pendencias de projetos fora da lista (arquivados) ficam de fora.
    if (projectTodos.length > 0) groups.push({ project, todos: projectTodos });
  }

  return groups.sort((a, b) => {
    if (a.project.id === activeProjectId) return -1;
    if (b.project.id === activeProjectId) return 1;
    return a.project.name.localeCompare(b.project.name, "pt-BR");
  });
}
```

- [ ] **Step 4: Rodar o teste e confirmar que passa**

Run: `npx vitest run src/lib/todos.test.ts`
Expected: PASS — 6 testes.

- [ ] **Step 5: Commit**

```bash
git add src/lib/todos.ts src/lib/todos.test.ts
git commit -m "feat(notes): regra de exibicao das pendencias no painel"
```

---

### Task 5: Modal de anotacoes em Projetos

**Files:**
- Create: `src/features/projects/ProjectNotesModal.tsx`
- Modify: `src/features/projects/ProjectsPage.tsx` (icone na linha + montagem do modal)

**Interfaces:**
- Consumes: `useNotesStore` e `useCatalogStore.updateProjectNotes` (Task 3); `Modal`, `Button`, `Input`, `Checkbox` (`@/components/ui/*`).
- Produces:
  ```ts
  interface ProjectNotesModalProps {
    project: Project | null;   // null = fechado
    onClose: () => void;
  }
  export function ProjectNotesModal(props: ProjectNotesModalProps): JSX.Element | null;
  ```

Sem teste automatizado aqui: o comportamento testavel (a regra de exibicao) ja esta coberto na Task 4, e o resto e ligacao de UI com o store. A verificacao e a Task 7.

- [ ] **Step 1: Criar o modal**

Criar `src/features/projects/ProjectNotesModal.tsx`:

```tsx
import { useEffect, useState, type FormEvent } from "react";
import { Plus, Trash2 } from "lucide-react";
import type { Project } from "@/types/domain";
import { useNotesStore } from "@/stores/notesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { Input } from "@/components/ui/Field";

interface ProjectNotesModalProps {
  /** Projeto em edicao; `null` mantem o modal fechado. */
  project: Project | null;
  onClose: () => void;
}

/**
 * Anotacoes (texto livre) e pendencias (checklist) de um projeto.
 *
 * As anotacoes sao um campo do proprio projeto e salvam ao sair do campo; as
 * pendencias vivem no notesStore. Nenhuma pendencia dispara notificacao: elas
 * apenas ficam visiveis (aqui e no Painel).
 */
export function ProjectNotesModal({ project, onClose }: ProjectNotesModalProps) {
  const todos = useNotesStore((s) => s.todos);
  const createTodo = useNotesStore((s) => s.createTodo);
  const setTodoDone = useNotesStore((s) => s.setTodoDone);
  const deleteTodo = useNotesStore((s) => s.deleteTodo);
  const updateProjectNotes = useCatalogStore((s) => s.updateProjectNotes);

  const [notes, setNotes] = useState("");
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Recarrega o rascunho ao trocar de projeto.
  useEffect(() => {
    setNotes(project?.notes ?? "");
    setText("");
    setError(null);
  }, [project]);

  if (!project) return null;

  const mine = todos.filter((t) => t.projectId === project.id);
  const open = mine.filter((t) => !t.done);
  const done = mine.filter((t) => t.done);

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(typeof err === "string" ? err : "Operacao falhou.");
    }
  }

  async function saveNotes() {
    if (!project || notes === (project.notes ?? "")) return;
    await run(() => updateProjectNotes(project.id, notes));
  }

  async function addTodo(e: FormEvent) {
    e.preventDefault();
    if (!project || !text.trim()) return;
    await run(() => createTodo(project.id, text));
    setText("");
  }

  return (
    <Modal
      open
      title={`Anotacoes — ${project.name}`}
      onClose={onClose}
      footer={
        <Button variant="primary" onClick={onClose}>
          Fechar
        </Button>
      }
    >
      <label
        htmlFor="proj-notes"
        className="text-2xs uppercase tracking-wide text-text-subtle"
      >
        Anotacoes
      </label>
      <textarea
        id="proj-notes"
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        onBlur={() => void saveNotes()}
        rows={4}
        placeholder="Contexto, recados do cliente, o que esta pendente de terceiros…"
        className="mt-1.5 w-full rounded border border-border bg-surface-raised px-3 py-2 text-sm text-text placeholder:text-text-subtle focus:border-accent focus:outline-none"
      />

      <p className="mt-6 text-2xs uppercase tracking-wide text-text-subtle">
        Pendencias
      </p>

      <form onSubmit={(e) => void addTodo(e)} className="mt-1.5 flex gap-2">
        <Input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="O que precisa ser feito?"
        />
        <Button
          type="submit"
          variant="secondary"
          disabled={!text.trim()}
          icon={<Plus size={16} strokeWidth={2} />}
        >
          Adicionar
        </Button>
      </form>

      {open.length === 0 && done.length === 0 ? (
        <p className="mt-4 text-sm text-text-muted">
          Nenhuma pendencia neste projeto.
        </p>
      ) : (
        <ul className="mt-3 divide-y divide-border">
          {open.map((todo) => (
            <li key={todo.id} className="flex items-center gap-3 py-2">
              <Checkbox
                label=""
                ariaLabel={`Concluir ${todo.text}`}
                checked={false}
                onChange={() => void run(() => setTodoDone(todo.id, true))}
              />
              <span className="flex-1 text-sm text-text">{todo.text}</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => void run(() => deleteTodo(todo.id))}
                aria-label={`Excluir ${todo.text}`}
                icon={<Trash2 size={15} strokeWidth={1.75} />}
              />
            </li>
          ))}
        </ul>
      )}

      {done.length > 0 && (
        <details className="mt-4">
          <summary className="cursor-pointer text-xs text-text-muted">
            Concluidas ({done.length})
          </summary>
          <ul className="mt-2 divide-y divide-border">
            {done.map((todo) => (
              <li key={todo.id} className="flex items-center gap-3 py-2">
                <Checkbox
                  label=""
                  ariaLabel={`Reabrir ${todo.text}`}
                  checked
                  onChange={() => void run(() => setTodoDone(todo.id, false))}
                />
                <span className="flex-1 text-sm text-text-subtle line-through">
                  {todo.text}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void run(() => deleteTodo(todo.id))}
                  aria-label={`Excluir ${todo.text}`}
                  icon={<Trash2 size={15} strokeWidth={1.75} />}
                />
              </li>
            ))}
          </ul>
        </details>
      )}

      {error && <p className="mt-3 text-sm text-danger">{error}</p>}
    </Modal>
  );
}
```

**Contrato do `Checkbox`** (`src/components/ui/Checkbox.tsx`), ja refletido no codigo acima — nao e o contrato nativo do HTML:

```ts
{ label: string; checked: boolean; onChange: (checked: boolean) => void;
  disabled?: boolean; ariaLabel?: string }
```

Ele **renderiza o `label`** ao lado da caixinha, e `onChange` recebe um **booleano** (nao um evento). Com `label=""` o texto nao e renderizado e o nome acessivel vem de `ariaLabel` — e por isso que aqui usamos `label=""` + um `<span>` proprio (precisamos do `line-through` nas concluidas e do `flex-1`), enquanto no Painel (Task 6) usamos `label={todo.text}` direto.

- [ ] **Step 2: Ligar na `ProjectsPage`**

Em `src/features/projects/ProjectsPage.tsx`:

2a. No import do `lucide-react` (:2), acrescentar `StickyNote`:

```tsx
import { Archive, CheckCircle2, Pencil, Plus, Search, StickyNote, Users } from "lucide-react";
```

2b. Acrescentar o import do modal e do store, junto aos outros imports:

```tsx
import { ProjectNotesModal } from "./ProjectNotesModal";
import { useNotesStore } from "@/stores/notesStore";
```

2c. Junto aos outros `useState` (:31-34):

```tsx
  const [notesFor, setNotesFor] = useState<Project | null>(null);
```

2d. Carregar as pendencias junto com o catalogo. Substituir o `useEffect` existente (:36-38) por:

```tsx
  const loadTodos = useNotesStore((s) => s.load);
  const todosLoaded = useNotesStore((s) => s.loaded);

  useEffect(() => {
    if (!loaded) void loadAll();
    if (!todosLoaded) void loadTodos();
  }, [loaded, loadAll, todosLoaded, loadTodos]);
```

2e. Na coluna de acoes da linha, **antes** do botao de editar (:182-188):

```tsx
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setNotesFor(project)}
                        aria-label={`Anotacoes de ${project.name}`}
                        icon={<StickyNote size={15} strokeWidth={1.75} />}
                      />
```

2f. Junto aos outros modais no fim do componente (:214-219):

```tsx
      <ProjectNotesModal
        project={notesFor}
        onClose={() => setNotesFor(null)}
      />
```

- [ ] **Step 3: Verificar**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: sem erros; 0 warnings; todos os testes passando.

- [ ] **Step 4: Commit**

```bash
git add src/features/projects/ProjectNotesModal.tsx src/features/projects/ProjectsPage.tsx
git commit -m "feat(projects): modal de anotacoes e pendencias por projeto"
```

---

### Task 6: Painel de pendencias no Painel

**Files:**
- Create: `src/features/dashboard/TodosPanel.tsx`
- Modify: `src/features/dashboard/DashboardPage.tsx` (grade inferior)

**Interfaces:**
- Consumes: `openTodosByProject` e `TodoGroup` (Task 4); `useNotesStore` (Task 3); `useTimerStore`, `useCatalogStore`.
- Produces: `export function TodosPanel(): JSX.Element;`

- [ ] **Step 1: Criar o painel**

Criar `src/features/dashboard/TodosPanel.tsx`:

```tsx
import { useEffect } from "react";
import { Link } from "react-router-dom";
import { StickyNote } from "lucide-react";
import { useNotesStore } from "@/stores/notesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useTimerStore } from "@/stores/timerStore";
import { openTodosByProject } from "@/lib/todos";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Checkbox } from "@/components/ui/Checkbox";
import { EmptyState } from "@/components/ui/EmptyState";
import { ROUTES } from "@/app/routes";

/**
 * Pendencias abertas de todos os projetos. Um lembrete so serve se encontra o
 * usuario — por isso ele mora no Painel, e nao escondido dentro do projeto.
 * O projeto do cronometro ativo vem primeiro e destacado.
 */
export function TodosPanel() {
  const todos = useNotesStore((s) => s.todos);
  const loaded = useNotesStore((s) => s.loaded);
  const load = useNotesStore((s) => s.load);
  const setTodoDone = useNotesStore((s) => s.setTodoDone);
  const projects = useCatalogStore((s) => s.projects);
  const activeTimer = useTimerStore((s) => s.activeTimer);

  useEffect(() => {
    if (!loaded) void load();
  }, [loaded, load]);

  const activeProjectId = activeTimer?.projectId ?? null;
  const groups = openTodosByProject(todos, projects, activeProjectId);

  return (
    <Panel>
      <PanelHeader title="Pendencias" />
      {groups.length === 0 ? (
        <div className="p-4">
          <EmptyState
            title="Nenhuma pendencia"
            description="Anote lembretes de cada projeto na tela de Projetos."
            action={
              <Link
                to={ROUTES.projects}
                className="text-sm text-accent hover:underline"
              >
                Ir para Projetos
              </Link>
            }
          />
        </div>
      ) : (
        <div className="divide-y divide-border">
          {groups.map(({ project, todos: items }) => {
            const isActive = project.id === activeProjectId;
            return (
              <div
                key={project.id}
                className={isActive ? "border-l-2 border-l-accent px-4 py-3" : "px-4 py-3"}
              >
                <div className="flex items-center gap-1.5">
                  <p
                    className={
                      isActive
                        ? "text-sm font-medium text-text"
                        : "text-sm text-text-muted"
                    }
                  >
                    {project.code ? `${project.code} · ${project.name}` : project.name}
                  </p>
                  {project.notes && (
                    <StickyNote
                      size={13}
                      strokeWidth={1.75}
                      className="shrink-0 text-text-subtle"
                      aria-label="Este projeto tem anotacoes"
                    />
                  )}
                </div>
                <ul className="mt-1.5 space-y-1.5">
                  {items.map((todo) => (
                    <li key={todo.id}>
                      <Checkbox
                        label={todo.text}
                        checked={false}
                        onChange={() => void setTodoDone(todo.id, true)}
                      />
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
      )}
    </Panel>
  );
}
```

- [ ] **Step 2: Montar no Painel**

Em `src/features/dashboard/DashboardPage.tsx`:

2a. Acrescentar o import:

```tsx
import { TodosPanel } from "./TodosPanel";
```

2b. A grade inferior (:76) hoje tem duas colunas ("Sessoes recentes" e "Linha do tempo detectada"). Trocar por tres, e colocar o painel novo como **primeiro** — pendencias sao o que o usuario precisa ver ao abrir o app:

```tsx
      <div className="mt-4 grid gap-4 lg:grid-cols-3">
        <TodosPanel />
```

(o `<Panel>` de "Sessoes recentes" e o de "Linha do tempo detectada" continuam como estao, logo abaixo, dentro da mesma `<div>`.)

- [ ] **Step 3: Verificar**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: sem erros; 0 warnings; todos os testes passando.

- [ ] **Step 4: Commit**

```bash
git add src/features/dashboard/TodosPanel.tsx src/features/dashboard/DashboardPage.tsx
git commit -m "feat(dashboard): painel de pendencias abertas por projeto"
```

---

### Task 7: Verificacao no app real

**Files:** nenhum (verificacao manual) + `CHANGELOG.md`.

**PRE-REQUISITO — LEIA ANTES DE RODAR:** o app instalado (`%LOCALAPPDATA%\CronoCAD\cronocad.exe`) usa **o mesmo banco SQLite** que o `tauri:dev`. Duas instancias escrevendo no mesmo banco corrompem dados. O app instalado precisa estar **fechado** (fechar a janela apenas o manda para a bandeja — e preciso "Sair" pelo icone da bandeja), e o cronometro **pausado ou encerrado**, nunca descartado.

**Alem disso:** a migration 0005 sera aplicada ao banco **real** do usuario na primeira execucao. Ela e aditiva (uma coluna e uma tabela novas), mas e uma escrita definitiva — nao ha `Down`.

- [ ] **Step 1: Confirmar que o app instalado esta fechado**

Run: `powershell -NoProfile -Command "Get-Process cronocad -ErrorAction SilentlyContinue | Select-Object Id, Path"`
Expected: **sem saida**. Se algo aparecer, **pare** e peca ao usuario para sair pelo icone da bandeja.

- [ ] **Step 2: Subir o app**

Run: `npm run tauri:dev`
Expected: compila e abre. A migration 0005 e aplicada em silencio no startup.

- [ ] **Step 3: Percorrer o checklist**

1. Abrir **Projetos**. Cada linha tem um icone de bloquinho. Clicar nele abre "Anotacoes — {projeto}".
2. Escrever um texto nas anotacoes e clicar fora do campo (blur). Fechar o modal, reabrir: **o texto continua la**.
3. Editar o projeto pelo lapis (mudar o nome ou o valor/hora) e salvar. Reabrir as anotacoes: **o texto continua la** (a edicao do projeto nao apaga as anotacoes).
4. Adicionar duas pendencias. Elas aparecem na lista aberta.
5. Ir ao **Painel**: o card "Pendencias" mostra as duas, sob o nome do projeto, com o icone de bloquinho ao lado (porque o projeto tem anotacoes).
6. Marcar uma pendencia no Painel: ela **sai da lista** na hora.
7. Voltar ao modal do projeto: a marcada esta em "Concluidas (1)", riscada. Desmarcar reabre; o lixeirinha exclui de vez.
8. Iniciar o cronometro em um projeto que tenha pendencias: no Painel, esse projeto **sobe para o topo** e ganha a barra de acento a esquerda.
9. Com nenhuma pendencia aberta, o card mostra "Nenhuma pendencia" e o link para Projetos.

- [ ] **Step 4: Atualizar o CHANGELOG e commitar**

Acrescentar ao `CHANGELOG.md`, dentro de `## [Nao lancado]`, seguindo o formato do arquivo:

```markdown
### Adicionado — Anotacoes e pendencias por projeto
- Cada projeto ganha um bloco de **anotacoes** livres (`projects.notes`) e uma
  lista de **pendencias** (`project_todos`), editaveis num modal na tela de
  Projetos (icone de bloquinho na linha).
- O Painel ganha o card **Pendencias**: mostra as pendencias **abertas** de todos
  os projetos, agrupadas, com o projeto do cronometro ativo em primeiro e
  destacado. Marcar a caixinha conclui a pendencia ali mesmo.
- Sem notificacao, sem data e sem agendamento: um lembrete aqui apenas fica
  visivel. Pendencias usam hard delete — nao sao registro de tempo.
- Migration `0005_project_notes.sql`.
```

```bash
git add CHANGELOG.md
git commit -m "docs: registrar anotacoes por projeto no changelog"
git push -u origin feat/anotacoes-projeto
```

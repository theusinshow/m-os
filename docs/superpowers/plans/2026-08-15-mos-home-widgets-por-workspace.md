# Widgets da Home por Workspace, Etapa 2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cada Workspace escolhe quais dos sete widgets da Home aparecem, e a escolha fica guardada no banco.

**Architecture:** Uma tabela nova onde **a linha significa oculto** — ausência de linha é o padrão visível. O core guarda e devolve pares `(workspace_id, widget_id)` sem conhecer o catálogo de widgets, que vive no front. A conversão entre "visível" (linguagem da interface) e "oculto" (linguagem da tabela) acontece num lugar só: o comando Tauri. A Home filtra em memória, então trocar de Workspace não vai ao backend.

**Tech Stack:** Rust (rusqlite, SQLite STRICT), Tauri 2, React 19 + TypeScript. Nenhuma dependência nova.

**Spec:** `docs/superpowers/specs/2026-08-15-mos-home-widgets-por-workspace-design.md`

## Global Constraints

- **Migration publicada não se edita.** `0008_workspace_widgets.sql` é arquivo novo. Nenhuma das sete existentes muda.
- **O `id` de um widget é permanente.** Ele vai para o banco. Renomear `inbox_pulse` depois apaga em silêncio a escolha de quem o tinha ocultado. O rótulo exibido pode mudar à vontade; o id, não.
- **Mensagem de erro no core é português sem acento**, como todo o resto: `"O nome do Workspace nao pode estar vazio."` (`work.rs:134`).
- **Não anotar tipo de retorno de componente React.** Nenhum arquivo de `apps/desktop/src/` usa `JSX.Element`, e no React 19 o namespace global `JSX` não é exposto. Anotar quebra o `tsc`.
- **Não existe teste automatizado de front.** `package.json` define apenas `"build": "tsc && vite build"`. Não instalar dependência de teste. A metade Rust, essa sim, é escrita com teste antes.
- **Build do Rust exige ambiente portable.** Antes de `cargo test` ou `npm run tauri dev`, no PowerShell:
  ```powershell
  $env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
  $env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
  ```
  Se `tauri dev` reclamar de porta 1420 ocupada, matar o processo `node` que a segura — matar o wrapper do npm não basta.
- **Sem dependência nova** em nenhum dos dois lados.

---

### Task 1: Migration 0008 e o core

Tudo abaixo do Tauri: tabela, tipo, validação, repositório e serviço. Esta é a única task com teste automatizado, e por isso é escrita com teste antes.

**Files:**
- Create: `crates/mos-storage-sqlite/migrations/0008_workspace_widgets.sql`
- Modify: `crates/mos-storage-sqlite/src/lib.rs:18` (`SCHEMA_VERSION`), `:25` (constante da migration), `:186` (cadeia), `:338`, `:367`, `:449`, `:575` (asserts de `schema_version`)
- Modify: `crates/mos-core/src/work.rs` (struct `HiddenWidget` e `validate_widget_id`)
- Modify: `crates/mos-core/src/lib.rs` (exports)
- Modify: `crates/mos-core/src/ports.rs:76` (dois métodos no trait `WorkRepository`)
- Modify: `crates/mos-storage-sqlite/src/work_repository.rs:374` (implementação, após `set_app_workspace`)
- Modify: `crates/mos-core/src/service.rs:542` (wrapper em `WorkService`, após `set_app_workspace`)
- Test: `crates/mos-storage-sqlite/src/work_repository.rs`, no `mod tests` que começa em `:1021`

**Interfaces:**
- Consumes: `SqliteStorage`, `NewWorkspace::create(name, description)`, `WorkspaceId`, `CoreError`, `ErrorCode` — todos já existentes.
- Produces:
  - `pub struct HiddenWidget { pub workspace_id: WorkspaceId, pub widget_id: String }`, serializado como `{ workspaceId, widgetId }`;
  - `WorkRepository::set_widget_hidden(&self, workspace_id: WorkspaceId, widget_id: &str, hidden: bool) -> Result<(), CoreError>`;
  - `WorkRepository::hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError>`;
  - `WorkService::set_widget_hidden(&self, workspace_id: &str, widget_id: &str, hidden: bool) -> Result<(), CoreError>`;
  - `WorkService::hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError>`.

  As Tasks 2 a 5 consomem estes nomes exatos.

- [ ] **Step 1: Escrever os testes que falham**

Em `crates/mos-storage-sqlite/src/work_repository.rs`, dentro do `mod tests`, no fim do arquivo:

```rust
    #[test]
    fn hidden_widget_is_per_workspace_and_repeating_the_call_is_idempotent() {
        let (_directory, storage) = storage();
        let engineering = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();
        let finance = storage
            .create_workspace(NewWorkspace::create("Finance", "").unwrap())
            .unwrap();

        storage
            .set_widget_hidden(engineering.id, "inbox_pulse", true)
            .unwrap();
        storage
            .set_widget_hidden(engineering.id, "inbox_pulse", true)
            .unwrap();

        let hidden = storage.hidden_widgets().unwrap();
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].workspace_id, engineering.id);
        assert_eq!(hidden[0].widget_id, "inbox_pulse");

        storage
            .set_widget_hidden(engineering.id, "inbox_pulse", false)
            .unwrap();
        storage
            .set_widget_hidden(engineering.id, "inbox_pulse", false)
            .unwrap();
        assert!(storage.hidden_widgets().unwrap().is_empty());

        storage
            .set_widget_hidden(finance.id, "system_health", true)
            .unwrap();
        let hidden = storage.hidden_widgets().unwrap();
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].workspace_id, finance.id);
    }

    #[test]
    fn widget_id_outside_the_allowed_shape_is_refused() {
        let (_directory, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();

        for invalid in ["", "  ", "Inbox Pulse", "inbox-pulse", "1inbox"] {
            assert!(
                storage.set_widget_hidden(workspace.id, invalid, true).is_err(),
                "aceitou o id invalido {invalid:?}"
            );
        }
        assert!(storage.hidden_widgets().unwrap().is_empty());
    }

    /// Nao existe delete de Workspace no produto — arquivar e o caminho. O DELETE
    /// cru aqui prova que a FK esta ativa: se `foreign_keys=ON` se perder em
    /// `configure_connection` (lib.rs:103), a linha sobrevive e este teste falha.
    #[test]
    fn deleting_the_workspace_takes_its_hidden_widgets() {
        let (_directory, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();
        storage
            .set_widget_hidden(workspace.id, "system_health", true)
            .unwrap();

        storage
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM workspaces WHERE id = ?1",
                params![workspace.id.to_string()],
            )
            .unwrap();

        assert!(storage.hidden_widgets().unwrap().is_empty());
    }
```

`params!` e `NewWorkspace` já estão no escopo do arquivo (`work_repository.rs:1` a `:8` e `:1022`). `storage.connection` é acessível: `work_repository` é módulo filho da raiz do crate, onde `SqliteStorage` é definido.

- [ ] **Step 2: Rodar e confirmar que falha**

```powershell
$env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
$env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
cargo test -p mos-storage-sqlite
```

Esperado: erro de compilação, `no method named 'set_widget_hidden' found for struct 'SqliteStorage'`. Falha por ausência do método, não por asserção — é o esperado nesta etapa.

- [ ] **Step 3: Criar a migration**

Arquivo novo `crates/mos-storage-sqlite/migrations/0008_workspace_widgets.sql`:

```sql
-- A escolha de widgets da Home, por Workspace.
--
-- A LINHA SIGNIFICA OCULTO. Ausencia de linha = o widget aparece. A inversao e
-- deliberada e tem tres consequencias, todas desejadas:
--   1. Workspace novo mostra tudo sem nenhuma escrita;
--   2. widget criado depois nasce visivel em todos os Workspaces — guardar o
--      visivel faria cada recurso novo nascer invisivel para quem ja usa o app;
--   3. a tabela fica vazia para quem nunca configurou nada, que e a maioria.
--
-- widget_id e string opaca: o core nao conhece o catalogo, que vive no front em
-- HOME_WIDGETS. O CHECK garante formato, nao vocabulario — enum aqui faria de
-- cada widget novo uma migration. Linha orfa de widget extinto e inofensiva: o
-- front ignora id que nao esta no catalogo.
--
-- Sem indice: a PRIMARY KEY (workspace_id, widget_id) ja serve as buscas por
-- workspace, e a tabela tem no maximo sete linhas por Workspace.

BEGIN IMMEDIATE;

CREATE TABLE workspace_hidden_widgets (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    widget_id TEXT NOT NULL CHECK (widget_id GLOB '[a-z][a-z0-9_]*'),
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, widget_id)
) STRICT;

PRAGMA user_version = 8;

COMMIT;
```

- [ ] **Step 4: Ligar a migration**

Em `crates/mos-storage-sqlite/src/lib.rs`:

1. Linha 18, trocar `const SCHEMA_VERSION: u32 = 7;` por `const SCHEMA_VERSION: u32 = 8;`
2. Após a linha 25 (`MIGRATION_007`), acrescentar:

```rust
const MIGRATION_008: &str = include_str!("../migrations/0008_workspace_widgets.sql");
```

3. Na função `migrate`, depois do bloco `if current <= 6 { ... }` que termina na linha 190, acrescentar:

```rust
    if current <= 7 {
        connection
            .execute_batch(MIGRATION_008)
            .map_err(map_sql_error)?;
    }
```

4. Quatro asserts de versão passam de `7` para `8`: linhas `338`, `367`, `449` e `575`. São testes de migration existentes que afirmam a versão final do schema; deixá-los em 7 faria a suíte falhar por uma razão que não é a desta task.

- [ ] **Step 5: Criar o tipo e a validação no core**

Em `crates/mos-core/src/work.rs`, após o `struct Workspace` (termina por volta de `:120`):

```rust
/// Uma linha desta lista significa OCULTO. Ver a migration 0008.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenWidget {
    pub workspace_id: WorkspaceId,
    pub widget_id: String,
}
```

E junto de `required` (`work.rs:232`):

```rust
/// Espelha o CHECK da migration 0008: minuscula inicial, depois minuscula,
/// digito ou `_`. O core valida forma, nao vocabulario — quem conhece o
/// catalogo de widgets e o front.
pub fn validate_widget_id(value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 40
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_');
    if valid {
        Ok(value.to_owned())
    } else {
        Err(CoreError::new(
            ErrorCode::InvalidInput,
            "ID de widget invalido.",
            false,
        ))
    }
}
```

Em `crates/mos-core/src/lib.rs`, no bloco `pub use work::{...}`, acrescentar `validate_widget_id` e `HiddenWidget` em ordem alfabética — a lista passa a ser:

```rust
pub use work::{
    validate_widget_id, HiddenWidget, NewProject, NewTask, NewWorkspace, Project, ProjectId,
    SearchItem, Task, TaskId, TaskState, Workspace, WorkspaceId,
};
```

- [ ] **Step 6: Declarar os métodos no trait**

Em `crates/mos-core/src/ports.rs`, dentro de `pub trait WorkRepository`, logo após `set_app_workspace` (termina em `:81`):

```rust
    fn set_widget_hidden(
        &self,
        workspace_id: WorkspaceId,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError>;
    fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError>;
```

O `use crate::{...}` do topo de `ports.rs` precisa passar a incluir `HiddenWidget`.

- [ ] **Step 7: Implementar no repositório**

Em `crates/mos-storage-sqlite/src/work_repository.rs`, dentro de `impl WorkRepository for SqliteStorage`, após `set_app_workspace` (termina em `:374`):

```rust
    fn set_widget_hidden(
        &self,
        workspace_id: WorkspaceId,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError> {
        let widget_id = validate_widget_id(widget_id)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        if hidden {
            let now = format_time(OffsetDateTime::now_utc())?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO workspace_hidden_widgets (workspace_id, widget_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![workspace_id.to_string(), widget_id, now],
                )
                .map_err(map_sql_error)?;
        } else {
            connection
                .execute(
                    "DELETE FROM workspace_hidden_widgets WHERE workspace_id = ?1 AND widget_id = ?2",
                    params![workspace_id.to_string(), widget_id],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    /// Devolve todos os pares de uma vez. No teto sao sete linhas por Workspace,
    /// e uma chamada so deixa a troca de contexto na Home filtrar em memoria em
    /// vez de ir ao core a cada clique.
    fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_id, widget_id FROM workspace_hidden_widgets
                 ORDER BY workspace_id, widget_id",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql_error)?;
        let mut hidden = Vec::new();
        for row in rows {
            let (workspace_id, widget_id) = row.map_err(map_sql_error)?;
            hidden.push(HiddenWidget {
                workspace_id: WorkspaceId::parse(&workspace_id)?,
                widget_id,
            });
        }
        Ok(hidden)
    }
```

O `use mos_core::{...}` do topo do arquivo precisa incluir `validate_widget_id` e `HiddenWidget`.

- [ ] **Step 8: Rodar os testes**

```powershell
cargo test -p mos-storage-sqlite
```

Esperado: tudo passa, incluindo os três testes novos e os quatro de migration com a versão 8.

- [ ] **Step 9: Expor no serviço**

Em `crates/mos-core/src/service.rs`, dentro de `impl WorkService`, após `set_app_workspace` (termina em `:542`):

```rust
    pub fn set_widget_hidden(
        &self,
        workspace_id: &str,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError> {
        self.repository
            .set_widget_hidden(WorkspaceId::parse(workspace_id)?, widget_id, hidden)
    }

    pub fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError> {
        self.repository.hidden_widgets()
    }
```

O `use crate::{...}` do topo de `service.rs` precisa incluir `HiddenWidget`.

```powershell
cargo test
```

Esperado: a suíte inteira passa.

- [ ] **Step 10: Commit**

```bash
git add crates/mos-storage-sqlite/migrations/0008_workspace_widgets.sql crates/mos-storage-sqlite/src/lib.rs crates/mos-storage-sqlite/src/work_repository.rs crates/mos-core/src/work.rs crates/mos-core/src/lib.rs crates/mos-core/src/ports.rs crates/mos-core/src/service.rs
git commit -m "feat(core): escolha de widgets da Home por Workspace no banco"
```

---

### Task 2: Comandos Tauri e API do front

A ponte. Aqui acontece a única inversão de sinal do sistema: a interface fala `visible`, a tabela guarda o oculto.

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs:7-14` (import), `:419` (comandos novos, após `set_app_workspace`), `:1081` (registro no `invoke_handler`)
- Modify: `apps/desktop/src/types.ts` (tipo `HiddenWidget`)
- Modify: `apps/desktop/src/api.ts:153` (dois métodos, após `setAppWorkspace`)
- Test: nenhum — a camada Tauri não tem teste no projeto

**Interfaces:**
- Consumes: `WorkService::set_widget_hidden`, `WorkService::hidden_widgets` e `HiddenWidget` da Task 1. `notify_data_changed(&AppHandle, &str)` (`lib.rs:792`) e `schedule_snapshot(&DataService, &Arc<Mutex<String>>, &AppHandle)` (`lib.rs:796`).
- Produces: `api.hiddenWidgets(): Promise<HiddenWidget[]>` e `api.setWorkspaceWidget(widgetId, workspaceId, visible): Promise<void>`, mais o tipo `HiddenWidget = { workspaceId: string; widgetId: string }`. Tasks 3 e 4 consomem os dois.

> **Correcao aplicada durante a execucao.** Este passo carregava tambem o dado no componente raiz. Nao compila: `noUnusedLocals` recusa um estado que ninguem le, e o primeiro leitor so aparece na Task 3. A carga foi movida para la, junto do primeiro consumidor.

- [ ] **Step 1: Escrever os comandos**

Em `apps/desktop/src-tauri/src/lib.rs`, após `set_app_workspace` (termina em `:419`):

```rust
#[tauri::command]
fn list_hidden_widgets(state: tauri::State<'_, AppState>) -> Result<Vec<HiddenWidget>, CoreError> {
    state.work.hidden_widgets()
}

/// A interface fala em visivel; a tabela guarda o oculto. A inversao acontece
/// aqui, num lugar so — espalha-la pelos componentes seria garantir que um dia
/// dois deles discordem sobre o que a ausencia de linha significa.
#[tauri::command]
fn set_workspace_widget(
    workspace_id: &str,
    widget_id: &str,
    visible: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state
        .work
        .set_widget_hidden(workspace_id, widget_id, !visible)?;
    notify_data_changed(&app, "workspace-widget");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}
```

No `use mos_core::{...}` do topo (linhas 7 a 14), acrescentar `HiddenWidget` em ordem alfabética — entre `FunctionDefinition` e `MemoryService`.

- [ ] **Step 2: Registrar no invoke_handler**

Em `apps/desktop/src-tauri/src/lib.rs`, na lista do `invoke_handler`, logo após `set_app_workspace,` (linha 1081):

```rust
            set_workspace_widget,
            list_hidden_widgets,
```

Esquecer este passo compila sem erro e falha só em runtime, com o front recebendo "command not found".

- [ ] **Step 3: Compilar o Rust**

```powershell
cargo build
```

Esperado: compila sem erro.

- [ ] **Step 4: Declarar o tipo no front**

Em `apps/desktop/src/types.ts`, após o `type Workspace` (termina em `:33`):

```ts
/** Uma entrada significa OCULTO — ausência é o padrão visível. Ver migration 0008. */
export type HiddenWidget = {
  workspaceId: string;
  widgetId: string;
};
```

- [ ] **Step 5: Acrescentar os métodos da API**

Em `apps/desktop/src/api.ts`, após `setAppWorkspace` (termina em `:155`):

```ts
  hiddenWidgets() {
    return invoke<HiddenWidget[]>("list_hidden_widgets");
  },
  setWorkspaceWidget(widgetId: string, workspaceId: string, visible: boolean) {
    return invoke<void>("set_workspace_widget", { widgetId, workspaceId, visible });
  },
```

`HiddenWidget` entra no `import type { ... } from "./types"` da linha 4, em ordem alfabética — entre `FunctionDefinition` e `Project`.

- [ ] **Step 6: Verificar o build do front**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro de TypeScript.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs apps/desktop/src/types.ts apps/desktop/src/api.ts
git commit -m "feat(api): comandos de visibilidade de widget por Workspace"
```

---

### Task 3: Painel WIDGETS na página Workspaces

O lugar onde se configura. Reusa o `.relation-row` e o `.workspace-grid` que já existem — nenhum CSS novo.

**Files:**
- Modify: `apps/desktop/src/App.tsx:1293` (estado no raiz), `:1313` (o `refresh`)
- Modify: `apps/desktop/src/App.tsx:116` (catálogo `HOME_WIDGETS`, antes de `ScopedEmptyState`)
- Modify: `apps/desktop/src/App.tsx:457` (props de `WorkspacesPage`), `:483` (`refreshLinks`), `:509` (função `toggleWidget`), `:512` (o `.workspace-grid`)
- Modify: `apps/desktop/src/App.tsx:1409` (chamada de `WorkspacesPage`)
- Test: nenhum

**Interfaces:**
- Consumes: `api.setWorkspaceWidget` e o tipo `HiddenWidget` da Task 2. `Panel({ label, count, action, rule, children, className })` (`App.tsx:76`). `appError` e `EmptyState`, já usados no arquivo.
- Produces: `const HOME_WIDGETS: { id: string; label: string }[]` e a variável `hiddenWidgets: HiddenWidget[]` no componente raiz, carregada pelo `refresh` — a Task 4 consome as duas, sem alterá-las.

- [ ] **Step 1: Carregar o dado no componente raiz**

O lugar dele é o mesmo de todos os outros: o `refresh` que já carrega o app inteiro. Vem nesta task, e não na anterior, porque `noUnusedLocals` recusa um estado que ninguém lê — a carga e o primeiro leitor têm que viajar no mesmo commit.

Em `apps/desktop/src/App.tsx:1293`, junto dos outros estados do componente raiz:

```tsx
  const [hiddenWidgets, setHiddenWidgets] = useState<HiddenWidget[]>([]);
```

No `refresh` (`:1313`), acrescentar `api.hiddenWidgets()` **no fim** do array do `Promise.all` e `nextHiddenWidgets` no fim da desestruturação, mantendo a ordem — a lista é posicional, e mexer na ordem de um lado só embaralha tudo em silêncio. Na linha seguinte (`:1314`), acrescentar `setHiddenWidgets(nextHiddenWidgets);` ao fim da sequência de setters.

`HiddenWidget` entra no `import type { ... } from "./types"` do topo de `App.tsx`.

- [ ] **Step 2: Criar o catálogo**

Em `apps/desktop/src/App.tsx`, antes de `function ScopedEmptyState` (`:117`):

```tsx
/* Fonte de verdade unica dos ids de widget. Os ids VAO PARA O BANCO: renomear
   um deles apaga em silencio a escolha de quem tinha ocultado o widget, porque
   a linha guardada deixa de casar com qualquer widget do catalogo. O rotulo
   pode mudar a vontade; o id, nunca. */
const HOME_WIDGETS: { id: string; label: string }[] = [
  { id: "now", label: "EM ANDAMENTO" },
  { id: "recent", label: "RECENTES" },
  { id: "projects", label: "PROJECTS" },
  { id: "apps", label: "APPS" },
  { id: "inbox_pulse", label: "INBOX" },
  { id: "quick_actions", label: "AÇÕES" },
  { id: "system_health", label: "SISTEMA" },
];
```

- [ ] **Step 3: Receber os ocultos em WorkspacesPage**

Na assinatura em `App.tsx:457`, acrescentar `hiddenWidgets` à desestruturação e ao tipo. A lista passa a ser:

```tsx
function WorkspacesPage({ workspaces, projects, apps, hiddenWidgets, initialWorkspaceId, refresh, openProject, openApp, intent }: { workspaces: Workspace[]; projects: Project[]; apps: RegisteredApp[]; hiddenWidgets: HiddenWidget[]; initialWorkspaceId: string; refresh: () => Promise<void>; openProject: (project: Project) => void; openApp: (app: RegisteredApp) => void; intent?: FunctionIntent }) {
```

Na chamada em `App.tsx:1409`, acrescentar `hiddenWidgets={hiddenWidgets}` logo após `apps={apps}`. A variável vem do Step 1 desta mesma task.

- [ ] **Step 4: Derivar o conjunto oculto e escrever o toggle**

Junto de `linkedAppIds` (`App.tsx:482`), acrescentar:

```tsx
  const hiddenWidgetIds = new Set(hiddenWidgets.filter((entry) => entry.workspaceId === selectedId).map((entry) => entry.widgetId));
```

E após `toggleApp` (termina em `:509`):

```tsx
  async function toggleWidget(widget: { id: string; label: string }, visible: boolean) {
    if (!selected) return;
    try {
      await api.setWorkspaceWidget(widget.id, selected.id, visible);
      setMessage(visible ? "Widget visível na Home." : "Widget oculto na Home.");
      await refresh();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }
```

`refresh` (o global, que recarrega tudo) e não `refreshLinks`: o dado dos ocultos vem do componente raiz, não do estado local desta página.

- [ ] **Step 5: Acrescentar o painel**

Em `App.tsx:512`, dentro de `<div className="workspace-grid">`, depois do `<div data-function-section="workspace.link_app">…</div>` e antes do `</div>` que fecha a grade:

```tsx
<div data-function-section="workspace.set_widget"><Panel label="WIDGETS">{HOME_WIDGETS.map((widget) => <div className="relation-row" key={widget.id}><label><input type="checkbox" checked={!hiddenWidgetIds.has(widget.id)} onChange={(event) => void toggleWidget(widget, event.currentTarget.checked)} /><span><strong>{widget.label}</strong><small>Widget da Home.</small></span></label></div>)}</Panel></div>
```

Duas diferenças em relação aos painéis vizinhos, ambas propositais: a caixa marcada significa **visível** (a interface fala em visível, não em oculto), e não há botão `Abrir` — widget não é entidade que se abre.

O `data-function-section` aponta para a função que a Task 5 registra. Até lá ele não resolve para nada, e isso não quebra nada: o seletor só é usado quando um intent chega.

- [ ] **Step 6: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 7: Verificar no app**

Subir o app com `npm run tauri dev` e, na página Workspaces:

1. O painel `WIDGETS` lista os sete, todos marcados.
2. Desmarcar `INBOX` mostra a mensagem "Widget oculto na Home."
3. **Fechar e reabrir o app.** A caixa continua desmarcada — é isso que prova que foi ao banco e não ficou na memória.
4. Selecionar outro Workspace: os sete aparecem marcados. A escolha é por Workspace.
5. A Home **ainda não muda**. É esperado: aplicá-la é a Task 4.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(ui): painel WIDGETS escolhe o que a Home mostra por Workspace"
```

---

### Task 4: A Home aplica a escolha

**Files:**
- Modify: `apps/desktop/src/App.tsx:86` (componente `Widget`)
- Modify: `apps/desktop/src/App.tsx:204` (props de `HomePage`), `:269` (a grade), `:1405` (chamada de `HomePage`)
- Test: nenhum

**Interfaces:**
- Consumes: `HOME_WIDGETS` e a variável `hiddenWidgets` da Task 3 (sem alterá-las), e o tipo `HiddenWidget` da Task 2, `ScopedEmptyState`/`EmptyState` e a classe `.scoped-empty` (`App.css:1226`), já existentes.
- Produces: nada consumido por tasks posteriores.

- [ ] **Step 1: Widget aceita id e sabe sumir**

Em `App.tsx:86`, trocar o componente inteiro por:

```tsx
/* Cuida so do posicionamento na grade. A moldura e o rotulo continuam no Panel, para
   que a etapa 2 (modo de edicao) mude posicao sem tocar em nenhum widget.
   `hidden` devolve null: a regra de visibilidade fica num lugar so, e a grade
   nao precisa saber de nada — os widgets restantes reflowam sozinhos. */
function Widget({ id, size, hidden = false, children }: { id: string; size: "1x1" | "2x1" | "2x2" | "full"; hidden?: boolean; children: ReactNode }) {
  if (hidden) return null;
  return <div className="widget" data-widget={id} data-size={size}>{children}</div>;
}
```

- [ ] **Step 2: Passar ao HomePage**

Na assinatura em `App.tsx:204`, acrescentar `hiddenWidgets` à desestruturação e `hiddenWidgets: HiddenWidget[]` ao tipo, logo após `apps`. Na chamada em `:1405`, acrescentar `hiddenWidgets={hiddenWidgets}` logo após `apps={apps}`.

- [ ] **Step 3: Derivar o conjunto oculto**

Dentro de `HomePage`, junto de `inboxCapped` (`:262`):

```tsx
  // Sem Workspace selecionado nada e ocultado: "Todos" e a visao sem filtro, e
  // sem Workspace nao ha escolha a aplicar.
  const hiddenIds = useMemo(() => new Set(currentWorkspaceId ? hiddenWidgets.filter((entry) => entry.workspaceId === currentWorkspaceId).map((entry) => entry.widgetId) : []), [hiddenWidgets, currentWorkspaceId]);
  const allWidgetsHidden = HOME_WIDGETS.every((widget) => hiddenIds.has(widget.id));
```

`useMemo` já está importado (`App.tsx:1`).

- [ ] **Step 4: Ligar cada widget da grade**

No bloco `<div className="home-grid">` (`:269`), acrescentar `id` e `hidden` a cada um dos sete `<Widget>`, **sem tocar em nada dentro deles**:

| Painel | Atributos a acrescentar |
|---|---|
| `EM ANDAMENTO` | `id="now" hidden={hiddenIds.has("now")}` |
| `RECENTES` | `id="recent" hidden={hiddenIds.has("recent")}` |
| `PROJECTS` | `id="projects" hidden={hiddenIds.has("projects")}` |
| `APPS` | `id="apps" hidden={hiddenIds.has("apps")}` |
| `INBOX` | `id="inbox_pulse" hidden={hiddenIds.has("inbox_pulse")}` |
| `AÇÕES` | `id="quick_actions" hidden={hiddenIds.has("quick_actions")}` |
| `SISTEMA` | `id="system_health" hidden={hiddenIds.has("system_health")}` |

Os ids têm que bater **exatamente** com os de `HOME_WIDGETS` (Task 3, Step 1). Um id trocado aqui produz um widget que nunca some e uma caixa que nunca faz nada — sem erro nenhum na tela.

- [ ] **Step 5: O estado vazio honesto**

Logo depois do `</div>` que fecha o `.home-grid`, ainda dentro do `.page.home-page`:

```tsx
    {allWidgetsHidden ? <div className="scoped-empty"><EmptyState>Todos os widgets estão ocultos neste Workspace.</EmptyState><Button variant="outline" size="sm" onClick={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }}>Ajustar</Button></div> : null}
```

Reusa `.scoped-empty` (`App.css:1226`) — nenhum CSS novo. `currentWorkspace` e `openWorkspace` já existem no escopo do `HomePage`.

- [ ] **Step 6: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 7: Verificar no app**

1. Com um Workspace selecionado, desmarcar `SISTEMA` em Workspaces e voltar à Home: o widget sumiu, e os outros reflowaram sem buraco na grade.
2. Clicar em `Todos` no CONTEXTO: os sete voltam. Sem Workspace não há filtro.
3. Voltar ao Workspace: some de novo, sem recarregar a página.
4. Desmarcar os sete: a Home fica com Capture, CONTEXTO e a mensagem com o botão `Ajustar`, que leva à página do Workspace.
5. Trocar de Workspace várias vezes: a troca é instantânea, sem piscar — é a prova de que o filtro é em memória e não vai ao backend.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(ui): Home respeita os widgets escolhidos no Workspace"
```

---

### Task 5: A função no registro

O M/OS declara suas capacidades no registro de Functions, que alimenta a busca do Command e o painel FUNCTIONS em Settings. Uma capacidade que existe na interface e não no registro é uma mentira por omissão.

**Files:**
- Modify: `crates/mos-core/src/functions.rs:138` (entrada nova, após `workspace.link_app`)
- Modify: `apps/desktop/src/functionIntents.ts:3` (alvo novo) e `:19` (mapa)
- Modify: `apps/desktop/src/App.tsx:467` (o `useEffect` de intent em `WorkspacesPage`)
- Test: `crates/mos-core/src/functions.rs`, no `mod tests` de `:241`

**Interfaces:**
- Consumes: `function(id, name, description, category, risk, confirmation)` — o helper já usado em `functions.rs`. `FunctionIntentTarget` e `resolveFunctionTarget` (`functionIntents.ts:36`). O `data-function-section="workspace.set_widget"` posto pela Task 3, Step 4.
- Produces: nada consumido por tasks posteriores.

- [ ] **Step 1: Escrever o teste que falha**

Em `crates/mos-core/src/functions.rs`, dentro do `mod tests`:

```rust
    #[test]
    fn widget_visibility_is_a_declared_function() {
        assert!(function_registry()
            .iter()
            .any(|item| item.id == "workspace.set_widget"));
    }
```

- [ ] **Step 2: Rodar e confirmar que falha**

```powershell
cargo test -p mos-core
```

Esperado: `widget_visibility_is_a_declared_function` falha na asserção.

- [ ] **Step 3: Registrar a função**

Em `crates/mos-core/src/functions.rs`, após a entrada `workspace.link_app` (termina em `:145`):

```rust
        function(
            "workspace.set_widget",
            "Escolher widgets da Home",
            "Mostra ou oculta um widget da Home dentro de uma lente de contexto.",
            FunctionCategory::Work,
            FunctionRisk::Low,
            FunctionConfirmation::None,
        ),
```

- [ ] **Step 4: Rodar o teste**

```powershell
cargo test -p mos-core
```

Esperado: passa.

- [ ] **Step 5: Mapear o intent no front**

Em `apps/desktop/src/functionIntents.ts`, acrescentar `| "workspaces_set_widget"` ao tipo `FunctionIntentTarget` (após `"workspaces_link_app"`, linha 14) e a entrada no mapa `lowRiskTargets`, após a de `workspace.link_app`:

```ts
  "workspace.set_widget": "workspaces_set_widget",
```

- [ ] **Step 6: Levar o foco ao painel**

Em `apps/desktop/src/App.tsx`, no `useEffect` de intent do `WorkspacesPage` (`:467`), trocar o bloco:

```tsx
    if (intent.target === "workspaces_link_project" || intent.target === "workspaces_link_app") {
      setMode("view");
      const relation = intent.target === "workspaces_link_project" ? "workspace.link_project" : "workspace.link_app";
      window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-function-section='${relation}'] input`)?.focus());
    }
```

por:

```tsx
    const sections: Partial<Record<FunctionIntentTarget, string>> = {
      workspaces_link_project: "workspace.link_project",
      workspaces_link_app: "workspace.link_app",
      workspaces_set_widget: "workspace.set_widget",
    };
    const relation = sections[intent.target];
    if (relation) {
      setMode("view");
      window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-function-section='${relation}'] input`)?.focus());
    }
```

O `import` da linha 7 já traz `type FunctionIntentTarget`.

- [ ] **Step 7: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 8: Verificar no app**

1. Abrir o Command e buscar `widget`. A função `Escolher widgets da Home` aparece.
2. Executá-la leva à página Workspaces, com o foco na primeira caixa do painel `WIDGETS`.
3. O painel FUNCTIONS em Settings lista a função nova junto das outras de Work.

- [ ] **Step 9: Commit**

```bash
git add crates/mos-core/src/functions.rs apps/desktop/src/functionIntents.ts apps/desktop/src/App.tsx
git commit -m "feat(functions): registrar a escolha de widgets como capacidade"
```

---

## Limite conhecido

Ao fim das cinco tasks um Workspace pode esconder qualquer um dos sete widgets, e a escolha sobrevive a backup, export e restauração.

O que **não** existe ao fim disso: arrastar, redimensionar e ordem salva. Continuam adiados — e agora com o dado que faltava para decidir se valem o preço, porque esta etapa mostra se escolher o que ver já resolve o problema que o modo de edição prometia resolver.

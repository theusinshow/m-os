# Resource com Contexto, Fase 3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resource passa a pertencer a Workspaces, a Library passa a honrar o contexto que já anuncia no caminho, e o widget `RECURSOS` da Home ganha o escopo que lhe falta.

**Architecture:** Tabela de junção `resource_workspaces`, cópia estrutural de `app_workspaces`. O contexto ativo — hoje estado local do `HomePage` — sobe para o componente raiz, porque deixa de ser assunto da Home no instante em que a Library filtra por ele. Vincula-se um Resource por vez, no detalhe da Library; a página Workspaces não ganha painel.

**Tech Stack:** Rust (rusqlite, SQLite STRICT), Tauri 2, React 19 + TypeScript. Nenhuma dependência nova.

**Spec:** `docs/superpowers/specs/2026-08-15-mos-resource-contexto-design.md`

## Global Constraints

- **Migration publicada não se edita.** `0009_resource_workspaces.sql` é arquivo novo. Nenhuma das oito existentes muda.
- **`noUnusedLocals` está ligado.** Estado que ninguém lê não compila. Dado novo no componente raiz tem que viajar no mesmo commit que seu primeiro consumidor — foi o que derrubou a Etapa 2 e está registrado no plano dela.
- **Mensagem de erro no core é português sem acento**, como todo o resto: `"Resource ID invalido."` (`resource.rs:19`).
- **Não anotar tipo de retorno de componente React.** No React 19 o namespace global `JSX` não é exposto; anotar quebra o `tsc`.
- **Não existe teste automatizado de front.** `package.json` define apenas `"build": "tsc && vite build"`. A camada Rust é escrita com teste antes.
- **Build do Rust exige ambiente portable.** Antes de `cargo test` ou `npm run tauri dev`, no PowerShell:
  ```powershell
  $env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
  $env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
  ```
  Se `tauri dev` reclamar de porta 1420 ocupada, matar o processo `node` que a segura.
- **Nenhum Resource nasce vinculado.** Toda verificação em tela deve considerar que, antes da Task 4, o estado normal é "nada vinculado a nada".

---

### Task 1: Migration 0009 e o core

**Files:**
- Create: `crates/mos-storage-sqlite/migrations/0009_resource_workspaces.sql`
- Modify: `crates/mos-storage-sqlite/src/lib.rs:18` (`SCHEMA_VERSION`), `:26` (constante), `:192` (cadeia), e os quatro asserts de `schema_version` (hoje em `8`)
- Modify: `crates/mos-core/src/resource.rs` (struct `ResourceWorkspace`)
- Modify: `crates/mos-core/src/lib.rs` (export)
- Modify: `crates/mos-core/src/ports.rs:152` (dois métodos no trait `ResourceRepository`)
- Modify: `crates/mos-storage-sqlite/src/resource_repository.rs` (implementação)
- Modify: `crates/mos-core/src/service.rs:257` (wrappers em `MemoryService`)
- Test: `crates/mos-storage-sqlite/src/resource_repository.rs`, no `mod tests` de `:360`

**Interfaces:**
- Consumes: `SqliteStorage`, `NewResource::create(kind, title, url, note, source_capture_id)`, `NewWorkspace::create(name, description)`, `ResourceId`, `WorkspaceId`.
- Produces:
  - `pub struct ResourceWorkspace { pub resource_id: ResourceId, pub workspace_id: WorkspaceId }`, serializado como `{ resourceId, workspaceId }`;
  - `ResourceRepository::set_resource_workspace(&self, resource_id: ResourceId, workspace_id: WorkspaceId, linked: bool) -> Result<(), CoreError>`;
  - `ResourceRepository::resource_workspaces(&self) -> Result<Vec<ResourceWorkspace>, CoreError>`;
  - `MemoryService::set_resource_workspace(&self, resource_id: &str, workspace_id: &str, linked: bool)`;
  - `MemoryService::resource_workspaces(&self) -> Result<Vec<ResourceWorkspace>, CoreError>`.

- [ ] **Step 1: Escrever os testes que falham**

Em `crates/mos-storage-sqlite/src/resource_repository.rs`, dentro do `mod tests`, no fim do arquivo:

```rust
    fn workspace(storage: &SqliteStorage, name: &str) -> mos_core::Workspace {
        storage
            .create_workspace(NewWorkspace::create(name, "").unwrap())
            .unwrap()
    }

    fn site(storage: &SqliteStorage, title: &str) -> Resource {
        storage
            .create_resource(
                NewResource::create(ResourceKind::Site, title, "https://motion.dev", "", None)
                    .unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn resource_workspace_link_is_idempotent_and_isolated() {
        let (_directory, storage) = storage();
        let design = workspace(&storage, "Web Design");
        let finance = workspace(&storage, "Finance");
        let motion = site(&storage, "Motion");

        storage
            .set_resource_workspace(motion.id, design.id, true)
            .unwrap();
        storage
            .set_resource_workspace(motion.id, design.id, true)
            .unwrap();

        let links = storage.resource_workspaces().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].resource_id, motion.id);
        assert_eq!(links[0].workspace_id, design.id);

        // O mesmo Resource pode servir a dois contextos.
        storage
            .set_resource_workspace(motion.id, finance.id, true)
            .unwrap();
        assert_eq!(storage.resource_workspaces().unwrap().len(), 2);

        // Desvincular apaga so o par pedido, e repetir nao e erro.
        storage
            .set_resource_workspace(motion.id, finance.id, false)
            .unwrap();
        storage
            .set_resource_workspace(motion.id, finance.id, false)
            .unwrap();
        let links = storage.resource_workspaces().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].workspace_id, design.id);
    }

    /// As duas cascatas. Nao existe delete de Resource nem de Workspace no
    /// produto — arquivar e o caminho —, entao o DELETE cru prova que as FKs
    /// estao ativas: se `foreign_keys=ON` se perder em `configure_connection`
    /// (lib.rs:103), as linhas sobrevivem e este teste falha.
    #[test]
    fn deleting_either_side_takes_the_link() {
        let (_directory, storage) = storage();
        let design = workspace(&storage, "Web Design");
        let motion = site(&storage, "Motion");
        let easing = site(&storage, "Easings");
        storage
            .set_resource_workspace(motion.id, design.id, true)
            .unwrap();
        storage
            .set_resource_workspace(easing.id, design.id, true)
            .unwrap();

        storage
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM resources WHERE id = ?1",
                params![motion.id.to_string()],
            )
            .unwrap();
        assert_eq!(storage.resource_workspaces().unwrap().len(), 1);

        storage
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM workspaces WHERE id = ?1",
                params![design.id.to_string()],
            )
            .unwrap();
        assert!(storage.resource_workspaces().unwrap().is_empty());
    }
```

O `use` do `mod tests` (`:362`) hoje é `use mos_core::{CaptureRepository, CaptureSource, NewCapture};`. Passa a incluir `NewWorkspace` e `WorkRepository` — `create_workspace` é método do trait `WorkRepository`, e sem o trait no escopo o método não é visível.

- [ ] **Step 2: Rodar e confirmar que falha**

```powershell
$env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
$env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
cargo test -p mos-storage-sqlite
```

Esperado: erro de compilação, `no method named 'set_resource_workspace' found for struct 'SqliteStorage'`.

- [ ] **Step 3: Criar a migration**

Arquivo novo `crates/mos-storage-sqlite/migrations/0009_resource_workspaces.sql`:

```sql
-- Resource ganha contexto: o elo que faltava na cadeia da fase 3 do ROADMAP
-- (Task -> Project -> Workspace -> Resource -> App).
--
-- Copia estrutural de app_workspaces (0004_workspaces.sql). N-para-N porque uma
-- referencia pode servir a dois contextos: motion.dev vale em Web Design e pode
-- valer em Learning. Forcar um so seria uma decisao que o produto nao precisa
-- tomar, e uma coluna em resources que um dia viraria tabela mesmo assim.
--
-- Nenhuma linha nasce preenchida. No primeiro dia todo Workspace tem zero
-- resources vinculados, e a interface precisa dizer isso em vez de parecer
-- acervo vazio — ver a secao 5.3 do spec.

BEGIN IMMEDIATE;

CREATE TABLE resource_workspaces (
    resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (resource_id, workspace_id)
) STRICT;

CREATE INDEX resource_workspaces_workspace_order
    ON resource_workspaces(workspace_id, created_at DESC);

PRAGMA user_version = 9;

COMMIT;
```

- [ ] **Step 4: Ligar a migration**

Em `crates/mos-storage-sqlite/src/lib.rs`:

1. Linha 18: `const SCHEMA_VERSION: u32 = 9;`
2. Após a linha 26 (`MIGRATION_008`):

```rust
const MIGRATION_009: &str = include_str!("../migrations/0009_resource_workspaces.sql");
```

3. Na função `migrate`, depois do bloco `if current <= 7 { ... }`:

```rust
    if current <= 8 {
        connection
            .execute_batch(MIGRATION_009)
            .map_err(map_sql_error)?;
    }
```

4. Os quatro asserts de `schema_version` passam de `8` para `9`. Localizá-los com:

```bash
grep -n "schema_version, 8" crates/mos-storage-sqlite/src/lib.rs
```

São testes de migration existentes que afirmam a versão final do schema; deixá-los em 8 faz a suíte falhar por uma razão que não é a desta task.

- [ ] **Step 5: Criar o tipo no core**

Em `crates/mos-core/src/resource.rs`, após o `struct Resource`:

```rust
/// Um par significa: este Resource pertence a este contexto.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceWorkspace {
    pub resource_id: ResourceId,
    pub workspace_id: crate::WorkspaceId,
}
```

Em `crates/mos-core/src/lib.rs`, o bloco `pub use resource::{...}` passa a ser:

```rust
pub use resource::{
    validate_resource_url, NewResource, Resource, ResourceId, ResourceKind, ResourceWorkspace,
};
```

- [ ] **Step 6: Declarar os métodos no trait**

Em `crates/mos-core/src/ports.rs`, dentro de `pub trait ResourceRepository` (`:152`), após `set_resource_lifecycle`:

```rust
    fn set_resource_workspace(
        &self,
        resource_id: crate::ResourceId,
        workspace_id: crate::WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn resource_workspaces(&self) -> Result<Vec<crate::ResourceWorkspace>, CoreError>;
```

O trait já qualifica os tipos de Resource com `crate::`; seguir o mesmo estilo evita mexer no `use` do topo.

- [ ] **Step 7: Implementar no repositório**

Em `crates/mos-storage-sqlite/src/resource_repository.rs`, dentro de `impl ResourceRepository for SqliteStorage`, após `set_resource_lifecycle`:

```rust
    fn set_resource_workspace(
        &self,
        resource_id: ResourceId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        if linked {
            let now = format_time(OffsetDateTime::now_utc())?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO resource_workspaces (resource_id, workspace_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![resource_id.to_string(), workspace_id.to_string(), now],
                )
                .map_err(map_sql_error)?;
        } else {
            connection
                .execute(
                    "DELETE FROM resource_workspaces
                     WHERE resource_id = ?1 AND workspace_id = ?2",
                    params![resource_id.to_string(), workspace_id.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    /// Todos os pares numa chamada. O filtro da Library responde no instante em
    /// que o contexto muda; uma consulta por Workspace faria cada troca de
    /// contexto ir ao core, e a troca deixaria de ser instantanea.
    fn resource_workspaces(&self) -> Result<Vec<ResourceWorkspace>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT resource_id, workspace_id FROM resource_workspaces
                 ORDER BY workspace_id, created_at DESC",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql_error)?;
        let mut links = Vec::new();
        for row in rows {
            let (resource_id, workspace_id) = row.map_err(map_sql_error)?;
            links.push(ResourceWorkspace {
                resource_id: ResourceId::parse(&resource_id)?,
                workspace_id: WorkspaceId::parse(&workspace_id)?,
            });
        }
        Ok(links)
    }
```

O `use mos_core::{...}` do topo do arquivo (`:1`) precisa passar a incluir `ResourceWorkspace` e `WorkspaceId`.

- [ ] **Step 8: Rodar os testes**

```powershell
cargo test -p mos-storage-sqlite
```

Esperado: tudo passa, incluindo os dois testes novos e os quatro de migration com a versão 9.

- [ ] **Step 9: Expor no serviço**

Em `crates/mos-core/src/service.rs`, dentro de `impl MemoryService`, após `set_resource_lifecycle` ou equivalente:

```rust
    pub fn set_resource_workspace(
        &self,
        resource_id: &str,
        workspace_id: &str,
        linked: bool,
    ) -> Result<(), CoreError> {
        self.repository.set_resource_workspace(
            crate::ResourceId::parse(resource_id)?,
            crate::WorkspaceId::parse(workspace_id)?,
            linked,
        )
    }

    pub fn resource_workspaces(&self) -> Result<Vec<crate::ResourceWorkspace>, CoreError> {
        self.repository.resource_workspaces()
    }
```

```powershell
cargo test
```

Esperado: a suíte inteira passa.

- [ ] **Step 10: Commit**

```bash
git add crates/
git commit -m "feat(core): Resource pertence a Workspaces"
```

---

### Task 2: Comandos Tauri e API do front

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs:7-15` (import), `:247` (comandos novos, junto dos outros de resource), `:1088` (registro no `invoke_handler`)
- Modify: `apps/desktop/src/types.ts` (tipo `ResourceWorkspace`)
- Modify: `apps/desktop/src/api.ts:78` (dois métodos, junto dos outros de resource)
- Test: nenhum

**Interfaces:**
- Consumes: `MemoryService::set_resource_workspace` e `::resource_workspaces` da Task 1. `notify_data_changed` (`lib.rs:792`) e `schedule_snapshot` (`lib.rs:796`).
- Produces: `api.setResourceWorkspace(resourceId, workspaceId, linked): Promise<void>`, `api.resourceWorkspaces(): Promise<ResourceWorkspace[]>` e o tipo `ResourceWorkspace = { resourceId: string; workspaceId: string }`. Tasks 4, 5 e 6 consomem.

- [ ] **Step 1: Escrever os comandos**

Em `apps/desktop/src-tauri/src/lib.rs`, junto dos outros comandos de resource:

```rust
#[tauri::command]
fn list_resource_workspaces(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ResourceWorkspace>, CoreError> {
    state.memory.resource_workspaces()
}

#[tauri::command]
fn set_resource_workspace(
    resource_id: &str,
    workspace_id: &str,
    linked: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state
        .memory
        .set_resource_workspace(resource_id, workspace_id, linked)?;
    notify_data_changed(&app, "resource-workspace");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}
```

Acrescentar `ResourceWorkspace` ao `use mos_core::{...}` do topo, em ordem alfabética.

- [ ] **Step 2: Registrar no invoke_handler**

Na lista do `invoke_handler`, junto de `list_resources`:

```rust
            set_resource_workspace,
            list_resource_workspaces,
```

Esquecer este passo compila sem erro e falha só em runtime, com "command not found".

- [ ] **Step 3: Compilar o Rust**

```powershell
cargo build
```

Esperado: compila sem erro.

- [ ] **Step 4: Declarar o tipo no front**

Em `apps/desktop/src/types.ts`, após o `type Resource`:

```ts
/** Um par significa: este Resource pertence a este contexto. */
export type ResourceWorkspace = {
  resourceId: string;
  workspaceId: string;
};
```

- [ ] **Step 5: Acrescentar os métodos da API**

Em `apps/desktop/src/api.ts`, junto dos outros métodos de resource:

```ts
  resourceWorkspaces() {
    return invoke<ResourceWorkspace[]>("list_resource_workspaces");
  },
  setResourceWorkspace(resourceId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_resource_workspace", { resourceId, workspaceId, linked });
  },
```

`ResourceWorkspace` entra no `import type { ... } from "./types"` da linha 4, em ordem alfabética.

- [ ] **Step 6: Verificar o build do front**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs apps/desktop/src/types.ts apps/desktop/src/api.ts
git commit -m "feat(api): comandos de vinculo entre Resource e Workspace"
```

---

### Task 3: O contexto ativo sobe para o componente raiz

Refatoração pura: nenhum comportamento muda. Isolada de propósito — se algo quebrar no contexto ativo depois, esta é a task a olhar, e ela não contém nenhuma funcionalidade nova junto para confundir.

**Files:**
- Modify: `apps/desktop/src/App.tsx:224-240` (bloco do contexto no `HomePage`)
- Modify: `apps/desktop/src/App.tsx:222` (props de `HomePage`) e a chamada de `HomePage`
- Modify: `apps/desktop/src/App.tsx:698` (props de `LibraryPage`), `:714` (`workspaceSegment`) e a chamada de `LibraryPage`
- Modify: `apps/desktop/src/App.tsx` (componente raiz: estado e efeito de persistência)
- Test: nenhum

**Interfaces:**
- Consumes: nada novo.
- Produces: no componente raiz, `currentWorkspaceId: string`, `setCurrentWorkspaceId: (id: string) => void` e `currentWorkspace: Workspace | null`. As Tasks 4, 5 e 6 consomem `currentWorkspace`.

- [ ] **Step 1: Criar o estado no raiz**

No componente raiz, junto dos outros estados:

```tsx
  // O contexto ativo deixou de ser assunto da Home: a Library filtra por ele.
  // Continua em localStorage porque e preferencia de leitura, nao dado do core.
  const [currentWorkspaceId, setCurrentWorkspaceId] = useState(() => localStorage.getItem("m-os-current-workspace") ?? "");
  const currentWorkspace = workspaces.find((workspace) => workspace.id === currentWorkspaceId && workspace.lifecycleState === "active") ?? null;
  useEffect(() => {
    if (!currentWorkspace) {
      localStorage.removeItem("m-os-current-workspace");
      return;
    }
    localStorage.setItem("m-os-current-workspace", currentWorkspace.id);
  }, [currentWorkspace]);
```

A chave `m-os-current-workspace-name` **deixa de ser escrita**. Ela existia só para a Library desenhar o segmento do caminho sem ter o objeto; com o `currentWorkspace` chegando por prop, guardar o nome vira uma segunda fonte de verdade que pode divergir. Sobra em `localStorage` de quem já usa o app, e é inofensivo.

- [ ] **Step 2: HomePage recebe em vez de possuir**

Em `App.tsx:222`, acrescentar às props: `currentWorkspaceId: string`, `setCurrentWorkspaceId: (id: string) => void` e `currentWorkspace: Workspace | null`.

Remover do corpo do `HomePage`:

- a linha 224 (`const [currentWorkspaceId, setCurrentWorkspaceId] = useState(...)`);
- a linha do `const currentWorkspace = activeWorkspaces.find(...)`;
- as quatro linhas de `localStorage` dentro do `useEffect` (as duas de `removeItem` e as duas de `setItem`).

O `useEffect` **permanece**, com o resto do corpo intacto: ele ainda busca `workspaceProjects` e `workspaceApps`, que continuam sendo dado da Home. Apenas deixa de cuidar de persistência.

Na chamada de `HomePage`, acrescentar `currentWorkspaceId={currentWorkspaceId} setCurrentWorkspaceId={setCurrentWorkspaceId} currentWorkspace={currentWorkspace}`.

- [ ] **Step 3: LibraryPage recebe o Workspace**

Em `App.tsx:698`, acrescentar `currentWorkspace: Workspace | null` às props. Trocar a linha 714:

```tsx
  const workspaceSegment = (localStorage.getItem("m-os-current-workspace-name") ?? "").toUpperCase() || null;
```

por:

```tsx
  const workspaceSegment = currentWorkspace?.name.toUpperCase() ?? null;
```

Na chamada de `LibraryPage`, acrescentar `currentWorkspace={currentWorkspace}`.

- [ ] **Step 4: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`. Se o `tsc` reclamar de `Workspace` não usado em algum ponto, é sinal de que uma das remoções do Step 2 levou junto algo que ainda era lido.

- [ ] **Step 5: Verificar no app**

Nada pode ter mudado de comportamento:

1. A Home abre no Workspace que estava selecionado antes de fechar o app.
2. Trocar de contexto no `CONTEXTO` continua estreitando PROJECTS e APPS.
3. `Todos` continua limpando o contexto.
4. A Library continua mostrando `M / <WORKSPACE> / LIBRARY`, e o segmento **acompanha** a troca de contexto sem precisar recarregar — isso é ganho, e é o sinal de que o estado subiu de verdade.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "refactor(ui): contexto ativo sobe da Home para o componente raiz"
```

---

### Task 4: Vincular no detalhe do Resource

**Files:**
- Modify: `apps/desktop/src/App.tsx` (componente raiz: estado `resourceWorkspaces` e o `refresh`)
- Modify: `apps/desktop/src/App.tsx:698` (props de `LibraryPage`), `:932` (bloco novo após o `POR QUÊ?`) e a chamada de `LibraryPage`
- Modify: `apps/desktop/src/App.css` (regras de `.resource-context`)
- Test: nenhum

**Interfaces:**
- Consumes: `api.setResourceWorkspace` e `api.resourceWorkspaces` da Task 2. `Panel` (`App.tsx:76`), `appError`.
- Produces: no componente raiz, `resourceWorkspaces: ResourceWorkspace[]`, carregado pelo `refresh`. As Tasks 5 e 6 consomem.

- [ ] **Step 1: Carregar os vínculos no raiz**

Junto dos outros estados do componente raiz:

```tsx
  const [resourceWorkspaces, setResourceWorkspaces] = useState<ResourceWorkspace[]>([]);
```

No `refresh`, acrescentar `api.resourceWorkspaces()` **no fim** do array do `Promise.all` e `nextResourceWorkspaces` no fim da desestruturação — a lista é posicional, e mexer na ordem de um lado só embaralha tudo em silêncio. Na linha seguinte, acrescentar `setResourceWorkspaces(nextResourceWorkspaces);` ao fim da sequência de setters.

`ResourceWorkspace` entra no `import type { ... } from "./types"` do topo de `App.tsx`.

- [ ] **Step 2: LibraryPage recebe vínculos e Workspaces**

Em `App.tsx:698`, acrescentar às props: `workspaces: Workspace[]` e `resourceWorkspaces: ResourceWorkspace[]`. Na chamada, acrescentar `workspaces={workspaces} resourceWorkspaces={resourceWorkspaces}`.

Dentro do componente, junto das outras derivações:

```tsx
  const activeWorkspaces = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const linkedWorkspaceIds = new Set(resourceWorkspaces.filter((link) => link.resourceId === selectedId).map((link) => link.workspaceId));
```

- [ ] **Step 3: Escrever o toggle**

Dentro do `LibraryPage`, junto das outras ações:

```tsx
  async function toggleWorkspace(workspaceId: string, linked: boolean) {
    if (!selected) return;
    try {
      await api.setResourceWorkspace(selected.id, workspaceId, linked);
      await refresh();
    } catch (nextError) { setMessage(appError(nextError).message); }
  }
```

Sem mensagem de sucesso: a caixa marcada já é a confirmação, e uma frase a cada clique numa lista de cinco viraria ruído. `setMessage` continua para o erro, que é o caso em que o silêncio mentiria.

- [ ] **Step 4: Acrescentar o bloco CONTEXTO**

Em `App.tsx:932`, logo **depois** da linha do `resource-note`:

```tsx
        {activeWorkspaces.length ? <div className="resource-context"><span className="micro-label">CONTEXTO</span><div>{activeWorkspaces.map((workspace) => <label key={workspace.id}><input type="checkbox" checked={linkedWorkspaceIds.has(workspace.id)} onChange={(event) => void toggleWorkspace(workspace.id, event.currentTarget.checked)} /><span>{workspace.name}</span></label>)}</div></div> : null}
```

As duas perguntas se leem juntas: `POR QUÊ?` guarda o motivo, `CONTEXTO` diz a que lente pertence. Sem Workspace ativo o bloco não aparece — marcar nada em lugar nenhum não é escolha, é confusão.

- [ ] **Step 5: Acrescentar o CSS**

Em `apps/desktop/src/App.css`, após o bloco `.resource-note`:

```css
.resource-context {
  display: grid;
  gap: var(--space-2);
  margin-top: var(--space-4);
}

.resource-context div {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-4);
}

.resource-context label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
  font: var(--text-small);
}
```

Só tokens existentes, nenhum valor cravado.

- [ ] **Step 6: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 7: Verificar no app**

1. Abrir um Resource na Library. Abaixo do `POR QUÊ?` aparece `CONTEXTO` com um checkbox por Workspace ativo, todos desmarcados — nenhum Resource nasce vinculado.
2. Marcar um. **Fechar e reabrir o app.** Continua marcado.
3. Marcar dois: o mesmo Resource em dois contextos é permitido, e é o motivo de a tabela ser N-para-N.
4. Nada mais na tela muda ainda: filtrar é a Task 5.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(ui): detalhe do Resource declara a que contexto ele pertence"
```

---

### Task 5: A Library honra o contexto

**Files:**
- Modify: `apps/desktop/src/App.tsx:711` (estado `scoped`), `:714` (`workspaceSegment`), `:716` (`visibleResources`), `:838` (o `filter-bar`), `:873` (o vazio)
- Test: nenhum

**Interfaces:**
- Consumes: `currentWorkspace` da Task 3, `resourceWorkspaces` da Task 4, `ScopedEmptyState` (`App.tsx:117`).
- Produces: nada consumido por tasks posteriores.

- [ ] **Step 1: O estado do recorte**

Junto de `kindFilter` e `view` (`App.tsx:711`), que já são declarados como preferência de leitura:

```tsx
  // Ligado por padrao quando ha contexto: o caminho anuncia o recorte, entao a
  // lista tem que cumpri-lo. Sem contexto ativo o estado nao tem efeito.
  const [scoped, setScoped] = useState(true);
```

- [ ] **Step 2: Aplicar o recorte**

Trocar as linhas 714 a 716:

```tsx
  const workspaceSegment = currentWorkspace?.name.toUpperCase() ?? null;
  const liveResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
  const visibleResources = kindFilter === "all" ? liveResources : liveResources.filter((resource) => resource.kind === kindFilter || resource.id === selectedId);
```

por:

```tsx
  // O recorte so existe quando ha contexto ativo E o usuario o mantem ligado.
  // O `currentWorkspace !== null` repetido abaixo nao e redundancia: o tsc nao
  // estreita um objeto a partir de um boolean derivado guardado em variavel.
  const scoping = scoped && currentWorkspace !== null;
  // O caminho so anuncia o recorte quando ele esta de fato aplicado. Anunciar
  // sem aplicar foi o que este ciclo veio corrigir.
  const workspaceSegment = scoping && currentWorkspace ? currentWorkspace.name.toUpperCase() : null;
  const scopedResourceIds = new Set(currentWorkspace ? resourceWorkspaces.filter((link) => link.workspaceId === currentWorkspace.id).map((link) => link.resourceId) : []);
  const liveResources = resources.filter((resource) => resource.lifecycleState === "active" || resource.id === selectedId);
  // O selecionado nunca some da lista, mesmo fora do recorte: ele esta aberto
  // ao lado, e sumir da lista o que esta aberto e desorientador.
  const contextResources = scoping ? liveResources.filter((resource) => scopedResourceIds.has(resource.id) || resource.id === selectedId) : liveResources;
  const visibleResources = kindFilter === "all" ? contextResources : contextResources.filter((resource) => resource.kind === kindFilter || resource.id === selectedId);
```

Atenção ao contador do cabeçalho, na linha do `pane-heading`: ele mostra `liveResources.length`. Passa a mostrar `contextResources.length`, senão o número contradiz a lista logo abaixo dele.

- [ ] **Step 3: O escape no filter-bar**

Em `App.tsx:838`, dentro do `<div className="filter-bar">`, como **primeiro** grupo, antes do de tipo:

```tsx
        {currentWorkspace ? <div className="filter-group" role="group" aria-label="Filtrar por contexto">
          {([[true, "NESTE CONTEXTO"], [false, "TUDO"]] as const).map(([value, label]) => <button key={label} type="button" className="filter-label" data-active={scoped === value || undefined} aria-pressed={scoped === value} onClick={() => setScoped(value)}>{label}</button>)}
        </div> : null}
```

Mesmo formato dos filtros de tipo e do `GRID/LISTA` — nenhum CSS novo. Sem contexto ativo o grupo não aparece: um botão que não muda nada é pior que botão nenhum.

- [ ] **Step 4: O vazio do primeiro dia**

Na linha 873, o vazio de hoje é:

```tsx
      {!visibleResources.length && mode !== "new" ? <div className="library-empty"><EmptyState>Guarde um link junto do motivo pelo qual ele merece ser lembrado.</EmptyState><Button variant="primary" onClick={startNew}>Salvar primeiro link</Button></div> : null}
```

Ele assume acervo vazio. Com recorte ligado a lista pode estar vazia com o acervo cheio — que é o estado de **todo** Workspace no dia seguinte à migration. Trocar por:

```tsx
      {!visibleResources.length && mode !== "new" ? (scoping && liveResources.length ? <div className="library-empty"><ScopedEmptyState total={liveResources.length} workspace={currentWorkspace} noun="resource" onLink={() => setScoped(false)} /></div> : <div className="library-empty"><EmptyState>Guarde um link junto do motivo pelo qual ele merece ser lembrado.</EmptyState><Button variant="primary" onClick={startNew}>Salvar primeiro link</Button></div>) : null}
```

`ScopedEmptyState` (`App.tsx:117`) hoje aceita `noun: "app" | "project"`. Estender para `"app" | "project" | "resource"` e acrescentar o caso:

```tsx
  const counted = noun === "app"
    ? `${total} ${total === 1 ? "app cadastrado" : "apps cadastrados"}`
    : noun === "resource"
      ? `${total} ${total === 1 ? "resource salvo" : "resources salvos"}`
      : `${total} ${total === 1 ? "Project criado" : "Projects criados"}`;
```

No mesmo componente, o primeiro `if` — o que trata acervo vazio ou ausencia de contexto — precisa da frase de `resource` **agora**, e não na Task 6. Estender o tipo sem estender a frase deixa um ramo que responderia "Projects criados aparecerão aqui" para um acervo de Resources; a Task 5 não chega a acioná-lo, mas ramo errado que ninguem aciona hoje e bug agendado:

```tsx
  if (total === 0 || !workspace) {
    return <EmptyState>{noun === "app" ? "Apps cadastrados aparecerão aqui." : noun === "resource" ? "Referências salvas aparecerão aqui." : "Projects criados aparecerão aqui."}</EmptyState>;
  }
```

O `onLink` aqui **não** é "Vincular": daqui não há como vincular em lote, e o caminho útil é ver o acervo inteiro. O rótulo do botão precisa acompanhar — acrescentar uma prop `linkLabel?: string` ao `ScopedEmptyState`, com `Vincular` como padrão para não mexer em Projects e Apps, e passar `linkLabel="Ver tudo"` aqui.

- [ ] **Step 5: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 6: Verificar no app**

Este é o roteiro mais importante do plano, porque cobre o estado do primeiro dia:

1. **Com um Workspace ativo e nenhum Resource vinculado a ele:** a Library mostra "N resources salvos, nenhum em `<Workspace>`" com o botão `Ver tudo`. Não pode parecer acervo vazio nem perda de dado.
2. Clicar em `Ver tudo`: o recorte desliga, o acervo inteiro aparece, e o caminho volta a ser `M / LIBRARY` — sem o segmento do Workspace.
3. Vincular um Resource ao Workspace ativo (bloco `CONTEXTO` da Task 4), voltar para `NESTE CONTEXTO`: ele aparece, e o contador do cabeçalho bate com a lista.
4. Trocar de contexto na Home e voltar à Library: a lista acompanha, instantaneamente e sem recarregar.
5. Clicar em `Todos` na Home: o grupo `NESTE CONTEXTO / TUDO` some da Library, porque não há contexto a aplicar.
6. Com um Resource aberto fora do recorte, ligar o recorte: ele **continua** na lista, para não sumir de baixo do que está aberto ao lado.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(ui): Library cumpre o contexto que o caminho anuncia"
```

---

### Task 6: O widget RECURSOS ganha escopo

Fecha a inconsistência que o commit `14c85a9` registrou como limitação assumida, e que motivou este ciclo.

**Files:**
- Modify: `apps/desktop/src/App.tsx:222` (props de `HomePage`), `:284` (derivação) e a linha do widget `RECURSOS`
- Test: nenhum

**Interfaces:**
- Consumes: `resourceWorkspaces` da Task 4, `currentWorkspace` da Task 3, `ScopedEmptyState` com `noun="resource"` da Task 5.
- Produces: nada.

- [ ] **Step 1: Passar os vínculos ao HomePage**

Em `App.tsx:222`, acrescentar `resourceWorkspaces: ResourceWorkspace[]` às props. Na chamada, acrescentar `resourceWorkspaces={resourceWorkspaces}`.

- [ ] **Step 2: Estreitar a lista**

Trocar a derivação de `activeResources` (`App.tsx:284`):

```tsx
  const activeResources = resources.filter((resource) => resource.lifecycleState === "active");
```

por:

```tsx
  const allActiveResources = resources.filter((resource) => resource.lifecycleState === "active");
  // Mesma regra dos vizinhos: com contexto ativo, so o que pertence a ele.
  const scopedResourceIds = new Set(currentWorkspace ? resourceWorkspaces.filter((link) => link.workspaceId === currentWorkspace.id).map((link) => link.resourceId) : []);
  const activeResources = currentWorkspace ? allActiveResources.filter((resource) => scopedResourceIds.has(resource.id)) : allActiveResources;
```

- [ ] **Step 3: O vazio com escopo**

Na linha do widget `RECURSOS`, trocar o estado vazio:

```tsx
<EmptyState>Referências salvas aparecerão aqui.</EmptyState>
```

por:

```tsx
<ScopedEmptyState total={allActiveResources.length} workspace={currentWorkspace} noun="resource" onLink={() => openLibraryPage()} linkLabel="Ver tudo" />
```

`ScopedEmptyState` já trata `total === 0` ou sem workspace devolvendo a frase simples, e a frase de `resource` foi acrescentada na Task 5 — aqui é só usar.

Também remover o comentário do widget que registrava a ausência de escopo — ele deixou de ser verdade, e comentário desatualizado mente pior que comentário nenhum.

- [ ] **Step 4: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 5: Verificar no app**

1. Com contexto ativo e nada vinculado: o widget mostra "N resources salvos, nenhum em `<Workspace>`" com `Ver tudo`, que leva à Library.
2. Vincular um Resource ao contexto: ele aparece no widget.
3. `Todos` na Home: o widget volta a mostrar o acervo inteiro.
4. Os três widgets com escopo — PROJECTS, APPS e RECURSOS — passam a se comportar igual. Essa era a dívida.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(ui): widget RECURSOS respeita o contexto ativo"
```

---

## Limite conhecido

Ao fim das seis tasks, Resource tem contexto e a Library o honra. O que **não** existe:

- `Resource ↔ Project`. É a outra metade da §9.1 do Roadmap, e vem junto com a §9.2 — a página de Project como centro de contexto, que hoje só mostra Tasks.
- Vincular em lote. Cada Resource é vinculado um a um, no detalhe. Se o acervo crescer muito, isso vira trabalho, e aí a página Workspaces ganha o painel que esta etapa recusou.
- Recents e Favorites (§9.4).

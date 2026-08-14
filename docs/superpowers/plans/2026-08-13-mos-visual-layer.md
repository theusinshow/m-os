# Camada visual e estrutural — plano de implementação (Spec A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aplicar o design fechado do handoff sobre o app existente, criando no back-end o schema que o desenho exige.

**Architecture:** O app é Tauri 2 + React 19 + TypeScript, com `api.ts` como fronteira única sobre comandos Rust, e um core hexagonal (`mos-core` = domínio puro, `mos-storage-sqlite` = adapter). As mudanças de schema descem por `MIGRATION_007` na cadeia existente; a camada visual é CSS em `App.css` sobre tokens, sem nenhuma dependência nova.

**Tech Stack:** Rust (rusqlite, serde, time, uuid), React 19, TypeScript 5.8, Vite 7, CSS puro.

**Spec:** `docs/superpowers/specs/2026-08-13-mos-visual-layer-design.md`
**Mãe:** `docs/superpowers/specs/2026-08-13-mos-v03-design.md`
**Design:** `Design System/design_handoff_frontend/README.md` — manda sobre qualquer palpite visual.

## Global Constraints

- **Zero dependência nova.** Nem UI, nem estilo, nem animação, nem ícone, nem test runner.
- **Zero literal de cor** (`#`, `rgb(`, `hsl(`) fora de `mos-tokens.css`. Verificável: `grep -rn "#[0-9a-fA-F]\{3,6\}" apps/desktop/src` = vazio.
- **Spacing só na escala:** 4 · 8 · 12 · 20 · 32 · 52 · 84. Radius só 2 / 3 / 8.
- **Mono (`--font-system`) é só dado de sistema:** timestamp, caminho, id, atalho, contagem, tipo. Nunca título, nome, texto do usuário ou rótulo de botão.
- **Motion só o que está na tabela do README.** Nada recorrente acima de 200ms. Zero bounce, zero skeleton pulsante, zero spinner que não seja a barra girando.
- **Sem emoji na interface.**
- **Warning não tem cor:** ícone + frase + borda neutra. Cor só em `--danger` e sódio.
- **Focus idêntico em todo o sistema:** `border-color: var(--signal-ink)` + `box-shadow: 0 0 0 var(--focus-ring) var(--signal-ring)`, sem `outline`.
- **CI é o bar:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run build`.
- **Front-end não tem test runner.** Tarefas de UI são verificadas por `npm run build` (que roda `tsc`) e conferência contra o protótipo em `design/M-OS Redesign v0.7 - Telas.dc.html`. Não adicionar Vitest/Jest para satisfazer o formato deste plano.

---

### Task 1: Destravar o build — tokens

O `main.tsx:7` importa `Design System/handoff/mos-tokens.css`, pasta deletada. **O build está quebrado agora.** Nada mais funciona até isso ser resolvido.

O pacote novo não define tokens que o `App.css` usa (`--line`, `--marker`, `--z-*`, `--overlay-backdrop`, `--border-control`, `--content-max`, `--column-min`, `--list-pane-width`, `--target-min`, `--focus-ring`) nem o bloco `forced-colors`. Adotá-lo cru quebraria o CSS inteiro.

**Files:**
- Modify: `Design System/design_handoff_frontend/mos-tokens.css`
- Modify: `apps/desktop/src/main.tsx:7`

**Interfaces:**
- Consumes: nada
- Produces: todos os tokens usados por `App.css` disponíveis no `:root`, e o app compilando.

- [ ] **Step 1: Confirmar que o build está quebrado**

```bash
cd apps/desktop && npm run build
```

Esperado: FALHA, com erro de resolução em `../../../Design System/handoff/mos-tokens.css`. Se passar, o estado do repositório mudou — pare e reavalie antes de seguir.

- [ ] **Step 2: Remover o `@import` do Google Fonts**

Apagar a linha 8 de `mos-tokens.css`:

```css
@import url('https://fonts.googleapis.com/css2?family=Schibsted+Grotesk:wght@400;500;700&family=JetBrains+Mono:wght@400;500&display=swap');
```

Substituir por:

```css
/* Fontes são empacotadas pelo cliente. O desktop importa via Fontsource em main.tsx.
   O desktop não deve depender de rede para renderizar texto. */
```

- [ ] **Step 3: Acrescentar o bloco de extensões de implementação**

No fim do bloco `:root`, antes do `}` de fechamento:

```css
  /* ---------- Extensões de implementação ----------
     Não fazem parte do handoff. São dimensões operacionais que o app precisa e
     que o pacote não define. Extensão de token é permitida e deve ser explícita
     (DECISIONS.md:448). Nenhuma delas é cor de marca. */
  --line: 1px;
  --marker: 2px;
  --focus-ring: 3px;
  --target-min: 24px;
  --content-max: 1100px;
  --column-min: 260px;
  --list-pane-width: 400px;
  --border-control: #626A70;
  --overlay-backdrop: rgba(10, 12, 14, 0.72);
  --z-rail: 10;
  --z-drawer: 20;
  --z-backdrop: 30;
  --z-overlay: 40;
  --z-receipt: 50;
```

E no bloco `[data-theme='light']`, antes do `}`:

```css
  --border-control: #868E93;
  --overlay-backdrop: rgba(20, 24, 26, 0.28);
```

- [ ] **Step 4: Restaurar o bloco `forced-colors`**

No fim do arquivo:

```css
@media (forced-colors: active) {
  :root,
  [data-theme='light'] {
    --canvas: Canvas;
    --surface: Canvas;
    --surface-raised: Canvas;
    --surface-hover: Canvas;
    --surface-active: Highlight;
    --border: ButtonBorder;
    --border-strong: ButtonText;
    --border-control: ButtonText;
    --text: CanvasText;
    --text-secondary: CanvasText;
    --text-system: GrayText;
    --text-disabled: GrayText;
    --text-placeholder: GrayText;
    --signal-fill: Highlight;
    --signal-ink: Highlight;
    --signal-wash: Highlight;
    --signal-ring: Highlight;
    --signal-hover: Highlight;
    --signal-press: Highlight;
    --on-signal: HighlightText;
    --success: CanvasText;
    --danger: Mark;
    --shadow-overlay: none;
    --overlay-backdrop: Canvas;
  }
}
```

`TECHNICAL-SPIKE-DESKTOP-SHELL.md:55` registra alto contraste como requisito arquitetural aprovado. Perder este bloco seria regressão.

- [ ] **Step 5: Apontar o import para o pacote novo**

Em `main.tsx:7`:

```tsx
import "../../../Design System/design_handoff_frontend/mos-tokens.css";
```

- [ ] **Step 6: Verificar que o build volta**

```bash
cd apps/desktop && npm run build
```

Esperado: PASSA.

- [ ] **Step 7: Verificar que nenhum token ficou órfão**

```bash
grep -o "var(--[a-z0-9-]*)" apps/desktop/src/App.css | sort -u | sed 's/var(--\(.*\))/\1/' > /tmp/usados.txt
grep -o "^\s*--[a-z0-9-]*:" "Design System/design_handoff_frontend/mos-tokens.css" | tr -d ' :' | sed 's/--//' | sort -u > /tmp/definidos.txt
comm -23 /tmp/usados.txt /tmp/definidos.txt
```

Esperado: saída **vazia**. Qualquer nome listado é um token usado e não definido — resolver antes de seguir.

- [ ] **Step 8: Commit**

```bash
git add "Design System/" apps/desktop/src/main.tsx
git commit -m "fix: restaurar tokens apos troca do pacote de handoff

O pacote design_handoff_frontend substituiu Design System/handoff, quebrando
o import em main.tsx. Mescla os valores canonicos do pacote novo com as
extensoes de implementacao que o App.css usa, remove a dependencia de rede
do Google Fonts (as fontes ja vem do Fontsource) e restaura forced-colors."
```

---

### Task 2: `TaskState` — de 3 para 6 estados

O kanban do desenho tem seis colunas. O core tem três estados.

**Files:**
- Modify: `crates/mos-core/src/work.rs:47-76`
- Modify: `crates/mos-storage-sqlite/src/lib.rs` (constante `SCHEMA_VERSION`, cadeia de `migrate`)
- Test: `crates/mos-core/src/work.rs` (módulo `tests` no fim), `crates/mos-storage-sqlite/src/lib.rs` (módulo `tests`)

**Interfaces:**
- Consumes: nada
- Produces: `TaskState::{Inbox, Backlog, Planned, Doing, Review, Done}`, com `as_str()` e `parse()` cobrindo os seis. `SCHEMA_VERSION = 7`.

- [ ] **Step 1: Escrever o teste que falha**

Substituir o teste `task_states_have_stable_storage_values` em `crates/mos-core/src/work.rs`:

```rust
    #[test]
    fn task_states_have_stable_storage_values() {
        assert_eq!(TaskState::Inbox.as_str(), "inbox");
        assert_eq!(TaskState::Backlog.as_str(), "backlog");
        assert_eq!(TaskState::Planned.as_str(), "planned");
        assert_eq!(TaskState::Doing.as_str(), "doing");
        assert_eq!(TaskState::Review.as_str(), "review");
        assert_eq!(TaskState::Done.as_str(), "done");
    }

    #[test]
    fn task_states_round_trip_through_parse() {
        for state in [
            TaskState::Inbox,
            TaskState::Backlog,
            TaskState::Planned,
            TaskState::Doing,
            TaskState::Review,
            TaskState::Done,
        ] {
            assert_eq!(TaskState::parse(state.as_str()).unwrap(), state);
        }
    }

    #[test]
    fn unknown_task_state_is_rejected() {
        assert!(TaskState::parse("arquivado").is_err());
        assert!(TaskState::parse("").is_err());
    }
```

- [ ] **Step 2: Rodar e ver falhar**

```bash
cargo test -p mos-core work::tests
```

Esperado: FALHA na compilação — `no variant named 'Inbox' found for enum 'TaskState'`.

- [ ] **Step 3: Ampliar o enum**

Em `crates/mos-core/src/work.rs`, substituir o enum e o `impl`:

```rust
/// Estado de trabalho da Task.
///
/// A ordem das variantes e a ordem das colunas do kanban.
///
/// NOTA: `Inbox` aqui NAO e a Inbox de Captures. Sao conceitos distintos que
/// compartilham o nome porque o design usa INBOX como rotulo da primeira coluna.
/// Capture tem `processing_state`; Task tem `state`. Nunca sao a mesma coisa.
/// Ver docs/superpowers/specs/2026-08-13-mos-v03-design.md secao 4.3.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Inbox,
    Backlog,
    Planned,
    Doing,
    Review,
    Done,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Backlog => "backlog",
            Self::Planned => "planned",
            Self::Doing => "doing",
            Self::Review => "review",
            Self::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "inbox" => Ok(Self::Inbox),
            "backlog" => Ok(Self::Backlog),
            "planned" => Ok(Self::Planned),
            "doing" => Ok(Self::Doing),
            "review" => Ok(Self::Review),
            "done" => Ok(Self::Done),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de Task desconhecido.",
                false,
            )),
        }
    }
}
```

- [ ] **Step 4: Rodar e ver passar**

```bash
cargo test -p mos-core work::tests
```

Esperado: PASSA. Se `cargo build` apontar `match` não exaustivo em outro arquivo, é o compilador fazendo o trabalho dele — corrija cada ponto tratando os três estados novos.

- [ ] **Step 5: Escrever o teste da migration**

Em `crates/mos-storage-sqlite/src/lib.rs`, no módulo `tests`:

```rust
    #[test]
    fn migration_007_accepts_the_three_new_task_states() {
        let temporary = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(temporary.path()).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, 7);

        for state in ["inbox", "planned", "review"] {
            storage
                .connection()
                .execute(
                    "INSERT INTO tasks (id, title, description, project_id, source_capture_id, \
                     state, lifecycle_state, created_at, updated_at, completed_at) \
                     VALUES (?1, 'titulo', '', NULL, NULL, ?2, 'active', '2026-01-01T00:00:00Z', \
                     '2026-01-01T00:00:00Z', NULL)",
                    rusqlite::params![uuid::Uuid::now_v7().to_string(), state],
                )
                .expect("estado novo deve ser aceito pela constraint");
        }
    }

    #[test]
    fn migration_007_preserves_existing_task_states() {
        let temporary = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(temporary.path()).unwrap();
        let id = uuid::Uuid::now_v7().to_string();
        storage
            .connection()
            .execute(
                "INSERT INTO tasks (id, title, description, project_id, source_capture_id, \
                 state, lifecycle_state, created_at, updated_at, completed_at) \
                 VALUES (?1, 'antiga', '', NULL, NULL, 'backlog', 'active', \
                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', NULL)",
                rusqlite::params![id],
            )
            .unwrap();

        let state: String = storage
            .connection()
            .query_row("SELECT state FROM tasks WHERE id = ?1", [&id], |row| row.get(0))
            .unwrap();
        assert_eq!(state, "backlog", "estado existente nao pode ser reescrito");
    }
```

Se `SqliteStorage` não expuser `connection()`, adicione um acessor `#[cfg(test)] pub(crate) fn connection(&self) -> &Connection` seguindo o padrão que os testes existentes do arquivo já usam para chegar ao banco.

- [ ] **Step 6: Rodar e ver falhar**

```bash
cargo test -p mos-storage-sqlite migration_007
```

Esperado: FALHA — `schema_version` é 6, não 7.

- [ ] **Step 7: Escrever a migration**

Em `crates/mos-storage-sqlite/src/lib.rs`, subir a constante:

```rust
const SCHEMA_VERSION: u32 = 7;
```

Acrescentar ao fim da cadeia em `migrate`, seguindo exatamente o padrão dos passos anteriores:

```rust
    if current <= 6 {
        connection
            .execute_batch(MIGRATION_007)
            .map_err(map_sql_error)?;
    }
```

E a constante da migration, ao lado das outras. SQLite não altera `CHECK`, então a mesa é recriada — que é o padrão que as migrations anteriores deste arquivo já usam:

```rust
const MIGRATION_007: &str = r#"
PRAGMA foreign_keys = OFF;

CREATE TABLE tasks_new (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  source_capture_id TEXT REFERENCES captures(id) ON DELETE SET NULL,
  state TEXT NOT NULL CHECK (state IN ('inbox','backlog','planned','doing','review','done')),
  lifecycle_state TEXT NOT NULL CHECK (lifecycle_state IN ('active','archived','trashed')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT
);

INSERT INTO tasks_new SELECT id, title, description, project_id, source_capture_id,
  state, lifecycle_state, created_at, updated_at, completed_at FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

PRAGMA foreign_keys = ON;
PRAGMA user_version = 7;
"#;
```

**Antes de escrever isto, leia `MIGRATION_006` no mesmo arquivo e copie a forma dele** — nomes de coluna, índices recriados e a maneira como ele carimba `user_version`. As colunas acima são as de `types.ts:35-46`; se a tabela real divergir, a tabela real manda.

- [ ] **Step 8: Rodar e ver passar**

```bash
cargo test -p mos-storage-sqlite
```

Esperado: PASSA, incluindo os testes existentes que afirmam `schema_version` — eles esperavam 6 e precisam ser atualizados para 7 (`lib.rs:332`, `lib.rs:361`, `lib.rs:405`).

- [ ] **Step 9: Verificação completa**

```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

Esperado: tudo passa.

- [ ] **Step 10: Commit**

```bash
git add crates/
git commit -m "feat(core): seis estados de Task

Acrescenta inbox, planned e review para as seis colunas do kanban do design.
Estados existentes sao preservados sem reescrita. O snapshot pre-migration
automatico ja protege os dados (lib.rs:188).

Reverte a recomendacao de ARCHITECTURE-REVIEW.md de remover Planned; a
justificativa esta na spec-mae secao 4.2."
```

---

### Task 3: `ResourceKind` — de 1 para 4 variantes

Os filtros da Library (`TUDO · SITES · LIBRARIES · IMAGENS · NOTAS`) exigem tipo. O core só conhece `Link`.

**Files:**
- Modify: `crates/mos-core/src/resource.rs:35-58, 87-109`
- Modify: `crates/mos-storage-sqlite/src/lib.rs` (`MIGRATION_007`, mesma migration da Task 2)
- Test: `crates/mos-core/src/resource.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `MIGRATION_007` da Task 2 — as duas mudanças de schema vão no **mesmo** passo de migration.
- Produces: `ResourceKind::{Site, Library, Image, Note}`; `NewResource::create(kind, title, url, note, source_capture_id)`; `NewResource::create_link` preservado como atalho para `Site`.

- [ ] **Step 1: Escrever o teste que falha**

Substituir o módulo `tests` de `crates/mos-core/src/resource.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_round_trip() {
        for kind in [
            ResourceKind::Site,
            ResourceKind::Library,
            ResourceKind::Image,
            ResourceKind::Note,
        ] {
            assert_eq!(ResourceKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(ResourceKind::parse("link").is_err());
    }

    #[test]
    fn site_and_library_require_http_url() {
        assert!(NewResource::create(ResourceKind::Site, "", "motion.dev", "", None).is_err());
        assert!(NewResource::create(ResourceKind::Library, "", "", "", None).is_err());
    }

    #[test]
    fn note_has_no_url() {
        let resource =
            NewResource::create(ResourceKind::Note, "Ideia", "", "o motivo", None).unwrap();
        assert_eq!(resource.url, "");
        assert_eq!(resource.note, "o motivo");
    }

    #[test]
    fn note_requires_a_title() {
        assert!(NewResource::create(ResourceKind::Note, "  ", "", "corpo", None).is_err());
    }

    #[test]
    fn image_accepts_local_path_or_url() {
        assert!(NewResource::create(
            ResourceKind::Image,
            "captura",
            "C:/imagens/hero.png",
            "",
            None
        )
        .is_ok());
        assert!(
            NewResource::create(ResourceKind::Image, "captura", "https://x.dev/a.png", "", None)
                .is_ok()
        );
        assert!(NewResource::create(ResourceKind::Image, "captura", "", "", None).is_err());
    }

    #[test]
    fn create_link_is_a_shortcut_for_site() {
        let resource =
            NewResource::create_link("", "https://motion.dev", "Animacoes", None).unwrap();
        assert_eq!(resource.kind, ResourceKind::Site);
        assert_eq!(resource.title, "https://motion.dev");
    }
}
```

- [ ] **Step 2: Rodar e ver falhar**

```bash
cargo test -p mos-core resource::tests
```

Esperado: FALHA na compilação — variantes e `create` não existem.

- [ ] **Step 3: Implementar**

Em `crates/mos-core/src/resource.rs`, substituir o enum, o `impl` e `create_link`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Site,
    Library,
    Image,
    Note,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Library => "library",
            Self::Image => "image",
            Self::Note => "note",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "site" => Ok(Self::Site),
            "library" => Ok(Self::Library),
            "image" => Ok(Self::Image),
            "note" => Ok(Self::Note),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de Resource desconhecido.",
                false,
            )),
        }
    }
}

impl NewResource {
    /// Cria um Resource de qualquer tipo.
    ///
    /// A validacao de URL varia por tipo: Site e Library exigem http(s);
    /// Image aceita http(s) ou caminho local; Note nao tem URL e por isso
    /// exige titulo proprio, ja que nao ha URL para servir de fallback.
    pub fn create(
        kind: ResourceKind,
        title: &str,
        url: &str,
        note: &str,
        source_capture_id: Option<CaptureId>,
    ) -> Result<Self, CoreError> {
        let url = match kind {
            ResourceKind::Site | ResourceKind::Library => validate_resource_url(url)?,
            ResourceKind::Image => validate_image_location(url)?,
            ResourceKind::Note => String::new(),
        };

        let title = match title.trim() {
            "" if url.is_empty() => {
                return Err(CoreError::new(
                    ErrorCode::InvalidInput,
                    "Uma Note precisa de titulo.",
                    false,
                ))
            }
            "" => url.clone(),
            value => value.to_owned(),
        };

        Ok(Self {
            id: ResourceId::new(),
            kind,
            title,
            url,
            note: note.trim().to_owned(),
            source_capture_id,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    /// Atalho historico: um link e um Site.
    pub fn create_link(
        title: &str,
        url: &str,
        note: &str,
        source_capture_id: Option<CaptureId>,
    ) -> Result<Self, CoreError> {
        Self::create(ResourceKind::Site, title, url, note, source_capture_id)
    }
}

fn validate_image_location(value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Uma imagem precisa de um endereco ou caminho.",
            false,
        ));
    }
    Ok(value.to_owned())
}
```

- [ ] **Step 4: Rodar e ver passar**

```bash
cargo test -p mos-core resource::tests
```

Esperado: PASSA.

- [ ] **Step 5: Acrescentar a conversão à migration**

Em `MIGRATION_007`, ao lado do bloco de `tasks`, recriar `resources` com a `CHECK` nova e converter os dados:

```sql
CREATE TABLE resources_new (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('site','library','image','note')),
  title TEXT NOT NULL,
  url TEXT NOT NULL DEFAULT '',
  note TEXT NOT NULL DEFAULT '',
  source_capture_id TEXT REFERENCES captures(id) ON DELETE SET NULL,
  lifecycle_state TEXT NOT NULL CHECK (lifecycle_state IN ('active','archived','trashed')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

INSERT INTO resources_new SELECT id, 'site', title, url, note, source_capture_id,
  lifecycle_state, created_at, updated_at FROM resources;

DROP TABLE resources;
ALTER TABLE resources_new RENAME TO resources;
```

Todo `link` vira `site`: era o único valor possível e sempre significou endereço na web.

Se `resources` participa da projeção de busca FTS, o índice precisa ser reconstruído depois do rename — `ensure_search_projection` (`lib.rs:204`) já existe para isso e roda na abertura. Confirme lendo a função antes de assumir.

- [ ] **Step 6: Escrever o teste da conversão**

Em `crates/mos-storage-sqlite/src/lib.rs`, módulo `tests`:

```rust
    #[test]
    fn migration_007_converts_link_resources_to_site() {
        let temporary = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(temporary.path()).unwrap();
        let count: i64 = storage
            .connection()
            .query_row("SELECT count(*) FROM resources WHERE kind = 'link'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "nenhum resource pode restar como 'link'");
    }
```

- [ ] **Step 7: Verificação completa**

```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

- [ ] **Step 8: Commit**

```bash
git add crates/
git commit -m "feat(core): quatro tipos de Resource

site, library, image e note, para os filtros da Library. Validacao de URL
passa a variar por tipo: Note nao tem URL e por isso exige titulo proprio.
create_link permanece como atalho para Site."
```

---

### Task 4: Capacidades de App e repositório de Project

Duas colunas simples que o desenho exige e que fecham a migration.

**Files:**
- Modify: `crates/mos-core/src/app.rs`, `crates/mos-core/src/work.rs` (struct `Project`)
- Modify: `crates/mos-storage-sqlite/src/lib.rs` (`MIGRATION_007`), `app_repository.rs`, `work_repository.rs`
- Test: nos módulos `tests` correspondentes

**Interfaces:**
- Consumes: `MIGRATION_007` das Tasks 2 e 3.
- Produces: `RegisteredApp { can_open, can_read, can_write, can_automate: bool }`; `Project { repository: String }`.

- [ ] **Step 1: Acrescentar as colunas à migration**

Em `MIGRATION_007`. Estas são aditivas, então `ALTER TABLE` basta — sem recriar tabela:

```sql
ALTER TABLE registered_apps ADD COLUMN can_open INTEGER NOT NULL DEFAULT 0;
ALTER TABLE registered_apps ADD COLUMN can_read INTEGER NOT NULL DEFAULT 0;
ALTER TABLE registered_apps ADD COLUMN can_write INTEGER NOT NULL DEFAULT 0;
ALTER TABLE registered_apps ADD COLUMN can_automate INTEGER NOT NULL DEFAULT 0;

UPDATE registered_apps SET can_open = 1
  WHERE launch_target IS NOT NULL AND launch_target <> '';

ALTER TABLE projects ADD COLUMN repository TEXT NOT NULL DEFAULT '';
```

`can_open = 1` para quem já tem `launch_target` porque abrir é o que esses apps comprovadamente já fazem. Declarar `false` para eles seria mentir sobre uma capacidade em uso.

- [ ] **Step 2: Escrever o teste que falha**

Em `crates/mos-storage-sqlite/src/lib.rs`, módulo `tests`:

```rust
    #[test]
    fn migration_007_marks_launchable_apps_as_openable() {
        let temporary = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(temporary.path()).unwrap();
        let mismatched: i64 = storage
            .connection()
            .query_row(
                "SELECT count(*) FROM registered_apps \
                 WHERE launch_target IS NOT NULL AND launch_target <> '' AND can_open = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(mismatched, 0);
    }

    #[test]
    fn migration_007_defaults_project_repository_to_empty() {
        let temporary = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(temporary.path()).unwrap();
        let nulls: i64 = storage
            .connection()
            .query_row("SELECT count(*) FROM projects WHERE repository IS NULL", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(nulls, 0);
    }
```

- [ ] **Step 3: Rodar e ver falhar**

```bash
cargo test -p mos-storage-sqlite migration_007
```

Esperado: FALHA — colunas não existem.

- [ ] **Step 4: Estender as structs do core**

Em `crates/mos-core/src/app.rs`, na struct `RegisteredApp`, depois de `launch_target`:

```rust
    pub can_open: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_automate: bool,
```

Em `crates/mos-core/src/work.rs`, na struct `Project`, depois de `description`:

```rust
    pub repository: String,
```

`serde(rename_all = "camelCase")` já está nas duas structs, então o TypeScript recebe `canOpen`, `canRead`, `canWrite`, `canAutomate` e `repository` sem trabalho extra.

- [ ] **Step 5: Atualizar os repositories**

Em `app_repository.rs` e `work_repository.rs`: acrescentar as colunas em cada `SELECT`, cada `INSERT` e cada mapeamento de linha. O compilador aponta todos os pontos que faltam — siga os erros até zero.

Os inteiros do SQLite viram `bool` com `row.get::<_, i64>(n)? != 0`, que é o padrão que o arquivo já usa para campos booleanos, se houver. Se não houver, use essa forma.

- [ ] **Step 6: Rodar e ver passar**

```bash
cargo test --workspace
```

- [ ] **Step 7: Verificação completa**

```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

- [ ] **Step 8: Commit**

```bash
git add crates/
git commit -m "feat(core): capacidades de App e repositorio de Project

can_open/read/write/automate em RegisteredApp e repository em Project.
Apps com launch_target ja definido nascem com can_open, porque abrir e o
que eles comprovadamente ja fazem."
```

---

### Task 5: Fronteira — comandos Tauri e `api.ts`

O schema mudou; a fronteira precisa expor isso. As assinaturas mudam — o handoff proíbe, a decisão do proprietário sobrepõe (spec-mãe §4.1).

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src/api.ts`, `apps/desktop/src/types.ts`

**Interfaces:**
- Consumes: as structs das Tasks 2, 3 e 4.
- Produces: `api.createResource(kind, title, url, note, sourceCaptureId)`; `api.updateProject(id, name, description, repository)`; `api.updateRegisteredApp(…, capabilities)`; tipos TS espelhando os seis `TaskState` e os quatro `ResourceKind`.

- [ ] **Step 1: Atualizar os tipos do renderer**

Em `apps/desktop/src/types.ts`:

```ts
export type TaskState = "inbox" | "backlog" | "planned" | "doing" | "review" | "done";

export type ResourceKind = "site" | "library" | "image" | "note";
```

Na type `Resource`, trocar `kind: "link"` por `kind: ResourceKind`.

Na type `Project`, acrescentar `repository: string;`.

Na type `RegisteredApp`, acrescentar:

```ts
  canOpen: boolean;
  canRead: boolean;
  canWrite: boolean;
  canAutomate: boolean;
```

- [ ] **Step 2: Estender `api.ts`**

```ts
  createResource(kind: ResourceKind, title: string, url: string, note: string, sourceCaptureId: string | null = null) {
    return invoke<Resource>("create_resource", { input: { kind, title, url, note, sourceCaptureId } });
  },
  updateResource(id: string, kind: ResourceKind, title: string, url: string, note: string) {
    return invoke<Resource>("update_resource", { input: { id, kind, title, url, note } });
  },
  updateProject(id: string, name: string, description: string, repository: string) {
    return invoke<Project>("update_project", { input: { id, name, description, repository } });
  },
  createProject(name: string, description: string, repository: string) {
    return invoke<Project>("create_project", { input: { name, description, repository } });
  },
```

Para `updateRegisteredApp`, acrescentar um parâmetro `capabilities`:

```ts
  updateRegisteredApp(id: string, name: string, description: string, sourceUrl: string | null, launchKind: AppLaunchKind | null, launchTarget: string | null, capabilities: { canOpen: boolean; canRead: boolean; canWrite: boolean; canAutomate: boolean }) {
    return invoke<RegisteredApp>("update_registered_app", { input: { id, name, description, sourceUrl, launchKind, launchTarget, ...capabilities } });
  },
```

- [ ] **Step 3: Atualizar os comandos Rust**

Em `src-tauri/src/lib.rs`, os structs de input dos comandos `create_resource`, `update_resource`, `create_project`, `update_project` e `update_registered_app` ganham os campos novos. Eles usam `#[serde(rename_all = "camelCase")]` — mantenha.

`create_resource` passa a rotear por tipo:

```rust
let new_resource = mos_core::NewResource::create(
    mos_core::ResourceKind::parse(&input.kind)?,
    &input.title,
    &input.url,
    &input.note,
    source_capture_id,
)?;
```

- [ ] **Step 4: Compilar os dois lados**

```bash
cargo build --workspace && cd apps/desktop && npm run build
```

Esperado: PASSA. O `tsc` vai apontar cada chamada de `createResource`/`createProject`/`updateRegisteredApp` no `App.tsx` que precisa do argumento novo — corrija todas. Nesta task, passe `"site"` como `kind` e `""` como `repository` nos pontos de chamada existentes; as telas ganham controle real nas tasks 9 a 13.

- [ ] **Step 5: Verificação completa**

```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cd apps/desktop && npm run build
```

- [ ] **Step 6: Commit**

```bash
git add crates/ apps/desktop/
git commit -m "feat(api): expor tipo de Resource, repositorio e capacidades

Assinaturas de api.ts mudam por decisao registrada na spec-mae secao 4.1:
o handoff supunha um back-end que ja suportasse o desenho, e ele nao suporta."
```

---

### Task 6: Shell — símbolo, rail de seis, topbar

**Files:**
- Create: `apps/desktop/src/Symbol.tsx`
- Modify: `apps/desktop/src/App.tsx:1043` (array `nav`), `:1062` (JSX do shell)
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: tokens da Task 1.
- Produces: `<Symbol size={16 | 26 | 44} />`; shell com rail de seis destinos e slot de estado de sistema na topbar.

- [ ] **Step 1: Criar o símbolo**

Três desenhos, um por faixa de escala. **Nunca escalar um único SVG** — é regra explícita do README, e o motivo é que a barra precisa de ângulo diferente para manter a mesma leitura óptica em cada tamanho.

`apps/desktop/src/Symbol.tsx`:

```tsx
/**
 * Simbolo do M/OS: barra solida em campo sodio.
 *
 * Tres desenhos com angulo corrigido por escala (22 / 18 / 14 graus).
 * Escalar um unico SVG entre tamanhos e proibido pelo handoff: o angulo
 * precisa mudar para a leitura optica se manter. viewBox 0 0 64 64 nos tres.
 */
const bars = {
  large: "38,8 53,8 26,56 11,56",   // 1024 · 512 · 256 · 128 — 22 graus
  medium: "40,10 54,10 24,54 10,54", // 64 · 48 — 18 graus
  small: "42,12 56,12 22,52 8,52",   // 32 · 24 · 16 — 14 graus
} as const;

function barFor(size: number) {
  if (size >= 128) return bars.large;
  if (size >= 48) return bars.medium;
  return bars.small;
}

export function Symbol({ size = 26, spinning = false }: { size?: number; spinning?: boolean }) {
  return (
    <svg
      className="mos-symbol"
      data-spinning={spinning || undefined}
      width={size}
      height={size}
      viewBox="0 0 64 64"
      aria-hidden="true"
      focusable="false"
    >
      <polygon points={barFor(size)} />
    </svg>
  );
}
```

- [ ] **Step 2: Estilo do símbolo e do motion oficial**

Em `App.css`:

```css
.mos-symbol { display: block; }
.mos-symbol polygon { fill: currentColor; transform-origin: center; }

/* Meia-volta: a barra e simetrica em 180 graus, entao a meia-volta cai
   exatamente sobre ela mesma. Nada entra, nada sai. */
@keyframes barHalf {
  0%, 22%   { transform: rotate(0deg); }
  62%, 100% { transform: rotate(180deg); }
}

/* Trabalhando: o unico spinner do sistema. Nao usar circulo nem tres pontos. */
@keyframes barSpin {
  from { transform: rotate(0deg); }
  to   { transform: rotate(180deg); }
}

.mos-symbol[data-spinning] polygon {
  animation: barSpin 900ms linear infinite;
}
```

- [ ] **Step 3: Reduzir o rail a seis destinos**

Em `App.tsx`, substituir o array `nav` (linha 1043):

```tsx
  // Seis destinos. O sistema tem oito paginas e o rail aceita seis:
  // Workspaces entra pelo Command; Settings fica no rodape do rail.
  const nav: { page: Page; label: string; icon: IconName; count?: number }[] = [
    { page: "home", label: "Home", icon: "home" },
    { page: "inbox", label: "Inbox", icon: "inbox", count: inbox.length },
    { page: "tasks", label: "Tasks", icon: "board" },
    { page: "projects", label: "Projects", icon: "projects" },
    { page: "library", label: "Library", icon: "library" },
    { page: "apps", label: "Apps", icon: "apps" },
  ];
```

A ordem é a do README: `home · inbox · board · projects · library · apps`.

- [ ] **Step 4: Reescrever o shell**

Substituir o `<aside className="nav-rail">` e o `<header className="topbar">` em `App.tsx:1062`:

```tsx
<aside className="nav-rail">
  <div className="rail-symbol" aria-hidden="true"><Symbol size={26} /></div>
  <nav aria-label="Navegação principal">
    {nav.map((item) => (
      <button
        key={item.page}
        aria-current={page === item.page ? "page" : undefined}
        aria-label={item.label}
        title={item.label}
        onClick={() => navigate(item.page)}
      >
        <Icon name={item.icon} filled={page === item.page} />
        {item.count ? <span className="rail-count">{item.count}</span> : null}
      </button>
    ))}
  </nav>
  <div className="rail-footer">
    <IconButton label="Quick Capture" icon="capture" onClick={() => void api.showQuickCapture()} />
    <IconButton label="Settings" icon="settings" active={page === "settings"} onClick={() => navigate("settings")} />
  </div>
</aside>
```

O símbolo tem `aria-hidden` e não é botão: é assinatura, não destino.

Topbar com o slot de estado à direita:

```tsx
<header className="topbar">
  <button className="command-trigger" onClick={() => setCommandOpen(true)}>
    <span className="slash">/</span>
    <span>Command</span>
    <kbd>CTRL K</kbd>
  </button>
  <div className="system-state" aria-live="polite">
    {busy
      ? <><Symbol size={13} spinning /><span className="micro">SINCRONIZANDO</span></>
      : <span className="micro">{pageMeta}</span>}
  </div>
</header>
```

- [ ] **Step 5: Acrescentar o estado `busy` e o meta da página**

Em `DesktopApp`, junto aos outros `useState`:

```tsx
  const [busy, setBusy] = useState(false);
```

Em `refresh`, envolver a carga:

```tsx
  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      // ... corpo existente, sem alteracao ...
    } finally {
      setBusy(false);
    }
  }, []);
```

E o meta, que na Home é data e hora e nas outras é o nome da página:

```tsx
  const pageMeta = useMemo(() => {
    if (page !== "home") return page.toUpperCase();
    return new Intl.DateTimeFormat("pt-BR", {
      weekday: "short", day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit",
    }).format(new Date()).toUpperCase().replace(",", " ·");
  }, [page]);
```

- [ ] **Step 6: CSS do rail e da topbar**

Todos os valores vêm do README §Shell. Rail 52px, símbolo 26px com `margin-bottom: 24px`, destinos de 40px com `gap: 4px`, ícone 20px stroke 1.25. Ativo: barra de 2px em `--signal-fill` colada na borda esquerda, 16px de altura, 12px do topo, mais ícone em `--text`. Inativo: ícone em `--text-system`. Topbar 44px com borda inferior `--border`.

```css
.nav-rail {
  width: var(--rail-width);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-3) 0;
  border-right: var(--line) solid var(--border);
  z-index: var(--z-rail);
}
.rail-symbol {
  color: var(--on-signal);
  background: var(--signal-fill);
  border-radius: 5px;
  padding: 4px;
  margin-bottom: 24px;
}
.nav-rail nav { display: flex; flex-direction: column; gap: var(--space-1); }
.nav-rail nav button {
  position: relative;
  width: 40px; height: 40px;
  display: grid; place-items: center;
  background: none; border: none; border-radius: var(--radius);
  color: var(--text-system);
  cursor: pointer;
  transition: color var(--dur-instant) var(--ease-state),
              background var(--dur-instant) var(--ease-state);
}
.nav-rail nav button:hover { background: var(--surface-hover); }
.nav-rail nav button[aria-current='page'] { color: var(--text); }
.nav-rail nav button[aria-current='page']::before {
  content: '';
  position: absolute; left: 0; top: 12px;
  width: var(--marker); height: 16px;
  background: var(--signal-fill);
}
.rail-footer { margin-top: auto; display: flex; flex-direction: column; gap: var(--space-1); }

.topbar {
  height: 44px;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 var(--space-4);
  border-bottom: var(--line) solid var(--border);
}
.system-state { display: flex; align-items: center; gap: var(--space-2); color: var(--text-system); }
.micro { font: var(--text-micro); letter-spacing: var(--tracking-micro); text-transform: uppercase; }
```

- [ ] **Step 7: Verificar**

```bash
cd apps/desktop && npm run build && npm run tauri dev
```

Conferir lado a lado com `design/M-OS Redesign v0.7 - Telas.dc.html`: seis ícones no rail, símbolo no topo sem ser clicável, Settings e Quick Capture no rodapé, barra de seleção de 2px no item ativo, topbar com o meta à direita. Navegar por teclado e confirmar que o foco é visível em todos.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/
git commit -m "feat(ui): shell do design v0.7

Simbolo em tres desenhos por escala (nunca escalado), rail reduzido aos seis
destinos com Workspaces indo para o Command e Settings para o rodape, e
topbar com o slot de estado de sistema alimentado por um busy global."
```

---

### Tasks 7 a 12: as seis telas

Cada tela é uma task independente, na ordem do README §"Ordem sugerida de implementação": **Home → Inbox → Command/Quick Capture/recibo → Tasks → Projects → Apps → Library**.

Cada uma segue a mesma forma, e por isso a estrutura está descrita uma vez aqui em vez de repetida seis vezes:

1. Ler a seção correspondente do README **inteira** antes de escrever qualquer linha.
2. Reescrever o componente da página em `App.tsx` conforme a seção.
3. Escrever o CSS em `App.css` usando exclusivamente tokens.
4. Implementar os cinco estados: repouso, hover, focus, ativo, bloqueado.
5. Conferir operação por teclado e foco visível.
6. Verificar nos dois temas.
7. `npm run build` + comparação lado a lado com o protótipo.
8. `grep -rn "#[0-9a-fA-F]\{3,6\}" apps/desktop/src` = vazio.
9. Commit por tela.

Pontos onde a tela exige decisão além do README, e a decisão já tomada:

- **Home:** o `savedWash` de 900ms na row recém-criada exige o `Set` efêmero `savedIds`. É o único uso de fundo sódio em row não selecionada.
- **Inbox:** o bloco de interpretação do Hermes nasce em **estado vazio honesto**. A moldura visual correta, dizendo que a interpretação ainda não está ligada. Não fabricar interpretação falsa (spec A §4).
- **Tasks:** seis colunas; `DOING` é a única com rótulo e régua em sódio. Drag existente e `J/K` continuam valendo. FLIP de 180ms no movimento entre colunas.
- **Library:** o "motivo pelo qual foi salvo" mapeia em `note`. Resource sem `note` mostra estado vazio que convida a preencher, nunca linha em branco. Tile sem imagem usa a hachura, nunca ilustração ou ícone gigante.
- **Overlays:** Command 720px, Quick Capture 640px, ambos a 34% do topo, entrando em 160ms e saindo em 90ms só com opacity. O **recibo passa a viver ~5s** — hoje são 8s (`App.tsx:1019`, `window.setTimeout(… , 8_000)`). Canto inferior esquerdo, 72px da borda, 24px do pé.
- **Apps:** o bloco CAPACIDADES lê os quatro booleanos da Task 4. `✓` em `--text`, `—` em `--text-disabled`.
- **Projects:** `REPOSITÓRIO` em mono 14, lendo o campo da Task 4. Vazio mostra estado vazio, não string em branco.

---

### Task 13: Ícones rasterizados e light mode

**Files:**
- Create: `apps/desktop/src-tauri/icons/*` (regerados)
- Modify: `apps/desktop/src/App.css`

- [ ] **Step 1: Gerar os ícones a partir dos três polygons**

Rasterizar para os tamanhos que o Tauri exige, usando o desenho correto por faixa: 22° para 128 e acima, 18° para 48–64, 14° para 32 e abaixo. Campo `#E7C24E`, barra em `#0A0C0E`. Radius do quadrado: 20% em 1024, 11 em 64, 6 em 32, 3 em 16.

- [ ] **Step 2: Conferir 16px real**

Abrir o app e olhar a taskbar, o tray e o favicon. O desenho de 14° é o que precisa sobreviver ali — se a barra virar um borrão, o ângulo está errado para o tamanho.

- [ ] **Step 3: Conferir light mode**

Alternar o tema e percorrer as seis telas. Light é **paridade, não inversão**. Conferir especificamente que âmbar puro não aparece como tinta de texto (`--signal-ink` no light é `#8A6A12`).

- [ ] **Step 4: Conferir reduced motion**

Ativar `prefers-reduced-motion` no sistema e confirmar que nenhuma animação roda e que nenhum transform anima. O `mos-tokens.css` já zera as durações; a obrigação do código é não escrever transform fora da tabela.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/
git commit -m "feat(ui): simbolo rasterizado e paridade de light mode"
```

---

### Task 14: Atualizar a documentação de produto

Os docs de produto contradizem o que foi construído. Deixá-los assim é pior que não ter doc.

**Files:**
- Modify: `docs/CORE-FOUNDATION.md:100-110`, `docs/DECISIONS.md`, `docs/ROADMAP.md`

- [ ] **Step 1: Corrigir `CORE-FOUNDATION.md`**

O trecho que lista `backlog/doing/done` como estados iniciais e justifica a exclusão de `Planned` e `Review` passa a registrar os seis estados, mantendo a justificativa original como histórico e acrescentando o motivo da mudança: coluna de kanban é visualização, não semântica. A observação sobre `inbox` colidir com a Inbox de Captures **permanece** — a colisão é real e a mitigação é nomenclatura.

- [ ] **Step 2: Registrar as decisões em `DECISIONS.md`**

Uma entrada por decisão da spec-mãe §3 e §4, no formato que o arquivo já usa. As duas que mais precisam de registro: o handoff proíbe mudar back-end e foi sobreposto; `ARCHITECTURE-REVIEW.md` recomendava remover `Planned` e a recomendação fica revogada.

- [ ] **Step 3: Ajustar `ROADMAP.md`**

`12.1 Repository Association` deixa de estar inteiramente na Fase 5: o campo existe desde já, a integração com a API do GitHub continua adiada.

- [ ] **Step 4: Commit**

```bash
git add docs/
git commit -m "docs: alinhar documentacao de produto ao v0.3

Seis estados de Task, campo repository antecipado e as decisoes de
sobreposicao registradas. Doc que contradiz o codigo e pior que doc ausente."
```

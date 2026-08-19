# O leque radial e o rail de volta a oito — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tirar Calendário, Finance e Reuniões do rail, reagrupar os oito que ficam, e dar aos três um leque de cinco pétalas fixas no rodapé ao centro.

**Architecture:** A regra de resolução (padrão de fábrica + geometria dos ângulos) mora numa **única cópia**, em `apps/desktop/src/leque.ts`, testada por Vitest. O Rust carrega só o que o banco precisa para não aceitar lixo: um validador de `kind` e a tabela `radial_pins`, cuja ausência de linhas significa "o que o desenho escolheu". O componente `Leque.tsx` desenha e não decide.

**Tech Stack:** React 18 + TypeScript (Vite, Vitest), Rust (Tauri 2, rusqlite, SQLite STRICT), CSS puro com tokens de `packages/design-system/tokens.css`.

## Global Constraints

- **A regra tem UMA cópia, e ela é a do front.** `apps/desktop/src/homeLayout.ts` registra que a regra do arranjo já viveu em dois lugares e que a cópia do Rust *"ficou para tras em silencio — com os testes dela passando, que e o pior jeito de ficar para tras"*. Não crie `leque.rs` no core com padrão ou geometria.
- **Tabela vazia significa "o que o desenho escolheu"**, nunca "nada fixado". Herdado das migrations `0017`/`0018`.
- **Não existe teste de DOM neste repo**, por decisão registrada em `apps/desktop/vitest.config.ts`. O que for testado tem de ser função pura.
- **Comentários e mensagens de commit em português, sem acento em código Rust** (o repo escreve `funcao`, `nao`, `posicao` dentro de `.rs`). Em `.ts`, `.tsx`, `.md` e `.sql` o acento é usado normalmente.
- **Nenhuma mudança no `CommandSurface`.**
- Verificação final obrigatória pela skill `ver-o-app`; `orca computer` não funciona nesta máquina.
- Antes de qualquer `cargo`: `export TMP="<scratchpad>/tmp"; export TEMP="$TMP"`.

---

### Task 1: A migration e o validador de `kind`

**Files:**
- Create: `crates/mos-storage-sqlite/migrations/0021_radial_pins.sql`
- Modify: `crates/mos-storage-sqlite/src/lib.rs:51` (constante) e `:279` (aplicação)
- Modify: `crates/mos-core/src/work.rs:349` (ao lado de `validate_widget_id`)
- Test: `crates/mos-core/src/work.rs` (módulo `tests` no fim do arquivo)

**Interfaces:**
- Produces: `mos_core::validate_pin_kind(&str) -> Result<String, CoreError>`; tabela `radial_pins`; `PRAGMA user_version = 21`.

- [ ] **Step 1: Escreva o teste que falha**

No fim de `crates/mos-core/src/work.rs`, dentro do `mod tests` existente:

```rust
    #[test]
    fn kind_de_petala_aceita_forma_e_recusa_lixo() {
        assert_eq!(validate_pin_kind("app").unwrap(), "app");
        assert_eq!(validate_pin_kind("  pagina  ").unwrap(), "pagina");
        assert_eq!(validate_pin_kind("acao_rapida").unwrap(), "acao_rapida");

        // Forma, e nao vocabulario: um kind novo passa sem migration.
        assert_eq!(validate_pin_kind("widget3").unwrap(), "widget3");

        for lixo in ["", "  ", "App", "3app", "app-ficha", "app.ficha", "açao"] {
            assert!(validate_pin_kind(lixo).is_err(), "deveria recusar {lixo:?}");
        }
    }
```

- [ ] **Step 2: Rode e confirme que falha**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"; mkdir -p "$TMP"
cargo test -p mos-core kind_de_petala
```
Esperado: FAIL com `cannot find function 'validate_pin_kind'`.

- [ ] **Step 3: Implemente o validador**

Em `crates/mos-core/src/work.rs`, logo depois de `validate_widget_id`:

```rust
/// A forma de um `kind` de petala, e so a forma.
///
/// O vocabulario — `app`, `acao`, `pagina` — vive no front, em `leque.ts`, pelo
/// mesmo motivo que `widget_id` e opaco aqui: um enum no banco faria de cada
/// tipo novo de petala uma migration, e tipo de petala muda mais rapido que
/// schema.
pub fn validate_pin_kind(value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 40
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
    if valid {
        Ok(value.to_owned())
    } else {
        Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Tipo de petala invalido.",
            false,
        ))
    }
}
```

- [ ] **Step 4: Rode e confirme que passa**

```bash
cargo test -p mos-core kind_de_petala
```
Esperado: PASS.

- [ ] **Step 5: Escreva a migration**

Crie `crates/mos-storage-sqlite/migrations/0021_radial_pins.sql`:

```sql
-- O leque: cinco petalas fixas no rodape ao centro.
--
-- TABELA VAZIA SIGNIFICA "O QUE O DESENHO ESCOLHEU", e nao "nada fixado". E a
-- mesma inversao das migrations 0017 e 0018, e ela paga duas contas: mudar o
-- padrao de fabrica alcanca todo mundo que ainda nao personalizou, e trocar um
-- slot nao congela os outros quatro.
--
-- `kind` e string opaca. O vocabulario de hoje e `app`, `acao` e `pagina`, e ele
-- mora em `leque.ts`; o CHECK garante FORMA, nao vocabulario. Um enum aqui faria
-- de cada tipo novo de petala uma migration.
--
-- `slot` aceita 0..11 embora o desenho use CINCO. O banco guarda "qual das
-- posicoes", que e forma; QUANTAS posicoes a interface oferece e vocabulario, e
-- a 0017 ja registrou que vocabulario muda mais rapido que migration. Ir a seis
-- petalas um dia nao custa migration nenhuma.
--
-- `workspace_id` nasce nullable com NULL significando "Todos", copiado da 0018 —
-- inclusive o indice sobre COALESCE, que existe porque no SQLite coluna de
-- PRIMARY KEY aceita NULL e NULL nunca colide com NULL. Sem ele, "Todos"
-- aceitaria doze linhas no mesmo slot e o leque viraria lixo silencioso.
-- Com isso, "um leque por Workspace" depois e comportamento novo e nao
-- estrutura nova.

BEGIN IMMEDIATE;

CREATE TABLE radial_pins (
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL CHECK (slot >= 0 AND slot <= 11),
    kind TEXT NOT NULL CHECK (kind GLOB '[a-z][a-z0-9_]*'),
    target TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX radial_pins_escopo
    ON radial_pins (COALESCE(workspace_id, ''), slot);

PRAGMA user_version = 21;

COMMIT;
```

- [ ] **Step 6: Registre a migration**

Em `crates/mos-storage-sqlite/src/lib.rs`, depois da linha da `MIGRATION_020`:

```rust
const MIGRATION_021: &str = include_str!("../migrations/0021_radial_pins.sql");
```

E depois do bloco `if current <= 19`:

```rust
    if current <= 20 {
        connection
            .execute_batch(MIGRATION_021)
            .map_err(map_sql_error)?;
    }
```

- [ ] **Step 7: Confirme que o schema sobe**

```bash
cargo test -p mos-storage-sqlite
```
Esperado: PASS. Os testes de storage abrem bancos temporários e rodam todas as migrations; qualquer erro de SQL aparece aqui.

- [ ] **Step 8: Commit**

```bash
git add crates/mos-core/src/work.rs crates/mos-storage-sqlite/migrations/0021_radial_pins.sql crates/mos-storage-sqlite/src/lib.rs
git commit -m "feat(leque): a tabela das petalas, com o vazio significando o desenho"
```

---

### Task 2: Ler e gravar as pétalas

**Files:**
- Modify: `crates/mos-core/src/work.rs` (tipos `RadialPin` e `RadialPinInput`)
- Modify: `crates/mos-core/src/ports.rs:122` (ao lado de `set_widget_layout`)
- Modify: `crates/mos-core/src/service.rs:1031` (ao lado de `set_widget_layout`)
- Modify: `crates/mos-storage-sqlite/src/work_repository.rs` (ao lado de `widget_placements`)
- Test: `crates/mos-storage-sqlite/src/work_repository.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `mos_core::validate_pin_kind` (Task 1).
- Produces: `RadialPin { workspace_id: Option<WorkspaceId>, slot: i64, kind: String, target: String }`; `RadialPinInput { slot: i64, kind: String, target: String }`; `WorkRepository::radial_pins() -> Result<Vec<RadialPin>, CoreError>`; `WorkRepository::set_radial_pin(Option<WorkspaceId>, RadialPinInput) -> Result<Vec<RadialPin>, CoreError>`; `WorkRepository::clear_radial_pin(Option<WorkspaceId>, i64) -> Result<Vec<RadialPin>, CoreError>`.

- [ ] **Step 1: Escreva o teste que falha**

No `mod tests` de `crates/mos-storage-sqlite/src/work_repository.rs`:

```rust
    #[test]
    fn petala_grava_le_e_respeita_o_escopo_de_todos() {
        // `storage()` devolve (TempDir, SqliteStorage); o TempDir precisa
        // continuar vivo, senao o diretorio some debaixo da conexao.
        let (_dir, storage) = storage();

        // Vazio significa o desenho, e nao lista vazia de erro.
        assert!(storage.radial_pins().unwrap().is_empty());

        let pins = storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput {
                    slot: 0,
                    kind: "pagina".into(),
                    target: "calendario".into(),
                },
            )
            .unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].slot, 0);
        assert_eq!(pins[0].kind, "pagina");
        assert!(pins[0].workspace_id.is_none());

        // O MESMO slot em "Todos" substitui, e nao acumula. E o buraco que o
        // indice sobre COALESCE existe para fechar.
        let pins = storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput {
                    slot: 0,
                    kind: "app".into(),
                    target: "019ffc4f-2936-7152-84b7-672d7bdb5bfc".into(),
                },
            )
            .unwrap();
        assert_eq!(pins.len(), 1, "slot 0 de Todos nao pode ter duas linhas");
        assert_eq!(pins[0].kind, "app");

        // Kind fora de forma nunca chega ao banco.
        assert!(storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput { slot: 1, kind: "Pagina".into(), target: "x".into() },
            )
            .is_err());

        // Limpar devolve o slot ao desenho.
        let pins = storage.clear_radial_pin(None, 0).unwrap();
        assert!(pins.is_empty());
    }
```

- [ ] **Step 2: Rode e confirme que falha**

```bash
cargo test -p mos-storage-sqlite petala_grava
```
Esperado: FAIL com `no method named 'set_radial_pin'`.

- [ ] **Step 3: Declare os tipos no core**

Em `crates/mos-core/src/work.rs`, junto dos tipos de widget:

```rust
/// Uma petala fixada. `workspace_id` nulo e a visao "Todos" (migration 0021).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadialPin {
    pub workspace_id: Option<WorkspaceId>,
    pub slot: i64,
    pub kind: String,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadialPinInput {
    pub slot: i64,
    pub kind: String,
    pub target: String,
}
```

Exporte os dois no `lib.rs` do core junto dos tipos de widget já exportados.

- [ ] **Step 4: Declare no port**

Em `crates/mos-core/src/ports.rs`, dentro do mesmo trait que tem `set_widget_layout`:

```rust
    fn radial_pins(&self) -> Result<Vec<crate::RadialPin>, CoreError>;

    fn set_radial_pin(
        &self,
        workspace: Option<WorkspaceId>,
        pin: crate::RadialPinInput,
    ) -> Result<Vec<crate::RadialPin>, CoreError>;

    fn clear_radial_pin(
        &self,
        workspace: Option<WorkspaceId>,
        slot: i64,
    ) -> Result<Vec<crate::RadialPin>, CoreError>;
```

- [ ] **Step 5: Implemente no repositório**

Em `crates/mos-storage-sqlite/src/work_repository.rs`, ao lado de `widget_placements`:

```rust
    /// Todas as petalas de uma vez, pelo mesmo motivo de `widget_placements`:
    /// sao no maximo algumas dezenas de linhas, e uma chamada so deixa a troca
    /// de Workspace filtrar em memoria em vez de ir ao core a cada clique.
    fn radial_pins(&self) -> Result<Vec<mos_core::RadialPin>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_id, slot, kind, target
                 FROM radial_pins
                 ORDER BY COALESCE(workspace_id, ''), slot",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            let (workspace_id, slot, kind, target) = row.map_err(map_sql_error)?;
            found.push(mos_core::RadialPin {
                // Nulo e a visao "Todos", e nao um dado faltando (migration 0021).
                workspace_id: workspace_id.as_deref().map(WorkspaceId::parse).transpose()?,
                slot,
                kind,
                target,
            });
        }
        Ok(found)
    }

    fn set_radial_pin(
        &self,
        workspace: Option<WorkspaceId>,
        pin: mos_core::RadialPinInput,
    ) -> Result<Vec<mos_core::RadialPin>, CoreError> {
        // Valida ANTES de abrir a transacao, como `set_widget_layout` faz.
        let kind = mos_core::validate_pin_kind(&pin.kind)?;
        let target = pin.target.trim().to_owned();
        if target.is_empty() {
            return Err(CoreError::new(
                mos_core::ErrorCode::InvalidInput,
                "Petala sem alvo.",
                false,
            ));
        }

        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            let escopo = workspace.as_ref().map(|id| id.to_string());
            // Apaga e insere em vez de UPSERT: o indice unico e sobre
            // COALESCE(workspace_id, ''), e um ON CONFLICT nao sabe apontar para
            // indice de expressao sem repetir a expressao inteira.
            connection
                .execute(
                    "DELETE FROM radial_pins
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '') AND slot = ?2",
                    rusqlite::params![escopo, pin.slot],
                )
                .map_err(map_sql_error)?;
            connection
                .execute(
                    "INSERT INTO radial_pins (workspace_id, slot, kind, target, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![escopo, pin.slot, kind, target, agora_iso()],
                )
                .map_err(map_sql_error)?;
        }
        self.radial_pins()
    }

    fn clear_radial_pin(
        &self,
        workspace: Option<WorkspaceId>,
        slot: i64,
    ) -> Result<Vec<mos_core::RadialPin>, CoreError> {
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            let escopo = workspace.as_ref().map(|id| id.to_string());
            // Limpar e APAGAR a linha, e nao gravar um alvo vazio: e o mesmo
            // motivo do `reset_widget_layout` — gravar por cima petrificaria o
            // desenho de hoje, que e o oposto da inversao da 0021.
            connection
                .execute(
                    "DELETE FROM radial_pins
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '') AND slot = ?2",
                    rusqlite::params![escopo, slot],
                )
                .map_err(map_sql_error)?;
        }
        self.radial_pins()
    }
```

Use para `agora_iso()` a mesma função de timestamp que `set_widget_layout` usa neste arquivo — leia a linha do `INSERT` dele e copie a chamada.

- [ ] **Step 6: Exponha no serviço**

Em `crates/mos-core/src/service.rs`, ao lado de `set_widget_layout`. Repasse puro — a regra do leque mora em `leque.ts`, e uma segunda copia aqui e exatamente o que o `homeLayout.ts` conta ter dado errado:

```rust
    pub fn radial_pins(&self) -> Result<Vec<crate::RadialPin>, CoreError> {
        self.repository.radial_pins()
    }

    pub fn set_radial_pin(
        &self,
        workspace_id: Option<&str>,
        pin: crate::RadialPinInput,
    ) -> Result<Vec<crate::RadialPin>, CoreError> {
        let workspace = workspace_id.map(WorkspaceId::parse).transpose()?;
        self.repository.set_radial_pin(workspace, pin)
    }

    pub fn clear_radial_pin(
        &self,
        workspace_id: Option<&str>,
        slot: i64,
    ) -> Result<Vec<crate::RadialPin>, CoreError> {
        let workspace = workspace_id.map(WorkspaceId::parse).transpose()?;
        self.repository.clear_radial_pin(workspace, slot)
    }
```

Use o nome de campo que `set_widget_layout` usa para alcançar o repositório neste arquivo (`self.repository` ou equivalente) — leia a função vizinha e copie a forma.

- [ ] **Step 7: Rode e confirme que passa**

```bash
cargo test -p mos-storage-sqlite petala_grava
```
Esperado: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/
git commit -m "feat(leque): ler, fixar e limpar petala, com Todos no mesmo escopo"
```

---

### Task 3: Os comandos e o cliente

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs:1161-1180` (ao lado de `widget_placements`) e o `invoke_handler`
- Modify: `apps/desktop/src/types.ts`
- Modify: `apps/desktop/src/api.ts:26` e `:138`

**Interfaces:**
- Consumes: `service.radial_pins`, `set_radial_pin`, `clear_radial_pin` (Task 2).
- Produces: `api.radialPins()`, `api.setRadialPin(workspaceId, pin)`, `api.clearRadialPin(workspaceId, slot)`; tipos `RadialPin` e `RadialPinInput` em `types.ts`.

- [ ] **Step 1: Declare os tipos no front**

Em `apps/desktop/src/types.ts`, junto de `WidgetPlacement`:

```ts
/** Uma pétala fixada. `workspaceId` nulo é a visão "Todos" (migration 0021). */
export type RadialPin = {
  workspaceId: string | null;
  slot: number;
  kind: string;
  target: string;
};

export type RadialPinInput = {
  slot: number;
  kind: string;
  target: string;
};
```

- [ ] **Step 2: Escreva os comandos no Rust**

Em `apps/desktop/src-tauri/src/lib.rs`, ao lado de `widget_placements`:

```rust
#[tauri::command]
fn radial_pins(state: tauri::State<'_, AppState>) -> Result<Vec<RadialPin>, CoreError> {
    state.work.radial_pins()
}

#[tauri::command]
fn set_radial_pin(
    state: tauri::State<'_, AppState>,
    workspace_id: Option<String>,
    pin: RadialPinInput,
) -> Result<Vec<RadialPin>, CoreError> {
    state.work.set_radial_pin(workspace_id.as_deref(), pin)
}

#[tauri::command]
fn clear_radial_pin(
    state: tauri::State<'_, AppState>,
    workspace_id: Option<String>,
    slot: i64,
) -> Result<Vec<RadialPin>, CoreError> {
    state.work.clear_radial_pin(workspace_id.as_deref(), slot)
}
```

Siga a assinatura exata de `set_widget_layout` neste arquivo para converter `Option<String>` em `Option<WorkspaceId>` — ele já resolve isso, e repetir a conversão dele evita divergência.

- [ ] **Step 3: Registre os três no `invoke_handler`**

Acrescente `radial_pins,`, `set_radial_pin,` e `clear_radial_pin,` à lista do `tauri::generate_handler![...]`.

- [ ] **Step 4: Escreva o cliente**

Em `apps/desktop/src/api.ts`, junto de `setWidgetLayout`:

```ts
  radialPins() {
    return invoke<RadialPin[]>("radial_pins");
  },
  // `workspaceId` nulo é a visão "Todos", como no arranjo da Home.
  setRadialPin(workspaceId: string | null, pin: RadialPinInput) {
    return invoke<RadialPin[]>("set_radial_pin", { workspaceId, pin });
  },
  // Limpar APAGA a linha e devolve o slot ao desenho — não grava alvo vazio.
  clearRadialPin(workspaceId: string | null, slot: number) {
    return invoke<RadialPin[]>("clear_radial_pin", { workspaceId, slot });
  },
```

Importe `RadialPin` e `RadialPinInput` no topo do arquivo, junto dos tipos já importados.

- [ ] **Step 5: Confirme que compila dos dois lados**

```bash
cd apps/desktop && npx tsc --noEmit
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
```
Esperado: nenhuma saída de erro nos dois.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs apps/desktop/src/types.ts apps/desktop/src/api.ts
git commit -m "feat(leque): comandos e cliente das petalas"
```

---

### Task 4: A regra, em cópia única

**Files:**
- Create: `apps/desktop/src/leque.ts`
- Test: `apps/desktop/src/leque.test.ts`

**Interfaces:**
- Consumes: `RadialPin` de `types.ts` (Task 3).
- Produces: `SLOTS = 5`; `type Petala = { slot: number; kind: PetalaKind; target: string }`; `type PetalaKind = "app" | "acao" | "pagina"`; `PETALAS_DE_FABRICA: Petala[]`; `resolverPetalas(pins: RadialPin[], workspaceId: string | null): Petala[]`; `anguloDaPetala(slot: number): number`; `posicaoDaPetala(slot: number, raio: number): { x: number; y: number }`.

- [ ] **Step 1: Escreva os testes que falham**

Crie `apps/desktop/src/leque.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  PETALAS_DE_FABRICA,
  SLOTS,
  anguloDaPetala,
  posicaoDaPetala,
  resolverPetalas,
} from "./leque";
import type { RadialPin } from "./types";

describe("o padrão de fábrica", () => {
  it("nasce com os cinco slots preenchidos", () => {
    expect(PETALAS_DE_FABRICA).toHaveLength(SLOTS);
    expect(PETALAS_DE_FABRICA.map((p) => p.target)).toEqual([
      "calendario",
      "finance",
      "reunioes",
      "019ffc4f-2936-7152-84b7-672d7bdb5bfc",
      "quick_capture",
    ]);
  });

  it("lista vazia devolve o desenho, e não um leque vazio", () => {
    expect(resolverPetalas([], null)).toEqual(PETALAS_DE_FABRICA);
  });
});

describe("resolverPetalas", () => {
  it("um slot gravado substitui só aquele", () => {
    const pins: RadialPin[] = [
      { workspaceId: null, slot: 1, kind: "acao", target: "attention_create" },
    ];
    const petalas = resolverPetalas(pins, null);
    expect(petalas).toHaveLength(SLOTS);
    expect(petalas[1]).toEqual({ slot: 1, kind: "acao", target: "attention_create" });
    // Os outros quatro continuam sendo o desenho.
    expect(petalas[0]).toEqual(PETALAS_DE_FABRICA[0]);
    expect(petalas[4]).toEqual(PETALAS_DE_FABRICA[4]);
  });

  it("ignora pino de outro Workspace", () => {
    const pins: RadialPin[] = [
      { workspaceId: "outro", slot: 0, kind: "acao", target: "quick_capture" },
    ];
    expect(resolverPetalas(pins, null)).toEqual(PETALAS_DE_FABRICA);
  });

  it("ignora slot fora da faixa que o desenho oferece", () => {
    // O banco aceita 0..11 de propósito; a interface oferece cinco.
    const pins: RadialPin[] = [
      { workspaceId: null, slot: 9, kind: "acao", target: "quick_capture" },
    ];
    expect(resolverPetalas(pins, null)).toEqual(PETALAS_DE_FABRICA);
  });

  it("kind desconhecido cai fora em vez de virar pétala morta", () => {
    const pins: RadialPin[] = [
      { workspaceId: null, slot: 2, kind: "widget3", target: "x" },
    ];
    expect(resolverPetalas(pins, null)[2]).toEqual(PETALAS_DE_FABRICA[2]);
  });
});

describe("a geometria", () => {
  it("os ângulos são simétricos em torno da vertical", () => {
    const angulos = Array.from({ length: SLOTS }, (_, i) => anguloDaPetala(i));
    // -90° é para cima. O arco é simétrico: o primeiro e o último são espelhos.
    expect(angulos[0] + angulos[SLOTS - 1]).toBeCloseTo(-180, 5);
    expect(angulos[2]).toBeCloseTo(-90, 5);
  });

  it("os ângulos são crescentes, da esquerda para a direita", () => {
    for (let i = 1; i < SLOTS; i += 1) {
      expect(anguloDaPetala(i)).toBeGreaterThan(anguloDaPetala(i - 1));
    }
  });

  it("o ângulo de um slot não depende de quantos estão preenchidos", () => {
    // É a razão de o leque existir: o alvo não pode se mover debaixo da mão.
    const antes = anguloDaPetala(3);
    resolverPetalas([{ workspaceId: null, slot: 0, kind: "acao", target: "quick_capture" }], null);
    expect(anguloDaPetala(3)).toBe(antes);
  });

  it("posicaoDaPetala põe o slot do meio direto acima da âncora", () => {
    const { x, y } = posicaoDaPetala(2, 100);
    expect(x).toBeCloseTo(0, 5);
    expect(y).toBeCloseTo(-100, 5);
  });
});
```

- [ ] **Step 2: Rode e confirme que falha**

```bash
cd apps/desktop && npx vitest run src/leque.test.ts
```
Esperado: FAIL — `Failed to resolve import "./leque"`.

- [ ] **Step 3: Escreva o módulo**

Crie `apps/desktop/src/leque.ts`:

```ts
/**
 * O leque: quais pétalas existem, onde o desenho as pôs, e o que a pessoa mudou.
 *
 * ESTA É A ÚNICA CÓPIA DA REGRA, e isso é decisão e não acaso. O `homeLayout.ts`
 * registra o que aconteceu quando a regra do arranjo viveu em dois lugares: a
 * cópia do Rust "ficou para trás em silêncio — com os testes dela passando".
 * O que ficou no core é só o que o BANCO precisa para não aceitar lixo, que é o
 * validador de forma do `kind`.
 *
 * Vive fora do `App.tsx` para poder ser testado: não há teste de DOM neste repo,
 * então o que se verifica tem de ser função pura.
 */
import type { RadialPin } from "./types";

/** Cinco, e o número é a feature.
 *
 *  O leque só é mais rápido que o Ctrl+K enquanto for memória muscular, e
 *  memória muscular exige que o alvo não se mova. Se o número de pétalas
 *  variasse com quantas estão fixadas, cada nova pétala moveria as outras
 *  quatro — e o que sobraria seria um Ctrl+K pior, sem busca. */
export const SLOTS = 5;

export type PetalaKind = "app" | "acao" | "pagina";

export type Petala = {
  slot: number;
  kind: PetalaKind;
  target: string;
};

const KINDS: readonly PetalaKind[] = ["app", "acao", "pagina"];

/** O padrão de fábrica.
 *
 *  Os três primeiros são exatamente os que saíram do rail. Não é conveniência:
 *  a ADR-038 tirou Apps do rail e acrescentou a porta nova NO MESMO commit,
 *  registrando que sem ela "a pagina ficaria inalcancavel". Aqui é a mesma
 *  dívida, paga do mesmo jeito.
 *
 *  O quarto é o M-Finance porque, dos cinco apps cadastrados, ele é o único com
 *  `launch_kind` e `can_open` — os outros quatro dariam pétalas que não fazem
 *  nada. O id é o do registro, e não o nome, porque o nome muda. */
export const PETALAS_DE_FABRICA: Petala[] = [
  { slot: 0, kind: "pagina", target: "calendario" },
  { slot: 1, kind: "pagina", target: "finance" },
  { slot: 2, kind: "pagina", target: "reunioes" },
  { slot: 3, kind: "app", target: "019ffc4f-2936-7152-84b7-672d7bdb5bfc" },
  { slot: 4, kind: "acao", target: "quick_capture" },
];

/**
 * O leque efetivo: o desenho, com os slots que a pessoa trocou por cima.
 *
 * Lista vazia devolve o desenho INTEIRO, e não um leque vazio — é a inversão que
 * a migration 0021 documenta. Trocar um slot não congela os outros quatro, então
 * mudar o padrão de fábrica ainda alcança quem nunca personalizou.
 */
export function resolverPetalas(pins: RadialPin[], workspaceId: string | null): Petala[] {
  const doEscopo = new Map<number, Petala>();
  for (const pin of pins) {
    if ((pin.workspaceId ?? null) !== workspaceId) continue;
    // O banco aceita slot 0..11 de propósito — ele guarda forma. QUANTAS
    // posições a interface oferece é vocabulário, e o vocabulário é este SLOTS.
    if (!Number.isInteger(pin.slot) || pin.slot < 0 || pin.slot >= SLOTS) continue;
    // `kind` é opaco no banco pelo mesmo motivo. Um tipo que este front não
    // conhece cai fora aqui, e o slot volta ao desenho, em vez de virar uma
    // pétala que não sabe o que fazer quando clicada.
    if (!KINDS.includes(pin.kind as PetalaKind)) continue;
    if (!pin.target.trim()) continue;
    doEscopo.set(pin.slot, { slot: pin.slot, kind: pin.kind as PetalaKind, target: pin.target });
  }
  return PETALAS_DE_FABRICA.map((padrao) => doEscopo.get(padrao.slot) ?? padrao);
}

/** Abertura total do arco, em graus. 120° cabe cinco pétalas com folga de toque
 *  sem que as das pontas cheguem à horizontal, onde elas apontariam para o
 *  recibo de desfazer e para o toast de atenção. */
const ARCO = 120;

/**
 * O ângulo de um slot, em graus, com -90 apontando para cima.
 *
 * Depende do slot e de `SLOTS`, e de mais NADA — em particular, não depende de
 * quantas pétalas estão preenchidas. É essa independência que a memória muscular
 * consome, e há um teste só para ela.
 */
export function anguloDaPetala(slot: number): number {
  const passo = ARCO / (SLOTS - 1);
  return -90 - ARCO / 2 + slot * passo;
}

/** O deslocamento da pétala em relação à âncora, em pixels. `y` negativo sobe,
 *  como no sistema de coordenadas da tela. */
export function posicaoDaPetala(slot: number, raio: number): { x: number; y: number } {
  const radianos = (anguloDaPetala(slot) * Math.PI) / 180;
  return { x: raio * Math.cos(radianos), y: raio * Math.sin(radianos) };
}
```

- [ ] **Step 4: Rode e confirme que passa**

```bash
cd apps/desktop && npx vitest run src/leque.test.ts
```
Esperado: PASS, 10 testes.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/leque.ts apps/desktop/src/leque.test.ts
git commit -m "feat(leque): a regra das petalas, em copia unica e testada"
```

---

### Task 5: O rail volta a oito

**Files:**
- Modify: `apps/desktop/src/App.tsx:2999-3042` (a lista `nav` e os `navGroups`)

**Interfaces:**
- Produces: nenhum símbolo novo. As páginas `calendario`, `finance` e `reunioes` continuam existindo em `Page` e no `pageContent` — só saem da navegação lateral.

- [ ] **Step 1: Tire os três da lista `nav`**

Em `apps/desktop/src/App.tsx`, remova de `nav` as três entradas `{ page: "calendario", ... }`, `{ page: "finance", ... }` e `{ page: "reunioes", ... }`, **com os comentários de bloco que as justificavam**. Não apague as entradas de `pageLabels` nem os ramos de `pageContent` — as páginas continuam existindo e alcançáveis.

A lista resultante tem oito, nesta ordem: `home`, `hermes`, `inbox`, `tasks`, `projects`, `workspaces`, `tempo`, `library`.

- [ ] **Step 2: Reagrupe**

Substitua o bloco `navGroups`:

```tsx
  /* Os grupos usam o vocabulario que a ADR-038 fixou ao definir o que e item de
     rail: "Library e memoria, Inbox e a entrada dela, Workspaces e a lente sobre
     tudo, e Tempo e de onde sai a renda".

     Antes eram tres, sete e um. Sete itens sob um rotulo e uma lista, nao um
     grupo — o rotulo para de informar. E Inbox ficava em GERAL, longe da Library
     que ele alimenta, enquanto Workspaces sumia no meio dos sete. */
  const navGroups = [
    { label: "GERAL", items: nav.slice(0, 2) },
    { label: "TRABALHO", items: nav.slice(2, 6) },
    { label: "MEMÓRIA", items: nav.slice(6) },
  ];
```

E reordene `nav` para que os índices batam: `home`, `hermes` · `tasks`, `projects`, `tempo`, `workspaces` · `inbox`, `library`.

- [ ] **Step 3: Confirme que compila e que os testes seguem verdes**

```bash
cd apps/desktop && npx tsc --noEmit && npx vitest run
```
Esperado: sem erro de tipo; 93 testes passando (83 de antes + 10 do `leque.test.ts`).

- [ ] **Step 4: Veja o rail de verdade**

Siga a skill `ver-o-app`: suba `npm run tauri dev`, capture a janela e **olhe a imagem**. Confirme oito ícones em três grupos e que nenhum rótulo de grupo ficou órfão.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(rail): volta a oito, e os grupos passam a significar algo"
```

---

### Task 6: As três portas que faltam na Home

**Files:**
- Modify: `apps/desktop/src/App.tsx:424` (assinatura de `HomePage` — ela vive **dentro** do `App.tsx`, não há `HomePage.tsx`)
- Modify: `apps/desktop/src/App.tsx:716` (widget `quick_actions`)
- Modify: `apps/desktop/src/App.tsx:3050` (chamada de `<HomePage .../>`)

**Interfaces:**
- Consumes: `setPage` do shell.
- Produces: props `openFinancePage: () => void`, `openCalendarPage: () => void`, `openMeetingsPage: () => void` em `HomePage`.

**Onde as portas vão, e por quê.** Nenhum dos três tem widget próprio na Home — os widgets existentes são `now`, `timer`, `recent`, `projects`, `task_progress`, `recent_resources`, `apps`, `quick_actions` e `system_health`. O widget **AÇÕES** (`quick_actions`) já é o lugar de "ir fazer uma coisa": ele hoje carrega Capturar, Nova Task e Novo Project. As três portas entram ali, e não em widgets novos — criar três widgets para três botões seria pagar caro por uma dívida que a ADR-038 pagou com um botão só.

- [ ] **Step 1: Declare as props**

Em `App.tsx:424`, na desestruturação e no tipo de `HomePage`, ao lado de `openAppsPage`, acrescente `openFinancePage`, `openCalendarPage` e `openMeetingsPage` — nos dois lugares, porque a assinatura repete a lista.

No tipo: `openFinancePage: () => void; openCalendarPage: () => void; openMeetingsPage: () => void;`

- [ ] **Step 2: Ligue os botões no widget AÇÕES**

Substitua o `node` do `quick_actions` (linha ~716):

```tsx
        { id: "quick_actions", node: <Panel label="AÇÕES"><div className="quick-actions"><Button variant="outline" size="sm" onClick={() => void api.showQuickCapture()}>Capturar</Button><Button variant="outline" size="sm" onClick={() => openTasksPage()}>Nova Task</Button><Button variant="outline" size="sm" onClick={() => openProjectsPage()}>Novo Project</Button>{/* As tres portas dos destinos que sairam do rail (ADR-045). Entram JUNTO
            com a saida, e nao depois: a ADR-038 registrou que tirar Apps do rail
            sem porta nova deixaria "a pagina inalcancavel", e o leque sozinho nao
            resolve — uma petala pode ser desfixada. */}<Button variant="outline" size="sm" onClick={() => openCalendarPage()}>Calendário</Button><Button variant="outline" size="sm" onClick={() => openFinancePage()}>Finance</Button><Button variant="outline" size="sm" onClick={() => openMeetingsPage()}>Reuniões</Button></div></Panel> },
```

- [ ] **Step 3: Passe as três portas**

Na chamada de `<HomePage ... />` na linha ~3050, junto de `openAppsPage={() => setPage("apps")}`:

```tsx
openFinancePage={() => setPage("finance")} openCalendarPage={() => setPage("calendario")} openMeetingsPage={() => setPage("reunioes")}
```

- [ ] **Step 4: Confirme e veja**

```bash
cd apps/desktop && npx tsc --noEmit && npx vitest run
```

Depois, pela skill `ver-o-app`, fotografe a Home e confirme que os três botões existem e levam à página certa.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(home): as portas de Finance, Calendario e Reunioes"
```

---

### Task 7: O leque na tela

**Files:**
- Create: `apps/desktop/src/Leque.tsx`
- Modify: `apps/desktop/src/App.tsx` (montar dentro de `.main-column`, depois de `<main className="content">`)
- Modify: `apps/desktop/src/App.css` (bloco novo no fim, antes das media queries)

**Interfaces:**
- Consumes: `resolverPetalas`, `posicaoDaPetala`, `SLOTS`, `Petala` (Task 4); `api.radialPins`, `api.clearRadialPin`, `api.setRadialPin` (Task 3).
- Produces: `<Leque pins={RadialPin[]} workspaceId={string | null} apps={RegisteredApp[]} onNavegar={(page: Page) => void} onAbrirApp={(app: RegisteredApp) => void} onAcao={(target: string) => void} onFixar={(slot: number) => void} />`.

- [ ] **Step 1: Escreva o componente**

Crie `apps/desktop/src/Leque.tsx`. Ele **desenha e não decide**: toda a resolução vem de `leque.ts`.

```tsx
import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "./Icon";
import { SLOTS, posicaoDaPetala, resolverPetalas, type Petala } from "./leque";
import type { Page, RadialPin, RegisteredApp } from "./types";

/** O raio do arco, em pixels. Curto o bastante para o leque caber acima da
 *  âncora em 840px sem alcançar a topbar. */
const RAIO = 96;

/**
 * O leque — cinco pétalas fixas, no rodapé ao centro.
 *
 * Existe apesar de o Ctrl+K já ser um lançador universal, e a diferença é
 * evocar contra reconhecer: o Command exige saber o nome e digitá-lo, o leque é
 * memória muscular. Daí sai a única regra que ele não pode quebrar — as pétalas
 * não se reordenam sozinhas, nunca. Um leque que se reorganiza é um Ctrl+K pior.
 *
 * A âncora é `position: absolute` e NÃO ocupa espaço no fluxo: uma faixa
 * permanente roubaria altura de toda página para servir a um gesto. Em troca, a
 * `.page-surface` ganha `padding-bottom`, senão "sobrepor" viraria "esconder".
 */
export function Leque({ pins, workspaceId, apps, onNavegar, onAbrirApp, onAcao, onFixar }: {
  pins: RadialPin[];
  workspaceId: string | null;
  apps: RegisteredApp[];
  onNavegar: (page: Page) => void;
  onAbrirApp: (app: RegisteredApp) => void;
  onAcao: (target: string) => void;
  onFixar: (slot: number) => void;
}) {
  const [aberto, setAberto] = useState(false);
  const raiz = useRef<HTMLDivElement>(null);
  const ancora = useRef<HTMLButtonElement>(null);
  const petalas = resolverPetalas(pins, workspaceId);

  const fechar = useCallback(() => {
    setAberto(false);
    ancora.current?.focus();
  }, []);

  // Esc e clique fora fecham. O foco volta para a âncora, e não some no body:
  // sem isso, fechar por Esc deixaria o teclado no início da página.
  useEffect(() => {
    if (!aberto) return;
    const tecla = (evento: KeyboardEvent) => { if (evento.key === "Escape") fechar(); };
    const fora = (evento: MouseEvent) => {
      if (!raiz.current?.contains(evento.target as Node)) setAberto(false);
    };
    document.addEventListener("keydown", tecla);
    document.addEventListener("mousedown", fora);
    return () => {
      document.removeEventListener("keydown", tecla);
      document.removeEventListener("mousedown", fora);
    };
  }, [aberto, fechar]);

  function disparar(petala: Petala) {
    setAberto(false);
    if (petala.kind === "pagina") { onNavegar(petala.target as Page); return; }
    if (petala.kind === "acao") { onAcao(petala.target); return; }
    const app = apps.find((candidato) => candidato.id === petala.target);
    // App apagado ou arquivado não vira erro: o slot passa a pedir o que fixar,
    // que é a única saída que resolve em vez de avisar.
    if (app) onAbrirApp(app); else onFixar(petala.slot);
  }

  return (
    <div className="leque" ref={raiz} data-aberto={aberto || undefined}>
      <div className="leque-petalas" role="menu" aria-label="Leque" aria-hidden={!aberto}>
        {petalas.map((petala) => {
          const { x, y } = posicaoDaPetala(petala.slot, RAIO);
          return (
            <button
              key={petala.slot}
              type="button"
              role="menuitem"
              className="leque-petala"
              tabIndex={aberto ? 0 : -1}
              style={{ "--x": `${x}px`, "--y": `${y}px`, "--ordem": petala.slot } as React.CSSProperties}
              aria-label={rotuloDaPetala(petala, apps)}
              onClick={() => disparar(petala)}
            >
              <Icon name={iconeDaPetala(petala)} />
            </button>
          );
        })}
      </div>
      <button
        ref={ancora}
        type="button"
        className="leque-ancora"
        aria-expanded={aberto}
        aria-label={aberto ? "Fechar o leque" : "Abrir o leque"}
        onClick={() => setAberto((estava) => !estava)}
      >
        <Icon name="more" />
      </button>
    </div>
  );
}

function iconeDaPetala(petala: Petala) {
  if (petala.kind === "acao") return "capture" as const;
  if (petala.kind === "app") return "apps" as const;
  if (petala.target === "calendario") return "calendar" as const;
  if (petala.target === "finance") return "finance" as const;
  if (petala.target === "reunioes") return "meetings" as const;
  return "more" as const;
}

function rotuloDaPetala(petala: Petala, apps: RegisteredApp[]) {
  if (petala.kind === "app") {
    return apps.find((candidato) => candidato.id === petala.target)?.name ?? "Escolher o que fixar";
  }
  if (petala.kind === "acao") return "Quick Capture";
  const nomes: Record<string, string> = { calendario: "Calendário", finance: "Finance", reunioes: "Reuniões" };
  return nomes[petala.target] ?? petala.target;
}
```

Nota para quem implementa: `SLOTS` é importado para o leitor saber de onde vem o número, mas `petalas.map` já tem cinco itens — se o lint reclamar de import não usado, tire o `SLOTS` do import em vez de inventar uso para ele.

- [ ] **Step 2: Escreva o CSS**

No fim de `apps/desktop/src/App.css`, antes do bloco `@media (max-width: 960px)`:

```css
/* --- O leque ------------------------------------------------------------- */

/* `absolute` e nao um lugar no fluxo: uma faixa permanente roubaria altura de
   TODA pagina para servir a um gesto. O par obrigatorio disso e o
   `padding-bottom` da `.page-surface` logo abaixo — sem ele, "sobrepor" vira
   "esconder", e o fim de uma lista longa mora debaixo da ancora. */
.leque {
  position: absolute;
  bottom: var(--space-4);
  left: 50%;
  transform: translateX(-50%);
  /* Acima do recibo de desfazer e do toast de atencao. O criterio nao e
     hierarquia visual, e intencao: o leque so esta aberto porque alguem acabou
     de clicar nele, enquanto recibo e toast aparecem sozinhos. */
  z-index: calc(var(--z-receipt) + 1);
}

.page-surface {
  padding-bottom: calc(var(--height-control-lg) + var(--space-6));
}

.leque-ancora {
  display: grid;
  place-items: center;
  width: var(--height-control-lg);
  height: var(--height-control-lg);
  color: var(--on-signal);
  background: var(--signal-fill);
  border: 0;
  border-radius: 50%;
  box-shadow: var(--shadow-overlay);
}

.leque-petalas {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.leque[data-aberto] .leque-petalas {
  pointer-events: auto;
}

.leque-petala {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  width: var(--height-control);
  height: var(--height-control);
  margin: auto;
  color: var(--text);
  background: var(--surface-raised);
  border: var(--line) solid var(--border-strong);
  border-radius: 50%;
  opacity: 0;
  transform: translate3d(0, 0, 0) scale(0.6);
  transition:
    transform var(--dur-enter) var(--ease-enter),
    opacity var(--dur-instant) linear;
}

.leque[data-aberto] .leque-petala {
  opacity: 1;
  transform: translate3d(var(--x), var(--y), 0) scale(1);
  /* Escalonado a partir da ancora: o olho segue a abertura em vez de receber
     cinco pontos de uma vez. Dentro do orcamento da ADR-034. */
  transition-delay: calc(var(--ordem) * 20ms);
}

.leque-petala:hover {
  border-color: var(--signal-ink);
}

/* O percurso e o que se corta, e nao a presenca: as petalas continuam
   aparecendo, so nao viajam. */
@media (prefers-reduced-motion: reduce) {
  .leque-petala {
    transition: opacity var(--dur-instant) linear;
    transform: translate3d(var(--x), var(--y), 0);
  }
  .leque[data-aberto] .leque-petala {
    transition-delay: 0ms;
  }
}
```

- [ ] **Step 3: Monte no shell**

Em `App.tsx`, dentro de `.main-column`, logo **depois** do `</main>` e antes do fechamento da `div`:

```tsx
<Leque pins={radialPins} workspaceId={currentWorkspaceId ?? null} apps={apps} onNavegar={navigate} onAbrirApp={openRegisteredApp} onAcao={(target) => { if (target === "quick_capture") void api.showQuickCapture(); else if (target === "attention_create") setComposerOpen(true); }} onFixar={(slot) => setSlotEmEscolha(slot)} />
```

Acrescente o estado `const [radialPins, setRadialPins] = useState<RadialPin[]>([]);` junto dos outros estados do shell, e carregue em `refresh` com `api.radialPins().then(setRadialPins).catch(() => undefined)`.

**ATENÇÃO — leia o comentário de `pageContent` na linha ~3062 antes deste passo.** Ele registra que a lista de dependências do `useMemo` é manual e sem lint, e que um estado novo que não entre nela deixa a tela CONGELADA. `radialPins` não é usado dentro de `pageContent`, então não entra lá — mas confirme isso lendo, e não presumindo.

- [ ] **Step 4: Confirme que compila**

```bash
cd apps/desktop && npx tsc --noEmit && npx vitest run
```
Esperado: sem erro; 93 testes passando.

- [ ] **Step 5: Veja de verdade — este é o gate**

Pela skill `ver-o-app`, com `export TMP`/`TEMP` antes do PowerShell:

1. capture a janela em **1280** e em **840** de largura, com o leque **fechado** e **aberto**;
2. capture nos **dois temas** (force `data-theme` por HMR, como já foi feito antes, e reverta);
3. **olhe cada imagem** com a ferramenta Read — a foto é a única prova de aparência;
4. confirme: as cinco pétalas não encostam umas nas outras, nenhuma alcança a topbar em 840px, e o arco não cobre o recibo quando ele está visível;
5. navegue só por teclado: Tab até a âncora, Enter abre, setas percorrem, Enter dispara, Esc fecha e o foco volta à âncora.

Se qualquer um dos cinco falhar, corrija antes de comitar. Não comite com "provavelmente está bom".

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/Leque.tsx apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(leque): as cinco petalas no rodape, com o gesto que o rail perdeu"
```

---

### Task 8: O seletor — sem ele ninguém escolhe nada

**Files:**
- Create: `apps/desktop/src/LequeSeletor.tsx`
- Modify: `apps/desktop/src/App.tsx` (estado `slotEmEscolha` e montagem)
- Modify: `apps/desktop/src/App.css` (reaproveita `.meeting-scrim`; só o corpo é novo)

**Interfaces:**
- Consumes: `api.setRadialPin`, `api.clearRadialPin` (Task 3); `PetalaKind` (Task 4).
- Produces: `<LequeSeletor slot={number} workspaceId={string | null} apps={RegisteredApp[]} onGravado={(pins: RadialPin[]) => void} onFechar={() => void} />`.

**Por que esta tarefa existe.** A decisão do proprietário foi *"fixas, escolhidas por você"*, e a §2 da spec explica que a escolha manual é o que separa o leque de um Ctrl+K pior. Sem seletor, o leque é apenas fixo — ninguém escolhe, e o padrão de fábrica vira camisa de força.

- [ ] **Step 1: Escreva o seletor**

Crie `apps/desktop/src/LequeSeletor.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import type { PetalaKind } from "./leque";
import type { RadialPin, RegisteredApp } from "./types";

/** Os destinos que o leque sabe abrir. Fica AQUI, e não em `leque.ts`, porque é
 *  vocabulário de interface: `leque.ts` guarda a regra, esta lista o cardápio. */
const PAGINAS: { target: string; nome: string }[] = [
  { target: "calendario", nome: "Calendário" },
  { target: "finance", nome: "Finance" },
  { target: "reunioes", nome: "Reuniões" },
  { target: "tempo", nome: "Tempo" },
  { target: "apps", nome: "Apps" },
  { target: "library", nome: "Library" },
];

const ACOES: { target: string; nome: string }[] = [
  { target: "quick_capture", nome: "Quick Capture" },
  { target: "attention_create", nome: "Novo lembrete" },
];

/**
 * O que fixar num slot.
 *
 * Só troca o CONTEÚDO de um slot; a posição não se mexe, e isso é a feature —
 * mover pétala moveria o alvo debaixo da mão, que é o que o leque existe para
 * não fazer.
 */
export function LequeSeletor({ slot, workspaceId, apps, onGravado, onFechar }: {
  slot: number;
  workspaceId: string | null;
  apps: RegisteredApp[];
  onGravado: (pins: RadialPin[]) => void;
  onFechar: () => void;
}) {
  const [erro, setErro] = useState("");
  const [gravando, setGravando] = useState(false);
  const corpo = useRef<HTMLDivElement>(null);

  // O foco entra no primeiro botão do corpo. Não uso `ref` no `Button` porque
  // ele é função simples e não repassa ref — e envolvê-lo em `forwardRef` seria
  // mexer num componente compartilhado por uma necessidade local desta tela.
  useEffect(() => { corpo.current?.querySelector("button")?.focus(); }, []);
  useEffect(() => {
    const tecla = (evento: KeyboardEvent) => { if (evento.key === "Escape") onFechar(); };
    document.addEventListener("keydown", tecla);
    return () => document.removeEventListener("keydown", tecla);
  }, [onFechar]);

  async function fixar(kind: PetalaKind, target: string) {
    setGravando(true);
    setErro("");
    try {
      onGravado(await api.setRadialPin(workspaceId, { slot, kind, target }));
      onFechar();
    } catch (causa) {
      // O erro fica NO seletor: mandar a pessoa procurar o motivo em outra tela
      // desfaz o motivo de o seletor existir.
      setErro(causa instanceof Error ? causa.message : String(causa));
      setGravando(false);
    }
  }

  async function devolverAoDesenho() {
    setGravando(true);
    setErro("");
    try {
      // APAGA a linha em vez de gravar vazio: é a inversão da 0021, e é o que
      // faz o slot voltar a seguir o padrão em vez de congelar no de hoje.
      onGravado(await api.clearRadialPin(workspaceId, slot));
      onFechar();
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
      setGravando(false);
    }
  }

  const abriveis = apps.filter((app) => app.lifecycleState === "active" && app.canOpen);

  return (
    <div
      className="meeting-scrim"
      role="dialog"
      aria-modal="true"
      aria-label={"Fixar na posição " + String(slot + 1)}
      onMouseDown={(evento) => { if (evento.target === evento.currentTarget) onFechar(); }}
    >
      <div className="leque-seletor" ref={corpo}>
        <span className="micro-label">POSIÇÃO {slot + 1} DE 5</span>

        <span className="micro-label">PÁGINAS</span>
        <div className="leque-seletor-grade">
          {PAGINAS.map((pagina) => (
            <Button key={pagina.target} variant="outline" size="sm" disabled={gravando}
                    onClick={() => void fixar("pagina", pagina.target)}>{pagina.nome}</Button>
          ))}
        </div>

        <span className="micro-label">AÇÕES</span>
        <div className="leque-seletor-grade">
          {ACOES.map((acao) => (
            <Button key={acao.target} variant="outline" size="sm" disabled={gravando}
                    onClick={() => void fixar("acao", acao.target)}>{acao.nome}</Button>
          ))}
        </div>

        <span className="micro-label">APPS</span>
        <div className="leque-seletor-grade">
          {abriveis.map((app) => (
            <Button key={app.id} variant="outline" size="sm" disabled={gravando}
                    onClick={() => void fixar("app", app.id)}>{app.name}</Button>
          ))}
          {/* Dos cinco apps cadastrados, só o M-Finance tem alvo de abertura. Um
              app sem `canOpen` daria uma pétala que não faz nada quando clicada,
              então ele não entra na lista — e a lista vazia diz por quê. */}
          {!abriveis.length ? <p className="support-copy">Nenhum app com abertura configurada. Cadastre um alvo em Apps para ele aparecer aqui.</p> : null}
        </div>

        {erro ? <p className="support-copy" role="alert">{erro}</p> : null}

        <div className="form-actions">
          <Button variant="ghost" disabled={gravando} onClick={() => void devolverAoDesenho()}>Voltar ao padrão</Button>
          <Button variant="ghost" disabled={gravando} onClick={onFechar}>Cancelar</Button>
        </div>
      </div>
    </div>
  );
}
```

Confirme em `types.ts` que `RegisteredApp` tem mesmo o campo `canOpen` (o banco tem `can_open`, e o `serde` do projeto usa `camelCase`). Se o nome for outro, use o do tipo — não invente um alias.

- [ ] **Step 2: Escreva o CSS**

No fim de `App.css`, junto do bloco do leque:

```css
.leque-seletor {
  display: grid;
  gap: var(--space-3);
  width: min(28rem, calc(100vw - var(--space-6)));
  max-height: calc(100vh - var(--space-6));
  overflow-y: auto;
  padding: var(--space-4);
  background: var(--surface-raised);
  border: var(--line) solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-overlay);
}

/* `auto-fit` e nao um numero fixo de colunas: a lista de apps cresce com o
   cadastro, e uma grade fixa deixaria buraco com um app e estouro com sete. */
.leque-seletor-grade {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
  gap: var(--space-2);
}
```

- [ ] **Step 3: Monte no shell**

Em `App.tsx`, junto dos outros estados do shell:

```tsx
const [slotEmEscolha, setSlotEmEscolha] = useState<number | null>(null);
```

E junto das outras sobreposições, no fim do `return`:

```tsx
{slotEmEscolha !== null ? <LequeSeletor slot={slotEmEscolha} workspaceId={currentWorkspaceId ?? null} apps={apps} onGravado={setRadialPins} onFechar={() => setSlotEmEscolha(null)} /> : null}
```

- [ ] **Step 4: Confirme que compila**

```bash
cd apps/desktop && npx tsc --noEmit && npx vitest run
```
Esperado: sem erro; 93 testes passando.

- [ ] **Step 5: Exercite de verdade**

Pela skill `ver-o-app`, com o app rodando:

1. abra o leque e clique numa pétala — ela leva ao destino;
2. abra o seletor e fixe uma página num slot;
3. feche e reabra o app: a escolha sobreviveu;
4. use "Voltar ao padrão" e confirme que o slot volta ao de fábrica;
5. confira no banco que a linha existe e depois some:

```bash
python -c "import sqlite3,os; db=os.path.expandvars(r'%APPDATA%\com.codedbym.mos\m-os.db'); con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); print(con.execute('select * from radial_pins').fetchall())"
```

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/LequeSeletor.tsx apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(leque): o seletor, que e a metade escolhida da decisao"
```

---

### Task 9: A ADR-045 e o índice duplicado

**Files:**
- Modify: `docs/DECISIONS.md` (tabela de índice, ~linha 60, e corpo no fim)

- [ ] **Step 1: Corrija a linha duplicada**

Na tabela de índice, a linha `| ADR-044 | O rail vai a doze, e Reuniões entra sem tirar ninguém | Accepted |` aparece **duas vezes**. Apague uma.

- [ ] **Step 2: Acrescente a ADR-045 ao índice**

```markdown
| ADR-045 | O rail volta a oito, e o recém-chegado nasce no leque | Accepted |
```

- [ ] **Step 3: Escreva a ADR**

No fim do `docs/DECISIONS.md`, seguindo a estrutura das vizinhas (Contexto, Decisão, Consequências, e uma seção "Por que não X"). O argumento central, que precisa estar escrito:

O teto do rail foi de seis a oito (ADR-031), nove (036), dez (038), onze (039) e doze (044) — cinco revisões em pouco mais de duas semanas. Cada uma argumentou bem o seu caso; nenhuma segurou o conjunto, porque o teto era um número e não um caminho. A regra nova: **destino novo nasce no leque, e só sobe ao rail quando provar ser renda ou memória**, pelo critério que a própria ADR-036 escreveu. O leque deixa de ser só um gesto e vira o degrau que faltava entre "existe" e "mora no rail".

Registre também, como consequência aceita e não como omissão:

- **Reuniões sai com dois dias de vida**, sem evidência de uso — o oposto do que a ADR-038 fez com Apps, que saiu porque o banco provava zero cadastros. Mitigação em duas camadas: nasce fixada no leque, e a barra de gravação continua na topbar, onde mora a promessa da §17.2 do `MEETING-AGENT.md`.
- **Cinco é teto, não ponto de partida.** Um leque que cresce vira um segundo rail, e aí o problema apenas mudou de lugar.
- **O que esta ADR não consegue prever:** se o leque de fato substitui o rail no uso diário. Reveja em uma semana de uso real; se não substituir, o caminho de volta é promover ao rail o que estiver sendo mais tocado no leque.

- [ ] **Step 4: Commit**

```bash
git add docs/DECISIONS.md
git commit -m "docs: ADR-045, o rail volta a oito e o recem-chegado nasce no leque"
```

---

### Task 10: Fechamento

- [ ] **Step 1: Rode tudo**

```bash
export TMP="$SCRATCH/tmp"; export TEMP="$TMP"
cargo test -p mos-core && cargo test -p mos-storage-sqlite
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cd apps/desktop && npx tsc --noEmit && npx vitest run
```
Esperado: tudo verde, clippy sem avisos, 93 testes no renderer.

- [ ] **Step 2: Confirme a migration num banco de verdade**

```bash
python -c "import sqlite3,os; db=os.path.expandvars(r'%APPDATA%\com.codedbym.mos\m-os.db'); con=sqlite3.connect(f'file:{db}?mode=ro',uri=True); print(con.execute('PRAGMA user_version').fetchone()); print(con.execute(\"select name from sqlite_master where name='radial_pins'\").fetchall())"
```
Esperado: `(21,)` e `[('radial_pins',)]`, **depois** de abrir o app uma vez. Antes disso o banco ainda está em 20, e isso é o esperado, não um erro.

- [ ] **Step 3: Relate honestamente**

No resumo final, diga o que foi verificado por foto e o que não foi. Em particular: o gate visual roda a 100% de escala; 125% e 150% não foram observados.

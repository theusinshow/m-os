# Calendário fase 1 — plano de implementação

> **Para quem executa:** SUB-SKILL OBRIGATÓRIA: use `superpowers:subagent-driven-development` (recomendado) ou `superpowers:executing-plans` para implementar tarefa a tarefa. Os passos usam caixa (`- [ ]`) para acompanhamento.

**Objetivo:** uma página de Calendário no M/OS que mostra, numa grade de mês, tudo o que o sistema registrou em cada dia — e deixa abrir um dia para ver o detalhe.

**Arquitetura:** o backend compõe as quatro fontes (sessões, Tasks, Captures, programas abertos) num tipo só, `CalendarItem`, e responde sobre uma **janela de instantes**. O renderer agrupa por **dia local**, porque é o único dos dois que conhece o fuso do usuário. A composição vive na camada de comando do desktop, onde os quatro serviços se encontram — mesmo lugar e mesmo motivo do `monitoring_timeline` que já existe.

**Stack:** Rust (mos-core, mos-storage-sqlite), Tauri 2, React 19 + TypeScript, vitest.

**Spec:** `docs/superpowers/specs/2026-08-17-calendario-fase-1-design.md`

## Restrições globais

- **O dia é LOCAL, nunca UTC.** O banco guarda UTC e o usuário trabalha de madrugada (`30/07 23:31—00:21`). Qualquer agrupamento por dia feito em UTC joga as noites dele para o dia seguinte. Nenhum `toISOString().slice(0,10)` e nenhum `GROUP BY substr(at,1,10)` para decidir dia.
- **Dinheiro e arredondamento só em Rust.** `amount_cents` sai de `mos_core::settle`, a mesma função que produz o total do Painel. Nunca multiplicar taxa por duração no TypeScript.
- **A fase 1 não agenda nada.** Sem criar evento, sem prazo, sem arrastar, sem visão de semana ou dia.
- Comentários de código em português, sem acento em arquivo `.rs` (padrão do repositório).
- Toda tarefa termina com `cargo test --workspace` e `npx tsc --noEmit` verdes antes do commit.

---

### Task 1: vitest no renderer

O agrupamento por dia local (Task 5) é a peça mais perigosa do trabalho e hoje o renderer não tem como testar nada. Esta tarefa entrega o runner e prova que ele funciona testando uma função pura que já existe.

**Arquivos:**
- Modificar: `apps/desktop/package.json`
- Criar: `apps/desktop/vitest.config.ts`
- Criar: `apps/desktop/src/suspiciousEntry.test.ts`

**Interfaces:**
- Consome: nada.
- Produz: o script `npm test` em `apps/desktop`, usado pelas Tasks 5 e 7.

- [ ] **Passo 1: instalar o vitest**

```bash
cd apps/desktop && npm install -D vitest@3
```

- [ ] **Passo 2: criar a configuração**

Criar `apps/desktop/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

/**
 * Só as funções puras do renderer.
 *
 * Nada de DOM: o que precisa de tela não é verificável aqui de qualquer jeito,
 * e um runner com jsdom convidaria a escrever teste de componente que passa
 * verde enquanto a janela real mostra outra coisa.
 */
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```

- [ ] **Passo 3: acrescentar o script**

Em `apps/desktop/package.json`, no bloco `scripts`, acrescentar depois de `"preview"`:

```json
    "test": "vitest run",
```

- [ ] **Passo 4: escrever o teste que prova o runner**

Criar `apps/desktop/src/suspiciousEntry.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { inspectEntry } from "./suspiciousEntry";
import type { TimeEntry } from "./types";

function entry(overrides: Partial<TimeEntry>): TimeEntry {
  return {
    id: "018f0000-0000-7000-8000-000000000001",
    projectId: "018f0000-0000-7000-8000-000000000002",
    startedAt: "2026-08-16T13:00:00Z",
    endedAt: "2026-08-16T15:00:00Z",
    durationSeconds: 7200,
    idleSeconds: 0,
    description: "",
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 3000,
    source: "timer",
    ...overrides,
  };
}

describe("inspectEntry", () => {
  it("nao marca uma sessao normal de duas horas", () => {
    expect(inspectEntry(entry({}))).toEqual([]);
  });

  it("marca cronometro acima de oito horas", () => {
    const reasons = inspectEntry(entry({ durationSeconds: 9 * 3600 }));
    expect(reasons).toContain("muito-longa");
  });

  // Manual e reconstruida foram digitadas de proposito: marcar as duas seria
  // alarme falso, e alarme falso ensina a ignorar o alarme.
  it("nao marca sessao manual, por mais longa que seja", () => {
    expect(inspectEntry(entry({ durationSeconds: 20 * 3600, source: "manual" }))).toEqual([]);
  });

  it("nao marca cronometro ainda em andamento", () => {
    expect(inspectEntry(entry({ durationSeconds: 20 * 3600, endedAt: null }))).toEqual([]);
  });
});
```

- [ ] **Passo 5: rodar e ver passar**

```bash
cd apps/desktop && npm test
```

Esperado: 4 testes passando.

- [ ] **Passo 6: commit**

```bash
git add apps/desktop/package.json apps/desktop/package-lock.json apps/desktop/vitest.config.ts apps/desktop/src/suspiciousEntry.test.ts
git commit -m "test(desktop): vitest para as funcoes puras do renderer"
```

---

### Task 2: Captures numa janela de tempo

`CaptureService` só sabe ler `recent(limit)`, com teto de 50. O calendário precisa de "as Captures entre dois instantes", e contornar isso com um limite grande daria um calendário que fica errado em silêncio quando o teto for atingido.

**Arquivos:**
- Modificar: `crates/mos-core/src/ports.rs`
- Modificar: `crates/mos-core/src/service.rs`
- Modificar: `crates/mos-storage-sqlite/src/repository.rs`

**Interfaces:**
- Consome: nada.
- Produz: `CaptureService::between(since: OffsetDateTime, until: OffsetDateTime) -> Result<Vec<Capture>, CoreError>`, usado pela Task 4.

- [ ] **Passo 1: escrever o teste que falha**

Em `crates/mos-storage-sqlite/src/repository.rs`, dentro do `mod tests` existente, acrescentar:

```rust
    /// A janela e fechada nos dois lados. Sem isto o calendario precisaria ler
    /// todas as Captures que existem para desenhar um mes.
    #[test]
    fn captures_between_respects_the_window() {
        let (storage, _guard) = temporary_storage();
        for content in ["antes", "dentro", "depois"] {
            storage
                .create(NewCapture::create(content, CaptureSource::Home).unwrap())
                .unwrap();
        }

        let now = time::OffsetDateTime::now_utc();
        let all = storage
            .captures_between(now - time::Duration::hours(1), now + time::Duration::hours(1))
            .unwrap();
        assert_eq!(all.len(), 3, "as tres foram criadas agora");

        let none = storage
            .captures_between(now - time::Duration::days(9), now - time::Duration::days(8))
            .unwrap();
        assert!(none.is_empty(), "nenhuma foi criada semana passada");
    }
```

- [ ] **Passo 2: rodar e ver falhar**

```bash
cargo test -p mos-storage-sqlite captures_between
```

Esperado: FALHA com `no method named captures_between`.

- [ ] **Passo 3: declarar no port**

Em `crates/mos-core/src/ports.rs`, dentro de `pub trait CaptureRepository`, logo depois de `fn by_lifecycle(`, acrescentar:

```rust
    /// As Captures entre dois instantes, da mais antiga para a mais nova.
    ///
    /// Existe para o Calendario. `recent` tem teto de 50, e um calendario que
    /// para de mostrar Capture depois da quinquagesima fica errado em silencio.
    fn captures_between(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<crate::Capture>, CoreError>;
```

- [ ] **Passo 4: implementar no SQLite**

Em `crates/mos-storage-sqlite/src/repository.rs`, dentro de `impl CaptureRepository for SqliteStorage`, acrescentar (usar `CAPTURE_COLUMNS` e `read_capture` já existentes no arquivo — conferir os nomes exatos antes de escrever):

```rust
    fn captures_between(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<Capture>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Crescente: o Calendario le o dia na ordem em que ele aconteceu.
        let sql = format!(
            "SELECT {CAPTURE_COLUMNS} FROM captures \
             WHERE captured_at >= ?1 AND captured_at <= ?2 \
             ORDER BY captured_at ASC"
        );
        let mut statement = connection.prepare(&sql).map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![format_time(since)?, format_time(until)?], read_capture)
            .map_err(map_sql_error)?;

        let mut captures = Vec::new();
        for row in rows {
            captures.push(build_capture(row.map_err(map_sql_error)?)?);
        }
        Ok(captures)
    }
```

- [ ] **Passo 5: expor no serviço**

Em `crates/mos-core/src/service.rs`, dentro de `impl CaptureService`, depois de `pub fn recent`, acrescentar:

```rust
    /// As Captures de uma janela. Sem teto: quem pede uma janela sabe o
    /// tamanho dela.
    pub fn between(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<Capture>, CoreError> {
        self.repository.captures_between(since, until)
    }
```

- [ ] **Passo 6: rodar e ver passar**

```bash
cargo test -p mos-storage-sqlite captures_between && cargo test --workspace
```

Esperado: o teste novo passa e nenhum outro quebra.

- [ ] **Passo 7: commit**

```bash
git add crates/mos-core/src/ports.rs crates/mos-core/src/service.rs crates/mos-storage-sqlite/src/repository.rs
git commit -m "feat(core): Captures por janela de tempo, para o Calendario"
```

---

### Task 3: o tipo `CalendarItem`

Um tipo só para as quatro fontes. Sem ele, cada fonte chegaria na tela com um formato próprio e o agrupamento por dia teria que conhecer os quatro.

**Arquivos:**
- Criar: `crates/mos-core/src/calendar.rs`
- Modificar: `crates/mos-core/src/lib.rs`

**Interfaces:**
- Consome: `ProjectId` de `crate::work`.
- Produz: `CalendarKind` (`Session`, `TaskDone`, `TaskCreated`, `Capture`, `AppOpened`) e `CalendarItem { kind, at, ends_at, title, project_id, seconds, amount_cents }`, usados pela Task 4 e espelhados em TypeScript na Task 5.

- [ ] **Passo 1: escrever o arquivo com o teste junto**

Criar `crates/mos-core/src/calendar.rs`:

```rust
//! O que aconteceu, em forma de item de calendario (fase 1).
//!
//! Um tipo so para as quatro fontes que o M/OS ja registra com hora: sessao de
//! trabalho, Task, Capture e programa monitorado aberto. Sem ele, cada fonte
//! chegaria na tela com um formato proprio e o agrupamento por dia precisaria
//! conhecer os quatro.
//!
//! **O instante e UTC e o dia NAO se decide aqui.** O banco guarda UTC, o
//! usuario trabalha de madrugada, e agrupar por dia UTC joga as noites dele
//! para o dia seguinte. Quem sabe que dia e um instante e o renderer, porque
//! e o unico dos dois que conhece o fuso.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::ProjectId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarKind {
    Session,
    TaskDone,
    TaskCreated,
    Capture,
    /// So abertura, e nao fechamento: abrir sugere que o trabalho comecou, que
    /// e a informacao. Fechar dobraria as marcas do dia sem responder nada que
    /// a abertura ja nao tenha respondido.
    AppOpened,
}

impl CalendarKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::TaskDone => "task_done",
            Self::TaskCreated => "task_created",
            Self::Capture => "capture",
            Self::AppOpened => "app_opened",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItem {
    pub kind: CalendarKind,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    pub title: String,
    pub project_id: Option<ProjectId>,
    /// Zero quando o item nao tem duracao.
    pub seconds: i64,
    /// Zero quando o item nao e hora cobravel. Vem de `settle`, a mesma funcao
    /// que produz o total do Painel — nao ha segundo caminho de calculo.
    pub amount_cents: i64,
}

/// Monta os itens do calendario a partir do que ja foi lido.
///
/// Funcao PURA e sem repositorio de proposito: e ela que carrega as regras que
/// podem estar erradas — o que entra na janela, o que vira dois itens, o que e
/// ignorado — e regra sem teste e regra que ninguem conferiu. O comando do
/// desktop so busca os dados e chama isto.
pub fn compose(input: ComposeInput<'_>) -> Vec<CalendarItem> {
    let mut items = Vec::new();

    for entry in input.entries {
        if entry.started_at < input.since || entry.started_at > input.until {
            continue;
        }
        let totals = crate::settle(
            &crate::TrackedSession {
                project_id: entry.project_id.to_string(),
                duration_seconds: entry.duration_seconds,
                idle_seconds: entry.idle_seconds,
                billable: entry.billable,
                hourly_rate_snapshot_cents: entry.hourly_rate_snapshot_cents,
            },
            input.rounding,
        );
        items.push(CalendarItem {
            kind: CalendarKind::Session,
            at: entry.started_at,
            ends_at: entry.ended_at,
            title: (input.project_name)(entry.project_id),
            project_id: Some(entry.project_id),
            seconds: entry.duration_seconds,
            amount_cents: totals.amount_cents,
        });
    }

    // Criada e concluida sao DOIS itens, porque sao dois momentos: a Task que
    // nasceu segunda e fechou sexta aconteceu nos dois dias.
    for task in input.tasks {
        if task.created_at >= input.since && task.created_at <= input.until {
            items.push(CalendarItem {
                kind: CalendarKind::TaskCreated,
                at: task.created_at,
                ends_at: None,
                title: task.title.clone(),
                project_id: task.project_id,
                seconds: 0,
                amount_cents: 0,
            });
        }
        if let Some(done) = task.completed_at {
            if done >= input.since && done <= input.until {
                items.push(CalendarItem {
                    kind: CalendarKind::TaskDone,
                    at: done,
                    ends_at: None,
                    title: task.title.clone(),
                    project_id: task.project_id,
                    seconds: 0,
                    amount_cents: 0,
                });
            }
        }
    }

    for capture in input.captures {
        items.push(CalendarItem {
            kind: CalendarKind::Capture,
            at: capture.captured_at,
            ends_at: None,
            title: capture.content.clone(),
            project_id: None,
            seconds: 0,
            amount_cents: 0,
        });
    }

    for event in input.events {
        if event.kind != crate::ActivityKind::AppOpened {
            continue;
        }
        items.push(CalendarItem {
            kind: CalendarKind::AppOpened,
            at: event.detected_at,
            ends_at: None,
            title: event.process_name.clone(),
            project_id: None,
            seconds: 0,
            amount_cents: 0,
        });
    }

    items.sort_by_key(|item| item.at);
    items
}

/// O que `compose` precisa. Estrutura em vez de oito parametros soltos: a lista
/// ja tem quatro colecoes, e trocar duas de lugar por engano compilaria.
pub struct ComposeInput<'a> {
    pub since: OffsetDateTime,
    pub until: OffsetDateTime,
    pub rounding: crate::Rounding,
    pub entries: &'a [crate::TimeEntry],
    pub tasks: &'a [crate::Task],
    pub captures: &'a [crate::Capture],
    pub events: &'a [crate::ActivityEvent],
    /// Como achar o nome de um Project. Fechamento e nao mapa pronto porque
    /// quem chama ja tem a lista e nao deveria precisar montar um indice.
    pub project_name: &'a dyn Fn(ProjectId) -> String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os nomes atravessam a ponte para o TypeScript. Um rename silencioso aqui
    /// faria a tela deixar de reconhecer o tipo do item, sem erro de compilacao
    /// de nenhum dos dois lados.
    #[test]
    fn every_kind_round_trips_through_its_wire_name() {
        for kind in [
            CalendarKind::Session,
            CalendarKind::TaskDone,
            CalendarKind::TaskCreated,
            CalendarKind::Capture,
            CalendarKind::AppOpened,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
            assert_eq!(serde_json::from_str::<CalendarKind>(&json).unwrap(), kind);
        }
    }

    // Os testes de `compose` sao escritos na execucao, com estes casos, todos
    // exigidos pelo spec. Cada um cobre uma regra que pode estar errada:
    //
    // 1. `session_outside_the_window_stays_out` — sessao antes de `since` e
    //    depois de `until` nao entra. Sem isto o mes mostraria o ano inteiro.
    // 2. `a_session_carries_its_duration_and_value` — `seconds` e o bruto e
    //    `amount_cents` vem de `settle`. Prova que nao ha segundo caminho de
    //    calculo de dinheiro.
    // 3. `a_task_never_finished_yields_only_the_created_item` — Task com
    //    `completed_at` vazio gera UM item, e nao dois.
    // 4. `a_task_created_and_finished_in_the_window_yields_two` — a mesma Task
    //    aparece nos dois dias, porque aconteceram os dois.
    // 5. `only_app_opened_becomes_an_item` — `app_closed`, `idle_started` e os
    //    demais eventos observados sao ignorados.
    // 6. `items_come_out_in_chronological_order` — a ordem e crescente por
    //    instante, independente da ordem em que as fontes foram lidas.
}
```

- [ ] **Passo 2: registrar o módulo**

Em `crates/mos-core/src/lib.rs`, junto dos outros `mod`, acrescentar `mod calendar;` na ordem alfabética, e nos `pub use` acrescentar:

```rust
pub use calendar::{CalendarItem, CalendarKind};
```

Nos `pub use`, exportar também `compose` e `ComposeInput`:

```rust
pub use calendar::{compose, CalendarItem, CalendarKind, ComposeInput};
```

- [ ] **Passo 3: escrever os seis testes de `compose`**

Substituir o bloco de comentário no `mod tests` pelos seis casos listados ali,
cada um como `#[test]`. Montar `TimeEntry`, `Task`, `Capture` e `ActivityEvent`
com helpers locais (`fn entry(...)`, `fn task(...)`), no padrão dos testes de
`tracking_repository.rs`. O caso 2 deve assertar que `amount_cents` bate com
`settle` chamado à mão sobre a mesma sessão — é isso que prova que não há
segundo caminho de cálculo.

- [ ] **Passo 4: rodar e ver passar**

```bash
cargo test -p mos-core calendar
```

Esperado: 7 testes passando (o round-trip mais os seis de `compose`).

- [ ] **Passo 5: commit**

```bash
git add crates/mos-core/src/calendar.rs crates/mos-core/src/lib.rs
git commit -m "feat(core): CalendarItem e a composicao das quatro fontes, testada"
```

---

### Task 4: o comando `calendar_window`

**Arquivos:**
- Criar: `apps/desktop/src-tauri/src/calendar.rs`
- Modificar: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consome: `CalendarItem`/`CalendarKind` (Task 3), `CaptureService::between` (Task 2), `state.tracking`, `state.work`, `state.monitoring`.
- Produz: o comando Tauri `calendar_window(since: String, until: String) -> Vec<CalendarItem>`, consumido pela Task 6.

- [ ] **Passo 1: escrever o módulo**

Criar `apps/desktop/src-tauri/src/calendar.rs`:

```rust
//! O Calendario (fase 1).
//!
//! A composicao das quatro fontes vive AQUI, e nao num servico do core, porque
//! nenhum servico existente tem os quatro repositorios e esta e a camada onde
//! eles se encontram. Mesmo lugar e mesmo motivo do `monitoring_timeline`.

use mos_core::{CalendarItem, CalendarKind, CoreError};
use tauri::{AppHandle, Manager, Runtime};

use crate::AppState;

/// Tudo o que o M/OS registrou entre dois instantes, em ordem crescente.
///
/// A janela vem como instante ISO e nao como data: quem decide onde um dia
/// comeca e o renderer, que conhece o fuso. Este comando so responde "o que
/// aconteceu entre X e Y".
#[tauri::command]
pub fn calendar_window<R: Runtime>(
    app: AppHandle<R>,
    since: String,
    until: String,
) -> Result<Vec<CalendarItem>, CoreError> {
    let state = app.state::<AppState>();
    let from = mos_core::parse_moment(&since)?;
    let to = mos_core::parse_moment(&until)?;
    if to < from {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "O fim da janela vem antes do inicio.",
            false,
        ));
    }

    // Este comando SO busca e delega. Toda a regra — o que entra na janela, o
    // que vira dois itens, o que e ignorado, a ordem — vive em
    // `mos_core::compose`, que e pura e tem os seis testes da Task 3.
    let projects = state.work.projects(true)?;
    let name_of = move |id: mos_core::ProjectId| {
        projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project removido".to_owned())
    };

    Ok(mos_core::compose(mos_core::ComposeInput {
        since: from,
        until: to,
        rounding: state.tracking.settings()?.rounding,
        entries: &state.tracking.entries(None)?,
        tasks: &state.work.tasks(true)?,
        captures: &state.captures.between(from, to)?,
        events: &state.monitoring.events(from, to)?,
        project_name: &name_of,
    }))
}
```

Se o `borrow checker` reclamar dos temporários passados por referência, ligar
cada leitura numa variável antes (`let entries = state.tracking.entries(None)?;`)
e passar `&entries`. Não mudar a assinatura de `ComposeInput` para contornar:
ela recebe fatias de propósito, para `compose` não ficar dono dos dados.

- [ ] **Passo 2: registrar o módulo e o comando**

Em `apps/desktop/src-tauri/src/lib.rs`:
- junto dos outros `mod`, acrescentar `mod calendar;` em ordem alfabética (antes de `mod hermes;`);
- dentro de `tauri::generate_handler![`, acrescentar `calendar::calendar_window,` antes de `tracking::tracking_default_cronocad_path,`.

- [ ] **Passo 3: compilar**

```bash
cargo check -p mos-desktop && cargo clippy --workspace --all-targets -- -D warnings
```

Esperado: sem erro e sem aviso.

- [ ] **Passo 4: commit**

```bash
git add apps/desktop/src-tauri/src/calendar.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(calendario): o comando que compoe as quatro fontes numa janela"
```

---

### Task 5: o agrupamento por dia local

A peça mais perigosa do trabalho, e a razão de a Task 1 existir.

**Arquivos:**
- Modificar: `apps/desktop/src/types.ts`
- Criar: `apps/desktop/src/calendarDays.ts`
- Criar: `apps/desktop/src/calendarDays.test.ts`
- Modificar: `apps/desktop/src/api.ts`

**Interfaces:**
- Consome: `CalendarItem` do comando da Task 4.
- Produz: `monthGrid(reference: Date): Date[]` (42 dias, segunda primeiro) e `groupByLocalDay(items: CalendarItem[]): Map<number, CalendarItem[]>`, usados pela Task 6. A chave do Map é `new Date(y, m, d).getTime()`.

- [ ] **Passo 1: espelhar o tipo em TypeScript**

Em `apps/desktop/src/types.ts`, antes de `export type ImportReport`, acrescentar:

```ts
export type CalendarKind = "session" | "task_done" | "task_created" | "capture" | "app_opened";

/**
 * Um item de calendário: algo que o M/OS registrou, com hora.
 *
 * `at` vem em **UTC**. Que dia isso é decide-se aqui no renderer, que é o
 * único dos dois lados que conhece o fuso de quem está olhando.
 */
export type CalendarItem = {
  kind: CalendarKind;
  at: string;
  endsAt: string | null;
  title: string;
  projectId: string | null;
  seconds: number;
  amountCents: number;
};
```

- [ ] **Passo 2: escrever o teste que falha**

Criar `apps/desktop/src/calendarDays.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { groupByLocalDay, monthGrid, startOfLocalDay } from "./calendarDays";
import type { CalendarItem } from "./types";

function item(at: string): CalendarItem {
  return {
    kind: "session",
    at,
    endsAt: null,
    title: "043 - Rancho Queimado",
    projectId: null,
    seconds: 3000,
    amountCents: 2500,
  };
}

describe("monthGrid", () => {
  it("devolve 42 dias e comeca numa segunda", () => {
    const grid = monthGrid(new Date(2026, 7, 16));
    expect(grid).toHaveLength(42);
    expect(grid[0].getDay()).toBe(1);
  });

  it("cobre o mes inteiro pedido", () => {
    const grid = monthGrid(new Date(2026, 7, 16));
    const days = grid.filter((day) => day.getMonth() === 7);
    expect(days).toHaveLength(31);
  });
});

describe("groupByLocalDay", () => {
  /**
   * O teste que justifica o vitest existir.
   *
   * O usuario trabalha de madrugada. Uma sessao que ele iniciou as 23:31 do dia
   * 30 e, em UTC-3, `2026-07-31T02:31:00Z`. Agrupar pelo dia do texto UTC a
   * colocaria no dia 31 — a grade mostraria horas num dia em que ele nao
   * trabalhou, sem nada quebrar.
   */
  it("poe a sessao das 23:31 no dia em que ela comecou, e nao no seguinte", () => {
    const localNight = new Date(2026, 6, 30, 23, 31, 0);
    const grouped = groupByLocalDay([item(localNight.toISOString())]);

    const thirtieth = startOfLocalDay(new Date(2026, 6, 30)).getTime();
    const thirtyFirst = startOfLocalDay(new Date(2026, 6, 31)).getTime();

    expect(grouped.get(thirtieth)).toHaveLength(1);
    expect(grouped.get(thirtyFirst)).toBeUndefined();
  });

  it("junta dois itens do mesmo dia local", () => {
    const grouped = groupByLocalDay([
      item(new Date(2026, 7, 12, 10, 39).toISOString()),
      item(new Date(2026, 7, 12, 18, 51).toISOString()),
    ]);
    expect(grouped.get(startOfLocalDay(new Date(2026, 7, 12)).getTime())).toHaveLength(2);
  });

  it("devolve um mapa vazio para lista vazia", () => {
    expect(groupByLocalDay([]).size).toBe(0);
  });
});
```

- [ ] **Passo 3: rodar e ver falhar**

```bash
cd apps/desktop && npm test
```

Esperado: FALHA — `Cannot find module './calendarDays'`.

- [ ] **Passo 4: implementar**

Criar `apps/desktop/src/calendarDays.ts`:

```ts
import type { CalendarItem } from "./types";

/**
 * O dia, decidido no fuso de quem está olhando.
 *
 * Todo o resto deste arquivo existe por causa de uma coisa: o banco guarda UTC,
 * o usuário trabalha de madrugada, e uma sessão iniciada às 23:31 do dia 30 é
 * `2026-07-31T02:31:00Z`. Qualquer agrupamento que olhe o TEXTO da data UTC a
 * põe no dia 31 — e a grade mostra horas num dia em que ninguém trabalhou, sem
 * erro nenhum aparecer.
 *
 * O construtor `new Date(ano, mês, dia)` é local por definição, e é ele que
 * mantém a conta honesta.
 */
export function startOfLocalDay(moment: Date) {
  return new Date(moment.getFullYear(), moment.getMonth(), moment.getDate());
}

/**
 * As 42 células de um mês: seis semanas, começando na SEGUNDA.
 *
 * Sempre 42, e não o mínimo necessário: uma grade que muda de altura conforme
 * o mês faz o conteúdo abaixo dela pular a cada navegação.
 *
 * Segunda como primeiro dia porque é assim no resto do M/OS — `WeekRings` e
 * `MonthDensity` já leem a semana de trabalho, e um calendário que começasse no
 * domingo desalinharia a leitura entre as três telas.
 */
export function monthGrid(reference: Date) {
  const first = new Date(reference.getFullYear(), reference.getMonth(), 1);
  const weekday = (first.getDay() + 6) % 7;
  const start = new Date(first);
  start.setDate(first.getDate() - weekday);

  return Array.from({ length: 42 }, (_, index) => {
    const day = new Date(start);
    day.setDate(start.getDate() + index);
    return day;
  });
}

/** Os itens por dia local. A chave é o instante da meia-noite local do dia. */
export function groupByLocalDay(items: CalendarItem[]) {
  const days = new Map<number, CalendarItem[]>();
  for (const item of items) {
    const key = startOfLocalDay(new Date(item.at)).getTime();
    const bucket = days.get(key);
    if (bucket) {
      bucket.push(item);
    } else {
      days.set(key, [item]);
    }
  }
  return days;
}
```

- [ ] **Passo 5: rodar e ver passar**

```bash
cd apps/desktop && npm test
```

Esperado: todos passando, incluindo o das 23:31.

- [ ] **Passo 6: ligar a API**

Em `apps/desktop/src/api.ts`, acrescentar `CalendarItem` à lista de tipos importados de `./types`, e acrescentar o método logo antes de `trackingTotals()`:

```ts
  /**
   * Tudo o que o M/OS registrou entre dois instantes.
   *
   * A janela vai como instante e não como data: quem decide onde um dia começa
   * é esta ponta, que conhece o fuso.
   */
  calendarWindow(since: string, until: string) {
    return invoke<CalendarItem[]>("calendar_window", { since, until });
  },
```

- [ ] **Passo 7: verificar tipos e commitar**

```bash
cd apps/desktop && npx tsc --noEmit && npm test
git add apps/desktop/src/calendarDays.ts apps/desktop/src/calendarDays.test.ts apps/desktop/src/types.ts apps/desktop/src/api.ts
git commit -m "feat(calendario): agrupamento por dia LOCAL, com o teste das 23:31"
```

---

### Task 6: a página do Calendário

**Arquivos:**
- Criar: `apps/desktop/src/CalendarPage.tsx`
- Modificar: `apps/desktop/src/App.css`

**Interfaces:**
- Consome: `api.calendarWindow`, `monthGrid`, `groupByLocalDay`, `startOfLocalDay` (Task 5), `PageHeader`, `Card`, `EmptyState` de `./Surface`, `Button` de `./Button`.
- Produz: `export function CalendarPage()` — sem props. A fase 1 não navega para lugar nenhum: clicar num dia abre o detalhe na própria tela.

- [ ] **Passo 1: escrever a página**

Criar `apps/desktop/src/CalendarPage.tsx` com esta estrutura (o corpo completo é escrito na execução, seguindo o padrão das telas do Tempo):

```tsx
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { groupByLocalDay, monthGrid, startOfLocalDay } from "./calendarDays";
import { Card, ContextPath, EmptyState, PageHeader } from "./Surface";
import type { CalendarItem, CalendarKind } from "./types";

const WEEKDAYS = ["SEG", "TER", "QUA", "QUI", "SEX", "SÁB", "DOM"];

/** O que cada tipo quer dizer na tela. O nome técnico não aparece. */
const KIND_LABEL: Record<CalendarKind, string> = {
  session: "sessão",
  task_done: "Task concluída",
  task_created: "Task criada",
  capture: "capture",
  app_opened: "programa aberto",
};

/** `2h30` ou `45min`. */
function durationOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h${String(minutes).padStart(2, "0")}` : `${minutes}min`;
}

function clockOf(iso: string) {
  return new Date(iso).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

export function CalendarPage() {
  // O mês visível, sempre no dia 1 para a navegação não escorregar em meses
  // de tamanhos diferentes (31 de março + 1 mês daria 1 de maio).
  const [month, setMonth] = useState(() => {
    const now = new Date();
    return new Date(now.getFullYear(), now.getMonth(), 1);
  });
  const [items, setItems] = useState<CalendarItem[]>([]);
  const [chosen, setChosen] = useState<number | null>(null);
  const [note, setNote] = useState("");

  const grid = useMemo(() => monthGrid(month), [month]);

  const load = useCallback(async () => {
    setNote("");
    // A janela é a GRADE inteira, e não o mês: as células da primeira e da
    // última semana pertencem aos meses vizinhos, e sem elas esses dias
    // apareceriam sempre vazios.
    const since = grid[0].toISOString();
    const lastDay = grid[grid.length - 1];
    const until = new Date(
      lastDay.getFullYear(), lastDay.getMonth(), lastDay.getDate(), 23, 59, 59, 999,
    ).toISOString();
    try {
      setItems(await api.calendarWindow(since, until));
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, [grid]);

  useEffect(() => { void load(); }, [load]);

  const byDay = useMemo(() => groupByLocalDay(items), [items]);
  const today = startOfLocalDay(new Date()).getTime();

  // ... grade, célula e detalhe do dia, conforme os passos abaixo
}
```

- [ ] **Passo 2: a grade e a célula**

Dentro do componente, antes do `return`:

```tsx
  const monthLabel = month
    .toLocaleDateString("pt-BR", { month: "long", year: "numeric" })
    .toUpperCase();

  const step = (months: number) =>
    setMonth(new Date(month.getFullYear(), month.getMonth() + months, 1));
```

E o `return`:

```tsx
  return (
    <div className="page tempo-page">
      <ContextPath segments={["M", "CALENDÁRIO"]} />

      <PageHeader
        title="Calendário"
        subtitle="O que aconteceu em cada dia."
        actions={
          <>
            <Button variant="ghost" size="sm" onClick={() => step(-1)} aria-label="Mês anterior">‹</Button>
            <Button variant="ghost" size="sm" onClick={() => { setMonth(new Date(new Date().getFullYear(), new Date().getMonth(), 1)); setChosen(today); }}>Hoje</Button>
            <Button variant="ghost" size="sm" onClick={() => step(1)} aria-label="Próximo mês">›</Button>
          </>
        }
      />

      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      <Card label={monthLabel}>
        <div className="calendar-grid" role="grid" aria-label={`Calendário de ${monthLabel}`}>
          {WEEKDAYS.map((weekday) => (
            <span className="micro-label calendar-weekday" key={weekday}>{weekday}</span>
          ))}
          {grid.map((day) => {
            const key = startOfLocalDay(day).getTime();
            const dayItems = byDay.get(key) ?? [];
            const worked = dayItems
              .filter((item) => item.kind === "session")
              .reduce((sum, item) => sum + item.seconds, 0);
            // Um ponto por TIPO presente, nunca um por item: três Tasks
            // concluídas fazem um ponto, não três. A célula responde "houve
            // Task aqui"; a contagem exata é o que o detalhe do dia dá. Sem
            // isso, um dia movimentado vira uma nuvem que não se conta de
            // relance nem se lê como número.
            const kinds = [...new Set(dayItems.map((item) => item.kind))];
            return (
              <button
                type="button"
                key={key}
                className="calendar-cell"
                data-outside={day.getMonth() !== month.getMonth() || undefined}
                data-today={key === today || undefined}
                aria-pressed={key === chosen}
                onClick={() => setChosen(key)}
              >
                <span className="calendar-day">{day.getDate()}</span>
                {worked ? <span className="calendar-hours">{durationOf(worked)}</span> : null}
                {kinds.length ? (
                  <span className="calendar-dots" aria-hidden="true">
                    {kinds.map((kind) => <span key={kind} data-kind={kind} />)}
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
      </Card>

      {chosen !== null ? <DayDetail at={chosen} items={byDay.get(chosen) ?? []} /> : null}
    </div>
  );
}
```

- [ ] **Passo 3: o detalhe do dia**

No mesmo arquivo, abaixo de `CalendarPage`:

```tsx
/**
 * O dia aberto.
 *
 * Vive fora da grade de propósito: a célula responde "houve algo aqui" em meio
 * segundo, e é este painel que responde "o quê". Misturar os dois faria a
 * célula crescer até a grade deixar de caber num mês.
 */
function DayDetail({ at, items }: { at: number; items: CalendarItem[] }) {
  const label = new Date(at).toLocaleDateString("pt-BR", {
    weekday: "long",
    day: "2-digit",
    month: "long",
  });

  return (
    <Card label={label.toUpperCase()} count={items.length ? String(items.length) : undefined}>
      {items.length ? (
        <ul className="calendar-day-list">
          {items.map((item, index) => (
            <li key={`${item.at}-${index}`}>
              <span className="calendar-item-time">{clockOf(item.at)}</span>
              <span className="calendar-item-kind" data-kind={item.kind}>{KIND_LABEL[item.kind]}</span>
              <span className="calendar-item-title">{item.title}</span>
              {item.seconds ? <span className="calendar-item-duration">{durationOf(item.seconds)}</span> : null}
            </li>
          ))}
        </ul>
      ) : (
        <EmptyState>Nada registrado neste dia.</EmptyState>
      )}
    </Card>
  );
}
```

- [ ] **Passo 4: o CSS**

Em `apps/desktop/src/App.css`, junto do bloco do Tempo, acrescentar `.calendar-grid` (grade de 7 colunas iguais), `.calendar-cell` (célula com número no topo e horas embaixo, borda de 1px, `aspect-ratio: 1 / 0.8`), `.calendar-cell[data-outside]` (número em `--text-disabled`), `.calendar-cell[data-today]` (número em `--signal-ink`), `.calendar-cell[aria-pressed='true']` (borda em `--signal-ink`), `.calendar-dots` (linha de pontos de 4px) e `.calendar-day-list`. Seguir os tokens; nenhum valor de cor literal.

- [ ] **Passo 5: verificar**

```bash
cd apps/desktop && npx tsc --noEmit && npm run build
```

- [ ] **Passo 6: commit**

```bash
git add apps/desktop/src/CalendarPage.tsx apps/desktop/src/App.css
git commit -m "feat(calendario): a grade do mes e o detalhe do dia"
```

---

### Task 7: Calendário entra no rail, Apps sai

**Arquivos:**
- Modificar: `apps/desktop/src/Icon.tsx`
- Modificar: `apps/desktop/src/App.tsx`
- Modificar: `docs/DECISIONS.md`

**Interfaces:**
- Consome: `CalendarPage` (Task 6).
- Produz: nada que outra tarefa use — é a última.

- [ ] **Passo 1: o ícone**

Em `apps/desktop/src/Icon.tsx`, acrescentar `| "calendar"` ao `IconName` e a entrada em `OUTLINE_20` (o compilador exige, porque é um `Record` completo):

```tsx
  calendar: <><rect x="3.5" y="5.5" width="13" height="11" /><path d="M3.5 9.5h13M7 3.5v3M13 3.5v3" /></>,
```

E em `SOLID_20`, que é `Partial`:

```tsx
  calendar: <><rect x="3.2" y="5.2" width="13.6" height="11.6" /><rect x="6.4" y="2.9" width="1.6" height="3.4" /><rect x="12" y="2.9" width="1.6" height="3.4" /></>,
```

- [ ] **Passo 2: trocar o destino no rail**

Em `apps/desktop/src/App.tsx`:
- no tipo `Page`, trocar `"apps"` por `"apps" | "calendario"` (Apps continua existindo como página, só sai do rail);
- no array `nav`, **remover** `{ page: "apps", label: "Apps", icon: "apps" }` e acrescentar `{ page: "calendario", label: "Calendário", icon: "calendar" }` depois de `tempo`;
- em `pageLabels`, acrescentar `calendario: "Calendário"`;
- junto dos outros `if (page === ...)`, acrescentar `if (page === "calendario") return <CalendarPage />;`;
- importar `CalendarPage`.

- [ ] **Passo 3: verificar que Apps continua alcançável**

```bash
cd apps/desktop && npx tsc --noEmit && npm run build
```

Conferir à mão no código que a página `apps` ainda é roteada e que o Command (`CommandSurface`) ainda a encontra. Se o Command listar destinos a partir do array `nav`, acrescentar Apps à lista do Command explicitamente — **sem isso, remover do rail some com a página**, que é exatamente o que falhou com Workspaces na ADR-031.

- [ ] **Passo 4: escrever a ADR-038**

Acrescentar ao fim de `docs/DECISIONS.md` uma ADR seguindo a forma das anteriores, com: contexto (o teto de nove da ADR-036 e a regra de que o décimo exige retirar um), decisão (Apps sai, Calendário entra), a evidência (zero apps cadastrados no banco do usuário, e o critério "renda ou memória" da própria ADR-036), por que não Workspaces (a ADR-031 registra que rebaixá-lo já falhou), a ressalva de que "zero apps" mede conteúdo e não frequência, e as consequências (Apps segue no Command e nos atalhos `Ctrl+1..9`; a decisão é reversível).

- [ ] **Passo 5: verificação final e commit**

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
cd apps/desktop && npm test && npx tsc --noEmit && npm run build
```

```bash
git add apps/desktop/src/Icon.tsx apps/desktop/src/App.tsx docs/DECISIONS.md
git commit -m "feat(calendario): decimo destino do rail, Apps sai (ADR-038)"
```

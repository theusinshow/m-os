# Obsolescência — Tasks e Projects parados — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mostrar o que está parado há tempo demais — Tasks por coluna do Kanban e Projects sem atividade — num widget novo da Home e numa marca no card do quadro, sem nenhuma tabela nova.

**Architecture:** Um módulo puro `mos-core::stale` decide tudo: qual a tolerância de cada coluna, qual é a atividade real de um Project (a Task mais recente dele, e não o campo `updated_at` que só muda quando alguém renomeia), quem passou do limite e em que ordem. Um comando Tauri lê `tasks(false)`/`projects(false)` e delega. O front recebe pronto e só desenha — o que ele decide (rótulo, corte em cinco) mora em `stale.ts`, testado. Zero persistência: nenhuma migration, nenhuma tabela, nenhuma operação de sync.

**Tech Stack:** Rust (`mos-core`, `time`, `serde`), Tauri 2 (comando), React 19 + TypeScript (`apps/desktop/src`), Vitest, CSS com tokens do `packages/design-system`.

**Spec:** `docs/superpowers/specs/2026-08-21-obsolescencia-design.md` (aprovado em 2026-08-22)

## Global Constraints

- **Ambiente, antes de qualquer `cargo`:** exportar `TMP` e `TEMP` para um diretório gravável. Sem isso o build falha nesta máquina.
- **Comando de teste do Rust:** `cargo test --workspace --exclude mos-desktop`. `cargo test -p mos-desktop --lib` falha com `STATUS_ENTRYPOINT_NOT_FOUND` por um problema de linker **pré-existente** — não é regressão desta feature e não deve ser investigado aqui.
- **Comando de teste do front:** `npm test` dentro de `apps/desktop` (é `vitest run`).
- **Nada de plataforma em `mos-core`.** Um `#[cfg(windows)]` ali significa que o desenho quebrou (AGENTS.md, regra dura 1).
- **Zero persistência.** Nenhuma migration, nenhuma coluna, nenhuma linha em `outbox`. Se o plano em algum ponto pedir um `CREATE TABLE`, o plano está errado.
- **Nenhum fuso.** `updated_at` é UTC e a conta é de **duração**, não de data civil. Não replicar o offset da tela que a Daily Session precisou.
- **Comentários e nomes em português**, como o resto do repositório. Comentário explica *por quê*, não *o quê*.
- **Ids de widget nunca mudam.** O id novo é `stale`, e ele é definitivo — a linha guardada em `widget_placements` deixa de casar se o id mudar.
- **Os oito eixos do `FEATURE-DEVELOPMENT.md`**, declarados:
  - **Core:** `mos-core::stale`, puro, com testes.
  - **Database:** nenhuma migration — a obsolescência é derivada de `updated_at`, que já existe.
  - **Sync:** não sincroniza, porque não há estado próprio. O que viaja são as Tasks e os Projects, que já emitem.
  - **Desktop:** widget `PARADAS` na faixa Retomar, marca no card do Kanban, correção do ponto do widget `PROJECTS`.
  - **iOS:** a regra já está no core e viaja inteira. A manifestação mobile não se aplica hoje — não existe superfície iOS construída (`PLATFORMS.md` §3, coluna toda em "—"); quando existir, o widget é a mesma lista.
  - **Notifications:** nenhuma. Mesma decisão do §8 do `DAILY-SESSION.md` — obsolescência é para ser notada ao olhar, não para interromper.
  - **Hermes:** lê pelo caminho que já existe (`mos-query` alcança Tasks e Projects). Sem função nova.
  - **Tests:** domínio em `stale.rs` (inline `#[cfg(test)]`), apresentação em `stale.test.ts`.

## Estrutura de arquivos

| Arquivo | Responsabilidade |
| --- | --- |
| `crates/mos-core/src/stale.rs` (criar) | tolerância por coluna, atividade real do Project, composição e ordenação das paradas, `StaleView` |
| `crates/mos-core/src/lib.rs` (modificar) | `mod stale;` e os `pub use` |
| `apps/desktop/src-tauri/src/stale.rs` (criar) | comando `stale_list`: lê os dois repositórios e delega |
| `apps/desktop/src-tauri/src/lib.rs` (modificar) | `mod stale;` e registro no `invoke_handler` |
| `apps/desktop/src/types.ts` (modificar) | `Parada`, `ProjectActivity`, `StaleView` |
| `apps/desktop/src/api.ts` (modificar) | `staleList()` |
| `apps/desktop/src/stale.ts` (criar) | rótulo "12d", corte em cinco, índices por id — o que a tela decide |
| `apps/desktop/src/stale.test.ts` (criar) | testes do acima |
| `apps/desktop/src/homeLayout.ts` (modificar) | entrada `stale` no catálogo |
| `apps/desktop/src/App.tsx` (modificar) | estado no `refresh`, widget `PARADAS`, `data-stale` no `DataRow`, Kanban, widget `PROJECTS` |
| `apps/desktop/src/App.css` (modificar) | `[data-stale]` no card e no ponto |
| `docs/DECISIONS.md` (modificar) | ADR-056 |
| `docs/PLATFORMS.md`, `README.md` (modificar) | matriz e lista de capacidades |

---

### Task 1: A tolerância de cada coluna, e a atividade real de um Project

O módulo novo, com as duas funções que tudo o mais consome. Elas vêm juntas porque sozinhas não entregam nada visível, e porque a segunda é a correção do defeito que a §3 do spec encontrou.

**Files:**
- Create: `crates/mos-core/src/stale.rs`
- Modify: `crates/mos-core/src/lib.rs` (linha 21, junto de `mod service;` — a lista é alfabética: `stale` entra entre `service` e `tracking`)
- Test: inline em `crates/mos-core/src/stale.rs` (`#[cfg(test)] mod tests`), como `weekly.rs:384` e `calendar.rs:257`

**Interfaces:**
- Consumes: `crate::{LifecycleState, Project, ProjectId, Task, TaskState}` — todos já existem em `work.rs`.
- Produces:
  - `pub fn tolerancia(state: TaskState) -> Option<Duration>`
  - `pub const TOLERANCIA_PROJECT: Duration`
  - `pub fn atividade_do_project(project: &Project, tasks: &[Task]) -> OffsetDateTime` — recebe a lista **completa** de Tasks e filtra por dentro.
  - `pub fn trabalho_aberto(project: &Project, tasks: &[Task]) -> usize`

- [ ] **Step 1: Criar o arquivo com o cabeçalho e as declarações, sem corpo**

Crie `crates/mos-core/src/stale.rs`:

```rust
//! O que esta parado ha tempo demais.
//!
//! Dominio PURO e sem persistencia nenhuma: nao ha tabela, nao ha migration e
//! nao ha operacao de sync. Obsolescencia e uma LEITURA de `updated_at`, que ja
//! existe em toda Task e todo Project — inventar estado para ela seria guardar
//! uma conta que o banco ja sabe fazer.
//!
//! **O limiar e por coluna, e nao um numero unico.** Um limiar so transformaria
//! o backlog inteiro num alerta permanente: num sistema com meses de uso o
//! backlog domina a lista e afoga o sinal. Por coluna, o resultado tipico e
//! tres paradas, e nao quarenta e sete.
//!
//! **Nao ha fuso aqui.** `updated_at` e UTC e a conta e de DURACAO, e nao de
//! data civil — diferente do `Day` da Daily Session, que precisou do offset da
//! tela porque um dia e um lugar no calendario de quem olha.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::{LifecycleState, Project, ProjectId, Task, TaskState};

/// Quanto tempo uma Task pode ficar parada naquela coluna antes de virar
/// pergunta. `None` significa que a coluna nao tem limite — e nao que o limite
/// e infinito por descuido.
pub fn tolerancia(_state: TaskState) -> Option<Duration> {
    todo!("Step 3")
}

/// Project se move em semanas, e Task em dias.
pub const TOLERANCIA_PROJECT: Duration = Duration::days(21);

/// A atividade de um Project e a atividade das Tasks dele.
pub fn atividade_do_project(_project: &Project, _tasks: &[Task]) -> OffsetDateTime {
    todo!("Step 6")
}

/// Quantas Tasks ativas e nao concluidas o Project tem.
pub fn trabalho_aberto(_project: &Project, _tasks: &[Task]) -> usize {
    todo!("Step 6")
}
```

Registre o módulo em `crates/mos-core/src/lib.rs`, entre `mod service;` e `mod tracking;`:

```rust
mod service;
mod stale;
mod tracking;
```

E o `pub use`, logo depois do bloco `pub use service::{...}` (a ordem dos `pub use` acompanha a dos `mod`):

```rust
pub use stale::{atividade_do_project, tolerancia, trabalho_aberto, TOLERANCIA_PROJECT};
```

- [ ] **Step 2: Escrever os testes da tolerância**

No fim de `crates/mos-core/src/stale.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::{ProjectId, TaskId};
    use time::macros::datetime;

    #[test]
    fn cada_coluna_tem_a_sua_tolerancia() {
        assert_eq!(tolerancia(TaskState::Doing), Some(Duration::days(7)));
        assert_eq!(tolerancia(TaskState::Review), Some(Duration::days(7)));
        assert_eq!(tolerancia(TaskState::Planned), Some(Duration::days(21)));
        assert_eq!(tolerancia(TaskState::Inbox), Some(Duration::days(14)));
    }

    /// Backlog e onde as coisas esperam, e done acabou. Nenhum dos dois pode
    /// entrar: um limiar unico afogaria o sinal com o backlog inteiro.
    #[test]
    fn backlog_e_done_nunca_entram() {
        assert_eq!(tolerancia(TaskState::Backlog), None);
        assert_eq!(tolerancia(TaskState::Done), None);
    }
}
```

- [ ] **Step 3: Rodar e ver falhar, depois implementar a tolerância**

Rode: `cargo test -p mos-core stale`
Esperado: FALHA com `not yet implemented` no `todo!`.

Substitua o corpo de `tolerancia`:

```rust
pub fn tolerancia(state: TaskState) -> Option<Duration> {
    match state {
        // Comecou e largou.
        TaskState::Doing => Some(Duration::days(7)),
        // Esperando alguem.
        TaskState::Review => Some(Duration::days(7)),
        // Foi planejada e nao andou.
        TaskState::Planned => Some(Duration::days(21)),
        // Entrou e nunca foi decidida.
        TaskState::Inbox => Some(Duration::days(14)),
        // Backlog e onde as coisas esperam; done acabou. Os dois `None` sao
        // decisao, e nao lacuna.
        TaskState::Backlog | TaskState::Done => None,
    }
}
```

Rode de novo: `cargo test -p mos-core stale`
Esperado: os dois testes PASSAM.

- [ ] **Step 4: Commit**

```bash
git add crates/mos-core/src/stale.rs crates/mos-core/src/lib.rs
git commit -m "feat(paradas): a tolerancia e por coluna, e duas delas nao tem tolerancia"
```

- [ ] **Step 5: Escrever os testes da atividade do Project**

Acrescente ao `mod tests` de `stale.rs`, logo depois dos helpers (crie os helpers junto):

```rust
    fn instante() -> OffsetDateTime {
        datetime!(2026-08-22 09:00 UTC)
    }

    fn project(nome: &str, atualizado: OffsetDateTime) -> Project {
        Project {
            id: ProjectId::new(),
            name: nome.to_owned(),
            description: String::new(),
            repository: String::new(),
            lifecycle_state: LifecycleState::Active,
            created_at: atualizado,
            updated_at: atualizado,
        }
    }

    fn task(
        titulo: &str,
        state: TaskState,
        atualizada: OffsetDateTime,
        project: Option<&Project>,
    ) -> Task {
        Task {
            id: TaskId::new(),
            title: titulo.to_owned(),
            description: String::new(),
            project_id: project.map(|alvo| alvo.id),
            source_capture_id: None,
            state,
            lifecycle_state: LifecycleState::Active,
            created_at: atualizada,
            updated_at: atualizada,
            completed_at: None,
        }
    }

    /// O defeito que a §3 do spec encontrou: `projects.updated_at` so muda
    /// quando o Project e EDITADO. Criar Task, mover no Kanban e concluir nao
    /// tocam naquela coluna. Usa-lo como sinal marcaria como parado o Project em
    /// que se trabalhou ontem.
    #[test]
    fn a_atividade_do_project_vem_da_task_mais_recente() {
        let alvo = project("Casa", instante() - Duration::days(30));
        let tasks = vec![
            task("velha", TaskState::Doing, instante() - Duration::days(20), Some(&alvo)),
            task("nova", TaskState::Doing, instante() - Duration::days(2), Some(&alvo)),
        ];
        assert_eq!(
            atividade_do_project(&alvo, &tasks),
            instante() - Duration::days(2)
        );
    }

    #[test]
    fn project_sem_task_nenhuma_cai_no_proprio_campo() {
        let alvo = project("Sozinho", instante() - Duration::days(9));
        assert_eq!(atividade_do_project(&alvo, &[]), instante() - Duration::days(9));
    }

    /// A Task de outro Project nao pode dar vida a este.
    #[test]
    fn a_task_de_outro_project_nao_conta() {
        let alvo = project("Casa", instante() - Duration::days(30));
        let outro = project("Trabalho", instante() - Duration::days(30));
        let tasks = vec![task("de la", TaskState::Doing, instante(), Some(&outro))];
        assert_eq!(
            atividade_do_project(&alvo, &tasks),
            instante() - Duration::days(30)
        );
    }

    #[test]
    fn trabalho_aberto_ignora_concluida_e_arquivada() {
        let alvo = project("Casa", instante());
        let mut arquivada = task("arquivada", TaskState::Doing, instante(), Some(&alvo));
        arquivada.lifecycle_state = LifecycleState::Archived;
        let tasks = vec![
            task("aberta", TaskState::Doing, instante(), Some(&alvo)),
            task("pronta", TaskState::Done, instante(), Some(&alvo)),
            arquivada,
        ];
        assert_eq!(trabalho_aberto(&alvo, &tasks), 1);
    }
```

- [ ] **Step 6: Rodar, ver falhar, implementar**

Rode: `cargo test -p mos-core stale`
Esperado: FALHA com `not yet implemented`.

Substitua os dois corpos:

```rust
/// A atividade de um Project e a atividade das Tasks dele — `max(updated_at)`,
/// caindo no campo do proprio Project so quando ele nao tem Task nenhuma.
///
/// # Por que nao `project.updated_at`
///
/// Porque aquela coluna so muda quando o Project e EDITADO: so `update_project`
/// e `set_project_lifecycle` escrevem nela. Criar Task, mover no Kanban,
/// concluir — nada disso a toca. Usa-la como sinal de obsolescencia marcaria
/// como "parado" o Project em que se trabalhou ontem, e como "vivo" o que foi
/// renomeado e abandonado.
///
/// Recebe a lista COMPLETA e filtra por dentro de proposito: quem chama ja tem
/// todas as Tasks na mao, e montar uma fatia por Project so criaria trabalho
/// para desfazer.
pub fn atividade_do_project(project: &Project, tasks: &[Task]) -> OffsetDateTime {
    tasks
        .iter()
        .filter(|task| {
            task.project_id == Some(project.id) && task.lifecycle_state == LifecycleState::Active
        })
        .map(|task| task.updated_at)
        .max()
        .unwrap_or(project.updated_at)
}

/// Quantas Tasks ativas e nao concluidas o Project tem.
///
/// Project sem trabalho aberto NAO esta travado: ele acabou e ninguem arquivou,
/// que e outra pergunta e merece outra resposta.
pub fn trabalho_aberto(project: &Project, tasks: &[Task]) -> usize {
    tasks
        .iter()
        .filter(|task| {
            task.project_id == Some(project.id)
                && task.lifecycle_state == LifecycleState::Active
                && task.state != TaskState::Done
        })
        .count()
}
```

Rode: `cargo test -p mos-core stale`
Esperado: os seis testes PASSAM.

- [ ] **Step 7: Commit**

```bash
git add crates/mos-core/src/stale.rs
git commit -m "fix(paradas): a atividade de um Project e a das Tasks dele, e nao o campo dele"
```

---

### Task 2: A composição — quem está parado, e em que ordem

**Files:**
- Modify: `crates/mos-core/src/stale.rs`
- Modify: `crates/mos-core/src/lib.rs` (o `pub use` cresce)
- Test: inline em `crates/mos-core/src/stale.rs`

**Interfaces:**
- Consumes: `tolerancia`, `TOLERANCIA_PROJECT`, `atividade_do_project`, `trabalho_aberto` (Task 1).
- Produces:
  - `pub enum StaleKind { Task, Project }` com `as_str()`
  - `pub struct Parada { kind, id: String, title: String, context: String, state: String, days: i64 }` — `Serialize`/`Deserialize`, `camelCase`
  - `pub struct StaleInput<'a> { now, tasks, projects, project_name }`
  - `pub fn compose_stale(input: StaleInput<'_>) -> Vec<Parada>`

- [ ] **Step 1: Escrever os tipos, com a função ainda vazia**

Acrescente a `crates/mos-core/src/stale.rs`, antes do `mod tests`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleKind {
    Task,
    Project,
}

impl StaleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Project => "project",
        }
    }
}

/// Uma coisa parada, ja pronta para a tela.
///
/// `context` e `state` sao Strings e nao tipos porque as duas variantes os
/// preenchem com coisas diferentes: a Task traz o nome do Project e a coluna, e
/// o Project traz "N tasks abertas" e nada. Um enum com dois formatos custaria
/// mais do que a ambiguidade que ele evitaria.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parada {
    pub kind: StaleKind,
    pub id: String,
    pub title: String,
    /// Nome do Project, para Task. "N tasks abertas", para Project. Vazio
    /// quando a Task nao tem Project.
    pub context: String,
    /// A coluna do Kanban, para Task. Vazio para Project.
    pub state: String,
    /// Dias inteiros parados.
    pub days: i64,
}

/// O que [`compose_stale`] precisa.
///
/// Estrutura em vez de quatro parametros soltos, como no `ComposeInput` do
/// calendario: duas das entradas sao colecoes, e troca-las de lugar por engano
/// compilaria sem reclamacao nenhuma.
pub struct StaleInput<'a> {
    /// O agora vem de fora para o teste poder fixa-lo. Nao ha `now_utc()` aqui
    /// dentro: funcao pura que le o relogio nao se testa.
    pub now: OffsetDateTime,
    pub tasks: &'a [Task],
    pub projects: &'a [Project],
    /// Como achar o nome de um Project. Fechamento e nao mapa pronto, pelo mesmo
    /// motivo do `calendar::ComposeInput`.
    pub project_name: &'a dyn Fn(ProjectId) -> String,
}

/// Tudo o que passou da tolerancia, do mais excedido para o menos.
pub fn compose_stale(_input: StaleInput<'_>) -> Vec<Parada> {
    todo!("Step 4")
}
```

E amplie o `pub use` em `crates/mos-core/src/lib.rs`:

```rust
pub use stale::{
    atividade_do_project, compose_stale, tolerancia, trabalho_aberto, Parada, StaleInput,
    StaleKind, TOLERANCIA_PROJECT,
};
```

- [ ] **Step 2: Escrever os testes da fronteira e dos excluídos**

Acrescente ao `mod tests`:

```rust
    fn entrada<'a>(
        agora: OffsetDateTime,
        tasks: &'a [Task],
        projects: &'a [Project],
    ) -> StaleInput<'a> {
        StaleInput {
            now: agora,
            tasks,
            projects,
            project_name: &|_| "Casa".to_owned(),
        }
    }

    /// A fronteira exata do limiar de 7 dias. Entra quem PASSOU dele: 7 dias
    /// cravados ainda estao dentro da tolerancia, e o primeiro rotulo que
    /// aparece e "8d".
    #[test]
    fn seis_dias_nao_entra_e_oito_entra() {
        let seis = vec![task("seis", TaskState::Doing, instante() - Duration::days(6), None)];
        assert!(compose_stale(entrada(instante(), &seis, &[])).is_empty());

        let sete = vec![task("sete", TaskState::Doing, instante() - Duration::days(7), None)];
        assert!(compose_stale(entrada(instante(), &sete, &[])).is_empty());

        let oito = vec![task("oito", TaskState::Doing, instante() - Duration::days(8), None)];
        let paradas = compose_stale(entrada(instante(), &oito, &[]));
        assert_eq!(paradas.len(), 1);
        assert_eq!(paradas[0].days, 8);
        assert_eq!(paradas[0].state, "doing");
        assert_eq!(paradas[0].kind, StaleKind::Task);
    }

    #[test]
    fn backlog_e_done_ficam_de_fora_por_mais_velhos_que_sejam() {
        let tasks = vec![
            task("guardada", TaskState::Backlog, instante() - Duration::days(400), None),
            task("pronta", TaskState::Done, instante() - Duration::days(400), None),
        ];
        assert!(compose_stale(entrada(instante(), &tasks, &[])).is_empty());
    }

    #[test]
    fn arquivada_e_na_lixeira_ficam_fora_de_tudo() {
        let mut arquivada = task("arquivada", TaskState::Doing, instante() - Duration::days(90), None);
        arquivada.lifecycle_state = LifecycleState::Archived;
        let mut no_lixo = task("no lixo", TaskState::Doing, instante() - Duration::days(90), None);
        no_lixo.lifecycle_state = LifecycleState::Trashed;
        let tasks = vec![arquivada, no_lixo];
        assert!(compose_stale(entrada(instante(), &tasks, &[])).is_empty());
    }

    #[test]
    fn a_task_traz_o_nome_do_project_como_contexto() {
        let alvo = project("Casa", instante());
        let tasks = vec![task("pintar", TaskState::Doing, instante() - Duration::days(9), Some(&alvo))];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].context, "Casa");
    }

    #[test]
    fn task_sem_project_fica_sem_contexto_em_vez_de_texto_inventado() {
        let tasks = vec![task("solta", TaskState::Doing, instante() - Duration::days(9), None)];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].context, "");
    }
```

- [ ] **Step 3: Escrever os testes do Project e da ordem**

Ainda no `mod tests`:

```rust
    #[test]
    fn project_parado_com_trabalho_aberto_entra_com_a_contagem_no_contexto() {
        let alvo = project("Casa", instante() - Duration::days(60));
        let tasks = vec![
            // Planned tem tolerancia de 21: com 30 dias ela tambem entra, e o
            // teste confere que o Project entra ALEM dela.
            task("uma", TaskState::Planned, instante() - Duration::days(30), Some(&alvo)),
            task("outra", TaskState::Backlog, instante() - Duration::days(30), Some(&alvo)),
        ];
        let projects = vec![alvo];
        let paradas = compose_stale(entrada(instante(), &tasks, &projects));
        let project = paradas
            .iter()
            .find(|parada| parada.kind == StaleKind::Project)
            .expect("o Project parado precisa entrar");
        assert_eq!(project.days, 30);
        assert_eq!(project.context, "2 tasks abertas");
        assert_eq!(project.state, "");
    }

    #[test]
    fn uma_task_aberta_fala_no_singular() {
        let alvo = project("Casa", instante() - Duration::days(60));
        let tasks = vec![task("uma", TaskState::Backlog, instante() - Duration::days(30), Some(&alvo))];
        let projects = vec![alvo];
        let paradas = compose_stale(entrada(instante(), &tasks, &projects));
        assert_eq!(paradas[0].context, "1 task aberta");
    }

    /// Project sem trabalho aberto nao esta travado: ele acabou e ninguem
    /// arquivou, que e outra pergunta.
    #[test]
    fn project_sem_trabalho_aberto_nao_entra() {
        let alvo = project("Acabado", instante() - Duration::days(60));
        let tasks = vec![task("pronta", TaskState::Done, instante() - Duration::days(60), Some(&alvo))];
        let projects = vec![alvo];
        let paradas = compose_stale(entrada(instante(), &tasks, &projects));
        assert!(paradas.iter().all(|parada| parada.kind != StaleKind::Project));
    }

    #[test]
    fn project_arquivado_nao_entra() {
        let mut alvo = project("Guardado", instante() - Duration::days(60));
        alvo.lifecycle_state = LifecycleState::Archived;
        let tasks = vec![task("aberta", TaskState::Backlog, instante() - Duration::days(60), Some(&alvo))];
        let projects = vec![alvo];
        assert!(compose_stale(entrada(instante(), &tasks, &projects)).is_empty());
    }

    /// A ordem e o EXCESSO PROPORCIONAL, e nao os dias crus. Uma Task 12 dias
    /// parada num limiar de 7 esta a 171%; uma 24 dias num limiar de 21 esta a
    /// 114%. Ordenar por dias colocaria a segunda primeiro, e ela e a menos
    /// urgente das duas.
    #[test]
    fn a_ordem_e_o_excesso_proporcional_e_nao_os_dias() {
        let tasks = vec![
            task("planejada ha muito", TaskState::Planned, instante() - Duration::days(24), None),
            task("largada no meio", TaskState::Doing, instante() - Duration::days(12), None),
        ];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].title, "largada no meio");
        assert_eq!(paradas[1].title, "planejada ha muito");
    }

    /// Empate no excesso nao pode sair em ordem que muda a cada leitura: a lista
    /// da Home seria diferente a cada refresh sem nada ter mudado.
    #[test]
    fn empate_desempata_pelo_titulo_para_a_ordem_nao_dancar() {
        let tasks = vec![
            task("zebra", TaskState::Doing, instante() - Duration::days(14), None),
            task("abelha", TaskState::Doing, instante() - Duration::days(14), None),
        ];
        let paradas = compose_stale(entrada(instante(), &tasks, &[]));
        assert_eq!(paradas[0].title, "abelha");
        assert_eq!(paradas[1].title, "zebra");
    }

    /// Relogio para tras, ou `updated_at` no futuro, nao pode virar dias
    /// negativos entrando na lista por acidente de sinal.
    #[test]
    fn atualizada_no_futuro_nao_entra() {
        let tasks = vec![task("adiantada", TaskState::Doing, instante() + Duration::days(3), None)];
        assert!(compose_stale(entrada(instante(), &tasks, &[])).is_empty());
    }
```

- [ ] **Step 4: Rodar, ver falhar, implementar**

Rode: `cargo test -p mos-core stale`
Esperado: FALHA com `not yet implemented`.

Substitua o corpo de `compose_stale` e acrescente o auxiliar:

```rust
/// Dias inteiros entre dois instantes, nunca negativo.
///
/// Truncar para dias inteiros e o que faz a comparacao casar com o rotulo: com
/// limiar de 7, o primeiro item que aparece diz "8d". Comparar `Duration` cheia
/// deixaria entrar um item de 7 dias e uma hora exibindo "7d" — o numero na tela
/// contradiria o criterio.
fn dias_parado(agora: OffsetDateTime, desde: OffsetDateTime) -> i64 {
    (agora - desde).whole_days().max(0)
}

pub fn compose_stale(input: StaleInput<'_>) -> Vec<Parada> {
    // A tolerancia viaja ao lado da parada porque a ordenacao precisa dela, e o
    // consumidor nao. Ela morre nesta funcao.
    let mut medidas: Vec<(Parada, i64)> = Vec::new();

    for task in input.tasks {
        if task.lifecycle_state != LifecycleState::Active {
            continue;
        }
        let Some(limite) = tolerancia(task.state) else {
            continue;
        };
        let limite = limite.whole_days();
        let dias = dias_parado(input.now, task.updated_at);
        if dias <= limite {
            continue;
        }
        medidas.push((
            Parada {
                kind: StaleKind::Task,
                id: task.id.to_string(),
                title: task.title.clone(),
                context: task
                    .project_id
                    .map(input.project_name)
                    .unwrap_or_default(),
                state: task.state.as_str().to_owned(),
                days: dias,
            },
            limite,
        ));
    }

    let limite_project = TOLERANCIA_PROJECT.whole_days();
    for project in input.projects {
        if project.lifecycle_state != LifecycleState::Active {
            continue;
        }
        let abertas = trabalho_aberto(project, input.tasks);
        if abertas == 0 {
            continue;
        }
        let dias = dias_parado(input.now, atividade_do_project(project, input.tasks));
        if dias <= limite_project {
            continue;
        }
        medidas.push((
            Parada {
                kind: StaleKind::Project,
                id: project.id.to_string(),
                title: project.name.clone(),
                context: if abertas == 1 {
                    "1 task aberta".to_owned()
                } else {
                    format!("{abertas} tasks abertas")
                },
                state: String::new(),
                days: dias,
            },
            limite_project,
        ));
    }

    // Excesso proporcional, por produto cruzado: `a.days / a.limite` contra
    // `b.days / b.limite` sem tocar em float. Empate cai no titulo para a lista
    // da Home nao dancar entre dois refreshes iguais.
    medidas.sort_by(|(a, limite_a), (b, limite_b)| {
        (b.days * limite_a)
            .cmp(&(a.days * limite_b))
            .then_with(|| b.days.cmp(&a.days))
            .then_with(|| a.title.cmp(&b.title))
    });

    medidas.into_iter().map(|(parada, _)| parada).collect()
}
```

Rode: `cargo test -p mos-core stale`
Esperado: todos os testes PASSAM.

- [ ] **Step 5: Rodar a suíte do crate inteiro**

Rode: `cargo test -p mos-core`
Esperado: PASS, sem regressão nos módulos vizinhos.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src/stale.rs crates/mos-core/src/lib.rs
git commit -m "feat(paradas): o que passou do limite, ordenado pelo excesso e nao pelos dias"
```

---

### Task 3: A atividade de todos os Projects, para o segundo consumidor

O widget `PROJECTS` acende o ponto com `project.updatedAt` — o mesmo campo errado. A função da Task 1 é a correção, e ela precisa chegar ao front. Este é o "uma função, dois consumidores" do spec.

**Files:**
- Modify: `crates/mos-core/src/stale.rs`
- Modify: `crates/mos-core/src/lib.rs`
- Test: inline em `crates/mos-core/src/stale.rs`

**Interfaces:**
- Consumes: `atividade_do_project` (Task 1), `Parada` (Task 2).
- Produces:
  - `pub struct ProjectActivity { project_id: ProjectId, last_activity: OffsetDateTime }`
  - `pub struct StaleView { paradas: Vec<Parada>, activity: Vec<ProjectActivity> }`
  - `pub fn project_activity(projects: &[Project], tasks: &[Task]) -> Vec<ProjectActivity>`

- [ ] **Step 1: Escrever o teste**

Acrescente ao `mod tests` de `stale.rs`:

```rust
    /// O widget PROJECTS acende o ponto com `project.updatedAt`, e por isso ele
    /// acende quando o Project e RENOMEADO. A mesma funcao que a lista de
    /// paradas usa e a que corrige o ponto — uma funcao, dois consumidores.
    #[test]
    fn a_atividade_sai_para_todos_os_projects_ativos() {
        let casa = project("Casa", instante() - Duration::days(30));
        let vazio = project("Vazio", instante() - Duration::days(4));
        let mut guardado = project("Guardado", instante());
        guardado.lifecycle_state = LifecycleState::Archived;

        let tasks = vec![task("hoje", TaskState::Doing, instante(), Some(&casa))];
        let projects = vec![casa.clone(), vazio.clone(), guardado];

        let atividade = project_activity(&projects, &tasks);
        assert_eq!(atividade.len(), 2, "o arquivado nao entra");
        assert_eq!(atividade[0].project_id, casa.id);
        assert_eq!(atividade[0].last_activity, instante());
        assert_eq!(atividade[1].project_id, vazio.id);
        assert_eq!(
            atividade[1].last_activity,
            instante() - Duration::days(4),
            "sem Task, cai no campo do proprio Project"
        );
    }
```

- [ ] **Step 2: Rodar e ver falhar**

Rode: `cargo test -p mos-core stale::tests::a_atividade_sai_para_todos_os_projects_ativos`
Esperado: FALHA na compilação — `cannot find function project_activity`.

- [ ] **Step 3: Implementar**

Acrescente a `stale.rs`, depois de `Parada`:

```rust
/// Quando um Project foi mexido de verdade.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectActivity {
    pub project_id: ProjectId,
    #[serde(with = "time::serde::rfc3339")]
    pub last_activity: OffsetDateTime,
}

/// O que o comando devolve: as paradas, e a atividade real de cada Project.
///
/// As duas coisas viajam juntas porque a tela precisa das duas ao mesmo tempo —
/// a lista de paradas para o widget novo, e a atividade para o ponto do widget
/// PROJECTS. Dois comandos fariam duas leituras das mesmas duas tabelas.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaleView {
    pub paradas: Vec<Parada>,
    pub activity: Vec<ProjectActivity>,
}

/// A atividade real de cada Project ativo, na ordem em que eles chegaram.
pub fn project_activity(projects: &[Project], tasks: &[Task]) -> Vec<ProjectActivity> {
    projects
        .iter()
        .filter(|project| project.lifecycle_state == LifecycleState::Active)
        .map(|project| ProjectActivity {
            project_id: project.id,
            last_activity: atividade_do_project(project, tasks),
        })
        .collect()
}
```

Amplie o `pub use` em `lib.rs`:

```rust
pub use stale::{
    atividade_do_project, compose_stale, project_activity, tolerancia, trabalho_aberto, Parada,
    ProjectActivity, StaleInput, StaleKind, StaleView, TOLERANCIA_PROJECT,
};
```

Se o teste reclamar que `Project` não é `Clone`, confirme em `work.rs:111` — ele deriva `Clone`; se não derivar, use referências no teste em vez de acrescentar derives.

- [ ] **Step 4: Rodar e ver passar**

Rode: `cargo test -p mos-core stale`
Esperado: PASS, todos.

- [ ] **Step 5: Commit**

```bash
git add crates/mos-core/src/stale.rs crates/mos-core/src/lib.rs
git commit -m "feat(paradas): a atividade real de cada Project sai junto, para o ponto da Home"
```

---

### Task 4: O comando, e o caminho até o front

**Files:**
- Create: `apps/desktop/src-tauri/src/stale.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs` (a lista `mod` na linha 28-41, e o `invoke_handler` perto da linha 1990)
- Modify: `apps/desktop/src/types.ts`
- Modify: `apps/desktop/src/api.ts`

**Interfaces:**
- Consumes: `mos_core::{compose_stale, project_activity, StaleInput, StaleView}` (Tasks 2 e 3); `AppState.work` (`WorkService`, com `tasks(bool)` e `projects(bool)`).
- Produces: comando Tauri `stale_list` (sem argumentos) devolvendo `StaleView`; `api.staleList(): Promise<StaleView>`; os tipos `Parada`, `ProjectActivity`, `StaleView` em `types.ts`.

- [ ] **Step 1: Escrever o comando**

Crie `apps/desktop/src-tauri/src/stale.rs`:

```rust
//! O que esta parado ha tempo demais.
//!
//! Mesma divisao do `calendar.rs`: a regra vive em `mos_core::stale`, que e pura
//! e testada, e este arquivo so BUSCA e delega. Comando Tauri nao se testa, e as
//! decisoes que podem estar erradas — o limiar de cada coluna, o que conta como
//! atividade, a ordem — sao justamente as regras.

use mos_core::{CoreError, StaleView};
use tauri::{AppHandle, Manager, Runtime};
use time::OffsetDateTime;

use crate::AppState;

/// As paradas de agora, e a atividade real de cada Project.
///
/// Sem argumento e sem janela: obsolescencia e sempre "ate agora", e um
/// parametro de data so ofereceria uma pergunta que ninguem faz.
#[tauri::command]
pub fn stale_list<R: Runtime>(app: AppHandle<R>) -> Result<StaleView, CoreError> {
    let state = app.state::<AppState>();

    // `false` ja exclui arquivadas e lixeira nos dois repositorios. A funcao
    // pura filtra de novo por lifecycle, e as duas defesas sao de proposito: a
    // do core e a que tem teste.
    let projects = state.work.projects(false)?;
    let tasks = state.work.tasks(false)?;

    let name_of = |id: mos_core::ProjectId| {
        projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project removido".to_owned())
    };

    Ok(StaleView {
        paradas: mos_core::compose_stale(mos_core::StaleInput {
            now: OffsetDateTime::now_utc(),
            tasks: &tasks,
            projects: &projects,
            project_name: &name_of,
        }),
        activity: mos_core::project_activity(&projects, &tasks),
    })
}
```

- [ ] **Step 2: Registrar o módulo e o comando**

Em `apps/desktop/src-tauri/src/lib.rs`, acrescente à lista de módulos (ela está entre as linhas 28 e 41, em ordem alfabética a partir de `mod daily;`):

```rust
mod stale;
```

E no `invoke_handler`, logo depois de `calendar::calendar_window,` (perto da linha 1990):

```rust
            stale::stale_list,
```

- [ ] **Step 3: Compilar**

Rode: `cargo check -p mos-desktop`
Esperado: compila sem erro. (Note a Global Constraint: `cargo test -p mos-desktop --lib` falha por linker pré-existente — `check` é o que vale aqui.)

- [ ] **Step 4: Declarar os tipos no front**

Em `apps/desktop/src/types.ts`, no fim do arquivo (depois de `WeekSummary`, que está na linha 1141):

```ts
/** O que está parado há tempo demais. Vem pronto de `mos-core::stale`. */
export type Parada = {
  kind: "task" | "project";
  id: string;
  title: string;
  /** Nome do Project, para Task. "N tasks abertas", para Project. */
  context: string;
  /** A coluna do Kanban, para Task. Vazio para Project. */
  state: string;
  days: number;
};

/** Quando um Project foi mexido de verdade — e não quando foi renomeado. */
export type ProjectActivity = { projectId: string; lastActivity: string };

export type StaleView = { paradas: Parada[]; activity: ProjectActivity[] };
```

- [ ] **Step 5: Escrever o wrapper da api**

Em `apps/desktop/src/api.ts`, acrescente `Parada, ProjectActivity, StaleView` ao import de `./types` (linha 8) e o método, logo depois do bloco `// ---- Weekly Review` (perto da linha 136):

```ts
  // ---------------------------------------------------------------- Paradas
  // Sem argumento: obsolescencia e sempre "ate agora".
  staleList() {
    return invoke<StaleView>("stale_list");
  },
```

- [ ] **Step 6: Conferir a compilação do TypeScript**

Rode, em `apps/desktop`: `npx tsc --noEmit`
Esperado: sem erro. (`Parada` e `ProjectActivity` ainda não são usados fora de `types.ts` — se o `tsc` reclamar de import não usado em `api.ts`, importe apenas `StaleView` ali.)

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/stale.rs apps/desktop/src-tauri/src/lib.rs apps/desktop/src/types.ts apps/desktop/src/api.ts
git commit -m "feat(paradas): o comando le os dois repositorios e delega para a funcao pura"
```

---

### Task 5: O que a tela decide, em `stale.ts`

Não há teste de DOM neste repositório (decisão registrada em `vitest.config.ts`). A consequência prática, igual à do `daily.ts` e do `weekly.ts`: tudo que decide alguma coisa mora num módulo puro, e o componente só desenha.

**Files:**
- Create: `apps/desktop/src/stale.ts`
- Create: `apps/desktop/src/stale.test.ts`

**Interfaces:**
- Consumes: `Parada`, `ProjectActivity` de `./types` (Task 4).
- Produces:
  - `export function rotuloDeDias(days: number): string`
  - `export function paradasVisiveis(paradas: Parada[], limite?: number): { visiveis: Parada[]; restantes: number }`
  - `export function diasPorTask(paradas: Parada[]): Map<string, number>`
  - `export function projectsParados(paradas: Parada[]): Set<string>`
  - `export function atividadePorProject(activity: ProjectActivity[]): Map<string, string>`
  - `export function mexidoHoje(iso: string | undefined, agora?: Date): boolean`

- [ ] **Step 1: Escrever os testes**

Crie `apps/desktop/src/stale.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  atividadePorProject,
  diasPorTask,
  mexidoHoje,
  paradasVisiveis,
  projectsParados,
  rotuloDeDias,
} from "./stale";
import type { Parada } from "./types";

function parada(over: Partial<Parada> = {}): Parada {
  return {
    kind: "task",
    id: "t1",
    title: "pintar a sala",
    context: "Casa",
    state: "doing",
    days: 12,
    ...over,
  };
}

describe("rótulo de dias", () => {
  it("é curto porque mora dentro de um card estreito", () => {
    expect(rotuloDeDias(12)).toBe("12d");
  });

  it("passa de 99 dias sem virar número gigante no card", () => {
    expect(rotuloDeDias(365)).toBe("99+d");
  });

  it("dia nenhum não vira rótulo", () => {
    expect(rotuloDeDias(0)).toBe("");
    expect(rotuloDeDias(-3)).toBe("");
  });
});

describe("corte da lista", () => {
  const cinco = Array.from({ length: 5 }, (_, indice) => parada({ id: `t${indice}` }));

  it("cinco cabem inteiras, e não sobra nada", () => {
    const { visiveis, restantes } = paradasVisiveis(cinco);
    expect(visiveis).toHaveLength(5);
    expect(restantes).toBe(0);
  });

  it("a sexta vira contagem em vez de sumir em silêncio", () => {
    const oito = Array.from({ length: 8 }, (_, indice) => parada({ id: `t${indice}` }));
    const { visiveis, restantes } = paradasVisiveis(oito);
    expect(visiveis).toHaveLength(5);
    expect(restantes).toBe(3);
  });

  it("preserva a ordem que veio do domínio", () => {
    // A ordem é o excesso proporcional, e ela é decidida no Rust. Reordenar
    // aqui seria uma segunda regra, divergindo em silêncio.
    const entrada = [parada({ id: "a" }), parada({ id: "b" }), parada({ id: "c" })];
    expect(paradasVisiveis(entrada).visiveis.map((item) => item.id)).toEqual(["a", "b", "c"]);
  });
});

describe("índices para o Kanban e a Home", () => {
  it("os dias de cada Task ficam achavéis por id", () => {
    const paradas = [parada({ id: "t1", days: 9 }), parada({ id: "t2", days: 40 })];
    const indice = diasPorTask(paradas);
    expect(indice.get("t1")).toBe(9);
    expect(indice.get("t2")).toBe(40);
  });

  it("o Project parado não entra no índice de Tasks", () => {
    const paradas = [parada({ kind: "project", id: "p1", days: 30 })];
    expect(diasPorTask(paradas).size).toBe(0);
    expect(projectsParados(paradas).has("p1")).toBe(true);
  });
});

describe("atividade do Project", () => {
  it("o ponto acende pela atividade real, e não pelo campo renomeado", () => {
    const indice = atividadePorProject([
      { projectId: "p1", lastActivity: "2026-08-22T09:00:00Z" },
    ]);
    expect(indice.get("p1")).toBe("2026-08-22T09:00:00Z");
  });

  it("mexido hoje compara a data local de quem está olhando", () => {
    const agora = new Date(2026, 7, 22, 15, 0, 0);
    const cedo = new Date(2026, 7, 22, 1, 0, 0).toISOString();
    const ontem = new Date(2026, 7, 21, 23, 0, 0).toISOString();
    expect(mexidoHoje(cedo, agora)).toBe(true);
    expect(mexidoHoje(ontem, agora)).toBe(false);
  });

  it("Project sem atividade conhecida não acende", () => {
    expect(mexidoHoje(undefined)).toBe(false);
    expect(mexidoHoje("lixo")).toBe(false);
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Rode, em `apps/desktop`: `npm test -- stale`
Esperado: FALHA — `Failed to resolve import "./stale"`.

- [ ] **Step 3: Escrever o módulo**

Crie `apps/desktop/src/stale.ts`:

```ts
/**
 * As paradas do lado da tela: só o que dá para verificar.
 *
 * Mesma divisão do `daily.ts` e do `weekly.ts`, e pelo mesmo motivo: não há
 * teste de DOM neste repositório (`vitest.config.ts`), então tudo que decide
 * alguma coisa — o rótulo, o corte da lista, quando o ponto acende — mora aqui,
 * e o componente só desenha o resultado.
 *
 * **Nenhuma regra de domínio.** Qual é a tolerância de cada coluna, o que conta
 * como atividade e em que ordem a lista sai vivem em `mos-core::stale`, com
 * teste. Aqui é apresentação.
 */
import type { Parada, ProjectActivity } from "./types";

/** Quantas paradas cabem no widget antes de o resto virar contagem. */
export const PARADAS_VISIVEIS = 5;

/**
 * "12d". Curto porque mora dentro de um card de Kanban estreito, ao lado do
 * título — "parada há 12 dias" empurraria o título para duas linhas em toda
 * task marcada.
 *
 * Acima de 99 vira "99+d": o número exato de um abandono de um ano não muda
 * decisão nenhuma, e três dígitos quebram a linha do card.
 */
export function rotuloDeDias(days: number): string {
  if (!Number.isFinite(days) || days <= 0) return "";
  return days > 99 ? "99+d" : `${Math.trunc(days)}d`;
}

/**
 * As primeiras, e quantas ficaram de fora.
 *
 * O resto vira contagem em vez de sumir: uma lista cortada em silêncio faz o
 * widget dizer "cinco paradas" quando são vinte.
 */
export function paradasVisiveis(
  paradas: Parada[],
  limite = PARADAS_VISIVEIS,
): { visiveis: Parada[]; restantes: number } {
  return {
    visiveis: paradas.slice(0, limite),
    restantes: Math.max(0, paradas.length - limite),
  };
}

/** Id da Task para dias parados. É o que o card do Kanban consulta. */
export function diasPorTask(paradas: Parada[]): Map<string, number> {
  return new Map(
    paradas.filter((parada) => parada.kind === "task").map((parada) => [parada.id, parada.days]),
  );
}

/** Os ids dos Projects parados. */
export function projectsParados(paradas: Parada[]): Set<string> {
  return new Set(
    paradas.filter((parada) => parada.kind === "project").map((parada) => parada.id),
  );
}

/** Id do Project para o instante da última atividade real dele. */
export function atividadePorProject(activity: ProjectActivity[]): Map<string, string> {
  return new Map(activity.map((linha) => [linha.projectId, linha.lastActivity]));
}

/**
 * O instante caiu no dia de hoje, no fuso de quem está olhando.
 *
 * O dia é local aqui de propósito, e isso não contradiz o domínio: lá a conta é
 * de DURAÇÃO ("parado há 12 dias") e não precisa de fuso; aqui a pergunta é
 * "mexi nisto hoje?", que é uma data civil, e data civil é do renderer — o mesmo
 * raciocínio do `calendar.rs`.
 */
export function mexidoHoje(iso: string | undefined, agora = new Date()): boolean {
  if (!iso) return false;
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return false;
  return quando.toDateString() === agora.toDateString();
}
```

- [ ] **Step 4: Rodar e ver passar**

Rode, em `apps/desktop`: `npm test -- stale`
Esperado: PASS, os nove testes.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/stale.ts apps/desktop/src/stale.test.ts
git commit -m "feat(paradas): o rotulo, o corte em cinco e os indices, com teste"
```

---

### Task 6: O widget PARADAS na faixa Retomar

**Files:**
- Modify: `apps/desktop/src/homeLayout.ts` (o catálogo `HOME_WIDGETS`, linha 85-87)
- Modify: `apps/desktop/src/App.tsx` (o `refresh` na linha 3089; a assinatura de `HomePage` na linha 501; a lista de widgets perto da linha 780; a chamada de `HomePage` na linha 3456)
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `api.staleList()` (Task 4); `paradasVisiveis`, `rotuloDeDias` (Task 5); `Panel` de `./Surface`; `DataRow` (`App.tsx:482`).
- Produces: o estado `stale: StaleView` no componente raiz, passado a `HomePage` como prop `stale`; o widget de id `stale`.

- [ ] **Step 1: Acrescentar o widget ao catálogo**

Em `apps/desktop/src/homeLayout.ts`, na lista `HOME_WIDGETS`, logo **depois** de `inbox_pulse` (o `INBOX` é o vizinho de propósito: os dois são atenção, e o desenho os quer lado a lado):

```ts
  { id: "inbox_pulse", label: "INBOX", section: "resume", role: "attention", span: 3 },
  /* PARADAS ao lado do INBOX porque as duas perguntas sao a mesma familia: o que
     entrou e nao foi decidido, e o que foi decidido e nao andou. Faixa
     "Retomar" e o lugar literal do nome dela.

     Largura 3: a lista e de cinco linhas curtas, e um quarto de linha as
     comporta sem quebrar. */
  { id: "stale", label: "PARADAS", section: "resume", role: "attention", span: 3 },
```

- [ ] **Step 2: Carregar o dado no refresh**

Em `apps/desktop/src/App.tsx`, junto das outras declarações de estado do componente raiz (perto de onde `const [tasks, setTasks] = useState<Task[]>([])` vive):

```tsx
  const [stale, setStale] = useState<StaleView>({ paradas: [], activity: [] });
```

Acrescente `StaleView` ao import de `./types` no topo do arquivo.

No `refresh` (linha 3089), acrescente `api.staleList()` ao `Promise.all` e o `setStale` na linha de baixo. O array desestruturado ganha `nextStale` no fim:

```tsx
      const [nextRecent, nextInbox, nextArchived, nextTrashed, nextProjects, nextWorkspaces, nextApps, nextResources, nextTrashedResources, nextTasks, nextStatus, nextHiddenWidgets, nextResourceWorkspaces, nextWidgetPlacements, nextRadialPins, nextIngestions, nextStale] = await Promise.all([api.recent(), api.inbox(), api.archived(), api.trashed(), api.projects(true), api.workspaces(true), api.registeredApps(true), api.resources(true), api.trashedResources(), api.tasks(true), api.status(), api.hiddenWidgets(), api.resourceWorkspaces(), api.widgetPlacements(), api.radialPins(), api.ingestions(), api.staleList()]);
```

E, na linha do `setRecent(...)`, acrescente ao fim:

```tsx
setStale(nextStale);
```

- [ ] **Step 3: Passar para a Home e para o Kanban**

Na assinatura de `HomePage` (linha 501), acrescente `stale` às props desestruturadas e `stale: StaleView` ao tipo.

Na chamada de `HomePage` (linha 3456), acrescente `stale={stale}`.

Na chamada de `BoardPage` (procure `page === "tasks"` no mesmo bloco de roteamento), acrescente `stale={stale}` — a Task 7 usa.

- [ ] **Step 4: Desenhar o widget**

Em `apps/desktop/src/App.tsx`, na lista de widgets da Home, logo **depois** da entrada `{ id: "inbox_pulse", ... }` (linha 780):

```tsx
        { id: "stale", ...(paradasDaHome.restantes ? { footRight: `E MAIS ${paradasDaHome.restantes}` } : {}), node: <Panel label="PARADAS" value={String(stale.paradas.length)} unit={stale.paradas.length === 1 ? "parada" : "paradas"}>
          {paradasDaHome.visiveis.map((parada) => <DataRow
            key={`${parada.kind}-${parada.id}`}
            primary={parada.title}
            secondary={parada.context || undefined}
            meta={rotuloDeDias(parada.days)}
            onClick={() => abrirParada(parada)}
          />)}
          {/* Vazio nao e falha, e o texto diz isso: "nada parado" e um bom
              resultado, e um estado vazio de erro faria o widget parecer
              quebrado no dia em que tudo esta em dia. */}
          {!stale.paradas.length ? <p className="widget-empty">Nada parado.</p> : null}
        </Panel> },
```

Logo antes do `return` do `HomePage` (junto das outras derivações como `inboxCapped`), acrescente:

```tsx
  const paradasDaHome = paradasVisiveis(stale.paradas);
  /* Task abre a gaveta; Project abre o Project. A parada e onde se NOTA, e o
     clique leva a onde se AGE — sem acao em massa aqui, porque uma lista que se
     resolve num clique convida a limpar sem decidir. */
  function abrirParada(parada: Parada) {
    if (parada.kind === "task") {
      const alvo = tasks.find((task) => task.id === parada.id);
      if (alvo) openTask(alvo);
      return;
    }
    const alvo = projects.find((project) => project.id === parada.id);
    if (alvo) openProject(alvo);
  }
```

Acrescente ao import de `./stale` no topo do `App.tsx`:

```tsx
import { atividadePorProject, diasPorTask, mexidoHoje, paradasVisiveis, projectsParados, rotuloDeDias } from "./stale";
```

(As três primeiras são usadas nas Tasks 7 e 8; deixe o import completo agora para não voltar ao topo do arquivo três vezes. Se o `tsc` reclamar de símbolo não usado, acrescente-os apenas quando cada task os usar.)

E `Parada` ao import de `./types`.

- [ ] **Step 5: O estado vazio no CSS**

Confira se `.widget-empty` já existe em `apps/desktop/src/App.css`:

```bash
grep -n "widget-empty" apps/desktop/src/App.css
```

Se não existir, acrescente perto de `.kanban-empty` (linha 3985):

```css
/* Vazio dito em voz baixa: "nada parado" e um bom resultado, e nao um aviso. */
.widget-empty {
  color: var(--text-secondary);
  font: var(--text-small);
  padding: var(--space-2) 0;
}
```

Se existir com outro nome (`.panel-empty`, por exemplo), use o que já existe em vez de criar um segundo.

- [ ] **Step 6: Conferir**

Rode, em `apps/desktop`: `npx tsc --noEmit && npm test`
Esperado: sem erro de tipo, e a suíte inteira PASSA (inclusive `homeLayout.test.ts`, que pode ter uma contagem de widgets — se falhar por causa do widget novo, o teste está afirmando o tamanho do catálogo e precisa acompanhar o número novo).

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/homeLayout.ts apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(paradas): o widget na faixa Retomar, e o clique leva a onde se age"
```

---

### Task 7: A marca no card do Kanban

O widget é onde se **nota**; o card é onde se **age**, arrastando ali mesmo. Mesma divisão que o `INBOX` já tem.

**Files:**
- Modify: `apps/desktop/src/App.tsx` (`DataRow`, linha 482; `BoardPage`, linha 2202 e o card na linha ~2303)
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `diasPorTask`, `rotuloDeDias` (Task 5); `stale` (Task 6).
- Produces: `DataRow` ganha a prop `stale?: boolean`, que vira `data-stale` no botão.

- [ ] **Step 1: Dar ao `DataRow` o atributo**

Em `apps/desktop/src/App.tsx:482`, acrescente `stale = false` à desestruturação e `stale?: boolean` ao tipo; no `<button>` da linha 483, acrescente:

```tsx
data-stale={stale || undefined}
```

(`|| undefined` e não `|| false`: é o padrão que as outras flags da mesma linha usam — atributo ausente em vez de `data-stale="false"`.)

- [ ] **Step 2: Marcar o card**

Em `BoardPage` (linha 2202), acrescente `stale` às props e `stale: StaleView` ao tipo. Logo depois de `const activeTasks = ...` (linha 2209):

```tsx
  const diasParados = diasPorTask(stale.paradas);
```

No `DataRow` do card (perto da linha 2303), acrescente as duas props:

```tsx
stale={diasParados.has(task.id)}
meta={rotuloDeDias(diasParados.get(task.id) ?? 0)}
```

`rotuloDeDias(0)` devolve string vazia, e o `DataRow` já não renderiza `.row-meta` quando `meta` é vazio — nenhuma condicional a mais é necessária.

- [ ] **Step 3: Desenhar**

Em `apps/desktop/src/App.css`, junto do bloco `.kanban-column .data-row` (linha 3995):

```css
/* Parada: uma barra de sodio na borda de ataque, e o numero de dias na meta.
   Nao um fundo colorido — o quadro tem seis colunas e um fundo por card
   transformaria o Kanban num semaforo. A barra marca sem tingir. */
.kanban-column .data-row[data-stale] {
  border-left: 2px solid var(--signal-fill);
  padding-left: calc(var(--space-3) - 2px);
}

.kanban-column .data-row[data-stale] .row-meta {
  color: var(--signal-fill);
}
```

Confira o `padding-left` real do `.kanban-column .data-row` antes de escrever o `calc`:

```bash
sed -n '3995,4012p' apps/desktop/src/App.css
```

Se o padding lá for outro token, use aquele — a subtração dos 2px da borda é o que impede o texto de dançar 2px ao ser marcado.

- [ ] **Step 4: Conferir**

Rode, em `apps/desktop`: `npx tsc --noEmit && npm test`
Esperado: sem erro, suíte PASSA.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(paradas): o card do quadro diz ha quantos dias parou"
```

---

### Task 8: O ponto do widget PROJECTS passa a dizer a verdade

O defeito existente: o ponto acende quando o Project é **renomeado**, não quando se trabalha nele.

**Files:**
- Modify: `apps/desktop/src/App.tsx` (linha 728, `isActiveToday`; linha 782, o widget `projects`)
- Modify: `apps/desktop/src/App.css` (`.project-dot`, linha 2694)

**Interfaces:**
- Consumes: `atividadePorProject`, `mexidoHoje`, `projectsParados` (Task 5); `stale` (Task 6).
- Produces: nada novo — corrige o consumidor existente.

- [ ] **Step 1: Trocar a fonte do ponto**

Em `apps/desktop/src/App.tsx`, substitua a linha 728:

```tsx
  const isActiveToday = (project: Project) => new Date(project.updatedAt).toDateString() === new Date().toDateString();
```

por:

```tsx
  /* A atividade vem das Tasks do Project, e nao do `updatedAt` dele.
     Aquele campo so muda quando o Project e EDITADO: criar Task, mover no
     Kanban e concluir nao o tocam. O ponto acendia ao RENOMEAR. */
  const atividade = atividadePorProject(stale.activity);
  const parados = projectsParados(stale.paradas);
  const atividadeDe = (project: Project) => atividade.get(project.id) ?? project.updatedAt;
  const isActiveToday = (project: Project) => mexidoHoje(atividadeDe(project));
```

- [ ] **Step 2: Marcar o estado oposto e corrigir a meta**

No widget `projects` (linha 782), o `marker` e o `meta` do `DataRow` passam a:

```tsx
marker={<span className="project-dot" data-active={isActiveToday(project) || undefined} data-stale={parados.has(project.id) || undefined} aria-hidden="true" />}
meta={relativeTime(atividadeDe(project))}
```

A meta muda junto de propósito: `relativeTime(project.updatedAt)` dizia "há 2 minutos" para um Project renomeado e abandonado há um mês. O ponto e o texto precisam contar a mesma história.

- [ ] **Step 3: Desenhar o estado oposto**

Em `apps/desktop/src/App.css`, depois de `.project-dot[data-active]` (linha 2702):

```css
/* Parado: o ponto se esvazia. Cheio-sodio = mexido hoje, cheio-cinza = normal,
   OCO = parado ha tempo demais. Tres estados numa forma so, sem cor de alarme —
   um ponto vermelho na Home pediria acao imediata, e obsolescencia e uma
   pergunta, nao uma urgencia. */
.project-dot[data-stale] {
  background: transparent;
  box-shadow: inset 0 0 0 1px var(--border-strong);
}
```

- [ ] **Step 4: Conferir**

Rode, em `apps/desktop`: `npx tsc --noEmit && npm test`
Esperado: sem erro, suíte PASSA.

- [ ] **Step 5: Ver na tela**

O widget `PARADAS`, a marca no card e o ponto do `PROJECTS` são mudanças visuais, e teste nenhum as vê. Use a skill `ver-o-app` do repositório para subir o M/OS e capturar a janela:

1. Home — o widget `PARADAS` na faixa Retomar, ao lado do `INBOX`.
2. Aba Tasks — o card marcado, com a barra e o "12d".
3. Home — o ponto do `PROJECTS`, nos três estados que houver.

Se o banco de trabalho não tiver nada parado, crie uma Task em `doing` e ajuste `updated_at` para trás **numa cópia do banco**, nunca no banco de trabalho:

```sql
UPDATE tasks SET updated_at = '2026-08-01T09:00:00Z' WHERE id = '<id>';
```

Confira na tela e desfaça depois. Com o app rodando, ler o `m-os.db` de fora devolve dado velho — a tela é a fonte da verdade.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "fix(paradas): o ponto do PROJECTS acende por trabalho, e nao por renomeacao"
```

---

### Task 9: O registro — ADR, matriz e README

**Files:**
- Modify: `docs/DECISIONS.md` (acrescentar ADR-056 no fim; o último é o ADR-055, linha 2721)
- Modify: `docs/PLATFORMS.md` (a matriz de features, linha 120-143)
- Modify: `README.md` (a lista de capacidades, linha 11-20)
- Modify: `docs/superpowers/specs/2026-08-21-obsolescencia-design.md` (o Status)

**Interfaces:**
- Consumes: tudo o que as Tasks 1-8 construíram.
- Produces: nada de código.

- [ ] **Step 1: Escrever o ADR-056**

No fim de `docs/DECISIONS.md`:

```markdown
---

## ADR-056 — Obsolescência é por coluna, e o Project vive pelas Tasks dele

**Estado:** Accepted · 2026-08-22

### Contexto

O pedido chegou como `IDEAS.md` #56 — o "retrato de estado". Ao explorar, três
dos cinco itens dele já existiam na Home: Projects ativos é o widget `PROJECTS`,
Tasks concluídas são o `CONCLUÍDO` e o `TASKS NA SEMANA`, e o Inbox é o
`INBOX`, que já envelhece em três dias. Dos dois que sobravam, "próximas
prioridades" não tem lastro — `Task` não tem campo de prioridade, só `Reminder`
tem — e, depois da Daily Session, "o que vem" já é respondido pelo Start My Day.

O que era novo de verdade era "Tasks paradas": o #57 e o #58.

### Decisão

**O limiar é por coluna do Kanban, e não um número único.** `doing` e `review`
em 7 dias, `planned` em 21, `inbox` em 14; `backlog` e `done` não têm limiar
nenhum. Um limiar único transformaria o backlog inteiro num alerta permanente —
num sistema com meses de uso o backlog domina a lista e afoga o sinal. Com
limiar por coluna, o resultado típico é três paradas, e não quarenta e sete.

**A atividade de um Project é a atividade das Tasks dele** — `max(task.updated_at)`,
caindo no campo do próprio Project só quando ele não tem Task nenhuma.
`projects.updated_at` só muda quando o Project é *editado*: apenas
`update_project` e `set_project_lifecycle` escrevem naquela coluna. Criar Task,
mover no Kanban, concluir — nada disso a toca. Usá-la como sinal marcaria como
"parado" o Project em que se trabalhou ontem, e como "vivo" o que foi renomeado
e abandonado. **Esse defeito já existia**: o ponto do widget `PROJECTS` acendia
ao renomear, e a mesma função que a lista de paradas precisa é a que o corrige.
Uma função, dois consumidores.

**Project só entra quando tem trabalho aberto.** Project sem Task aberta e sem
atividade não está travado — ele acabou e ninguém arquivou, que é outra pergunta
e merece outra resposta.

**A ordem é o excesso proporcional, e não os dias crus.** Uma Task 12 dias
parada num limiar de 7 está a 171%; uma 24 dias num limiar de 21 está a 114%.
Ordenar por dias colocaria a segunda primeiro, e ela é a menos urgente das duas.

**Zero persistência.** Nenhuma migration, nenhuma tabela, nenhuma operação de
sync: obsolescência é uma leitura de `updated_at`, que já existe. Os limiares são
fixos pelo mesmo motivo que os 3 dias do `INBOX` são — um número que se ajusta
uma vez não paga uma migration, uma tabela e uma tela.

### Consequências

- `mos-core::stale` é puro e testado; o comando `stale_list` só busca e delega.
- Três superfícies e nenhuma nova: o widget `PARADAS` na faixa Retomar, a marca
  no card do Kanban, e o ponto corrigido do `PROJECTS`.
- Não há ação em massa. Uma lista que se resolve num clique convida a limpar sem
  decidir; o gesto certo já existe no Kanban, arrastando.
- Não há notificação. Mesma decisão do §8 do `DAILY-SESSION.md`.
- Prioridade em `Task` continua não existindo. É a ausência que o #56 revelou, e
  é outra feature — maior que esta.
```

- [ ] **Step 2: Acrescentar a linha na matriz**

Em `docs/PLATFORMS.md`, na tabela que começa na linha 120, depois da linha `| Knowledge Graph (relações) | ... |`:

```markdown
| Obsolescência (paradas) | ✓ | ✓ | — | n/a (derivada) | lê |
```

- [ ] **Step 3: Acrescentar a linha no README**

Em `README.md`, na lista de capacidades, depois da linha da Weekly Review:

```markdown
- Obsolescência: o que está parado há tempo demais, por coluna do Kanban
  (ver `DECISIONS.md`, ADR-056);
```

- [ ] **Step 4: Fechar o spec**

Em `docs/superpowers/specs/2026-08-21-obsolescencia-design.md`, troque o bloco de Status por:

```markdown
**Status:** ✅ **IMPLEMENTADO** em 2026-08-22. Ver `DECISIONS.md`, ADR-056, e o
plano em `docs/superpowers/plans/2026-08-22-obsolescencia.md`.
```

- [ ] **Step 5: A verificação final, inteira**

```bash
cargo test --workspace --exclude mos-desktop
cargo check -p mos-desktop
```

E, em `apps/desktop`:

```bash
npx tsc --noEmit
npm test
```

Esperado: os quatro passam. Cole a saída real — nenhuma afirmação de "pronto" antes de ver os quatro verdes.

- [ ] **Step 6: Commit**

```bash
git add docs/DECISIONS.md docs/PLATFORMS.md README.md docs/superpowers/specs/2026-08-21-obsolescencia-design.md
git commit -m "docs(paradas): ADR-056, e a obsolescencia entra na matriz"
```

---

## Auto-revisão contra o spec

| Seção do spec | Onde o plano a cumpre |
| --- | --- |
| §2.1 escopo só obsolescência | Nenhuma task toca no retrato de estado nem em prioridade de Task |
| §2.2 limiar por coluna | Task 1, `tolerancia` |
| §2.3 widget + marca no card | Tasks 6 e 7 |
| §2.4 limiares fixos, zero persistência | Global Constraints; nenhuma migration em nenhuma task |
| §3 atividade do Project, e o defeito existente | Tasks 1 e 8 |
| §4 os seis limiares e o de Project | Task 1, Steps 2-3 |
| §4 "não há fuso aqui" | `dias_parado` em duração; `mexidoHoje` documenta por que o *front* usa data local |
| §5 ordem por excesso proporcional | Task 2, produto cruzado + teste |
| §6 `stale.rs` puro, assinaturas | Tasks 1-3 |
| §6 três superfícies | Tasks 6, 7, 8 |
| §7 os oito testes listados | Task 1 (atividade, Project sem Task), Task 2 (limiar por coluna, dois `None`, fronteira 6/7/8, sem trabalho aberto, ordenação, arquivado e lixeira), Task 5 (`stale.ts`: rótulo, corte em cinco, "e mais N") |
| §8 fora de escopo | Nada de limiar configurável, ação em massa, ação do Hermes, notificação, #56 literal, prioridade em Task |

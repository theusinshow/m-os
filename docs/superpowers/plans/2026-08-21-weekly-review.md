# Weekly Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fechar a semana em menos de dois minutos, vendo o que dominou, o que voltou toda vez e o que foi largado — sem placar de produtividade.

**Architecture:** Uma entidade minúscula (`WeeklyReview`: semana + texto), porque a narrativa inteira é **derivada** das Daily Sessions por uma função pura `compose_week`. A semana é identificada pela **data da segunda-feira**, nunca por número ISO. A tela é uma terceira aba na gaveta que já existe, e o gatilho reusa a linha discreta que a Home já desenha para "dia em aberto".

**Tech Stack:** Rust (mos-core puro, mos-storage-sqlite com rusqlite, mos-desktop com Tauri 2), TypeScript + React 19 + framer-motion, vitest para funções puras.

**Spec:** `docs/superpowers/specs/2026-08-21-weekly-review-design.md`

## Global Constraints

- **Nada de plataforma em `mos-core`.** Um `#[cfg(windows)]` ali significa que o desenho quebrou (`FEATURE-DEVELOPMENT.md` §3).
- **Nenhuma alteração em tabela existente.** A migration 0029 só acrescenta (`FEATURE-DEVELOPMENT.md` §4).
- **Toda mutação emite a operação de sync DENTRO da mesma transação** (`sync_emit.rs`).
- **Proibido mostrar placar de produtividade.** Nenhum `X de Y` de objetivos na semana. `ATTENTION-SYSTEM.md` §19.
- **A regra da semana existe em UM lugar:** `Week::containing`. Nada de `date(day,'weekday 0','-6 days')` em SQL.
- **Comentário explica POR QUÊ, não o quê.** É o padrão de todo o repositório.
- **Sem acento em mensagem de commit** (padrão do histórico). Código e comentário levam acento normalmente.
- **Todo comando `cargo` precisa de `TMP`/`TEMP` apontando para diretório gravável** nesta máquina — ver "Ambiente" abaixo.
- Testes de front só cobrem `.ts` puro. Não há DOM no runner (`vitest.config.ts`).

## Ambiente

Antes de qualquer `cargo`, na sessão do shell:

```bash
export TMP="/c/WINDOWS/TEMP/claude/scratch" TEMP="$TMP"
mkdir -p "$TMP"
```

`cargo test -p mos-desktop --lib` **falha nesta máquina** com `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139), por um problema de linker que **já existia antes deste trabalho**. Use sempre:

```bash
cargo test --workspace --exclude mos-desktop
cargo check -p mos-desktop
```

## Estrutura de arquivos

| Arquivo | Responsabilidade | Task |
|---|---|---|
| `crates/mos-core/src/weekly.rs` (novo) | `Week`, `WeeklyReview`, `WeekSummary`, `compose_week`. Puro. | 1, 2 |
| `crates/mos-core/src/daily.rs` | ganha `Day::date()` | 1 |
| `crates/mos-core/src/lib.rs` | registra e exporta o módulo | 1 |
| `crates/mos-core/src/ports.rs` | 4 métodos novos no `DailyRepository` | 3, 4 |
| `crates/mos-core/src/service.rs` | 5 métodos novos no `DailyService` | 5 |
| `crates/mos-storage-sqlite/migrations/0029_weekly_review.sql` (novo) | a tabela | 3 |
| `crates/mos-storage-sqlite/src/lib.rs` | registra a migration | 3 |
| `crates/mos-storage-sqlite/src/daily_repository.rs` | implementa os 4 métodos | 3, 4 |
| `crates/mos-storage-sqlite/tests/weekly_review.rs` (novo) | testes contra banco real | 3, 5 |
| `apps/desktop/src-tauri/src/daily.rs` | 3 comandos novos + `project_of` | 6 |
| `apps/desktop/src-tauri/src/lib.rs` | registra os comandos | 6 |
| `apps/desktop/src/weekly.ts` (novo) | apresentação pura | 7 |
| `apps/desktop/src/weekly.test.ts` (novo) | teste dela | 7 |
| `apps/desktop/src/WeeklyReview.tsx` (novo) | o painel da aba | 8 |
| `apps/desktop/src/DailySession.tsx` | terceira aba + linha na Home | 8, 9 |
| `apps/desktop/src/App.tsx` | passa a semana pendente ao widget | 9 |
| `apps/desktop/src/types.ts`, `api.ts`, `App.css` | contrato e estilo | 6, 8 |
| `docs/DAILY-SESSION.md`, `docs/DECISIONS.md` | registro | 10 |

---

### Task 1: `Week` — a segunda-feira como identidade

**Files:**
- Create: `crates/mos-core/src/weekly.rs`
- Modify: `crates/mos-core/src/daily.rs` (adicionar `Day::date`)
- Modify: `crates/mos-core/src/lib.rs`
- Test: dentro de `crates/mos-core/src/weekly.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `mos_core::{Day, CoreError, ErrorCode}` — `Day::parse(&str) -> Result<Day, CoreError>`, `Day::as_str(&self) -> &str`.
- Produces:
  - `Day::date(&self) -> Result<time::Date, CoreError>`
  - `Week` (`Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`, serializa como a string da segunda)
  - `Week::containing(&Day) -> Result<Week, CoreError>`
  - `Week::parse(&str) -> Result<Week, CoreError>`
  - `Week::start(&self) -> &Day`
  - `Week::end(&self) -> Result<Day, CoreError>`
  - `Week::previous(&self) -> Result<Week, CoreError>`
  - `Week::next(&self) -> Result<Week, CoreError>`
  - `Week::contains(&self, &Day) -> Result<bool, CoreError>`

- [ ] **Step 1: Escrever o teste que falha**

Criar `crates/mos-core/src/weekly.rs` com só o módulo de testes e os `use`:

```rust
//! Weekly Review: o fecho da semana sobre as Daily Sessions.

use serde::{Deserialize, Serialize};
use time::Duration;

use crate::{CoreError, Day, ErrorCode};

#[cfg(test)]
mod tests {
    use super::*;

    fn dia(valor: &str) -> Day {
        Day::parse(valor).unwrap()
    }

    #[test]
    fn a_semana_e_a_segunda_que_contem_o_dia() {
        // 2026-08-21 e uma sexta. A semana dela comeca em 17 e termina em 23.
        let semana = Week::containing(&dia("2026-08-21")).unwrap();
        assert_eq!(semana.start().as_str(), "2026-08-17");
        assert_eq!(semana.end().unwrap().as_str(), "2026-08-23");
    }

    #[test]
    fn a_segunda_e_o_domingo_caem_na_mesma_semana() {
        // As duas bordas sao o caso que uma conta de "menos N dias" erra: a
        // segunda nao pode andar para tras, e o domingo nao pode virar a
        // semana seguinte.
        let segunda = Week::containing(&dia("2026-08-17")).unwrap();
        let domingo = Week::containing(&dia("2026-08-23")).unwrap();
        assert_eq!(segunda, domingo);
        assert_eq!(segunda.start().as_str(), "2026-08-17");
    }

    #[test]
    fn a_semana_atravessa_mes_e_ano_sem_numero_iso() {
        // 2026-01-01 e uma quinta, e a semana dela comeca em 2025-12-29. Com
        // numero ISO isto seria "semana 1 de 2026" comecando em 2025, que e a
        // convencao que este desenho recusa por nao precisar dela.
        let virada = Week::containing(&dia("2026-01-01")).unwrap();
        assert_eq!(virada.start().as_str(), "2025-12-29");
        assert_eq!(virada.end().unwrap().as_str(), "2026-01-04");
    }

    #[test]
    fn anterior_e_proxima_andam_sete_dias() {
        let semana = Week::containing(&dia("2026-03-02")).unwrap();
        assert_eq!(semana.previous().unwrap().start().as_str(), "2026-02-23");
        assert_eq!(semana.next().unwrap().start().as_str(), "2026-03-09");
    }

    #[test]
    fn ano_bissexto_nao_quebra_a_conta() {
        // 2028 e bissexto. A semana que contem 29/02 comeca em 28/02.
        let semana = Week::containing(&dia("2028-02-29")).unwrap();
        assert_eq!(semana.start().as_str(), "2028-02-28");
        assert_eq!(semana.next().unwrap().start().as_str(), "2028-03-06");
    }

    #[test]
    fn a_semana_so_aceita_segunda_feira() {
        // Ela e CHAVE: duas representacoes do mesmo intervalo criariam duas
        // linhas para a mesma semana, e o indice unico nao veria a duplicata.
        assert!(Week::parse("2026-08-17").is_ok());
        assert!(Week::parse("2026-08-21").is_err(), "sexta nao e inicio de semana");
        assert!(Week::parse("2026-8-17").is_err(), "sem zero a esquerda e outra chave");
        assert!(Week::parse("").is_err());
    }

    #[test]
    fn contains_responde_pelas_duas_bordas() {
        let semana = Week::containing(&dia("2026-08-19")).unwrap();
        assert!(semana.contains(&dia("2026-08-17")).unwrap());
        assert!(semana.contains(&dia("2026-08-23")).unwrap());
        assert!(!semana.contains(&dia("2026-08-16")).unwrap());
        assert!(!semana.contains(&dia("2026-08-24")).unwrap());
    }

    #[test]
    fn a_semana_atravessa_a_ponte_como_a_data_da_segunda() {
        // O nome vai para o TypeScript e para o banco. Um formato diferente de
        // cada lado faria a tela deixar de reconhecer a semana sem erro de
        // compilacao de nenhum dos dois.
        let semana = Week::containing(&dia("2026-08-19")).unwrap();
        assert_eq!(serde_json::to_string(&semana).unwrap(), "\"2026-08-17\"");
        assert_eq!(
            serde_json::from_str::<Week>("\"2026-08-17\"").unwrap(),
            semana
        );
    }

    #[test]
    fn as_semanas_se_ordenam_no_tempo() {
        // `pending_week` escolhe a mais recente com `max()`. Sem Ord correto,
        // ela escolheria por ordem alfabetica — que por sorte coincide neste
        // formato, e e exatamente o tipo de sorte que quebra quando o formato
        // muda.
        let antes = Week::containing(&dia("2025-12-31")).unwrap();
        let depois = Week::containing(&dia("2026-01-05")).unwrap();
        assert!(antes < depois);
    }
}
```

- [ ] **Step 2: Rodar o teste e verificar que falha**

Run: `cargo test -p mos-core weekly`
Expected: FAIL na compilação — `cannot find type Week in this scope` (o módulo ainda não está registrado, e `Week` não existe).

- [ ] **Step 3: Adicionar `Day::date` em `daily.rs`**

Em `crates/mos-core/src/daily.rs`, dentro de `impl Day`, logo depois de `as_str`:

```rust
    /// A data, para quem precisa fazer conta de calendario com ela.
    ///
    /// `Result` e nao `Date` direto porque `from_local` tem um caminho de
    /// formatacao que pode falhar e hoje cai num texto vazio — um `Day`
    /// invalido nao deveria existir, e enquanto ele puder, quem faz conta
    /// precisa poder recusar em vez de inventar uma data.
    pub fn date(&self) -> Result<time::Date, CoreError> {
        time::Date::parse(&self.0, DAY_FORMAT).map_err(|_| {
            CoreError::new(
                ErrorCode::DataIntegrity,
                "Data persistida e invalida.",
                false,
            )
        })
    }

    /// O contrario: a data vira `Day`.
    pub(crate) fn from_date(date: time::Date) -> Result<Self, CoreError> {
        date.format(DAY_FORMAT)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::DataIntegrity, "Data ilegivel.", false))
    }
```

- [ ] **Step 4: Escrever `Week` em `weekly.rs`**

No topo de `crates/mos-core/src/weekly.rs`, **antes** do `mod tests`:

```rust
/// Uma semana civil, identificada pela **data da segunda-feira**.
///
/// # Por que a segunda, e nao o numero ISO
///
/// Numero ISO tem duas armadilhas que a data da segunda simplesmente nao tem:
/// **semanas 53**, e o 1º de janeiro que pertence a semana 52 do ano anterior.
/// Guardar `2026-W01` obrigaria a escolher uma convencao de virada de ano e a
/// acerta-la em todo lugar que compara; guardar `2026-08-17` nao obriga a nada.
///
/// # Por que civil e fixa, e nao sete dias deslizantes
///
/// Um "fecho" de janela deslizante nao fecha nada, e a unicidade precisa de uma
/// chave. E a mesma razao pela qual [`Day`] existe como campo em vez de ser
/// decidido por cada leitor.
///
/// **Esta e a unica copia da regra.** Nada de `date(day, 'weekday 0', '-6
/// days')` em SQL: a semana calculada em dois lugares e como as duas versoes
/// divergem.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Week(Day);

impl Week {
    /// A semana que contem este dia.
    pub fn containing(day: &Day) -> Result<Self, CoreError> {
        let date = day.date()?;
        // `number_days_from_monday` da 0 na segunda e 6 no domingo, entao a
        // segunda nao anda e o domingo volta seis. As duas bordas sao o caso
        // que uma conta ingenua erra.
        let recuo = i64::from(date.weekday().number_days_from_monday());
        Day::from_date(date - Duration::days(recuo)).map(Self)
    }

    /// Le a data de uma segunda-feira. Recusa qualquer outro dia.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        let day = Day::parse(value)?;
        let date = day.date()?;
        if date.weekday().number_days_from_monday() != 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A semana e identificada pela segunda-feira.",
                false,
            ));
        }
        Ok(Self(day))
    }

    pub fn start(&self) -> &Day {
        &self.0
    }

    pub fn end(&self) -> Result<Day, CoreError> {
        Day::from_date(self.0.date()? + Duration::days(6))
    }

    pub fn previous(&self) -> Result<Self, CoreError> {
        Day::from_date(self.0.date()? - Duration::days(7)).map(Self)
    }

    pub fn next(&self) -> Result<Self, CoreError> {
        Day::from_date(self.0.date()? + Duration::days(7)).map(Self)
    }

    /// As duas bordas entram.
    pub fn contains(&self, day: &Day) -> Result<bool, CoreError> {
        Ok(*day >= *self.start() && *day <= self.end()?)
    }
}

impl std::fmt::Display for Week {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
```

- [ ] **Step 5: Registrar e exportar o módulo**

Em `crates/mos-core/src/lib.rs`, na lista de `mod` (ordem alfabética, depois de `mod voice_when;`... na verdade a lista termina em `mod work;` — inserir antes dele):

```rust
mod weekly;
mod work;
```

E o `pub use`, logo antes de `pub use work::{`:

```rust
pub use weekly::Week;
```

- [ ] **Step 6: Rodar os testes e verificar que passam**

Run: `cargo test -p mos-core weekly`
Expected: PASS — 9 testes.

- [ ] **Step 7: Rodar o resto do core para garantir que nada quebrou**

Run: `cargo test -p mos-core`
Expected: PASS — 386 anteriores + 9 novos = 395.

- [ ] **Step 8: Commit**

```bash
git add crates/mos-core/src/weekly.rs crates/mos-core/src/daily.rs crates/mos-core/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(semana): a semana e a segunda-feira, e nao o numero ISO

Numero ISO tem duas armadilhas que a data da segunda nao tem: semanas 53, e
o 1º de janeiro que pertence a semana 52 do ano anterior. Guardar `2026-W01`
obrigaria a escolher uma convencao de virada de ano e a acerta-la em todo
lugar que compara.

`Week::containing` e a UNICA copia da regra. A conta usa
`number_days_from_monday`, que da zero na segunda e seis no domingo — as duas
bordas sao exatamente o caso que uma subtracao ingenua erra.

`Day` ganhou `date()` devolvendo Result: um Day invalido nao deveria existir,
e enquanto `from_local` puder produzir um, quem faz conta de calendario
precisa poder recusar em vez de inventar uma data.

Nove testes, incluindo as duas bordas, a virada de ano e o ano bissexto.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: A narrativa — `compose_week`

**Files:**
- Modify: `crates/mos-core/src/weekly.rs`
- Modify: `crates/mos-core/src/lib.rs` (exportar os tipos novos)
- Test: dentro de `crates/mos-core/src/weekly.rs`

**Interfaces:**
- Consumes: `Week` (Task 1); `mos_core::{DailySession, DailyObjective, DailyReflection, DailyObjectiveId, ObjectiveLink, ObjectivePriority, ObjectiveStatus, DayMood, normalize}`.
- Produces:
  - `WeeklyReviewId` (mesmo macro de id das outras entidades)
  - `WeeklyReview { id, week, summary, closed_at, created_at, updated_at }`
  - `NewWeeklyReview { id, week, summary, closed_at }` com `NewWeeklyReview::create(Week, &str, OffsetDateTime) -> Self`
  - `Dominant { label: String, main_days: usize, days: usize }`
  - `Recurring { title: String, times_carried: usize }`
  - `WeekSummary { week, days_with_session, dominated, recurring, dropped, blocked_days, review, empty }`
  - `WeekInput<'a> { week, sessions, objectives, reflections, project_of, carry_depth }`
  - `compose_week(WeekInput) -> Result<WeekSummary, CoreError>`

- [ ] **Step 1: Escrever os testes que falham**

Adicionar dentro de `mod tests` em `crates/mos-core/src/weekly.rs`:

```rust
    use crate::{
        DailyObjective, DailyObjectiveId, DailyReflection, DailySession, DailySessionId, DayMood,
        LinkKind, ObjectiveLink, ObjectivePriority, ObjectiveStatus, SessionStatus,
    };
    use time::macros::datetime;
    use time::OffsetDateTime;

    fn instante() -> OffsetDateTime {
        datetime!(2026-08-17 09:00 -03:00)
    }

    fn sessao(day: &str) -> DailySession {
        DailySession {
            id: DailySessionId::new(),
            day: dia(day),
            status: SessionStatus::Completed,
            note: String::new(),
            started_at: instante(),
            ended_at: Some(instante()),
            created_at: instante(),
            updated_at: instante(),
        }
    }

    fn objetivo(
        sessao: &DailySession,
        title: &str,
        priority: ObjectivePriority,
        status: ObjectiveStatus,
        link: Option<ObjectiveLink>,
    ) -> DailyObjective {
        DailyObjective {
            id: DailyObjectiveId::new(),
            session_id: sessao.id,
            title: title.to_owned(),
            description: String::new(),
            link,
            priority,
            status,
            position: 0,
            carried_from: None,
            created_at: instante(),
            updated_at: instante(),
            completed_at: None,
        }
    }

    fn link_task(id: &str) -> ObjectiveLink {
        ObjectiveLink::new(LinkKind::Task, id).unwrap()
    }

    const TASK_A: &str = "018f0000-0000-7000-8000-0000000000a1";
    const TASK_B: &str = "018f0000-0000-7000-8000-0000000000b2";

    fn semana_de_teste() -> Week {
        Week::containing(&dia("2026-08-19")).unwrap()
    }

    /// Monta a entrada com fechamentos triviais. Os testes que precisam de
    /// Project ou de profundidade sobrescrevem depois.
    fn entrada<'a>(
        semana: &'a Week,
        sessions: &'a [DailySession],
        objectives: &'a [DailyObjective],
        reflections: &'a [DailyReflection],
        project_of: &'a dyn Fn(&ObjectiveLink) -> Option<String>,
        carry_depth: &'a dyn Fn(DailyObjectiveId) -> usize,
    ) -> WeekInput<'a> {
        WeekInput {
            week: semana.clone(),
            sessions,
            objectives,
            reflections,
            project_of,
            carry_depth,
        }
    }

    #[test]
    fn conta_dias_com_sessao_e_ignora_o_que_esta_fora_da_semana() {
        let semana = semana_de_teste();
        let sessoes = [sessao("2026-08-17"), sessao("2026-08-19"), sessao("2026-08-24")];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &[],
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.days_with_session, 2, "24/08 e da semana seguinte");
        assert!(!resumo.empty);
    }

    #[test]
    fn semana_sem_sessao_nenhuma_e_marcada_como_vazia() {
        // A tela usa isto para NAO oferecer o fecho: nao ha o que revisar, e um
        // botao ali ensinaria que o M/OS quer um registro por semana mesmo
        // quando nao houve semana.
        let semana = semana_de_teste();
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo =
            compose_week(entrada(&semana, &[], &[], &[], &sem_project, &sem_profundidade)).unwrap();
        assert!(resumo.empty);
        assert_eq!(resumo.days_with_session, 0);
    }

    #[test]
    fn o_que_dominou_agrupa_por_project_quando_o_vinculo_resolve() {
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let terca = sessao("2026-08-18");
        let objetivos = vec![
            objetivo(&segunda, "Planta de formas", ObjectivePriority::Main, ObjectiveStatus::Completed, Some(link_task(TASK_A))),
            objetivo(&terca, "Detalhamento", ObjectivePriority::Main, ObjectiveStatus::Pending, Some(link_task(TASK_B))),
        ];
        let sessoes = [segunda, terca];
        // As duas Tasks pertencem ao MESMO Project: e essa a agregacao que a
        // tela precisa, e ela e invisivel olhando so para os titulos.
        let project_of = |_: &ObjectiveLink| Some("063-26".to_owned());
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &project_of,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dominated.len(), 1);
        assert_eq!(resumo.dominated[0].label, "063-26");
        assert_eq!(resumo.dominated[0].main_days, 2);
        assert_eq!(resumo.dominated[0].days, 2);
    }

    #[test]
    fn o_que_dominou_cai_no_titulo_quando_nao_ha_vinculo() {
        // O caso mais comum do inicio. Agrupar so por Project deixaria a unica
        // secao que responde "onde foi meu tempo" vazia numa semana inteira de
        // texto livre.
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let terca = sessao("2026-08-18");
        let objetivos = vec![
            objetivo(&segunda, "Resolver pendencias financeiras", ObjectivePriority::Main, ObjectiveStatus::Completed, None),
            objetivo(&terca, "resolver PENDENCIAS financeiras", ObjectivePriority::Secondary, ObjectiveStatus::Pending, None),
        ];
        let sessoes = [segunda, terca];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dominated.len(), 1, "caixa diferente e o mesmo assunto");
        assert_eq!(resumo.dominated[0].main_days, 1);
        assert_eq!(resumo.dominated[0].days, 2);
        assert_eq!(
            resumo.dominated[0].label, "Resolver pendencias financeiras",
            "o rotulo e o primeiro titulo escrito, e nao a forma normalizada"
        );
    }

    #[test]
    fn dominou_ordena_por_dias_como_principal() {
        // Ser principal tres vezes e um fato mais forte que aparecer cinco
        // vezes como secundario.
        let semana = semana_de_teste();
        let dias: Vec<DailySession> = ["2026-08-17", "2026-08-18", "2026-08-19", "2026-08-20"]
            .iter()
            .map(|d| sessao(d))
            .collect();
        let mut objetivos = Vec::new();
        for dia_da_semana in &dias[..2] {
            objetivos.push(objetivo(dia_da_semana, "Principal duas vezes", ObjectivePriority::Main, ObjectiveStatus::Pending, None));
        }
        for dia_da_semana in &dias {
            objetivos.push(objetivo(dia_da_semana, "Secundario sempre", ObjectivePriority::Secondary, ObjectiveStatus::Pending, None));
        }
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &dias,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dominated[0].label, "Principal duas vezes");
        assert_eq!(resumo.dominated[1].label, "Secundario sempre");
        assert_eq!(resumo.dominated[1].days, 4, "e ele apareceu mais vezes");
    }

    #[test]
    fn o_que_voltou_toda_vez_corta_em_dois_e_aparece_uma_vez_so() {
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let terca = sessao("2026-08-18");
        let veterano_a = objetivo(&segunda, "Atualizar documentacao", ObjectivePriority::Secondary, ObjectiveStatus::CarriedOver, None);
        let mut veterano_b = objetivo(&terca, "Atualizar documentacao", ObjectivePriority::Secondary, ObjectiveStatus::CarriedOver, None);
        // A corrente: o de terca veio do de segunda.
        veterano_b.carried_from = Some(veterano_a.id);
        let novato = objetivo(&terca, "Veio de ontem", ObjectivePriority::Secondary, ObjectiveStatus::Pending, None);

        let id_a = veterano_a.id;
        let id_b = veterano_b.id;
        let objetivos = vec![veterano_a, veterano_b, novato];
        let sessoes = [segunda, terca];
        let sem_project = |_: &ObjectiveLink| None;
        let profundidade = move |id: DailyObjectiveId| {
            if id == id_a { 3 } else if id == id_b { 4 } else { 1 }
        };
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.recurring.len(), 1, "a corrente e uma linha, e nao duas");
        assert_eq!(resumo.recurring[0].title, "Atualizar documentacao");
        assert_eq!(
            resumo.recurring[0].times_carried, 4,
            "a profundidade e a do elo mais recente"
        );
    }

    #[test]
    fn o_que_voce_largou_lista_os_abandonados() {
        let semana = semana_de_teste();
        let segunda = sessao("2026-08-17");
        let objetivos = vec![
            objetivo(&segunda, "Revisar proposta antiga", ObjectivePriority::Secondary, ObjectiveStatus::Dropped, None),
            objetivo(&segunda, "Feito", ObjectivePriority::Main, ObjectiveStatus::Completed, None),
        ];
        let sessoes = [segunda];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dropped, ["Revisar proposta antiga"]);
    }

    #[test]
    fn dias_travados_vem_dos_humores_ja_respondidos() {
        // NAO e uma pergunta nova: perguntar o humor da semana no domingo seria
        // pedir a mesma coisa uma oitava vez, com menos precisao.
        let semana = semana_de_teste();
        let quarta = sessao("2026-08-19");
        let quinta = sessao("2026-08-20");
        let sexta = sessao("2026-08-21");
        let reflexoes = vec![
            DailyReflection { session_id: quarta.id, mood: Some(DayMood::Blocked), summary: String::new(), created_at: instante(), updated_at: instante() },
            DailyReflection { session_id: quinta.id, mood: Some(DayMood::Blocked), summary: String::new(), created_at: instante(), updated_at: instante() },
            DailyReflection { session_id: sexta.id, mood: Some(DayMood::Productive), summary: String::new(), created_at: instante(), updated_at: instante() },
        ];
        let sessoes = [quarta, quinta, sexta];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &[],
            &reflexoes,
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        let dias: Vec<&str> = resumo.blocked_days.iter().map(|d| d.as_str()).collect();
        assert_eq!(dias, ["2026-08-19", "2026-08-20"]);
    }

    #[test]
    fn objetivo_de_sessao_fora_da_semana_nao_entra_em_nada() {
        // As sessoes chegam filtradas pelo repositorio, mas os objetivos podem
        // vir de um lote maior. Confiar no chamador aqui seria a semana mostrar
        // o trabalho de outra.
        let semana = semana_de_teste();
        let dentro = sessao("2026-08-18");
        let fora = sessao("2026-08-25");
        let objetivos = vec![
            objetivo(&dentro, "Da semana", ObjectivePriority::Main, ObjectiveStatus::Dropped, None),
            objetivo(&fora, "De outra semana", ObjectivePriority::Main, ObjectiveStatus::Dropped, None),
        ];
        let sessoes = [dentro];
        let sem_project = |_: &ObjectiveLink| None;
        let sem_profundidade = |_: DailyObjectiveId| 0;
        let resumo = compose_week(entrada(
            &semana,
            &sessoes,
            &objetivos,
            &[],
            &sem_project,
            &sem_profundidade,
        ))
        .unwrap();
        assert_eq!(resumo.dropped, ["Da semana"]);
    }
```

- [ ] **Step 2: Rodar e verificar que falha**

Run: `cargo test -p mos-core weekly`
Expected: FAIL na compilação — `cannot find function compose_week`, `cannot find type WeekInput`.

- [ ] **Step 3: Escrever os tipos e a função**

Adicionar em `crates/mos-core/src/weekly.rs`, depois do `impl Week` e antes do `mod tests`:

```rust
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    DailyObjective, DailyObjectiveId, DailyReflection, DailySession, DayMood, ObjectiveLink,
    ObjectivePriority, ObjectiveStatus,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeeklyReviewId(Uuid);

impl WeeklyReviewId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value).map(Self).map_err(|_| {
            CoreError::new(ErrorCode::InvalidInput, "ID de fecho de semana invalido.", false)
        })
    }

    /// O UUID cru, para a sincronizacao. Ver `docs/SYNC.md`.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WeeklyReviewId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WeeklyReviewId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// O fecho de uma semana.
///
/// **Minusculo de proposito: a narrativa inteira e DERIVADA.** Guardar o resumo
/// duplicaria dado para exibir noutra superficie, que o `CORE-FOUNDATION.md` §2
/// principio 6 proibe — e ele envelheceria: reabrir um objetivo de terca
/// mudaria a semana, e o texto gravado continuaria dizendo o contrario.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReview {
    pub id: WeeklyReviewId,
    pub week: Week,
    /// Vazio e legitimo: fechar a semana e o gesto, escrever e opcional.
    pub summary: String,
    #[serde(with = "time::serde::rfc3339")]
    pub closed_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewWeeklyReview {
    pub id: WeeklyReviewId,
    pub week: Week,
    pub summary: String,
    pub closed_at: OffsetDateTime,
}

/// Teto do texto da semana. Sete dias cabem em dois paragrafos; o que passa
/// disso e journaling, que este desenho recusou por nome.
const MAX_SUMMARY: usize = 4_000;

impl NewWeeklyReview {
    /// Texto vazio NAO impede o fecho.
    ///
    /// Difere do `NewDailyReflection::create`, que devolve `None` quando nao ha
    /// nada a guardar: la a reflexao e acessorio do encerramento; aqui ela e o
    /// unico campo, e a linha precisa existir para a semana constar como
    /// fechada.
    pub fn create(week: Week, summary: &str, now: OffsetDateTime) -> Self {
        let summary = summary.trim();
        Self {
            id: WeeklyReviewId::new(),
            week,
            summary: summary.chars().take(MAX_SUMMARY).collect(),
            closed_at: now,
        }
    }
}

/// O que ocupou a semana: um Project, ou um objetivo que se repetiu.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dominant {
    pub label: String,
    /// Em quantos dias isto foi o objetivo PRINCIPAL.
    pub main_days: usize,
    /// Em quantos dias apareceu, de qualquer peso.
    pub days: usize,
}

/// Um objetivo que atravessou a semana sendo adiado.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurring {
    pub title: String,
    pub times_carried: usize,
}

/// A semana em narrativa. **Nenhum placar.**
///
/// `ATTENTION-SYSTEM.md` §19 proibe resumo de produtividade em digest semanal,
/// e a razao vale aqui: um numero que soma sete dias de decisoes numa fracao
/// ensina a inflar o denominador na segunda e a evitar objetivo dificil na
/// quinta. A unica contagem que sobrevive e `days_with_session`, que e fato
/// sobre o uso do sistema e nao sobre o trabalho.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekSummary {
    pub week: Week,
    pub days_with_session: usize,
    pub dominated: Vec<Dominant>,
    pub recurring: Vec<Recurring>,
    pub dropped: Vec<String>,
    pub blocked_days: Vec<Day>,
    pub review: Option<WeeklyReview>,
    /// Nenhuma sessao na semana. A tela usa isto para NAO oferecer o fecho.
    pub empty: bool,
}

/// Tudo o que [`compose_week`] precisa ler.
///
/// Estrutura em vez de seis parametros soltos, pela mesma razao do
/// `ComposeInput` do calendario: trocar duas colecoes de lugar por engano
/// compilaria sem reclamacao nenhuma.
pub struct WeekInput<'a> {
    pub week: Week,
    pub sessions: &'a [DailySession],
    pub objectives: &'a [DailyObjective],
    pub reflections: &'a [DailyReflection],
    /// Como achar o Project de um vinculo. Fechamento e nao mapa pronto porque
    /// so o comando do desktop conhece Tasks e Projects.
    pub project_of: &'a dyn Fn(&ObjectiveLink) -> Option<String>,
    pub carry_depth: &'a dyn Fn(DailyObjectiveId) -> usize,
}

/// A partir de quantos elos um carry-over vira assunto.
///
/// Dois, e o mesmo corte do `avisoDeCarregado` no front: quase todo carry-over
/// veio de ontem, e "veio de ontem" e ruido.
const MIN_CORRENTE: usize = 2;

/// Monta a narrativa da semana a partir do que ja foi lido.
///
/// PURA e sem repositorio, igual ao `calendar::compose` e ao
/// `daily::compose_context`: e ela que carrega as regras que podem estar
/// erradas — como se agrupa o que dominou, o que conta como recorrente, o que
/// fica de fora —, e regra sem teste e regra que ninguem conferiu.
pub fn compose_week(input: WeekInput<'_>) -> Result<WeekSummary, CoreError> {
    use std::collections::HashMap;

    // As sessoes DA SEMANA. O repositorio ja filtra, e filtrar de novo aqui e
    // barato: confiar no chamador seria a semana mostrar o trabalho de outra.
    let mut dias: Vec<&DailySession> = Vec::new();
    for session in input.sessions {
        if input.week.contains(&session.day)? {
            dias.push(session);
        }
    }
    let da_semana: std::collections::HashSet<_> = dias.iter().map(|s| s.id).collect();
    let dia_da_sessao: HashMap<_, _> = dias.iter().map(|s| (s.id, s.day.clone())).collect();

    let meus: Vec<&DailyObjective> = input
        .objectives
        .iter()
        .filter(|objetivo| da_semana.contains(&objetivo.session_id))
        .collect();

    // ---------------------------------------------------------- o que dominou
    //
    // A chave e o Project quando o vinculo resolve, e o titulo normalizado
    // quando nao resolve. Agrupar so por Project deixaria a unica secao que
    // responde "onde foi meu tempo" vazia numa semana inteira de texto livre —
    // que e o caso mais comum de quem esta comecando.
    struct Acumulado {
        label: String,
        principais: std::collections::HashSet<Day>,
        dias: std::collections::HashSet<Day>,
    }
    let mut grupos: HashMap<String, Acumulado> = HashMap::new();
    for objetivo in &meus {
        let Some(dia) = dia_da_sessao.get(&objetivo.session_id).cloned() else {
            continue;
        };
        let project = objetivo.link.as_ref().and_then(|link| (input.project_of)(link));
        let (chave, rotulo) = match project {
            Some(nome) => (format!("p:{}", crate::normalize(&nome)), nome),
            None => (
                format!("t:{}", crate::normalize(&objetivo.title)),
                objetivo.title.clone(),
            ),
        };
        let entrada = grupos.entry(chave).or_insert_with(|| Acumulado {
            // O rotulo e o PRIMEIRO titulo escrito, e nao a forma normalizada:
            // ninguem quer ler o proprio objetivo sem acento e em caixa baixa.
            label: rotulo,
            principais: std::collections::HashSet::new(),
            dias: std::collections::HashSet::new(),
        });
        entrada.dias.insert(dia.clone());
        if objetivo.priority == ObjectivePriority::Main {
            entrada.principais.insert(dia);
        }
    }
    let mut dominated: Vec<Dominant> = grupos
        .into_values()
        .map(|acumulado| Dominant {
            label: acumulado.label,
            main_days: acumulado.principais.len(),
            days: acumulado.dias.len(),
        })
        .collect();
    // Ser principal tres vezes e um fato mais forte que aparecer cinco vezes
    // como secundario. Empate desempata pelo rotulo, para a ordem nao dancar
    // entre duas leituras — `HashMap` nao promete ordem nenhuma.
    dominated.sort_by(|esquerda, direita| {
        direita
            .main_days
            .cmp(&esquerda.main_days)
            .then(direita.days.cmp(&esquerda.days))
            .then(esquerda.label.cmp(&direita.label))
    });

    // ------------------------------------------------ o que voltou toda vez
    //
    // Uma corrente que atravessa a semana aparece UMA vez, com a profundidade
    // do elo mais recente: cinco linhas iguais seriam a mesma informacao
    // repetida cinco vezes.
    let elos_anteriores: std::collections::HashSet<DailyObjectiveId> = meus
        .iter()
        .filter_map(|objetivo| objetivo.carried_from)
        .collect();
    let mut recurring: Vec<Recurring> = meus
        .iter()
        // Quem e elo intermediario DENTRO da semana sai: o elo final carrega a
        // corrente inteira.
        .filter(|objetivo| !elos_anteriores.contains(&objetivo.id))
        .filter_map(|objetivo| {
            let vezes = (input.carry_depth)(objetivo.id);
            (vezes >= MIN_CORRENTE).then(|| Recurring {
                title: objetivo.title.clone(),
                times_carried: vezes,
            })
        })
        .collect();
    recurring.sort_by(|esquerda, direita| {
        direita
            .times_carried
            .cmp(&esquerda.times_carried)
            .then(esquerda.title.cmp(&direita.title))
    });

    // -------------------------------------------------- o que voce largou
    let mut dropped: Vec<String> = meus
        .iter()
        .filter(|objetivo| objetivo.status == ObjectiveStatus::Dropped)
        .map(|objetivo| objetivo.title.clone())
        .collect();
    dropped.sort();
    dropped.dedup();

    // ------------------------------------------------------ dias travados
    let mut blocked_days: Vec<Day> = input
        .reflections
        .iter()
        .filter(|reflexao| reflexao.mood == Some(DayMood::Blocked))
        .filter_map(|reflexao| dia_da_sessao.get(&reflexao.session_id).cloned())
        .collect();
    blocked_days.sort();
    blocked_days.dedup();

    Ok(WeekSummary {
        week: input.week,
        days_with_session: dias.len(),
        dominated,
        recurring,
        dropped,
        blocked_days,
        review: None,
        empty: dias.is_empty(),
    })
}
```

- [ ] **Step 4: Exportar os tipos**

Em `crates/mos-core/src/lib.rs`, trocar a linha `pub use weekly::Week;` por:

```rust
pub use weekly::{
    compose_week, Dominant, NewWeeklyReview, Recurring, Week, WeekInput, WeekSummary, WeeklyReview,
    WeeklyReviewId,
};
```

- [ ] **Step 5: Rodar e verificar que passam**

Run: `cargo test -p mos-core weekly`
Expected: PASS — 18 testes (9 da Task 1 + 9 novos).

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src/weekly.rs crates/mos-core/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(semana): a narrativa da semana e derivada, e a entidade e minuscula

`WeeklyReview` guarda uma semana e um texto, e mais nada. Guardar o resumo
duplicaria dado para exibir noutra superficie — o que o CORE-FOUNDATION §2
principio 6 proibe — e ele envelheceria: reabrir um objetivo de terca mudaria
a semana, e o texto gravado continuaria dizendo o contrario.

NENHUM PLACAR. `WeekSummary` nao tem "X de Y", e a unica contagem que
sobrevive e `days_with_session`, que e fato sobre o uso do sistema e nao
sobre o trabalho. O ATTENTION-SYSTEM §19 ja proibia resumo de produtividade
em digest semanal.

"O que dominou" agrupa por Project quando o vinculo resolve e PELO TITULO
quando nao resolve. Agrupar so por Project falharia em silencio no caso mais
comum do inicio: uma semana inteira de objetivos em texto livre mostraria
vazia justamente a secao que responde onde foi o tempo.

"O que voltou toda vez" segue a CORRENTE e nao o titulo: quem e elo
intermediario dentro da semana sai, e o elo final carrega a profundidade
inteira. Cinco linhas iguais seriam a mesma informacao repetida cinco vezes.

Nove testes novos.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Migration 0029 e persistência

**Files:**
- Create: `crates/mos-storage-sqlite/migrations/0029_weekly_review.sql`
- Create: `crates/mos-storage-sqlite/tests/weekly_review.rs`
- Modify: `crates/mos-storage-sqlite/src/lib.rs`
- Modify: `crates/mos-core/src/ports.rs`
- Modify: `crates/mos-storage-sqlite/src/daily_repository.rs`

**Interfaces:**
- Consumes: `Week`, `WeeklyReview`, `NewWeeklyReview`, `WeeklyReviewId` (Task 2); `SqliteStorage::{emitir, emitir_update}`; `repository::{format_time, parse_time}`.
- Produces, no trait `DailyRepository`:
  - `fn sessions_between(&self, from: &Week) -> Result<Vec<DailySession>, CoreError>`
  - `fn weekly_review(&self, week: &Week) -> Result<Option<WeeklyReview>, CoreError>`
  - `fn save_weekly_review(&self, review: NewWeeklyReview, now: OffsetDateTime) -> Result<WeeklyReview, CoreError>`
  - `fn weekly_reviews(&self, limit: usize) -> Result<Vec<WeeklyReview>, CoreError>`

- [ ] **Step 1: Escrever o teste que falha**

Criar `crates/mos-storage-sqlite/tests/weekly_review.rs`:

```rust
//! O fecho da semana contra um banco de verdade.
//!
//! O que se prova aqui e o que so o banco pode desmentir: a unicidade por
//! semana, o upsert preservando o instante do fecho, a janela de sessoes nas
//! duas bordas, e a emissao de sync na mesma transacao.

use mos_core::{
    DailyRepository, Day, NewDailyObjective, NewDailySession, NewWeeklyReview, ObjectivePriority,
    Week,
};
use mos_storage_sqlite::SqliteStorage;
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

fn banco() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();
    (dir, storage)
}

fn com_sync() -> (tempfile::TempDir, SqliteStorage) {
    use mos_sync::DeviceRepository;
    let (dir, storage) = banco();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    (dir, storage)
}

fn agora() -> OffsetDateTime {
    datetime!(2026-08-23 18:00 -03:00)
}

fn dia(valor: &str) -> Day {
    Day::parse(valor).unwrap()
}

fn semana(valor: &str) -> Week {
    Week::containing(&dia(valor)).unwrap()
}

/// Cria uma sessao com um objetivo, num dia.
fn sessao_em(storage: &SqliteStorage, day: &str, titulo: &str) {
    let quando = agora();
    let nova = NewDailySession::create(dia(day), "", quando).unwrap();
    let id = nova.id;
    let objetivo =
        NewDailyObjective::create(id, titulo, "", None, ObjectivePriority::Main, 0, quando).unwrap();
    storage.start_day(nova, vec![objetivo], quando).unwrap();
}

#[test]
fn a_janela_da_semana_pega_as_duas_bordas() {
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-16", "domingo anterior");
    sessao_em(&storage, "2026-08-17", "segunda");
    sessao_em(&storage, "2026-08-23", "domingo");
    sessao_em(&storage, "2026-08-24", "segunda seguinte");

    let dentro = storage.sessions_between(&semana("2026-08-19")).unwrap();
    let dias: Vec<&str> = dentro.iter().map(|s| s.day.as_str()).collect();
    assert_eq!(dias, ["2026-08-17", "2026-08-23"], "as duas bordas entram, e so elas");
}

#[test]
fn fechar_a_semana_grava_e_le_de_volta() {
    let (_dir, storage) = banco();
    let alvo = semana("2026-08-19");
    assert!(storage.weekly_review(&alvo).unwrap().is_none());

    let fechada = storage
        .save_weekly_review(
            NewWeeklyReview::create(alvo.clone(), "  o 063-26 tomou a semana  ", agora()),
            agora(),
        )
        .unwrap();
    assert_eq!(fechada.week, alvo);
    assert_eq!(fechada.summary, "o 063-26 tomou a semana");

    let lida = storage.weekly_review(&alvo).unwrap().unwrap();
    assert_eq!(lida.id, fechada.id);
}

#[test]
fn texto_vazio_ainda_fecha_a_semana() {
    // Fechar e o gesto; escrever e opcional. Difere da reflexao do dia, que e
    // acessorio do encerramento — aqui o texto e o unico campo, e a linha
    // precisa existir para a semana constar como fechada.
    let (_dir, storage) = banco();
    let alvo = semana("2026-08-19");
    storage
        .save_weekly_review(NewWeeklyReview::create(alvo.clone(), "   ", agora()), agora())
        .unwrap();
    let lida = storage.weekly_review(&alvo).unwrap().unwrap();
    assert_eq!(lida.summary, "");
}

#[test]
fn regravar_preserva_o_instante_do_fecho() {
    // Editar o texto na quarta nao pode dizer que a semana foi fechada na
    // quarta: quando ela foi fechada e um fato, e o texto e outro.
    let (_dir, storage) = banco();
    let alvo = semana("2026-08-19");
    let primeira = storage
        .save_weekly_review(NewWeeklyReview::create(alvo.clone(), "primeira", agora()), agora())
        .unwrap();

    let depois = agora() + Duration::days(3);
    let segunda = storage
        .save_weekly_review(NewWeeklyReview::create(alvo.clone(), "corrigida", depois), depois)
        .unwrap();

    assert_eq!(segunda.summary, "corrigida");
    assert_eq!(segunda.closed_at, primeira.closed_at, "o fecho nao se move");
    assert_eq!(segunda.id, primeira.id, "e o registro continua sendo o mesmo");
    assert!(segunda.updated_at > primeira.updated_at);
    assert_eq!(storage.weekly_reviews(10).unwrap().len(), 1, "uma linha por semana");
}

#[test]
fn as_semanas_vem_da_mais_recente_para_a_mais_antiga() {
    let (_dir, storage) = banco();
    for valor in ["2026-08-05", "2026-08-19", "2026-08-12"] {
        storage
            .save_weekly_review(NewWeeklyReview::create(semana(valor), "", agora()), agora())
            .unwrap();
    }
    let semanas: Vec<String> = storage
        .weekly_reviews(10)
        .unwrap()
        .iter()
        .map(|review| review.week.to_string())
        .collect();
    assert_eq!(semanas, ["2026-08-17", "2026-08-10", "2026-08-03"]);
}

#[test]
fn fechar_a_semana_emite_a_operacao() {
    use mos_sync::{OpBody, OutboxRepository};
    let (_dir, storage) = com_sync();
    let alvo = semana("2026-08-19");
    let fechada = storage
        .save_weekly_review(NewWeeklyReview::create(alvo, "foi boa", agora()), agora())
        .unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].entity.kind.as_str(), "weekly_review");
    assert_eq!(ops[0].entity.id, fechada.id.as_uuid());
    let campos = match &ops[0].body {
        OpBody::Create { fields } | OpBody::Update { fields } => fields.clone(),
        outro => panic!("esperava campos, veio {outro:?}"),
    };
    assert_eq!(
        campos["weekStart"],
        serde_json::json!("2026-08-17"),
        "a semana precisa viajar sabendo de que semana ela e: o id e um UUID e nao diz nada"
    );
}

#[test]
fn sem_sync_ligado_nada_e_emitido_e_nada_falha() {
    use mos_sync::OutboxRepository;
    let (_dir, storage) = banco();
    storage
        .save_weekly_review(NewWeeklyReview::create(semana("2026-08-19"), "x", agora()), agora())
        .unwrap();
    assert!(storage.pendentes(10).unwrap().is_empty());
}
```

- [ ] **Step 2: Rodar e verificar que falha**

Run: `cargo test -p mos-storage-sqlite --test weekly_review`
Expected: FAIL na compilação — `no method named sessions_between`.

- [ ] **Step 3: Escrever a migration**

Criar `crates/mos-storage-sqlite/migrations/0029_weekly_review.sql`:

```sql
-- Migration 0029: o fecho da semana.
--
-- UMA tabela nova, e nenhuma alteracao em tabela existente — a mesma regra da
-- 0027 e da 0028. Se o desenho da revisao semanal mudar, ela se apaga sem tocar
-- em uma linha do que existe.
--
-- A tabela e minuscula porque A NARRATIVA INTEIRA E DERIVADA das Daily
-- Sessions. Guardar o resumo duplicaria dado para exibir noutra superficie, e
-- ele envelheceria: reabrir um objetivo de terca mudaria a semana, e o texto
-- gravado continuaria dizendo o contrario.

BEGIN IMMEDIATE;

CREATE TABLE weekly_reviews (
    id          TEXT PRIMARY KEY NOT NULL,

    -- A data da SEGUNDA-FEIRA da semana, `AAAA-MM-DD`.
    --
    -- Nao e numero ISO, e a escolha evita duas armadilhas: semanas 53, e o 1º
    -- de janeiro que pertence a semana 52 do ano anterior. `2026-W01` obrigaria
    -- a escolher uma convencao de virada de ano e a acerta-la em todo lugar que
    -- compara; `2026-08-17` nao obriga a nada.
    week_start  TEXT NOT NULL,

    -- Vazio e legitimo: fechar a semana e o gesto, escrever e opcional.
    summary     TEXT NOT NULL DEFAULT '',

    -- Quando a semana foi fechada. NAO se move ao editar o texto: quando ela
    -- foi fechada e um fato, e o texto e outro.
    closed_at   TEXT NOT NULL,

    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,

    -- Formato exato, e nao "parece uma data": `2026-8-17` e `2026-08-17` sao a
    -- mesma semana e duas chaves diferentes, e o indice unico abaixo nao veria
    -- a duplicata. Mesma razao do CHECK da 0028.
    CONSTRAINT weekly_reviews_week_shape CHECK (
        week_start GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    )
);

-- O CHECK NAO verifica que e segunda-feira. SQLite conseguiria, com
-- `strftime('%w', week_start) = '1'`, e isso seria a regra da semana escrita num
-- segundo lugar. Quem garante e `Week`, que e o unico construtor.

CREATE UNIQUE INDEX weekly_reviews_one_per_week ON weekly_reviews (week_start);

PRAGMA user_version = 29;

COMMIT;
```

- [ ] **Step 4: Registrar a migration**

Em `crates/mos-storage-sqlite/src/lib.rs`, três edições:

```rust
const SCHEMA_VERSION: u32 = 29;
```

```rust
const MIGRATION_029: &str = include_str!("../migrations/0029_weekly_review.sql");
```
(logo depois da linha do `MIGRATION_028`)

E dentro de `fn migrate`, logo depois do bloco `if current <= 27 { ... MIGRATION_028 ... }`:

```rust
    if current <= 28 {
        connection
            .execute_batch(MIGRATION_029)
            .map_err(map_sql_error)?;
    }
```

- [ ] **Step 5: Declarar os métodos no trait**

Em `crates/mos-core/src/ports.rs`, dentro de `pub trait DailyRepository`, antes de `fn search_objectives`:

```rust
    /// As sessoes de uma semana, da segunda ao domingo. As duas bordas entram.
    ///
    /// Recebe a `Week` e nao um par de datas: a semana e um tipo, e passar duas
    /// datas soltas abriria a porta para alguem montar uma janela de seis dias
    /// sem que nada reclamasse.
    fn sessions_between(&self, week: &crate::Week) -> Result<Vec<crate::DailySession>, CoreError>;

    /// O fecho de uma semana, se houver.
    fn weekly_review(&self, week: &crate::Week)
        -> Result<Option<crate::WeeklyReview>, CoreError>;

    /// Grava o fecho, ou corrige o texto de um que ja existe.
    ///
    /// UPSERT por semana, e `closed_at` NAO se move na correcao: quando a
    /// semana foi fechada e um fato, e o texto e outro.
    fn save_weekly_review(
        &self,
        review: crate::NewWeeklyReview,
        now: time::OffsetDateTime,
    ) -> Result<crate::WeeklyReview, CoreError>;

    /// Os fechos mais recentes, da semana mais nova para a mais antiga.
    fn weekly_reviews(&self, limit: usize) -> Result<Vec<crate::WeeklyReview>, CoreError>;
```

- [ ] **Step 6: Implementar no repositório**

Em `crates/mos-storage-sqlite/src/daily_repository.rs`:

Acrescentar aos `use` do topo: `NewWeeklyReview, Week, WeeklyReview, WeeklyReviewId`.

Acrescentar junto das outras constantes de tipo:

```rust
const KIND_WEEK: &str = "weekly_review";

const WEEK_COLUMNS: &str = "id, week_start, summary, closed_at, created_at, updated_at";
```

Acrescentar a função de leitura, junto de `read_objective`:

```rust
fn read_week(row: &Row<'_>) -> rusqlite::Result<Result<WeeklyReview, CoreError>> {
    let id: String = row.get(0)?;
    let week_start: String = row.get(1)?;
    let summary: String = row.get(2)?;
    let closed_at: String = row.get(3)?;
    let created_at: String = row.get(4)?;
    let updated_at: String = row.get(5)?;

    Ok((|| {
        Ok(WeeklyReview {
            id: WeeklyReviewId::parse(&id)?,
            week: Week::parse(&week_start)?,
            summary,
            closed_at: parse_time(&closed_at)?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}
```

E os quatro métodos, dentro de `impl DailyRepository for SqliteStorage`, antes de `fn search_objectives`:

```rust
    fn sessions_between(&self, week: &Week) -> Result<Vec<DailySession>, CoreError> {
        let fim = week.end()?;
        self.query_sessions(
            "WHERE day >= ?1 AND day <= ?2 ORDER BY day",
            &[&week.start().as_str(), &fim.as_str()],
        )
    }

    fn weekly_review(&self, week: &Week) -> Result<Option<WeeklyReview>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {WEEK_COLUMNS} FROM weekly_reviews WHERE week_start = ?1"
            ))
            .map_err(map_sql_error)?;
        let mut rows = statement
            .query_map(params![week.start().as_str()], read_week)
            .map_err(map_sql_error)?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(map_sql_error)??)),
            None => Ok(None),
        }
    }

    fn save_weekly_review(
        &self,
        review: NewWeeklyReview,
        now: OffsetDateTime,
    ) -> Result<WeeklyReview, CoreError> {
        let momento = format_time(now)?;
        let fechado = format_time(review.closed_at)?;
        let semana = review.week.start().as_str().to_owned();

        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        // `closed_at` fica FORA do UPDATE de propósito: corrigir o texto na
        // quarta nao pode dizer que a semana foi fechada na quarta. E o `id`
        // fica de fora pelo mesmo motivo — o registro continua sendo o mesmo, e
        // trocar o id faria a sincronizacao ver uma entidade nova.
        transaction
            .execute(
                "INSERT INTO weekly_reviews (id, week_start, summary, closed_at, created_at, \
                 updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5) \
                 ON CONFLICT(week_start) DO UPDATE SET summary = ?3, updated_at = ?5",
                params![
                    review.id.to_string(),
                    semana,
                    review.summary,
                    fechado,
                    momento,
                ],
            )
            .map_err(map_sql_error)?;

        // O id que FICOU gravado, e nao o que foi mandado: numa correcao o
        // INSERT perde para o ON CONFLICT, e o id novo nunca chegou ao banco.
        // Emitir com ele criaria uma segunda entidade do outro lado.
        let gravado: String = transaction
            .query_row(
                "SELECT id FROM weekly_reviews WHERE week_start = ?1",
                params![semana],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        let gravado = WeeklyReviewId::parse(&gravado)?;

        self.emitir_update(
            &transaction,
            KIND_WEEK,
            gravado.as_uuid(),
            &[
                // `weekStart` viaja porque e a identidade do registro: um
                // dispositivo que recebe a operacao sem nunca ter visto esta
                // semana precisa saber de que semana ela e — o id e um UUID e
                // nao diz nada.
                ("weekStart", serde_json::json!(semana)),
                ("summary", serde_json::json!(review.summary)),
                ("closedAt", serde_json::json!(fechado)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        DailyRepository::weekly_review(self, &review.week)?
            .ok_or_else(|| not_found("Fecho de semana nao encontrado."))
    }

    fn weekly_reviews(&self, limit: usize) -> Result<Vec<WeeklyReview>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {WEEK_COLUMNS} FROM weekly_reviews ORDER BY week_start DESC LIMIT ?1"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![(limit.min(520)) as i64], read_week)
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }
```

- [ ] **Step 7: Rodar e verificar que passam**

Run: `cargo test -p mos-storage-sqlite --test weekly_review`
Expected: PASS — 7 testes.

- [ ] **Step 8: Verificar que a migration não quebrou nada**

Run: `cargo test -p mos-storage-sqlite`
Expected: PASS — inclusive os testes que afirmam `SCHEMA_VERSION`.

- [ ] **Step 9: Commit**

```bash
git add crates/mos-storage-sqlite/migrations/0029_weekly_review.sql \
        crates/mos-storage-sqlite/tests/weekly_review.rs \
        crates/mos-storage-sqlite/src/lib.rs \
        crates/mos-storage-sqlite/src/daily_repository.rs \
        crates/mos-core/src/ports.rs
git commit -m "$(cat <<'EOF'
feat(semana): a tabela do fecho, e o instante do fecho nao se move

Migration 0029: uma tabela nova, nenhuma alteracao em tabela existente —
mesma regra da 0027 e da 0028.

O UPSERT deixa `closed_at` de fora do UPDATE de proposito: corrigir o texto
na quarta nao pode dizer que a semana foi fechada na quarta. Quando ela foi
fechada e um fato, e o texto e outro.

A emissao usa o id que FICOU GRAVADO, e nao o que foi mandado. Numa correcao
o INSERT perde para o ON CONFLICT e o id novo nunca chega ao banco — emitir
com ele criaria uma segunda entidade do outro lado, e ela nao existiria aqui.

O CHECK trava o formato e NAO verifica que e segunda-feira. SQLite conseguiria
com `strftime`, e isso seria a regra da semana escrita num segundo lugar.

Sete testes contra banco de verdade.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: `reflections_of` — e o N+1 que ele paga

**Files:**
- Modify: `crates/mos-core/src/ports.rs`
- Modify: `crates/mos-storage-sqlite/src/daily_repository.rs`
- Modify: `crates/mos-core/src/service.rs` (`history`)
- Test: `crates/mos-storage-sqlite/tests/weekly_review.rs`

**Interfaces:**
- Produces: `fn reflections_of(&self, sessions: &[DailySessionId]) -> Result<Vec<DailyReflection>, CoreError>`

- [ ] **Step 1: Escrever o teste que falha**

Acrescentar em `crates/mos-storage-sqlite/tests/weekly_review.rs`:

```rust
#[test]
fn as_reflexoes_de_varias_sessoes_vem_numa_consulta() {
    use mos_core::{DayMood, EndDayInput};
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-17", "segunda");
    sessao_em(&storage, "2026-08-18", "terca");

    let sessoes = storage.sessions_between(&semana("2026-08-19")).unwrap();
    for (sessao, humor) in sessoes.iter().zip([DayMood::Blocked, DayMood::Productive]) {
        let entrada = EndDayInput {
            resolutions: Vec::new(),
            mood: humor.as_str().to_owned(),
            summary: String::new(),
        };
        let reflexao = entrada.reflection().unwrap().unwrap().for_session(sessao.id);
        storage.end_day(sessao.id, &[], Some(reflexao), agora()).unwrap();
    }

    let ids: Vec<_> = sessoes.iter().map(|sessao| sessao.id).collect();
    let reflexoes = storage.reflections_of(&ids).unwrap();
    assert_eq!(reflexoes.len(), 2);
    assert!(reflexoes.iter().any(|r| r.mood == Some(DayMood::Blocked)));

    assert!(
        storage.reflections_of(&[]).unwrap().is_empty(),
        "lista vazia nao vira consulta"
    );
}
```

- [ ] **Step 2: Rodar e verificar que falha**

Run: `cargo test -p mos-storage-sqlite --test weekly_review reflexoes`
Expected: FAIL — `no method named reflections_of`.

- [ ] **Step 3: Declarar no trait**

Em `crates/mos-core/src/ports.rs`, logo depois de `fn reflection(...)`:

```rust
    /// As reflexoes de VARIAS sessoes, numa consulta.
    ///
    /// A semana precisa de sete de uma vez, e o `history()` fazia uma consulta
    /// por dia listado. Mesma forma do `objectives_of`, e pelo mesmo motivo:
    /// trinta dias de historico nao podem custar trinta idas ao banco.
    fn reflections_of(
        &self,
        sessions: &[crate::DailySessionId],
    ) -> Result<Vec<crate::DailyReflection>, CoreError>;
```

- [ ] **Step 4: Implementar**

Em `daily_repository.rs`, dentro de `impl DailyRepository for SqliteStorage`, logo depois de `fn reflection`:

```rust
    fn reflections_of(
        &self,
        sessions: &[DailySessionId],
    ) -> Result<Vec<DailyReflection>, CoreError> {
        if sessions.is_empty() {
            return Ok(Vec::new());
        }
        // Lista montada por interpolacao pelo mesmo motivo do `objectives_of`:
        // `IN (?)` nao aceita array em SQLite, e os ids sao UUIDs que ja
        // passaram por `parse` — nao ha texto de usuario nesta string.
        let lista = sessions
            .iter()
            .map(|id| format!("'{id}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT session_id, mood, summary, created_at, updated_at \
                 FROM daily_reflections WHERE session_id IN ({lista})"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                let session_id: String = row.get(0)?;
                let mood: Option<String> = row.get(1)?;
                let summary: String = row.get(2)?;
                let created_at: String = row.get(3)?;
                let updated_at: String = row.get(4)?;
                Ok((session_id, mood, summary, created_at, updated_at))
            })
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            let (session_id, mood, summary, created_at, updated_at) =
                row.map_err(map_sql_error)?;
            found.push(DailyReflection {
                session_id: DailySessionId::parse(&session_id)?,
                mood: mood.as_deref().map(DayMood::parse).transpose()?,
                summary,
                created_at: parse_time(&created_at)?,
                updated_at: parse_time(&updated_at)?,
            });
        }
        Ok(found)
    }
```

- [ ] **Step 5: Trocar o N+1 no `history()`**

Em `crates/mos-core/src/service.rs`, dentro de `impl DailyService`, substituir o corpo de `history` por:

```rust
    pub fn history(&self, limit: usize) -> Result<Vec<crate::DailySessionSummary>, CoreError> {
        let sessions = self.repository.sessions(limit)?;
        let ids: Vec<_> = sessions.iter().map(|session| session.id).collect();
        let objectives = self.repository.objectives_of(&ids)?;
        // Tres consultas para N dias, e nao 2N+1: as sessoes, os objetivos de
        // todas elas, e as reflexoes de todas elas. A versao anterior lia a
        // reflexao de cada dia numa consulta propria.
        let reflections = self.repository.reflections_of(&ids)?;
        Ok(sessions
            .into_iter()
            .map(|session| {
                let mine: Vec<_> = objectives
                    .iter()
                    .filter(|objective| objective.session_id == session.id)
                    .cloned()
                    .collect();
                let mood = reflections
                    .iter()
                    .find(|reflection| reflection.session_id == session.id)
                    .and_then(|reflection| reflection.mood);
                crate::summarize(session, &mine, mood)
            })
            .collect())
    }
```

- [ ] **Step 6: Rodar e verificar que passam**

Run: `cargo test -p mos-storage-sqlite --test weekly_review && cargo test -p mos-storage-sqlite --test daily_session`
Expected: PASS nos dois — 8 e 22.

- [ ] **Step 7: Commit**

```bash
git add crates/mos-core/src/ports.rs crates/mos-core/src/service.rs \
        crates/mos-storage-sqlite/src/daily_repository.rs \
        crates/mos-storage-sqlite/tests/weekly_review.rs
git commit -m "$(cat <<'EOF'
perf(dia): as reflexoes de N sessoes viram uma consulta, e nao N

A semana precisa de sete de uma vez, e o `history()` lia a reflexao de cada
dia numa consulta propria — trinta dias de historico custavam trinta idas ao
banco para desenhar trinta linhas.

`reflections_of` tem a mesma forma do `objectives_of`, que ja existia ao
lado dele. O historico passa de 2N+1 consultas para tres.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Os métodos da semana no `DailyService`

**Files:**
- Modify: `crates/mos-core/src/service.rs`
- Test: `crates/mos-storage-sqlite/tests/weekly_review.rs`

**Interfaces:**
- Consumes: `Week`, `WeekSummary`, `NewWeeklyReview`, `compose_week`, `WeekInput`; `DailyRepository::{sessions, sessions_between, objectives_of, reflections_of, weekly_review, weekly_reviews, save_weekly_review, carry_depth}`.
- Produces, em `impl DailyService`:
  - `pub fn week(&self, week: &Week, project_of: &dyn Fn(&ObjectiveLink) -> Option<String>) -> Result<WeekSummary, CoreError>`
  - `pub fn pending_week(&self, current: &Week) -> Result<Option<Week>, CoreError>`
  - `pub fn close_week(&self, week: &Week, summary: &str) -> Result<WeeklyReview, CoreError>`

- [ ] **Step 1: Escrever o teste que falha**

Acrescentar em `crates/mos-storage-sqlite/tests/weekly_review.rs`:

```rust
// ---------------------------------------------------------------- o servico

fn servico(storage: SqliteStorage) -> (std::sync::Arc<SqliteStorage>, mos_core::DailyService) {
    let storage = std::sync::Arc::new(storage);
    let clock: std::sync::Arc<dyn mos_core::Clock> = std::sync::Arc::new(mos_core::SystemClock);
    let service = mos_core::DailyService::new(storage.clone(), clock);
    (storage, service)
}

#[test]
fn a_semana_pendente_e_a_mais_recente_sem_fecho() {
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-05", "semana de 03");
    sessao_em(&storage, "2026-08-12", "semana de 10");
    sessao_em(&storage, "2026-08-19", "semana de 17");
    let (_arc, service) = servico(storage);

    // A semana corrente e a de 24; as tres anteriores tiveram sessao.
    let corrente = semana("2026-08-26");
    assert_eq!(
        service.pending_week(&corrente).unwrap().unwrap(),
        semana("2026-08-19"),
        "a mais recente entre as candidatas"
    );

    // Fechada a de 17, a pendencia recua para a de 10.
    service.close_week(&semana("2026-08-19"), "").unwrap();
    assert_eq!(
        service.pending_week(&corrente).unwrap().unwrap(),
        semana("2026-08-12")
    );
}

#[test]
fn a_semana_corrente_nunca_e_pendente() {
    // Ela ainda esta acontecendo. Oferecer o fecho de uma semana em curso seria
    // pedir para revisar o que ainda nao terminou.
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-19", "hoje");
    let (_arc, service) = servico(storage);
    assert!(service.pending_week(&semana("2026-08-19")).unwrap().is_none());
}

#[test]
fn semana_sem_sessao_nenhuma_nao_e_pendente() {
    // Nao ha o que revisar, e a linha da Home nunca deve apontar para uma
    // semana vazia.
    let (_dir, storage) = banco();
    let (_arc, service) = servico(storage);
    assert!(service.pending_week(&semana("2026-08-26")).unwrap().is_none());
}

#[test]
fn o_resumo_da_semana_traz_o_fecho_quando_ele_existe() {
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-18", "planta");
    let (_arc, service) = servico(storage);
    let alvo = semana("2026-08-18");
    let sem_project = |_: &mos_core::ObjectiveLink| None;

    let antes = service.week(&alvo, &sem_project).unwrap();
    assert!(antes.review.is_none());
    assert_eq!(antes.days_with_session, 1);
    assert!(!antes.empty);

    service.close_week(&alvo, "foi uma semana").unwrap();
    let depois = service.week(&alvo, &sem_project).unwrap();
    assert_eq!(depois.review.unwrap().summary, "foi uma semana");
}
```

- [ ] **Step 2: Rodar e verificar que falha**

Run: `cargo test -p mos-storage-sqlite --test weekly_review pendente`
Expected: FAIL — `no method named pending_week`.

- [ ] **Step 3: Implementar os três métodos**

Em `crates/mos-core/src/service.rs`, dentro de `impl DailyService`, depois de `pub fn sessions`:

```rust
    /// A semana em narrativa, com o fecho dela quando existe.
    ///
    /// `project_of` entra por parametro porque so quem conhece Tasks e Projects
    /// consegue resolver o Project de um vinculo — e esse alguem e o comando do
    /// desktop, nao este servico.
    pub fn week(
        &self,
        week: &crate::Week,
        project_of: &dyn Fn(&crate::ObjectiveLink) -> Option<String>,
    ) -> Result<crate::WeekSummary, CoreError> {
        let sessions = self.repository.sessions_between(week)?;
        let ids: Vec<_> = sessions.iter().map(|session| session.id).collect();
        let objectives = self.repository.objectives_of(&ids)?;
        let reflections = self.repository.reflections_of(&ids)?;
        let depth = |id: crate::DailyObjectiveId| self.carry_depth(id);

        let mut summary = crate::compose_week(crate::WeekInput {
            week: week.clone(),
            sessions: &sessions,
            objectives: &objectives,
            reflections: &reflections,
            project_of,
            carry_depth: &depth,
        })?;
        summary.review = self.repository.weekly_review(week)?;
        Ok(summary)
    }

    /// A semana mais recente, anterior a corrente, que teve sessao e nao tem
    /// fecho.
    ///
    /// # Por que aqui, e nao em SQL
    ///
    /// Daria para derivar a segunda-feira com `date(day, 'weekday 0', '-6
    /// days')`. Seria a regra da semana escrita num segundo lugar — e e assim
    /// que o `arrange_widgets` do Rust ficou para tras em silencio, com os
    /// testes dele passando. `Week::containing` continua sendo a unica copia.
    pub fn pending_week(
        &self,
        current: &crate::Week,
    ) -> Result<Option<crate::Week>, CoreError> {
        use std::collections::HashSet;

        // 120 sessoes sao ~quatro meses de uso diario. Alem disso, uma semana
        // nao fechada deixou de ser pendencia e virou historico.
        let sessions = self.repository.sessions(120)?;
        let fechadas: HashSet<crate::Week> = self
            .repository
            .weekly_reviews(60)?
            .into_iter()
            .map(|review| review.week)
            .collect();

        let mut candidatas: Vec<crate::Week> = Vec::new();
        for session in &sessions {
            let semana = crate::Week::containing(&session.day)?;
            if semana < *current && !fechadas.contains(&semana) {
                candidatas.push(semana);
            }
        }
        Ok(candidatas.into_iter().max())
    }

    /// Fecha a semana, ou corrige o texto de um fecho que ja existe.
    pub fn close_week(
        &self,
        week: &crate::Week,
        summary: &str,
    ) -> Result<crate::WeeklyReview, CoreError> {
        let now = self.clock.now();
        self.repository.save_weekly_review(
            crate::NewWeeklyReview::create(week.clone(), summary, now),
            now,
        )
    }
```

- [ ] **Step 4: Rodar e verificar que passam**

Run: `cargo test -p mos-storage-sqlite --test weekly_review`
Expected: PASS — 12 testes.

- [ ] **Step 5: Rodar o workspace**

Run: `cargo test --workspace --exclude mos-desktop`
Expected: PASS, sem falhas.

- [ ] **Step 6: Commit**

```bash
git add crates/mos-core/src/service.rs crates/mos-storage-sqlite/tests/weekly_review.rs
git commit -m "$(cat <<'EOF'
feat(semana): a semana pendente e calculada no servico, e nao em SQL

Daria para derivar a segunda-feira com `date(day,'weekday 0','-6 days')`, e
nao vamos: seria a regra da semana escrita num segundo lugar, e e assim que o
`arrange_widgets` do Rust ficou para tras em silencio — com os testes dele
passando, que e o pior jeito de ficar para tras. `Week::containing` continua
sendo a unica copia.

A semana CORRENTE nunca e pendente: ela ainda esta acontecendo, e oferecer o
fecho de uma semana em curso seria pedir para revisar o que nao terminou.
Semana sem sessao nenhuma tambem nao e — nao ha o que revisar, e a linha da
Home nunca deve apontar para uma semana vazia.

Quatro testes de servico contra banco de verdade.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Comandos do desktop

**Files:**
- Modify: `apps/desktop/src-tauri/src/daily.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src/types.ts`
- Modify: `apps/desktop/src/api.ts`

**Interfaces:**
- Consumes: `DailyService::{week, pending_week, close_week}` (Task 5); `crate::daily::hoje`.
- Produces (Tauri): `weekly_week(week: Option<String>)`, `weekly_pending()`, `weekly_close(week: String, summary: String)`.
- Produces (TS): `api.weeklyWeek(week?: string)`, `api.weeklyPending()`, `api.weeklyClose(week, summary)`; tipos `Week`, `WeeklyReview`, `Dominant`, `Recurring`, `WeekSummary`.

- [ ] **Step 1: Escrever os comandos**

Em `apps/desktop/src-tauri/src/daily.rs`, acrescentar ao fim:

```rust
// ------------------------------------------------------------------- semana

/// Como achar o Project de um vinculo de objetivo.
///
/// Vive aqui, e nao no servico, porque so este lado conhece Tasks e Projects.
/// Um objetivo ligado a uma Task resolve pelo Project DA TASK — e e essa
/// agregacao que faz "o que dominou" dizer algo: tres Tasks diferentes do mesmo
/// Project sao uma semana daquele Project, e nao tres assuntos.
fn resolvedor_de_project<R: Runtime>(
    app: &AppHandle<R>,
) -> impl Fn(&ObjectiveLink) -> Option<String> + '_ {
    // As duas listas sao lidas UMA vez e capturadas: resolver por consulta a
    // cada objetivo faria uma semana de vinte objetivos custar quarenta idas ao
    // banco para desenhar cinco linhas.
    let state = app.state::<AppState>();
    let tasks = state.work.tasks(true).unwrap_or_default();
    let projects = state.work.projects(true).unwrap_or_default();

    move |link: &ObjectiveLink| {
        let project_id = match link.kind {
            LinkKind::Project => mos_core::ProjectId::parse(&link.id).ok(),
            LinkKind::Task => tasks
                .iter()
                .find(|task| task.id.to_string() == link.id)
                .and_then(|task| task.project_id),
            // Capture, Resource e Meeting nao levam a Project por um caminho
            // que valha uma agregacao semanal.
            _ => None,
        }?;
        projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.name.clone())
    }
}

/// A semana pedida, ou a corrente quando nenhuma vem.
#[tauri::command]
pub fn weekly_week<R: Runtime>(
    app: AppHandle<R>,
    week: Option<String>,
) -> Result<mos_core::WeekSummary, CoreError> {
    let alvo = match week.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(valor) => mos_core::Week::parse(valor)?,
        None => mos_core::Week::containing(&hoje(&app))?,
    };
    let project_of = resolvedor_de_project(&app);
    crate::services(&app)?.daily.week(&alvo, &project_of)
}

/// A semana que acabou e nao foi fechada, se houver.
#[tauri::command]
pub fn weekly_pending<R: Runtime>(app: AppHandle<R>) -> Result<Option<mos_core::Week>, CoreError> {
    let corrente = mos_core::Week::containing(&hoje(&app))?;
    crate::services(&app)?.daily.pending_week(&corrente)
}

#[tauri::command]
pub fn weekly_close<R: Runtime>(
    app: AppHandle<R>,
    week: String,
    summary: String,
) -> Result<mos_core::WeekSummary, CoreError> {
    let alvo = mos_core::Week::parse(&week)?;
    app.state::<AppState>().daily.close_week(&alvo, &summary)?;
    avisar(&app);
    let project_of = resolvedor_de_project(&app);
    app.state::<AppState>().daily.week(&alvo, &project_of)
}
```

E acrescentar `LinkKind` já está nos `use` do arquivo; conferir que `ObjectiveLink` também está (ele está).

- [ ] **Step 2: Registrar os comandos**

Em `apps/desktop/src-tauri/src/lib.rs`, na lista do `generate_handler!`, logo depois de `daily::daily_reopen,`:

```rust
            daily::weekly_week,
            daily::weekly_pending,
            daily::weekly_close,
```

- [ ] **Step 3: Compilar**

Run: `cargo check -p mos-desktop`
Expected: sem erros e sem avisos.

- [ ] **Step 4: Espelhar os tipos no TypeScript**

Em `apps/desktop/src/types.ts`, ao fim do bloco da Daily Session:

```ts
// ===========================================================================
// Weekly Review — o fecho da semana
// ===========================================================================
//
// Espelha `crates/mos-core/src/weekly.rs`.

/** A data da SEGUNDA-FEIRA da semana, `AAAA-MM-DD`. Nunca número ISO. */
export type Week = string;

export type WeeklyReview = {
  id: string;
  week: Week;
  /** Vazio é legítimo: fechar a semana é o gesto, escrever é opcional. */
  summary: string;
  closedAt: string;
  createdAt: string;
  updatedAt: string;
};

export type Dominant = {
  label: string;
  /** Em quantos dias isto foi o objetivo principal. */
  mainDays: number;
  days: number;
};

export type Recurring = {
  title: string;
  timesCarried: number;
};

/**
 * A semana em narrativa. **Nenhum placar** — não existe `X de Y` aqui, e a
 * ausência é a decisão: `ATTENTION-SYSTEM.md` §19 proíbe resumo de
 * produtividade em digest semanal. `daysWithSession` é fato sobre o uso do
 * sistema, e não sobre o trabalho.
 */
export type WeekSummary = {
  week: Week;
  daysWithSession: number;
  dominated: Dominant[];
  recurring: Recurring[];
  dropped: string[];
  blockedDays: Day[];
  review: WeeklyReview | null;
  /** Nenhuma sessão na semana. A tela usa isto para NÃO oferecer o fecho. */
  empty: boolean;
};
```

- [ ] **Step 5: Acrescentar os métodos da API**

Em `apps/desktop/src/api.ts`, dentro do bloco `// Daily Session`, ao fim dele (antes do comentário `// Voice Inbox`), e acrescentar `WeekSummary, Week` à lista de tipos importados:

```ts
  // ------------------------------------------------------------ Weekly Review
  //
  // `weeklyWeek()` sem argumento devolve a semana CORRENTE: quem decide que
  // semana e hoje e o backend, que le o fuso publicado por `surfaceSetLocale` —
  // mesmo motivo dos comandos do dia.

  weeklyWeek(week?: Week) {
    return invoke<WeekSummary>("weekly_week", { week: week ?? null });
  },
  /** A semana que acabou e nao foi fechada, ou `null`. */
  weeklyPending() {
    return invoke<Week | null>("weekly_pending");
  },
  weeklyClose(week: Week, summary: string) {
    return invoke<WeekSummary>("weekly_close", { week, summary });
  },
```

- [ ] **Step 6: Typecheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: sem saída.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/daily.rs apps/desktop/src-tauri/src/lib.rs \
        apps/desktop/src/types.ts apps/desktop/src/api.ts
git commit -m "$(cat <<'EOF'
feat(semana): os comandos da semana, e o Project resolvido uma vez so

`resolvedor_de_project` le Tasks e Projects UMA vez e captura as duas listas.
Resolver por consulta a cada objetivo faria uma semana de vinte objetivos
custar quarenta idas ao banco para desenhar cinco linhas.

Um objetivo ligado a uma Task resolve pelo Project DA TASK, e e essa
agregacao que faz "o que dominou" dizer algo: tres Tasks diferentes do mesmo
Project sao uma semana daquele Project, e nao tres assuntos.

`weeklyWeek()` sem argumento devolve a semana corrente — quem decide que
semana e hoje e o backend, que le o fuso publicado pela tela. Mesmo motivo
dos comandos do dia.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: `weekly.ts` — a apresentação pura

**Files:**
- Create: `apps/desktop/src/weekly.ts`
- Create: `apps/desktop/src/weekly.test.ts`

**Interfaces:**
- Consumes: tipos `WeekSummary`, `Week`, `Day` (Task 6); `dataPorExtenso` de `./daily`.
- Produces:
  - `rotuloDaSemana(week: Week, locale?: string): string`
  - `secoesDaSemana(resumo: WeekSummary): SecaoDaSemana[]`
  - `podeFechar(resumo: WeekSummary): boolean`
  - `diaDaSemanaCurto(day: Day, locale?: string): string`
  - `type SecaoDaSemana = { chave: string; titulo: string; linhas: LinhaDaSemana[] }`
  - `type LinhaDaSemana = { texto: string; detalhe: string }`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/desktop/src/weekly.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { diaDaSemanaCurto, podeFechar, rotuloDaSemana, secoesDaSemana } from "./weekly";
import type { WeekSummary } from "./types";

function resumo(over: Partial<WeekSummary> = {}): WeekSummary {
  return {
    week: "2026-08-17",
    daysWithSession: 5,
    dominated: [],
    recurring: [],
    dropped: [],
    blockedDays: [],
    review: null,
    empty: false,
    ...over,
  };
}

describe("rótulo da semana", () => {
  it("diz o intervalo, e não 'a semana passada'", () => {
    // Quem passa duas semanas sem abrir o M/OS vê a linha apontando para a
    // semana retrasada. O rótulo precisa dizer a data.
    expect(rotuloDaSemana("2026-08-17")).toBe("17 a 23 de agosto");
  });

  it("atravessa mês nomeando os dois", () => {
    expect(rotuloDaSemana("2026-09-28")).toBe("28 de setembro a 4 de outubro");
  });

  it("atravessa ano nomeando os dois", () => {
    expect(rotuloDaSemana("2025-12-29")).toBe("29 de dezembro de 2025 a 4 de janeiro de 2026");
  });

  it("data inválida volta como veio, em vez de virar texto quebrado", () => {
    expect(rotuloDaSemana("lixo")).toBe("lixo");
  });
});

describe("seções", () => {
  it("seção vazia não vira rótulo — zero não desenha título", () => {
    expect(secoesDaSemana(resumo())).toEqual([]);
  });

  it("o que dominou vira linha com o peso em palavras", () => {
    const secoes = secoesDaSemana(
      resumo({ dominated: [{ label: "063-26", mainDays: 3, days: 4 }] }),
    );
    expect(secoes).toHaveLength(1);
    expect(secoes[0].titulo).toBe("O QUE DOMINOU");
    expect(secoes[0].linhas[0]).toEqual({ texto: "063-26", detalhe: "principal em 3 dias" });
  });

  it("o que apareceu sem nunca ser principal diz isso, e não 'principal em 0 dias'", () => {
    const secoes = secoesDaSemana(
      resumo({ dominated: [{ label: "Leituras", mainDays: 0, days: 2 }] }),
    );
    expect(secoes[0].linhas[0].detalhe).toBe("em 2 dias");
  });

  it("singular e plural saem certos", () => {
    const secoes = secoesDaSemana(
      resumo({ dominated: [{ label: "Um dia só", mainDays: 1, days: 1 }] }),
    );
    expect(secoes[0].linhas[0].detalhe).toBe("principal em 1 dia");
  });

  it("as quatro seções saem na ordem da leitura", () => {
    const secoes = secoesDaSemana(
      resumo({
        dominated: [{ label: "063-26", mainDays: 2, days: 2 }],
        recurring: [{ title: "Documentação", timesCarried: 4 }],
        dropped: ["Proposta antiga"],
        blockedDays: ["2026-08-19"],
      }),
    );
    expect(secoes.map((secao) => secao.chave)).toEqual([
      "dominated",
      "recurring",
      "dropped",
      "blocked",
    ]);
    expect(secoes[1].linhas[0].detalhe).toBe("carregado 4 vezes");
    expect(secoes[3].linhas[0].texto).toBe("qua");
  });
});

describe("fechar", () => {
  it("semana sem sessão nenhuma não oferece fecho", () => {
    // Não há o que revisar, e um botão ali ensinaria que o M/OS quer um
    // registro por semana mesmo quando não houve semana.
    expect(podeFechar(resumo({ empty: true, daysWithSession: 0 }))).toBe(false);
    expect(podeFechar(resumo())).toBe(true);
  });

  it("semana já fechada continua podendo ser corrigida", () => {
    const fechada = resumo({
      review: {
        id: "w1",
        week: "2026-08-17",
        summary: "foi boa",
        closedAt: "2026-08-23T21:00:00Z",
        createdAt: "2026-08-23T21:00:00Z",
        updatedAt: "2026-08-23T21:00:00Z",
      },
    });
    expect(podeFechar(fechada)).toBe(true);
  });
});

describe("dia da semana", () => {
  it("sai abreviado e sem passar por new Date(texto)", () => {
    // `new Date("2026-08-19")` é meia-noite UTC; em UTC-3 volta como dia 18, e
    // a quarta viraria terça.
    expect(diaDaSemanaCurto("2026-08-19")).toBe("qua");
    expect(diaDaSemanaCurto("2026-08-17")).toBe("seg");
    expect(diaDaSemanaCurto("lixo")).toBe("");
  });
});
```

- [ ] **Step 2: Rodar e verificar que falha**

Run: `cd apps/desktop && npx vitest run src/weekly.test.ts`
Expected: FAIL — `Failed to resolve import "./weekly"`.

- [ ] **Step 3: Escrever `weekly.ts`**

Criar `apps/desktop/src/weekly.ts`:

```ts
/**
 * A Weekly Review do lado da tela: só o que dá para verificar.
 *
 * Mesma divisão do `daily.ts`, e pelo mesmo motivo: não há teste de DOM neste
 * repositório (`vitest.config.ts`), então tudo que decide alguma coisa — que
 * seção aparece, como um número vira frase, quando o fecho é oferecido — mora
 * aqui, e o componente só desenha o resultado.
 *
 * **Nenhuma regra de domínio.** O que a semana é, o que dominou e o que se
 * repetiu vivem em `mos-core::weekly`, com teste. Aqui é apresentação.
 */
import type { Day, Week, WeekSummary } from "./types";

/** Uma linha de seção: o assunto, e o número que é o assunto. */
export type LinhaDaSemana = { texto: string; detalhe: string };

export type SecaoDaSemana = { chave: string; titulo: string; linhas: LinhaDaSemana[] };

/** `AAAA-MM-DD` para `Date` local, **sem passar por `new Date(texto)`**. */
function dataLocal(dia: string): Date | null {
  const [ano, mes, data] = dia.split("-").map(Number);
  if (!ano || !mes || !data) return null;
  const resolvida = new Date(ano, mes - 1, data);
  return Number.isNaN(resolvida.getTime()) ? null : resolvida;
}

/**
 * "17 a 23 de agosto", "28 de setembro a 4 de outubro".
 *
 * O rótulo diz o **intervalo**, e nunca "a semana passada": quem passa duas
 * semanas sem abrir o M/OS vê a linha apontando para a semana retrasada, e
 * "passada" seria mentira.
 */
export function rotuloDaSemana(week: Week, locale = "pt-BR"): string {
  const inicio = dataLocal(week);
  if (!inicio) return week;
  const fim = new Date(inicio);
  fim.setDate(fim.getDate() + 6);

  const mesDe = (quando: Date) => new Intl.DateTimeFormat(locale, { month: "long" }).format(quando);

  if (inicio.getFullYear() !== fim.getFullYear()) {
    return `${inicio.getDate()} de ${mesDe(inicio)} de ${inicio.getFullYear()} a ${fim.getDate()} de ${mesDe(fim)} de ${fim.getFullYear()}`;
  }
  if (inicio.getMonth() !== fim.getMonth()) {
    return `${inicio.getDate()} de ${mesDe(inicio)} a ${fim.getDate()} de ${mesDe(fim)}`;
  }
  return `${inicio.getDate()} a ${fim.getDate()} de ${mesDe(fim)}`;
}

/** "seg", "qua". Vazio quando a data não resolve. */
export function diaDaSemanaCurto(day: Day, locale = "pt-BR"): string {
  const quando = dataLocal(day);
  if (!quando) return "";
  return new Intl.DateTimeFormat(locale, { weekday: "short" })
    .format(quando)
    .replace(".", "")
    .toLowerCase();
}

function plural(valor: number, singular: string, muitos: string): string {
  return `${valor} ${valor === 1 ? singular : muitos}`;
}

/**
 * As seções da semana, na ordem da leitura.
 *
 * **Seção vazia não vira rótulo.** Uma semana sem nada largado não deve mostrar
 * "O QUE VOCÊ LARGOU" seguido de vazio — é a mesma regra do `resumoDoDia`, onde
 * zero não vira linha.
 */
export function secoesDaSemana(resumo: WeekSummary): SecaoDaSemana[] {
  const secoes: SecaoDaSemana[] = [
    {
      chave: "dominated",
      titulo: "O QUE DOMINOU",
      linhas: resumo.dominated.map((item) => ({
        texto: item.label,
        // "principal em 0 dias" seria uma frase que informa o contrário do que
        // parece. Quem apareceu sem nunca ser principal diz só em quantos dias.
        detalhe: item.mainDays
          ? `principal em ${plural(item.mainDays, "dia", "dias")}`
          : `em ${plural(item.days, "dia", "dias")}`,
      })),
    },
    {
      chave: "recurring",
      titulo: "O QUE VOLTOU TODA VEZ",
      linhas: resumo.recurring.map((item) => ({
        texto: item.title,
        detalhe: `carregado ${plural(item.timesCarried, "vez", "vezes")}`,
      })),
    },
    {
      chave: "dropped",
      titulo: "O QUE VOCÊ LARGOU",
      linhas: resumo.dropped.map((titulo) => ({ texto: titulo, detalhe: "" })),
    },
    {
      chave: "blocked",
      titulo: "DIAS TRAVADOS",
      linhas: resumo.blockedDays.map((dia) => ({ texto: diaDaSemanaCurto(dia), detalhe: "" })),
    },
  ];
  return secoes.filter((secao) => secao.linhas.length > 0);
}

/**
 * A semana pode ser fechada?
 *
 * Semana sem sessão nenhuma **não** oferece o fecho: não há o que revisar, e um
 * botão ali ensinaria que o M/OS quer um registro por semana mesmo quando não
 * houve semana — que é a carga de organização que o `VISION.md` §14 proíbe
 * criar.
 *
 * Semana já fechada continua podendo: o botão vira "Salvar", e corrigir o texto
 * é a única mudança possível num registro.
 */
export function podeFechar(resumo: WeekSummary): boolean {
  return !resumo.empty;
}
```

- [ ] **Step 4: Rodar e verificar que passam**

Run: `cd apps/desktop && npx vitest run src/weekly.test.ts`
Expected: PASS — 12 testes.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/weekly.ts apps/desktop/src/weekly.test.ts
git commit -m "$(cat <<'EOF'
feat(semana): a apresentacao da semana, e secao vazia nao vira rotulo

Mesma divisao do `daily.ts`: sem DOM no runner, o que decide alguma coisa
tem de ser funcao pura, e o componente so desenha o resultado.

O rotulo diz o INTERVALO e nunca "a semana passada". Quem passa duas semanas
sem abrir o M/OS ve a linha apontando para a retrasada, e "passada" seria
mentira.

"principal em 0 dias" seria uma frase que informa o contrario do que parece;
quem apareceu sem nunca ser principal diz so em quantos dias apareceu.

A data civil nao passa por `new Date(texto)`: isso e meia-noite UTC, e num
fuso negativo a quarta viraria terca.

Doze testes.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: A aba da semana

**Files:**
- Create: `apps/desktop/src/WeeklyReview.tsx`
- Modify: `apps/desktop/src/DailySession.tsx`
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `api.{weeklyWeek, weeklyClose}` (Task 6); `weekly.ts` (Task 7); `Button`, `StateMessage`, `EmptyState` de `./Surface`.
- Produces: `<WeeklyReviewPanel semanaInicial={Week | null} />`, exportado de `WeeklyReview.tsx`.

- [ ] **Step 1: Escrever o painel**

Criar `apps/desktop/src/WeeklyReview.tsx`:

```tsx
import { useCallback, useEffect, useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { EmptyState, StateMessage } from "./Surface";
import { podeFechar, rotuloDaSemana, secoesDaSemana } from "./weekly";
import type { Week, WeekSummary } from "./types";

/**
 * O fecho da semana.
 *
 * **Nenhum placar.** Não existe `X de Y` aqui, e a ausência é a decisão:
 * `ATTENTION-SYSTEM.md` §19 proíbe resumo de produtividade em digest semanal.
 * A única contagem é "N dias com sessão", que é fato sobre o uso do sistema e
 * não sobre o trabalho.
 *
 * Abre na semana **pendente** quando há uma; senão, na corrente. Abrir sempre
 * na corrente faria a linha da Home levar a uma tela que não é a que ela
 * anunciou.
 */
export function WeeklyReviewPanel({ semanaInicial }: { semanaInicial: Week | null }) {
  const [semana, setSemana] = useState<Week | null>(semanaInicial);
  const [resumo, setResumo] = useState<WeekSummary | null>(null);
  const [texto, setTexto] = useState("");
  const [carregando, setCarregando] = useState(true);
  const [salvando, setSalvando] = useState(false);
  const [erro, setErro] = useState("");

  const carregar = useCallback(async (alvo: Week | null) => {
    setCarregando(true);
    try {
      const proximo = await api.weeklyWeek(alvo ?? undefined);
      setResumo(proximo);
      setSemana(proximo.week);
      // O texto salvo entra no campo: editar é a única mudança possível num
      // registro, e começar com o campo vazio pareceria que ele se perdeu.
      setTexto(proximo.review?.summary ?? "");
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    } finally {
      setCarregando(false);
    }
  }, []);

  useEffect(() => {
    void carregar(semanaInicial);
  }, [carregar, semanaInicial]);

  function andar(direcao: -1 | 1) {
    if (!semana) return;
    const [ano, mes, dia] = semana.split("-").map(Number);
    const quando = new Date(ano, mes - 1, dia);
    quando.setDate(quando.getDate() + direcao * 7);
    const proxima = [
      quando.getFullYear(),
      String(quando.getMonth() + 1).padStart(2, "0"),
      String(quando.getDate()).padStart(2, "0"),
    ].join("-");
    void carregar(proxima);
  }

  async function fechar() {
    if (!resumo || salvando) return;
    setSalvando(true);
    try {
      setResumo(await api.weeklyClose(resumo.week, texto));
      setErro("");
    } catch (falha) {
      setErro(appError(falha).message);
    } finally {
      setSalvando(false);
    }
  }

  if (carregando && !resumo) return <StateMessage state="loading" label="Lendo a semana..." />;
  if (erro && !resumo) {
    return <StateMessage state="error" label="A semana não pôde ser lida." detail={erro} />;
  }
  if (!resumo) return null;

  const secoes = secoesDaSemana(resumo);
  const fechada = Boolean(resumo.review);

  return (
    <div className="daily-session-body" data-busy={carregando || undefined}>
      <div className="weekly-head">
        <strong>{rotuloDaSemana(resumo.week)}</strong>
        <div className="weekly-nav">
          <button type="button" aria-label="Semana anterior" onClick={() => andar(-1)}>‹</button>
          <button type="button" aria-label="Próxima semana" onClick={() => andar(1)}>›</button>
        </div>
      </div>

      {resumo.empty ? (
        <EmptyState>Nenhum dia iniciado nesta semana. Não há o que revisar.</EmptyState>
      ) : (
        <>
          <p className="daily-widget-quiet">
            {resumo.daysWithSession} {resumo.daysWithSession === 1 ? "dia com sessão" : "dias com sessão"}
          </p>

          {secoes.map((secao) => (
            <section className="weekly-secao" key={secao.chave}>
              <span className="micro-label">{secao.titulo}</span>
              <ul>
                {secao.linhas.map((linha, indice) => (
                  <li key={`${secao.chave}-${indice}-${linha.texto}`}>
                    <span className="weekly-linha-texto">{linha.texto}</span>
                    {linha.detalhe ? <span className="micro-label">{linha.detalhe}</span> : null}
                  </li>
                ))}
              </ul>
            </section>
          ))}

          {!secoes.length ? (
            <EmptyState>A semana teve sessões, e nada nela pede uma frase.</EmptyState>
          ) : null}

          <section className="daily-reflection">
            <span className="micro-label">COMO FOI A SEMANA?</span>
            <textarea
              rows={3}
              value={texto}
              aria-label="Como foi a semana"
              placeholder="Opcional"
              onChange={(evento) => setTexto(evento.currentTarget.value)}
            />
          </section>

          {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}

          {podeFechar(resumo) ? (
            <div className="form-actions">
              <Button variant="primary" disabled={salvando} onClick={() => void fechar()}>
                {salvando ? "Salvando" : fechada ? "Salvar" : "Encerrar a semana"}
              </Button>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Ligar a terceira aba**

Em `apps/desktop/src/DailySession.tsx`, quatro edições.

Acrescentar ao import de tipos: `Week`. E o import do painel:

```tsx
import { WeeklyReviewPanel } from "./WeeklyReview";
```

Trocar o tipo `Aba`:

```tsx
type Aba = "hoje" | "historico" | "semana";
```

Acrescentar `semanaPendente` às props de `DailySessionView` (na desestruturação e no tipo):

```tsx
  /** A semana que acabou e não foi fechada. A aba abre nela quando existe. */
  semanaPendente: Week | null;
```

Acrescentar o botão da aba, depois do de Histórico:

```tsx
          <button type="button" role="tab" aria-selected={aba === "semana"} onClick={() => setAba("semana")}>Semana</button>
```

E o corpo, trocando o ternário existente por:

```tsx
        {aba === "semana" ? (
          <WeeklyReviewPanel semanaInicial={semanaPendente} />
        ) : aba === "historico" ? (
          <DailySessionHistory abrirVinculo={abrirVinculo} />
        ) : (
```

- [ ] **Step 3: Passar a prop no `App.tsx`**

Em `apps/desktop/src/App.tsx`, no `<DailySessionView ... />`, acrescentar:

```tsx
      semanaPendente={daily.semanaPendente}
```

E em `useDaily` (`DailySession.tsx`), acrescentar o estado e a leitura:

```tsx
  const [semanaPendente, setSemanaPendente] = useState<Week | null>(null);
```

Dentro do `recarregar`, trocar o `Promise.all` por:

```tsx
      const [proximoDia, proximoContexto, pendente] = await Promise.all([
        api.dailyToday(),
        api.dailyContext(),
        api.weeklyPending(),
      ]);
      setDia(proximoDia);
      setContexto(proximoContexto);
      setSemanaPendente(pendente);
```

E devolver `semanaPendente` no retorno do hook.

- [ ] **Step 4: Estilo**

Acrescentar ao fim de `apps/desktop/src/App.css`:

```css
/* --------------------------------------------------------- a semana */

.weekly-head {
  display: flex;
  gap: var(--space-3);
  align-items: baseline;
  justify-content: space-between;
}

.weekly-head strong {
  color: var(--text);
  font: var(--text-ui);
  font-weight: 600;
}

.weekly-nav {
  display: flex;
  gap: var(--space-1);
}

.weekly-nav button {
  min-width: var(--target-min);
  min-height: var(--target-min);
  padding: 0;
  color: var(--text-secondary);
  font: var(--text-ui);
  background: none;
  border: 0;
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.weekly-nav button:hover,
.weekly-nav button:focus-visible {
  color: var(--text);
  background: var(--surface-hover);
}

.weekly-secao {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.weekly-secao ul {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
}

.weekly-secao li {
  display: flex;
  gap: var(--space-3);
  align-items: baseline;
  justify-content: space-between;
  min-height: var(--height-row-dense);
}

/* O número fica à direita e o assunto à esquerda, porque o assunto é o que se
   lê primeiro. Nenhum dos dois recebe sódio: a ADR-034 reserva a cor para
   carga, e a semana já passou — não há nada aqui a fazer agora. */
.weekly-linha-texto {
  min-width: 0;
  overflow: hidden;
  color: var(--text);
  font: var(--text-ui);
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

- [ ] **Step 5: Typecheck e testes**

Run: `cd apps/desktop && npx tsc --noEmit && npx vitest run`
Expected: sem erros; 232 + 12 = 244 testes passando.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/WeeklyReview.tsx apps/desktop/src/DailySession.tsx \
        apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "$(cat <<'EOF'
feat(semana): a terceira aba da gaveta, e ela abre na semana pendente

Abrir sempre na corrente faria a linha da Home levar a uma tela que nao e a
que ela anunciou.

A navegacao por semanas mora aqui, com dois botoes, e nao numa segunda lista:
o historico de semanas fica alcancavel sem mais uma superficie, e a aba de
Historico continua sendo so dos dias.

Semana sem sessao nenhuma NAO oferece o fecho. Nao ha o que revisar, e um
botao ali ensinaria que o M/OS quer um registro por semana mesmo quando nao
houve semana — a carga de organizacao que o VISION §14 proibe criar.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: A linha na Home

**Files:**
- Modify: `apps/desktop/src/DailySession.tsx` (`DailyFocusWidget`)
- Modify: `apps/desktop/src/App.tsx`

**Interfaces:**
- Consumes: `daily.semanaPendente` (Task 8); `rotuloDaSemana` (Task 7).
- Produces: prop `abrirSemana: () => void` em `DailyFocusWidget` e em `DailyProps`.

- [ ] **Step 1: Acrescentar a linha ao widget**

Em `DailyFocusWidget` (`DailySession.tsx`), acrescentar às props `semanaPendente: Week | null` e `abrirSemana: () => void`, e desenhar a linha **nos três estados** — ela não pertence só ao estado ocioso, porque a semana pode acabar com o dia de hoje já iniciado.

Extrair o trecho para uma função no mesmo arquivo:

```tsx
/**
 * A porta da semana que acabou. Discreta, e sem travar nada.
 *
 * Reusa `.daily-stale` — o mesmo estilo que já diz "você ainda não encerrou
 * 20/08". Um segundo desenho para o mesmo tipo de aviso faria a Home ter dois
 * vocabulários para a mesma coisa.
 *
 * Aparece nos três estados do dia: a semana pode acabar numa segunda em que o
 * dia de hoje já foi iniciado, e amarrar o aviso ao estado ocioso o faria sumir
 * exatamente para quem começou a semana trabalhando.
 */
function LinhaDaSemana({ semana, abrir }: { semana: Week | null; abrir: () => void }) {
  if (!semana) return null;
  return (
    <p className="daily-stale" role="status">
      A semana de {rotuloDaSemana(semana)} acabou.
      <Button size="sm" variant="ghost" onClick={abrir}>Encerrar</Button>
    </p>
  );
}
```

Importar `rotuloDaSemana` de `./weekly` e usar `<LinhaDaSemana semana={semanaPendente} abrir={abrirSemana} />`:
- no estado ocioso, logo depois da linha `.daily-stale` do dia;
- no estado ativo, logo antes do botão "Ver sessão do dia";
- no estado encerrado, logo antes do botão "Ver resumo".

- [ ] **Step 2: Ligar no `App.tsx`**

Acrescentar a `DailyProps`:

```tsx
  semanaPendente: Week | null;
  abrirSemana: () => void;
```

E preencher em `dailyProps`:

```tsx
    semanaPendente: daily.semanaPendente,
    // Abre a gaveta já na aba da semana. Levar para a aba da sessão obrigaria
    // um segundo clique logo depois de a linha ter dito o que ia acontecer.
    abrirSemana: () => setFluxoDoDia({ tipo: "sessao", aba: "semana" }),
```

Estender o estado do fluxo para carregar a aba inicial:

```tsx
| { tipo: "sessao"; carregada?: DailyToday; aba?: "hoje" | "historico" | "semana" }
```

E em `DailySessionView`, aceitar `abaInicial?: Aba` e usá-la no `useState`:

```tsx
  const [aba, setAba] = useState<Aba>(abaInicial ?? "hoje");
```

Passando no `App.tsx`: `abaInicial={fluxoDoDia.aba}`.

Importar `Week` no `App.tsx`.

- [ ] **Step 3: Typecheck e testes**

Run: `cd apps/desktop && npx tsc --noEmit && npx vitest run && npm run build`
Expected: sem erros; 244 testes; build OK.

- [ ] **Step 4: Ver a tela**

Seguir a skill `ver-o-app`:

```bash
cd apps/desktop && npm run tauri dev &
# esperar o processo
until powershell.exe -NoProfile -Command "if (Get-Process mos-desktop -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }"; do sleep 3; done
```

Capturar (o `TMP`/`TEMP` gravável é obrigatório, senão o `Add-Type` do script falha):

```bash
W='C:\WINDOWS\TEMP\claude\scratch'
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "\$env:TMP='$W'; \$env:TEMP='$W'; & '$(cygpath -w .claude/skills/ver-o-app/capturar-janela.ps1)' -Titulo 'M/OS' -Processo 'mos-desktop' -Saida '$W\semana.png'"
```

Ler a imagem com a ferramenta Read. Conferir: a aba "Semana" existe e abre; o rótulo diz o intervalo; nenhuma seção vazia desenha título; **nenhum `X de Y` aparece na tela**.

`orca computer` **não funciona nesta máquina** (`Add-Type` sem TEMP gravável). Para chegar a um estado específico, editar temporariamente o `useState` da aba, capturar, e **reverter antes do commit**.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/DailySession.tsx apps/desktop/src/App.tsx
git commit -m "$(cat <<'EOF'
feat(semana): a linha discreta na Home, e ela aparece nos tres estados do dia

Reusa `.daily-stale`, o mesmo estilo que ja diz "voce ainda nao encerrou
20/08". Um segundo desenho para o mesmo tipo de aviso faria a Home ter dois
vocabularios para a mesma coisa.

Ela NAO fica so no estado ocioso: a semana pode acabar numa segunda em que o
dia de hoje ja foi iniciado, e amarrar o aviso ao ocioso o faria sumir
exatamente para quem comecou a semana trabalhando.

O clique abre a gaveta ja na aba da semana. Levar para a aba da sessao
obrigaria um segundo clique logo depois de a linha ter dito o que ia
acontecer.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Documentação e verificação final

**Files:**
- Modify: `docs/DAILY-SESSION.md`
- Modify: `docs/DECISIONS.md`
- Modify: `docs/SYNC.md`
- Modify: `README.md`

- [ ] **Step 1: Atualizar `DAILY-SESSION.md`**

Na tabela do §11 ("O que ficou de fora"), remover a linha do Weekly Review e acrescentar ao fim do documento:

```markdown
---

## 12. A Weekly Review

Construída em 2026-08-21. Ver `docs/superpowers/specs/2026-08-21-weekly-review-design.md` e a ADR-055.

Ela consome esta camada e não acrescenta nada a ela: `carried_from` dá a
corrente, `dropped` dá o abandono, o vínculo dá o Project, e `sessions()` dá a
contagem de dias. A única coisa que ela guarda é um texto por semana.
```

- [ ] **Step 2: Escrever a ADR-055**

Acrescentar ao fim de `docs/DECISIONS.md`:

```markdown
## ADR-055 — A semana é a segunda-feira, e a revisão não mostra placar

**Estado:** Accepted · 2026-08-21

### Contexto

O §29 do pedido da Daily Session preparou o modelo para uma revisão semanal sem
construí-la. Duas perguntas ficaram abertas: como identificar uma semana, e o
que mostrar nela.

A segunda é a que tinha uma armadilha. A coisa mais óbvia de mostrar numa semana
é `12 de 20 objetivos`, e o `ATTENTION-SYSTEM.md` §19 já proibia resumo de
produtividade em digest semanal — *"nenhuma sequência, nenhuma medalha, nenhuma
comparação com ontem"*.

### Decisão

**A semana é identificada pela data da segunda-feira**, e não por número ISO.
Semanas 53 e o 1º de janeiro que pertence à semana 52 do ano anterior são duas
armadilhas que a data da segunda simplesmente não tem. `Week::containing` é a
única cópia da regra — nada de `date(day,'weekday 0','-6 days')` em SQL.

**A revisão é narrativa, e não placar.** O que dominou, o que voltou toda vez, o
que foi largado, que dias travaram. Número aparece só quando ele é o assunto:
*"carregado 4 vezes"* informa uma decisão a tomar; *"12 de 20"* mede o
planejamento e não o trabalho, e ensina a inflar o denominador na segunda.

**A entidade é minúscula porque a narrativa é derivada.** Uma semana e um texto.
Guardar o resumo duplicaria dado para exibir noutra superfície e envelheceria:
reabrir um objetivo de terça mudaria a semana, e o texto gravado continuaria
dizendo o contrário.

**O fecho é registro, e não decisão.** Encerrar a semana não toca em objetivo
nenhum. O Start My Day já pergunta sobre os carry-overs todo dia, e uma segunda
superfície decidindo o destino dos mesmos objetivos criaria dois lugares onde a
mesma escolha é feita — com resultados possivelmente diferentes na mesma manhã.

### Consequências

- Migration 0029 acrescenta uma tabela e não altera nenhuma existente.
- `weekly_review` viaja na sincronização, com merge por campo.
- A revisão não tem ação do Hermes, entrada no Command palette nem notificação.
  As três ausências são decisões, e estão no §4 do spec.
- Semana sem sessão nenhuma não oferece fecho — não há o que revisar.
- O `history()` da Daily Session deixou de fazer N+1 de reflexões, porque a
  semana precisava de sete de uma vez.

### Revisar quando

Aparecer a terceira camada temporal — mês, ou trimestre. A pergunta será se ela
é outra entidade ou uma agregação desta, e o precedente aqui é que agregação
ganha enquanto o registro couber num texto.
```

- [ ] **Step 3: Atualizar `SYNC.md` e `README.md`**

Em `docs/SYNC.md` §14, acrescentar `weekly_review` à lista do que já emite.

Em `README.md`, depois da linha da Daily Session:

```markdown
- Weekly Review: o fecho da semana em narrativa, sem placar de produtividade;
```

- [ ] **Step 4: Verificação final completa**

```bash
export TMP="/c/WINDOWS/TEMP/claude/scratch" TEMP="$TMP"
cargo test --workspace --exclude mos-desktop
cargo check -p mos-desktop
cd apps/desktop && npx tsc --noEmit && npx vitest run && npm run build
```

Expected: Rust sem falhas; `tsc` sem saída; 244 testes de front; build OK.

Conferir também que nenhuma edição temporária de captura sobrou:

```bash
git diff --stat
grep -n 'useState<Aba>' apps/desktop/src/DailySession.tsx
```

- [ ] **Step 5: Commit**

```bash
git add docs/ README.md
git commit -m "$(cat <<'EOF'
docs(semana): ADR-055, e a Weekly Review sai da lista de pendencias

A semana e a segunda-feira e nao o numero ISO; a revisao e narrativa e nao
placar; a entidade e minuscula porque a narrativa e derivada; e o fecho e
registro e nao decisao sobre o que vem.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Auto-revisão do plano

**Cobertura do spec:**

| Seção do spec | Task |
|---|---|
| §5 `Week` é a segunda | 1 |
| §6 narrativa derivada, entidade minúscula | 2 |
| §6.1 agrupa por Project ou título | 2 (teste dedicado) |
| §6.2 corrente, não título | 2 (teste dedicado) |
| §6.3 dias travados dos humores | 2 (teste dedicado) |
| §7 só um texto, vazio fecha | 2 e 3 (teste dedicado) |
| §8 fecho não toca objetivo | por construção — nenhuma task escreve em objetivo |
| §9 migration, CHECK sem weekday | 3 |
| §9.1 sync | 3 (teste dedicado) |
| §9.2 pending_week no serviço | 5 (três testes) |
| §9.3 N+1 das reflexões | 4 |
| §10.1 aba com ‹ ›, seção vazia sem rótulo | 7 (teste) e 8 |
| §10.2 linha na Home | 9 |
| §10.3 semana vazia não oferece fecho | 2, 5 e 7 (testes) |
| §11 métodos no `DailyService` | 5 |
| §12 testes | todas |
| §13 riscos (normalização conservadora) | 2 — `crate::normalize`, que só dobra caixa e acento |

**Tipos conferidos entre tasks:** `Week` (1) usado em 2/3/5/6/7/8/9. `WeekSummary` (2) consumido em 5/6/7/8. `NewWeeklyReview::create(Week, &str, OffsetDateTime)` (2) chamado em 3 e 5. `sessions_between(&Week)` (3) chamado em 5. `reflections_of(&[DailySessionId])` (4) chamado em 5. `mainDays`/`timesCarried` em camelCase no TS (6) batem com `main_days`/`times_carried` com `rename_all = "camelCase"` (2).

**Risco conhecido:** a Task 9 exige ver a tela, e `orca computer` não funciona nesta máquina. O plano diz como contornar (editar o estado inicial, capturar, reverter) — foi o método usado na Daily Session, e foi ele que pegou o bug de centralização que nenhum teste pegaria.

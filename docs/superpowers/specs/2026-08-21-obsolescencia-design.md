# Obsolescência — o que está parado há tempo demais — Design

**Status:** ✅ **APROVADO para plano de implementação.**
O proprietário aceitou o conjunto em 2026-08-22; as quatro decisões da §2 estão
fechadas.

**Data:** 2026-08-21

**Baseline:** M/OS `v0.3.0` no commit `3122159`, que entregou a Weekly Review.

**Origem:** `IDEAS.md` #57 (Stale Tasks) e #58 (Stale Projects). Chegou como um
pedido pelo #56 (Weekly Review — retrato de estado), e virou isto pela análise
da §1.

---

## 1. Por que isto não é o `IDEAS.md` #56

O pedido original foi "faz o #56, o retrato de estado". Ao explorar, **três dos
cinco itens dele já existem na Home**:

| Item do #56 | Estado |
| --- | --- |
| Projects ativos | widget `PROJECTS` |
| Tasks concluídas | widgets `CONCLUÍDO` e `TASKS NA SEMANA` |
| Inbox | widget `INBOX`, já com envelhecimento ("N com mais de 3 dias") |
| **Tasks paradas** | **não existe** |
| **próximas prioridades** | **não existe, e não tem lastro** |

Dois vizinhos do #56 também já foram construídos sem esse nome: o **#55
(End-of-day review)** é o End My Day, e o **#59 (Activity history)** é o
Calendário.

"Próximas prioridades" é o item sem chão: **`Task` não tem campo de
prioridade** — só `Reminder` tem. Construí-lo exigiria uma migration e uma
decisão de domínio própria, e depois da Daily Session "o que vem" já é
respondido pelo Start My Day.

O que sobrava de novo era "Tasks paradas", que é o #57 e o #58. **Decisão do
proprietário: construir só isso.**

---

## 2. As quatro decisões tomadas

1. **Escopo:** só obsolescência — #57 e #58 juntos. Nada do retrato de estado.
2. **Critério:** limiar **por coluna**, e não um número único.
3. **Lugar:** widget novo na Home **mais** marca no card do Kanban.
4. **Limiares:** **fixos**, com o motivo escrito. A feature fica com **zero
   persistência**.

---

## 3. O achado que muda metade do desenho

**`projects.updated_at` só muda quando o Project é editado.** Criar Task, mover
no Kanban, concluir — nada disso toca o Project. Verificado em
`work_repository.rs`: só `update_project` e `set_project_lifecycle` escrevem
naquela coluna.

Usá-lo como sinal de obsolescência marcaria como "parado" o Project em que se
trabalhou ontem, e como "vivo" o que foi renomeado e abandonado.

**E isso já está errado hoje.** O widget `PROJECTS` da Home acende o ponto de
atividade com:

```ts
const isActiveToday = (project: Project) =>
  new Date(project.updatedAt).toDateString() === new Date().toDateString();
```

Ou seja: o ponto acende quando você **renomeia** o Project, não quando trabalha
nele. É um defeito existente, e a mesma função que esta feature precisa o
corrige.

> **A atividade de um Project é a atividade das Tasks dele** — `max(task.updated_at)`,
> caindo no `updated_at` do próprio Project só quando ele não tem Task nenhuma.

Uma função, dois consumidores.

---

## 4. O critério

```
doing      7 dias    começou e largou
review     7 dias    esperando alguém
planned   21 dias    foi planejada e não andou
inbox     14 dias    entrou e nunca foi decidida
backlog     —        é onde as coisas esperam
done        —        acabou
```

Um limiar único transformaria o backlog inteiro num alerta permanente — num
sistema com meses de uso o backlog domina a lista e afoga o sinal. Com limiar
por coluna, o resultado típico é **3 paradas, e não 47**.

**Project:** 21 dias sem atividade nas Tasks, **e só quando tem trabalho
aberto**. Project sem Task aberta e sem atividade não está travado — ele acabou e
ninguém arquivou, que é outra pergunta e merece outra resposta. Vinte e um dias
porque Project se move em semanas e Task em dias.

**Não há fuso aqui.** `updated_at` é UTC e a conta é de duração, não de data
civil — diferente do `Day` da Daily Session, que precisou do offset da tela.

---

## 5. A ordem é o excesso proporcional

Não os dias crus. Uma Task 12 dias parada num limiar de 7 está a **171%**; uma
24 dias num limiar de 21 está a **114%**. Ordenar por dias colocaria a segunda
primeiro, e ela é a menos urgente das duas.

---

## 6. Onde vive

**Domínio novo em `crates/mos-core/src/stale.rs`, puro.** Zero persistência:
nenhuma tabela, nenhuma migration, nenhum sync.

```rust
pub fn tolerancia(state: TaskState) -> Option<Duration>
pub fn atividade_do_project(project: &Project, tasks: &[Task]) -> OffsetDateTime
pub fn compose_stale(input: StaleInput<'_>) -> Vec<Parada>

pub struct Parada {
    pub kind: StaleKind,   // Task | Project
    pub id: String,
    pub title: String,
    pub context: String,   // nome do Project, ou "N tasks abertas"
    pub state: String,     // a coluna, ou vazio para Project
    pub days: i64,
}
```

Mesmo desenho do `calendar::compose`, do `daily::compose_context` e do
`weekly::compose_week`: o comando do desktop lê e delega, a função pura decide.

**Três superfícies, e nenhuma nova:**

| Onde | O que muda |
| --- | --- |
| Widget `PARADAS` na Home | novo, faixa "Retomar", ao lado do `INBOX`. Top 5 e "e mais N" |
| Card do Kanban (`BoardPage`) | ganha `data-stale` e o rótulo "12d" |
| Widget `PROJECTS` | o ponto passa a usar a atividade real, e ganha o estado oposto |

O widget é onde se **nota** — o ponto todo é que ninguém vai procurar. A marca no
card é onde se **age**, arrastando ali mesmo. Mesma divisão que o `INBOX` já tem.

---

## 7. Testes

- limiar por coluna, incluindo os dois `None` (backlog e done nunca entram);
- a fronteira exata: 6 dias não entra, 8 dias entra;
- atividade de Project vindo da Task mais recente, e não do campo dele;
- Project sem Task nenhuma caindo no próprio `updated_at`;
- Project sem trabalho aberto **não** entrando;
- ordenação por excesso proporcional, e não por dias;
- arquivado e na lixeira fora de tudo;
- front puro em `stale.ts`: o rótulo "12d", o corte em cinco, o "e mais N".

---

## 8. Fora de escopo, e por quê

| Fora | Motivo |
| --- | --- |
| limiar configurável | migration, tabela e tela para um número que se mexe uma vez. O `INBOX` usa 3 dias fixos desde sempre e nunca precisou de ajuste |
| ação em massa ("arquivar todas") | uma lista que se resolve num clique convida a limpar sem decidir. O gesto certo já existe no Kanban, arrastando |
| ação do Hermes | ele já enxerga Tasks; obsolescência é leitura, e o `mos-query` alcança |
| notificação | mesma decisão do §8 do `DAILY-SESSION.md` |
| `IDEAS.md` #56 ao pé da letra | ver §1 |
| prioridade em `Task` | é a ausência que o #56 revelou, e é outra feature — maior que esta |

---

## 9. Como retomar

1. **Confirmar o desenho com o proprietário** — ele foi apresentado e a sessão
   terminou antes da resposta. As quatro decisões da §2 estão fechadas; o que
   falta é o aceite do conjunto.
2. Trocar o `Status` deste arquivo para "aprovado para plano de implementação".
3. Invocar `superpowers:writing-plans` com este spec.
4. Executar com `superpowers:executing-plans` (o proprietário escolheu inline nas
   duas features anteriores).

**Ambiente, para não redescobrir:** exportar `TMP`/`TEMP` para um diretório
gravável antes de qualquer `cargo` e antes de qualquer `powershell.exe` com
`Add-Type`. `cargo test -p mos-desktop --lib` falha nesta máquina com
`STATUS_ENTRYPOINT_NOT_FOUND` por um problema de linker **pré-existente** — usar
`cargo test --workspace --exclude mos-desktop`. `orca computer` não funciona
aqui; para ver a tela, `ver-o-app` com `capturar-janela.ps1`.

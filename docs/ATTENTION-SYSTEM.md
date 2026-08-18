# M/OS — Attention System

**Status:** arquitetura decidida. P0 autorizado.

**Data:** 2026-08-18 · decisões da §35 fechadas pelo proprietário do produto no mesmo dia

**Subordinado a:** `VISION.md`, `PRODUCT.md`, `CORE.md`, `CORE-FOUNDATION.md`, `UX-PRINCIPLES.md`, `ARCHITECTURE.md`, `DECISIONS.md`

Este documento é a fundação do sistema que decide **o que** precisa de atenção, **quando**, **por que**, **com que urgência**, **onde aparece** e **o que a pessoa pode fazer**.

---

## 0. Auditoria: o terreno real

Esta seção existe porque três descobertas mudam o roadmap. Registrá-las antes do modelo evita desenhar sobre coisa que não existe.

### 0.1 Reminder não é conceito novo

`CORE.md` §1 lista Reminder entre os onze conceitos fundamentais do M/OS. §25 o define como *"a intenção de ser lembrado sobre algo em determinado momento **ou condição**"* — condição já estava previsto. §27 (Time Context) encerra dizendo *"a representação técnica disso será definida posteriormente"*.

`CORE-FOUNDATION.md` §3.8 vai além e já faz a separação central deste documento:

> Reminder é diferente de: prazo de Task; data planejada; evento de calendário; **notificação já entregue**.

`ARCHITECTURE.md` §8 lista **Time** como *future boundary*. `CORE-FOUNDATION.md` §10 lista *"datas, prazos, agenda e Reminders"* entre as decisões deliberadamente adiadas.

**Consequência:** este trabalho não introduz um conceito. Ele abre uma fronteira já nomeada e cumpre uma decisão adiada. O modelo abaixo deve caber na linguagem existente, não criar uma paralela.

### 0.2 Não existe Event, e Task não tem prazo

Verificado no código, não inferido:

`crates/mos-core/src/calendar.rs` é **retrospectivo**. Seu comentário de cabeçalho diz *"O que aconteceu, em forma de item de calendario (fase 1)"*. Os cinco `CalendarKind` são `Session`, `TaskDone`, `TaskCreated`, `Capture`, `AppOpened` — todos fatos passados. Não há entidade Event, não há `starts_at`, não há compromisso futuro.

`crates/mos-core/src/work.rs` — `struct Task` tem `id`, `title`, `description`, `project_id`, `source_capture_id`, `state`, `lifecycle_state`, `created_at`, `updated_at`, `completed_at`. **Não tem prazo.** Uma busca por `due_at|due_date|deadline|starts_at|scheduled_at|planned_at` em todo o `mos-core` retorna um único acerto: o `dueDay` da conta do M-Finance, dado de outro app.

`ADR-034` já havia registrado isso pelo outro lado: os widgets **W04 Next Up** e **W06 Day Arc** foram desenhados e **não construídos**, com a justificativa em tabela — *"calendário e lembretes | não — Fase 4 do ROADMAP"*.

**Consequência, e é a mais importante deste documento:**

| Requisito pedido | Estado |
|---|---|
| Reminder em hora exata | possível hoje |
| Reminder relativo a **deadline de Task** | **bloqueado** — Task não tem prazo |
| Reminder relativo a **Event** | **bloqueado** — não existe Event |
| Smart Snooze "após a reunião" | **bloqueado** — não há reunião no sistema |
| Reagendamento em cascata quando o evento muda | **bloqueado** — sem âncora |
| Widget FOCUS / "3 notifications held" | **bloqueado** — não existe focus block |

Isto não é atraso de implementação. São capacidades que dependem de dado que o M/OS não tem. A regra da ADR-034 se aplica inteira: *"um anel bonito preenchido com número inventado é pior que a ausência — ele ensina a confiar numa medida que o sistema não tem."*

O roadmap na §34 reflete isso: `Task.due_at` e `Event` viram **pré-requisitos explícitos**, com decisão própria, em vez de aparecerem como detalhe dentro de P2.

### 0.3 Já existe meio lembrete no sistema, e ele ensina três coisas

`apps/desktop/src-tauri/src/monitor.rs` dispara notificação nativa quando um programa monitorado abre ou fecha (`MonitoringSettings.remind_on_open` / `remind_on_close`). É o único caminho de notificação que existe. Ele traz precedentes que este desenho herda:

**Cooldown já foi necessário, com a razão exata da fadiga.** `REMINDER_COOLDOWN: Duration = from_secs(60)`, comentado assim: *"um AutoCAD que fecha e reabre tres vezes em dois minutos — coisa banal ao trocar de arquivo — dispara tres notificacoes, e a quarta o usuario desliga o recurso."* A §16 formaliza isso.

**A superfície não pode depender do front estar pronto.** O `struct PendingReminder` existe porque *"a janela pode nascer DEPOIS de o evento ter sido emitido... Um lembrete que depende de a janela estar pronta e um lembrete que se perde justamente na primeira vez."* A §7 generaliza: o backend é dono, o front é leitor.

**O fuso já tem padrão resolvido.** O `muted_until` do monitor é comentado: *"O instante vem da INTERFACE e nao daqui: 'hoje' acaba a meia-noite local, e o backend guarda tudo em UTC sem saber o fuso de quem clicou. A janela calcula o fim do dia dela e manda o instante pronto."* A §29 adota isso como regra geral.

E traz **uma contradição a resolver**: `fn remind` falha em silêncio *de propósito* — *"um lembrete que nao saiu e um lembrete perdido, nao um erro que valha interromper o usuario."* Isso é o oposto do que o Attention System promete. Ver §27 e a Decisão Aberta D-7.

### 0.4 Superfícies e infraestrutura que já existem

| Peça | Estado | Onde |
|---|---|---|
| Janela de toast própria | existe: `lembrete`, 400×232, `alwaysOnTop`, sem decoração, `focus: false` | `tauri.conf.json` |
| Tray | existe, com 3 itens: Abrir, Captura rápida, Sair | `lib.rs::setup_tray` |
| Processo vive fechada a janela | sim, decidido pela ADR-016 | — |
| Notificação nativa | plugin registrado, capability concedida | `tauri-plugin-notification` |
| Startup com Windows | **não existe** — sem `tauri-plugin-autostart`; ADR-016 o deixou fora da v0.1 | — |
| Sinais de contexto | nome de executável + segundos de inatividade, e **nada além** (ADR-037) | `monitor.rs` |
| Abstração de Clock | **não existe**; `now_utc()` direto em 12 pontos | — |
| Migrations | até `0014_project_budget.sql`; a próxima é a `0015` | `mos-storage-sqlite` |
| Backup / restore | existe: `DataMaintenance` com create/inspect/restore | `ports.rs` |
| Pipeline de ação do Hermes | existe e provado em produção: propor → preview → confirmar → executar | `action.rs`, `jarvis.rs` |
| Leitura do M/OS pelo Hermes | somente injeção explícita de contexto com chip visível (ADR-027, ADR-028) | `jarvis.rs::assemble_context` |
| Widgets | linguagem de anel e densidade em `packages/design-system/widgets.css` (ADR-034) | — |

### 0.5 Restrições de arquitetura que este desenho não pode violar

- **`ARCHITECTURE.md` §9** — Domain não depende de Tauri, React, SQLite ou cloud. O agendador tem duas metades: a decisão vive no domínio, o timer vive no adapter.
- **`CORE-FOUNDATION.md` §2, princípio 7** — *"Kanban, Inbox, Library, Home e Search são visualizações ou projeções, não entidades do domínio."* O Attention Center é projeção, não uma terceira tabela de itens.
- **`CORE-FOUNDATION.md` §2, princípio 6** — nada é duplicado para aparecer em outra visualização. O Attention System não guarda cópia de Task nem de Capture.
- **`ADR-012`** — sem abstração genérica de grafo. O target polimórfico do Reminder precisa de tratamento explícito (§5.3).
- **`ADR-031` / `ADR-039`** — o rail está em onze; o décimo segundo exige retirar um ou ADR que justifique não retirar. Mas a ADR-031 registra que *"Quick Capture e Settings continuam fora da contagem: eles não são destinos de conteúdo, e o rodapé do rail é uma zona própria."* Ver §19.
- **`ADR-034`** — orçamento de movimento: um loop por tela, movimento que carrega dado, cascata de 40ms com teto de oito, `reduced-motion` nascendo no valor final. E **o sódio é reservado para carga**.
- **`ADR-037`** — nenhum sinal de contexto novo sem revisar aquela ADR. Título de janela, linha de comando e tela estão fora por decisão.
- **`ADR-035`** — desfazer arquiva, nunca apaga.
- **`ARCHITECTURE.md` §20** — event sourcing completo não foi adotado. O histórico de notificações não pode virar event store.

---

## 1. Tese do produto

O M/OS deve conseguir garantir:

> Se é importante e eu preciso lembrar, o M/OS traz de volta na hora certa.

Sem produzir fadiga, spam, interrupção constante ou ansiedade.

A meta **não** é o usuário ver muitas notificações. É ver **a coisa certa, na hora certa, com a menor interrupção necessária**.

Isso alinha com `VISION.md` §14 (*"O M/OS existe para reduzir carga mental, não para criar uma nova carga"*) e responde §17, que termina em *"O que eu preciso fazer agora?"* e *"Me lembra disso amanhã."*

E tem âncora exata em `UX-PRINCIPLES.md` §87, *Measure UX by cognitive residue*:

> Depois de registrar algo, o usuário ainda continua pensando: *"Será que eu vou lembrar disso?"* Se sim, o M/OS ainda não conquistou confiança suficiente. A experiência ideal permite mentalmente **encerrar aquela preocupação**.

Encerrar aquela preocupação é a função deste sistema. E o critério de qualidade dele é o §85 do mesmo documento, *Measure UX by interruption*: *"quanto menor a interrupção, melhor."* Os dois juntos formam a tensão que este desenho administra — lembrar de tudo, interromper o mínimo.

E fixa o teste de admissão de qualquer regra futura deste sistema, herdado de `VISION.md` §16: **isso reduz o que eu preciso manter na cabeça?** Uma regra que só aumenta a chance de eu ser interrompido não pertence aqui.

### 1.1 A promessa de confiabilidade, em uma frase

**Nenhum Reminder é perdido em silêncio.**

Um Reminder pode ser entregue tarde, entregue de forma discreta, agrupado com outros ou nunca chegar como toast — mas não pode desaparecer. Se a entrega falhar, o Reminder continua existindo e continua visível no Attention Center.

Essa é a diferença entre este sistema e um despertador.

---

## 2. Terminologia

Fronteiras explícitas. Cada termo aparece uma vez neste sistema, com um dono.

| Termo | É | Não é |
|---|---|---|
| **Reminder** | intenção persistente de trazer algo de volta à atenção | uma notificação; um prazo; uma tarefa |
| **Notification** | uma entrega concreta, com canal, instante e desfecho | a intenção; o histórico da intenção |
| **Occurrence** | uma instância de um Reminder recorrente | um Reminder separado |
| **Trigger** | a regra que decide *quando* um Reminder vence | um timer |
| **Delivery** | o ato de entregar por um canal | a decisão de entregar |
| **Channel** | por onde a entrega sai: in-app, Windows, tray | a superfície visual |
| **Attention Item** | uma linha do Attention Center | uma entidade persistida |
| **Digest** | agrupamento deliberado de itens de baixa urgência | uma notificação a mais |
| **Task** | algo que precisa ser feito — **domínio já existente** | um Reminder |
| **Deadline** | limite temporal de uma Task — **não existe ainda** (§0.2) | um Reminder |
| **Event** | algo que acontece num período — **não existe** (§0.2) | um Reminder |
| **Alert** | informação potencialmente urgente vinda de fora do M/OS | um Reminder criado pelo usuário |

Um Reminder gera **muitas** Notifications ao longo da vida. Descartar uma Notification não descarta o Reminder — é essa separação que sustenta a promessa da §1.1.

**Regra de não-duplicação.** O Attention System não implementa uma segunda Task, um segundo Calendar nem um segundo Project. Ele **aponta** para eles. Onde precisar de dado que não existe, o dado é criado no domínio dono dele — `Task.due_at` pertence a Task, não ao Attention System.

---

## 3. Modelo de domínio

Mora em `crates/mos-core/src/attention.rs`. Puro, testável sem janela, sem SQLite, sem Tauri — como `calendar.rs` e `tracking.rs` já são.

```rust
pub struct Reminder {
    pub id: ReminderId,              // UUID v7, gerado no cliente
    pub title: String,
    pub body: String,                // vazio quando não há

    pub target: Option<ReminderTarget>,
    pub trigger: Trigger,
    pub priority: Priority,
    pub status: ReminderStatus,

    pub policy: DeliveryPolicy,

    pub source: ReminderSource,      // quem criou: usuário, Hermes, regra, import

    pub series_id: Option<SeriesId>, // recorrência e cadeias (§8, §31)
    pub occurrence: Option<u32>,

    pub next_due_at: Option<OffsetDateTime>,  // UTC; None quando terminal
    pub snoozed_until: Option<OffsetDateTime>,
    pub delivered_count: u32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub lifecycle_state: LifecycleState,      // reusa o enum existente
}
```

Três escolhas que merecem justificativa:

**`next_due_at` é campo derivado e persistido.** Derivado do `trigger`; persistido porque é a coluna que o agendador consulta (§7.3). Recalcular o trigger de todos os Reminders a cada tick trocaria uma query indexada por um laço.

**`lifecycle_state` reusa o enum de Capture e Task.** `active`/`archived`/`trashed`. Um enum novo criaria uma terceira semântica de retenção para o usuário aprender, e a ADR-035 (*desfazer arquiva, nunca apaga*) já vale para tudo.

**`status` é separado de `lifecycle_state`.** Mesmo precedente da ADR-015, que separou `processing_state` de `lifecycle_state` em Capture: uma dimensão diz onde a intenção está no seu ciclo, a outra diz se ela participa das superfícies. Um Reminder concluído e arquivado volta a ser concluído ao ser restaurado.

### 3.1 Notification é entidade própria

```rust
pub struct Notification {
    pub id: NotificationId,
    pub reminder_id: ReminderId,
    pub channel: Channel,
    pub dedupe_key: String,          // §17
    pub bundle_id: Option<BundleId>, // §18
    pub status: NotificationStatus,
    pub level: VisualLevel,          // §21
    pub created_at: OffsetDateTime,
    pub delivered_at: Option<OffsetDateTime>,
    pub resolved_at: Option<OffsetDateTime>,
    pub failure: Option<String>,     // §27
}
```

Sem ela não há como distinguir *"o lembrete falhou"* de *"o lembrete foi entregue e ignorado"* — e são situações que pedem respostas opostas.

**Retenção, para não virar event store** (`ARCHITECTURE.md` §20): Notifications resolvidas são podadas depois de 90 dias, exceto as com `failure`, que ficam até serem lidas em diagnóstico. O Reminder nunca é podado por essa rotina.

---

## 4. Máquina de estados do Reminder

```text
                    ┌──────────────┐
       criado ─────►│  scheduled   │
                    └──────┬───────┘
                           │ vence
                    ┌──────▼───────┐
              ┌─────│     due      │─────┐
              │     └──────┬───────┘     │
     entregue │            │             │ nunca entregue
              │            │             │ e prazo passou
     ┌────────▼──────┐     │      ┌──────▼──────┐
     │   delivered   │     │      │   missed    │
     └───┬───────┬───┘     │      └──────┬──────┘
         │       │         │             │
  ack    │       │ snooze  │             │ reconciliado
         │       │         │             │
┌────────▼───┐ ┌─▼─────────▼──┐          │
│acknowledged│ │   snoozed    │◄─────────┘
└────┬───────┘ └──────┬───────┘
     │                │ acorda
     │                └────────► scheduled
     │
     ├──────► completed   (o usuário resolveu)
     ├──────► cancelled   (o usuário desistiu)
     └──────► expired     (perdeu utilidade sem ação)

failed é ortogonal: registra falha de entrega e devolve
o Reminder a `due`, nunca a um estado terminal.
```

`scheduled`, `due`, `delivered`, `acknowledged`, `snoozed`, `completed`, `cancelled`, `missed`, `expired`. Nove estados, todos com transição explícita e testável.

Quatro regras que o código impõe:

1. **`failed` não é terminal.** Falha de entrega devolve a `due` e agenda retry (§27). Um Reminder nunca sai da existência por falha técnica — é a §1.1.
2. **`missed` é estado real, não ausência.** Quando o PC dormiu, o Reminder venceu e ninguém viu, ele fica `missed` com o instante em que deveria ter vencido. A superfície mostra *"perdido há 50 min"* em vez de fingir que é de agora (§28).
3. **`expired` exige política explícita.** Só existe se o Reminder tiver expiração declarada. Sem isso, um Reminder vencido e não atendido continua `missed` — visível — para sempre. Silenciar por decurso de prazo é a forma mais fácil de quebrar a promessa da §1.1.
4. **`completed` do Reminder não conclui a Task.** Ver §22.

---

## 5. Modelo de Trigger

```rust
pub enum Trigger {
    /// Instante exato, em UTC.
    At { instant: OffsetDateTime },

    /// Repetição.
    Recurring { rule: RecurrenceRule, from: OffsetDateTime },

    /// Se nada acontecer até `deadline`, cobrar.
    FollowUp { after: Duration, unless: Resolution },

    /// Reservado. Não implementado.
    Condition { watch: ConditionWatchId },
    Inactivity { target: ReminderTarget, quiet_for: Duration },
    Contextual { when: ContextPredicate },
}
```

**Só `At` é implementado em P0.** Os braços reservados custam um `match` exaustivo que o compilador cobra — e é ele que garante que ninguém esqueça um caso ao implementar o próximo.

### 5.1 `Relative` não existe no enum, e isso é decisão

A versão anterior deste documento incluía `Relative { anchor, offset }` com o argumento de que um enum completo evita migration futura. **As decisões D-1 e D-4 tornaram esse argumento falso:** sem prazo em Task e sem entidade Event, `Anchor` não tem nenhum valor possível. Um braço que nenhum dado pode referenciar não protege contra migration nenhuma — ele só ocupa espaço e sugere uma capacidade que não existe.

Ele volta ao enum no dia em que existir âncora, e nesse dia não haverá dado para migrar. Ver §34.

### 5.2 `FollowUp` é o único trigger derivado que P0 pode ter

`FollowUp` não depende de âncora externa: ele mede a partir da própria entrega. *"Se eu não marcar como concluído, me lembra amanhã"* precisa só do `delivered_at` e do status — dados que este domínio é dono. Com `Relative` fora (§5.1), ele passa de "antecipado" a **único** trigger derivado do sistema.

### 5.3 Target polimórfico sem violar a ADR-012

```rust
pub enum ReminderTarget {
    Task(TaskId),
    Project(ProjectId),
    Capture(CaptureId),
    Resource(ResourceId),
    Conversation(ConversationId),
    App(AppId),
}
```

Enum fechado com um id tipado por braço, persistido como `target_type TEXT` + `target_id TEXT` com índice composto. **Não** é tabela genérica de arestas: adicionar um tipo novo exige migration e uma linha no `match`, que é exatamente a consequência que a ADR-012 aceitou (*"novos tipos exigirão migration explícita no início"*).

A integridade referencial fica na aplicação, não em foreign key — uma FK por braço multiplicaria colunas nulas. Em troca, o domínio precisa tratar **target órfão**: se a Task apontada for para Trash, o Reminder não morre; ele passa a mostrar o título que guardou e o Attention Center marca o vínculo como perdido. Perder o Reminder junto com o alvo seria apagar a intenção porque o objeto mudou de estado.

---

## 6. Prioridade e Attention Score

### 6.1 Prioridade

`Low`, `Normal`, `High`, `Urgent`. Prioridade **não** é cor. Ela afeta: escolha de canal, direito de interromper em Quiet Hours e Focus, elegibilidade para digest, e agressividade de escalonamento.

`Urgent` é rara por construção: nenhuma regra automática atribui `Urgent`. Só o usuário, explicitamente. Uma prioridade que o sistema distribui sozinho deixa de significar algo em uma semana.

### 6.2 Attention Score

Interno. **Nunca exibido.** Não é score de produtividade.

Não há proibição escrita nos documentos sobre métricas de produtividade — esta é uma escolha deste desenho, e a razão é a `VISION.md` §14: o M/OS existe para reduzir carga mental. Um número que mede o quanto você está devendo ao próprio sistema acrescenta carga em vez de tirar, e o §87 do `UX-PRINCIPLES.md` mede exatamente o resíduo que sobra na cabeça depois de usar o produto.

Determinístico e puro:

```rust
pub fn attention_score(reminder: &Reminder, ctx: &AttentionContext) -> Score
```

Fatores em P3: tempo até vencer, tempo desde que venceu, prioridade, origem, histórico de snooze do próprio Reminder, notificações recentes (orçamento), Quiet Hours, contexto atual.

Duas regras de segurança:

**O score decide como entregar, nunca se existe.** Ele pode escolher entre agora, discreto, agrupado ou adiado. Não pode escolher "descartar". A §1.1 é invariante.

**Determinístico antes de inteligente.** Nenhuma IA no caminho de decisão. O Hermes pode *sugerir* mudança de configuração (§25), nunca decidir uma entrega. Um sistema em que "às vezes o lembrete não vem, e não sei por quê" é um sistema em que se para de confiar — e confiança é o produto (`PRODUCT.md` §33).

---

## 7. Arquitetura do agendador

### 7.1 Onde cada metade mora

`ARCHITECTURE.md` §9 obriga a divisão:

```text
mos-core (domínio, puro)
  ├── trait Clock                        ← §7.4
  ├── fn next_due(trigger, clock) -> Option<Instant>
  ├── fn decide(reminder, ctx) -> Decision   ← deliver | delay | digest | escalate
  ├── fn reconcile(pending, clock) -> Vec<Reconciliation>
  └── máquinas de estado das §4 e §12

mos-storage-sqlite (adapter)
  └── AttentionRepository                ← migration 0015

apps/desktop/src-tauri (adapter)
  ├── AttentionScheduler                 ← o único timer
  ├── canais de entrega                  ← §11, §20
  └── comandos Tauri                     ← superfície
```

Toda regra que pode estar errada é função pura em `mos-core`, testável com relógio falso. O adapter só sabe dormir, acordar e entregar. É o mesmo padrão de `calendar.rs::compose` — *"funcao PURA e sem repositorio de proposito: e ela que carrega as regras que podem estar erradas... e regra sem teste e regra que ninguem conferiu."*

### 7.2 Nunca `setTimeout` no renderer

Explicitamente proibido. Um Reminder tem de sobreviver a reload do front, janela fechada, navegação e sleep. O renderer é leitor e ator; não é dono do tempo. O precedente já está no `PendingReminder` do monitor (§0.3).

### 7.3 Um timer, não um por Reminder

```text
consulta MIN(next_due_at) entre os agendados
        │
        ├── nada  → dorme sem prazo, acorda em mudança de dados
        └── t     → dorme até min(t, teto de sanidade)
                        │
                acorda  ├── processa TODOS os vencidos, não só o primeiro
                        ├── reconcilia (§30)
                        └── volta a consultar
```

Um timer por Reminder desperdiça e não escala. O teto de sanidade (proposta: 15 min) existe por outro motivo: é ele que detecta salto de relógio e retorno de sleep sem depender de evento do sistema operacional (§28).

Custo em repouso: um `SELECT MIN(next_due_at) WHERE status = 'scheduled'` com índice, e uma tarefa dormindo. Sem polling de segundo, sem re-render contínuo — atende a §33.

### 7.4 Clock como porta

Não existe hoje (§0.4). Proposta:

```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;      // instante UTC
    fn monotonic(&self) -> Instant;       // para medir decurso sem sofrer salto
}
```

Duas funções e não uma: `now` sofre ajuste de relógio e DST, `monotonic` não. É a diferença entre as duas que revela sleep e mudança de hora (§28).

Os 12 `now_utc()` existentes **não** são refatorados agora. Estão em construtores, onde o instante é carimbo e não decisão. O Clock entra onde há regra temporal, que é este sistema.

### 7.5 Escrita durável antes de agendar

```text
validar → BEGIN → INSERT reminder → COMMIT → agendar → confirmar na UI
```

Nesta ordem, sem exceção. Um Reminder que existe no agendador e não no banco é um Reminder que o próximo restart apaga. Mesmo contrato da Capture (`ARCHITECTURE.md` §11.2) e da atomicidade de processamento (`CORE-FOUNDATION.md` §4.4).

Se o agendador morrer, o banco continua sendo a verdade: na abertura, reconcilia (§30).

---

## 8. Recorrência

`RecurrenceRule` cobre diário, dias úteis, semanal com dias escolhidos, mensal por dia do mês, mensal por posição (*"última sexta"*), anual e intervalo (*"a cada 2 semanas"*).

Modelo próprio, inspirado em RFC 5545 sem adotá-lo inteiro. RRULE completo carrega décadas de casos de borda de calendário compartilhado que este produto não tem — e um parser de RRULE é superfície grande para um sistema cuja promessa é confiabilidade.

**Occurrences são materializadas sob demanda, não pré-geradas.** Só a próxima existe como linha agendada. Pré-gerar doze meses — como o M-Finance faz com contas recorrentes — encheria o Attention Center de futuro e, pior, tornaria "editar a série" uma migração de dados.

### 8.1 Edição de série

Três operações, previstas no domínio desde já: **esta ocorrência**, **esta e as futuras**, **a série inteira**. `series_id` existe no modelo por isso. Só "esta ocorrência" e "a série inteira" chegam em P2; "esta e as futuras" exige cortar a regra em duas e é P3.

### 8.2 O horário de uma recorrência é local, não UTC

*"Todo dia útil às 08:30"* significa 08:30 na parede de quem pediu — não um instante UTC fixo. Guardar só UTC faz a recorrência andar uma hora no horário de verão. Ver §29.

---

## 9. Agendamento relativo

Bloqueado (§5.1). Documentado aqui para que o desenho não seja reinventado quando desbloquear:

```text
Task.due_at muda  →  domínio de Task emite mudança
                  →  Attention recalcula next_due_at dos Reminders ancorados
                  →  agendador reconsulta o mínimo
```

A recalculação é derivação, não cópia: o Reminder guarda `anchor` + `offset`, nunca o instante resolvido. Guardar o instante criaria a segunda fonte de verdade que a §2 proíbe, e ela dessincronizaria no primeiro reagendamento.

---

## 10. Canais de entrega

```rust
pub enum Channel { InApp, Windows, Tray }
```

Futuro previsto e não construído: iOS/push (`ARCHITECTURE.md` §14 já prevê o companion), e-mail. `Channel` é enum fechado; adicionar exige migration e um braço — mesma disciplina da §5.3.

**Nem tudo vira toast do Windows.** "Capture salva" é feedback in-app e nunca sai da janela. A escolha de canal vem da decisão da §6.2, não do gosto de quem escreve a chamada.

---

## 11. Integração com o Windows — capacidades verificadas

Esta seção foi escrita a partir do código das dependências, não de documentação nem de memória. É a resposta a *"não invente APIs"*.

### 11.1 O que o plugin instalado realmente faz

`tauri-plugin-notification 2.3.3`, `src/desktop.rs`. O `show()` do desktop encaminha **quatro** campos:

```rust
title · body · icon · sound
```

O builder expõe `schedule`, `action_type_id`, `group`, `group_summary`, `silent`, `ongoing`, `auto_cancel`, `attachment`, `extra`, `inbox_line`, `large_body`, `summary`, `icon_color`, `large_icon` — e **todos são descartados em silêncio no desktop**. São campos de Android/iOS.

Não há botão de ação. Não há callback de clique. Não há agendamento nativo.

**Consequência:** com o plugin como está, uma notificação do Windows é um aviso de mão única. `[Concluir] [Adiar] [Abrir]` não existiriam.

### 11.2 A capacidade existe uma camada abaixo, e já está vendorizada

`tauri-plugin-notification` depende de `notify-rust 4.18.0`, que no Windows depende de `tauri-winrt-notification 0.7.3`. **As duas já estão no nosso `Cargo.lock`** como dependências transitivas.

`notify-rust/src/windows.rs` monta os botões a partir de `notification.actions` e registra `.on_activated`. E `pub fn action(identifier, label)` **não é cfg-gated** — está disponível no Windows.

`tauri-winrt-notification::Toast` oferece, verificado na fonte:

| Capacidade | Método | Serve para |
|---|---|---|
| Botões de ação | `add_button(content, action)` | `[Concluir] [Adiar] [Abrir]` |
| Ativação e clique | `on_activated(FnMut(Option<String>))` | deep link e semântica do botão |
| Descarte com motivo | `on_dismissed(...)` | distinguir ignorado de resolvido |
| Persistir na tela | `scenario(Scenario::Reminder)` | *"pre-expanded and stay on the user's screen till dismissed"* |
| Duração | `Duration::Short` (7s) / `Long` (25s) | níveis visuais da §21 |
| Som | `sound(...)` / ausência | política de som da §23 |
| Barra de progresso | `progress` / `set_progress` | não usado |

O `action` do botão é string livre, devolvida ao `on_activated`. Isso dá semântica real: `"done:{reminder_id}"`, `"snooze:15m:{reminder_id}"`, `"open:{deep_link}"`.

**Decisão proposta (D-3):** falar com `notify-rust` diretamente para o canal Windows, em vez do builder do plugin. Sem nova superfície de supply chain — o crate já está na árvore.

### 11.3 Três limites reais, que a UI não pode prometer contornar

**AUMID.** `Toast::new(app_id)` exige um AppUserModelID. O próprio crate documenta o fallback: *"If the program you are using this in was not installed, use `Toast::POWERSHELL_APP_ID` for now"*. Em produção o instalador NSIS cria atalho no Menu Iniciar e o identificador do app serve; **em `tauri dev` o toast provavelmente exige o fallback**. Precisa de verificação empírica antes de P1 fechar (§27, gate).

**Ativação exige processo vivo.** `on_activated` é callback em memória. Se o M/OS não estiver rodando, clicar no toast não o traz de volta — isso exigiria servidor COM registrado, que está fora de qualquer coisa que estas dependências ofereçam. Como o M/OS vive no tray (ADR-016), é aceitável; mas é o argumento central a favor do autostart (§26, D-5).

**`Scenario::Alarm` não será usado.** Ele repete áudio em loop. A §1 pede urgência rara; um loop de alarme é a interrupção máxima e não há caso no produto que a justifique hoje.

---

## 12. Máquina de estados da Notification

```text
queued → delivering → delivered → seen → acted
                   │                  └─► dismissed
                   └─► failed → (retry) → delivering
```

`failed` guarda o motivo em `failure`. Retry com espera crescente, teto de três tentativas por canal — e depois disso o Reminder **continua** em `due`, visível no Attention Center. Falha de canal nunca resolve um Reminder.

---

## 13. Snooze

Adiar bem é a operação mais usada de um sistema desses, e a mais fácil de fazer mal.

**Rápidos:** 5 min, 15 min, 30 min, 1 hora, amanhã, amanhã de manhã, próximo dia útil.

**Contextuais:** dependem de dado que não existe (§0.2) — "após a reunião", "após o bloco de foco", "quando eu tiver tempo livre" ficam para depois do desbloqueio.

**Custom:** data e hora.

Quatro regras:

1. **Snooze não conta como resolução.** Volta a `scheduled`, incrementa histórico, mantém a intenção viva.
2. **Snooze tem limite visível.** A partir do quinto adiamento do mesmo Reminder, a superfície oferece explicitamente *reagendar* ou *cancelar* junto do adiar. Adiar quinze vezes é o sistema falhando em ajudar a decidir; oferecer só "adiar" é cumplicidade.
3. **Os instantes relativos ao dia vêm do renderer.** "Amanhã de manhã" é hora local. O backend recebe instante pronto — precedente do `muted_until` (§0.3).
4. **Snooze pode ser proibido por política.** `DeliveryPolicy.snooze_allowed`. Existe para o raro caso em que adiar não faz sentido; o default é permitir.

### 13.1 Smart Snooze

Sugestões calculadas a partir do calendário. **Bloqueado** — não há compromisso futuro no sistema (§0.2). Em P2, as sugestões que sobram são as de relógio ("amanhã 09:00"), sem inteligência de agenda. Prometer "após a reunião" antes de existir reunião seria inventar dado.

---

## 14. Escalonamento

Um Reminder pode subir de tom conforme o tempo passa: aviso discreto a 24h, mais forte a 2h, `missed` depois de vencer.

Duas restrições:

**Nada agressivo por default.** `escalation` é `None` a menos que o usuário peça. Escalonamento automático em cima de prioridade automática é a receita da fadiga.

**Escalonar respeita todos os freios.** Quiet Hours, Focus, orçamento e dedupe continuam valendo. Escalonamento que fura silêncio deixa de ser escalonamento e passa a ser alarme.

Também depende de âncora (24h antes de quê?), então segue os mesmos bloqueios da §9 quando aplicado a prazos.

---

## 15. Quiet Hours e Focus

### 15.1 Quiet Hours

Janela local configurável, proposta 22:00 → 08:00.

| Prioridade | Comportamento |
|---|---|
| `Low` | adia até o fim da janela |
| `Normal` | adia; entra no Attention Center sem toast |
| `High` | configurável; default é adiar com registro |
| `Urgent` | pode interromper, se o usuário tiver permitido |

**O silêncio do sistema operacional é soberano.** Se o Windows está em Focus Assist / Não Perturbe, o M/OS não tenta furar. Nada nas dependências verificadas (§11) ofereceria esse caminho — e, se oferecesse, usá-lo seria errado. A ADR-037 já fixou a forma de pensar isso para o monitoramento: *"observação que não pode ser desligada é vigilância, mesmo quando o observado é o dono da máquina."* Uma notificação que não pode ser silenciada é do mesmo tipo.

### 15.2 Focus

O M/OS **não tem** focus block nem time block (§0.2). O que existe é o oposto: detecção de **inatividade** (`GetLastInputInfo`, ADR-037).

Portanto, em P3, "Focus" começa como **estado próprio e manual** do M/OS — `Normal` / `Focus` / `Quiet` — ligado pelo usuário ou pelo Hermes (*"não me interrompa até terminar esse bloco"*), não inferido de agenda inexistente.

Ao sair de Focus: *"3 itens esperando por você"*. Um único aviso agrupado, nunca a fila enfileirada disparando de uma vez.

### 15.3 Contexto atual: o que dá para saber, e só isso

Sinais realmente disponíveis:

1. superfície ativa do M/OS (rota do renderer) — o front sabe;
2. nomes de executáveis em execução (ADR-037);
3. segundos desde o último teclado/mouse (ADR-037);
4. Reminder cujo target é a Task que está aberta no momento.

O caso útil e verificável é o quarto: se o usuário já está dentro da Task, o toast é redundante — entrega discreta no Attention Center basta.

**Fora, e não por esquecimento:** título de janela, linha de comando, conteúdo de tela. A ADR-037 fecha essa porta na API, e qualquer PR que a abra contradiz aquela ADR. O Attention Engine **não** é motivo suficiente para reabri-la.

E herda o princípio da ADR-037 — *"observação não vira hora sozinha"*: sinal observado pode **suavizar** uma entrega, nunca cancelá-la.

---

## 16. Fadiga de notificação

Prioridade absoluta, com cinco mecanismos:

| Mecanismo | O que faz |
|---|---|
| **Dedupe** | impede cópia enquanto uma equivalente está viva (§17) |
| **Cooldown** | espera mínima entre entregas do mesmo assunto — o `REMINDER_COOLDOWN` de 60s do monitor generalizado e **persistido**, porque hoje ele vive em memória e zera no restart |
| **Bundling** | várias viram uma (§18) |
| **Digest** | baixa urgência agrupada em horário escolhido (§19) |
| **Orçamento** | teto de entregas interruptivas por hora; excedente cai para o Attention Center |

O caso que motiva tudo isso, do pedido original:

```text
ruim:   "Task atrasada" · "Task ainda atrasada" · "Task atrasada há 1h" · "Task atrasada"
bom:    "4 tarefas precisam de atenção"  [Revisar]
```

---

## 17. Deduplicação

`dedupe_key` fica na **Notification**, não no Reminder. Formato: `{assunto}:{id}` — por exemplo `task-overdue:{taskId}`.

Enquanto existir Notification com a mesma chave em estado não resolvido, uma nova não é criada — a existente é **atualizada**. Isso preserva a distinção que a §2 exige: a intenção continua uma, as entregas não se multiplicam.

Nota de escopo: `CORE-FOUNDATION.md` §10 lista "deduplicação" entre as decisões adiadas, mas ali o assunto é deduplicar **Captures**. É outro problema; este não o resolve nem o antecipa.

---

## 18. Bundling

Quando duas ou mais Notifications elegíveis coincidem na mesma janela curta, elas viram uma entrega com contagem e uma ação que abre o Attention Center filtrado.

Bundling agrupa **entregas**, nunca Reminders. Cada Reminder segue com seu próprio estado, e resolver o bundle não resolve nenhum deles — só a entrega.

Não agrupa: `Urgent`, e Reminder cuja política peça entrega individual.

---

## 19. Digests

Morning Brief, Midday Check, Evening Review, Weekly Review. **Todos opcionais e todos desligados por default.**

```text
HOJE
2 eventos · 3 tarefas · 1 prazo

Próximo
09:30  Design Review
```

Duas restrições: **sem gamificação** — nenhuma sequência, nenhuma medalha, nenhuma comparação com ontem (`VISION.md` §14); e **digest não é resumo de produtividade**, é a fila de baixa urgência sendo entregue de uma vez, em horário escolhido.

Nota: o exemplo acima cita eventos e prazos, que não existem (§0.2). O digest que P3 pode entregar é o dos Reminders e, se `Task.due_at` existir até lá, o dos prazos.

---

## 20. Attention Center

**É projeção, não entidade.** `CORE-FOUNDATION.md` §2 princípio 7 é explícito, e o princípio 6 proíbe duplicar dado para exibir em outra superfície. O Attention Center lê Reminders e Notifications e compõe — do mesmo jeito que `calendar.rs::compose` já faz com quatro fontes.

```text
ATENÇÃO

Agora
──────────────────────────────
Design Review            em 12 min

Precisa de ação
──────────────────────────────
Enviar proposta        atrasado 1h

Depois
──────────────────────────────
Ligar para o fornecedor    amanhã

Recente
──────────────────────────────
```

**Filtros:** nenhum em P0. `UX-PRINCIPLES.md` §41 pede navegação previsível e estável, e abas criadas antes de existir volume que as justifique são reorganização gratuita. Search resolve histórico. Se surgirem, serão `Precisa de ação` e `Recente` — nunca cinco abas por origem.

### 20.1 Onde ele mora — e por que não no rail

O rail está em **onze** (ADR-039), e a regra vigente é: o décimo segundo exige retirar um ou uma ADR que justifique não retirar.

Existe caminho melhor. A ADR-031 registra: *"Quick Capture e Settings continuam fora da contagem: eles não são destinos de conteúdo, e o rodapé do rail é uma zona própria."*

O Attention Center é exatamente isso — não é um substantivo do produto ao lado de Tasks e Projects; é uma superfície de sistema, como Quick Capture. Ele também precisa de presença permanente para carregar o badge (§21.1), o que a paleta de comandos não dá — e a ADR-031 registra que rebaixar destino para o `Ctrl+K` já falhou uma vez com Workspaces (*"invisível para quem não conhece o Command"*).

**Proposta (D-6):** rodapé do rail, ao lado de Quick Capture, sem alterar o teto de onze nem disputar destino.

---

## 21. Níveis visuais

| Nível | Onde aparece | Frequência esperada |
|---|---|---|
| **Quiet** | só Attention Center e widget | o normal |
| **Normal** | toast in-app; Windows quando fora do M/OS | comum |
| **Important** | Windows com `Scenario::Reminder`, persiste na tela | incomum |
| **Critical** | Windows persistente + som | raríssimo |

**Cor.** A ADR-034 fixa que **o sódio é reservado para carga**, e que "agora" e "hoje" são traço branco de 2px. Isso serve bem aqui: "precisa de atenção" **é** carga. Vermelho não entra como decoração; se entrar algum dia, entra por ADR e só para estado destrutivo.

**Cor nunca é o único indicador** (§32).

### 21.1 Badge

Um número: **itens que realmente precisam de ação**. Não é contador de não lidas.

Conta: Reminders em `due`, `missed` ou `delivered` sem reconhecimento. Não conta: `snoozed`, `scheduled` futuros, `acknowledged`, nem Notifications individuais. Um badge que sobe com coisa que não pede ação é um badge que se aprende a ignorar — e aí ele deixa de servir para o que importa.

---

## 22. Integração com Tasks

**Reminder e Task permanecem separados** (`CORE.md` §26, `CORE-FOUNDATION.md` §3.8).

Semântica do `[Concluir]` de um Reminder cujo target é Task — e essa é uma decisão de produto, não de implementação:

**Concluir o Reminder NÃO conclui a Task.** O botão resolve a intenção de ser lembrado, não o trabalho. Concluir a Task por tabela seria o sistema afirmando algo que o usuário não disse, e `ADR-035` (*desfazer arquiva, nunca apaga*) mostra a inclinação do produto a não presumir.

A superfície oferece **duas** ações explícitas quando há Task: `Concluir lembrete` e `Concluir tarefa`. Duas ações claras custam um clique; uma ação ambígua custa confiança.

**`Task.due_at` não existe** e é pré-requisito de metade do que se espera aqui. Ver D-1.

---

## 23. Integração com Calendar

O Calendar é retrospectivo (§0.2). Então, hoje:

- o Calendar **pode** mostrar Reminders agendados, como um `CalendarKind` novo;
- Reminder **não** é Event, e a distinção tem de ser visual;
- reminder relativo a evento, e Smart Snooze por agenda, seguem bloqueados.

Acrescentar `CalendarKind::Reminder` é barato e cabe no `compose` existente. Mas note: hoje o `compose` recebe só coisas passadas, e um Reminder futuro numa função chamada *"o que aconteceu"* é uma inconsistência conceitual que merece decisão própria (D-4).

---

## 24. Integração com Projects e Capture

**Capture → Reminder.** `CORE.md` §5 já lista *"receber Reminder"* entre os destinos de processamento da Inbox, e `VISION.md` §5 lista "lembrete" como tipo de Capture. Processar uma Capture em Reminder segue a regra de proveniência da `CORE-FOUNDATION.md` §4: cria o Reminder, mantém a Capture, registra a derivação, tudo na mesma transação. **A Capture não é convertida.**

**Projects.** Podem gerar atenção — prazo próximo, tarefa parada, item aguardando. Tudo isso depende de dado inexistente (prazo) ou de conceito inexistente (waiting/blocked). E é onde spam automático nasce mais facilmente.

**Proposta:** nenhuma regra automática de Project em P0–P3. Se entrar, entra como **opt-in por Project**, nunca ligada por default.

---

## 25. Integração com Hermes/Jarvis

### 25.1 Escrita: o caminho já existe e foi provado

O pipeline propor → preview → confirmar → executar existe e rodou em produção. Criar Reminder por linguagem natural deve ser um `ActionKind` novo, não um protocolo de ferramentas novo:

```text
attention.create_reminder     risco Low     confirmação Explicit
attention.snooze_reminder     risco Low     confirmação None
attention.complete_reminder   risco Low     confirmação Explicit
attention.cancel_reminder     risco Medium  confirmação Explicit
```

O preview mostra título, quando e alvo, formatados — o mesmo card que o M-Finance usa hoje. Ambiguidade vira pergunta única (*"Amanhã de manhã. 09:00?"*), nunca um formulário.

`FunctionCategory` precisa de um braço `Attention`. Precedente existe: `Time` ganhou categoria própria com justificativa registrada.

### 25.2 Leitura: aqui há conflito real com ADR aceita

O pedido inclui `mos_get_reminders`, `mos_get_attention`, `mos_get_upcoming`, `mos_get_missed`, `mos_get_notification_history`.

Mas a **ADR-028** decidiu que *"a leitura do M/OS pelo Hermes começa por injeção de contexto"*, e a **ADR-027** que *"nada sai para o M/OS sem chip visível e registro do que foi enviado"*. Hoje o contexto é montado a partir de entidades que o **usuário escolhe** (`assemble_context`), com chip na tela.

Ferramentas de leitura chamadas pelo modelo, por iniciativa dele, são outro modelo de acesso. Não é impossível — mas contradiz duas ADRs aceitas e precisa de decisão, não de implementação silenciosa. Ver **D-2**.

### 25.3 O Hermes nunca é dependência operacional

`CORE.md` §31 e `CORE-FOUNDATION.md` §2 princípio 8. Todo Reminder tem de poder ser criado, adiado, entregue e resolvido com o Hermes desligado, sem rede. O Hermes é camada de linguagem sobre o sistema, não parte dele.

---

## 26. Ciclo de vida do processo

Aqui está o nó de confiabilidade, e ele exige decisão de produto.

**Hoje:** ADR-016 — fechar a janela esconde no tray, `Quit` encerra, e *"startup com Windows não entra na v0.1"*, com a consequência registrada: *"promover startup automático depende de necessidade observada."*

**O problema:** um Reminder só dispara com o processo vivo. Sem autostart, a promessa da §1 fica condicionada a *"se você tiver aberto o M/OS depois do login"* — o que a esvazia.

Três alternativas, avaliadas:

| Alternativa | Avaliação |
|---|---|
| **A. M/OS no tray + autostart opt-in** | ✅ Recomendada. Não inventa nada: `tauri-plugin-autostart` é oficial, o tray já existe, o comportamento é comunicável |
| **B. Processo em background separado** | ❌ Dois processos, dois donos do banco, contenção de WAL. Contradiz `ARCHITECTURE.md` §4 (monólito modular) |
| **C. Agendador de Tarefas do Windows** | ⚠️ Sobreviveria ao app fechado, mas a entrega precisaria do app ou de um segundo binário, recaindo em B. Guardado como contingência |

**Isto é a "necessidade observada" que a ADR-016 pedia.** Virou a **ADR-043**, com `Iniciar com o Windows` e `Iniciar minimizado` **opt-in**, desligados por default.

A ADR trouxe uma consequência que este documento não previa: o `auto-launch` também escreve na chave que o **Gerenciador de Tarefas** usa, então o Windows dá ao usuário um interruptor fora do M/OS. O toggle passa a **ler `is_enabled()`** em vez de espelhar uma configuração nossa — duas fontes de verdade divergiriam no primeiro clique feito por lá.

---

## 27. Confiabilidade e falha

**A invariante:** falha de entrega nunca resolve nem apaga um Reminder (§1.1, §12).

Isso contradiz frontalmente o `monitor.rs`, cujo `fn remind` falha em silêncio *de propósito* — *"um lembrete que nao saiu e um lembrete perdido, nao um erro que valha interromper o usuario"* (§0.3).

**A contradição é real e a resolução proposta é separar os dois casos** (D-7): o aviso do monitor é **efêmero** — ele diz "o AutoCAD abriu agora", e aquilo não tem valor cinco minutos depois; perder um é aceitável. Um Reminder é **intenção persistente**; perder um quebra o produto. São coisas de natureza diferente que hoje compartilham o nome "lembrete" no código.

Consequência prática: o `monitor.rs` mantém o comportamento atual, e o nome dele no código muda para não sugerir que é a mesma coisa.

### 27.1 Gates de P0

P0 não fecha sem, cada um com evidência reproduzível:

- Reminder sobrevive a restart do app;
- Reminder não depende do renderer estar em qualquer página;
- Reminder perdido por sleep ou restart é recuperado e marcado `missed`;
- transições de estado testadas, incluindo as de falha;
- `cancel` e `complete` funcionam;
- arquitetura de snooze presente, mesmo sem UI completa;
- Attention Center lista e resolve;
- backup inclui os dados; restore reconcilia (§30);
- agendador em repouso sem polling agressivo;
- funciona sem rede e sem Hermes;
- **nenhum Reminder perdido em silêncio**, provado por teste e não por inspeção.

E um gate empírico próprio de P1: **o toast do Windows com botões funciona no alvo real**, com AUMID resolvido nos dois cenários (instalado e `tauri dev`) — §11.3.

---

## 28. Sleep, wake e mudança de relógio

**Investigação, não invenção:** a stack atual **não** expõe evento de sleep/resume. Tauri 2 não oferece; Windows sinaliza por `WM_POWERBROADCAST`, que nenhuma dependência presente encaminha. Não vou desenhar em cima de um evento que não existe.

**A abordagem que funciona sem aquele evento** é comparar as duas leituras do Clock (§7.4):

```text
a cada tick:
  esperado = monotonic anterior + duração do sono
  real     = monotonic agora
  parede   = now agora

  se |parede - parede esperada| >> deriva tolerável
      → houve sleep, ajuste de relógio ou DST
      → reconcilia tudo em vez de confiar no timer
```

É por isso que o teto de sanidade da §7.3 existe. Ele garante que o sistema acorda periodicamente para conferir a realidade, em vez de dormir seis horas confiando num prazo calculado antes do sleep.

E, de todo modo, **toda abertura reconcilia** (§30). Sleep é só um dos caminhos para o mesmo tratamento.

---

## 29. Fuso, DST e instantes

Regra normativa da `CORE-FOUNDATION.md` §5: *"Datas técnicas devem ser armazenadas em UTC. A interpretação de datas naturais e a apresentação devem respeitar timezone e locale do usuário."*

Aplicado aqui:

| Guardar | Como |
|---|---|
| Instante de disparo (`At`) | UTC |
| Intenção de hora local de recorrência | hora de parede + zona IANA, **separado** do instante |
| Dia inteiro | data local, sem instante |

**Por que a recorrência guarda os dois.** *"Todo dia útil às 08:30"* com só UTC anda uma hora no horário de verão. Guardar a intenção local e derivar o próximo instante UTC a cada cálculo mantém 08:30 sendo 08:30.

**Limites de dia vêm do renderer.** "Amanhã", "amanhã de manhã", "fim do dia" são conceitos locais. O backend recebe instante pronto — precedente explícito do `muted_until` (§0.3). O backend guarda UTC e não tenta adivinhar fuso.

**Nunca guardar `"18/08/2026 15:00"` como string.** Nem como texto de exibição em campo que alimenta decisão.

---

## 30. Backup, restore e reconciliação

Reminders e Notifications entram no backup existente (`DataMaintenance`) e nas migrations, como qualquer entidade.

**Restore precisa de reconciliação, senão dispara uma avalanche.** Restaurar um backup de três dias atrás traria dezenas de Reminders vencidos, todos "devidos agora".

```text
restore  →  para o agendador
         →  carrega Reminders
         →  para cada vencido durante o intervalo:
                marca `missed` com o instante ORIGINAL
                NÃO entrega retroativamente
         →  agrupa numa única entrada "enquanto você esteve fora"
         →  reagenda o futuro
         →  religa o agendador
```

Idempotente: rodar duas vezes dá o mesmo resultado. É a mesma rotina da abertura normal do app — restore não ganha caminho próprio, ganha o mesmo caminho com uma janela maior.

---

## 31. Cadeias e watches condicionais

**Cadeias.** *"1 dia antes, 1 hora antes, 10 min antes"* é **uma** intenção com três entregas, não três Reminders soltos. `series_id` cobre isso: cancelar a intenção cancela as três. Depende de âncora (§9), então segue bloqueado.

**Watches condicionais.** Capacidade futura maior: *"me avisa quando o PR for aprovado"*.

```text
Condition Watch  →  Connector  →  condição satisfeita  →  Attention Engine  →  Notification
```

O `Trigger::Condition` existe no enum para que o modelo não bloqueie isso. **Nenhum connector será construído** — nem GitHub, nem e-mail. Construir o connector antes de a fundação estar confiável seria começar pela ponta que dá menos garantia.

---

## 32. Acessibilidade

A baseline não é inventada aqui: `DESIGN-FOUNDATIONS.md` §14 já a declara para o produto inteiro — WCAG 2.2 AA, texto normal em `4.5:1`, foco e estados não textuais em `3:1`, ordem de foco acompanhando a ordem visual, *"foco nunca é removido sem destino previsível"*, alvo mínimo de `28px` com `36px` para ações frequentes, e **"nenhum estado depende apenas de cor"**.

O que este sistema acrescenta, por ser o primeiro a interromper de fora da janela:

- navegação inteira por teclado; toast e Attention Center alcançáveis e dispensáveis sem mouse;
- foco visível, e devolvido ao lugar de origem ao fechar;
- rótulo de ação que diz o efeito ("Adiar 15 minutos"), não o ícone;
- leitor de tela: toast anuncia por região *polite*; nunca *assertive*, que atropela a leitura em curso;
- **cor nunca é o único indicador** — urgência tem texto e forma;
- alto contraste: o `@media (forced-colors: active)` que já existe no `App.css` cobre as superfícies novas;
- `reduced-motion` nasce no valor final (ADR-034), não degrada;
- **tempo em palavras claras:** "atrasado 1 hora", "em 12 minutos", nunca só "1h" isolado nem timestamp cru.

---

## 33. Performance

O Attention Engine fica **praticamente inerte** quando não há nada a processar.

Proibido: polling de segundo, timer por Reminder, re-render contínuo, recalcular trigger de tudo a cada tick.

Em repouso: uma tarefa dormindo e um `SELECT MIN(next_due_at)` indexado ao acordar. Orçamentos, na linha do `ARCHITECTURE.md` §12: abrir o Attention Center com dezenas de itens sem travar a digitação; entrega não bloqueia a interface; a tarefa do agendador nunca roda no fio da UI — mesma regra que a ADR-037 impôs ao laço do monitor.

---

## 34. Roadmap

Reordenado em relação ao pedido, por causa da §0.2. Duas capacidades saem de dentro das fases e viram **pré-requisitos com decisão própria**, porque sem elas metade de P2 é indesenhável.

### Fora de escopo por decisão

| Item | O que fica de fora com ele | Decisão |
|---|---|---|
| `Task.due_at` | reminder relativo a prazo, escalonamento por prazo, digest de prazos | **D-1: não por agora** |
| Entidade `Event` | reminder relativo a evento, Smart Snooze por agenda, cascata de reagendamento | **D-4: decidir separadamente** |

Nenhum dos dois bloqueia P0–P3. O que eles bloqueiam está listado acima, e a superfície não deve oferecer a opção nem sugerir que ela existe.

### P0 — Fundação de confiabilidade

Domínio, persistência (migration 0015), Clock, agendador de um timer, máquinas de estado, `Trigger::At`, reconciliação na abertura, `missed`, Attention Center, entrega in-app. Fecha pelos gates da §27.1.

### P1 — Windows

Canal Windows por `notify-rust` (§11.2), botões e ativação, deep links, tray com "Próximo" e contagem, autostart opt-in (D-5), privacidade de conteúdo em tela bloqueada. Gate empírico de AUMID.

### P2 — Reminders inteligentes

Snooze completo, `FollowUp` (§5.2), recorrência, dedupe, bundling, Quiet Hours.

Fora de P2 por decisão, e não por prazo: reminder relativo a prazo (D-1) e a evento (D-4). Smart Snooze entrega só as sugestões de relógio (§13.1).

### P3 — Inteligência de atenção

Attention Score, estado Focus manual (§15.2), entrega contextual pelos quatro sinais reais, escalonamento, digests, orçamento de notificação.

### P4 — Jarvis

Escrita por `ActionKind` (§25.1). Leitura **depende de D-2**.

### P5 — Watches externos

Arquitetura de Condition Watch, connectors, push iOS. Nada antes de P0–P3 estarem estáveis em uso real.

---

## 35. Decisões tomadas

Fechadas em 2026-08-18 pelo proprietário do produto. Ficam registradas com o que foi decidido e com o que cada decisão custa, porque uma decisão sem custo anotado é uma decisão que ninguém consegue revisar depois.

| | Decisão | Custo aceito |
|---|---|---|
| **D-1** | `Task.due_at` **não entra por agora** | reminder relativo a prazo, escalonamento por prazo e digest de prazos ficam fora indefinidamente |
| **D-2** | Hermes lê por **tipo de contexto `attention`** | leitura só acontece quando o usuário anexa o contexto; o modelo não consulta por iniciativa própria |
| **D-3** | Canal Windows por **`notify-rust` direto** | dependência direta de um crate hoje transitivo, e um caminho fora do plugin oficial |
| **D-4** | Entidade `Event` **decidida separadamente** | tudo que precisa de âncora de evento fica fora; a página Calendar segue significando "o que aconteceu" |
| **D-5** | **Autostart opt-in em P1**, desligado por default — **ADR-043**, escrita em 2026-08-18 | o usuário precisa ligar para ter a confiabilidade completa; a superfície não pode prometer o que depende de uma opção desligada |
| **D-6** | Attention Center no **rodapé do rail** | não é destino de conteúdo; some da contagem de onze e vive na zona de Quick Capture e Settings |
| **D-7** | Aviso do monitor e Reminder são **coisas declaradamente diferentes** | o monitor mantém falha em silêncio; só o nome muda no código |

### 35.1 O que D-1 e D-4 juntos implicam

As duas negativas se somam: sem prazo e sem evento, **não existe âncora de tempo futuro em nenhum lugar do M/OS**. Consequências que valem estar escritas, para ninguém as redescobrir:

- `Trigger::Relative` sai do modelo (§5.1);
- Smart Snooze entrega só sugestões de relógio (§13.1);
- escalonamento existe, mas só a partir do próprio vencimento do Reminder, nunca de um prazo alheio (§14);
- os widgets W04 *Next Up* e W06 *Day Arc* (ADR-034) destravam **parcialmente** — passam a ter Reminders para mostrar, mas continuam sem eventos;
- a UI **não oferece** essas opções nem as mostra desabilitadas. Um campo cinza ensina que a capacidade existe e está quebrada; a ausência é honesta.

### 35.2 O caminho de D-2, e por que ele não precisou de ADR

A pergunta era se o Hermes ganharia ferramentas de leitura chamáveis por iniciativa dele. Ferramentas assim contradizem a ADR-027 (*chip visível*) e a ADR-028 (*leitura começa por injeção de contexto*).

A saída não foi escolher um lado: `jarvis.rs::assemble_context` já aceita tipos de contexto que o **usuário** anexa, com chip na tela. Acrescentar um tipo `attention` entrega o estado de atenção ao modelo pelo mesmo caminho, com a mesma garantia — e sem contradizer nada.

`mos_get_reminders` e companhia ficam reservados para quando aparecer um caso concreto que a injeção não resolva. Nesse dia, a ADR que os autorizar terá de recriar a garantia de visibilidade de outra forma.

---

## 36. Handoff para o Design System

Depois das decisões, e não antes, as superfícies abaixo vão ao agente de UI/UX. Nenhuma tem visual final improvisado aqui.

Attention Center · Reminder Composer · Quick Reminder · Reminder Inspector · toast in-app · menu de Snooze · widgets de atenção · Settings · estado de Reminder perdido · estado Quiet/Focus · card de confirmação do Hermes · experiência de tray.

Duas notas para quem receber:

**Dois widgets já foram desenhados e nunca construídos.** ADR-034 registra W04 *Next Up* e W06 *Day Arc* como cortados por falta exatamente deste dado. Eles vêm primeiro, e o desenho existe.

**Herdar, não reinventar:** as famílias de anel e densidade, o orçamento de movimento, o sódio reservado para carga, e o "zero não desenha nada" — tudo da ADR-034.

---

## 37. Estratégia de testes

Sistema crítico. Cada item abaixo é teste, não inspeção — e todos rodam com Clock falso, sem esperar tempo real.

**Domínio, puro:** `At` exato; recorrência nas seis formas; borda de DST; snooze simples e múltiplo; teto de snooze; transições válidas e inválidas das §4 e §12; dedupe; bundling; Quiet Hours por prioridade; cálculo de score determinístico.

**Persistência:** durabilidade antes de agendar; reconciliação idempotente; migration `0015` sobre banco vazio e sobre a `0014`; backup e restore com reconciliação; poda de Notification que preserva Reminder.

**Confiabilidade:** kill do processo antes e depois do commit; restart com vencidos; simulação de sleep por salto de relógio; mudança de fuso; retrocesso de relógio; falha de canal com retry e esgotamento; target apagado; permissão de notificação negada.

**Fronteira:** nomes de wire entre Rust e TypeScript — o `calendar.rs` já tem o teste que pega rename silencioso (`every_kind_round_trips_through_its_wire_name`), e o mesmo vale para cada enum novo.

**O teste que resume o sistema:** para cada caminho de falha acima, provar que **o Reminder continua existindo e visível**. Não que a entrega funcionou — que a intenção não se perdeu.

---

## 38. Observabilidade

Logs estruturados, seguindo `ARCHITECTURE.md` §18: correlation ID por comando, e **redaction de conteúdo pessoal por default**.

Registram-se decisões, não conteúdo: `reminder_id`, transição, canal, motivo da decisão (`quiet_hours`, `budget`, `dedupe`, `context`), latência, falha. **Título e corpo do Reminder não vão para log** — o mesmo cuidado que a ADR-037 aplicou ao monitoramento e que `ARCHITECTURE.md` §15.1 pede para Captures.

Isso permite responder *"por que este lembrete não apareceu às 15h?"* sem que o arquivo de log se torne uma cópia em texto claro da vida do usuário.

---

## 39. Privacidade

Reminder pode conter dado financeiro, de cliente ou pessoal.

`DeliveryPolicy` carrega:

- `show_content` — título e corpo no toast;
- `title_only` — só o título;
- `hidden` — só "M/OS: um lembrete precisa de atenção".

**A verificar antes de prometer:** o comportamento em tela bloqueada é do Windows, não nosso. As dependências verificadas (§11) não expõem controle de lock screen. Portanto a política acima controla **o que colocamos no payload** — que é o que de fato está nas nossas mãos — e não onde o Windows decide mostrá-lo. Prometer "não aparece na tela bloqueada" sem controlar isso seria prometer o que não podemos cumprir.

Default proposto: `show_content`, com `title_only` disponível por Reminder e um interruptor global em Settings.

---

## 40. Settings

Poucos, com defaults fortes. `UX-PRINCIPLES.md` §8 (*progressive disclosure*) pede mostrar só o necessário e revelar o resto sob demanda, e §88 mede a experiência por **decisões desnecessárias** — cada configuração exposta é uma decisão cobrada de alguém que só queria ser lembrado de enviar a proposta.

```text
Notificações
  [✓] Notificações do Windows

  Horário de silêncio
  22:00 → 08:00

  Som
  Só o importante

  Privacidade
  [ ] Ocultar conteúdo nas notificações

  Resumos
  Manhã            [desligado]

  Iniciar com o Windows
  [ ] Iniciar com o Windows
  [ ] Iniciar minimizado

  Avançado
  ...
```

Som: `nenhum` / `discreto` / `só o importante`. Default `só o importante` — a §23 pede som raro, e o M/OS não apita o dia inteiro.

Seguindo a ADR-037, o texto na tela explica em português o que o sistema faz, onde o usuário pode conferir: *"uma promessa de privacidade que só existe no código é uma promessa que o usuário não pode cobrar."*

---

## 41. O que este documento não autoriza

- **P1 em diante** — P0 está autorizado; a ADR-043 destravou o autostart, o resto de P1 segue por fazer;
- entidade `Event` ou `Task.due_at` — negados por D-4 e D-1; voltam por decisão própria, não por conveniência de uma fase;
- oferecer na UI qualquer opção que dependa de âncora de tempo futuro (§35.1);
- connectors externos;
- qualquer sinal de contexto além dos três da ADR-037;
- ferramentas de leitura do Hermes antes de D-2;
- machine learning no caminho de decisão de entrega.

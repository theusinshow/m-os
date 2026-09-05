# O Pocket como extensão do M/OS — auditoria antes de mexer

Investigação do estado atual, feita antes de qualquer implementação, para
responder à pergunta que o dono fez: *Desktop, Web e Pocket já compartilham a
mesma fonte de dados?*

## Resposta curta

**Sim, e a arquitetura está certa.** Não há banco paralelo, não há entidade
duplicada, e o Pocket não é um sistema separado — é uma terceira instalação do
mesmo M/OS.

O que falta não é arquitetura. É **superfície**: operações que o núcleo não tem
(editar um lembrete), e operações que o núcleo tem e nenhuma tela do bolso expõe
(processar uma Capture em Task ou Resource).

## 1. Como Desktop e Web estão sincronizados hoje

Cada instalação tem seu próprio SQLite e todas conversam com o mesmo hub
(`mos-sync-server`, na VPS). O motor é `mos-sync`: relógio lógico (HLC), outbox
por aparelho, e resolução **último-a-escrever-vence por CAMPO** — dois aparelhos
editando campos diferentes do mesmo Reminder não se atropelam.

O Pocket **não é um cliente do Desktop**. Ele é um par: o `mos-web` roda o mesmo
`mos-storage-sqlite`, emite as mesmas operações e materializa da mesma forma.

Provado por teste de integração, não por suposição — `tests/de_bolso.rs`:
`a_captura_do_bolso_chega_no_pc`, `a_task_do_pc_aparece_no_bolso`,
`o_lembrete_do_bolso_chega_no_pc`, `concluir_no_bolso_tira_da_lista`.

## 2. Quais entidades já atravessam

26 famílias, listadas em `sync_cobertura.rs` — e a lista é verificada por teste
contra o schema, de modo que uma tabela nova precisa ser classificada ou o teste
quebra. Entre elas:

`tasks` · `projects` · `workspaces` · `captures` · `resources` · `reminders` ·
os seis `academic_*` · `time_entries` · `project_tracking` · `clients` ·
`conversations`/`messages` · `daily_sessions` · `daily_objectives` ·
`daily_reflections` · `weekly_reviews`

O que **fica local** tem motivo escrito ao lado: a maquinaria do próprio sync, a
telemetria da máquina, o que descreve o aparelho, o que guarda arquivo em disco,
e o **layout das telas** — `workspace_widget_layout`, `radial_pins` — pela mesma
razão que o arranjo da Home do bolso mora no `localStorage`: arranjo de tela é da
tela.

**Conclusão: nada a construir aqui.** A resposta ao item 2 do pedido é preservar.

## 3. A Agenda já é uma função compartilhada

`mos_core::calendar::compose` é uma função **pura**, e os dois a chamam:
`apps/desktop/src-tauri/src/calendar.rs:72` e `apps/mos-web/src/api.rs:798`. As
regras de o que entra na janela, o que vira dois itens e em que ordem sai moram
num lugar só.

`CalendarKind` hoje: `session`, `task_done`, `task_created`, `capture`,
`app_opened`, `day_started`, `day_ended`, `objective_done`, `assignment_due`,
`exam_scheduled`, `academic_planned`, `meeting`.

**O buraco: lembrete não é item de calendário.** `ComposeInput` não recebe
reminders. Um lembrete para quinta às 14h não aparece na agenda de quinta — nem
no Desktop, nem no bolso.

**Não há feriado**, em lugar nenhum do sistema.

## 4. Tasks não têm prazo

`Task` tem `id, title, description, project_id, source_capture_id, state,
lifecycle_state, created_at, updated_at, completed_at`. **Não há `due_at`.**

Isso não é esquecimento: o M/OS já resolve "esta task tem hora" pelo
`ReminderTarget::Task`, e o sino da lista de tasks no bolso já cria exatamente
esse vínculo. O prazo de uma Task **é um Reminder apontado para ela**.

Consequência para o item 3 do pedido: "Tasks com prazo no calendário" se resolve
pondo **reminders** no calendário — e não acrescentando um campo de data à Task,
que criaria duas fontes para a mesma pergunta.

`WorkService` já oferece `update_task`, `set_task_state`, `set_task_archived`,
`delete_task`, `task(id)`. O `mos-web` expõe **só** `criar_task` e
`mudar_estado`. O resto existe e está inalcançável do celular.

## 5. Lembretes: o núcleo é menor do que a tela precisa

`AttentionService`: `create_at`, `draft_at`, `reminder(id)`, `open`, `waiting`,
`needs_attention_count`, `transition`, `set_lifecycle`, `next_wake`,
`reconcile`, `queue_delivery`, `mark_delivered`, `mark_failed`.

`Transition`: `Ring`, `Deliver`, `Acknowledge`, `Snooze{until}`, `Complete`,
`Cancel`, `Miss`, `Expire`.

Duas ausências, e as duas são do NÚCLEO — não do web:

- **Não existe editar.** Nem título, nem corpo, nem hora. O Desktop também não
  edita: `attention.rs` oferece create, list, count, snooze, complete,
  acknowledge, cancel, archive. Reagendar hoje só existe como `Snooze`, que é
  semanticamente "adiar", não "eu errei a hora".
- **Não existe recorrência.** `Trigger` tem um braço só, `At { instant }`, e o
  comentário no código diz por quê: *"cada braço novo traz decisão de
  persistência própria, e persistir formato de regra que ainda não foi desenhada
  é criar migration para depois."*

O `mos-web` expõe `criar`, `concluir`, `cancelar`. Falta abrir `snooze` e
`archive`, que já existem.

**Portanto:** fazer CRUD de lembrete só no web seria exatamente o sistema
paralelo que o pedido proíbe. `update` tem que nascer no `mos-core` e aparecer
nas duas telas.

## 6. Captures: a classificação existe, mas depois da Capture

`Capture` tem `content`, `source` (Home/QuickCapture/Drop/Voice…),
`processing_state` (**Inbox** ou **Processed**) e `lifecycle_state`. **Não há
tipo.** Uma Capture não é "task" nem "referência" — ela é o registro cru.

A classificação acontece **quando a Capture é processada**, e o núcleo já sabe
fazer isso: `create_task_from_capture_with_reminder` transforma Capture em Task
guardando a proveniência (`source_capture_id`), e `Resource` tem o mesmo campo —
ou seja, "virar referência" é uma operação prevista pelo modelo.

O `mos-web` **não expõe nada disso**. Ele lista captures cruas e pronto. É
exatamente o sintoma que o dono descreveu: *"clico em algo dentro de Fazer e
aparece um link que eu havia salvo apenas como referência"* — porque não há como
dizer ao sistema que aquilo é uma referência.

`CaptureService` já tem: `mark_processed`, `move_to_inbox`, `archive`, `trash`,
`restore`, `delete_capture`, `between`, `search`.

## 7. CronoCAD

`TimeTrackingService.report(desde, ate)` já aceita qualquer janela. A rota
`/api/horas` também. **A limitação é só de UI**: a tela do bolso oferece dois
botões, Semana e Mês. Histórico completo e período personalizado não pedem nada
do servidor — pedem controles.

## 8. Acadêmico

Seis tabelas sincronizando, `academic::compose_dashboard` decidindo o que conta,
e o resultado já entra no calendário via `ComposeInput.academic`. Está inteiro.
O item 8 do pedido (integrar melhor com Home/Agenda/Daily) é composição de dados
que já existem, sem tocar no domínio — o menor risco de regressão da lista.

## 9. Start My Day / Daily

`daily_sessions`, `daily_objectives`, `daily_reflections` e `weekly_reviews`
sincronizam. `daily::compose_context` é pura, como o calendário. O bolso não
mostra nada disso.

## 10. Onde há duplicação

Quase nenhuma, e é notável. As três regras difíceis — calendário, dashboard
acadêmico, contexto do dia — são funções puras compartilhadas. A única
duplicação real encontrada é de **formatação**: `emHoras`/`emReais` no front do
bolso reimplementam o arredondamento que o Rust já faz — mas sobre números já
arredondados pelo servidor, então não divergem.

## O que isto muda no plano

O pedido supunha que talvez fosse preciso construir sincronização. Não é. O
trabalho real é outro, e menor:

| Pedido | Onde de fato mora |
|---|---|
| CRUD de lembretes | **`mos-core`**: falta `update`. Depois: rota + tela + Desktop |
| Sincronização | **Nada a fazer.** Preservar |
| Calendário unificado | `ComposeInput` ganha reminders; `CalendarKind` ganha `reminder` e `holiday`; filtro é da tela |
| Tasks com prazo | Já existe como Reminder apontado para a Task — expor, não criar campo |
| Kanban no bolso | `WorkService` já tem tudo; falta rota e tela |
| Horas: histórico | Servidor já aceita; falta controle na tela |
| Captures com tipo | Não é campo novo: é expor `mark_processed` e a criação de Task/Resource a partir da Capture |
| Feriados | Não existe nada. Decisão nova |
| Home viva | Composição do que já existe |

**Regra que sai daqui:** toda operação nova nasce no `mos-core`, é exposta ao
Desktop e ao Web na mesma leva, e sincroniza por consequência — nunca uma rota
do `mos-web` que escreve o que o Desktop não sabe fazer.

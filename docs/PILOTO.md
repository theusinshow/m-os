# PILOTO — o M/OS que procura a pessoa

**Estado:** implementado · 2026-09-15 · ADR-068, ADR-069, ADR-070

Implementação: `crates/mos-core/src/piloto/` (motores, puros), `apps/desktop/src-tauri/src/piloto.rs`
(retrato, comandos, laço de avisos), `apps/mos-web/src/piloto.rs` (as mesmas rotas no bolso),
`apps/desktop/src/HomePiloto.tsx` (o painel). Tabelas: migration `0041_autopilot.sql`.

---

## 1. A tese

> O usuário não deveria precisar ter disciplina para usar um sistema criado
> para ajudá-lo a ter disciplina.

Antes desta camada o M/OS era um lugar aonde se ia para descobrir o que fazer.
O piloto inverte: **o M/OS percebe o contexto e mostra o que precisa de
atenção**, monta o dia sozinho, diz o que fazer agora, e quando a pessoa some
por dias ele a ajuda a voltar — em vez de puni-la com uma parede.

```
Capture automaticamente → Organize automaticamente → Mostre o próximo passo.
```

Organização manual continua existindo. Ela deixou de ser pré-requisito.

---

## 2. Um retrato, seis motores

Todos os motores leem o **mesmo** `Retrato` — tasks, projects, reminders, inbox,
compromissos acadêmicos, agenda de hoje e amanhã, o dia, o estado do sync, a
última presença e os hábitos — e devolvem tipos que se ordenam entre si. Antes
havia quatro respostas para "o que é urgente" que não se comparavam
(`attention::needs_attention`, `stale.rs`, o Academic e o carry-over da Daily
Session). Agora há uma.

| módulo | pergunta | saída |
| --- | --- | --- |
| `atencao.rs` | o que precisa da pessoa, com que urgência | `Vec<ItemDeAtencao>` — tipo, severidade, alvo, razões, ação recomendada |
| `proximo.rs` | o que fazer AGORA, e por quê | `Recomendacao` — a candidata, as seguintes, minutos livres, razões |
| `planejador.rs` | como começar e encerrar o dia com um clique | `PropostaDoDia` (pronta para `DailyService::start`) e `PropostaDeEncerramento` |
| `resgate.rs` | a pessoa sumiu — por onde retomar | `Ausencia` e `PlanoDeResgate` em quatro passos |
| `avisos.rs` | o que vale uma notificação, e o que é spam | candidatos + política (dedupe, cooldown, silêncio, teto, snooze) |
| `autopilot.rs` | tudo junto, numa leitura | `Panorama` — o que a Home desenha inteira |

**Tudo é puro.** Recebe o tempo por parâmetro, não lê repositório, não conhece
Tauri nem HTTP. Determinístico e explicável: cada recomendação carrega as razões
em texto, porque "faça isto" sem "porquê" é um chute com cara de sistema.

### 2.1 O Attention Engine

Tipos: `overdue`, `upcoming_deadline`, `stale_waiting_for`, `unprocessed_capture`,
`unsynced_changes`, `academic_deadline`, `unfinished_day`, `day_not_started`,
`stale_task`, `scheduling_conflict`, `reminder_due`. Severidade em quatro
degraus (`baixa`, `media`, `alta`, `urgente`), deduplicado por `(tipo, alvo)`.

Regras que decidiram o desenho:

- **Quem espera terceiro não está atrasada por culpa própria.** Task com
  `waiting_for` vira `stale_waiting_for` (cobrar), nunca `overdue`.
- **A mesma Task não aparece duas vezes** por ter dois motivos: vira um item com
  dois motivos. O lembrete que aponta para uma Task já listada não vira item.
- **Sync passageiro não é atenção.** Offline só entra depois de duas horas com
  fila; erro que exige ação (credencial, contrato) entra na hora.
- **Dia não iniciado só depois das 9h30** (ou 30 min depois do horário habitual,
  quando há histórico), e nunca à noite. Dia aberto pede encerramento a partir
  das 21h.

### 2.2 O Next Action Engine

Pesos fixos: começada +50, vencida +60, vence hoje +45, amanhã +25, planejada
para hoje +30 (ou +35 se ficou de um dia anterior), prioridade urgente +30 /
alta +18 / baixa −10, objetivo principal do dia +35 / secundário +25, cabe no
tempo livre até o próximo compromisso +10 (não cabe e faltam menos de 2h: −15),
adiamentos +3 cada (teto 15), Academic com prazo hoje +30 / amanhã +20. Ficam
de fora: Task aguardando terceiro, bloqueada por outra aberta, pai com subtasks
abertas.

Não há aprendizado. Quando houver sinais de hábito, eles entram pelo
`Habitos` como mais um peso — hoje só o horário habitual de início (mediana das
últimas 14 sessões) é aprendido, e só para decidir quando perguntar.

### 2.3 Início e encerramento com um clique

`propor_dia` põe os carry-overs primeiro (a pessoa JÁ decidiu que importavam),
depois as Tasks que mais pesam pela mesma escala do Next Action — até um
principal e três secundários —, a agenda do dia e as contagens do cabeçalho. O
que sai é um `StartDayInput` pronto: **"Montar meu dia"** grava isso como veio;
"Escolher eu mesmo" abre o fluxo com as vagas já preenchidas.

`propor_encerramento` conta concluídas, abertas e vencidas, e propõe **mover
para amanhã** o que estava planejado para hoje e não terminou. Mover muda
`scheduled_for` e conta adiamento. **`due_at` nunca é tocado.**

### 2.4 O Rescue Mode

Ausência = três dias ou mais sem presença (a última presença antes desta
abertura é guardada em memória no boot, porque depois de gravada a de hoje ela
só existe ali). O plano tem quatro passos — Urgente, Atrasadas, Captures,
Waiting For — com no máximo sete itens por passo e a ação sugerida já marcada.
Ausência curta planeja as três primeiras atrasadas para hoje e o resto para
amanhã; ausência longa (10+ dias) espalha uma a cada dois dias, para a semana
não nascer impossível. Captures com mais de 30 dias sugerem arquivar. Nada é
gravado até o último passo.

### 2.5 O Notification Engine

Candidatos: compromisso em 15 min (`upcoming`), Task planejada para hoje e não
começada depois das 15h (`forgotten_task`), Task vencida urgente, entrega
acadêmica de hoje/amanhã, follow-up vencido, dia aberto à noite, dia não
iniciado, sync persistente (erro que exige ação, ou offline há horas com fila).

Política, na ordem de custo: Autopilot desligado → nada; mesma chave já
avisada → nada (a chave é tipo + alvo + dia); adiada → nada até vencer;
resolvida → nunca mais; cooldown por tipo (upcoming 10 min, forgotten 2h,
academic e waiting 4h, sync 6h); um por tipo por rodada (agrupamento); silêncio
noturno (só `upcoming` fura, e só se `allow_urgent`); teto de quatro por hora.

O lembrete clássico continua com o agendador do Attention System. Este motor
cobre o que aquele nunca cobriu (Task, Academic, dia, sync) e não o substitui.

---

## 3. O que o Autopilot faz sozinho, e o que só sugere

| ação | modo |
| --- | --- |
| montar a proposta do dia | automática (nada gravado até o clique) |
| avisar | automática, com a política acima |
| encerrar o dia de ontem que ficou aberto | sugerida, um clique (e automática ao "Montar meu dia") |
| mover Task planejada para amanhã no encerramento | sugerida, marcada por default |
| reagendar Task vencida, arquivar Capture velha | sugerida, no resgate |

O que muda dado de verdade passa sempre pela pessoa. `Autopilot: ON` é o
interruptor em Ajustes; nasce ligado.

---

## 4. Onde cada coisa mora

| camada | arquivo | o que decide |
| --- | --- | --- |
| domínio | `crates/mos-core/src/piloto/*` | tudo que pode estar errado, com teste (39) |
| persistência | `crates/mos-storage-sqlite/src/piloto_repository.rs` | saúde do sync, presença, avisos, interruptor, hábitos |
| desktop | `apps/desktop/src-tauri/src/piloto.rs` | monta o retrato, expõe comandos, roda o laço de avisos |
| bolso | `apps/mos-web/src/piloto.rs` | as mesmas rotas, o mesmo motor |
| apresentação | `apps/desktop/src/piloto.ts` | como estado vira frase (com teste) |
| tela | `HomePiloto.tsx`, `SyncHealth.tsx`, `RescueMode.tsx`, `EncerrarDia.tsx`, `AutopilotToast.tsx` | só desenho |

**Nenhuma regra de negócio vive em componente.** A Home inteira sai de
`piloto_panorama`, numa chamada; a tela não recalcula um número.

### 4.1 O laço de fundo

`piloto::run` no desktop: primeira passada 45 s depois do boot (a Home já
mostra tudo; um toast em cima da abertura diria duas vezes), depois a cada
5 min, acordado por `data-changed` com 20 s de debounce. Cada passada lê o
retrato uma vez, gera candidatos, aplica a política contra o histórico dos
últimos dois dias (`autopilot_avisos`), entrega pelo toast in-app
(`autopilot-aviso`) e pelo toast do Windows, e registra. O histórico é podado
em sete dias.

No bolso não há laço de avisos: o Web Push que já existia continua avisando o
que chegou do PC, e o painel do piloto responde ao abrir. Notificação de fundo
no iPhone depende de APNs/servidor — ver §7.

### 4.2 Tasks: planejado, começado, adiado (ADR-070)

Três colunas na migration 0041:

- `scheduled_for` — o dia em que a pessoa planejou trabalhar (data civil, como
  `daily_sessions.day`). **Não é o prazo.**
- `started_at` — quando clicou Começar. Começar põe em `doing`; parar não move
  de volta; concluir limpa.
- `postponed_count` — quantas vezes o planejamento foi empurrado. Nunca decresce.

Sincronizam como campos próprios da Task: planejar no celular e pôr prazo no
PC convivem (prova em `tests/piloto_planejamento.rs`). Uma Task ativa por
aparelho: começar outra para a anterior sem concluir.

---

## 5. O ciclo do dia

```
abrir o M/OS
  ↓ dia não iniciado?            → cartão "Montar meu dia" (ou "Escolher eu mesmo", "Agora não")
  ↓ sumiu por 3+ dias?           → cartão "Organizar para mim" (Rescue Mode)
AGORA: a Task recomendada, com "Por que agora?"
  ↓ Começar → started_at, doing, indicador global no cabeçalho
  ↓ Concluir / Continuar depois / próxima
durante o dia: capturar; o Autopilot avisa o que importa
  ↓ 21h com dia aberto           → "Hora de encerrar o dia"
Encerrar dia → move planejadas para amanhã (prazo intacto), carrega objetivos
```

Estados do dia, como a Home os vê (`EstadoDoDia`): `not_started`, `stale_open`
(ontem aberto), `active`, `ended`. O `DailySession` continua sendo a entidade;
isto é a projeção que o painel lê.

---

## 6. Hermes

Lê: o bloco `[O que o piloto ve]` desce no preâmbulo quando há algo — a Task
recomendada com as razões e até cinco itens de atenção. "O que faço agora?" e
"tenho algo atrasado?" são perguntas, e a regra determinística já tem a
resposta; gastar um turno de proposta/preview/confirmação seria o mesmo erro
que o `DAILY-SESSION.md` §6 recusou.

Age: `mos.task.start { task }`, `mos.task.plan { task, day: AAAA-MM-DD|hoje|amanha|"" }`
("joga isso para amanhã"), além de `mos.day.start` e `mos.day.end` que já
existiam. As ações passam pelos mesmos serviços que a interface.

---

## 7. Limitações reais

- **Notificação de fundo no iPhone.** O bolso é uma PWA; sem app nativo não há
  APNs. O que existe: Web Push disparado pelo servidor quando algo chega do
  PC, e o painel do piloto ao abrir. Um laço de avisos no servidor (a mesma
  política, entregando por Web Push) é o próximo passo natural quando houver
  VAPID em produção.
- **Push do hub para o desktop.** O hub continua pull-only; a reconexão é
  coberta pela escada do backoff e pelo evento `online` da webview.

---

## 8. Testes

- domínio: `crates/mos-core/src/piloto/*` — 39 testes (severidade, dedupe,
  resolvido, escala do Next Action, proposta com carry-over, encerramento sem
  tocar prazo, ausência curta/longa/backlog grande, dedupe/cooldown/snooze/
  silêncio/teto dos avisos, panorama);
- persistência: `piloto_repository.rs` (saúde, presença, avisos, interruptor) e
  `tests/piloto_planejamento.rs` (emissão por campo, dois aparelhos, saúde
  contra hub caído);
- apresentação: `apps/desktop/src/piloto.test.ts` (selo do sync, frases de
  erro, próxima tentativa, contagens).

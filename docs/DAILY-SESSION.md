# DAILY SESSION — a camada de intenção sobre o dia

**Estado:** implementado
**Data:** 2026-08-21
**Decisão:** ADR-054. Antecedentes: ADR-012 (sem grafo genérico), ADR-035
(desfazer arquiva), ADR-045 (destino novo nasce no leque), ADR-053 (sync por
campo).

---

## 1. O que ela é, em uma frase

Uma `Task` representa algo que precisa ser feito. Um `DailyObjective` representa
algo que a pessoa **decidiu que importa hoje** — e essas são perguntas
diferentes.

A lista de Tasks abertas de um sistema com meses de uso tem dezenas de linhas, e
nenhuma delas diz qual é a de hoje. A Daily Session é a camada que responde isso,
**sem virar outra base de tarefas**: um objetivo pode apontar para uma Task, e
quando aponta, a Task continua sendo a dona do trabalho.

---

## 2. O checklist do `FEATURE-DEVELOPMENT.md`

```
feature: daily-session

core:          DailySession, DailyObjective, DailyReflection em mos-core::daily.
               Puro, sem I/O, sem plataforma. `Day` é campo e o fuso entra por
               parâmetro — mesma lei do voice_when.
database:      migration 0028. Três tabelas NOVAS, zero alteração em tabela
               existente. Unicidade do dia é índice único em `day`.
sync:          sincroniza. Três EntityKind novos: daily_session,
               daily_objective, daily_reflection. Merge por campo, como o resto.
               Remover objetivo é OpBody::Delete; encerrar e reabrir são campo.
desktop:       widget na faixa "Agora" da Home + dois fluxos em sobreposição +
               gaveta da sessão com histórico. Command palette com seis entradas.
ios:           **não se aplica hoje, porque não há cliente iOS.** Quando houver:
               o widget vira card no topo da Home, os dois fluxos viram folhas
               (bottom sheets) e a sessão vira uma tela da pilha. Nada de
               plataforma está no domínio, então nenhuma regra se reescreve —
               ver §7.
notifications: **nenhuma, e é decisão.** Ver §8.
hermes:        lê (bloco "Os objetivos de hoje" no preâmbulo) e age (cinco ações
               no catálogo). Ver §6.
tests:         domínio (18) + banco (22) + ações e preâmbulo (10) + front puro
               (29). Ver §9.
```

---

## 3. O modelo, e as três coisas que ele NÃO tem

```
DailySession       um dia de trabalho:  day, status, note, startedAt, endedAt
  └── DailyObjective  o que importa:    title, link?, priority, status, position,
  │                                     carriedFrom?
  └── DailyReflection  como foi:        mood?, summary
```

**Não existe `mainObjectiveId` na sessão.** Qual objetivo é o principal já está
em `priority`, e guardar a mesma resposta em dois lugares é como as duas versões
divergem. Com merge por campo seria pior: um dispositivo mudaria `priority` e o
outro `mainObjectiveId`, e os dois venceriam. A garantia de "no máximo um
principal" é estrutural — um índice único parcial em `(session_id) WHERE
priority = 'main'`.

**Não existe coluna `type` no objetivo.** O tipo é a presença e o tipo do
vínculo: sem vínculo é intenção livre, com vínculo é o que ele aponta. Uma coluna
a mais só criaria um segundo jeito de a mesma pergunta ser respondida.

**Não existem quatro campos de reflexão.** `wins` e `blockers` são a mesma frase
repartida em duas caixas, e colunas que a interface nunca preenche são schema
morto. Sobraram `mood` (três valores) e `summary`.

### O dia é campo, e é local

O resto do M/OS guarda UTC e deixa o renderer decidir a que dia um instante
pertence (`calendar.rs`). Ali isso está certo, porque um item de calendário não
tem identidade de dia. **Aqui tem**: "uma sessão por data" é impossível de
garantir se cada leitor decidir sozinho que dia é hoje. Quem trabalha até 23h30
em UTC-3 está no dia 21; em UTC já é dia 22, e o mesmo dia de trabalho viraria
duas sessões.

Então `Day` é `AAAA-MM-DD`, decidido uma vez, no fuso que a tela publicou em
`surface.rs`. O `CHECK` da migration trava o formato exato — `2026-8-21` e
`2026-08-21` são a mesma data e duas chaves diferentes, e o índice único não
veria a duplicata.

---

## 4. Onde cada regra mora

| Camada | Arquivo | O que decide |
| --- | --- | --- |
| Domínio | `crates/mos-core/src/daily.rs` | o que é um dia, o que é progresso, o que vira carry-over, quando uma Task fecha um objetivo |
| Aplicação | `mos-core::DailyService` | começar, acrescentar, promover, resolver, encerrar, reabrir |
| Persistência | `crates/mos-storage-sqlite/src/daily_repository.rs` | as transações, e a emissão de sync dentro delas |
| Comandos | `apps/desktop/src-tauri/src/daily.rs` | que dia é hoje, e ler as outras entidades para o contexto |
| Apresentação | `apps/desktop/src/daily.ts` | em que estado a Home está, o que o resumo diz, quantas vagas sobraram |
| Tela | `DailySession.tsx`, `DailyFlows.tsx` | só desenho |

**Nenhuma regra de negócio vive em componente React.** O `daily.ts` é puro e tem
teste; os `.tsx` chamam serviços e desenham o que voltou. Toda mutação devolve o
**dia inteiro**, e não só o que mudou — a tela nunca recalcula progresso nem
ordem, porque recalcular seria a regra saindo do domínio por uma segunda porta.

### O contexto do dia não mora num serviço

`DailyContext` soma Reminders, Tasks, Projects, Captures e Meetings. Um serviço
de domínio que dependesse dos cinco repositórios só para desenhar uma tela seria
um serviço que não dá para instanciar sem o sistema inteiro. Quem lê é o comando
do desktop; quem decide o que os números significam é a função pura
`compose_context`. É o mesmo desenho do `calendar::compose`.

---

## 5. As três ausências do M/OS que esta feature encontrou

O pedido listava "Tasks com prazo hoje", "Tasks vencidas", "eventos de hoje" e
"Waiting For vencendo". **Nenhuma das quatro existe no M/OS**, e inventar um
número seria pior que a ausência:

| Pedido | Realidade | O que entrou no lugar |
| --- | --- | --- |
| prazo de Task | **Task não tem prazo** (decisão D-1, ver `attention.rs`) | Reminder apontado para a Task, vencendo hoje |
| eventos/compromissos | **`Event` não existe** (decisão D-4) | Reminders de hoje; Meetings entram como fato passado |
| Waiting For | **não existe no `CORE.md`** | nada — a contagem não aparece |
| capacidade do dia (§20) | precisa de agenda futura, que não há | **omitido**, como o próprio pedido autorizou |

O `DailyContext` documenta as quatro ausências no próprio doc comment, para
ninguém tentar "consertar" isso somando um número inventado.

---

## 6. Hermes

### Ele lê

Um bloco novo desce no preâmbulo, **só quando há sessão aberta**:

```
[Os objetivos de hoje (2026-08-21)]
- [ ] Finalizar planta de formas (principal)
- [x] Revisar memorial (secundário)
1 de 2 concluídos. Objetivo é a decisão sobre o que importa hoje — nunca crie
Task para representar um.
[Fim dos objetivos]
```

*"O que falta dos meus objetivos de hoje?"* é uma **pergunta**, e responder por
ação gastaria um turno inteiro — proposta, preview, confirmação — para devolver
três linhas que o M/OS já tem na mão. É o mesmo critério do §15.3 do
`MEETING-AGENT.md`: onde a regra determinística serve, ela ganha da IA.

Sem sessão aberta o bloco **não desce**. Ele custa token em toda mensagem, e um
bloco que só anuncia ausência gastaria isso para informar nada — mesma regra do
`here_block`.

### Ele age

```
mos.day.start          { main, mainRef?, secondaries?: ["..."], note? }
mos.day.add_objective  { title, priority?: main|secondary, taskRef?, projectRef? }
mos.day.set_objective  { objective, status: completed|carried_over|dropped|pending }
mos.day.set_main       { objective }
mos.day.end            { mood?: productive|normal|blocked, summary? }
```

As cinco existem por um motivo estrutural, e não por conveniência: sem elas,
"inicia meu dia" só podia virar `mos.task.create` — a mesma armadilha que o §2 do
`HERMES-ACTION-LAYER.md` registrou quando faltava `ReminderCreate`. Um modelo sem
a ação certa usa a errada.

`note` é a justificativa curta que o §7 do pedido descreve — *"você tem duas
entregas hoje e uma reunião às 15h"*. **Não é raciocínio**: o domínio corta em
400 caracteres, e o preview do cartão mostra a frase, porque autorizar um dia
montado por outro sem ver o porquê seria assinar em branco.

`mainRef` liga o objetivo principal a uma Task ou Project. Sem ele, um dia
montado pelo Hermes teria objetivos de texto solto e a conclusão automática nunca
dispararia.

### O desfazer de cada uma

| Ação | Inverso |
| --- | --- |
| `day.start` | **nenhum** — o inverso de começar o dia não é apagar o dia. Quem começou por engano encerra, e isso é uma decisão |
| `day.add_objective` | remove o objetivo (a exceção à ADR-035, ver §10) |
| `day.set_objective` | devolve ao estado anterior, lido ANTES da mudança |
| `day.set_main` | promove o principal anterior; sem anterior, rebaixa o promovido |
| `day.end` | reabre o dia — e os desfechos gravados nos objetivos **permanecem**, porque resolvê-los e encerrar o dia foram decisões diferentes |

---

## 7. iOS

Não há cliente iOS hoje. Quando houver, **nenhuma regra se reescreve** — o
domínio inteiro está em `mos-core`, que não conhece plataforma, e a persistência
está em `mos-storage-sqlite`, que roda nos dois lados.

O que muda é a manifestação:

| Desktop | iOS |
| --- | --- |
| widget na faixa "Agora" da Home | card no topo da Home |
| fluxos em sobreposição centrada | folhas (bottom sheets) |
| sessão em gaveta lateral | tela da pilha de navegação |
| arrastar para reordenar | toque longo e arrastar |
| Command palette | busca contextual |
| — | haptic ao concluir o objetivo principal |

`daily.ts` continua servindo os dois: ele decide **o que dizer**, não como
desenhar. O que precisa de uma cópia por plataforma é o `.tsx`, e é exatamente o
que a regra 3 do `FEATURE-DEVELOPMENT.md` permite.

---

## 8. Notificações: nenhuma, e é decisão

O pedido descreve dois avisos possíveis: "ainda não definiu seus objetivos de
hoje" pela manhã, e "seu dia ainda está aberto" no fim da tarde. Os dois são
**notificações que o sistema inventa sobre um compromisso que ninguém assumiu**,
e o mesmo pedido diz, duas seções acima, para não criar notificações agressivas.

A infraestrutura existe — `AttentionService::create_at` com
`ReminderSource::System` agenda qualquer coisa. O que não existe é a decisão de
que o M/OS deve cobrar a pessoa por não ter começado o dia. Quando ela existir, o
gancho está pronto e não precisa de nada novo.

A porta de ontem em aberto (§24 do pedido) é resolvida **na tela**, e não por
aviso: uma linha discreta no widget da Home, com um botão para encerrar. Ela
informa e oferece — não bloqueia.

---

## 9. O que os testes provam

**Domínio** (`mos-core::daily`, 18 testes) — o dia sai do offset de quem está
olhando; data inválida é recusada em vez de normalizada; o dia anterior atravessa
mês, ano e ano bissexto; abandonar não piora o placar; só o objetivo que **é**
aquela Task fecha junto com ela; reflexão vazia não vira linha; `not_started`
nunca volta do banco.

**Banco** (`tests/daily_session.rs`, 22 testes) — dois inícios no mesmo dia são
recusados; o dia sobrevive ao fechamento do aplicativo; só existe um principal;
concluir pelo Kanban conclui o objetivo **na mesma transação**; tirar a Task do
Done devolve o objetivo a pendente; concluir uma Task não reescreve o placar de um
dia encerrado; objetivo vinculado a entidade apagada continua legível; começar
hoje fecha ontem sem decidir o destino de nada; a corrente de carry-over conta os
elos; remover o elo antigo não derruba o novo; reordenar ignora id de outra
sessão; a busca escapa `%` e `_`.

**Ações e preâmbulo** (`action.rs`, `agent.rs`, 10 testes) — um dia sem principal
é recusado; objetivo sem prioridade nasce secundário; desfecho desconhecido é
recusado **na leitura**, e não na execução; encerrar não exige campo nenhum;
o cartão mostra a justificativa; toda ação do catálogo tem função declarada; o
bloco de hoje entra antes do catálogo.

**Front puro** (`daily.test.ts`, 29 testes) — "ontem em aberto" ganha de "não
iniciado" mas não de "hoje já começou"; zero não vira linha no resumo; a ordem é
a da urgência e não a da grandeza; o principal pendente ganha o marcador cheio;
levado e abandonado se distinguem sem cor; mover mira um vizinho e não um índice;
a data civil não passa por `new Date(texto)`.

**O que NÃO é testado, e por quê:** não há teste de DOM neste repositório
(`vitest.config.ts`), então nenhum componente `.tsx` tem teste. É a razão de
`daily.ts` existir: tudo que decide algo saiu dos componentes para lá.

---

## 10. As duas exceções a regras do M/OS, e por quê

**1. `remove_objective` APAGA.** A ADR-035 diz que desfazer arquiva e nunca
apaga, e o resto do M/OS obedece. Aqui não: um objetivo removido antes de o dia
acabar **nunca chegou a ser história**. Arquivar também não serviria — objetivo
não tem `lifecycle_state`, porque o registro é o dia inteiro, e ele continua de
pé. Quem quer manter o registro usa `dropped`, que é a outra porta e está logo
acima no menu.

**2. O `carried_from` aponta para a própria tabela.** É a única auto-referência do
schema. Ela existe porque titulo não serve de chave — a pessoa pode reescrever o
objetivo ao carregá-lo — e sem ela "isto já foi adiado quatro vezes" só se
responderia comparando strings. `carry_depth` tem teto explícito de 365 elos:
o schema não proíbe ciclo, e um ciclo não pode virar laço infinito na abertura da
Home.

---

## 11. O que ficou de fora

| Fora | Motivo |
| --- | --- |
| percepção de capacidade do dia (§20) | exige agenda futura. `Event` não existe (D-4), e Meeting é gravação — fato passado. Somar "2h30 em reuniões" a partir de reuniões que já aconteceram descreveria ontem, não hoje |
| notificações | ver §8 |
| Waiting For no contexto | o conceito não existe no `CORE.md` |
| reordenar por teclado na sessão | o arrasto existe; as setas, não. A Home tem as duas (`Arrangeable`), e a paridade aqui é dívida registrada |
| linkar objetivo a Capture/Resource/Meeting pela interface | o domínio, o banco e o Hermes aceitam os cinco tipos. O seletor da tela oferece Task e Project, que são os dois que a busca unificada devolve como candidatos úteis |

---

## 12. A Weekly Review

Construída em 2026-08-21. Ver `docs/superpowers/specs/2026-08-21-weekly-review-design.md` e a ADR-055.

Ela **consome** esta camada e não acrescenta nada a ela: `carried_from` dá a
corrente, `dropped` dá o abandono, o vínculo dá o Project, e `sessions()` dá a
contagem de dias. A única coisa que ela guarda é um texto por semana — a
narrativa inteira é derivada.

Ela também pagou uma dívida daqui: o `history()` fazia uma consulta de reflexão
por dia listado, e a semana precisava de sete de uma vez. Agora são três
consultas para N dias.

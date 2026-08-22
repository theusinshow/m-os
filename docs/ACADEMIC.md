# ACADEMIC — a camada acadêmica do M/OS

Implementação: `crates/mos-core/src/academic.rs`, `crates/mos-storage-sqlite/src/academic_repository.rs`,
`apps/desktop/src/AcademicPage.tsx`. Tabelas: migration `0031_academic.sql`.
Decisões: `DECISIONS.md`, ADR-058.

---

## 1. O que ele é, e o que ele não é

O M/Academic responde cinco perguntas:

> O que tenho da faculdade? · O que está chegando? · O que preciso fazer? ·
> O que preciso estudar? · Como estou em cada disciplina?

**Ele não é um portal da faculdade.** Não guarda ementa, não fala com a
instituição e não representa um curso. Tudo que ele mostra foi a pessoa que
escreveu.

## 2. A regra que organiza tudo

**Faculdade é um CONTEXTO sobre os primitivos do M/OS, e não um segundo M/OS.**

| O que a faculdade precisa | O que ela usa |
| --- | --- |
| executar uma entrega | `Task` de verdade, no Kanban |
| guardar um material | `Resource` de verdade, na Library |
| aparecer no dia | Calendário e Daily Session que já existem |
| ser encontrado | a Search global |

O que o módulo acrescenta é só o que não tinha lugar: o **período**, a
**disciplina**, o **peso na média** e o **tempo de estudo**.

## 3. O modelo

```
Semester
└── Subject
    ├── Assignment ──→ Task
    ├── Exam
    ├── (materiais) ──→ Resource
    └── StudySession
```

**Não existe tabela de notas.** A nota mora na avaliação que a produziu —
`score` e `max_score` em `academic_exams` e `academic_assignments`. Uma tabela
`grades` separada seria uma terceira fonte para o mesmo fato: a prova diria 7,5
e a nota diria 8,0, e nada no banco diria qual está certa.

**O semestre não guarda "ativo".** O status é derivado das datas. Guardá-lo
criaria a linha que diz "ativo" num semestre que acabou em dezembro, e o sistema
passaria a depender de alguém corrigi-la.

## 4. A média

Três decisões, todas com teste em `mos_core::academic`:

**Só entra o que tem nota E teto.** Prova marcada sem nota não é zero — é prova
que ainda não aconteceu. Tratá-la como zero faria a média desabar em março e
subir sozinha depois.

**A escala é a fração, e não o valor cru.** Um trabalho de 0 a 100 com 80 e uma
prova de 0 a 10 com 8 valem o mesmo. Somar 80 com 8 diria que o trabalho pesa
dez vezes mais.

**Peso zero significa "não conta"** quando há outra avaliação com peso — é a
lista de exercícios que não vale nota. Sem peso nenhum configurado, a média é a
aritmética; sem esse caso, a divisão por zero devolveria `NaN` e a tela mostraria
"NaN" como nota.

`nota_necessaria` já existe e responde *"quanto preciso tirar na próxima?"*. Ela
não tem tela ainda — a estrutura é que já responde.

## 5. O tempo

`Horizonte` — `overdue`, `today`, `tomorrow`, `this_week`, `later` — é calculado
no **fuso de quem está olhando**, publicado pelo renderer em `surface.rs`.
Decidi-lo em UTC jogaria toda entrega de madrugada para o dia seguinte.

Duas sutilezas com teste: a entrega das 10h vista às 15h é **atraso**, e não
"hoje"; e "esta semana" são os próximos sete dias, não "até domingo" — um prazo
no sábado importa tanto quanto um na sexta.

## 6. Progresso

A única medida de progresso que este módulo produz é **quanto do período já
passou**, contando dias. "Progresso da disciplina" em porcentagem exigiria saber
quantas atividades o semestre *terá*, e ninguém sabe isso em março. O resto é
**estado**, não número: `situacaoDe` devolve "2 atrasadas", "1 pendente" ou "em
dia".

## 7. As integrações

| Integração | Como |
| --- | --- |
| **Tasks** | `academic_assignments.task_id`, `ON DELETE SET NULL`. Concluir a atividade fecha a Task; reabrir reabre. Apagar a Task deixa a atividade de pé — ela perde o braço executor, não a existência. |
| **Calendar** | `CalendarKind::AssignmentDue` e `ExamScheduled`. Não há segundo calendário: o do M/OS ganhou duas fontes. |
| **Start My Day** | `DailyContext.academic` traz entregas de hoje, atrasos e estudo sugerido, prontos de `academic::compose_today`. O objetivo criado aponta para a **Task** da atividade, quando ela existe. |
| **End My Day** | Bloco com o estudo do dia, as entregas atrasadas e as provas próximas. |
| **Search** | `SearchItem::Subject`, `Exam` e `Assignment`. LIKE e não FTS: o volume é limitado por disciplinas vezes um punhado de itens. |
| **Hermes** | `EntityKind::Subject`, `Exam` e `Assignment`. Ele já pode apontar para uma prova ao responder "quando é minha próxima?". |
| **Sync** | Cinco tipos emitem: `academic_semester`, `academic_subject`, `academic_assignment`, `academic_exam`, `academic_study_session`. O material viaja como **relação**, e não como campo. |

## 8. Estudo

Tabela própria, e não `time_entries` do CronoCAD: aquela mede hora **cobrável** —
carrega cliente, arredondamento e valor por hora, e `settle()` converte tudo em
dinheiro. Estudar não se fatura, e enfiar estudo ali faria a receita do Painel
somar horas que ninguém vai cobrar.

Uma sessão aberta por vez, garantida por índice único. Começar a estudar **fecha**
a sessão esquecida em vez de recusar a nova: o app pode ter sido fechado com o
cronômetro rodando, e travar a pessoa por isso seria pior que a aproximação.

## 9. Fora de escopo, e por quê

| Fora | Motivo |
| --- | --- |
| Gemini, Study Agent, RAG | O §35 do pedido. A arquitetura não impede: `AcademicToday` e `Compromisso` já são o contexto que um agente consumiria. |
| Google Drive | Material é `Resource`. Quem resolver storage externo mexe em `resources`, e a junção continua valendo sem uma linha de mudança. |
| KNOW/OS | Mesma porta: a relação disciplina→recurso já existe e é extensível. |
| Prioridade em `Task` | Continua não existindo. `Assignment` tem prioridade própria; promovê-la a `Task` é outra feature. |
| Notificação de prazo | O M/OS já tem Reminders. Criar um segundo canal de aviso para a faculdade duplicaria o Attention System. |

## 10. iOS

O domínio inteiro está em `mos-core` e compila idêntico nos dois lados. Não há
superfície iOS construída hoje (`PLATFORMS.md` §3); quando existir, o painel é a
mesma lista, com o mesmo `compose_dashboard`.

---

## 11. A camada operacional

Até aqui o M/Academic respondia *"o que existe na faculdade"*. Esta camada o faz
responder **"o que exige minha atenção agora"** — e a diferença entre as duas
perguntas é inteiramente a decisão de quem estuda.

Implementação: `mos-core/src/academic_decision.rs`, migration `0034`,
`AcademicPage.tsx`.

### 11.1 Os dois vocabulários

| | quem escreve | o que responde |
| --- | --- | --- |
| `status` | o Univirtus, ou a pessoa à mão | o que o **portal** registra |
| `decision` | só a pessoa | o que **eu** resolvi |

Eles discordam de propósito, e o caso comum é esse: a pessoa entrega às 23h e o
portal só atualiza no dia seguinte. Até lá o M/OS continuaria cobrando algo já
feito — a não ser que ela possa dizer "já entreguei", e que essa frase sobreviva
ao próximo sync. Sobrevive: os `UPDATE` do
`academic_provider_repository.rs` listam colunas, e nenhum nomeia `decision`,
`decided_at` ou `planned_at`.

Três decisões, e só três: `none`, `done`, `skipped`.

**Não existe `planned`** porque planejado não é decisão: é um fato derivado de
`planned_at` existir. Guardar o estado ao lado da data criaria a linha que diz
"planejado" sem data, e a que diz "não planejado" com data.

**Não existe `ignored`** separado de `skipped`. A diferença não muda nada: os
dois saem da atenção, ficam no histórico e podem voltar.

### 11.2 As faixas

Derivadas por `academic_decision::faixa_de`, **nunca gravadas**: "precisa de
atenção" muda sozinho toda madrugada, e uma coluna estaria errada todo dia.

```
PRECISA DE MIM   vencido, hoje, amanhã, ou prova nos próximos 3 dias
ESTA SEMANA      data nos próximos sete dias
DEPOIS           mais adiante
HISTÓRICO        resolvido, descartado, ou resto de calendário antigo
```

Cada compromisso cai em **uma** faixa. Aparecer em duas faria a pessoa decidir
duas vezes sobre a mesma coisa e contar duas vezes o que falta.

Duas regras impedem a tela de virar depósito:

- **semestre encerrado → histórico.** Uma atividade vencida há 151 dias de um
  período fechado não é urgência, é arqueologia;
- **prazo anterior ao início do próprio semestre → histórico.** O Univirtus
  republica etapas de ciclos antigos: "Estática dos Corpos" chega no 2026B2
  (julho) com entregas vencidas em março e maio. Nada que valha para este
  período vence antes de ele começar, e esse é o sinal.

Os contadores do card de disciplina usam a **mesma** regra. Sem isso o card
dizia "3 atrasadas" enquanto a faixa dizia "0 compromissos", uma frase acima da
outra — foi o defeito que a primeira verificação em tela pegou.

### 11.3 O prazo e o plano são duas datas

```
due_at      quando o prazo FECHA          — do portal
planned_at  quando eu vou FAZER           — meu
```

Confundi-las é o erro que faz o calendário mostrar "entregar APOL" às 23h59 de
sexta, quando a pessoa vai escrever na quarta. O calendário do M/OS ganhou
`CalendarKind::AcademicPlanned` para o bloco planejado — ele entra **além** do
prazo, e não no lugar dele.

`planned_at` mora no compromisso acadêmico, e não na Task: a `Task` do M/OS não
tem data planejada nem prioridade (o ADR-058 já registrou que promovê-las é
outra feature), e o plano sobrevive à Task ser apagada.

### 11.4 A hora exata

O Univirtus manda `23:59`, e a hora importa: "vence 23h59" é diferente de "vence
hoje". Mas um compromisso criado à mão sem hora vira meia-noite.

A regra é **meia-noite em ponto significa "sem hora"**. Ela não inventa 23:59 no
lugar — inventar horário é pior que omiti-lo — e não esconde a hora quando ela
existe. Vale nos dois lados: `academic_decision::tem_hora_real` e
`academic.ts::temHoraReal`.

**O fuso é do renderer, e o sync espera por ele.** A primeira sincronização real
rodou antes de a tela montar, leu offset zero, gravou `23:59Z` e a tela mostrou
`20:59`. Agora `surface::offset_minutes` é `Option`: zero é um fuso legítimo,
"ainda não sei" é outra coisa, e `univirtus_sync` recusa rodar enquanto não
souber. A correção se cura sozinha na sincronização seguinte, porque a impressão
digital da avaliação inclui o instante.

### 11.5 O que a pessoa faz na tela

Uma ação primária visível — **Planejar** — e o resto no menu de cada linha. Sete
botões expostos por linha transformariam uma lista de doze itens em oitenta e
quatro alvos de clique.

| Ação | Efeito |
| --- | --- |
| Planejar | `planned_at` + duração; vira bloco no Calendário |
| Já entreguei | `decision = done`; sai da atenção |
| Não vou fazer | `decision = skipped`; sai da atenção |
| Desfazer o plano | limpa `planned_at` |

Todas com **Undo** em vez de confirmação: são reversíveis, e perguntar "tem
certeza?" a cada item transformaria a operação mais comum da tela em dois
cliques.

### 11.6 O dia

`AcademicToday` ganhou duas listas, e as três antigas passaram a ler
`needs_attention` em vez do horizonte puro — o horizonte só sabe de data, e não
de decisão:

- `planned_today` — o que **eu decidi fazer hoje**, vença quando vencer. É a
  diferença entre o Start My Day mostrar prazos e mostrar ações;
- `decided_today` — o que foi resolvido hoje, para o End My Day.

### 11.7 O Hermes

Os candidatos acadêmicos passaram a carregar a decisão no detalhe: "vence sexta ·
marcada como não vou fazer", "vence sexta · planejada para quarta às 19h". Sem
isso ele não consegue responder *"o que eu já marquei como não vou fazer?"* nem
*"o que ainda não planejei?"* — as duas perguntas que esta camada criou.

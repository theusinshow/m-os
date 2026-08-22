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

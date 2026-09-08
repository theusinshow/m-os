# TASK-EXECUTION — A Task passa a representar trabalho

**Decisão:** ADR-066 · **Migration:** `0039_task_execution.sql` · 2026-09-08

---

## 0. O pedido, em uma frase

> Uma Task não deve ser apenas *"fazer X"*. Ela precisa conseguir representar
> *"fazer X"* com vários passos menores.

E a restrição que vem junto, que vale tanto quanto:

> O M/OS deve continuar extremamente direto, rápido e leve. Sem síndrome de
> Jira.

Tudo neste documento é a tensão entre essas duas frases.

---

## 1. Os quatro conceitos

A confusão entre dois destes é como uma lista de tarefas vira burocracia.

| | é | não é |
|---|---|---|
| **Task** | a unidade de trabalho: *"revisar o projeto estrutural"* | — |
| **Checklist item** | um passo que se CONCLUI: *"conferir níveis"* | um cartão do Kanban. Nunca vira um |
| **Subtask** | trabalho que merece existir sozinho — estado, prazo e checklist próprios | um item de checklist com outro nome |
| **Nota** | contexto que se LÊ: *"manter cobrimento de 5 cm"* | algo que se conclui |

**A fronteira, escrita:** o que merece existir sozinho no quadro é Subtask; o
que só faz sentido dentro do trabalho maior é item de checklist.

---

## 2. O checklist é entidade, e essa é a decisão inteira

Um item tem tabela própria (`task_checklist_items`), id próprio e operação de
sync própria. Isso não é preferência de esquema — é a única forma de o cenário
abaixo terminar certo:

```
PC:      marca "corrigir nível"
celular: acrescenta "enviar para o Victor"
         ↓ sincroniza
os dois gestos sobrevivem
```

Com um array numa coluna da Task, os dois seriam escritas concorrentes sobre o
**mesmo campo**: o merge por campo escolheria uma e mandaria a outra para
`sync_conflicts`. Com uma linha por item, eles nem se encontram.

*Merge por campo não serve para conjunto* — a mesma frase do `SYNC.md` §13,
aplicada dentro de uma entidade em vez de entre duas.

Ganho de lado: `GROUP BY task_id` responde o progresso do cartão numa consulta
indexada. O mesmo número, tirado de um JSON, exigiria ler e desserializar a
coluna de toda Task do quadro.

---

## 3. Prazo e lembrete

A ADR-066 supera a decisão D-1, que mantinha `Task.due_at` fora do M/OS. Os dois
passam a coexistir, e respondem perguntas diferentes:

| | responde | mora em |
|---|---|---|
| `due_at` | quando o trabalho **vence** | `tasks`; aparece no Calendar como `TaskDue` |
| Reminder | quando o M/OS **interrompe** | `reminders`, pelo par `(target_type, target_id)` da 0015 |

**Ter prazo não gera aviso nenhum sozinho.** Reminder relativo a prazo,
escalonamento e digest deixaram de estar bloqueados por falta de dado e passaram
a depender de uma decisão sobre quando é legítimo interromper. Nenhum foi
implementado.

---

## 4. O checklist do `FEATURE-DEVELOPMENT.md` §2

```
core:          ChecklistItem, TaskDetail, EditTask, parse_checklist_lines;
               Task ganha due_at, priority, estimate, parent, blocked_by,
               waiting_for, follow_up e os dois contadores derivados.
               Priority é a MESMA do Reminder — não há segunda escala.

database:      migration 0039, aditiva. Sete colunas em `tasks` (todas com
               default que significa "como era antes"), mais
               task_checklist_items, task_checklist_search e resource_tasks.
               Task antiga continua válida, sem prazo e sem checklist.

sync:          sincroniza. O item é entidade própria (`task_checklist_item`);
               marcar é o campo `completedAt`; a referência viaja como relação
               `resourceTask`. A cobertura sobe para a geração 3 e o backfill
               roda de novo. A EMISSÃO da Task é um diff — só o campo que
               mudou viaja (ver §6).

desktop:       cartão do Kanban com progresso e checklist expansível (um por
               vez, recolhido por padrão); gaveta reescrita, que salva campo
               por campo; criação rápida com progressive disclosure.

ios:           `mos-web` é o M/OS de bolso, e ele é a manifestação móvel hoje.
               A tela de Task ganha checklist com alvo de 44px, prazo e
               prioridade; a lista de Fazer mostra `3/6` e marca atraso.
               SEM arrastar para reordenar — reordenar por toque compete com a
               rolagem, e quem reordena está no PC; quem está na rua executa.
               O app iOS nativo não existe ainda; quando existir, consome as
               mesmas rotas.

notifications: NENHUMA nova. Prazo não notifica — é a §3 acima. O lembrete
               continua sendo o único caminho de interrupção, e ele já existe.

hermes:        três ações novas (`mos.task.add_checklist`,
               `mos.task.check_item`, `mos.task.set_plan`) e `mos.task.create`
               aceita checklist, prazo e prioridade. O contexto de cada Task
               candidata passa a levar `4/7`, prazo, prioridade e waiting-for
               numa linha — o checklist inteiro NÃO desce, porque doze
               candidatos com seis passos cada seriam setenta e duas linhas em
               toda mensagem.

tests:         domínio (parser de colagem, progresso, EditTask::validate);
               repositório (ciclo do item, ordem, busca pelo passo, migration
               antiga); sync de dois aparelhos (marcar+acrescentar, apagar,
               prazo+prioridade convivendo); emissão (diff, uma operação por
               gesto); rotas do bolso; e `tasks.test.ts` no desktop.
```

---

## 5. O que NÃO entrou, e por quê

| | por quê |
|---|---|
| **tags** | competiriam com Project/Área sem responder nada que eles não respondam. Project tem prioridade maior, e o pedido diz isso |
| **recorrência** | exige um gerador de ocorrências e não há caso concreto. A regra que ficaria escrita — checklist reseta na ocorrência nova, e completar hoje não altera ontem — está registrada aqui para quando houver |
| **auto-concluir a Task** | a tela OFERECE ("Todos os itens concluídos · [Concluir Task]") e nunca executa. Uma Task que se fecha sozinha é o sistema afirmando algo que ninguém disse |
| **hierarquia além de um nível** | o esquema aceita; a interface mostra um. A Task que já é filha não aparece como mãe possível |
| **slash commands** | não há infraestrutura para eles fora do Hermes, e criá-la para cinco atalhos seria arquitetura por conveniência. A colagem de múltiplas linhas cobre o caso real |
| **segunda dependência** | uma coluna responde a pergunta real. No dia em que houver duas travas de verdade, o caminho é uma tabela — não uma segunda coluna |

---

## 6. Duas armadilhas que só apareceram em teste

Ficam escritas porque nenhuma das duas dá erro na hora, e as duas custam dado.

**A emissão precisa ser um diff.** A escrita de Task é autoritativa (os onze
campos chegam prontos), e emitir os onze faria mudar a prioridade no celular
carregar junto o `dueAt: null` que ele leu antes — apagando, por ser mais
recente, o prazo que o PC acabou de pôr. Dois gestos em campos diferentes, e um
vencendo o outro. Com quatro campos era teoria; com onze é o caminho normal.

**A projeção do sync precisa manter o índice FTS.** Ela materializava linhas em
`tasks` sem tocar em `task_search`. Duas consequências, ambas reais: a busca não
achava o que veio do outro PC, e editar aqui uma Task criada lá falhava dizendo
que o banco estava corrompido — porque o comando `'delete'` do fts5 contra uma
linha ausente devolve `SQLITE_CORRUPT`, e não "não achei". O defeito era
anterior a esta feature e foi corrigido junto.

---

## 7. Onde as coisas moram

| | |
|---|---|
| domínio | `crates/mos-core/src/work.rs` |
| repositório | `crates/mos-storage-sqlite/src/work_repository.rs` |
| projeção de sync | `crates/mos-storage-sqlite/src/sync_projecao.rs` |
| ações do Hermes | `crates/mos-core/src/action.rs`, execução em `apps/desktop/src-tauri/src/jarvis.rs` |
| desktop | `apps/desktop/src/{tasks.ts,Checklist.tsx,TaskDrawer.tsx}` e o `TaskCard` em `App.tsx` |
| bolso | `apps/mos-web/ui/src/paginas/{Task.tsx,Checklist.tsx,Fazer.tsx}` |

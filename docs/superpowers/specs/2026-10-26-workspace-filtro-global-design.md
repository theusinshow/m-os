# Spec — O Workspace vira filtro global do M/OS

Data: 2026-10-26
Origem: pedido do dono — três workspaces (PROSUL, empresa; DEV, programação;
MMD, desenhos) e "separar melhor as coisas".
ADR: ADR-093 (a escrever junto da implementação).

---

## 0. O problema

Hoje, trocar de Workspace já muda três coisas: os widgets da Home (quais e onde),
os atalhos do leque radial e os Projects/Apps listados na Home. Todo o resto
mostra tudo junto. Tarefas da empresa aparecem ao lado das de programação; uma
captura feita "em PROSUL" não sabe que foi feita lá; o Hermes recebe o mesmo
contexto qualquer que seja o Workspace.

A meta é que estar em PROSUL signifique ver PROSUL — e que "Tudo" continue
existindo para quem quer o conjunto.

## 1. Decisões do dono

1. O Workspace é um **filtro global**, com a opção **TUDO**.
2. Task solta e Capture **nascem no Workspace atual**. Em TUDO, nascem sem
   Workspace.
3. **Sem Workspace aparece em todos.** Nada some por não ter sido classificado.
4. O Hermes muda **só o contexto**: em PROSUL ele enxerga PROSUL, e as conversas
   ficam separadas por Workspace. Sem instruções próprias por Workspace, sem
   bloqueio de ação.
5. Vínculo por **uma coluna `workspace_id` opcional**, e não por junção: cada
   Task ou Capture nasce em no máximo um.

## 2. A regra — `pertence`

Uma função em `mos-core`, sem plataforma, e a única fonte da resposta. Nenhuma
tela reimplementa a regra (regra dura 3 do `AGENTS.md`).

Para um item e um Workspace W **ativo**:

| Item | Workspaces declarados | Aparece em W quando |
| --- | --- | --- |
| Task | o próprio `workspace_id` ∪ os Workspaces do Project dela | W está entre os declarados, **ou** não há nenhum declarado |
| Capture | o próprio `workspace_id` | idem |
| Project | `project_workspaces` | idem |
| Resource | `resource_workspaces` | idem |
| App | `app_workspaces` | como já é hoje |

**Workspace arquivado.** Um declarado que esteja arquivado não conta. Se todos os
declarados de um item estão arquivados, o item aparece **só em TUDO** — ele tem
dono, o dono saiu de cena, e espalhar o item por todos os outros seria o
contrário de separar.

**TUDO** mostra tudo, sempre.

**Fora do filtro**, de propósito: Calendário, Reuniões, Faculdade, Finance,
Tempo, lembretes e a atenção do Piloto. São domínios do dia inteiro, não de um
modo de trabalho; filtrar um prazo por estar no Workspace errado é perder o
prazo. Podem entrar depois, um por um, com motivo.

### Por que o set, e não o predicado no front

O front recebe as listas inteiras e precisa filtrar. Levar a regra para o
TypeScript seria a segunda cópia. Em vez disso o core expõe:

```
escopo_do_workspace(workspace_id) -> EscopoDoWorkspace {
    tasks: HashSet<TaskId>, captures: HashSet<CaptureId>,
    projects: HashSet<ProjectId>, resources: HashSet<ResourceId>,
}
```

O front só pergunta "este id está no conjunto?". O conjunto é recalculado no
`data-changed`, como o resto.

## 3. Os oito eixos

```
core:          pertence + escopo_do_workspace em mos-core (work.rs/service.rs);
               CreateTaskInput e CreateCaptureInput ganham workspace_id;
               UpdateTaskInput também (mover de Workspace)
database:      migration 0043_workspace_dos_itens.sql — ADD COLUMN workspace_id
               TEXT REFERENCES workspaces(id) ON DELETE SET NULL em tasks,
               captures e conversations, com índice. Não destrói nada; o que
               existe fica NULL e, pela regra, aparece em todos
sync:          sincroniza. Campo próprio ("workspaceId") nos mapas task, capture
               e conversation de sync_projecao; reconciliação por campo, como os
               demais. Sem tabela nova, então a geração da cobertura não sobe.
               Réplica antiga guarda o campo que não conhece e reenvia
desktop:       seletor global no topo (sai da Home), com TUDO; Tarefas, Inbox,
               Projects, Biblioteca e Hermes filtram; criar herda; campo
               WORKSPACE no detalhe de Task e de Capture; filtro SEM WORKSPACE
               para a triagem do que já existe
ios:           não muda nesta fase. O bolso não conhece Workspace: mostra tudo e
               o que ele captura chega sem Workspace (aparece em todos). Seletor
               no bolso é fase 2 — exige a noção de "atual" num aparelho que hoje
               não tem nenhuma
notifications: nenhuma muda. Lembrete dispara em qualquer Workspace
hermes:        o contexto (candidatos, Projects, Tasks, Captures) passa por
               pertence com o Workspace da conversa; conversa nova nasce no
               Workspace atual; a lista de conversas filtra; o que ele cria herda
               o Workspace da conversa. Conversa do bolso nasce sem Workspace
tests:         pertence (cada linha da tabela do §2, arquivado, TUDO); migration
               sobre banco com dado; sync do campo em duas réplicas; criação
               herdando; o front com o set (filtro puro, testável em vitest)
```

## 4. Desktop em detalhe

**Seletor.** Hoje vive na Home (`App.tsx`, botões por Workspace). Sobe para a
barra do topo, visível em toda página. Primeira opção: TUDO. O atual continua em
`localStorage` (`m-os-current-workspace`) — é preferência da máquina, como o
tema; `""` é TUDO.

**Criar.** Captura rápida, Task nova, Capture → Task e ações do Hermes recebem o
`workspace_id` atual. Capture → Task herda o Workspace da Capture, não o atual:
quem triou em DEV algo capturado em PROSUL continua tendo uma coisa de PROSUL.

**Mover.** Campo WORKSPACE no `TaskDrawer` e no visualizador de Capture: um
`<select>` com "Sem workspace" e os ativos. Uma Task cujo Project dá o Workspace
mostra isso escrito ("PROSUL, pelo Project"), para a pessoa entender por que ela
aparece lá.

**Triagem.** Nas páginas filtradas, um filtro "SEM WORKSPACE" mostra só o que
ainda não foi classificado. É o caminho para o histórico, que nasce todo sem
Workspace.

**Vazio.** Workspace sem nada mostra o estado vazio com escopo que já existe
(`2026-08-14-mos-scoped-empty-state-design.md`), dizendo que é o filtro, e não a
falta de dado, e oferecendo TUDO.

## 5. Fora de escopo

- Item em vários Workspaces (Task/Capture).
- Instruções próprias do Hermes por Workspace; bloqueio de ação fora dele.
- Cor, ícone ou sigla por Workspace — entra melhor junto do Lab de ícones.
- Seletor no bolso.
- Filtrar Calendário, Reuniões, Faculdade, Finance, Tempo, lembretes.

## 6. Depois de pronto

Criar PROSUL, DEV e MMD no app, vincular os Projects existentes a cada um, e
usar o filtro SEM WORKSPACE para triar o resto.

# Anotacoes e pendencias por projeto

Data: 2026-07-13
Status: aprovado, aguardando implementacao

## Problema

O CronoCAD registra horas, mas nao guarda **contexto de trabalho**. Hoje o
usuario nao tem onde anotar "o cliente pediu para mudar a cota do corte BB" ou
"esperando o arquivo topografico do Joao" — isso vive fora do app (papel, bloco
de notas) e se perde.

Sao duas necessidades distintas:

- **Contexto solto** — texto corrido, sem estrutura, que o usuario re-le quando
  volta ao projeto.
- **Pendencias** — itens curtos que precisam ser feitos e que somem quando
  resolvidos.

## Objetivo

Dar a cada projeto um lugar para anotacoes livres e uma lista de pendencias, e
fazer as pendencias abertas aparecerem no Painel — onde o usuario as encontra
sem ir atras delas.

Nao-objetivos (decididos explicitamente com o usuario):

- **Sem notificacao, sem data/hora, sem agendamento.** "Lembrete" aqui significa
  apenas "fica visivel". Nada dispara alerta.
- **Sem o texto livre no Painel.** Ele poluiria o painel; seu lugar e dentro do
  projeto. No Painel aparece apenas um indicador discreto de que o projeto tem
  anotacoes.
- Sem prazos, sem sub-tarefas, sem anexos.

## Modelo de dados (migration `0005_project_notes.sql`)

Duas naturezas diferentes, dois tratamentos:

```sql
-- Texto livre: 1-para-1 com o projeto, editado no lugar.
ALTER TABLE projects ADD COLUMN notes TEXT;

-- Pendencias: N-para-1.
CREATE TABLE project_todos (
  id          TEXT PRIMARY KEY,
  project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  text        TEXT NOT NULL,
  done        INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0,1)),
  done_at     TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
CREATE INDEX idx_project_todos_project ON project_todos(project_id);
```

Registrar em `src-tauri/src/database/mod.rs` (`migrations()`) com
`version = 5` e `include_str!`.

**Hard delete nas pendencias (sem `deleted_at`), decisao deliberada.** Uma
pendencia nao e registro de tempo nem gera cobranca; se o usuario apagou um item
de checklist, ele quis apagar. A regra de preservar historico (regra critica 5 e
o soft delete de `time_entries`) existe para proteger o que vira dinheiro.

Timestamps em ISO 8601 UTC, como o resto do schema.

## Backend

Comandos novos em `src-tauri/src/commands/mod.rs`, finos, delegando para um
`repository/notes.rs` novo. Fluxo padrao do projeto:
`store -> service (invoke) -> command (valida) -> repository -> SQLite`.

| Comando | Acao |
|---|---|
| `update_project_notes(project_id, notes)` | grava o texto livre (`notes` vazio -> `NULL`) |
| `list_todos()` | todas as pendencias (o Painel filtra as abertas) |
| `create_todo(project_id, text)` | cria; `text` vazio e rejeitado |
| `set_todo_done(id, done)` | marca/desmarca; grava/limpa `done_at` |
| `update_todo_text(id, text)` | corrige o texto |
| `delete_todo(id)` | remove definitivamente |

Nada toca o cronometro nem `time_entries`. Risco zero para os registros.

## UI

### Painel (`DashboardPage.tsx`)

Painel novo **"Pendencias"** na grade inferior (hoje "Sessoes recentes" +
"Linha do tempo detectada").

- Lista **apenas as pendencias abertas** (`done = 0`), agrupadas por projeto.
- O projeto do **cronometro ativo sobe para o topo e fica destacado**. Assim o
  card serve tanto para "o que tenho pendente hoje" quanto para "o que eu tinha
  anotado neste projeto que estou tocando agora".
- Clicar na caixinha marca como feita e o item **sai da lista** ali mesmo
  (`set_todo_done`).
- Projeto com texto livre preenchido mostra um indicador discreto
  ("Aurora tem anotacoes"), linkando para o modal em Projetos.
- Vazio: "Nenhuma pendencia. Anote lembretes na tela de Projetos."

Nao ha criacao de pendencia pelo Painel — criar exige escolher o projeto, o que
pertence ao contexto de Projetos.

### Projetos (`ProjectsPage.tsx`)

A tela de Projetos e uma tabela, sem pagina de detalhe. As anotacoes entram como
**modal por projeto**, seguindo o padrao ja existente (`ProjectForm`,
`ClientsModal`) — nenhuma rota nova.

- Icone de bloquinho (`StickyNote`, lucide-react) na coluna de acoes da linha.
- Abre `ProjectNotesModal` — titulo "Anotacoes — {projeto}", com duas secoes:
  1. **Anotacoes** — `textarea` de texto livre, salva no `blur`.
  2. **Pendencias** — campo de adicionar + lista com caixinha, editar texto e
     excluir. As concluidas ficam numa secao "Concluidas" recolhida embaixo,
     riscadas.

### Estado

Store novo `src/stores/notesStore.ts` (Zustand), no padrao dos existentes: o
backend e a fonte da verdade; cada acao chama o comando e substitui o estado
local pelo retorno. Servico em `src/services/notes.ts`. Tipos `ProjectTodo` e o
campo `notes` de `Project` em `src/types/domain.ts`.

## Testes

**Vitest** — a regra de ordenacao/agrupamento do Painel vai para uma funcao pura
em `src/lib/todos.ts` (como `duration.ts` ja faz), e e testada isolada:

1. Filtra as concluidas (`done = 1` nao aparece).
2. Agrupa por projeto.
3. Projeto do cronometro ativo vem primeiro.
4. Sem cronometro ativo, a ordem e estavel (por nome de projeto).
5. Lista vazia -> resultado vazio (sem quebrar).

**Rust (`cargo test`)** — teste de repositorio: criar -> marcar como feita ->
listar -> excluir; e `ON DELETE CASCADE` (apagar o projeto leva as pendencias).

## Arquivos afetados

Novos:
- `src-tauri/migrations/0005_project_notes.sql`
- `src-tauri/src/repository/notes.rs`
- `src/services/notes.ts`
- `src/stores/notesStore.ts`
- `src/lib/todos.ts` + `src/lib/todos.test.ts`
- `src/features/projects/ProjectNotesModal.tsx`
- `src/features/dashboard/TodosPanel.tsx`

Editados:
- `src-tauri/src/database/mod.rs` (registrar migration 5)
- `src-tauri/src/commands/mod.rs` (comandos novos)
- `src-tauri/src/repository/mod.rs` (expor `notes`)
- `src-tauri/src/lib.rs` (registrar handlers)
- `src/types/domain.ts`
- `src/features/projects/ProjectsPage.tsx` (icone + modal)
- `src/features/dashboard/DashboardPage.tsx` (painel novo)

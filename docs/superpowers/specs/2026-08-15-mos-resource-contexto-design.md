# Spec — Resource ganha contexto, Fase 3

Data: 2026-08-15
Origem: `ROADMAP.md` §9.1, primeira lacuna real da Fase 3.
Escopo: `Resource ↔ Workspace`. Sem `Resource ↔ Project`, sem Recents, sem Favorites.

---

## 0. Por que esta e nao outra

A Fase 3 tem quatro itens. Este é o primeiro por um motivo que não veio do documento: o
buraco apareceu sozinho, duas vezes, em dois dias de trabalho.

1. O spec da Etapa 2 da Home (`2026-08-15-mos-home-widgets-por-workspace-design.md`) não
   pôde escopar Resources por Workspace — não há vínculo.
2. O widget `RECURSOS`, commitado hoje, mostra a Library inteira em qualquer contexto, e o
   commit teve que registrar isso como limitação assumida.

`ROADMAP.md` §9.1 pede a cadeia `Task → Project → Workspace → Resource → App`. Existem
`Task→Project`, `Project→Workspace` e `App→Workspace`. **Resource não se liga a nada** — é o
único elo ausente da cadeia, e o único que o uso já cobrou.

A ordem da §2 do Roadmap também aponta para cá: captura, confiança, recuperação, **contexto**.
As três primeiras estão de pé.

---

## 1. A relação

`Resource ↔ Workspace`, N-para-N. Não `Resource ↔ Project`.

Uma referência como `motion.dev` pertence a Web Design como um todo, não a um projeto
específico; amarrá-la a um Project exigiria escolher um, e a escolha seria falsa na maioria
dos casos. Workspace é a lente de contexto do produto, e é a lente que a Home e a Library
já usam.

N-para-N porque uma referência pode servir a dois contextos. Forçar um só seria uma decisão
que o produto não precisa tomar, e uma coluna em `resources` que um dia teria de virar
tabela mesmo assim.

`Resource ↔ Project` fica **fora**. É a outra metade da §9.1, e vem junto com a §9.2 (Project
como centro de contexto), que é outra decisão e outro ciclo.

---

## 2. Modelo de dados

Migration `0009_resource_workspaces.sql`, cópia estrutural de `app_workspaces`
(`0004_workspaces.sql`):

```sql
CREATE TABLE resource_workspaces (
    resource_id TEXT NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (resource_id, workspace_id)
) STRICT;

CREATE INDEX resource_workspaces_workspace_order
    ON resource_workspaces(workspace_id, created_at DESC);
```

`STRICT` e `PRAGMA user_version = 9` no fim. A cadeia em `mos-storage-sqlite/src/lib.rs`
ganha o degrau `if current <= 8`, e `SCHEMA_VERSION` passa a 9.

Nenhuma coluna nova em `resources`. As duas cascatas são declarativas e simétricas: apagar
o Resource ou apagar o Workspace leva o vínculo junto.

---

## 3. Caminho pela stack

Espelha `set_app_workspace` camada por camada. Nenhum padrão novo.

| Camada | O que entra | Modelo |
|---|---|---|
| `mos-core/src/ports.rs` | `set_resource_workspace(ResourceId, WorkspaceId, bool)` e `resource_workspaces()` no trait **`ResourceRepository`** (`ports.rs:152`), não em `WorkRepository` | `ports.rs:76` |
| `mos-storage-sqlite/src/resource_repository.rs` | `INSERT OR IGNORE` / `DELETE`, e o `SELECT` de todos os pares | `work_repository.rs:349` |
| `mos-core/src/service.rs` | wrapper em `MemoryService` (`service.rs:257`), que é quem carrega o `ResourceRepository`, com `ResourceId::parse` e `WorkspaceId::parse` | `service.rs:531` |
| `src-tauri/src/lib.rs` | dois comandos, com `notify_data_changed` e `schedule_snapshot` | `lib.rs:408` |
| `apps/desktop/src/api.ts` | `setResourceWorkspace(...)`, `resourceWorkspaces()` | `api.ts:153` |
| `apps/desktop/src/types.ts` | `ResourceWorkspace = { resourceId: string; workspaceId: string }` | — |

### 3.1 Todos os pares numa chamada

`resource_workspaces()` devolve **todos** os pares e entra no `Promise.all` do `refresh`, como
`hidden_widgets()` da etapa anterior.

O motivo é o mesmo, e é de interação: o filtro da Library tem que responder no instante em
que o contexto muda. Uma consulta por Workspace faria cada troca de contexto ir ao core, e a
troca deixaria de ser instantânea. São pares de id — mesmo um acervo grande cabe.

---

## 4. O contexto ativo sai da Home

Esta é a mudança de maior alcance do ciclo, e é inevitável.

Hoje `currentWorkspaceId` é estado **local do `HomePage`** (`App.tsx:224`), e a Library lê o
nome direto do `localStorage` (`App.tsx:714`) para escrever o segmento do caminho. Funciona
para desenhar texto, e só: não re-renderiza quando o contexto muda. No instante em que a
Library passa a filtrar por contexto, isso vira bug.

O estado sobe para o componente raiz, mantendo a persistência em `localStorage` que já
existe. A Home continua dona do seletor `CONTEXTO`; a Library passa a consumir o mesmo
estado. Nenhuma outra página muda.

---

## 5. Interface

### 5.1 A Library filtra

O escape é um par de rótulos no `filter-bar` que já existe (`App.tsx:838`) —
`NESTE CONTEXTO` e `TUDO` —, no mesmo formato dos filtros de tipo e do `GRID/LISTA`. Sem CSS
novo e sem seletor de Workspace duplicado: **trocar** de contexto continua sendo coisa da
Home; a Library só decide se aplica ou não o contexto vigente.

Sem Workspace ativo o par não aparece: não há contexto a aplicar, e um botão que não muda
nada é pior que botão nenhum.

### 5.2 O caminho para de mentir

`M / WEB DESIGN / LIBRARY` passa a aparecer **somente** quando o filtro está de fato
aplicado. Em `TUDO`, volta a ser `M / LIBRARY`.

Hoje o caminho anuncia um recorte que a lista não cumpre. É inofensivo enquanto vínculo não
existe; assim que existir, vira mentira visível.

### 5.3 O vazio do dia da migration

Filtro ativo, acervo cheio, nada neste contexto: `ScopedEmptyState` (`App.tsx:142`), o mesmo
componente que PROJECTS e APPS já usam — "12 resources salvos, nenhum em Web Design", com o
botão `Vincular`.

Isso é o que faz o dia seguinte à migration se explicar sozinho. Nenhum Resource nasce
vinculado, então **na primeira abertura todo Workspace mostra esse estado**. A mensagem
precisa dizer que o acervo está intacto e o que fazer — não parecer perda de dados.

### 5.4 Onde se vincula

No detalhe do Resource, na Library, logo abaixo do `POR QUÊ?` (`App.tsx:932`): um bloco
`CONTEXTO` com caixas de marcar para os Workspaces ativos.

As duas perguntas se leem juntas — por que guardei isto, e a que contexto pertence.

Não há painel espelho na página Workspaces, diferente de Projects e Apps. Motivo: Resources
são muito mais numerosos, e uma lista de caixas que cresce com o acervo inteiro seria pior
que não vincular. Vincula-se um por vez, olhando a referência, no momento em que a decisão
faz sentido.

### 5.5 O widget da Home ganha escopo

`RECURSOS` passa a filtrar pelo Workspace ativo e a usar `ScopedEmptyState` como PROJECTS e
APPS. Some a inconsistência que o commit de hoje registrou como limitação assumida.

---

## 6. Verificação

A camada Rust é escrita com teste antes: vínculo, desvínculo, idempotência da chamada
repetida, isolamento entre Workspaces, e as duas cascatas — apagar o Resource e apagar o
Workspace.

O front continua sem infraestrutura de teste (`package.json` define apenas
`"build": "tsc && vite build"`). Verificação é build mais inspeção, com um roteiro que
inclui explicitamente o estado do primeiro dia: abrir a Library com zero vínculos e conferir
que ela explica em vez de parecer vazia.

---

## 7. Fora de escopo

- `Resource ↔ Project` e a página de Project como centro de contexto (§9.2).
- Recents, Favorites e frequência (§9.4).
- Herdar o Workspace ativo ao salvar um Resource novo. Captura rápida acontece no contexto
  errado com frequência, e vínculo que o usuário não pediu é vínculo que ele terá de desfazer.
- Vincular em lote pela página Workspaces.

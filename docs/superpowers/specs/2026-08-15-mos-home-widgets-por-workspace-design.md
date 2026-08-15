# Spec — Widgets da Home por Workspace, Etapa 2

Data: 2026-08-15
Origem: `docs/superpowers/specs/2026-08-14-mos-home-grid-design.md`, seção 5, que deixou a
Etapa 2 fora de escopo.
Escopo: escolher **quais** widgets aparecem em cada Workspace. Sem arrastar, sem
redimensionar, sem ordem salva.

---

## 0. Decisão de escopo

O desenho `1D` prometia modo de edição completo: ligar, desligar, arrastar, redimensionar,
tudo salvo por Workspace. Esta etapa entrega só a primeira parte, por um motivo de produto.

`UX-PRINCIPLES.md` §3 diz que o M/OS deve exigir menos organização mental do que remove.
Arranjo livre inverte isso: transfere para o usuário o trabalho de diagramar uma tela que
hoje o desenho já resolveu. Escolher o que ver não tem esse custo — é uma decisão por
Workspace, tomada uma vez, que **reduz** o que a tela pede de atenção.

Arrastar continua possível depois. A Etapa 1 separou posição de conteúdo justamente para
isso: o `Widget` (`App.tsx:86`) só posiciona, e o `Panel` cuida de moldura e rótulo. Nada
aqui fecha aquela porta.

---

## 1. Onde a escolha mora

No banco, junto do Workspace. Não em `localStorage`.

O `localStorage` já guarda tema (`App.tsx:1359`) e Workspace atual (`App.tsx:206`), e seria
o caminho barato — nenhuma migration, nenhum comando novo. Foi recusado porque a natureza do
dado é outra. Tema é preferência da máquina; **quais widgets pertencem a um Workspace é uma
propriedade do Workspace**, como os Projects e os Apps vinculados a ele. Consequências
concretas de guardar no banco:

- entra no backup e no snapshot automático;
- sai no export de `DADOS E PORTABILIDADE`;
- acompanha o Workspace quando o companion iOS da ADR-003 existir;
- morre junto com o Workspace, via `ON DELETE CASCADE`.

Guardado em `localStorage`, nenhuma das quatro valeria, e nada na tela avisaria o usuário
disso — uma configuração que some ao restaurar backup é exatamente o tipo de surpresa que o
widget `SISTEMA` da Etapa 1 existe para tornar impossível.

---

## 2. Modelo de dados

Migration `0008_workspace_widgets.sql`, tabela única:

```sql
CREATE TABLE workspace_hidden_widgets (
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    widget_id TEXT NOT NULL CHECK (widget_id GLOB '[a-z][a-z0-9_]*'),
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, widget_id)
) STRICT;
```

`STRICT` e `PRAGMA user_version = 8` no fim, seguindo `0004_workspaces.sql` e
`0007_v03_design.sql`. A cadeia em `mos-storage-sqlite/src/lib.rs:186` ganha o degrau
`if current <= 7`, e `SCHEMA_VERSION` (`lib.rs:18`) passa a 8.

### 2.1 A linha significa oculto

Ausência de linha = widget aparece. Três consequências, todas desejadas:

1. Workspace novo mostra tudo sem nenhuma escrita.
2. Widget que eu criar depois nasce **visível** em todos os Workspaces. O inverso — guardar
   o que é visível — faria cada recurso novo nascer invisível para quem já usa o produto.
3. A tabela fica vazia para quem nunca configurou nada, que é o estado da maioria.

### 2.2 `widget_id` é string opaca

O core não conhece o catálogo de widgets. Quem conhece é o front, em `HOME_WIDGETS`.

Se o `widget_id` fosse enum validado no core, cada widget novo viraria mudança em Rust mais
migration — acoplamento caro para nenhum ganho, porque o core não faz nada com esse valor
além de guardar e devolver. O `CHECK` garante formato (minúscula, dígito e `_`), não
vocabulário.

**Regra que decorre disso, e que precisa ser respeitada para sempre:** o `id` de um widget é
permanente. Renomear `inbox_pulse` para outra coisa apaga em silêncio a escolha de quem o
tinha ocultado, porque a linha no banco deixa de casar com qualquer widget. O rótulo exibido
pode mudar à vontade; o id, não. Linha órfã de widget que deixou de existir é inofensiva: o
front ignora ids que não estão no catálogo.

### 2.3 Restaurar backup antigo

Um backup gerado na v7 restaurado num app v8 percorre a migration normal, com o snapshot
pré-migration que `lib.rs:154` já cria. Esse backup não tem configuração de widgets, então o
Workspace restaurado mostra os sete — o padrão. Nenhum tratamento especial é necessário.

---

## 3. Caminho pela stack

Espelha `set_app_workspace` camada por camada. Nenhum padrão novo é inventado.

| Camada | O que entra | Modelo a seguir |
|---|---|---|
| `mos-core/src/ports.rs` | `set_workspace_widget(WorkspaceId, &str, bool)`, `hidden_widgets()` | `ports.rs:76` |
| `mos-storage-sqlite/src/work_repository.rs` | `INSERT OR IGNORE` / `DELETE`, e um `SELECT` de todos os pares | `work_repository.rs:349` |
| `mos-core/src/service.rs` | wrapper com `WorkspaceId::parse` | `service.rs:531` |
| `src-tauri/src/lib.rs` | dois comandos, com `notify_data_changed` e `schedule_snapshot` | `lib.rs:408` |
| `apps/desktop/src/api.ts` | `setWorkspaceWidget(widgetId, workspaceId, visible)`, `hiddenWidgets()` | `api.ts:153` |
| `apps/desktop/src/types.ts` | `HiddenWidget = { workspaceId: string; widgetId: string }` | — |

Atenção à inversão de sinal: a API do front fala `visible`, como o resto da interface, e a
tabela guarda o oposto. A conversão acontece num lugar só — o comando Tauri —, nunca
espalhada pelos componentes.

### 3.1 Uma chamada, não uma por Workspace

`hidden_widgets()` devolve **todos** os pares de uma vez e entra no `Promise.all` do
`refresh` que já existe (`App.tsx:1313`). No teto absoluto são 7 linhas por Workspace.

Isso importa para a interação: trocar de Workspace no CONTEXTO filtra um `Map` em memória e
não vai ao backend. Se a busca fosse por Workspace, cada clique no CONTEXTO viraria uma ida
ao core, e a troca de contexto — que hoje é instantânea — passaria a piscar.

---

## 4. Interface

### 4.1 Onde se configura

Painel `WIDGETS` na página Workspaces, ao lado de `PROJECTS` e `APPS`. Mesma lista de
liga/desliga, mesma mensagem de confirmação dos `toggleProject` e `toggleApp`
(`App.tsx:493-509`).

Configurar a Home longe da Home é o preço, e é aceitável: a página Workspaces já é o lugar
onde se declara o que pertence a um Workspace. Widget entra na mesma frase que Project e App.

O catálogo vive no front, em `const HOME_WIDGETS = [{ id, label }]` — fonte de verdade única
dos ids, consumida pela Home e pelo painel.

### 4.2 Como a Home aplica

`Widget` ganha a prop `id` e devolve `null` quando oculto. A regra fica num lugar só, e a
grade não muda: os widgets restantes reflowam sozinhos, que é o que a grade de 12 colunas da
Etapa 1 já faz.

Sem Workspace selecionado — o `Todos` do CONTEXTO — nada é ocultado. `Todos` é a visão sem
filtro, e sem Workspace não há escolha a aplicar. Decisão deliberada: evita uma linha sem
`workspace_id` no schema e a pergunta de herança que viria junto.

### 4.3 O estado vazio novo

Ocultar os sete deixa a Home só com Capture e CONTEXTO. Em vez de um branco inexplicável,
uma mensagem dizendo que os widgets deste Workspace estão ocultos, com link para a página
Workspaces — o mesmo padrão do `ScopedEmptyState` estabelecido no ciclo anterior.

---

## 5. Verificação

Diferente da Etapa 1, metade desta etapa **tem** teste: são 58 testes nos crates, e
`work_repository.rs` já tem 4. A camada Rust é escrita com teste antes:

- `set_workspace_widget` esconde, revela e é idempotente na chamada repetida;
- apagar o Workspace leva as linhas junto (`ON DELETE CASCADE` exige `foreign_keys=ON`, que
  `configure_connection` já garante em `lib.rs:103`);
- a migration sobe de 7 para 8 preservando os dados, e o banco criado do zero chega ao mesmo
  schema — os dois caminhos que o teste de `lib.rs:397` já cobre para as migrations
  anteriores.

O front continua sem infraestrutura de teste: `package.json` define apenas
`"build": "tsc && vite build"`. Verificação é build mais inspeção com um Workspace real.

---

## 6. Fora de escopo

- Arrastar, redimensionar e ordem salva. Continuam adiados, agora com o dado que faltava:
  esta etapa prova se escolher o que ver já resolve o problema antes de pagar por arranjo.
- Configuração para o estado `Todos`.
- Ocultar `Capture` e `CONTEXTO` — são estrutura da página, não widgets da grade.
- Qualquer widget novo do catálogo de `IDEAS.md`. Esta etapa mexe em visibilidade, não em
  conteúdo.

# Spec — Home em grade, Etapa 1

Data: 2026-08-14
Arranjo escolhido: `1D` Grade editável, do desenho `M-OS Ideas - Hermes e Home v0.1`.
Escopo: apenas a grade e os widgets com fonte de dados existente. Sem modo de edição.

---

## 0. Decisão de arranjo

O desenho deixou três arranjos em aberto e pedia para escolher um. A escolha é `1D`, por
dois motivos:

1. `1B` e `1C` pressupõem agendamento. `1B` tem `TODAY` e `CALENDAR` como metade do
   conteúdo; `1C` é construído inteiro em torno de uma coluna de horas. A `Task` do M/OS
   não tem data — isso é Fase 4 do `ROADMAP.md`, não construída. Hoje o `1C` seria uma
   coluna de horas vazia.
2. `1D` é um contêiner: acomoda os widgets que existirem, e é o único que comporta os 25
   do catálogo (`IDEAS.md`, entradas 151 a 175).

O `1D` é também o mais caro, porque inclui modo de edição e arranjo salvo por Workspace.
Mas a grade e a edição são separáveis, e o mesmo sistema de posicionamento serve às duas.
Esta spec cobre **apenas a Etapa 1**: a grade com arranjo fixo.

---

## 1. A grade

Doze colunas. O desenho especifica tamanhos em células de widget — `1×1`, `2×1`, `2×2`,
largura total — que não são colunas. A conversão adotada é **1 célula = 3 colunas**,
resultando em quatro células por linha:

| Tamanho no desenho | Colunas | Linhas |
|---|---|---|
| `1×1` | 3 | 1 |
| `2×1` | 6 | 1 |
| `2×2` | 6 | 2 |
| largura total | 12 | 1 |

Nos breakpoints que já existem (`App.css:2242` para 1280px e `App.css:2254` para 960px) a
largura da célula muda, não o número de células por widget. **Nenhum breakpoint novo.**

| Largura | Colunas da grade | 1 célula | Células por linha |
|---|---|---|---|
| ≥ 1280px | 12 | 3 colunas | 4 |
| 960–1280px | 8 | 4 colunas | 2 |
| < 960px | 4 | 4 colunas | 1 (empilha) |

Um widget `2×1` ocupa 6 colunas no primeiro caso, 8 no segundo e 4 no terceiro — ou seja,
metade da linha, linha inteira, linha inteira. Span nunca excede as colunas disponíveis.

A Home deixou de ter teto de largura no ciclo anterior, então a grade cresce com a janela.

---

## 2. Widgets da Etapa 1

| Widget | Origem | Tamanho | Fonte |
|---|---|---|---|
| Capture | já existe na Home | total | core |
| Now | painel `EM ANDAMENTO` | 2×1 | tasks em Doing |
| Recently Captured | painel `RECENTES` | 2×1 | captures |
| Active Projects | painel `PROJECTS` | 2×2 | projects + tasks |
| Quick Apps | painel `APPS` | 2×1 | App Registry |
| Inbox Pulse | novo | 1×1 | captures |
| System Health | novo | 1×1 | AppStatus + HermesStatus |
| Quick Actions | novo | 1×1 | nenhuma |

Quatro dos oito já existem como painéis e só mudam de casa. Nenhum widget desta etapa
exige mudança em core, storage ou API.

`CONTEXTO` fica **fora** da grade. No `1D` é ele que determina qual arranjo carregar, então
é estrutura, não widget. Continua acima da grade, como hoje.

### 2.1 Now sem cronômetro

Mostra o projeto e a task em `Doing`. **Não** mostra tempo corrido nem botões de
Start/Stop.

O catálogo (`IDEAS.md` 151) e o desenho declaram a `Fonte` do `Now` como ChronoCAD, que é
Fase 8. Construir rastreio de tempo nativo aqui criaria uma segunda fonte de verdade sobre
horas trabalhadas — exatamente o que o `ROADMAP.md` §18.2 proíbe para o M-Finance.

### 2.2 Inbox Pulse

Contagem de captures por processar, mais quantas passaram de três dias, derivada de
`capturedAt`. Clicar abre a Inbox.

O limiar de três dias vem do texto do catálogo (`IDEAS.md` 155) e não é configurável nesta
etapa.

### 2.3 System Health

Três verificações, todas com fonte existente:

| Linha | Fonte | Verificação |
|---|---|---|
| Dados salvos | `AppStatus.storage.integrity` | igual a `"ok"` |
| Backup | `AppStatus.snapshot` | alimentado por `ensure_daily_snapshot()` (`crates/mos-core/src/service.rs:122`) |
| Hermes | `HermesConnectionState` (`apps/desktop/src/hermes.ts:14`) | `"offline"`, `"connecting"` ou `"online"` |

O estado do Hermes já é assinável por `onState` (`hermes.ts:81`), então o widget acompanha
mudanças sem polling.

**Fiação necessária:** `HomePage` hoje não recebe nenhum dos dois. Suas props
(`App.tsx:157`) são `recent`, `projects`, `tasks`, `workspaces`, `apps`, `refresh` e os
`open*`. O `AppStatus` já é buscado no componente raiz e passado ao `SettingsPage`; basta
passá-lo também ao `HomePage`. O estado do Hermes é assinado dentro do próprio widget, via
`onState`, sem subir para o pai.

Isso é mudança em `App.tsx`, que está no escopo. Continua valendo que **nada muda** em
`crates/`, `src-tauri/`, `api.ts` ou `types.ts`: os dois dados já existem e já trafegam.

Discrição é requisito, não estética: quando tudo está bem, este é o elemento mais silencioso
da tela. Só ganha peso visual quando algo falha.

### 2.4 Quick Actions

Três ações na Home: `Nova Task`, `Capturar`, `Novo Project`. Todas já existem como fluxos.

A versão contextual descrita no catálogo (`IDEAS.md` 175) — ações que mudam conforme o
Project aberto — depende de a Home saber em que contexto está, o que só a Etapa 2 resolve.
Fora de escopo aqui.

---

## 3. Componente

Um componente `Widget` que envolve o `Panel` existente e cuida **somente** do
posicionamento na grade, recebendo o tamanho como prop.

O `Panel` continua responsável pelo rótulo e pela moldura, sem alteração. A separação
importa: na Etapa 2, o modo de edição muda o posicionamento sem tocar em nenhum widget.

---

## 4. Verificação

Não existe infraestrutura de teste de front — `apps/desktop/package.json` define apenas
`"build": "tsc && vite build"`. Esta spec não promete teste automatizado.

Verificação é `npm run build` mais inspeção nas três larguras de sempre: 840px (mínimo da
janela), 1180px (padrão) e a largura máxima do monitor.

Vantagem desta etapa: **os oito widgets têm dado real no banco de referência** — 5 captures,
6 tasks, 2 projects, 5 apps. Nenhum aparece vazio, com uma exceção legítima: `Quick Apps`
fica vazio se nenhum app estiver vinculado ao Workspace ativo, e nesse caso exibe a
mensagem correta, corrigida no ciclo anterior.

---

## 5. Fora de escopo

- **Etapa 2 inteira:** modo de edição, arrastar, arranjo salvo por Workspace. Esta última
  exige armazenamento novo e é o que torna o `1D` caro.
- Cronômetro do `Now`, `Today`, `Calendar`, `Library recent`, `Design Inspiration`, e os
  demais widgets do catálogo que dependem de ChronoCAD, GitHub, M-Finance, Hermes ou
  agendamento.
- Qualquer mudança em `crates/`, `src-tauri/`, `api.ts` ou `types.ts`.

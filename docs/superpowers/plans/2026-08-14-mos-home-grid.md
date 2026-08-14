# Home em Grade, Etapa 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Trocar os quatro painéis fixos da Home por uma grade de 12 colunas com widgets dimensionados, e acrescentar três widgets novos com dado que já existe.

**Architecture:** Um componente `Widget` cuida somente do posicionamento na grade, declarando o tamanho por atributo; o `Panel` existente continua responsável pelo rótulo e pela moldura, sem alteração. Os quatro painéis atuais são realojados sem mudar seu conteúdo. Os três widgets novos consomem dados que já trafegam no componente raiz e só precisam ser passados adiante.

**Tech Stack:** React 19, TypeScript, CSS Grid com custom properties. Sem dependência nova.

## Global Constraints

- **Escopo é `App.tsx` e `App.css`.** Nada em `crates/`, `src-tauri/`, `api.ts` ou `types.ts`. Nenhuma migration, nenhuma query nova, nenhum comando Tauri novo. Todos os dados usados aqui já existem e já trafegam.
- **Spec de origem:** `docs/superpowers/specs/2026-08-14-mos-home-grid-design.md`.
- **Não anotar tipo de retorno de componente.** Nenhum arquivo de `apps/desktop/src/` usa `JSX.Element`, e o React é 19, onde o namespace global `JSX` não é exposto por padrão. Anotar quebra o `tsc`.
- **Conversão de tamanho:** 1 célula do desenho = 3 colunas da grade. `1×1` = 3, `2×1` = 6, `2×2` = 6 colunas por 2 linhas, largura total = 12.
- **Nenhum breakpoint novo.** Usar somente 1280px e 960px, que já existem em `App.css`.
- **`recent` é lista limitada.** `service.rs:39` declara `recent(&self, limit: usize)`. Nunca usar `recent.length` como contagem da Inbox — para isso existe `inbox`, de `api.inbox()`.
- **O `Now` desta etapa não tem cronômetro.** Só projeto e task em `Doing`. Tempo é ChronoCAD, Fase 8. Não construir rastreio de tempo.
- **Não existe teste automatizado de front.** `package.json` define apenas `"build": "tsc && vite build"`. Não instalar dependência de teste. Verificação é `npm run build` mais inspeção no app.
- **Build do Rust exige ambiente portable.** Antes de `npm run tauri dev`, no PowerShell:
  ```powershell
  $env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
  $env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
  ```
  Se `tauri dev` reclamar de porta 1420 ocupada, matar o processo `node` que a segura — matar o wrapper do npm não basta.

---

### Task 1: A grade e a migração dos quatro painéis

Nenhum dado novo. O conteúdo é idêntico ao de hoje; muda só o posicionamento. Isso torna a revisão fácil: qualquer diferença de conteúdo é regressão.

**Files:**
- Modify: `apps/desktop/src/App.tsx:78` (inserir `Widget` após `EmptyState`)
- Modify: `apps/desktop/src/App.tsx:223-233` (bloco `.home-sections`)
- Modify: `apps/desktop/src/App.css` (regras novas de `.home-grid` e `.widget`; remover `.home-sections`)
- Test: nenhum — não existe infra de teste de front

**Interfaces:**
- Consumes: `Panel({ label, count, action, rule, children, className })` (`App.tsx:72`), o tipo `ReactNode` (já importado em `App.tsx:2`).
- Produces: `Widget({ size, children })` com `size: "1x1" | "2x1" | "2x2" | "full"` e `children: ReactNode`. As Tasks 2 e 3 usam este componente sem alterá-lo.

- [ ] **Step 1: Criar o componente `Widget`**

Em `apps/desktop/src/App.tsx`, logo após o bloco do `EmptyState` que termina na linha 78, inserir:

```tsx
/* Cuida so do posicionamento na grade. A moldura e o rotulo continuam no Panel, para
   que a etapa 2 (modo de edicao) mude posicao sem tocar em nenhum widget. */
function Widget({ size, children }: { size: "1x1" | "2x1" | "2x2" | "full"; children: ReactNode }) {
  return <div className="widget" data-size={size}>{children}</div>;
}
```

- [ ] **Step 2: Trocar o contêiner e envolver os quatro painéis**

Em `App.tsx:223`, trocar a abertura:

```tsx
    <div className="home-sections">
```

por:

```tsx
    <div className="home-grid">
```

Depois, envolver cada um dos quatro `<Panel>` que estão dentro desse bloco, **sem alterar uma vírgula do conteúdo interno**:

- `<Panel label="EM ANDAMENTO" ...>…</Panel>` → envolver em `<Widget size="2x1">…</Widget>`
- `<Panel label="RECENTES" ...>…</Panel>` → envolver em `<Widget size="2x1">…</Widget>`
- `<Panel label="PROJECTS">…</Panel>` → envolver em `<Widget size="2x2">…</Widget>`
- `<Panel label="APPS">…</Panel>` → envolver em `<Widget size="2x1">…</Widget>`

Os comentários JSX que existem entre os painéis permanecem onde estão.

**Os rótulos não mudam nesta task.** O spec mapeia `EM ANDAMENTO` para o widget `Now`, mas
renomear é decisão de texto de interface, não de layout, e misturá-la aqui estragaria a
propriedade que torna esta task fácil de revisar: conteúdo idêntico, só posição diferente.
`EM ANDAMENTO`, `RECENTES`, `PROJECTS` e `APPS` continuam com os rótulos de hoje.

- [ ] **Step 3: Trocar as regras de CSS**

Em `apps/desktop/src/App.css`, substituir o bloco `.home-sections` inteiro:

```css
/* auto-fit colapsa as trilhas vazias, entao os paineis dividem entre si toda a
   largura disponivel em vez de ficarem presos a duas colunas. */
.home-sections {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(var(--column-min), 1fr));
  gap: var(--space-4);
  padding-top: var(--space-5);
}
```

por:

```css
/* Grade de 12 colunas do arranjo 1D. Uma celula do desenho vale 3 colunas, entao
   cabem quatro celulas por linha. As media queries repetem as larguras que ja
   existem no arquivo (1280 e 960) — nenhum breakpoint novo — mas ficam aqui, junto
   das regras que governam, em vez de espalhadas nos blocos de padding. */
.home-grid {
  display: grid;
  grid-template-columns: repeat(12, minmax(0, 1fr));
  gap: var(--space-4);
  padding-top: var(--space-5);
}

.widget {
  min-width: 0;
}

.widget[data-size="1x1"] { grid-column: span 3; }
.widget[data-size="2x1"] { grid-column: span 6; }
.widget[data-size="2x2"] { grid-column: span 6; grid-row: span 2; }
.widget[data-size="full"] { grid-column: span 12; }

@media (max-width: 1280px) {
  .home-grid { grid-template-columns: repeat(8, minmax(0, 1fr)); }
  .widget[data-size="1x1"] { grid-column: span 4; }
  .widget[data-size="2x1"] { grid-column: span 8; }
  .widget[data-size="2x2"] { grid-column: span 8; }
  .widget[data-size="full"] { grid-column: span 8; }
}

@media (max-width: 960px) {
  .home-grid { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .widget[data-size="1x1"],
  .widget[data-size="2x1"],
  .widget[data-size="2x2"],
  .widget[data-size="full"] { grid-column: span 4; }
  .widget[data-size="2x2"] { grid-row: span 1; }
}
```

O `--column-min` deixa de ser usado aqui, mas **não deve ser removido** de `mos-tokens.css`: o `.context-switcher` continua consumindo.

- [ ] **Step 4: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro de TypeScript.

- [ ] **Step 5: Verificar no app**

Subir o app e conferir nas três larguras:

- **~2545px** — quatro widgets na grade. `EM ANDAMENTO`, `RECENTES` e `APPS` ocupam meia linha cada; `PROJECTS` ocupa meia linha e duas alturas.
- **1180px** — cada widget ocupa a linha inteira, empilhados.
- **840px** — igual ao anterior, sem overflow horizontal.

O conteúdo de cada painel deve ser **idêntico** ao de antes: mesmas listas, mesmas contagens, mesmos estados vazios. Qualquer diferença aqui é regressão, não melhoria.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(ui): Home em grade de 12 colunas com widgets dimensionados"
```

---

### Task 2: Inbox Pulse e Quick Actions

Dois widgets `1×1`. O `Inbox Pulse` exige passar `inbox` ao `HomePage` — o dado já existe no raiz, só não chega lá.

**Files:**
- Modify: `apps/desktop/src/App.tsx:170` (props de `HomePage`)
- Modify: `apps/desktop/src/App.tsx:1345` (chamada de `HomePage`)
- Modify: `apps/desktop/src/App.tsx` (bloco `.home-grid`, acrescentar dois `Widget`)
- Modify: `apps/desktop/src/App.css` (regras de `.pulse-count` e `.quick-actions`)
- Test: nenhum

**Interfaces:**
- Consumes: `Widget({ size, children })` da Task 1, com `size: "1x1" | "2x1" | "2x2" | "full"`. `Panel({ label, count, action, rule, children, className })`. `Button` com `variant` e `size` (`App.tsx:53`). O tipo `Capture` (já importado).
- Produces: nada consumido por tasks posteriores.

- [ ] **Step 1: Passar `inbox` ao `HomePage`**

Na assinatura em `App.tsx:170`, acrescentar `inbox` à desestruturação e ao tipo. A lista de props passa a começar por:

```tsx
function HomePage({ recent, inbox, projects, tasks, workspaces, apps, refresh, openCapture, openProject, openWorkspace, openTask, openApp, intent }: { recent: Capture[]; inbox: Capture[]; projects: Project[]; tasks: Task[]; workspaces: Workspace[]; apps: RegisteredApp[]; refresh: () => Promise<void>; openCapture: (capture: Capture) => void; openProject: (project: Project) => void; openWorkspace: (workspace: Workspace) => void; openTask: (task: Task) => void; openApp: (app: RegisteredApp) => void; intent?: FunctionIntent }) {
```

Na chamada em `App.tsx:1345`, acrescentar `inbox={inbox}` logo após `recent={recent}`. A variável `inbox` já existe no componente raiz e já está nas dependências do `useMemo` da linha 1353 — não é preciso alterar o array de dependências.

- [ ] **Step 2: Calcular a métrica do Inbox Pulse**

Dentro de `HomePage`, junto das outras derivações (perto de `const projectName = ...`), acrescentar:

```tsx
  // Tres dias e o limiar do catalogo (IDEAS.md 155). A conta usa `inbox`, nunca
  // `recent`, porque recent(limit) e truncado no core e mentiria a contagem.
  const staleInbox = inbox.filter((capture) => Date.now() - new Date(capture.capturedAt).getTime() > 3 * 24 * 60 * 60 * 1000).length;
```

- [ ] **Step 3: Acrescentar os dois widgets**

Dentro do `<div className="home-grid">`, após o widget de `APPS`, inserir:

```tsx
      <Widget size="1x1"><Panel label="INBOX"><button type="button" className="pulse" onClick={() => openInbox()}><strong className="pulse-count">{inbox.length}</strong><small>{inbox.length === 1 ? "capture por processar" : "captures por processar"}</small>{staleInbox ? <small className="pulse-stale">{staleInbox === 1 ? "1 com mais de 3 dias" : `${staleInbox} com mais de 3 dias`}</small> : null}</button></Panel></Widget>
      <Widget size="1x1"><Panel label="AÇÕES"><div className="quick-actions"><Button variant="outline" size="sm" onClick={() => void api.showQuickCapture()}>Capturar</Button><Button variant="outline" size="sm" onClick={() => openTasksPage()}>Nova Task</Button><Button variant="outline" size="sm" onClick={() => openProjectsPage()}>Novo Project</Button></div></Panel></Widget>
```

Isso exige três navegações que `HomePage` ainda não tem. Acrescentar às props, no mesmo formato do Step 1: `openInbox: () => void`, `openTasksPage: () => void`, `openProjectsPage: () => void`. Na chamada da linha 1345, ligar às trocas de página que o raiz já faz:

```tsx
openInbox={() => setPage("inbox")} openTasksPage={() => setPage("tasks")} openProjectsPage={() => setPage("projects")}
```

`api.showQuickCapture()` já é usado no rail (`App.tsx`, botão de Quick Capture) e não precisa de prop.

- [ ] **Step 4: Acrescentar as regras de CSS**

Em `apps/desktop/src/App.css`, após o bloco `.widget[data-size="full"]` da Task 1, inserir:

```css
.pulse {
  display: grid;
  justify-items: start;
  gap: var(--space-1);
  padding: 0;
  border: 0;
  background: none;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.pulse-count {
  font: var(--text-display);
  letter-spacing: var(--tracking-display);
  color: var(--text);
}

.pulse small {
  color: var(--text-secondary);
  font: var(--text-small);
}

.pulse-stale {
  color: var(--text-system);
}

.quick-actions {
  display: grid;
  justify-items: start;
  gap: var(--space-2);
  padding-top: var(--space-2);
}
```

Só tokens existentes, nenhum valor hardcoded.

- [ ] **Step 5: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 6: Verificar no app**

1. O widget `INBOX` mostra a contagem em número grande. Com o banco de referência são **5 captures**, e o texto deve estar no plural.
2. Clicar no número abre a página Inbox.
3. O widget `AÇÕES` mostra três botões. `Capturar` abre o Quick Capture; `Nova Task` vai para Tasks; `Novo Project` vai para Projects.
4. Se nenhuma capture tiver mais de três dias, a terceira linha do Inbox Pulse **não aparece** — é condicional, não um zero escrito.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(ui): widgets Inbox Pulse e Quick Actions na Home"
```

---

### Task 3: System Health

O widget que representa a promessa central do produto: dá para confiar que o que entrou está guardado. Três verificações, todas com fonte existente.

**Files:**
- Modify: `apps/desktop/src/App.tsx:78` (inserir o widget após `Widget`)
- Modify: `apps/desktop/src/App.tsx:170` (props de `HomePage`)
- Modify: `apps/desktop/src/App.tsx:1345` (chamada de `HomePage`)
- Modify: `apps/desktop/src/App.tsx` (bloco `.home-grid`)
- Modify: `apps/desktop/src/App.css` (regras de `.health-list`)
- Test: nenhum

**Interfaces:**
- Consumes: `Widget({ size, children })` da Task 1. `Panel(...)`. O tipo `AppStatus` (já importado em `App.tsx:12`). De `./hermes`: `hermes.status()`, `hermes.onState(handler)` e os tipos `HermesStatus` e `HermesConnectionState`.
- Produces: nada consumido por tasks posteriores.

- [ ] **Step 1: Criar o componente do widget**

Em `App.tsx`, após o componente `Widget` criado na Task 1, inserir:

```tsx
/* A promessa central do produto e confianca: o que entrou esta guardado. Quando
   tudo esta bem este e o elemento mais silencioso da tela; so ganha peso ao falhar. */
function SystemHealth({ status }: { status: AppStatus | null }) {
  const [hermesState, setHermesState] = useState<HermesConnectionState>("offline");
  useEffect(() => {
    void hermes.status().then((next) => setHermesState(next.state)).catch(() => undefined);
    const subscription = hermes.onState((next) => setHermesState(next.state));
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);
  const saved = status?.storage.integrity === "ok";
  return <dl className="health-list">
    <div><dt>Dados</dt><dd data-ok={saved || undefined}>{saved ? "Salvos" : status ? status.storage.integrity : "—"}</dd></div>
    <div><dt>Backup</dt><dd data-ok={status?.snapshot ? true : undefined}>{status?.snapshot || "—"}</dd></div>
    <div><dt>Hermes</dt><dd data-ok={hermesState === "online" || undefined}>{hermesState === "online" ? "Online" : hermesState === "connecting" ? "Conectando" : "Offline"}</dd></div>
  </dl>;
}
```

O padrão de limpeza da assinatura — `subscription.then((dispose) => dispose())` — é o mesmo já usado em `HermesPage.tsx:93`. `hermes.onState` devolve uma Promise de `UnlistenFn`, não a função direta.

**O import precisa mudar.** `App.tsx:8` hoje é:

```tsx
import { hermes, hermesUnavailableLabel, type HermesStatus } from "./hermes";
```

Falta `HermesConnectionState`, que o código acima usa. Trocar por:

```tsx
import { hermes, hermesUnavailableLabel, type HermesConnectionState, type HermesStatus } from "./hermes";
```

Não criar um segundo import de `./hermes`. `AppStatus` já vem de `./types` na linha 12 e não precisa de nada.

- [ ] **Step 2: Passar `status` ao `HomePage`**

Acrescentar `status` à desestruturação e ao tipo em `App.tsx:170`, com `status: AppStatus | null`. Na chamada da linha 1345, acrescentar `status={status}`.

A variável `status` já existe no raiz (`App.tsx:1237`) e já é passada ao `SettingsPage` na linha 1352, então não é preciso buscá-la nem alterar o `useMemo`.

Atenção: existe **outro** `status` em `App.tsx:1100`, do tipo `HermesStatus`, dentro do componente do Hermes. Não confundir — o que interessa aqui é o da linha 1237, do tipo `AppStatus`.

- [ ] **Step 3: Acrescentar o widget à grade**

Dentro do `<div className="home-grid">`, após o widget de `AÇÕES`, inserir:

```tsx
      <Widget size="1x1"><Panel label="SISTEMA"><SystemHealth status={status} /></Panel></Widget>
```

- [ ] **Step 4: Acrescentar as regras de CSS**

Em `apps/desktop/src/App.css`, após o bloco `.quick-actions` da Task 2, inserir:

```css
.health-list {
  display: grid;
  gap: var(--space-2);
  margin: var(--space-2) 0 0;
}

.health-list div {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: baseline;
  gap: var(--space-2);
}

.health-list dt {
  color: var(--text-system);
  font: var(--text-meta);
  letter-spacing: var(--tracking-meta);
  text-transform: uppercase;
}

.health-list dd {
  margin: 0;
  color: var(--text-secondary);
  font: var(--text-small);
}

.health-list dd[data-ok] {
  color: var(--success);
}
```

- [ ] **Step 5: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 6: Verificar no app**

1. O widget `SISTEMA` mostra três linhas: `Dados`, `Backup` e `Hermes`.
2. `Dados` deve dizer **Salvos**, em verde — o banco de referência está íntegro.
3. `Backup` mostra o texto de `AppStatus.snapshot`, o mesmo que o painel de INTEGRIDADE já exibe em Settings. Comparar os dois: devem coincidir.
4. `Hermes` mostra `Offline`, `Conectando` ou `Online` conforme o estado da ponte. Se o túnel não estiver aberto, `Offline` é o resultado correto, não um bug.
5. Deixar a Home aberta e mudar o estado do Hermes: a linha deve acompanhar sozinha, sem recarregar a página — é isso que a assinatura `onState` compra.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(ui): widget System Health na Home"
```

---

## Limite conhecido

Ao fim das três tarefas a Home terá sete widgets mais o Capture. Nos ~2545px isso ainda deixa espaço, porque sete widgets pequenos não preenchem uma grade de 12 colunas em muitas linhas.

Isso é esperado. A densidade plena depende dos widgets que a Etapa 1 não pode construir — `Today`, `Calendar`, `Now` com cronômetro, `Library recent` — todos bloqueados por agendamento ou por integração inexistente. Não compensar inventando conteúdo nem inflando tamanhos.

# Estado Vazio Honesto nos Painéis com Escopo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer os painéis PROJECTS e APPS da Home distinguirem "nada cadastrado" de "nada vinculado ao Workspace ativo", e oferecer a ação de vincular.

**Architecture:** Um componente novo, `ScopedEmptyState`, concentra a lógica de três estados e é consumido pelos dois painéis. O dado necessário já chega em `HomePage` por prop — `apps` e `projects` completos, além dos recortes `workspaceApps` e `workspaceProjects`. A ação reusa `openWorkspace`, que já é prop, e a tela de vínculo que já existe em `WorkspacesPage`.

**Tech Stack:** React 19, TypeScript, CSS puro com custom properties. Sem dependência nova.

## Global Constraints

- **Escopo é `App.tsx` e `App.css`.** Nenhuma mudança em `crates/`, `src-tauri/`, `api.ts` ou `types.ts`. Nenhuma migration, nenhuma chamada de API nova, nenhuma query nova. Se um passo parecer exigir isso, o plano está errado — pare e reporte.
- **Spec de origem:** `docs/superpowers/specs/2026-08-14-mos-scoped-empty-state-design.md`.
- **Tipo do Workspace é `Workspace | null`, nunca `undefined`.** `App.tsx:162` termina em `?? null`. O spec diz `undefined` num trecho; o spec está errado nesse detalhe e o correto é `null`.
- **Não anotar tipo de retorno de componente.** Nenhum dos cinco arquivos de `apps/desktop/src/` usa `JSX.Element`, e o React é 19, onde o namespace global `JSX` não é exposto por padrão. Anotar quebra o `tsc`.
- **Contagem só de itens ativos:** `lifecycleState === "active"`. Item arquivado não entra no total.
- **Texto exato, com plural.** Não improvisar redação:
  - `Apps cadastrados aparecerão aqui.` (total 0)
  - `1 app cadastrado, nenhum em {workspace}.`
  - `{total} apps cadastrados, nenhum em {workspace}.`
  - `Projects criados aparecerão aqui.` (total 0)
  - `1 Project criado, nenhum em {workspace}.`
  - `{total} Projects criados, nenhum em {workspace}.`
  - Rótulo da ação: `Vincular`
- **Não existe teste automatizado de front.** `apps/desktop/package.json` define apenas `"build": "tsc && vite build"` — sem vitest, sem playwright. Não instale dependência de teste. Verificação é `npm run build` mais inspeção no app.
- **O banco de referência já reproduz o bug:** Workspace "Testes" ativo, 5 registros em `apps`, 0 em `app_workspaces`. Nenhuma preparação de dado é necessária.
- **Build do Rust exige ambiente portable.** Antes de `npm run tauri dev`, no PowerShell:
  ```powershell
  $env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
  $env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
  ```
  Se já houver um vite na porta 1420, `tauri dev` falha no `beforeDevCommand` — mate o processo do vite antes, não só o wrapper do npm.

---

### Task 1: Componente `ScopedEmptyState` e painel APPS

Primeira porque é o painel que manifesta o bug hoje: com os dados reais, dá para ver a mensagem errada antes e a certa depois.

**Files:**
- Modify: `apps/desktop/src/App.tsx:78` (inserir o componente logo após `EmptyState`)
- Modify: `apps/desktop/src/App.tsx:219` (painel APPS)
- Modify: `apps/desktop/src/App.css` (uma regra nova, `.scoped-empty`)
- Test: nenhum — não existe infra de teste de front (ver Global Constraints)

**Interfaces:**
- Consumes: `EmptyState({ children })` (`App.tsx:76`), `Button` com `variant` e `size` (`App.tsx:53`), o tipo `Workspace` (importado de `./types`), e as props de `HomePage`: `apps`, `openWorkspace`, além de `currentWorkspace` (`App.tsx:162`) e `activeApps` (`App.tsx:189`).
- Produces: `ScopedEmptyState({ total, workspace, noun, onLink })` com `total: number`, `workspace: Workspace | null`, `noun: "app" | "project"`, `onLink: () => void`. A Task 2 consome este mesmo componente sem alterá-lo.

- [ ] **Step 1: Ver o bug antes de mexer**

Suba o app:

```powershell
$env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
$env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
cd C:\Dev\pessoal\m-os\apps\desktop
npm run tauri dev
```

Na Home, com o Workspace "Testes" selecionado no painel CONTEXTO, o painel APPS mostra:

> Apps cadastrados aparecerão aqui.

Isso é falso — existem 5 apps. Confirme o número sem alterar nada:

```bash
"/c/Users/matheus.mendes/AppData/Local/hermes/hermes-agent/venv/Scripts/python" -c "
import sqlite3
p='C:/Users/matheus.mendes/AppData/Roaming/com.codedbym.mos/m-os.db'
c=sqlite3.connect('file:'+p+'?mode=ro',uri=True)
print('apps:', c.execute('select count(*) from apps').fetchone()[0])
print('app_workspaces:', c.execute('select count(*) from app_workspaces').fetchone()[0])
c.close()"
```

Esperado: `apps: 5` e `app_workspaces: 0`.

- [ ] **Step 2: Criar o componente**

Em `apps/desktop/src/App.tsx`, imediatamente após o bloco do `EmptyState` que termina na linha 78, inserir:

```tsx
/* O vazio de um painel com escopo tem duas causas que a mensagem antiga confundia:
   nada cadastrado, ou nada vinculado ao Workspace ativo. Sem separar as duas, a Home
   afirma que o usuario nao tem apps enquanto esconde os que ele tem. */
function ScopedEmptyState({ total, workspace, noun, onLink }: { total: number; workspace: Workspace | null; noun: "app" | "project"; onLink: () => void }) {
  if (total === 0 || !workspace) {
    return <EmptyState>{noun === "app" ? "Apps cadastrados aparecerão aqui." : "Projects criados aparecerão aqui."}</EmptyState>;
  }
  const counted = noun === "app"
    ? `${total} ${total === 1 ? "app cadastrado" : "apps cadastrados"}`
    : `${total} ${total === 1 ? "Project criado" : "Projects criados"}`;
  return <div className="scoped-empty"><EmptyState>{`${counted}, nenhum em ${workspace.name}.`}</EmptyState><Button variant="outline" size="sm" onClick={onLink}>Vincular</Button></div>;
}
```

O ramo `!workspace` cai no texto genérico porque, sem Workspace ativo, o escopo é o conjunto inteiro — se está vazio, é porque realmente não há nada cadastrado.

O tipo `Workspace` já está importado em `App.tsx:12`, junto com os demais tipos vindos de
`./types`. Não adicione import.

- [ ] **Step 3: Trocar o estado vazio do painel APPS**

Na linha 219, o painel termina com:

```tsx
{!activeApps.length ? <EmptyState>Apps cadastrados aparecerão aqui.</EmptyState> : null}</Panel>
```

Substituir esse trecho por:

```tsx
{!activeApps.length ? <ScopedEmptyState total={apps.filter((app) => app.lifecycleState === "active").length} workspace={currentWorkspace} noun="app" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel>
```

Não altere o resto da linha — o `<div className="app-row">` com os tiles permanece igual.

- [ ] **Step 4: Adicionar a regra de CSS**

Em `apps/desktop/src/App.css`, após o bloco `.empty-state` (linha 1127), inserir:

```css
.scoped-empty {
  display: grid;
  justify-items: start;
  gap: var(--space-2);
}
```

Só tokens existentes, nenhum valor hardcoded.

- [ ] **Step 5: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro de TypeScript.

- [ ] **Step 6: Verificar no app**

Com o app rodando, na Home:

1. Com "Testes" ativo, o painel APPS deve mostrar **"5 apps cadastrados, nenhum em Testes."** e um botão **Vincular**.
2. Clicar em **Vincular** abre a tela de Workspaces já em "Testes", com os checkboxes de Projects e Apps.
3. Marcar um app, voltar para a Home: o tile do app aparece no painel APPS.
4. Clicar em **Todos** no painel CONTEXTO: os 5 apps aparecem, como já acontecia.

Se o passo 1 continuar mostrando o texto antigo, o `total` está sendo calculado sobre `activeApps` (que já é o recorte) em vez de `apps`. Releia o Step 3.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "fix(ui): painel APPS distingue nada cadastrado de nada no workspace"
```

---

### Task 2: Painel PROJECTS

O mesmo defeito, ainda latente: `project_workspaces` tem 2 vínculos hoje, então a mensagem errada não aparece — mas aparece assim que um Workspace ficar sem Projects.

**Files:**
- Modify: `apps/desktop/src/App.tsx:216` (painel PROJECTS)
- Test: nenhum — ver Global Constraints

**Interfaces:**
- Consumes: `ScopedEmptyState({ total, workspace, noun, onLink })`, criado na Task 1 com `total: number`, `workspace: Workspace | null`, `noun: "app" | "project"`, `onLink: () => void`. Não altere o componente nesta task.
- Produces: nada consumido por tasks posteriores.

- [ ] **Step 1: Trocar o estado vazio do painel PROJECTS**

Na linha 216, o painel termina com:

```tsx
{!scopedProjects.length ? <EmptyState>Projects criados aparecerão aqui.</EmptyState> : null}</Panel>
```

Substituir esse trecho por:

```tsx
{!scopedProjects.length ? <ScopedEmptyState total={projects.filter((project) => project.lifecycleState === "active").length} workspace={currentWorkspace} noun="project" onLink={() => { if (currentWorkspace) openWorkspace(currentWorkspace); }} /> : null}</Panel>
```

Não altere o `.map` dos `DataRow` que vem antes.

- [ ] **Step 2: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <2s`, sem erro.

- [ ] **Step 3: Provocar o caso latente**

O banco tem os 2 Projects vinculados a "Testes", então o estado não aparece sozinho. Para vê-lo, crie um Workspace vazio pela própria interface:

1. Ir em Workspaces, criar um chamado `Vazio`.
2. Não vincular nenhum Project a ele.
3. Voltar à Home e selecioná-lo no painel CONTEXTO.
4. O painel PROJECTS deve mostrar **"2 Projects criados, nenhum em Vazio."** com o botão **Vincular**.
5. Clicar em **Vincular** abre Workspaces em "Vazio".

Depois de verificar, arquive o Workspace `Vazio` em Workspaces → menu → Arquivar, para não deixar lixo no banco real.

- [ ] **Step 4: Confirmar que não houve regressão**

Voltar para o Workspace "Testes" e conferir que o painel PROJECTS continua listando os 2 Projects normalmente, com os pontos de atividade e os tempos relativos.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "fix(ui): painel PROJECTS distingue nada cadastrado de nada no workspace"
```

---

## Nota sobre um limite deliberado

Quando `total > 0`, não há Workspace ativo e o escopo está vazio, o componente cai no texto
genérico. Isso é correto: sem Workspace, o escopo é o conjunto todo, então escopo vazio
implica total zero, e o ramo é inalcançável na prática. Está escrito assim por segurança de
tipo, não por ser um caso esperado.

Item arquivado não entra no `total`, por decisão registrada na seção 5 do spec. Consequência
aceita: com 5 apps todos arquivados e nenhum ativo, a mensagem volta a ser
`Apps cadastrados aparecerão aqui.` — tecnicamente verdadeira para o conjunto ativo.

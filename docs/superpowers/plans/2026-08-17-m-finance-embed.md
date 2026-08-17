# M-Finance embutido no M/OS (Feature A) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer "M Finance" abrir dentro da janela do M/OS (iframe numa página nativa de rail) em vez de abrir no navegador padrão do Windows.

**Architecture:** Uma página nova (`FinancePage`) hospeda um `<iframe>` apontando para `https://m-finance-silk.vercel.app`. Nenhum código de `apps/m-finance` muda. O rail ganha um 11º destino, o que exige revisar formalmente o teto documentado em ADR (ADR-031/036/038) antes de tocar o código — por isso a primeira tarefa é a ADR, não o iframe.

**Tech Stack:** React 19 + TypeScript, Tauri 2 (`apps/desktop`), CSS puro (`App.css`, sem CSS-in-JS).

## Global Constraints

- Este repositório **não tem testes de componente/DOM por design** (`apps/desktop/vitest.config.ts` roda só `src/**/*.test.ts` com `environment: "node"` — comentário no próprio arquivo explica que jsdom convidaria a "teste que mente"). Nenhuma tarefa deste plano deve criar um arquivo `*.test.tsx` ou introduzir `@testing-library/react`. Verificação de UI é: `npm run build` (tsc + vite build) para correção de tipos, e inspeção visual manual no cliente Tauri real para comportamento.
- Nenhuma regra de negócio, API, banco, schema ou contrato de domínio do M/OS ou do M-Finance é alterada por este plano.
- Design aprovado em `docs/superpowers/specs/2026-08-17-m-finance-embed-design.md` — qualquer divergência de escopo encontrada durante a implementação volta pra esse spec, não é decidida ad hoc no código.
- Todos os commits seguem o estilo já usado no repo (mensagens curtas, em português, sem `--no-verify`).

---

### Task 1: ADR-039 — revisar o teto do rail para 11

**Files:**
- Modify: `docs/DECISIONS.md` (adicionar seção no final do arquivo, depois de ADR-038)

**Interfaces:**
- Não produz nem consome símbolos de código — é a autorização documentada que a Task 5 (mudança de rail) referencia no commit message.

- [ ] **Step 1: Ler o final do arquivo para confirmar onde a ADR-038 termina**

Run: abrir `docs/DECISIONS.md` e localizar o fim da seção `## ADR-038 — O rail vai a dez, e Apps sai para o Calendário entrar` (ela termina com uma lista de "Consequências" seguida de linha em branco antes do próximo `##` ou do fim do arquivo).

- [ ] **Step 2: Escrever a ADR-039**

Adicionar ao final do arquivo, no mesmo formato das ADRs anteriores (título `##`, `**Data:**`, `**Status:**`, `**Revisa:**`, seções `### Contexto`, `### Decisão`, `### Consequências`):

```markdown

## ADR-039 — O rail vai a onze, e Finance entra sem tirar ninguém

**Data:** 2026-08-17
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-038, ADR-036, ADR-031

### Contexto

A ADR-038 levou o rail a dez e manteve a regra herdada da ADR-036: o próximo
destino exige retirar um. M-Finance é hoje um app web separado
(`apps/m-finance`, Next.js/Postgres/Supabase, deploy em produção), alcançável
pelo M/OS apenas através do App Registry, que abre o navegador padrão do
Windows e tira o usuário da janela do M/OS. A Feature A (ver
`docs/superpowers/specs/2026-08-17-m-finance-embed-design.md`) embute essa
mesma URL num iframe dentro de uma página nativa do M/OS, e o pedido natural é
um destino de rail para ela — Finance não é uma tela secundária, é onde o
usuário vê e mexe no próprio dinheiro.

### Decisão

**Finance entra no rail. Nada sai. O teto passa a ser onze.**

O critério já fixado pela ADR-036 é "algo de que depende a renda ou a memória
do usuário, não algo que ele usa com frequência". Finance passa nesse critério
com folga maior que Apps passava — Apps foi removido justamente por ser só
conveniência (ADR-038), e contas, vencimentos e faturas são renda de forma
direta, não uma leitura extensiva do critério.

Diferente da troca Apps→Calendário da ADR-038, aqui nada precisa sair: o rail
ainda comporta um décimo primeiro item sem ficar ilegível nas larguras
suportadas (840×600 em diante, conforme os lotes de UI/UX já validados), e não
há um destino de menor evidência para substituir sem repetir o experimento já
descartado com Workspaces (ADR-031).

Finance entra no grupo `TRABALHO`, depois de Calendário, antes do grupo
`MEMÓRIA` (Library).

### Consequências

- o teto vira onze, e a regra de troca continua valendo para o próximo pedido:
  o décimo segundo exige retirar um, ou uma ADR nova que justifique não
  retirar, como esta fez;
- o App Registry continua tendo a entrada `m-finance` como está — o rail é um
  caminho adicional, não uma substituição;
- esta ADR não reabre nem contradiz a ADR-032 (M-Finance continua Next.js,
  Postgres e Vercel, rodando exatamente como hoje; só o lugar onde a mesma URL
  é exibida muda).
```

- [ ] **Step 3: Commit**

```bash
git add docs/DECISIONS.md
git commit -m "docs: registra ADR-039, rail vai a onze para caber o Finance"
```

---

### Task 2: CSP — permitir o iframe do M-Finance

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json:53-54`

**Interfaces:**
- Não produz símbolos de código; é pré-requisito de runtime para a Task 4 (sem isso o iframe é bloqueado silenciosamente pelo WebView).

- [ ] **Step 1: Editar a CSP de produção**

Em `apps/desktop/src-tauri/tauri.conf.json`, linha 53, trocar:

```json
      "csp": "default-src 'self' customprotocol: asset:; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; style-src 'self' 'unsafe-inline'",
```

por:

```json
      "csp": "default-src 'self' customprotocol: asset:; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; style-src 'self' 'unsafe-inline'; frame-src https://m-finance-silk.vercel.app",
```

- [ ] **Step 2: Editar a CSP de desenvolvimento**

Na linha seguinte (`devCsp`), trocar:

```json
      "devCsp": "default-src 'self' customprotocol: asset:; connect-src ipc: http://ipc.localhost http://localhost:1420 ws://localhost:1420; img-src 'self' asset: http://asset.localhost data:; style-src 'self' 'unsafe-inline'"
```

por:

```json
      "devCsp": "default-src 'self' customprotocol: asset:; connect-src ipc: http://ipc.localhost http://localhost:1420 ws://localhost:1420; img-src 'self' asset: http://asset.localhost data:; style-src 'self' 'unsafe-inline'; frame-src https://m-finance-silk.vercel.app"
```

- [ ] **Step 3: Validar que o JSON continua válido**

Run: `node -e "JSON.parse(require('fs').readFileSync('apps/desktop/src-tauri/tauri.conf.json', 'utf8')); console.log('ok')"`
Expected: `ok`

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/tauri.conf.json
git commit -m "feat(desktop): libera frame-src do M-Finance na CSP"
```

---

### Task 3: Ícone "finance"

**Files:**
- Modify: `apps/desktop/src/Icon.tsx`

**Interfaces:**
- Produz: `IconName` passa a incluir `"finance"`; `<Icon name="finance" filled?: boolean />` consumível pela Task 5.

- [ ] **Step 1: Adicionar `"finance"` ao union `IconName`**

Em `apps/desktop/src/Icon.tsx:21-39`, adicionar `| "finance"` depois de `| "calendar"` (linha 31):

```tsx
export type IconName =
  | "home"
  | "hermes"
  | "inbox"
  | "projects"
  | "workspaces"
  | "apps"
  | "library"
  | "board"
  | "tempo"
  | "calendar"
  | "finance"
  | "settings"
  | "search"
  | "capture"
  | "plus"
  | "more"
  | "close"
  | "archive"
  | "trash";
```

- [ ] **Step 2: Adicionar o desenho de contorno em `OUTLINE_20`**

Em `apps/desktop/src/Icon.tsx`, logo depois da entrada `calendar` (linha 59), adicionar uma nota curta e a entrada `finance` — nota de banknote (retângulo) com o valor marcado por um círculo, seguindo a regra de primitivas separadas por vão (nunca `fill-rule`):

```tsx
  calendar: <><rect x="3.5" y="5.5" width="13" height="11" /><path d="M3.5 9.5h13M7 3.5v3M13 3.5v3" /></>,
  // Nota com o valor marcado por um circulo vazado: diferencia de "calendar" e
  // "board" sem precisar de simbolo de moeda (que nao cabe limpo no traco de
  // 1.25 a 20px).
  finance: <><rect x="3" y="6" width="14" height="8" /><circle cx="10" cy="10" r="2" /></>,
```

- [ ] **Step 3: Adicionar a silhueta preenchida em `SOLID_20`**

Em `apps/desktop/src/Icon.tsx`, logo depois da entrada `calendar` em `SOLID_20` (linha 94), adicionar:

```tsx
  calendar: <><rect x="3.2" y="5.2" width="13.6" height="11.6" /><rect x="6.4" y="2.9" width="1.6" height="3.4" /><rect x="12" y="2.9" width="1.6" height="3.4" /></>,
  finance: <><rect x="2.9" y="5.9" width="14.2" height="8.2" /></>,
```

Nota: a silhueta preenchida usa só o retângulo (sem o círculo vazado, que exigiria corte composto) — mesma solução já usada em `tempo`, que também simplifica a versão `filled`.

- [ ] **Step 4: Checar tipos**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: sem erros novos relacionados a `Icon.tsx` (a checagem completa só passa depois da Task 5 usar `"finance"` em algum lugar — se `tsc` reclamar de `"finance"` declarado e não usado em nenhum `Record` faltando, é porque `OUTLINE_20`/`SOLID_20` são `Record<IconName, ...>`/`Partial<Record<IconName, ...>>`; `OUTLINE_20` é obrigatório para todo `IconName`, então o Step 2 é obrigatório, não opcional).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/Icon.tsx
git commit -m "feat(desktop): adiciona icone finance"
```

---

### Task 4: `FinancePage` + estilos

**Files:**
- Create: `apps/desktop/src/FinancePage.tsx`
- Modify: `apps/desktop/src/App.css` (novo bloco, ao lado de `.page`/`.startup-state`, por volta da linha 565)

**Interfaces:**
- Produz: `export function FinancePage(): JSX.Element` — sem props, sem estado. Consumido pela Task 5 em `App.tsx`.

- [ ] **Step 1: Criar `FinancePage.tsx`**

```tsx
const FINANCE_URL = "https://m-finance-silk.vercel.app";

/**
 * M-Finance continua Next.js/Postgres/Vercel (ADR-032) — isto so exibe a
 * mesma URL dentro da janela do M/OS em vez de abrir no navegador padrao.
 * Sessao de login fica na propria webview; nao ha passagem de token.
 */
export function FinancePage() {
  return (
    <div className="page finance-page">
      <iframe
        className="finance-frame"
        src={FINANCE_URL}
        title="M Finance"
        allow="clipboard-write"
      />
    </div>
  );
}
```

- [ ] **Step 2: Adicionar estilos em `App.css`**

Logo depois do bloco `.page` (`apps/desktop/src/App.css:561-565`), adicionar:

```css
/* M-Finance continua sendo outro app (ADR-032); isto so o exibe dentro do
   shell do M/OS. Sem padding: o iframe deve ocupar a area de conteudo
   inteira, e o M-Finance ja tem sua propria navegacao interna. */
.finance-page {
  padding: 0;
}

.finance-frame {
  width: 100%;
  height: 100%;
  border: 0;
}
```

- [ ] **Step 3: Checar tipos**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: sem erros em `FinancePage.tsx` (o arquivo ainda não é importado em lugar nenhum nesta etapa — isso é esperado, `tsc` não reclama de arquivo não importado).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/FinancePage.tsx apps/desktop/src/App.css
git commit -m "feat(desktop): cria FinancePage com iframe do M-Finance"
```

---

### Task 5: Ligar Finance ao rail e ao roteamento de `App.tsx`

**Files:**
- Modify: `apps/desktop/src/App.tsx` (import, `Page` type, `nav`, `navGroups`, `pageLabels`, `pageContent`)

**Interfaces:**
- Consome: `FinancePage` de `./FinancePage` (Task 4); `IconName` incluindo `"finance"` (Task 3).
- Produz: rota `"finance"` navegável a partir do rail, sem alterar nenhuma outra rota existente.

- [ ] **Step 1: Importar `FinancePage`**

Em `apps/desktop/src/App.tsx:17` (junto às demais importações de página, próximo a `TempoPage`), adicionar:

```tsx
import { FinancePage } from "./FinancePage";
```

- [ ] **Step 2: Adicionar `"finance"` ao type `Page`**

Em `apps/desktop/src/App.tsx:29`, trocar:

```tsx
type Page = "home" | "hermes" | "inbox" | "projects" | "workspaces" | "apps" | "library" | "tasks" | "tempo" | "calendario" | "settings";
```

por:

```tsx
type Page = "home" | "hermes" | "inbox" | "projects" | "workspaces" | "apps" | "library" | "tasks" | "tempo" | "calendario" | "finance" | "settings";
```

- [ ] **Step 3: Adicionar o destino ao array `nav`, depois de `calendario`**

Em `apps/desktop/src/App.tsx:2523-2524`, trocar:

```tsx
  { page: "calendario", label: "Calendário", icon: "calendar" },
  { page: "library", label: "Library", icon: "library" }];
```

por:

```tsx
  { page: "calendario", label: "Calendário", icon: "calendar" },
  // ADR-039: onze destinos. Finance entra depois de Calendario porque, como
  // Tempo, e de onde sai (ou vai) renda — nao e conveniencia.
  { page: "finance", label: "Finance", icon: "finance" },
  { page: "library", label: "Library", icon: "library" }];
```

- [ ] **Step 4: Ajustar o corte do grupo `TRABALHO`**

Em `apps/desktop/src/App.tsx:2528-2532`, trocar:

```tsx
  const navGroups = [
    { label: "GERAL", items: nav.slice(0, 3) },
    { label: "TRABALHO", items: nav.slice(3, 8) },
    { label: "MEMÓRIA", items: nav.slice(8) },
  ];
```

por:

```tsx
  const navGroups = [
    { label: "GERAL", items: nav.slice(0, 3) },
    { label: "TRABALHO", items: nav.slice(3, 9) },
    { label: "MEMÓRIA", items: nav.slice(9) },
  ];
```

(`nav` passa a ter 10 índices, 0–9: `home, hermes, inbox, tasks, projects, workspaces, tempo, calendario, finance, library`. `TRABALHO` ganha `finance` no fim; `MEMÓRIA` continua só com `library`, agora no índice 9.)

- [ ] **Step 5: Adicionar `"finance"` a `pageLabels`**

Em `apps/desktop/src/App.tsx:2533`, trocar:

```tsx
  const pageLabels: Record<Page, string> = { home: "Home", hermes: "Hermes", inbox: "Inbox", tasks: "Tasks", projects: "Projects", tempo: "Tempo", calendario: "Calendário", library: "Library", apps: "Apps", workspaces: "Workspaces", settings: "Settings" };
```

por:

```tsx
  const pageLabels: Record<Page, string> = { home: "Home", hermes: "Hermes", inbox: "Inbox", tasks: "Tasks", projects: "Projects", tempo: "Tempo", calendario: "Calendário", finance: "Finance", library: "Library", apps: "Apps", workspaces: "Workspaces", settings: "Settings" };
```

- [ ] **Step 6: Adicionar o `if` de roteamento em `pageContent`**

Em `apps/desktop/src/App.tsx`, logo depois da linha do `tempo` (linha 2541: `if (page === "tempo") return <TempoPage ... />;`), adicionar:

```tsx
    if (page === "finance") return <FinancePage />;
```

- [ ] **Step 7: Checar tipos e build completo**

Run: `cd apps/desktop && npm run build`
Expected: build conclui sem erros (tsc + vite build). Se `tsc` acusar `Page`/`IconName` com união incompleta em algum outro `Record<Page, ...>` ou `Record<IconName, ...>` não coberto neste plano, esse é um `Record` que este plano não mapeou — localizar com a mensagem de erro do `tsc` e adicionar a entrada `finance` faltante antes de prosseguir (não pular a checagem).

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat(desktop): adiciona Finance ao rail (ADR-039)"
```

---

### Task 6: QA manual no cliente Tauri real

**Files:** nenhum (tarefa de verificação, sem alteração de código).

**Interfaces:** nenhuma — consome o resultado das Tasks 1–5 completas.

- [ ] **Step 1: Rodar o app em modo dev**

Run: `cd apps/desktop && npm run tauri dev`
Expected: janela do M/OS abre normalmente, sem erro no terminal.

- [ ] **Step 2: Navegar até Finance pelo rail**

No app aberto, clicar no destino "Finance" no rail (grupo TRABALHO, depois de Calendário).
Expected: a página carrega um iframe mostrando a tela de login (ou dashboard, se já houver cookie de sessão anterior) do M-Finance, dentro da janela do M/OS — rail e topbar continuam visíveis, sem sair da janela do M/OS.

- [ ] **Step 3: Verificar console sem erro de CSP**

Abrir o DevTools da webview (se disponível no build dev) e checar o console.
Expected: nenhuma mensagem `Refused to frame ... because it violates the following Content Security Policy directive`.

- [ ] **Step 4: Testar login e persistência de sessão**

Fazer login no M-Finance dentro do iframe. Navegar para outra página do M/OS (ex.: Home) e voltar para Finance.
Expected: sessão continua logada ao voltar (cookie da webview preservado).

- [ ] **Step 5: Verificar o rail em largura compacta**

Redimensionar a janela para 840×600 (ou usar o modo compacto do rail).
Expected: os onze destinos continuam acessíveis (rail expansível/scroll, sem nenhum destino sumir silenciosamente — mesmo critério de aceite já usado nos lotes anteriores de UI/UX).

- [ ] **Step 6: Confirmar que o App Registry não mudou**

Abrir a lista de Apps (Home > Gerenciar ou página Apps) e clicar em "M Finance" ali.
Expected: continua abrindo no navegador padrão do Windows, como antes — o rail é um caminho adicional, não uma substituição (conforme o spec).

- [ ] **Step 7: Rodar a suíte de testes existente**

Run: `cd apps/desktop && npm test -- --run`
Expected: os mesmos arquivos/testes de sempre passam (`calendarDays.test.ts`, `suspiciousEntry.test.ts` — nenhum teste novo é esperado, ver Global Constraints).

- [ ] **Step 8: Checar diffs antes do commit final**

Run: `git diff --check`
Expected: sem conflitos de whitespace.

- [ ] **Step 9: Registrar a evidência de QA**

Sem arquivo de doc dedicado para esta feature (ela não faz parte da trilha `UI-UX-REFINEMENT.md`) — a evidência de QA fica registrada na mensagem do commit final ou relatada diretamente ao usuário ao final da execução do plano, cobrindo os passos 1–7 acima.

# Layout Full-Screen Responsivo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fazer as telas do M/OS ocuparem a largura da janela acima de 1100px, mantendo medida legível nos blocos de texto.

**Architecture:** Inverter onde a medida de leitura mora. Hoje `.page` — o container mais externo — carrega um teto de 1100px que tudo herda, inclusive grades. O teto sai de `.page` e desce para os blocos que contêm texto corrido. As grades da Home passam a derivar o número de colunas da largura disponível, via `auto-fit`/`auto-fill` sobre o token `--column-min` que já existe, sem breakpoint novo.

**Tech Stack:** CSS puro (`App.css`), custom properties (`mos-tokens.css`), React 19 + Vite, shell Tauri 2.

## Global Constraints

- **Escopo é CSS e tokens.** Nenhuma mudança em `crates/`, `src-tauri/`, `api.ts`, `types.ts` ou `App.tsx`. Se um passo parecer exigir mudança em core, storage ou API, o plano está errado — pare e reporte.
- **Spec de origem:** `docs/superpowers/specs/2026-08-14-mos-fullscreen-layout-design.md`.
- **Valores visuais vivem no design system.** Token novo vai em `Design System/design_handoff_frontend/mos-tokens.css`, dentro do bloco `/* extensões de implementação */`, nunca hardcoded no `App.css`. `DECISIONS.md` ADR-019 (linha 431, regra na 447) exige que extensão de token seja explícita e documentada.
- **Não existe teste automatizado de front.** `apps/desktop/package.json` define apenas `"build": "tsc && vite build"` — sem vitest, sem playwright. Não invente framework de teste, não instale dependência de teste. A verificação de cada tarefa é `npm run build` mais inspeção visual nas três larguras.
- **Três larguras obrigatórias em toda verificação visual:** 840px (mínimo da janela, definido em `tauri.conf.json`), 1180px (padrão) e a largura máxima do monitor (~2545px no ambiente de referência).
- **Build do Rust exige ambiente portable.** O app não compila com toolchain padrão nesta máquina. Antes de `npm run tauri dev`, no PowerShell:
  ```powershell
  $env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
  $env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
  ```
  Sem isso o linker falha com `cannot find @C:\WINDOWS\TEMP\...: Invalid argument`.
- **Não alterar** as media queries de 1280px (`App.css:2242`) e 960px (`App.css:2254`), o kanban de seis colunas (`App.css:1407`), nem os caps de medida que já existem e já estão corretos: `.startup-state h1/p` (470), `.hermes-empty` (770), `.detail-header p` (1181), `.resource-note p` (1358), `.hermes-quiet` (2358).

---

### Task 1: Grades da Home derivam colunas da largura

Primeira porque não depende de nada e o estado intermediário é seguro: enquanto `.page` ainda estiver capada em 1100px, a Home passa de 2 para ~4 colunas dentro dessa faixa. Fica mais denso, nunca quebrado.

**Files:**
- Modify: `apps/desktop/src/App.css:529-534` (`.home-sections`)
- Modify: `apps/desktop/src/App.css:536-541` (`.context-switcher`)
- Test: nenhum — não existe infra de teste de front neste projeto (ver Global Constraints)

**Interfaces:**
- Consumes: `--column-min: 260px`, já definido em `Design System/design_handoff_frontend/mos-tokens.css:173`. Não redefinir.
- Produces: nada que tarefas seguintes consumam. Task 2 é independente desta.

- [ ] **Step 1: Registrar o estado visual atual**

Suba o app e fotografe a Home antes de mexer, para comparação.

```powershell
$env:PATH = "C:\Dev\pessoal\m-os\`$tools\w64devkit\bin;" + $env:PATH
$env:TMP = "$env:LOCALAPPDATA\Temp"; $env:TEMP = $env:TMP
cd C:\Dev\pessoal\m-os\apps\desktop
npm run tauri dev
```

Esperado: janela abre em 1180×760. Home mostra `CONTEXTO`, e abaixo os painéis `EM ANDAMENTO` e `RECENTES` lado a lado em duas colunas, com `PROJECTS` e `APPS` na linha seguinte. Anote isso — é o "antes".

- [ ] **Step 2: Trocar `.home-sections` para `auto-fit`**

Em `apps/desktop/src/App.css`, o bloco na linha 529. Substituir:

```css
.home-sections {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-4);
  padding-top: var(--space-5);
}
```

por:

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

- [ ] **Step 3: Trocar `.context-switcher` para `auto-fill`**

No mesmo arquivo, bloco na linha 536. Substituir:

```css
.context-switcher {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-3);
  padding: var(--space-3) 0 var(--space-4);
}
```

por:

```css
/* auto-fill, nao auto-fit: preserva as trilhas vazias para que um unico
   workspace continue do tamanho de um chip em vez de esticar pela tela. */
.context-switcher {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(var(--column-min), 1fr));
  gap: var(--space-3);
  padding: var(--space-3) 0 var(--space-4);
}
```

A diferença entre `auto-fit` no passo anterior e `auto-fill` aqui é intencional e é o ponto sutil da tarefa. Não uniformize os dois.

- [ ] **Step 4: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <1s`, sem erro de TypeScript e sem warning de CSS.

- [ ] **Step 5: Verificar visualmente nas três larguras**

Com o app rodando, redimensione a janela e confira:

- **840px** — `CONTEXTO` mostra o chip do workspace em tamanho de chip, não esticado. Painéis da Home empilham em 1–2 colunas. Nada corta nem some.
- **1180px** — painéis da Home agora aparecem em 3–4 colunas em vez de 2. Mais denso que o "antes" do Step 1. Correto.
- **~2545px** — **ainda não muda nada**, porque `.page` continua capada em 1100px. Isto é esperado nesta tarefa; a Task 2 é que libera.

Se o chip do workspace esticar pela largura toda em qualquer uma das três, o `auto-fill` do Step 3 virou `auto-fit` por engano. Volte e corrija.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/App.css
git commit -m "fix(ui): grades da Home derivam colunas da largura disponivel"
```

---

### Task 2: Página fluida e medida de leitura

**Files:**
- Modify: `Design System/design_handoff_frontend/mos-tokens.css` (bloco de extensões de implementação, junto de `--content-max` na linha 172)
- Modify: `apps/desktop/src/App.css:345-349` (`.page`)
- Modify: `apps/desktop/src/App.css:1919-1922` (`.settings-page`)
- Modify: `apps/desktop/src/App.css:1939-1944` (`.setting-row p, .support-copy`)
- Modify: `apps/desktop/src/App.css:1121-1125` (`.empty-state`)
- Test: nenhum — ver Global Constraints

**Interfaces:**
- Consumes: `--content-max: 1100px` (`mos-tokens.css:172`), que deixa de ser aplicado por `.page` e passa a ser opt-in de `.settings-page`. **Não remover o token.**
- Produces: `--measure: 70ch`, custom property nova, disponível para qualquer bloco de texto corrido que precise de cap no futuro.

- [ ] **Step 1: Declarar o token `--measure`**

Em `Design System/design_handoff_frontend/mos-tokens.css`, imediatamente após a linha 172 (`--content-max: 1100px;`), adicionar:

```css
  --measure: 70ch;
```

O valor 70ch não é arbitrário: é a medida que `.startup-state`, `.detail-header p` e `.resource-note p` já usam hardcoded no `App.css`. O token dá nome ao que já era prática.

- [ ] **Step 2: Tornar `.page` fluida**

Em `apps/desktop/src/App.css`, bloco na linha 345. Substituir:

```css
.page {
  width: min(100%, var(--content-max));
  min-height: 100%;
  padding: var(--space-5) var(--content-margin) var(--space-7);
}
```

por:

```css
/* Fluida por decisao: o teto de leitura nao mora mais no container mais externo,
   e sim nos blocos que contem texto corrido. Ver spec de 2026-08-14. */
.page {
  width: 100%;
  min-height: 100%;
  padding: var(--space-5) var(--content-margin) var(--space-7);
}
```

- [ ] **Step 3: Dar a `.settings-page` o teto que a página largou**

Bloco na linha 1919. Substituir:

```css
.settings-page {
  display: grid;
  gap: var(--space-6);
}
```

por:

```css
/* Unica pagina que e leitura e formulario de ponta a ponta, entao assume o teto
   que .page largou. Alinhada a esquerda de proposito: todo o resto do app alinha
   a esquerda a partir do rail, e centralizar so o Settings criaria excecao. */
.settings-page {
  display: grid;
  gap: var(--space-6);
  max-width: var(--content-max);
}
```

Não adicione `margin-inline: auto`.

- [ ] **Step 4: Capar os dois blocos de texto que não tinham cap**

Bloco na linha 1939. Substituir:

```css
.setting-row p,
.support-copy {
  margin: var(--space-1) 0 0;
  color: var(--text-secondary);
  font: var(--text-small);
}
```

por:

```css
.setting-row p,
.support-copy {
  margin: var(--space-1) 0 0;
  max-width: var(--measure);
  color: var(--text-secondary);
  font: var(--text-small);
}
```

E o bloco na linha 1121. Substituir:

```css
.empty-state {
  margin: var(--space-4) 0;
  color: var(--text-secondary);
  font: var(--text-body);
}
```

por:

```css
.empty-state {
  margin: var(--space-4) 0;
  max-width: var(--measure);
  color: var(--text-secondary);
  font: var(--text-body);
}
```

- [ ] **Step 5: Verificar o build**

```bash
cd apps/desktop && npm run build
```

Esperado: `✓ built in <1s`, sem erro.

- [ ] **Step 6: Verificar visualmente nas três larguras**

- **840px** — nada regride. As media queries de 1280/960 continuam mandando no padding. Home legível, sem overflow horizontal.
- **1180px** — conteúdo ocupa a janela toda, sem a faixa vazia à direita.
- **~2545px** — **este é o teste que importa.** Home ocupa a largura inteira, painéis distribuídos. O vazio assimétrico da direita sumiu.

Confira também, em 2545px, duas telas de texto:

- **Settings** — os parágrafos de apoio (ex.: *"Backups e exports podem conter dados pessoais em texto claro."*) devem quebrar em ~70 caracteres, **não** esticar por 2500px. A página inteira para em 1100px, alinhada à esquerda.
- **Qualquer estado vazio** (ex.: *"Apps cadastrados aparecerão aqui."* na Home) — também deve parar em ~70 caracteres.

Se algum desses textos atravessar a tela, o `max-width: var(--measure)` do Step 4 não pegou — provavelmente o token do Step 1 não foi declarado, ou foi declarado fora do seletor `:root`. Confira com o devtools se `--measure` resolve para `70ch`.

- [ ] **Step 7: Commit**

```bash
git add "Design System/design_handoff_frontend/mos-tokens.css" apps/desktop/src/App.css
git commit -m "fix(ui): pagina fluida com medida de leitura nos blocos de texto"
```

---

## Limite conhecido, esperado, não é bug

A 2545px, quatro painéis dividindo a largura dão ~600px cada, e listas de texto curto ficam ralas nessa medida. Este plano corrige o **enquadramento**, não a **densidade**.

A densidade depende dos oito widgets do `M-OS Home v0.6.dc.html` (`NOW`, `INBOX`, `PARADO`, `TODAY`, `PROJECTS`, `SALVO ESTA SEMANA`, `SHORTCUTS`, `SEM HORA`), que são ciclo seguinte e exigem a Fase 4 do roadmap. Não tente compensar a raleza aqui inventando conteúdo ou reduzindo o número de colunas — isso desfaria a tarefa.

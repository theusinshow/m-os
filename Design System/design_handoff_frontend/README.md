# Handoff: M/OS — front-end (shell, telas e motion)

## Overview

O back-end e o fluxo do M/OS já existem (`apps/desktop`, Tauri + React + TypeScript + SQLite, `api.ts` como fronteira). Este pacote traz **o desenho da camada visual**: shell, seis telas, overlays, símbolo, motion e tokens.

O objetivo do trabalho é **substituir a aparência sem tocar em comportamento**: as mesmas páginas, os mesmos dados, as mesmas funções de `api.ts`, com a linguagem visual fechada em v0.3–v0.7.

## About the Design Files

Os arquivos em `design/` são **referências de design escritas em HTML** — protótipos que mostram aparência e comportamento pretendidos. **Não são código de produção e não devem ser copiados para dentro do app.**

A tarefa é **recriar esses desenhos no ambiente que já existe**: React 18 + TypeScript, CSS em `apps/desktop/src/App.css`, ícones em `apps/desktop/src/Icon.tsx`, chamadas em `apps/desktop/src/api.ts`. Nada de nova biblioteca de UI, nada de framework de estilo novo, nada de dependência de animação — o motion descrito aqui é CSS puro (`@keyframes` + `transition`).

Para abrir os protótipos: qualquer `design/*.dc.html` roda direto no navegador (precisa do `design/support.js` ao lado, já incluído). O arquivo principal é `M-OS Redesign v0.7 - Telas.dc.html` — clique nos ícones do rail para navegar entre as seis telas, `Ctrl+K` abre o Command, `Esc` fecha.

## Fidelity

**Alta fidelidade.** Cores, tipografia, espaçamento, alturas de linha, durações e easings são finais. Onde o protótipo e este README divergirem, **este README manda**. Onde o README for omisso, vale `mos-design-system.md`; onde ele também for omisso, vale `mos-tokens.css`.

Conteúdo é falso (Captures, Projects, Apps de exemplo). Estrutura, hierarquia e estados são reais.

---

## Design Tokens

Fonte única de verdade: **`mos-tokens.css`** (incluído). Importar como primeiro arquivo de estilo e **nunca** repetir valor de cor, tamanho, spacing, radius ou duração dentro de componente.

Resumo dos valores usados nas telas:

### Cor · dark (padrão)

| Token | Valor | Uso |
|---|---|---|
| `--canvas` | `#0A0C0E` | fundo da janela |
| `--surface` | `#101316` | card, painel, coluna de kanban |
| `--surface-raised` | `#171B1F` | overlay, app icon, recibo |
| `--surface-hover` | `#1E2429` | hover de row, item selecionado no Command |
| `--border` | `#1E2429` | separador de row, borda de card |
| `--border-strong` | `#2A3136` | borda de input, botão secundário, moldura de overlay |
| `--text` | `#E7EAEC` | texto primário |
| `--text-secondary` | `#8C949A` | descrição, metadata legível |
| `--text-system` | `#626A70` | rótulo mono, ícone inativo |
| `--text-disabled` | `#565E63` | atalho de teclado, contagem secundária |
| `--text-placeholder` | `#4E565B` | placeholder de campo |
| `--signal-fill` | `#E7C24E` | sódio: preenchimento |
| `--signal-ink` | `#E7C24E` (dark) / `#8A6A12` (light) | sódio: tinta |
| `--signal-wash` | `rgba(231,194,78,0.16)` | fundo de selecionado |
| `--on-signal` | `#0A0C0E` | texto/barra sobre sódio |
| `--success` | `#5FA37E` | — |
| `--danger` | `#D95546` | destrutivo |

Light mode é **paridade, não inversão**: ver bloco `[data-theme='light']` em `mos-tokens.css`. A única regra que muda de comportamento: **âmbar puro não é usado como tinta no light** (`--signal-ink: #8A6A12`).

### Tipografia

Duas famílias, três pesos, nenhuma exceção.

- **Produto:** Schibsted Grotesk 400 / 500 / 700
- **Sistema:** JetBrains Mono 400 / 500 — só para dado de sistema: rótulo, atalho, contagem, caminho, timestamp, ID

| Papel | Valor | Tracking |
|---|---|---|
| display | `700 48px/1.05` | `-0.034em` |
| title | `700 28px/1.15` | `-0.024em` |
| capture | `400 21px/1.3` | `-0.022em` |
| body | `400 15px/1.55` | `-0.008em` |
| ui | `400 14px/1.4` | `-0.008em` |
| small | `400 13px/1.45` | `-0.008em` |
| meta (mono) | `400 11px/1.4` | `0.05em` |
| micro (mono) | `400 9px/1.4` | `0.14em` |

Rótulo de painel = mono 11px, `letter-spacing: 0.14em`, maiúscula, `--text-system`.

### Spacing, geometria, alturas

- Spacing: **apenas** 4 · 8 · 12 · 20 · 32 · 52 · 84
- Radius: `3px` em tudo · `2px` em elementos de 14–20px · `8px` só em app icon e overlay grande
- Stroke de ícone: 1.5 em 24px · 1.25 em 20px · 1 em 16px — **um desenho por tamanho, nunca escalar o SVG**; terminais retos
- Larguras: rail `52` · sidebar `232` · drawer `400` · overlay capture `640` · overlay command `720` · dialog `440` · menu `216` · margem de conteúdo `56`
- Alturas: row `34` · row densa `30` · row de duas linhas `56` · controle `36` (sm `28`, lg `44`) · input `38` · item de menu `30` · topbar `44`
- Overlay entra a `34%` do topo da janela

---

## Símbolo (fechado)

Barra sólida, **três desenhos** com ângulo corrigido por escala. Nunca escalar um único SVG.

```
1024 · 512 · 256 · 128   22°   polygon points="38,8 53,8 26,56 11,56"
64 · 48                  18°   polygon points="40,10 54,10 24,54 10,54"
32 · 24 · 16             14°   polygon points="42,12 56,12 22,52 8,52"
```

viewBox `0 0 64 64` nos três. Centroide em (32,32) — rotação é sempre `transform-origin: center`.

**Campo:** decidido em `#E7C24E` com a barra em tinta (`#0A0C0E` no dark, `#14181A` no light). Radius do quadrado: 20% em 1024 · 11 em 64 · 6 em 32 · 3 em 16.

**Motion oficial da marca: `Meia-volta`** — a barra é simétrica em 180°, então uma meia-volta cai exatamente sobre ela mesma. Nada entra, nada sai.

```css
@keyframes barHalf {
  0%, 22%   { transform: rotate(0deg); }
  62%, 100% { transform: rotate(180deg); }
}
/* abertura do app: 1x, 400ms  ·  capa/splash em loop: 5.2s */
```

Dois usos derivados, mesma geometria:

- **Cursor → marca** (`-22° → 0° → 158°`, 3.4s): a barra nasce vertical como o cursor do Capture e vira a marca. Uso opcional em onboarding.
- **Trabalhando** (`180°`, 900ms, `linear`, infinito): estado de sistema ocupado. **É o único spinner do sistema** — não usar círculo, não usar três pontos.

Os ícones dos apps da família M/ continuam **brancos, moldurados e um degrau menores**: só o sistema usa sódio e massa sólida.

---

## A sintaxe da barra `/`

Regra de identidade que atravessa a UI inteira — sempre em mono e `--signal-ink`:

1. **Caminho de contexto** no topo de cada tela: `M / INBOX`, `M / WEB-DESIGN / LIBRARY`. Substitui título de página e breadcrumb. Último segmento em `--text`, anteriores em `--text-system`.
2. **Limiar de entrada:** todo campo onde algo entra no sistema começa com a barra — Capture, Command, Search, ditado.
3. **Comando:** digitar `/` dentro do campo transforma texto em comando. Sem paleta separada, sem modo.
4. **Autoria do sistema:** barra vertical de 2px em sódio marca tudo que o sistema produziu (interpretação do Hermes, síntese) — e é **o mesmo marcador de seleção**.
5. **Transição de contexto:** na troca de Workspace a barra percorre 20px e o conteúdo faz cross-fade (220ms).

Nunca: dentro de conteúdo do usuário · duas barras com funções diferentes na mesma linha · como divisor decorativo · em cor que não seja `--signal-ink`.

---

## Motion

Tabela fechada. Toda animação da UI é uma linha desta tabela — se um gesto não está aqui, ele não existe.

| Gesto | Duração | Easing | Propriedade |
|---|---|---|---|
| Hover / press | 75ms | `--ease-state` | background, border-color |
| Check / select | 140ms | `--ease-state` | opacity, background |
| Overlay entra | 160ms | `--ease-enter` | opacity + translateY(-6px) + scale(0.99) |
| Overlay sai | 90ms | `--ease-exit` | opacity |
| Item se move (FLIP) | 180ms | `--ease-state` | transform |
| Row nova na lista | 180ms | `--ease-state` | opacity + translateY(-3px) |
| Troca de tela / Workspace | 220ms | `--ease-enter` | barra 20px em X + cross-fade |
| Primeira abertura | ≤400ms | `--ease-enter` | 1x por sessão |
| Reduced motion | 0–80ms | linear | **só opacity, nenhum transform** |

```
--ease-enter: cubic-bezier(0.16, 1, 0.3, 1);
--ease-exit:  cubic-bezier(0.4, 0, 1, 1);
--ease-state: cubic-bezier(0.2, 0, 0, 1);
```

Keyframes usados no protótipo (copiar como estão, renomear se colidir):

```css
@keyframes viewEnter    { from { opacity: 0; transform: translateY(4px); }  to { opacity: 1; transform: none; } }
@keyframes pathBar      { from { opacity: 0; transform: translateX(-20px); } to { opacity: 1; transform: none; } }
@keyframes rowEnter     { from { opacity: 0; transform: translateY(-3px); } to { opacity: 1; transform: none; } }
@keyframes receiptEnter { from { opacity: 0; transform: translateY(10px); } to { opacity: 1; transform: none; } }
@keyframes overlayEnter { from { opacity: 0; transform: translateY(-6px) scale(0.99); } to { opacity: 1; transform: none; } }
@keyframes scrimEnter   { from { opacity: 0; } to { opacity: 1; } }
@keyframes savedWash    { from { background: var(--signal-wash); } to { background: transparent; } }
@keyframes caret        { 0%, 46% { opacity: 1; } 47%, 100% { opacity: 0.15; } }
@keyframes barSpin      { from { transform: rotate(0deg); } to { transform: rotate(180deg); } }
@keyframes barHalf      { 0%, 22% { transform: rotate(0deg); } 62%, 100% { transform: rotate(180deg); } }
```

`@media (prefers-reduced-motion: reduce)`: `mos-tokens.css` já zera as durações e mata `animation`. Não escrever transform animado fora do que a tabela permite.

---

## Shell

Grid de duas colunas: `52px` (rail) + `1fr`. Janela de referência 1440×980, sem scroll no shell — cada tela cabe ou rola internamente.

### Rail — 52px

- Símbolo no topo: quadrado sódio de 26px, radius 5, desenho de 14°, 16px de barra dentro. **Não é clicável** — é assinatura, não destino. `margin-bottom: 24px`.
- Seis destinos, `gap: 4px`, cada um 40px de altura, ícone 20px stroke 1.25 (paths verbatim de `Icon.tsx`): **home · inbox · board · projects · library · apps**.
- Ativo: barra de 2px em `--signal-fill` colada na borda esquerda do item (16px de altura, 12px do topo) + ícone em `--text`. Inativo: ícone em `--text-system`.
- Rodapé: botão Quick Capture, 28px, borda `--border-strong`, radius 3, glifo `+` 16px; hover troca a borda para `--signal-ink`.
- `Settings` fica fora dos seis (rodapé ou Command). O sistema tem oito páginas e o rail aceita seis — **Workspaces e Settings entram pelo Command**, não pelo rail.

### Topbar — 44px

- Borda inferior `--border`.
- Gatilho de Command à esquerda: 28px de altura, borda `--border-strong`, radius 3, conteúdo `/` (sódio, mono 13) + "Command" (`--text-secondary`, 13) + `CTRL K` (mono 11, `--text-disabled`). Hover: borda `--text-system`.
- À direita: estado de sistema. Quando ocupado, a barra girando (`barSpin`, 13px) + `SINCRONIZANDO` em micro mono; depois, o meta da página (`QUA 13 AGO · 14:22` na Home, nome da página nas outras).

---

## Screens / Views

Seis telas. Todas abrem com `viewEnter` 220ms e o caminho de contexto entrando com `pathBar`.

### 1 · Home — "o que está acontecendo e o que preciso fazer"

Padding `32px 56px`.

1. **Caminho:** `M / HOME`.
2. **Capture** (a exceção deliberada do sistema: sem caixa, sem borda). Barra de 7×22px em sódio · texto 21px (`--text-placeholder` vazio, `--text` preenchido) · caret de bloco 8×20px em `--text-system` piscando (`caret`, 1.1s, `steps(1,end)`) · botão `Salvar ⏎` de 28px em sódio à direita. Fecha com uma linha de base `--border-strong` a 16px.
3. **CONTEXTO:** rótulo mono + régua. Grade de 4 colunas com os Workspaces: card radius 3, padding 12, nome 15/500, meta em mono 11. Ativo: borda `--signal-ink` + fundo `rgba(231,194,78,0.06)`.
4. **Quatro painéis** em grade 2×2, `gap: 20px`, cada um com rótulo mono + régua `--border`:
   - `EM ANDAMENTO` — rows de 34px: título em ui 14 + projeto em mono 11 à direita. Contagem no cabeçalho.
   - `RECENTES` — Captures, mesma row + tempo relativo. Cabeçalho mostra `INBOX <n>`. Row nova entra com `rowEnter` **e** um `savedWash` de 900ms (o único uso de fundo sódio em row não selecionada).
   - `PROJECTS` — ponto de 5px (sódio = ativo hoje, `--border-strong` = resto) + nome + meta.
   - `APPS` — app icons de 44px, radius 8, `--surface-raised`, borda `--border-strong`, inicial em 15/700; atalho `⌘1…9` em mono 11 embaixo. Hover só troca a borda.
5. Separador de row: 1px `--border`, **nunca gap**.

Motion da Home: nada se move sozinho, com uma exceção — se houver cronômetro (ChronoCAD), ele é o único elemento vivo.

### 2 · Inbox — "processar o que foi capturado"

Duas colunas `1fr 1fr`, divisor `--border`.

- **Esquerda** (`padding: 32 32 32 56`): caminho `M / INBOX` + `<n> ITENS` à direita. Rows de duas linhas (56px mínimo): barra de seleção de 2px colada à esquerda (transparente quando não selecionada), conteúdo 15/1.4, fonte da captura em mono 11 (`SHARE · IOS`, `QUICK CAPTURE`, `VOZ · 14 s`, `SCREENSHOT TOOL`), tempo em mono 11 à direita. Selecionada: `--signal-wash` a 6% + barra em sódio. Hover: `--surface`.
- **Direita** (`padding: 32 56 32 32`): rótulo `SELECIONADO`, o texto da captura em 21/1.35, dois chips de metadata (borda `--border-strong`, radius 2, mono 11).
  - **Interpretação do Hermes:** bloco com barra de 2px em sódio à esquerda, 16px de padding — tokens em mono com sublinhado tracejado `--border-strong` (`RESOURCE · Web Design · sem data`), cada um clicável e corrigível, seguido de uma linha em small explicando que Tab corrige e Enter aplica.
  - **Ações:** `Criar Task` (primário sódio, 36px) · `Salvar Resource` e `Arquivar` (secundário, borda `--border-strong`) · `Organizar tudo` empurrado à direita, com filete de 3px em sódio antes do rótulo (é ação do Hermes).
  - Pé: `J / K percorre · Espaço processa · ⌘Z desfaz` em small `--text-disabled`.

### 3 · Tasks (Kanban)

Padding `32px 56px`, coluna única de conteúdo com header + grade.

- Caminho `M / TASKS`; à direita, dica em mono.
- **Seis colunas** `gap: 12px`: `INBOX · BACKLOG · PLANNED · DOING · REVIEW · DONE`. Cabeçalho de coluna: rótulo mono 11 `0.14em` + contagem em mono `--text-disabled`, régua de 1px. **`DOING` é a única coluna com rótulo e régua em sódio** — é o estado que importa.
- **Card:** `--surface`, borda `--border`, radius 3, padding 12; título ui 14/1.35, projeto em mono 11 `--text-system` a 8px. Hover: borda `--border-strong`. Concluído: título em `--text-secondary` + line-through.
- Card entra com `rowEnter`; movimento entre colunas usa FLIP de 180ms (`transform`), nunca reflow visível. No protótipo, clicar avança de coluna — no app, drag (já implementado) e `J/K` continuam valendo.
- Toda mudança de estado emite recibo com `DESFAZER`.

### 4 · Projects

Duas colunas `1fr 1fr`, mesmo padrão de duas-panes da Inbox.

- **Esquerda:** caminho `M / PROJECTS`; botão `Novo Project` (secundário, 28px, borda `--border-strong`, hover borda sódio); rows de duas linhas com nome 15, descrição 13 `--text-secondary` truncada, `<n> TASKS` em mono à direita; seleção pela barra de 2px.
- **Direita:** rótulo `PROJECT`, nome em title 28/700, descrição em body. Bloco de metadata com `REPOSITÓRIO` (mono 14) e `ATUALIZADO`, fechado por régua. Depois `TASKS` do projeto em rows de 34px com o estado em mono à direita. Pé em small `--text-disabled`.

### 5 · Library

Padding `32px 56px`. Território de exploração — o único lugar onde imagem manda.

- Caminho de três segmentos: `M / WEB-DESIGN / LIBRARY` + `248 ITENS`.
- **Filtros como texto**, não chips: linha de rótulos em mono 11 `0.1em` (`TUDO · SITES · LIBRARIES · IMAGENS · NOTAS`), ativo em `--text`, resto em `--text-system`; à direita `GRID · LISTA`. Régua `--border` embaixo.
- **Grade de 4 colunas**, `gap: 20px`. Tile: proporção fixa 4:3, radius 3, borda `--border`; sem imagem, o fundo é `repeating-linear-gradient(135deg, #171B1F 0 7px, #101316 7px 15px)` — hachura, **nunca ilustração ou ícone gigante**. Tipo do recurso em mono 11 no canto inferior. Selecionado: borda em sódio + barra de 2px vertical dentro da borda esquerda.
- Abaixo do tile: título 15/1.4, o **motivo pelo qual foi salvo** em 13 `--text-secondary`, origem em mono 11 `--text-system`. O motivo é o que torna o acervo recuperável — ele nunca é omitido.
- Tile entra com `tileEnter` (opacity + scale 0.985, 180ms).

### 6 · Apps

Duas panes.

- **Esquerda:** caminho `M / APPS`, botão `Novo App`, rows com nome, descrição e tipo de lançamento (`LOCAL` / `WEB`) em mono.
- **Direita:** app icon de 44px + nome em title 28 + descrição. Ações `Abrir` (primário) e `Editar` (secundário). Grade 2×2 de metadata entre duas réguas: `TIPO`, `WORKSPACE`, `DESTINO` (mono 13, `word-break: break-all`), `ÚLTIMA ABERTURA`. Depois `CAPACIDADES`: quatro rows densas de 30px — `OPEN · READ · WRITE · AUTOMATE` — com `✓` em `--text` ou `—` em `--text-disabled`. Pé: "Capacidade não declarada é capacidade que o Hermes não tenta usar."

---

## Overlays

### Command — 720px

Scrim `rgba(8,9,11,0.62)` com `scrimEnter` 160ms; painel a 34% do topo, `--surface-raised`, borda `--border-strong`, radius 3, `box-shadow: 0 20px 48px rgba(0,0,0,0.5)`, entra com `overlayEnter` 160ms e sai em 90ms só com opacity.

- **Campo:** 52px, `/` em sódio mono 18 + consulta em 18 Grotesk + caret de bloco piscando; `ESC FECHA` em micro mono à direita; régua embaixo.
- **Resultados:** rows de 44px — barra de 2px (sódio no item ativo) · tipo em mono 11 `0.14em` numa coluna fixa de 84px · título em body 15 · meta em mono 11 à direita. Ativo: `--surface-hover`. Hover: idem.
- **Pé:** 34px com `↑↓ NAVEGA · ⏎ ABRE · / COMANDO · TAB HERMES` em micro mono.
- Busca atravessa Task, Resource, Capture, Project, App e ações. `Ctrl+K` abre e fecha; `Esc` fecha.

### Quick Capture — 640px

Mesma moldura e mesma entrada. Barra de 7×22 em sódio + `What's on your mind?` em 21 `--text-placeholder` + três traços de amplitude apagados à direita (a única presença da voz em repouso — sem ícone de microfone). Régua, e pé com `⏎ SALVA E FECHA` / `ESC`. Salva, fecha e devolve o foco ao app anterior; nunca bloqueia esperando interpretação.

### Recibo (undo)

Canto inferior esquerdo, 72px da borda, 24px do pé. Altura 44px, `--surface-raised`, borda `--border-strong`, radius 3, sombra de overlay, entra com `receiptEnter`. Filete de 3px em sódio + mensagem em ui 14 + `DESFAZER · CTRL Z` em mono 11 sódio. Vive ~5s e sai.

Regra: **executar → informar → permitir undo**, em vez de "tem certeza?" para cada operação. Confirmação explícita só em ação externa ou destrutiva (Issue no GitHub, restore de backup, lixeira definitiva).

---

## Interactions & Behavior

- **Rail:** clique navega; a troca de tela é cross-fade de 220ms com a barra do caminho percorrendo 20px. Estado anterior da tela (seleção, filtro, scroll) é preservado.
- **Capture:** `Enter` salva imediatamente e limpa o campo — nunca espera interpretação. A Capture aparece no topo de `RECENTES` com `rowEnter` + `savedWash`, e o recibo confirma. Se a interpretação do Hermes demorar mais de ~200ms, ela chega depois, na Inbox.
- **Inbox:** clique seleciona; `J/K` percorre; `Espaço` processa; ações do pane direito operam sobre a seleção. Toda ação reversível emite recibo.
- **Kanban:** drag (já existente) + `J/K` para mover; card se move com FLIP de 180ms; recibo com undo.
- **Library:** clique seleciona (borda + barra em sódio); duplo clique abre o link/arquivo.
- **Apps:** `Abrir` dispara o launch existente e atualiza `ÚLTIMA ABERTURA`.
- **Teclado:** `Ctrl+K` Command · `Ctrl+Shift+Space` Quick Capture global · `Esc` fecha overlay · `Ctrl+Z` desfaz a última ação com recibo vivo · `⌘1…9` abre app fixado.
- **Estados obrigatórios por componente:** default, hover, focus, pressed, selected, loading, success, warning, error, disabled, empty. Focus é **idêntico em todo o sistema**: borda `--signal-ink` + halo `0 0 0 3px --signal-ring`, sem outline do browser.
- **Warning não tem cor:** ícone + frase + borda neutra. Cor só em `--danger` (destrutivo) e sódio (sinal).
- **Empty state ensina:** "Nothing on your mind right now.", "Add first task" — nunca uma tela que pareça quebrada, nunca ilustração.
- **Erro preserva confiança:** o que aconteceu, se algo foi perdido, o que fazer agora. Nunca "Something went wrong" quando há informação melhor disponível.

## State Management

Nada novo. O estado do front-end continua o que `App.tsx` já tem: `page`, seleções por página (`selectedProjectId`, `selectedAppId`, `selectedResourceId`), `commandOpen`, `drawerTask`, `viewedCapture`, `undo` + timer do recibo, `theme`, `busy` durante chamadas de `api.ts`.

O protótipo acrescenta apenas dois estados visuais que valem a pena portar:

- `busy` global alimentando o indicador da topbar (a barra girando) — hoje só existe implícito.
- `savedIds` (ou um `Set` efêmero) para dar o `savedWash` na row recém-criada e deixar a criação **visível** sem toast extra.

Nenhuma chamada de `api.ts` muda de assinatura. Nenhuma tabela muda.

## Assets

- **Ícones:** os paths de `apps/desktop/src/Icon.tsx` já são os desenhos aprovados — manter verbatim, apenas garantir um desenho por tamanho e `stroke-linecap: butt`.
- **Símbolo:** gerar SVG a partir dos três polygons deste README (não há PNG neste pacote). Rasterizar para `src-tauri/icons/` nos tamanhos do Tauri; conferir 16px na taskbar, no tray e no favicon antes de fechar.
- **Fontes:** Schibsted Grotesk e JetBrains Mono. No protótipo vêm do Google Fonts; **no app, empacotar local** (`@font-face`) — o desktop não deve depender de rede para renderizar texto.
- **Imagens de Library:** nenhuma incluída. Enquanto não houver screenshot real, o tile usa a hachura descrita.

## Files

Em `design/`:

| Arquivo | O que é |
|---|---|
| `M-OS Redesign v0.7 - Telas.dc.html` | **principal** — shell + 6 telas + Command + Quick Capture + recibo, navegável |
| `M-OS Home v0.6.dc.html` | Home em grade de widgets (Now, Today, Calendar 3 dias, Shortcuts, Inbox, Nudge, Library strip) — próxima etapa da Home |
| `M-OS Symbol - Barra em Campo Sódio v0.1.dc.html` | símbolo, ícone em escala real, os três motions, fonte para exportação |
| `M-OS Components v0.5.dc.html` | biblioteca de componentes e estados |
| `M-OS Foundation v0.4 - Aberturas.dc.html` | voz, Library, drag & drop, densidade |
| `M-OS Foundation v0.3.dc.html` | fundação travada: território, tipografia, cor, geometria, marca, sintaxe da barra |
| `M-OS Ideas - Hermes e Home v0.1.dc.html` | banco de ideias (chat do Hermes, catálogo de widgets) — **não é escopo**, é contexto |

Na raiz deste pacote:

| Arquivo | O que é |
|---|---|
| `mos-tokens.css` | tokens reais, prontos para importar |
| `mos-design-system.md` | especificação completa do sistema |
| `AGENTS.md` | regras curtas para agente que escreve código neste sistema |

## Ordem sugerida de implementação

1. `mos-tokens.css` na raiz do estilo + `@font-face` local; remover do `App.css` todo valor de cor/spacing/duração hardcoded.
2. Shell: rail, topbar, caminho de contexto, focus ring único, keyframes da tabela de motion.
3. Home (v0.7) — é onde o Capture vive e onde a linguagem se prova.
4. Inbox, incluindo o bloco de interpretação do Hermes (visual pronto, mesmo que a interpretação ainda não exista).
5. Command + Quick Capture + recibo/undo.
6. Tasks, Projects, Apps, Library.
7. Símbolo e ícones rasterizados; conferir 16px real.
8. Light mode (paridade) e `prefers-reduced-motion`.
9. Só então a Home v0.6 em widgets, se ainda fizer sentido.

## O que não fazer

- Não introduzir biblioteca de UI, de estilo ou de animação.
- Não trocar o kanban, o drag ou qualquer chamada de `api.ts` por causa de visual.
- Não adicionar gradiente, glow, glassmorphism, orbe, partícula, emoji ou card genérico de assistente.
- Não usar mais de duas famílias tipográficas nem peso 600.
- Não usar cor para hierarquia: hierarquia é tipografia, spacing e a barra.
- Não transformar o Hermes em chat colado num canto: ele é camada, não destino.
- Não escalar um SVG do símbolo entre tamanhos.

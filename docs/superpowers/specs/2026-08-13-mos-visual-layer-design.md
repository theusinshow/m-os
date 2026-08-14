# Spec A — camada visual e estrutural

Data: 2026-08-13
Mãe: `2026-08-13-mos-v03-design.md`

---

## 0. Fonte de verdade

Valores visuais — cor, spacing, tipografia, duração, dimensão de cada tela — **não são
repetidos aqui**. Eles vivem em `Design System/design_handoff_frontend/README.md` e
`mos-tokens.css`, e duplicá-los criaria duas verdades que divergem na primeira correção.

Esta spec cobre o que o handoff não cobre: as mudanças de back-end que o desenho exige,
o plano por arquivo, e as decisões onde o handoff é omisso ou conflita com o app.

Precedência, do handoff: `README.md` > `mos-design-system.md` > `mos-tokens.css`.

---

## 1. Tokens (etapa bloqueante)

O app hoje importa `Design System/handoff/mos-tokens.css` — pasta que foi substituída
pelo pacote novo. **O build está quebrado** (`main.tsx:7`). Primeira coisa a consertar.

O arquivo antigo era uma adaptação: trazia tokens que o `App.css` usa e o pacote novo
não define. Adotar o novo cru quebraria o CSS inteiro. O arquivo final é uma mescla:

**Do pacote novo, canônico:** todos os valores de cor, spacing, geometria, motion, e os
`--tracking-*` reais (hoje zerados no app).

**Preservado da adaptação anterior, num bloco marcado `/* extensões de implementação */`:**
`--line`, `--marker`, `--focus-ring`, `--target-min`, `--content-max`, `--column-min`,
`--list-pane-width`, `--z-rail`, `--z-drawer`, `--z-backdrop`, `--z-overlay`,
`--z-receipt`, `--overlay-backdrop`, `--border-control`, e o bloco
`@media (forced-colors: active)`.

Extensão de token é permitida e deve ser explícita — é o que `DECISIONS.md:448` já exige.

**Removido:** o `@import` do Google Fonts na linha 8. O README manda empacotar fonte
local e o app já faz isso via Fontsource em `main.tsx`. Manter o `@import` colocaria o
desktop dependente de rede para renderizar texto — exatamente o que o handoff proíbe.

**Focus** passa a ser `border-color: var(--signal-ink)` + `box-shadow: 0 0 0 3px
var(--signal-ring)`, sem `outline`, idêntico em todo o sistema.

---

## 2. Back-end — migration de schema 6 → 7

Um único passo em `crates/mos-storage-sqlite`: `MIGRATION_007`, acrescentado à cadeia
existente com o mesmo padrão (`if current <= 6 { … }`).

Proteção de dados já é automática: `migrate` chama `create_pre_migration_snapshot`
(`lib.rs:188`) antes de aplicar qualquer passo, gravando `pre-migration-v6-<ts>.db` e
verificando a integridade do snapshot. Nada de novo precisa ser construído para isso.

### 2.1 `TaskState` — 3 → 6 estados

`crates/mos-core/src/work.rs`. Ordem canônica, que é a ordem das colunas:

```
inbox · backlog · planned · doing · review · done
```

Valores existentes (`backlog`, `doing`, `done`) permanecem válidos e **não são
reescritos** — a migration só amplia o domínio aceito. Transição é livre entre quaisquer
estados: o kanban permite arrastar qualquer card para qualquer coluna, e inventar
restrição aqui quebraria o gesto.

`completed_at` continua exclusivo de `done`: entrar em `done` carimba, sair limpa. Essa
regra já existe e não muda.

Comentário obrigatório no enum: `Task.state = "inbox"` não é a Inbox de Captures.

### 2.2 `ResourceKind` — 1 → 4 variantes

`crates/mos-core/src/resource.rs`. Os filtros da Library exigem tipo:

```
site · library · image · note
```

Migration: todo `link` existente vira `site`. Nenhum dado se perde — `link` era o único
valor possível e sempre significou "endereço na web".

`NewResource::create_link` vira `NewResource::create(kind, …)`, porque a validação atual
(`validate_resource_url`, que exige `http://` ou `https://`) **não pode valer para
`note`** — uma nota não tem URL. Regra por variante:

| kind | url |
|---|---|
| `site`, `library` | obrigatória, validada como hoje |
| `image` | obrigatória; aceita `http(s)://` ou caminho local |
| `note` | vazia; o conteúdo vive em `note` |

`create_link` permanece como atalho para `create(Site, …)`, para não quebrar chamadas.

### 2.3 `RegisteredApp` — capacidades

Quatro colunas booleanas: `can_open`, `can_read`, `can_write`, `can_automate`.
Default de todas em migration: `false`, exceto `can_open`, que vira `true` para apps que
já têm `launch_target` — porque abrir é o que eles comprovadamente já fazem.

O rodapé do painel no desenho não é decoração: *"Capacidade não declarada é capacidade
que o Hermes não tenta usar."* Ou seja, esses campos são contrato futuro, não rótulo.

### 2.4 `Project` — repositório

Uma coluna `repository TEXT NOT NULL DEFAULT ''`. Vazio significa sem repositório.
Sem API, sem token, sem sincronização. A única ação é abrir no navegador, e ela reusa o
caminho de abertura externa que `open_resource` já tem.

### 2.5 Fronteira

Comandos Tauri em `src-tauri/src/lib.rs` e `api.ts` estendidos junto. As assinaturas
mudam — o handoff proíbe, a decisão do proprietário sobrepõe, e o motivo está na
spec-mãe §4.1.

---

## 3. Shell

**Rail, 52px.** Símbolo no topo (não clicável — é assinatura, não destino), seis
destinos, e no rodapé Quick Capture + Settings.

Os seis: `home · inbox · board · projects · library · apps`. **Workspaces sai do rail** e
passa a ser alcançável pelo Command. A página continua existindo inteira; só perde o
ícone.

**Topbar, 44px.** Gatilho do Command à esquerda. À direita, o slot de estado de sistema:
quando ocupado, a barra girando (`barSpin`) + `SINCRONIZANDO`; quando não, o meta da
página. Na Spec B este mesmo slot passa a mostrar o estado da conexão com o Hermes.

**Caminho de contexto** (`M / INBOX`) substitui título de página e breadcrumb em todas as
telas.

---

## 4. Telas

Seis, conforme §"Screens / Views" do README. Três pontos onde o app e o desenho divergem
e a spec decide:

**Inbox — bloco de interpretação do Hermes.** O desenho mostra a Capture já interpretada
em tokens corrigíveis. Interpretação é Fase 3 da integração e **não existe na Fase 1**.
O bloco é construído com a moldura visual correta e nasce em **estado vazio honesto**,
dizendo que a interpretação ainda não está ligada. Não se fabrica interpretação falsa
para a tela parecer completa.

**Library — o motivo.** O desenho exige, sob cada tile, *"o motivo pelo qual foi salvo"*,
e afirma que ele nunca é omitido. Isso mapeia no campo `note`, que já existe. Resource
sem `note` mostra estado vazio que convida a preencher — não uma linha em branco.

**Tasks — seis colunas.** `DOING` é a única com rótulo e régua em sódio. Drag continua o
que já é; `J/K` continua valendo; movimento entre colunas usa FLIP de 180ms.

---

## 5. Overlays e estado

Command (720), Quick Capture (640), recibo de undo. O recibo hoje vive 8s; o desenho diz
~5s. Passa a 5s.

Dois estados novos no `App.tsx`, ambos citados pelo README:

- `busy` global, alimentando o indicador da topbar. Hoje existe implícito nas chamadas de
  `api.ts` e vira explícito.
- `savedIds`, um `Set` efêmero, para dar `savedWash` na row recém-criada — deixando a
  criação visível sem toast adicional.

Nada além disso muda em gerência de estado.

---

## 6. Símbolo e ícones

Três SVGs distintos, um por faixa de escala, com os ângulos corrigidos (22° / 18° / 14°)
conforme o README. **Nunca escalar um único SVG.** Rasterizar para `src-tauri/icons/`
nos tamanhos do Tauri, e conferir 16px real na taskbar, no tray e no favicon antes de
fechar — o desenho de 14° é o que precisa sobreviver ali.

Os paths de `Icon.tsx` já são os desenhos aprovados e ficam verbatim. A única correção é
garantir `stroke-linecap: butt` (terminais retos) e um desenho por tamanho.

---

## 7. Light mode e reduced motion

Light é **paridade, não inversão**. Nunca `filter: invert`, nunca derivar um modo do
outro. A única regra que muda de comportamento: âmbar puro não é tinta no light
(`--signal-ink: #8A6A12`).

`prefers-reduced-motion` já é tratado no `mos-tokens.css` (durações zeradas, `animation`
morta). A obrigação do código é não escrever transform animado fora da tabela de motion.

---

## 8. Verificação

Além do CI da spec-mãe:

- migration testada nos dois sentidos que importam: banco vazio e banco com dados de
  schema 6 reais;
- os cinco estados de cada componente conferidos nos dois temas;
- `#[0-9a-fA-F]{3,6}` em arquivos de UI = zero;
- navegação por teclado em 840×600, que é o piso que `TECHNICAL-FOUNDATION-V0.2-RESOURCES.md`
  já usou como critério.

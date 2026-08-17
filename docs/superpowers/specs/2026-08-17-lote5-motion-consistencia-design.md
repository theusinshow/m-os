# Lote 5 — Motion e consistência transversal — Design

**Status:** implementado e validado

**Data:** 2026-08-17

**Baseline:** M/OS `v0.2.11` + Lotes 0–4F já implementados (ver `docs/UI-UX-REFINEMENT.md` §18–27)

**Origem:** `docs/ROADMAP.md` §25, "Próximos lotes priorizados", item 2 ("Lote 5"); escopo original também descrito em `docs/UI-UX-REFINEMENT.md` §7.6, §7.7, §8 e como "Lote 7" em §14.

## 1. Objetivo

Consolidar as transições de page, Inspector, popover/menu, selected, saving e loading em uma linguagem única, sobre as superfícies já refinadas nos Lotes 4A–4F. Verificar `prefers-reduced-motion`, teclado, `forced-colors` e performance nessas transições. Não é um redesenho: nenhuma regra de negócio, API, banco, schema ou contrato de domínio muda.

## 2. Escopo

**Dentro:** Inbox, Tasks, Projects, Library, Calendar, Workspaces, Apps, Settings, mais os componentes compartilhados `Inspector` e `ActionMenu` (`apps/desktop/src/Surface.tsx`), o contêiner de página (`page-surface` em `App.tsx`/`App.css`), e os pontos de saving/loading hoje ad hoc: Capture inline, Quick Capture, formulários de Task/Project/Resource/Workspace/App, Settings, boot e sync indicator.

**Fora:** Tempo (aguarda lote estrutural próprio — tipografia, cardização e banners ainda divergem do resto do produto); Hermes 3B (aguarda conexão real); qualquer mudança de arquitetura de informação (decision gates do §15 de `UI-UX-REFINEMENT.md` continuam valendo).

## 3. Dependência nova: Framer Motion

Desvio explícito da diretriz "sem framework de motion externo" descrita em `UI-UX-REFINEMENT.md` §7.6/§13. Motivo: `AnimatePresence` resolve exit-animation declarativa (hoje inexistente no Inspector e incompleta no ActionMenu) sem reimplementar um hook de delayed-unmount à mão em múltiplos componentes.

Limites obrigatórios:

- uso restrito a `AnimatePresence` para orquestrar unmount/exit;
- proibido: prop `layout`, `LayoutGroup`, `useAnimate` com layout measurement, `whileHover`/`whileTap` substituindo CSS existente;
- entradas continuam via CSS/keyframes sobre os tokens `--dur-*`/`--ease-*` já existentes em `packages/design-system/tokens.css`; Framer decide apenas "quando desmontar";
- `prefers-reduced-motion` para os trechos que usam Framer é lido via `useReducedMotion()` da própria lib, não reimplementado.

**Gate de entrada do lote:** antes de aplicar a qualquer outra superfície, um spike isolado no `Inspector` deve rodar no cliente Tauri real e não produzir warnings de `ResizeObserver` no console/DevTools (risco concreto: o Lote 0 removeu uma animação de layout por esse mesmo motivo). Se o spike falhar, cai para CSS-only (hook de delayed-unmount manual) sem Framer Motion, e a spec é revisada.

## 4. Contratos de motion

### Page transition (`page-surface` / `viewEnter`)

Já existe: 160ms enter (`--dur-enter`), reaproveitado a 220ms (`--dur-context`) para a troca Command↔Hermes. Ação: nenhuma mudança de comportamento — apenas documentar formalmente como o contrato de "troca de superfície raiz". Sem exit-animation (a página de saída desmonta sem crossfade).

### Inspector (maior lacuna: hoje `display:none`/`flex` instantâneo)

- Entra: `opacity 0→1` + `translateY(4–8px)→0`, `--dur-enter` (160ms), `--ease-enter` — conforme já especificado no contrato de Inspector em `UI-UX-REFINEMENT.md` §10.
- Sai: `AnimatePresence`, inverso da entrada, `--dur-exit` (90ms), `--ease-exit`.
- Abaixo de 960px (pane única): mesmo contrato — é o mesmo componente, não um comportamento novo.
- Teclado: `Esc`/"Voltar à lista" devolve foco à lista imediatamente (não espera a exit-animation terminar); a decisão final sobre bloquear ou não input durante a transição é validada no spike.

### ActionMenu (falta exit-animation)

- Entra: mantém `menuEnter` (140ms) já existente.
- Sai: hoje é `removeAttribute("open")` instantâneo; passa a ter exit simétrico via `AnimatePresence`, `--dur-exit` (90ms), mesma família visual do Inspector (opacity + scale/translate curto).
- Task drawer (`drawer-in`/`overlayEnter`/`overlayExit`/`scrimExit`): unificar sob o mesmo vocabulário de tokens (`--dur-enter`/`--dur-exit`) sem obrigatoriamente fundir os keyframes — o backdrop/scrim tem necessidade visual própria, diferente de menu/inspector.

### Selected state

Já resolvido nos lotes 0–4F (surface + indicador posicional, não só accent). Este lote apenas **verifica** consistência entre Command, Inbox, Tasks, Projects, Library, Calendar, Workspaces, Apps e rail — corrige divergências pontuais encontradas, sem redesenhar.

## 5. Primitive `StateMessage`

Componente único cobrindo `empty | loading | error | saving | saved`.

API mínima: `state`, `label` (curto), `detail?` (technical/expandable — reaproveita o padrão "DETALHES TÉCNICOS" já usado no Hermes indisponível, Lote 3A), `aria-live="polite"` embutido.

Motion: entra com `--dur-state` (140ms), fade/shift pequeno; `saved` reaproveita a animação `savedWash` (900ms) já existente em `.data-row[data-saved]` em vez de criar uma nova.

Migração: substitui as implementações locais atuais listadas abaixo, preservando o texto/rótulo de cada uma — só a casca visual e o timing viram compartilhados. Nenhuma máquina de estado de domínio é alterada.

| Superfície | Estado atual (arquivo) |
|---|---|
| Capture inline | `App.tsx` (~L186–228), `state: idle\|saving\|success\|error` |
| Quick Capture overlay | `App.tsx` (~L2215–2229), `state: idle\|saving\|error` |
| Task / Project / Resource / Workspace / App forms | `App.tsx` (~L447, 1127, 1416, 1895), booleans locais `saving`/`pending` |
| Settings | `App.tsx` (~L2088, 2210), `settings-message` + `updateState` machine |
| Boot | `App.tsx` (~L2268, 2535–2540), `bootState: loading\|ready\|error` |
| Sync indicator | `App.tsx` (~L2547), `data-busy` + spinner |

Boot e sync indicator são os casos mais distintos. Se algum não couber de forma limpa no primitive sem forçar a API, documentar como exceção justificada em vez de espremer.

## 6. Consolidação reduced-motion / forced-colors

Hoje existem 5 blocos redundantes de `prefers-reduced-motion` e 4 de `forced-colors` espalhados em `apps/desktop/src/App.css`, além da regra global em `packages/design-system/tokens.css` (L269–281 e L286–313). Ação:

- remover os blocos locais redundantes; a regra global em `tokens.css` passa a ser a única fonte;
- exceções reais (ex.: a camada decorativa "CAMADA DE ACABAMENTO", `App.css` ~L5515–5545) viram comentário explícito documentando a razão, em vez de bloco solto;
- nenhuma mudança de comportamento visual esperada — é remoção de duplicação, não redesenho;
- a cópia duplicada de tokens em `Design System/design_handoff_frontend/mos-tokens.css` (drift risk identificado na pesquisa) é sincronizada ou removida, a decidir no plano de implementação.

## 7. QA / gate de conclusão

Herda o checklist de `UI-UX-REFINEMENT.md` §16:

- `npm run build`, `npm test -- --run`, detector Impeccable, `git diff --check`;
- inspeção visual real no cliente Tauri — Dark: 840×600, 1280×800, 1440×900, 1920×1080; Light onde selection/surface mudarem;
- teclado: Inspector abre/fecha sem perder foco; ActionMenu com exit-animation não deixa foco órfão; `Esc` funcional durante a transição;
- `prefers-reduced-motion`: zera todas as durações, incluindo os trechos com Framer Motion (via `useReducedMotion()`);
- `forced-colors`: nenhuma regressão nos três componentes que ganham motion novo (Inspector, ActionMenu, StateMessage);
- performance: sem jank perceptível ao abrir/fechar Inspector repetidamente; sem warnings de `ResizeObserver` no console;
- nenhuma regra de negócio, API, banco ou contrato de domínio alterado.

## 8. Fora de escopo / decision gates

- Tempo permanece fora até seu lote estrutural próprio;
- nenhuma mudança de IA de navegação (rail, Workspaces, Tempo Projects, Apps) — decision gates do §15 de `UI-UX-REFINEMENT.md` continuam valendo;
- nenhum novo widget, estado de domínio ou capacidade de produto é criado; `StateMessage` é puramente apresentacional.

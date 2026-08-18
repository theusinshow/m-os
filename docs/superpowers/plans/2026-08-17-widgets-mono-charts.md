# Widgets da Home — geometria macia, moldura e formas novas — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar aos 15 widgets da Home moldura de card, geometria arredondada e quatro formas de gráfico novas, sem inventar nenhum dado que o M/OS já não mostre hoje.

**Architecture:** A aritmética das formas vive num módulo puro (`plotGeometry.ts`) coberto por testes de nó; os primitivos SVG (`Plot.tsx`) só desenham o que esse módulo calcula. A moldura nasce em CSS escopado a `.home-grid .widget`, então chega aos quinze sem tocar JSX e sem alcançar o `Panel`, que é compartilhado com Settings, Workspaces e Tempo.

**Tech Stack:** React 19 + TypeScript, Vite, Vitest (`environment: "node"`), CSS puro (`App.css` + `packages/design-system/*.css`), Tauri 2.

**Spec:** `docs/superpowers/specs/2026-08-17-widgets-mono-charts-design.md`

## Global Constraints

- **Sem teste de DOM.** `apps/desktop/vitest.config.ts` roda só `src/**/*.test.ts` com `environment: "node"`, e o comentário do próprio arquivo explica por quê: *"pior que não ter teste é ter teste que mente"*. Nenhuma tarefa cria `*.test.tsx` nem instala `@testing-library/react`. Verificação de componente é `npm run build` (tsc + vite) e inspeção visual no cliente Tauri real.
- **A ADR-040 vem primeiro.** Sem ela, o código contradiz uma ADR aceita — exatamente a "mudança silenciosa de IA" que `UI-UX-REFINEMENT.md` §15 proíbe. Task 1 é pré-requisito das demais.
- **Nenhum dado novo.** Toda medida exibida já está na tela hoje. Nenhuma regra de negócio, API, banco, schema, rota ou contrato de domínio é alterado.
- **O `Panel` não é tocado.** Ele é usado em 3 seções de Settings, 5 painéis do Inspector de Workspaces e 2 lugares do Tempo.
- **`--radius: 3px` permanece** o padrão do sistema. O raio grande é autorizado só dentro da moldura de widget.
- **O sódio continua reservado para carga.** `--signal-fill` não entra em eixo, trilho, grade ou rótulo. Agora/hoje seguem em traço branco de 2px.
- **Orçamento de movimento da ADR-034:** um loop por tela (o cronômetro, que já existe), movimento que carrega dado, cascata de 40ms com teto de 8, `reduced-motion` nascendo no valor final.
- **Commits em português**, curtos, no estilo do repo. Nunca `--no-verify`.

---

### Task 1: ADR-040 — a decisão que autoriza o resto

**Files:**
- Modify: `docs/DECISIONS.md` (acrescentar ao final, depois da ADR-039)

**Interfaces:**
- Consumes: nada.
- Produces: a autorização documentada que as Tasks 3, 5 e 6 citam na mensagem de commit.

- [ ] **Step 1: Localizar o fim da ADR-039**

Run: `grep -n "^## ADR-039" docs/DECISIONS.md` e ir até o fim do arquivo. A ADR-039 termina com uma lista de "Consequências".

- [ ] **Step 2: Escrever a ADR-040 ao final do arquivo**

```markdown

## ADR-040 — A ponta arredondada entra compensada, e a moldura entra só na Home

**Data:** 2026-08-17
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-034

### Contexto

O proprietário trouxe `https://amicro.vercel.app/mono-charts` como referência
para os widgets: 30 visualizadores monocromáticos construídos sobre geometria
arredondada, cada um num card com superfície aninhada e rodapé de metas.

A ADR-034 fixou o contrário em dois pontos. Primeiro, "ponta reta, sempre",
com a justificativa de que "cap arredondado mente sobre o valor em anéis
pequenos". Segundo, a Home nunca teve moldura de card — o `Panel` é rótulo e
ar, e a nota em `Surface.tsx` registra que "card é a resposta preguiçosa".

Os dois pontos foram reafirmados pelo proprietário depois de a colisão ser
apontada.

### Decisão

**1. A ponta passa a ser arredondada, com compensação aritmética.**

A regra antiga estava certa sobre o problema e o resolvia proibindo. A nova
resolve compensando: desenha-se `L' = max(ε, L − espessura)`, de modo que a
extensão *pintada* — que o cap estende em meia espessura por ponta — volte a
ser exatamente `L`.

O erro que a regra antiga evitava, medido nos tamanhos da própria família:
2,3 pontos percentuais no anel de 88px, 3,4 no de 44px e 6,9 no de 14px. É o
último que justifica a proibição ter sido escrita, e é ele que a compensação
zera.

O limite fica declarado em vez de escondido: abaixo de uma espessura de traço,
`L'` cai no piso e o cap pinta um disco. Ali o anel **para de medir e passa a
afirmar presença** — "existe algo, menor que o menor traço que este anel sabe
desenhar". Zero continua não desenhando nada.

E uma distinção que a ADR-034 não precisava fazer, porque não havia retângulos
na família: **`rx` não mente, `linecap` mente**. O canto arredondado de um
`rect` arredonda para dentro da geometria e a barra mantém a altura exata do
valor; a ponta arredondada de um traço estende para fora. Só a segunda é
compensada, e é por isso que as formas retangulares novas não precisam de
correção nenhuma.

**2. A moldura de card entra, e só na Home.**

Os 15 widgets da Home ganham moldura, superfície aninhada para a forma e
rodapé de metas. A reversão da posição anti-cardização vale **apenas nesse
escopo**: o `Panel` sem moldura continua sendo a resposta em Settings, no
Inspector de Workspaces e no Tempo, e a nota do `Surface.tsx` segue valendo
para o resto do sistema. A regra é escopada a `.home-grid .widget` justamente
para não poder vazar.

**3. O raio ganha charter novo, sem mexer no padrão.**

`--radius-widget: 12px` para a moldura externa e `--radius-lg: 8px` — que era
reservado a "somente app icon e overlay grande" — liberado também para a
superfície aninhada de widget. `--radius: 3px` continua valendo para botão,
campo, linha e todo o resto. Subir o raio global foi considerado e recusado:
vazaria a maciez para o sistema inteiro sem ninguém ter pedido.

### O que foi recusado

**A paleta monocromática da referência.** O sódio continua reservado para
carga e agora/hoje continuam traço branco de 2px. Metade do charme da
referência vem do cinza puro, e a recusa precisa estar escrita para que quem
reabrir o assunto encontre uma decisão em vez de supor esquecimento.

**As formas sem domínio** — candlestick, Sankey, pirâmide, scatter, donut de
quatro fatias. A razão é a da própria ADR-034: "um anel bonito preenchido com
número inventado é pior que a ausência".

### Consequências

- a família de widgets ganha uma terceira classe ao lado do anel e da
  densidade: as formas de plot (`Bars`, `Stack`, `Bullet`, `Spark`), todas
  sobre dado que já estava na tela;
- o `Bullet` resolve uma limitação que estava escrita no código do
  `BudgetRing` — o anel parava em cheio e o estouro da meta só existia no
  texto;
- a compensação vira responsabilidade de um módulo puro e testado, e não de
  cada chamador;
- o risco assumido: uma linguagem visual macia é mais fácil de esticar para
  onde não foi decidida. A defesa é o escopo `.home-grid`, que faz o
  vazamento exigir uma edição deliberada em vez de acontecer por herança.
```

- [ ] **Step 3: Commit**

```bash
git add docs/DECISIONS.md
git commit -m "docs: registra ADR-040, ponta arredondada compensada e moldura na Home"
```

---

### Task 2: `plotGeometry.ts` — a aritmética, com testes

**Files:**
- Create: `apps/desktop/src/plotGeometry.ts`
- Test: `apps/desktop/src/plotGeometry.test.ts`

**Interfaces:**
- Consumes: nada.
- Produces:
  - `MIN_DASH: number`
  - `compensatedLength(desired: number, stroke: number): number`
  - `type Rect = { x: number; y: number; width: number; height: number }`
  - `barRects(ratios: number[], options: { width: number; height: number; gap: number }): Rect[]`
  - `type StackSegment = { index: number; x: number; width: number }`
  - `stackSegments(values: number[], options: { width: number; gap: number }): StackSegment[]`
  - `bulletGeometry(value: number, target: number, width: number): { fill: number; mark: number; over: boolean }`
  - `sparkPath(ratios: number[], options: { width: number; height: number; inset: number }): string`

- [ ] **Step 1: Escrever os testes que falham**

Create `apps/desktop/src/plotGeometry.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { barRects, bulletGeometry, compensatedLength, MIN_DASH, sparkPath, stackSegments } from "./plotGeometry";

describe("compensatedLength", () => {
  it("desconta uma espessura inteira, meia por ponta", () => {
    expect(compensatedLength(100, 6)).toBe(94);
  });

  it("zero continua nao desenhando nada", () => {
    expect(compensatedLength(0, 6)).toBe(0);
    expect(compensatedLength(-5, 6)).toBe(0);
  });

  it("abaixo de uma espessura cai no piso e vira disco", () => {
    expect(compensatedLength(3, 6)).toBe(MIN_DASH);
    expect(compensatedLength(6, 6)).toBe(MIN_DASH);
  });
});

describe("barRects", () => {
  it("divide a largura em barras iguais com os vaos entre elas", () => {
    const rects = barRects([1, 1, 1, 1, 1, 1, 1], { width: 140, height: 60, gap: 4 });
    expect(rects).toHaveLength(7);
    expect(rects[0].x).toBe(0);
    expect(rects[0].width).toBeCloseTo(16.571, 3);
    // A ultima barra encosta exatamente na borda direita.
    expect(rects[6].x + rects[6].width).toBeCloseTo(140, 6);
  });

  it("assenta a barra na linha de base", () => {
    const [rect] = barRects([0.5], { width: 20, height: 60, gap: 4 });
    expect(rect.height).toBe(30);
    expect(rect.y).toBe(30);
  });

  it("zero devolve altura zero, para o chamador nao desenhar", () => {
    const [rect] = barRects([0], { width: 20, height: 60, gap: 4 });
    expect(rect.height).toBe(0);
  });

  it("nao inventa altura minima: o rx da conta de arredondar barra baixa", () => {
    const [rect] = barRects([0.01], { width: 20, height: 60, gap: 4 });
    expect(rect.height).toBeCloseTo(0.6, 6);
  });
});

describe("stackSegments", () => {
  it("reparte a largura na proporcao dos valores, descontando os vaos", () => {
    const segments = stackSegments([3, 1], { width: 100, gap: 4 });
    expect(segments).toHaveLength(2);
    expect(segments[0].x).toBe(0);
    expect(segments[0].width).toBeCloseTo(72, 6);
    expect(segments[1].x).toBeCloseTo(76, 6);
    expect(segments[1].width).toBeCloseTo(24, 6);
  });

  it("pula os zeros e nao gasta vao com eles", () => {
    const segments = stackSegments([1, 0, 1], { width: 100, gap: 4 });
    expect(segments.map((segment) => segment.index)).toEqual([0, 2]);
    expect(segments[0].width).toBeCloseTo(48, 6);
  });

  it("sem total devolve vazio", () => {
    expect(stackSegments([0, 0], { width: 100, gap: 4 })).toEqual([]);
  });
});

describe("bulletGeometry", () => {
  it("abaixo da meta, a marca fica no fim e a barra e proporcional", () => {
    const geometry = bulletGeometry(30, 40, 100);
    expect(geometry.fill).toBeCloseTo(75, 6);
    expect(geometry.mark).toBeCloseTo(100, 6);
    expect(geometry.over).toBe(false);
  });

  it("acima da meta, a barra vai ao fim e a marca recua para dentro", () => {
    const geometry = bulletGeometry(50, 40, 100);
    expect(geometry.fill).toBeCloseTo(100, 6);
    expect(geometry.mark).toBeCloseTo(80, 6);
    expect(geometry.over).toBe(true);
  });

  it("sem meta e sem valor nao desenha nada", () => {
    expect(bulletGeometry(0, 0, 100)).toEqual({ fill: 0, mark: 0, over: false });
  });
});

describe("sparkPath", () => {
  it("desenha do canto inferior esquerdo ao superior direito, respeitando o inset", () => {
    expect(sparkPath([0, 1], { width: 100, height: 20, inset: 2 })).toBe("M2.00 18.00 L98.00 2.00");
  });

  it("com menos de dois pontos nao ha linha", () => {
    expect(sparkPath([1], { width: 100, height: 20, inset: 2 })).toBe("");
  });
});
```

- [ ] **Step 2: Rodar os testes para ver falhar**

Run: `cd apps/desktop && npx vitest run src/plotGeometry.test.ts`
Expected: FAIL — `Failed to resolve import "./plotGeometry"`.

- [ ] **Step 3: Escrever o módulo**

Create `apps/desktop/src/plotGeometry.ts`:

```ts
/**
 * A aritmética das formas de plot — `ADR-040`.
 *
 * Vive separada do SVG por um motivo: `vitest.config.ts` roda só funções puras
 * em ambiente de nó, e é aqui que mora o que pode mentir sobre um valor. O
 * `Plot.tsx` desenha o que este arquivo calcula, e não calcula nada por conta.
 *
 * A distinção que organiza tudo: **`rx` não mente, `linecap` mente**. O canto
 * arredondado de um `rect` arredonda para DENTRO da geometria e a barra mantém
 * a altura exata do valor. A ponta arredondada de um traço estende para FORA,
 * meia espessura por ponta — e só ela precisa de compensação.
 */

/** Piso do traço compensado. Um dash quase-zero com cap redondo pinta o disco
 *  de forma determinística; zero puro fica a critério do renderizador. */
export const MIN_DASH = 0.01;

/**
 * Comprimento a DESENHAR para que o PINTADO seja `desired`, com cap redondo.
 *
 * Abaixo de uma espessura o resultado cai no piso, e o que aparece é um disco
 * do diâmetro do traço: o anel para de medir e passa a afirmar presença. Zero
 * continua não desenhando nada, que é a regra herdada da ADR-034.
 */
export function compensatedLength(desired: number, stroke: number) {
  if (desired <= 0) return 0;
  return Math.max(MIN_DASH, desired - stroke);
}

export type Rect = { x: number; y: number; width: number; height: number };

/**
 * Barras de largura igual, assentadas na linha de base.
 *
 * Sem altura mínima de propósito: o `rx` do `rect` é limitado pela própria
 * altura, então uma barra baixa sai arredondada sem que ninguém precise
 * inflá-la. Altura zero volta zero, e quem desenha decide não desenhar.
 */
export function barRects(ratios: number[], options: { width: number; height: number; gap: number }): Rect[] {
  const { width, height, gap } = options;
  const count = ratios.length;
  if (count === 0) return [];

  const barWidth = Math.max(0, (width - gap * (count - 1)) / count);
  return ratios.map((ratio, index) => {
    const clamped = Math.max(0, Math.min(1, ratio));
    const barHeight = clamped * height;
    return {
      x: index * (barWidth + gap),
      y: height - barHeight,
      width: barWidth,
      height: barHeight,
    };
  });
}

export type StackSegment = { index: number; x: number; width: number };

/**
 * Uma barra repartida na proporção dos valores.
 *
 * Os vãos só são descontados entre segmentos que existem: um valor zero não
 * ocupa lugar nem deixa buraco, senão a soma das partes não fecharia a largura.
 */
export function stackSegments(values: number[], options: { width: number; gap: number }): StackSegment[] {
  const { width, gap } = options;
  const positive = values.map((value) => Math.max(0, value));
  const total = positive.reduce((sum, value) => sum + value, 0);
  if (total <= 0) return [];

  const visible = positive.filter((value) => value > 0).length;
  const available = Math.max(0, width - gap * Math.max(0, visible - 1));

  const segments: StackSegment[] = [];
  let cursor = 0;
  positive.forEach((value, index) => {
    if (value <= 0) return;
    const segmentWidth = (value / total) * available;
    segments.push({ index, x: cursor, width: segmentWidth });
    cursor += segmentWidth + gap;
  });
  return segments;
}

/**
 * Valor contra meta, numa régua só.
 *
 * A escala é o maior dos dois, e é isso que deixa o estouro ser DESENHADO: o
 * anel do `BudgetRing` parava em cheio e dizia o excesso só no texto, porque
 * uma segunda volta se leria como "começou de novo". Aqui a barra vai ao fim e
 * é a marca da meta que recua para dentro.
 */
export function bulletGeometry(value: number, target: number, width: number) {
  const scale = Math.max(Math.max(0, value), Math.max(0, target));
  if (scale <= 0) return { fill: 0, mark: 0, over: false };
  return {
    fill: (Math.max(0, value) / scale) * width,
    mark: (Math.max(0, target) / scale) * width,
    over: value > target,
  };
}

/**
 * O `d` de uma polilinha, do mais antigo à esquerda ao mais novo à direita.
 *
 * O `inset` existe para o cap redondo não ser cortado pela borda do viewBox:
 * ele estende meia espessura além do ponto final.
 */
export function sparkPath(ratios: number[], options: { width: number; height: number; inset: number }) {
  const { width, height, inset } = options;
  if (ratios.length < 2) return "";

  const usableWidth = Math.max(0, width - inset * 2);
  const usableHeight = Math.max(0, height - inset * 2);
  const step = usableWidth / (ratios.length - 1);

  return ratios
    .map((ratio, index) => {
      const clamped = Math.max(0, Math.min(1, ratio));
      const x = inset + index * step;
      const y = inset + usableHeight - clamped * usableHeight;
      return `${index === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}
```

- [ ] **Step 4: Rodar os testes para ver passar**

Run: `cd apps/desktop && npx vitest run src/plotGeometry.test.ts`
Expected: PASS — 14 testes.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/plotGeometry.ts apps/desktop/src/plotGeometry.test.ts
git commit -m "feat(widgets): adiciona a aritmetica das formas de plot"
```

---

### Task 3: A moldura, e os raios que ela usa

**Files:**
- Modify: `packages/design-system/tokens.css` (bloco de raio, ~linha 54)
- Modify: `apps/desktop/src/App.css` (regra `.widget`, ~linha 800)

**Interfaces:**
- Consumes: a autorização da ADR-040 (Task 1).
- Produces: `--radius-widget`, a classe `.widget-plot` e a moldura em `.home-grid .widget`, consumidas pelas Tasks 4 a 9.

- [ ] **Step 1: Acrescentar o token de raio**

Em `packages/design-system/tokens.css`, no bloco onde `--radius-sm`, `--radius` e `--radius-lg` são declarados, acrescentar depois de `--radius-lg` e ajustar o comentário dele:

```css
  --radius-lg: 8px;   /* app icon, overlay grande e superfície aninhada de widget (ADR-040) */
  --radius-widget: 12px; /* moldura de widget da Home, e nada além dela (ADR-040) */
```

- [ ] **Step 2: Escrever a moldura**

Em `apps/desktop/src/App.css`, logo depois do bloco `.widget { min-width: 0; }` e das regras `[data-span]`:

```css
/* ---------- Moldura de widget (ADR-040) ----------
   Escopada a `.home-grid` de propósito: o `Panel` é o mesmo componente usado em
   Settings, no Inspector de Workspaces e no Tempo, e a moldura não pode
   alcançá-los. Vazar daqui exige uma edição deliberada, não acontece por
   herança. */
.home-grid .widget {
  padding: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-widget);
  background: var(--surface-raised);
}

/* A superfície onde a forma é desenhada, um degrau abaixo do card.

   As duas propriedades existem nos dois temas, e muda qual faz o trabalho: no
   escuro `#101316` sobre `#171B1F` se lê pelo preenchimento; no claro
   `#FAFBFC` sobre `#FFFFFF` são 2% e é a borda que desenha o retângulo. Em
   `forced-colors` o preenchimento some e sobra a borda — o mesmo mecanismo do
   claro, e por isso não há exceção a escrever. */
.widget-plot {
  padding: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
}
```

- [ ] **Step 3: Verificar que compila e que nada fora da Home mudou**

Run: `cd apps/desktop && npm run build`
Expected: build limpo.

Run: `grep -c "home-grid .widget" src/App.css`
Expected: `1` — a regra existe uma vez só e é escopada.

- [ ] **Step 4: Commit**

```bash
git add packages/design-system/tokens.css apps/desktop/src/App.css
git commit -m "feat(widgets): a Home ganha moldura de card (ADR-040)"
```

---

### Task 4: Os slots de manchete e rodapé no `<Widget>`

**Files:**
- Modify: `apps/desktop/src/App.tsx:111-114` (componente `Widget`)
- Modify: `apps/desktop/src/App.css` (depois das regras da Task 3)

**Interfaces:**
- Consumes: `.home-grid .widget` (Task 3).
- Produces: `<Widget>` com as props opcionais `value?: string`, `unit?: string`, `footLeft?: string`, `footRight?: string`. As Tasks 7, 8 e 9 as preenchem.

- [ ] **Step 1: Estender o componente**

Em `apps/desktop/src/App.tsx`, substituir o componente `Widget` por:

```tsx
/* Cuida do posicionamento na grade e da moldura da ADR-040. O rótulo continua no
   Panel, para que a etapa 2 (modo de edicao) mude posicao sem tocar em nenhum
   widget.
   `hidden` devolve null: a regra de visibilidade fica num lugar so, e a grade nao
   precisa saber de nada — os widgets restantes reflowam sozinhos.

   Os quatro slots sao opcionais porque nem todo widget tem o que pôr neles. A
   manchete existe quando a FORMA nao carrega o numero — anel com `RingLabel` ja
   o tem no centro, e repetir seria o mesmo numero duas vezes no mesmo card. O
   rodape diz escala e extremo, e lista nao tem escala. */
function Widget({ id, role, span, hidden = false, value, unit, footLeft, footRight, children }: {
  id: string;
  role: HomeWidgetRole;
  span: HomeWidgetSpan;
  hidden?: boolean;
  value?: string;
  unit?: string;
  footLeft?: string;
  footRight?: string;
  children: ReactNode;
}) {
  if (hidden) return null;
  return (
    <div className="widget" data-widget={id} data-role={role} data-span={span}>
      {children}
      {value ? (
        <p className="widget-head">
          <span className="widget-value">{value}</span>
          {unit ? <span className="widget-unit">{unit}</span> : null}
        </p>
      ) : null}
      {footLeft || footRight ? (
        <p className="widget-foot">
          <span>{footLeft}</span>
          <span>{footRight}</span>
        </p>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 2: Escrever o CSS dos slots**

Em `apps/desktop/src/App.css`, depois do bloco `.widget-plot`:

```css
/* A manchete vem DEPOIS do Panel no DOM e sobe pela ordem visual: o `Panel` já
   emite o `h2` do rótulo, e um número antes dele no fluxo faria a árvore de
   acessibilidade anunciar o valor antes de dizer do que ele é. */
.home-grid .widget {
  display: flex;
  flex-direction: column;
}

.widget-head {
  order: -1;
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  margin: 0 0 var(--space-2);
}

.widget-value {
  font: var(--text-title);
  letter-spacing: var(--tracking-title);
  color: var(--text);
}

.widget-unit {
  font: var(--text-small);
  color: var(--text-secondary);
}

.widget-foot {
  display: flex;
  justify-content: space-between;
  gap: var(--space-2);
  margin: auto 0 0;
  padding-top: var(--space-3);
  border-top: 1px solid var(--border);
  font: var(--text-meta);
  letter-spacing: var(--tracking-meta);
  text-transform: uppercase;
  color: var(--text-system);
}
```

Nota sobre `order: -1`: o `Panel` traz o `h2` do rótulo e precisa vir antes do número na leitura assistiva. A ordem visual é rótulo → número → forma → rodapé; a ordem do DOM é rótulo/forma → número → rodapé, com o número reposicionado por `order`. O rodapé usa `margin-top: auto` para grudar no fim do card mesmo quando o conteúdo é curto.

- [ ] **Step 3: Verificar**

Run: `cd apps/desktop && npm run build`
Expected: build limpo. Nenhum widget passa as props ainda, então nada muda em tela.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.css
git commit -m "feat(widgets): Widget ganha slots de manchete e rodape"
```

---

### Task 5: `Plot.tsx` — os quatro primitivos

**Files:**
- Create: `apps/desktop/src/Plot.tsx`
- Modify: `packages/design-system/widgets.css` (acrescentar a família de plot ao final)

**Interfaces:**
- Consumes: `barRects`, `stackSegments`, `bulletGeometry`, `sparkPath` de `./plotGeometry` (Task 2); `stagger` de `./Ring`.
- Produces:
  - `Bars({ ratios, labels, highlight }: { ratios: number[]; labels: string[]; highlight?: number })`
  - `Stack({ values, labels }: { values: number[]; labels: string[] })`
  - `Bullet({ value, target, over }: { value: number; target: number; over: boolean })`
  - `Spark({ ratios }: { ratios: number[] })`

- [ ] **Step 1: Escrever os primitivos**

Create `apps/desktop/src/Plot.tsx`:

```tsx
import { barRects, bulletGeometry, sparkPath, stackSegments } from "./plotGeometry";
import { stagger } from "./Ring";

/**
 * Família de plot — `ADR-040`.
 *
 * A terceira classe da família de widgets, ao lado do anel (proporção de uma
 * coisa só) e da densidade (tempo como área). Aqui moram as formas que comparam
 * séries: barras, empilhada, bullet e linha.
 *
 * Nenhuma delas calcula: a aritmética inteira vem de `plotGeometry.ts`, que é
 * testado. Estes componentes só transformam número em `rect` e `path`.
 *
 * As três primeiras são retângulos com `rx`, e por isso não precisam de
 * compensação — o `rx` arredonda para dentro e a barra mantém a altura exata do
 * valor. Só o `Spark`, que é traço com cap redondo, recebe `inset` para o cap
 * não ser cortado pela borda do viewBox.
 */

const VIEW = { width: 240, height: 64 };

/** Barras de pílula, uma por período. `highlight` é o índice de hoje. */
export function Bars({ ratios, labels, highlight }: { ratios: number[]; labels: string[]; highlight?: number }) {
  const rects = barRects(ratios, { width: VIEW.width, height: VIEW.height, gap: 6 });

  return (
    <div className="mos-bars">
      <svg
        className="mos-bars-figure"
        viewBox={`0 0 ${VIEW.width} ${VIEW.height}`}
        preserveAspectRatio="none"
        aria-hidden="true"
        focusable="false"
      >
        {rects.map((rect, index) => (
          <rect
            key={index}
            className="mos-bars-track"
            x={rect.x}
            y={0}
            width={rect.width}
            height={VIEW.height}
            rx={rect.width / 2}
          />
        ))}
        {/* Altura zero não desenha: a mesma regra do anel, pelo mesmo motivo —
            um retângulo de altura zero com `rx` deixa resíduo de sub-pixel. */}
        {rects.map((rect, index) =>
          rect.height > 0 ? (
            <rect
              key={index}
              className="mos-bars-value"
              data-now={index === highlight || undefined}
              x={rect.x}
              y={rect.y}
              width={rect.width}
              height={rect.height}
              rx={rect.width / 2}
              style={{ ["--ring-delay" as string]: stagger(index) }}
            />
          ) : null,
        )}
      </svg>
      <div className="mos-bars-labels">
        {labels.map((label, index) => (
          <span className="micro-label" data-today={index === highlight || undefined} key={index}>
            {label}
          </span>
        ))}
      </div>
    </div>
  );
}

/** Uma barra repartida: composição, não comparação par a par. */
export function Stack({ values, labels }: { values: number[]; labels: string[] }) {
  const segments = stackSegments(values, { width: VIEW.width, gap: 4 });

  return (
    <div className="mos-stack">
      <svg
        className="mos-stack-figure"
        viewBox={`0 0 ${VIEW.width} 16`}
        preserveAspectRatio="none"
        aria-hidden="true"
        focusable="false"
      >
        {segments.map((segment) => (
          <rect
            key={segment.index}
            className="mos-stack-value"
            /* O primeiro é o sódio cheio; os demais descem os mesmos degraus de
               profundidade que o anel usa, 55% e 30%. */
            data-depth={segment.index === 0 ? undefined : segment.index === 1 ? 2 : 3}
            x={segment.x}
            y={0}
            width={segment.width}
            height={16}
            rx={8}
            style={{ ["--ring-delay" as string]: stagger(segment.index) }}
          />
        ))}
      </svg>
      <ul className="mos-stack-legend">
        {labels.map((label, index) => (
          <li key={index}>
            <span className="mos-stack-chip" data-depth={index === 0 ? undefined : index === 1 ? 2 : 3} aria-hidden="true" />
            <span className="micro-label">{label}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Valor contra meta, com a marca da meta desenhada — inclusive no estouro. */
export function Bullet({ value, target, over }: { value: number; target: number; over: boolean }) {
  const geometry = bulletGeometry(value, target, VIEW.width);

  return (
    <svg
      className="mos-bullet"
      viewBox={`0 0 ${VIEW.width} 16`}
      preserveAspectRatio="none"
      aria-hidden="true"
      focusable="false"
    >
      <rect className="mos-bullet-track" x={0} y={0} width={VIEW.width} height={16} rx={8} />
      {geometry.fill > 0 ? (
        <rect className="mos-bullet-value" data-over={over || undefined} x={0} y={0} width={geometry.fill} height={16} rx={8} />
      ) : null}
      {/* A marca da meta é branca de 2px, como agora/hoje no resto da família:
          o sódio está reservado para carga, e meta não é carga. */}
      <rect className="mos-bullet-mark" x={Math.max(0, geometry.mark - 1)} y={-2} width={2} height={20} />
    </svg>
  );
}

/** A série, como linha. Cap redondo compensado pelo `inset`. */
export function Spark({ ratios }: { ratios: number[] }) {
  const path = sparkPath(ratios, { width: VIEW.width, height: 32, inset: 2 });
  if (!path) return null;

  return (
    <svg
      className="mos-spark"
      viewBox={`0 0 ${VIEW.width} 32`}
      preserveAspectRatio="none"
      aria-hidden="true"
      focusable="false"
    >
      <path className="mos-spark-line" d={path} />
    </svg>
  );
}
```

- [ ] **Step 2: Escrever o CSS da família**

Ao final de `packages/design-system/widgets.css`:

```css
/* ---------- Plot (ADR-040) ----------
   A terceira família, ao lado do anel e da densidade: formas que comparam uma
   série. Todas são retângulo com `rx`, exceto o `Spark`.

   `rx` arredonda para DENTRO da geometria — a barra mantém a altura exata do
   valor, e nenhuma delas precisa da compensação que o anel precisa. */

@keyframes mos-bar-grow {
  from {
    transform: scaleY(0);
  }
}

@keyframes mos-stack-grow {
  from {
    transform: scaleX(0);
  }
}

.mos-bars,
.mos-stack {
  display: grid;
  gap: var(--space-2);
}

.mos-bars-figure {
  width: 100%;
  height: 64px;
}

.mos-bars-track {
  fill: var(--surface-hover);
}

.mos-bars-value {
  fill: var(--signal-fill);
  transform-origin: bottom;
  animation: mos-bar-grow var(--dur-enter) var(--ease-enter) both;
  animation-delay: var(--ring-delay, 0ms);
}

/* Hoje é traço branco, nunca sódio mais forte: a escala de sódio já significa
   carga, e usá-la também para "agora" faria a mesma cor dizer duas coisas. */
.mos-bars-value[data-now] {
  fill: var(--text);
}

.mos-bars-labels {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  text-align: center;
}

.mos-stack-figure {
  width: 100%;
  height: 16px;
}

.mos-stack-value {
  fill: var(--signal-fill);
  transform-origin: left;
  animation: mos-stack-grow var(--dur-enter) var(--ease-enter) both;
  animation-delay: var(--ring-delay, 0ms);
}

.mos-stack-value[data-depth="2"],
.mos-stack-chip[data-depth="2"] {
  fill: color-mix(in srgb, var(--signal-fill) 55%, transparent);
  background: color-mix(in srgb, var(--signal-fill) 55%, transparent);
}

.mos-stack-value[data-depth="3"],
.mos-stack-chip[data-depth="3"] {
  fill: color-mix(in srgb, var(--signal-fill) 30%, transparent);
  background: color-mix(in srgb, var(--signal-fill) 30%, transparent);
}

.mos-stack-legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1) var(--space-3);
  margin: 0;
  padding: 0;
  list-style: none;
}

.mos-stack-legend li {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  min-width: 0;
}

.mos-stack-chip {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-sm);
  background: var(--signal-fill);
  flex: none;
}

.mos-bullet {
  width: 100%;
  height: 16px;
}

.mos-bullet-track {
  fill: var(--surface-hover);
}

.mos-bullet-value {
  fill: var(--signal-fill);
  transform-origin: left;
  animation: mos-stack-grow var(--dur-enter) var(--ease-enter) both;
}

.mos-bullet-mark {
  fill: var(--text);
}

.mos-spark {
  width: 100%;
  height: 32px;
}

.mos-spark-line {
  fill: none;
  stroke: var(--signal-fill);
  stroke-width: 2;
  /* Ponta arredondada, compensada pelo `inset` do `sparkPath`. */
  stroke-linecap: round;
  stroke-linejoin: round;
}

@media (forced-colors: active) {
  /* O preenchimento some e a forma passa a se declarar pelo contorno, que é o
     mesmo mecanismo que o tema claro usa na superfície aninhada. */
  .mos-bars-value,
  .mos-stack-value,
  .mos-bullet-value {
    fill: Highlight;
  }

  .mos-bars-track,
  .mos-bullet-track {
    fill: Canvas;
    stroke: ButtonBorder;
  }
}
```

- [ ] **Step 3: Verificar**

Run: `cd apps/desktop && npm run build`
Expected: build limpo. `Plot.tsx` compila mas ainda não é importado por ninguém — o `tsc` do repo não reclama de módulo não usado, só de símbolo não usado dentro de um arquivo.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/Plot.tsx packages/design-system/widgets.css
git commit -m "feat(widgets): adiciona a familia de plot com barras, empilhada, bullet e linha"
```

---

### Task 6: O anel ganha ponta arredondada compensada

**Files:**
- Modify: `apps/desktop/src/Ring.tsx` (bloco de comentário do topo e o `map` de segmentos)
- Modify: `packages/design-system/widgets.css` (`.mos-ring-value`, `.mos-density-cell`)

**Interfaces:**
- Consumes: `compensatedLength` de `./plotGeometry` (Task 2).
- Produces: nada de novo — muda o desenho de `Ring`, que as Tasks 7 e 8 continuam usando com a mesma assinatura.

- [ ] **Step 1: Atualizar a regra no comentário do `Ring`**

Em `apps/desktop/src/Ring.tsx`, substituir a linha da lista de regras:

```
 * - ponta reta, sempre. Cap arredondado mente sobre o valor em anéis pequenos;
```

por:

```
 * - ponta arredondada, compensada. O cap estende meia espessura por ponta, e o
 *   traço é encurtado de uma espessura inteira para o PINTADO bater com o valor
 *   (ADR-040). Abaixo de uma espessura o anel afirma presença, não mede;
```

- [ ] **Step 2: Importar e aplicar a compensação**

No topo do arquivo, junto do import de tipos:

```tsx
import { compensatedLength } from "./plotGeometry";
```

E no `map` de segmentos, substituir o cálculo do `strokeDashoffset`:

```tsx
            const drawn = Math.max(0, Math.min(1, segment.value)) * span;
            // Zero não desenha nada: um traço de comprimento zero com ponta
            // reta some, mas deixar o elemento no DOM manteria o `dasharray`
            // ativo e, com sub-pixel, o navegador pinta um ponto de sódio.
            if (drawn <= 0) return null;
            // O cap redondo pinta meia espessura além de cada ponta, então o
            // traço desenhado é encurtado de uma espessura inteira (ADR-040).
            const painted = compensatedLength(length * drawn, stroke);
```

e, no `style` do mesmo `<circle>`:

```tsx
                style={{
                  strokeDashoffset: length - painted,
                  ["--ring-circumference" as string]: `${length}`,
                  ["--ring-delay" as string]: delay,
                }}
```

- [ ] **Step 3: Trocar o cap no CSS e arredondar a densidade**

Em `packages/design-system/widgets.css`, em `.mos-ring-value`, substituir:

```css
  /* Ponta reta, sempre. Cap arredondado mente sobre o valor em anéis pequenos
     e amolece a geometria do sistema. */
  stroke-linecap: butt;
```

por:

```css
  /* Ponta arredondada, compensada em `Ring.tsx` por `compensatedLength`: o cap
     estende meia espessura por ponta, e o traço já vem encurtado disso. Sem a
     compensação o erro seria de 2,3 pp no anel de 88px e 6,9 pp no de 14px
     (ADR-040). */
  stroke-linecap: round;
```

E em `.mos-density-cell`, garantir o raio macio da referência — a célula hoje usa `--radius-sm`:

```css
  border-radius: var(--radius-sm);
```

passa a:

```css
  /* `rx` de célula arredonda para dentro e não distorce a leitura de carga. */
  border-radius: var(--radius);
```

- [ ] **Step 4: Verificar**

Run: `cd apps/desktop && npm run build && npm test -- --run`
Expected: build limpo; 3 arquivos de teste passando (`calendarDays`, `suspiciousEntry`, `plotGeometry`).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/Ring.tsx packages/design-system/widgets.css
git commit -m "feat(widgets): anel passa a ponta arredondada compensada (ADR-040)"
```

---

### Task 7: TASKS NA SEMANA troca sete anéis por barras

**Files:**
- Modify: `apps/desktop/src/Widgets.tsx` (`WeekRings`)
- Modify: `apps/desktop/src/App.tsx` (a chamada do widget `week_rings`)

**Interfaces:**
- Consumes: `Bars` de `./Plot` (Task 5); os slots do `<Widget>` (Task 4).
- Produces: `WeekRings` mantém a assinatura `{ tasks, onOpen }` e passa a devolver barras. Expõe também os números que a manchete e o rodapé usam, via o `<Widget>` em `App.tsx`.

- [ ] **Step 1: Trocar a forma dentro de `WeekRings`**

Em `apps/desktop/src/Widgets.tsx`, substituir o `return` de `WeekRings` (o bloco `widget-week-grid` com os sete `<Ring>`) por:

```tsx
  return (
    <div className="widget-week">
      <div className="widget-week-head">
        <span className="micro-label">SEMANA</span>
        <button type="button" className="filter-label" onClick={onOpen}>
          {week.total} {week.total === 1 ? "TASK CONCLUÍDA" : "TASKS CONCLUÍDAS"}
        </button>
      </div>
      {/* Barras e não anéis: comparar sete alturas é mais rápido que comparar
          sete ângulos, e 44px era justamente o tamanho em que o cap arredondado
          mais distorceria (ADR-040). A proporção continua sendo contra o melhor
          dia da semana, e não contra uma meta que ninguém definiu. */}
      <div className="widget-plot">
        <Bars
          ratios={week.days.map((day) => (day.isFuture ? 0 : day.done / week.peak))}
          labels={week.days.map((day, index) => (day.isToday ? "HOJE" : WEEKDAYS[index]))}
          highlight={week.days.findIndex((day) => day.isToday)}
        />
      </div>
    </div>
  );
```

E ajustar os imports do topo do arquivo:

```tsx
import { useMemo } from "react";
import { Bars } from "./Plot";
import { Ring, RingLabel, stagger } from "./Ring";
import type { Capture, Task } from "./types";
```

O `week` já expõe `peak` e `total`; acrescentar o pico à memo não é necessário.

- [ ] **Step 2: Preencher manchete e rodapé na chamada**

Em `apps/desktop/src/App.tsx`, o widget `week_rings` passa a:

```tsx
      <Widget id="week_rings" role="overview" span={6} hidden={hiddenIds.has("week_rings")}
        value={String(tasks.filter((task) => task.completedAt && new Date(task.completedAt) >= startOfCurrentWeek).length)}
        unit="concluídas"
        footLeft="SEG–DOM · CONTRA O PICO"
        footRight={`PICO ${weekPeak}`}
      ><Panel label="TASKS NA SEMANA"><WeekRings tasks={tasks} onOpen={openTasksPage} /></Panel></Widget>
```

Isso exige dois valores no escopo do componente da Home. Acrescentá-los junto dos outros `useMemo` da Home:

```tsx
  /* A semana da Home: a mesma janela que o `WeekRings` usa, calculada aqui só
     para a manchete e o rodapé — o widget continua dono do próprio cálculo. */
  const { startOfCurrentWeek, weekPeak } = useMemo(() => {
    const today = new Date();
    const start = new Date(today);
    start.setHours(0, 0, 0, 0);
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));

    const perDay = new Array(7).fill(0);
    for (const task of tasks) {
      if (!task.completedAt) continue;
      const at = new Date(task.completedAt);
      if (at < start) continue;
      const index = Math.floor((at.getTime() - start.getTime()) / 86_400_000);
      if (index >= 0 && index < 7) perDay[index] += 1;
    }
    return { startOfCurrentWeek: start, weekPeak: Math.max(...perDay) };
  }, [tasks]);
```

- [ ] **Step 3: Verificar**

Run: `cd apps/desktop && npm run build && npm test -- --run`
Expected: build limpo, testes passando.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/Widgets.tsx apps/desktop/src/App.tsx
git commit -m "feat(widgets): tasks na semana passa a barras"
```

---

### Task 8: Os três widgets de tempo trocam de forma

**Files:**
- Modify: `apps/desktop/src/TimeWidgets.tsx` (`TodayHours`, `WeekByProject`, `BudgetRing`)
- Modify: `apps/desktop/src/App.tsx` (as chamadas de `today_hours`, `week_by_project` e `budget_ring`)

**Interfaces:**
- Consumes: `Bullet`, `Spark`, `Stack` de `./Plot` (Task 5); os slots do `<Widget>` (Task 4).
- Produces: as três funções mantêm as assinaturas atuais — `TodayHours({ time })`, `WeekByProject({ time, projects, onOpen })`, `BudgetRing({ time, projects, onOpen })`.

- [ ] **Step 1: `TodayHours` ganha a linha dos sete dias**

Em `apps/desktop/src/TimeWidgets.tsx`, no `return` de `TodayHours`, acrescentar a linha depois do bloco `widget-progress-copy`, dentro de um wrapper:

```tsx
  return (
    <div className="widget-time-today">
      <div className="widget-progress">
        <Ring size={88} arc={270} segments={[{ value: today / peak }]}>
          <RingLabel value={hoursOf(today)} unit={running ? "CONTANDO" : "HOJE"} />
        </Ring>
        <div className="widget-progress-copy">
          <span className="micro-label">HOJE</span>
          <p className="hermes-quiet">
            {today === 0
              ? "Nenhuma hora registrada hoje."
              : best === 0
                ? "Primeiro dia com horas nesta semana."
                : today >= best
                  ? "Seu melhor dia da semana."
                  : `Melhor dia da semana: ${clockOf(best)}.`}
          </p>
        </div>
      </div>
      {/* A série já era calculada e descartada: `dailySeconds` devolve sete dias
          e o widget usava só o de hoje e o pico. A linha mostra o que já estava
          computado — nenhum dado novo entrou. */}
      <div className="widget-plot">
        <Spark ratios={week.map((day) => day.seconds / peak)} />
      </div>
    </div>
  );
```

A classe `widget-time-today` é nova e precisa de CSS. Em `apps/desktop/src/App.css`, junto das outras regras de widget de tempo (perto de `.widget-time-grid`):

```css
/* O arco e a linha empilhados: o arco responde "quanto hoje", a linha responde
   "hoje comparado a quando". São duas perguntas, e por isso duas faixas. */
.widget-time-today {
  display: grid;
  gap: var(--space-3);
}
```

- [ ] **Step 2: `WeekByProject` troca quatro anéis por uma empilhada**

Substituir o bloco `widget-time-grid` (o `map` com quatro `<Ring>`) por:

```tsx
        // Empilhada e não quatro anéis: a pergunta do widget é "onde foi parar a
        // semana?", que é composição. Quatro anéis pedem comparação par a par,
        // que é uma leitura a mais para responder a mesma coisa.
        <div className="widget-plot">
          <Stack
            values={ranked.map((row) => row.seconds)}
            labels={ranked.map((row) => row.name)}
          />
        </div>
```

- [ ] **Step 3: `BudgetRing` troca o anel pelo bullet**

Substituir o `<Ring size={88} ...>` e seu `RingLabel` por:

```tsx
      {/* O bullet desenha o estouro, que o anel não conseguia: ele parava em
          cheio porque uma segunda volta se leria como "começou de novo", e o
          excesso vivia só no texto (ADR-040). */}
      <div className="widget-plot">
        <Bullet value={target.seconds} target={target.budgetSeconds} over={over} />
      </div>
```

O bloco `widget-progress-copy` que vem depois permanece exatamente como está — é ele que já diz o nome do Project e o quanto falta ou passou.

E ajustar o import do topo do arquivo:

```tsx
import { Bullet, Spark, Stack } from "./Plot";
import { Ring, RingLabel, stagger } from "./Ring";
```

Se `stagger` deixar de ser usado no arquivo depois das trocas, removê-lo do import — o `tsc` do build reclama de símbolo não usado.

- [ ] **Step 4: Preencher manchetes e rodapés das três chamadas**

Em `apps/desktop/src/App.tsx`:

```tsx
      <Widget id="today_hours" role="focus" span={3} hidden={hiddenIds.has("today_hours")}
        footLeft="7 DIAS · CONTRA O PICO"
        footRight={`PICO ${hoursLabel(weekTime.peakSeconds)}`}
      ><Panel label="HORAS HOJE"><TodayHours time={trackedTime} /></Panel></Widget>
```

```tsx
      <Widget id="week_by_project" role="overview" span={6} hidden={hiddenIds.has("week_by_project")}
        value={hoursLabel(weekTime.seconds)}
        unit="na semana"
        footLeft={`${weekTime.projectCount} PROJECTS · 7 DIAS`}
        footRight={weekTime.topProject ? `MAIOR: ${weekTime.topProject}` : undefined}
      ><Panel label="HORAS POR PROJECT"><WeekByProject time={trackedTime} projects={projects} onOpen={openTempoPage} /></Panel></Widget>
```

Isso exige um resumo da semana. O `TrackedTime` **não** ganha campos — ele é o formato dos dados carregados, e derivar dentro dele misturaria as duas coisas. Em vez disso, uma função pura exportada de `TimeWidgets.tsx`, ao lado de `dailySeconds`:

```tsx
/** `3,2 H` — a unidade do rodapé, a mesma que o resto do Tempo usa. */
export function hoursLabel(seconds: number) {
  return `${(seconds / 3600).toFixed(1).replace(".", ",")} H`;
}

/**
 * O resumo dos últimos sete dias, para a manchete e o rodapé da Home.
 *
 * Repete o corte de um minuto do `WeekByProject` de propósito: sem ele, um
 * cronômetro parado por engano contaria como um Project na contagem do rodapé,
 * e os dois números discordariam do que o widget desenha logo acima.
 */
export function weekSummary(time: TrackedTime, projects: Project[]) {
  const since = new Date();
  since.setDate(since.getDate() - 6);
  since.setHours(0, 0, 0, 0);

  const perProject = new Map<string, number>();
  const perDay = new Map<number, number>();
  for (const entry of time.entries) {
    const at = new Date(entry.startedAt);
    if (at < since) continue;
    const seconds = Math.max(0, entry.durationSeconds);
    perProject.set(entry.projectId, (perProject.get(entry.projectId) ?? 0) + seconds);
    const day = dayKey(entry.startedAt);
    perDay.set(day, (perDay.get(day) ?? 0) + seconds);
  }

  const counted = [...perProject.entries()].filter(([, seconds]) => seconds >= 60);
  const top = counted.sort((left, right) => right[1] - left[1])[0];

  return {
    seconds: counted.reduce((sum, [, seconds]) => sum + seconds, 0),
    peakSeconds: Math.max(0, ...perDay.values()),
    projectCount: counted.length,
    topProject: top ? projects.find((project) => project.id === top[0])?.name ?? null : null,
  };
}
```

E na Home, em `App.tsx`, junto dos outros `useMemo`:

```tsx
  const weekTime = useMemo(() => weekSummary(trackedTime, projects), [trackedTime, projects]);
```

As três chamadas acima passam a usar `weekTime.peakSeconds`, `weekTime.seconds`, `weekTime.projectCount` e `weekTime.topProject` no lugar dos campos de `trackedTime`. Acrescentar `hoursLabel` e `weekSummary` ao import de `./TimeWidgets` em `App.tsx`.

Para `budget_ring`, a manchete sai do próprio `BudgetRing`, que já calcula a razão. Como o `<Widget>` não tem acesso ao `target`, o rodapé fica com a escala fixa e a manchete não é preenchida aqui — o `Bullet` é acompanhado do texto que o widget já emite:

```tsx
      <Widget id="budget_ring" role="overview" span={3} hidden={hiddenIds.has("budget_ring") || !hasBudget}
        footLeft="CONTRA A META"
      ><Panel label="META"><BudgetRing time={trackedTime} projects={projects} onOpen={openProject} /></Panel></Widget>
```

- [ ] **Step 5: Verificar**

Run: `cd apps/desktop && npm run build && npm test -- --run`
Expected: build limpo, testes passando.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/TimeWidgets.tsx apps/desktop/src/App.tsx
git commit -m "feat(widgets): horas viram linha, empilhada e bullet"
```

---

### Task 9: Manchetes e rodapés nos widgets restantes

**Files:**
- Modify: `apps/desktop/src/App.tsx` (as chamadas de `now`, `inbox_pulse`, `recent`, `projects`, `month_density`, `recent_resources`, `apps`)

**Interfaces:**
- Consumes: os slots do `<Widget>` (Task 4).
- Produces: nada — é a última camada de conteúdo.

- [ ] **Step 1: Preencher os sete**

Em `apps/desktop/src/App.tsx`, acrescentar os slots. Cada valor já existe no escopo:

```tsx
      <Widget id="now" role="focus" span={6} hidden={hiddenIds.has("now")}
        value={String(doing.length)} unit="em andamento"
      >
```

O INBOX não recebe manchete — o `RingLabel` já traz o número no centro do anel —, mas recebe rodapé, porque a proporção desenhada tem escala e ela não está escrita em lugar nenhum hoje:

```tsx
      <Widget id="inbox_pulse" role="attention" span={3} hidden={hiddenIds.has("inbox_pulse")}
        footLeft="ENVELHECENDO"
        footRight={inbox.length ? `${staleInbox} DE ${inbox.length}${inboxCapped ? "+" : ""}` : undefined}
      >
```

```tsx
      <Widget id="recent" role="attention" span={5} hidden={hiddenIds.has("recent")}
        value={String(recent.length)} unit={recent.length === 1 ? "captura" : "capturas"}
      >
```

```tsx
      <Widget id="projects" role="attention" span={4} hidden={hiddenIds.has("projects")}
        value={String(scopedProjects.length)} unit="ativos"
      >
```

```tsx
      <Widget id="month_density" role="overview" span={6} hidden={hiddenIds.has("month_density")}
        value={String(monthRecords)} unit="registros"
        footLeft="30 DIAS · 4 DEGRAUS" footRight={`PICO ${monthPeak}`}
      >
```

```tsx
      <Widget id="recent_resources" role="collection" span={8} hidden={hiddenIds.has("recent_resources")}
        value={String(activeResources.length)} unit={activeResources.length === 1 ? "recurso" : "recursos"}
      >
```

```tsx
      <Widget id="apps" role="collection" span={4} hidden={hiddenIds.has("apps")}
        value={String(activeApps.length)} unit={activeApps.length === 1 ? "app" : "apps"}
      >
```

- [ ] **Step 2: Expor os dois números do mês**

`MonthDensity` calcula `total` e `peak` internamente e não os devolve. Para não duplicar a conta na Home, exportar a função de agregação de `Widgets.tsx` e usá-la nos dois lugares:

```tsx
/** A carga de cada dia do mês corrente: Task criada, Task concluída, Capture.
 *  Exportada porque a Home também precisa do total e do pico para o rodapé, e
 *  duplicar a conta em dois arquivos é como as duas versões divergem. */
export function monthActivity(tasks: Task[], captures: Capture[]) {
  const today = new Date();
  const activity = new Map<string, number>();
  const bump = (value: string | null) => {
    if (!value) return;
    const date = new Date(value);
    if (date.getMonth() !== today.getMonth() || date.getFullYear() !== today.getFullYear()) return;
    const key = String(date.getDate());
    activity.set(key, (activity.get(key) ?? 0) + 1);
  };
  tasks.forEach((task) => { bump(task.createdAt); bump(task.completedAt); });
  captures.forEach((capture) => bump(capture.capturedAt));
  return activity;
}
```

`MonthDensity` passa a chamar `monthActivity(tasks, captures)` dentro do seu `useMemo`, no lugar do bloco que hoje monta o `Map` inline. E na Home:

```tsx
  const { monthRecords, monthPeak } = useMemo(() => {
    const activity = monthActivity(tasks, recent);
    const values = [...activity.values()];
    return {
      monthRecords: values.reduce((sum, value) => sum + value, 0),
      monthPeak: values.length ? Math.max(...values) : 0,
    };
  }, [tasks, recent]);
```

Acrescentar `monthActivity` ao import de `./Widgets` em `App.tsx`.

- [ ] **Step 3: Verificar**

Run: `cd apps/desktop && npm run build && npm test -- --run`
Expected: build limpo, testes passando.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/Widgets.tsx
git commit -m "feat(widgets): manchete e rodape nos widgets restantes"
```

---

### Task 10: QA e registro de execução

**Files:**
- Modify: `docs/UI-UX-REFINEMENT.md` (acrescentar seção de estado de execução ao final)

**Interfaces:**
- Consumes: tudo que as Tasks 1 a 9 produziram.
- Produces: o registro que fecha o recorte.

- [ ] **Step 1: Rodar as suítes**

Run:
```bash
cd apps/desktop && npm run build && npm test -- --run && npx impeccable detect src
cd ../.. && git diff --check
```
Expected: tudo aprovado.

- [ ] **Step 2: QA visual no cliente Tauri real**

Run: `cd apps/desktop && npm run tauri dev`

Conferir na Home, e anotar o que falhar:
- Dark em 840×600, 1280×800, 1440×900 e 1920×1080;
- **Light nas quatro larguras** — a superfície aninhada muda de mecanismo entre os temas (§4.2 do spec), então Light não é amostragem aqui;
- moldura não vazou: abrir Settings, o Inspector de Workspaces e o Tempo e confirmar que os painéis continuam sem card;
- o rodapé gruda no fim do card em widgets de alturas diferentes na mesma faixa;
- barras: a de hoje é branca, as demais em sódio; dia sem task não desenha barra;
- bullet: com meta estourada, a marca recua para dentro e a barra vai ao fim;
- `reduced-motion` ligado no Windows: as formas nascem no valor final, sem crescer;
- `forced-colors` ligado: superfície aninhada e trilhos se declaram por contorno;
- teclado e foco sem regressão nos widgets que têm botão (SEMANA, META, RECENTES).

- [ ] **Step 3: Escrever o registro**

Ao final de `docs/UI-UX-REFINEMENT.md`, uma seção nova `## 29. Estado de execução — Widgets Mono Charts`, cobrindo: o que mudou de forma, a moldura e seu escopo, a compensação e seu limite, o que foi recusado, a evidência de QA (comandos e larguras efetivamente verificados) e o limite do lote (nenhuma regra de negócio, API, banco, schema ou contrato alterado; Tempo e Hermes 3B seguem pendentes).

- [ ] **Step 4: Commit**

```bash
git add docs/UI-UX-REFINEMENT.md
git commit -m "docs: registra a execucao dos widgets com geometria macia"
```

---

## Notas de execução

**A ordem importa em três pontos e só neles:** a Task 1 autoriza as demais; a Task 2 é dependência das Tasks 5 e 6; a Task 4 é dependência das Tasks 7, 8 e 9. As Tasks 7, 8 e 9 são independentes entre si e podem ser feitas em qualquer ordem.

**O que não é verificável pelo agente:** tudo na Task 10, Step 2. A janela do Tauri não é legível daqui, e nenhuma afirmação sobre o que aparece em tela deve ser escrita sem que o proprietário tenha olhado.

**Se a moldura ficar apertada em 840×600:** o ajuste é no `padding` de `.home-grid .widget` e no `gap` do `.home-grid`, não na remoção da moldura de widgets específicos — moldura em todos foi decisão explícita do proprietário.

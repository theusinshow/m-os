# Gráficos do M-Finance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Levar as dashboards do M-Finance de dois gráficos para oito, portando as *formas* da matos-ui para recharts com os tokens do M/OS, e corrigir o bug de agregação em `lib/budgets.ts` que o levantamento revelou.

**Architecture:** Toda transformação de dado é função pura em `lib/calculations/charts/`, testada com vitest em ambiente `node`. Os componentes em `components/charts/` são finos: consomem a função pura, desenham com recharts e não decidem nada. As páginas só passam dados.

**Tech Stack:** Next.js (App Router, RSC), TypeScript, Tailwind v4, recharts 3.8.1, drizzle-orm + postgres, vitest 3.

**Spec:** `docs/superpowers/specs/2026-08-24-m-finance-charts-design.md`

## Global Constraints

Estas valem para **toda** task. Copiadas do spec §3.

- **Nenhuma dependência nova.** `framer-motion`, `tailwind-variants` e `shadcn` estão fora. Só recharts 3.8.1, que já está no `package.json`.
- **Cor só de `@/lib/ui/colors.ts`.** Nenhum hex literal em componente, nenhuma matiz nova adicionada ao arquivo. A paleta categórica é `CHART_PALETTE`, uma rampa de luminância.
- **Sem gradiente de área (`<defs><linearGradient>`), sem glow, sem animação de entrada.** Em recharts isso significa `isAnimationActive={false}` em toda série.
- **Largura por `useChartWidth()`** de `@/components/charts/use-chart-width`, nunca `ResponsiveContainer`.
- **Vazio por `<InlineEmpty>`** de `@/components/ui/inline-empty`.
- **Tooltip por `<CurrencyTooltip />`** de `@/components/charts/chart-tooltip`.
- **Dinheiro em centavos** (`number` inteiro) no dado; `formatCurrency` / `formatCurrencyCompact` só na apresentação.
- **Componentes de gráfico levam `"use client"`** na primeira linha. As páginas continuam Server Components.
- **Datas `YYYY-MM-DD` são parseadas manualmente** com `split("-")`. Nunca `new Date("2026-08-01")` — isso interpreta como UTC e volta um dia no fuso do Brasil.
- **Testes** são `*.test.ts` (nunca `.tsx` — o ambiente vitest é `node`, sem DOM), ao lado do arquivo testado.
- Rodar teste: `cd apps/m-finance && npx vitest run <caminho>`. Suíte inteira: `npm test`.
- Todo comentário e string de interface em **português**, seguindo o app.

---

## Task 1: `formatCurrencyCompact`

O `YAxis` do `history-trend-chart` está `hide` porque `R$ 12.345,67` não cabe num eixo. Sem este formatador, a Task 3 não tem como devolver a escala.

**Files:**
- Modify: `apps/m-finance/lib/formatters/currency.ts`
- Test: `apps/m-finance/lib/formatters/currency.test.ts` (criar)

**Interfaces:**
- Consumes: nada.
- Produces: `formatCurrencyCompact(cents: number): string`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/formatters/currency.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { formatCurrencyCompact } from "@/lib/formatters/currency";

describe("formatCurrencyCompact", () => {
  it("mostra valores abaixo de mil sem sufixo e sem centavos", () => {
    expect(formatCurrencyCompact(0)).toBe("R$ 0");
    expect(formatCurrencyCompact(12_345)).toBe("R$ 123");
    expect(formatCurrencyCompact(99_999)).toBe("R$ 1000");
  });

  it("abrevia milhar com virgula decimal", () => {
    expect(formatCurrencyCompact(100_000)).toBe("R$ 1 mil");
    expect(formatCurrencyCompact(1_234_500)).toBe("R$ 12,3 mil");
    // Fronteira feia e assumida: R$ 999.999,00 arredonda para "1000 mil" em vez
    // de virar "1 mi". Num eixo isso é aceitável, e tratar o caso exigiria
    // reescalar depois de arredondar — complexidade que um tick não paga.
    expect(formatCurrencyCompact(99_999_900)).toBe("R$ 1000 mil");
  });

  it("abrevia milhao", () => {
    expect(formatCurrencyCompact(100_000_000)).toBe("R$ 1 mi");
    expect(formatCurrencyCompact(345_600_000)).toBe("R$ 3,5 mi");
  });

  it("preserva o sinal negativo", () => {
    expect(formatCurrencyCompact(-1_234_500)).toBe("-R$ 12,3 mil");
    expect(formatCurrencyCompact(-50_000)).toBe("-R$ 500");
  });
});
```

- [ ] **Step 2: Rodar o teste e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/formatters/currency.test.ts`
Expected: FAIL — `formatCurrencyCompact is not a function`.

- [ ] **Step 3: Implementar**

Adicionar ao fim de `apps/m-finance/lib/formatters/currency.ts`:

```ts
/**
 * Valor curto o bastante para caber num eixo de gráfico.
 *
 * `formatCurrency` devolve `R$ 12.345,67`, que num `YAxis` ou empurra a área
 * de desenho para fora ou é cortado. Aqui a precisão é trocada por largura de
 * propósito: quem precisa do centavo lê o tooltip, que continua usando
 * `formatCurrency`.
 */
export function formatCurrencyCompact(cents: number) {
  const reais = cents / 100;
  const absolute = Math.abs(reais);
  const sign = reais < 0 ? "-" : "";

  if (absolute >= 1_000_000) {
    return `${sign}R$ ${decimal(absolute / 1_000_000)} mi`;
  }
  if (absolute >= 1_000) {
    return `${sign}R$ ${decimal(absolute / 1_000)} mil`;
  }
  return `${sign}R$ ${Math.round(absolute)}`;
}

/** Uma casa decimal, vírgula no lugar do ponto, sem `,0` pendurado. */
function decimal(value: number) {
  return value.toFixed(1).replace(/\.0$/, "").replace(".", ",");
}
```

- [ ] **Step 4: Rodar o teste e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/formatters/currency.test.ts`
Expected: PASS, 4 testes.

- [ ] **Step 5: Commit**

```bash
git add apps/m-finance/lib/formatters/currency.ts apps/m-finance/lib/formatters/currency.test.ts
git commit -m "feat(m-finance): o valor que cabe no eixo"
```

---

## Task 2: `toCategorySlices` e a barra de categorias com número

Hoje `category-breakdown-chart` desenha barras sem valor e sem proporção, e a altura é `sorted.length * 44` — vinte categorias viram 880px.

**Files:**
- Create: `apps/m-finance/lib/calculations/charts/categories.ts`
- Test: `apps/m-finance/lib/calculations/charts/categories.test.ts`
- Modify: `apps/m-finance/components/charts/category-breakdown-chart.tsx`

**Interfaces:**
- Consumes: nada.
- Produces: `type CategorySlice = { name: string; value: number; percent: number }` e `toCategorySlices(data: { name: string; value: number }[], maxSlices?: number): CategorySlice[]`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/calculations/charts/categories.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { toCategorySlices } from "@/lib/calculations/charts/categories";

describe("toCategorySlices", () => {
  it("ordena da maior para a menor e descarta valor zero ou negativo", () => {
    const slices = toCategorySlices([
      { name: "Lazer", value: 1_000 },
      { name: "Casa", value: 5_000 },
      { name: "Vazia", value: 0 },
      { name: "Mercado", value: 4_000 },
    ]);

    expect(slices.map((slice) => slice.name)).toEqual(["Casa", "Mercado", "Lazer"]);
  });

  it("agrupa da oitava categoria em diante como Outras", () => {
    const data = Array.from({ length: 12 }, (_, index) => ({
      name: `Cat ${index}`,
      value: (12 - index) * 1_000,
    }));

    const slices = toCategorySlices(data);

    expect(slices).toHaveLength(8);
    expect(slices[7].name).toBe("Outras");
    // As cinco menores: 5000 + 4000 + 3000 + 2000 + 1000.
    expect(slices[7].value).toBe(15_000);
  });

  it("nao cria Outras quando cabe tudo", () => {
    const slices = toCategorySlices([
      { name: "Casa", value: 2_000 },
      { name: "Lazer", value: 1_000 },
    ]);

    expect(slices.map((slice) => slice.name)).toEqual(["Casa", "Lazer"]);
  });

  it("distribui os percentuais de modo que somem exatamente 100", () => {
    const slices = toCategorySlices([
      { name: "A", value: 1 },
      { name: "B", value: 1 },
      { name: "C", value: 1 },
    ]);

    expect(slices.reduce((total, slice) => total + slice.percent, 0)).toBe(100);
    expect(slices.map((slice) => slice.percent).sort()).toEqual([33, 33, 34]);
  });

  it("devolve lista vazia quando nao ha valor positivo", () => {
    expect(toCategorySlices([])).toEqual([]);
    expect(toCategorySlices([{ name: "Zerada", value: 0 }])).toEqual([]);
  });

  it("respeita um teto customizado", () => {
    const data = Array.from({ length: 5 }, (_, index) => ({
      name: `Cat ${index}`,
      value: 1_000,
    }));

    const slices = toCategorySlices(data, 3);

    expect(slices).toHaveLength(3);
    expect(slices[2].name).toBe("Outras");
    expect(slices[2].value).toBe(3_000);
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/categories.test.ts`
Expected: FAIL — módulo não encontrado.

- [ ] **Step 3: Implementar a função pura**

Criar `apps/m-finance/lib/calculations/charts/categories.ts`:

```ts
export type CategorySlice = {
  name: string;
  value: number;
  /** Percentual inteiro do total. A soma da lista é exatamente 100. */
  percent: number;
};

/**
 * Fatia as categorias do mês em algo que cabe num gráfico.
 *
 * Duas decisões moram aqui em vez do componente: o teto de categorias — sem
 * ele, vinte categorias viram um gráfico de 880px que ninguém lê inteiro — e a
 * distribuição do percentual, que precisa fechar em 100 porque o número aparece
 * rotulado na barra e `33% + 33% + 33%` lido numa tela parece defeito.
 */
export function toCategorySlices(
  data: { name: string; value: number }[],
  maxSlices = 8,
): CategorySlice[] {
  const positive = data.filter((item) => item.value > 0);
  const total = positive.reduce((sum, item) => sum + item.value, 0);
  if (total === 0) return [];

  const sorted = [...positive].sort((a, b) => b.value - a.value);

  // Cabendo tudo, não existe "Outras". Não cabendo, a última vaga é dela.
  const head = sorted.length > maxSlices ? sorted.slice(0, maxSlices - 1) : sorted;
  const tail = sorted.slice(head.length);
  const grouped =
    tail.length > 0
      ? [...head, { name: "Outras", value: tail.reduce((sum, item) => sum + item.value, 0) }]
      : head;

  const percents = distributePercents(
    grouped.map((slice) => slice.value),
    total,
  );

  return grouped.map((slice, index) => ({
    name: slice.name,
    value: slice.value,
    percent: percents[index],
  }));
}

/**
 * Maior resto: arredonda todo mundo para baixo e devolve as sobras para quem
 * tinha a maior fração. É o que garante que a soma feche em 100 sem distorcer
 * nenhuma fatia em mais de um ponto.
 */
function distributePercents(values: number[], total: number): number[] {
  const exact = values.map((value) => (value / total) * 100);
  const result = exact.map(Math.floor);
  const missing = 100 - result.reduce((sum, value) => sum + value, 0);

  const byFraction = exact
    .map((value, index) => ({ index, fraction: value - Math.floor(value) }))
    .sort((a, b) => b.fraction - a.fraction);

  for (let given = 0; given < missing; given += 1) {
    result[byFraction[given % byFraction.length].index] += 1;
  }

  return result;
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/categories.test.ts`
Expected: PASS, 6 testes.

- [ ] **Step 5: Reescrever o componente**

Substituir o conteúdo de `apps/m-finance/components/charts/category-breakdown-chart.tsx` por:

```tsx
"use client";

import { Bar, BarChart, Cell, LabelList, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { toCategorySlices, type CategorySlice } from "@/lib/calculations/charts/categories";
import { formatCurrency } from "@/lib/formatters/currency";
import { CHART_CURSOR_FILL, CHART_PALETTE, COLORS } from "@/lib/ui/colors";

export type CategoryDatum = { name: string; value: number };

export function CategoryBreakdownChart({ data }: { data: CategoryDatum[] }) {
  const { ref, width } = useChartWidth();
  const slices = toCategorySlices(data);

  if (slices.length === 0) {
    return <InlineEmpty>Sem contas categorizadas neste mês.</InlineEmpty>;
  }

  const height = Math.max(120, slices.length * 44);

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <BarChart
          barCategoryGap={10}
          data={slices}
          height={height}
          layout="vertical"
          // Espaço à direita para o rótulo de valor não ser cortado.
          margin={{ left: 0, right: 96, top: 4, bottom: 4 }}
          width={width}
        >
          <XAxis hide type="number" />
          <YAxis
            axisLine={false}
            dataKey="name"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
            type="category"
            width={96}
          />
          <Tooltip content={<CurrencyTooltip />} cursor={{ fill: CHART_CURSOR_FILL }} />
          <Bar dataKey="value" isAnimationActive={false} name="Total" radius={[0, 4, 4, 0]}>
            {slices.map((slice, index) => (
              <Cell fill={CHART_PALETTE[index % CHART_PALETTE.length]} key={slice.name} />
            ))}
            <LabelList content={<SliceLabel />} dataKey="value" position="right" />
          </Bar>
        </BarChart>
      ) : null}
    </div>
  );
}

type SliceLabelProps = {
  x?: number | string;
  y?: number | string;
  width?: number | string;
  height?: number | string;
  index?: number;
  // Recharts injeta a linha inteira do dado em `payload`.
  payload?: CategorySlice;
};

/**
 * Valor e proporção no fim da barra.
 *
 * `LabelList` com `dataKey` só desenha o número cru. O rótulo aqui é composto
 * — `R$ 1.234,00 · 32%` — e o percentual vem pronto de `toCategorySlices`, que
 * já garantiu que a coluna soma 100.
 */
function SliceLabel({ x, y, width, height, payload }: SliceLabelProps) {
  if (!payload) return null;

  const left = Number(x ?? 0) + Number(width ?? 0) + 8;
  const middle = Number(y ?? 0) + Number(height ?? 0) / 2;

  return (
    <text
      dominantBaseline="middle"
      fill={COLORS.textSecondary}
      fontSize={12}
      x={left}
      y={middle}
    >
      {formatCurrency(payload.value)}
      <tspan fill={COLORS.muted}> · {payload.percent}%</tspan>
    </text>
  );
}
```

- [ ] **Step 6: Conferir tipo e lint**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint`
Expected: sem erro. Se o recharts 3.8.1 não injetar `payload` no `content` de `LabelList`, troque `payload?: CategorySlice` por leitura via `index` num closure sobre `slices` — o array está em escopo no componente pai; mova `SliceLabel` para dentro de `CategoryBreakdownChart` e use `slices[index ?? 0]`.

- [ ] **Step 7: Commit**

```bash
git add apps/m-finance/lib/calculations/charts/categories.ts apps/m-finance/lib/calculations/charts/categories.test.ts apps/m-finance/components/charts/category-breakdown-chart.tsx
git commit -m "feat(m-finance): a barra que diz quanto, e para de crescer sem fim"
```

---

## Task 3: `history-trend-chart` com hierarquia e escala

Três linhas de 2px em cores de peso parecido, `YAxis hide`, legenda só com nome. A pergunta da `/history` é "a sobra está melhorando?", e o gráfico não responde.

**Files:**
- Modify: `apps/m-finance/components/charts/history-trend-chart.tsx`

**Interfaces:**
- Consumes: `formatCurrencyCompact` (Task 1).
- Produces: nada novo. `TrendDatum` continua igual — a `/history` não muda.

- [ ] **Step 1: Reescrever o componente**

Substituir o conteúdo de `apps/m-finance/components/charts/history-trend-chart.tsx` por:

```tsx
"use client";

import { CartesianGrid, Legend, Line, LineChart, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { formatCurrency, formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export type TrendDatum = {
  label: string;
  receita: number;
  comprometido: number;
  sobra: number;
};

/**
 * A ordem importa: `sobra` por último para desenhar por cima, e com o peso
 * visual que as outras duas não têm.
 *
 * A pergunta da tela é "a sobra está melhorando?". Três linhas de mesma
 * espessura e cor equivalente fazem o olho procurar qual delas responde; uma
 * linha em sódio sobre duas neutras finas já responde antes da leitura.
 */
const SERIES = [
  { key: "receita", name: "Receita", color: COLORS.muted, width: 1 },
  { key: "comprometido", name: "Comprometido", color: COLORS.textSecondary, width: 1 },
  { key: "sobra", name: "Sobra", color: COLORS.accent, width: 2 },
] as const;

export function HistoryTrendChart({ data }: { data: TrendDatum[] }) {
  const { ref, width } = useChartWidth();

  if (data.length < 2) {
    return <InlineEmpty>Salve pelo menos dois meses para ver a evolução.</InlineEmpty>;
  }

  const latest = data[data.length - 1];

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <LineChart
          data={data}
          height={240}
          margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
          width={width}
        >
          <CartesianGrid stroke={CHART_GRID} vertical={false} />
          <XAxis
            axisLine={false}
            dataKey="label"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
          />
          {/* Largura fixa: sem ela o eixo mede o rótulo mais largo de cada mês
              e a área de desenho pula de tamanho ao trocar de período. */}
          <YAxis
            axisLine={false}
            tick={{ fill: COLORS.muted, fontSize: 11 }}
            tickFormatter={formatCurrencyCompact}
            tickLine={false}
            width={64}
          />
          <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
          <Legend
            formatter={(value, entry) => {
              const key = String(entry?.dataKey ?? "") as keyof TrendDatum;
              const current = typeof latest[key] === "number" ? (latest[key] as number) : null;
              return (
                <span style={{ color: COLORS.muted, fontSize: 12 }}>
                  {value}
                  {current === null ? null : (
                    <span style={{ color: COLORS.textSecondary, marginLeft: 6 }}>
                      {formatCurrency(current)}
                    </span>
                  )}
                </span>
              );
            }}
            iconType="plainline"
          />
          {SERIES.map((series) => (
            <Line
              dataKey={series.key}
              dot={false}
              isAnimationActive={false}
              key={series.key}
              name={series.name}
              stroke={series.color}
              strokeWidth={series.width}
              type="monotone"
            />
          ))}
        </LineChart>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 2: Conferir tipo e lint**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint`
Expected: sem erro. Se o segundo parâmetro do `formatter` da `Legend` não tipar em recharts 3.8.1, tipe-o como `{ dataKey?: string | number }` numa anotação inline em vez de deixar implícito.

- [ ] **Step 3: Rodar a suíte para garantir que nada quebrou**

Run: `cd apps/m-finance && npm test`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/m-finance/components/charts/history-trend-chart.tsx
git commit -m "feat(m-finance): a sobra vira a protagonista, e o eixo volta"
```

---

## Task 4: O bug de agregação em `lib/budgets.ts`

`getSpentForBudget` faz `db.select({ total: bills.amountCents })` e pega `[row]` — o valor da primeira linha, não a soma. Não há `sum()` nem `groupBy` no arquivo. Um orçamento com cinco contas mostra o valor de uma. A Task 8 desenha em cima deste número.

**Files:**
- Modify: `apps/m-finance/lib/budgets.ts:57-105`
- Test: `apps/m-finance/lib/budgets.test.ts` (criar)

**Interfaces:**
- Consumes: nada.
- Produces: nada novo. `Budget` e `getBudgetsByMonth` mantêm a assinatura; só o valor de `spentCents` fica correto.

- [ ] **Step 1: Escrever o teste de regressão que falha**

O padrão de mock de banco deste repo está em `lib/mos/action-bridge.test.ts`: `vi.hoisted` + `vi.mock("@/db/client")` com um `fakeDb` que registra as chamadas.

Um mock devolve o que se enfileira nele, então "o valor está certo" não seria provado por ele. O que o teste prova é **estrutural**: a projeção passada ao `.select()` precisa ser uma expressão SQL (um agregado), não uma coluna crua. É exatamente a diferença entre o código quebrado e o corrigido.

Criar `apps/m-finance/lib/budgets.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";

type SelectCall = { projection: Record<string, unknown> };

const { dbState } = vi.hoisted(() => ({
  dbState: {
    selects: [] as SelectCall[],
    /** Uma entrada por consulta resolvida, na ordem em que acontecem. */
    resultQueue: [] as unknown[][],
  },
}));

/**
 * Encadeamento mínimo do drizzle usado por `budgets.ts`:
 * `.select().from().where()` e `.select().from().where().limit()`.
 * O `where`/`limit` é "thenable" para que o `await` resolva a fila.
 */
const fakeDb = {
  select(projection: Record<string, unknown>) {
    dbState.selects.push({ projection });
    const resolve = () => Promise.resolve(dbState.resultQueue.shift() ?? []);
    const tail = {
      where: () => ({
        limit: resolve,
        then: (onFulfilled: (rows: unknown[]) => unknown) => resolve().then(onFulfilled),
      }),
    };
    return { from: () => tail };
  },
};

vi.mock("@/db/client", () => ({
  get db() {
    return fakeDb;
  },
}));

const { getBudgetsByMonth } = await import("./budgets");

/** Uma expressão SQL do drizzle carrega `queryChunks`; uma coluna crua, não. */
function isSqlExpression(value: unknown) {
  return typeof value === "object" && value !== null && "queryChunks" in value;
}

beforeEach(() => {
  dbState.selects = [];
  dbState.resultQueue = [];
});

describe("getSpentForBudget, por getBudgetsByMonth", () => {
  it("pede um agregado ao banco em vez da primeira linha", async () => {
    // 1: as linhas de budgets. 2: a soma das contas. 3: o rótulo da categoria.
    dbState.resultQueue = [
      [
        {
          id: "budget-1",
          budgetType: "category",
          categoryId: "cat-1",
          cardId: null,
          limitCents: 100_000,
        },
      ],
      [{ total: 75_000 }],
      [{ name: "Mercado" }],
    ];

    const budgets = await getBudgetsByMonth("month-1", "user-1");

    // A consulta do gasto é a segunda: a primeira busca os budgets.
    const spentProjection = dbState.selects[1].projection;
    expect(isSqlExpression(spentProjection.total)).toBe(true);

    expect(budgets[0].spentCents).toBe(75_000);
    expect(budgets[0].remainingCents).toBe(25_000);
    expect(budgets[0].percentage).toBe(75);
    expect(budgets[0].isOverBudget).toBe(false);
    expect(budgets[0].isWarning).toBe(false);
  });

  it("marca alerta em 80% e estouro acima do limite", async () => {
    dbState.resultQueue = [
      [
        {
          id: "budget-2",
          budgetType: "category",
          categoryId: "cat-1",
          cardId: null,
          limitCents: 100_000,
        },
      ],
      [{ total: 120_000 }],
      [{ name: "Mercado" }],
    ];

    const [budget] = await getBudgetsByMonth("month-1", "user-1");

    expect(budget.isOverBudget).toBe(true);
    expect(budget.isWarning).toBe(false);
    expect(budget.remainingCents).toBe(-20_000);
  });

  it("soma contas e faturas no orcamento total", async () => {
    dbState.resultQueue = [
      [
        {
          id: "budget-3",
          budgetType: "total",
          categoryId: null,
          cardId: null,
          limitCents: 500_000,
        },
      ],
      [{ total: 200_000 }], // contas
      [{ total: 150_000 }], // faturas
    ];

    const [budget] = await getBudgetsByMonth("month-1", "user-1");

    expect(budget.spentCents).toBe(350_000);
    expect(isSqlExpression(dbState.selects[1].projection.total)).toBe(true);
    expect(isSqlExpression(dbState.selects[2].projection.total)).toBe(true);
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/budgets.test.ts`
Expected: FAIL — `expect(isSqlExpression(...)).toBe(true)` recebe `false`, porque hoje a projeção é a coluna `bills.amountCents`.

> Se falhar antes disso por causa do encadeamento do mock (`.where(...)` não sendo aguardável), ajuste `fakeDb` até o erro ser o do agregado. O alvo do Step 2 é ver **esta** asserção falhar, não outra.

- [ ] **Step 3: Corrigir os três ramos**

Em `apps/m-finance/lib/budgets.ts`, trocar o import da primeira linha:

```ts
import { and, asc, eq, sql } from "drizzle-orm";
```

E substituir o corpo de `getSpentForBudget` (linhas 57-105) por:

```ts
async function getSpentForBudget(
  userId: string,
  monthId: string,
  type: BudgetType,
  categoryId: string | null,
  cardId: string | null,
): Promise<number> {
  if (!db) return 0;

  // `coalesce(sum(...), 0)::int` é o mesmo padrão de `lib/invoice-sync.ts`:
  // sem o coalesce, um mês sem lançamento devolve `null` em vez de zero, e
  // sem o cast o driver entrega a soma como string.
  const total = sql<number>`coalesce(sum(${bills.amountCents}), 0)::int`;

  if (type === "total") {
    const [billRow] = await db
      .select({ total })
      .from(bills)
      .where(and(eq(bills.userId, userId), eq(bills.monthId, monthId)));
    const [invoiceRow] = await db
      .select({ total: sql<number>`coalesce(sum(${creditCardInvoices.amountCents}), 0)::int` })
      .from(creditCardInvoices)
      .where(and(eq(creditCardInvoices.userId, userId), eq(creditCardInvoices.monthId, monthId)));
    return (billRow?.total ?? 0) + (invoiceRow?.total ?? 0);
  }

  if (type === "category" && categoryId) {
    const [row] = await db
      .select({ total })
      .from(bills)
      .where(
        and(
          eq(bills.userId, userId),
          eq(bills.monthId, monthId),
          eq(bills.categoryId, categoryId),
        ),
      );
    return row?.total ?? 0;
  }

  if (type === "card" && cardId) {
    const [row] = await db
      .select({ total: sql<number>`coalesce(sum(${creditCardExpenses.amountCents}), 0)::int` })
      .from(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.userId, userId),
          eq(creditCardExpenses.cardId, cardId),
          eq(creditCardExpenses.monthId, monthId),
        ),
      );
    return row?.total ?? 0;
  }

  return 0;
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/budgets.test.ts`
Expected: PASS, 3 testes.

- [ ] **Step 5: Conferir tipo e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm test`
Expected: sem erro, tudo passando.

- [ ] **Step 6: Commit**

```bash
git add apps/m-finance/lib/budgets.ts apps/m-finance/lib/budgets.test.ts
git commit -m "fix(m-finance): o orcamento somava uma conta e chamava de total"
```

---

## Task 5: `MonthWaterfallChart` no `/dashboard`

`getDashboardSummary` calcula `receita − contas − faturas = sobra`. Hoje isso aparece como quatro cards soltos e a relação fica por conta de quem lê.

**Files:**
- Create: `apps/m-finance/lib/calculations/charts/waterfall.ts`
- Test: `apps/m-finance/lib/calculations/charts/waterfall.test.ts`
- Create: `apps/m-finance/components/charts/month-waterfall-chart.tsx`
- Modify: `apps/m-finance/app/(app)/app/dashboard/page.tsx`

**Interfaces:**
- Consumes: `formatCurrencyCompact` (Task 1).
- Produces: `type WaterfallStep = { label: string; offset: number; delta: number; value: number; kind: "in" | "out" | "total" }` e `toWaterfallSteps(input: { incomeCents: number; billsCents: number; invoicesCents: number }): WaterfallStep[]`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/calculations/charts/waterfall.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { toWaterfallSteps } from "@/lib/calculations/charts/waterfall";

describe("toWaterfallSteps", () => {
  it("encadeia receita, contas, faturas e sobra", () => {
    const steps = toWaterfallSteps({
      incomeCents: 500_000,
      billsCents: 200_000,
      invoicesCents: 100_000,
    });

    expect(steps.map((step) => [step.label, step.offset, step.delta])).toEqual([
      ["Receita", 0, 500_000],
      ["Contas", 300_000, 200_000],
      ["Faturas", 200_000, 100_000],
      ["Sobra", 0, 200_000],
    ]);
  });

  it("marca o papel de cada passo", () => {
    const steps = toWaterfallSteps({
      incomeCents: 500_000,
      billsCents: 200_000,
      invoicesCents: 100_000,
    });

    expect(steps.map((step) => step.kind)).toEqual(["in", "out", "out", "total"]);
  });

  it("guarda o valor com sinal, separado da altura da barra", () => {
    const steps = toWaterfallSteps({
      incomeCents: 500_000,
      billsCents: 200_000,
      invoicesCents: 100_000,
    });

    expect(steps.map((step) => step.value)).toEqual([500_000, -200_000, -100_000, 200_000]);
  });

  it("desenha a sobra negativa abaixo do zero", () => {
    const steps = toWaterfallSteps({
      incomeCents: 100_000,
      billsCents: 200_000,
      invoicesCents: 50_000,
    });

    const sobra = steps[3];
    expect(sobra.value).toBe(-150_000);
    expect(sobra.offset).toBe(-150_000);
    expect(sobra.delta).toBe(150_000);
  });

  it("nao quebra num mes zerado", () => {
    const steps = toWaterfallSteps({ incomeCents: 0, billsCents: 0, invoicesCents: 0 });

    expect(steps).toHaveLength(4);
    expect(steps.every((step) => step.delta === 0)).toBe(true);
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/waterfall.test.ts`
Expected: FAIL — módulo não encontrado.

- [ ] **Step 3: Implementar a função pura**

Criar `apps/m-finance/lib/calculations/charts/waterfall.ts`:

```ts
export type WaterfallKind = "in" | "out" | "total";

export type WaterfallStep = {
  label: string;
  /** Onde a barra começa no eixo. Empilhado como série transparente. */
  offset: number;
  /** Altura da barra. Sempre positiva — o sinal mora em `value`. */
  delta: number;
  /** O valor com sinal, para tooltip e para escolher a cor do total. */
  value: number;
  kind: WaterfallKind;
};

/**
 * O cálculo central do mês, como cascata.
 *
 * Recharts não tem waterfall, e não precisa ter: uma barra flutuante é uma
 * série transparente de `offset` com a série visível de `delta` empilhada em
 * cima. Separar `delta` (altura, sempre positiva) de `value` (o número com
 * sinal) existe porque barra de altura negativa não desenha — mas o tooltip
 * precisa dizer "−R$ 2.000,00", não "R$ 2.000,00".
 */
export function toWaterfallSteps({
  incomeCents,
  billsCents,
  invoicesCents,
}: {
  incomeCents: number;
  billsCents: number;
  invoicesCents: number;
}): WaterfallStep[] {
  const afterBills = incomeCents - billsCents;
  const remaining = afterBills - invoicesCents;

  return [
    { label: "Receita", offset: 0, delta: incomeCents, value: incomeCents, kind: "in" },
    { label: "Contas", offset: afterBills, delta: billsCents, value: -billsCents, kind: "out" },
    {
      label: "Faturas",
      offset: remaining,
      delta: invoicesCents,
      value: -invoicesCents,
      kind: "out",
    },
    {
      label: "Sobra",
      // Negativa, a barra desce do zero; positiva, sobe dele.
      offset: remaining < 0 ? remaining : 0,
      delta: Math.abs(remaining),
      value: remaining,
      kind: "total",
    },
  ];
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/waterfall.test.ts`
Expected: PASS, 5 testes.

- [ ] **Step 5: Criar o componente**

Criar `apps/m-finance/components/charts/month-waterfall-chart.tsx`:

```tsx
"use client";

import { Bar, BarChart, Cell, ReferenceLine, Tooltip, XAxis, YAxis } from "recharts";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { toWaterfallSteps, type WaterfallStep } from "@/lib/calculations/charts/waterfall";
import { formatCurrency, formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_FILL, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function MonthWaterfallChart({
  incomeCents,
  billsCents,
  invoicesCents,
}: {
  incomeCents: number;
  billsCents: number;
  invoicesCents: number;
}) {
  const { ref, width } = useChartWidth();
  const steps = toWaterfallSteps({ incomeCents, billsCents, invoicesCents });

  if (incomeCents === 0 && billsCents === 0 && invoicesCents === 0) {
    return <InlineEmpty>Cadastre receita e contas para ver o mês em cascata.</InlineEmpty>;
  }

  return (
    <div className="w-full" ref={ref}>
      {width > 0 ? (
        <BarChart
          data={steps}
          height={220}
          margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
          width={width}
        >
          <XAxis
            axisLine={false}
            dataKey="label"
            tick={{ fill: COLORS.muted, fontSize: 12 }}
            tickLine={false}
          />
          <YAxis
            axisLine={false}
            tick={{ fill: COLORS.muted, fontSize: 11 }}
            tickFormatter={formatCurrencyCompact}
            tickLine={false}
            width={64}
          />
          <ReferenceLine stroke={CHART_GRID} y={0} />
          <Tooltip content={<WaterfallTooltip />} cursor={{ fill: CHART_CURSOR_FILL }} />
          {/* A base é a série que posiciona a barra e não se vê. */}
          <Bar dataKey="offset" fillOpacity={0} isAnimationActive={false} stackId="cascata" />
          <Bar dataKey="delta" isAnimationActive={false} radius={[3, 3, 0, 0]} stackId="cascata">
            {steps.map((step) => (
              <Cell fill={stepColor(step)} key={step.label} />
            ))}
          </Bar>
        </BarChart>
      ) : null}
    </div>
  );
}

/**
 * Cor por papel, dentro da rampa.
 *
 * Entrada usa o verde e saída a escala neutra porque `globals.css` registra
 * que dinheiro entrando e saindo continua colorido: é informação. O total vai
 * para o sódio quando sobra e para o vermelho quando falta — é o único ponto
 * do gráfico onde o acento do sistema se justifica.
 */
function stepColor(step: WaterfallStep) {
  if (step.kind === "in") return COLORS.positive;
  if (step.kind === "out") return COLORS.muted;
  return step.value < 0 ? COLORS.negative : COLORS.accent;
}

type WaterfallTooltipProps = {
  active?: boolean;
  payload?: { payload?: WaterfallStep }[];
};

/**
 * Tooltip próprio em vez do `CurrencyTooltip` compartilhado: ali cada série
 * vira uma linha, e aqui a série `offset` é andaime invisível que não deve
 * aparecer. O que importa é o `value` com sinal.
 */
function WaterfallTooltip({ active, payload }: WaterfallTooltipProps) {
  const step = payload?.[0]?.payload;
  if (!active || !step) return null;

  return (
    <div className="rounded-md border border-border-default bg-background-elevated px-3 py-2 text-xs shadow-lg">
      <p className="font-semibold text-text-primary">{step.label}</p>
      <p className="num mt-0.5 font-semibold text-text-secondary">
        {step.value < 0 ? "−" : ""}
        {formatCurrency(Math.abs(step.value))}
      </p>
    </div>
  );
}
```

- [ ] **Step 6: Encaixar na dashboard**

Em `apps/m-finance/app/(app)/app/dashboard/page.tsx`, adicionar o import junto dos outros de `@/components/charts`:

```tsx
import { MonthWaterfallChart } from "@/components/charts/month-waterfall-chart";
```

E inserir, **imediatamente antes** do bloco `{categoryData.length > 0 ? (`:

```tsx
      {currentMonth ? (
        <DashboardCard
          description="De onde veio, para onde foi, e o que sobra."
          title="O mês em cascata"
        >
          <MonthWaterfallChart
            billsCents={summary.totalBillsCents}
            incomeCents={summary.totalIncomeCents}
            invoicesCents={summary.totalInvoicesCents}
          />
        </DashboardCard>
      ) : null}
```

- [ ] **Step 7: Conferir tipo, lint e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint && npm test`
Expected: sem erro.

- [ ] **Step 8: Commit**

```bash
git add apps/m-finance/lib/calculations/charts/waterfall.ts apps/m-finance/lib/calculations/charts/waterfall.test.ts apps/m-finance/components/charts/month-waterfall-chart.tsx "apps/m-finance/app/(app)/app/dashboard/page.tsx"
git commit -m "feat(m-finance): o mes em cascata, e nao quatro numeros soltos"
```

---

## Task 6: `MetricSparkline` nos cards de métrica

Os quatro cards de `monthMetrics` dizem o valor do mês e nada sobre a direção. `monthlySnapshots` já guarda a série dos quatro campos.

**Files:**
- Create: `apps/m-finance/components/charts/metric-sparkline.tsx`
- Modify: `apps/m-finance/app/(app)/app/dashboard/page.tsx`

**Interfaces:**
- Consumes: `getMonthlySnapshots(userId)` de `@/lib/history` — devolve, do mais novo para o mais antigo, linhas com `totalIncomeCents`, `totalBillsCents`, `totalInvoicesCents`, `totalPaidCents`, `totalPendingCents`, `totalOverdueCents`, `estimatedRemainingCents`, `month`, `year`.
- Produces: `<MetricSparkline points={number[]} tone?: "accent" | "neutral" />`

- [ ] **Step 1: Criar o componente**

Criar `apps/m-finance/components/charts/metric-sparkline.tsx`:

```tsx
"use client";

import { Line, LineChart } from "recharts";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { COLORS } from "@/lib/ui/colors";

/**
 * Tendência dentro do card de métrica.
 *
 * Sem eixo, sem grade, sem tooltip e sem ponto: o card já diz o valor exato, e
 * o que falta é a direção. Um sparkline que ganha eixo vira gráfico, e aí
 * compete com o número que ele deveria acompanhar.
 *
 * Menos de dois pontos não desenha nada — uma linha de um ponto é ruído com
 * aparência de informação.
 */
export function MetricSparkline({
  points,
  tone = "neutral",
}: {
  /** Do mais antigo para o mais novo. */
  points: number[];
  tone?: "accent" | "neutral";
}) {
  const { ref, width } = useChartWidth();

  if (points.length < 2) return null;

  const data = points.map((value, index) => ({ index, value }));

  return (
    <div aria-hidden="true" className="mt-3 h-10 w-full" ref={ref}>
      {width > 0 ? (
        <LineChart data={data} height={40} margin={{ left: 0, right: 0, top: 4, bottom: 4 }} width={width}>
          <Line
            dataKey="value"
            dot={false}
            isAnimationActive={false}
            stroke={tone === "accent" ? COLORS.accent : COLORS.muted}
            strokeWidth={1}
            type="monotone"
          />
        </LineChart>
      ) : null}
    </div>
  );
}
```

> `aria-hidden`: o valor já está no card em texto. Um leitor de tela lendo o mesmo dado duas vezes, a segunda como gráfico sem rótulo, atrapalha.

- [ ] **Step 2: Buscar os snapshots na dashboard**

Em `apps/m-finance/app/(app)/app/dashboard/page.tsx`, adicionar os imports:

```tsx
import { MetricSparkline } from "@/components/charts/metric-sparkline";
import { getMonthlySnapshots } from "@/lib/history";
```

E, junto das outras buscas (logo depois da linha de `settings`), adicionar:

```tsx
  const snapshots = appUser ? await getMonthlySnapshots(appUser.id) : [];
  // Os snapshots vêm do mais novo para o mais antigo; a linha lê da esquerda
  // para a direita, então a série vai ao contrário.
  const history = [...snapshots].reverse();
```

- [ ] **Step 3: Ligar a série a cada métrica**

Ainda em `dashboard/page.tsx`, substituir a montagem de `monthMetrics` por:

```tsx
  const monthMetrics = [
    {
      label: "Receita prevista",
      value: summary.totalIncomeCents,
      note: `${realIncomes.length} entrada${realIncomes.length === 1 ? "" : "s"}`,
      points: history.map((snapshot) => snapshot.totalIncomeCents),
      tone: "neutral" as const,
    },
    {
      label: "Comprometido",
      value: totalCommittedCents,
      note: "Contas e faturas",
      points: history.map(
        (snapshot) => snapshot.totalBillsCents + snapshot.totalInvoicesCents,
      ),
      tone: "neutral" as const,
    },
    {
      label: "Pago",
      value: summary.totalPaidCents,
      note: allSettled ? "Mês liquidado" : "Já resolvido",
      points: history.map((snapshot) => snapshot.totalPaidCents),
      tone: "neutral" as const,
    },
    {
      label: "Sobra estimada",
      value: summary.estimatedRemainingCents,
      note: "Depois de pagar tudo",
      points: history.map((snapshot) => snapshot.estimatedRemainingCents),
      // A sobra é a métrica que a pessoa acompanha; ela ganha o acento.
      tone: "accent" as const,
    },
  ];
```

E, no `map` que desenha os cards, adicionar o sparkline logo depois do `<p>` da nota:

```tsx
            <p className="mt-1 text-xs text-text-muted">{metric.note}</p>
            <MetricSparkline points={metric.points} tone={metric.tone} />
```

- [ ] **Step 4: Conferir tipo, lint e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint && npm test`
Expected: sem erro.

- [ ] **Step 5: Commit**

```bash
git add apps/m-finance/components/charts/metric-sparkline.tsx "apps/m-finance/app/(app)/app/dashboard/page.tsx"
git commit -m "feat(m-finance): o card diz o valor, e agora tambem a direcao"
```

---

## Task 7: `DueDateHeatmap` — onde o mês aperta

Responde "em que semana eu preciso ter dinheiro", que hoje só se responde abrindo `/calendar` e contando.

**Files:**
- Create: `apps/m-finance/lib/calculations/charts/due-dates.ts`
- Test: `apps/m-finance/lib/calculations/charts/due-dates.test.ts`
- Create: `apps/m-finance/components/charts/due-date-heatmap.tsx`
- Modify: `apps/m-finance/app/(app)/app/dashboard/page.tsx`

**Interfaces:**
- Consumes: nada.
- Produces: `type DueDateBucket = { day: number; cents: number; intensity: number }` e `toDueDateBuckets(items: { dueDate: string; amountCents: number }[], year: number, month: number): DueDateBucket[]`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/calculations/charts/due-dates.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { toDueDateBuckets } from "@/lib/calculations/charts/due-dates";

describe("toDueDateBuckets", () => {
  it("devolve uma celula por dia do mes", () => {
    expect(toDueDateBuckets([], 2026, 4)).toHaveLength(30);
    expect(toDueDateBuckets([], 2026, 8)).toHaveLength(31);
  });

  it("conhece fevereiro comum e bissexto", () => {
    expect(toDueDateBuckets([], 2026, 2)).toHaveLength(28);
    expect(toDueDateBuckets([], 2024, 2)).toHaveLength(29);
  });

  it("soma os vencimentos do mesmo dia", () => {
    const buckets = toDueDateBuckets(
      [
        { dueDate: "2026-08-10", amountCents: 30_000 },
        { dueDate: "2026-08-10", amountCents: 20_000 },
        { dueDate: "2026-08-05", amountCents: 10_000 },
      ],
      2026,
      8,
    );

    expect(buckets[9].cents).toBe(50_000);
    expect(buckets[4].cents).toBe(10_000);
  });

  it("deixa em zero o dia sem vencimento", () => {
    const buckets = toDueDateBuckets(
      [{ dueDate: "2026-08-10", amountCents: 30_000 }],
      2026,
      8,
    );

    expect(buckets[0].cents).toBe(0);
    expect(buckets[0].intensity).toBe(0);
  });

  it("normaliza a intensidade pelo dia mais pesado", () => {
    const buckets = toDueDateBuckets(
      [
        { dueDate: "2026-08-10", amountCents: 100_000 },
        { dueDate: "2026-08-20", amountCents: 50_000 },
      ],
      2026,
      8,
    );

    expect(buckets[9].intensity).toBe(1);
    expect(buckets[19].intensity).toBe(0.5);
  });

  it("ignora vencimento de outro mes", () => {
    const buckets = toDueDateBuckets(
      [
        { dueDate: "2026-09-10", amountCents: 100_000 },
        { dueDate: "2026-08-10", amountCents: 40_000 },
      ],
      2026,
      8,
    );

    expect(buckets[9].cents).toBe(40_000);
    expect(buckets.reduce((total, bucket) => total + bucket.cents, 0)).toBe(40_000);
  });

  it("nao quebra num mes sem nenhum vencimento", () => {
    const buckets = toDueDateBuckets([], 2026, 8);

    expect(buckets.every((bucket) => bucket.intensity === 0)).toBe(true);
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/due-dates.test.ts`
Expected: FAIL — módulo não encontrado.

- [ ] **Step 3: Implementar a função pura**

Criar `apps/m-finance/lib/calculations/charts/due-dates.ts`:

```ts
export type DueDateBucket = {
  /** 1 a 28/29/30/31. */
  day: number;
  cents: number;
  /** 0 a 1, relativo ao dia mais pesado do mês. */
  intensity: number;
};

/**
 * Quanto vence em cada dia do mês.
 *
 * A intensidade é relativa ao dia mais pesado, e não a um teto absoluto: o
 * gráfico responde "onde este mês aperta", não "este mês é pior que março".
 * Comparar meses é trabalho do histórico.
 */
export function toDueDateBuckets(
  items: { dueDate: string; amountCents: number }[],
  year: number,
  month: number,
): DueDateBucket[] {
  const days = daysInMonth(year, month);
  const cents = new Array<number>(days).fill(0);

  for (const item of items) {
    const parsed = parseIsoDate(item.dueDate);
    if (!parsed) continue;
    if (parsed.year !== year || parsed.month !== month) continue;
    if (parsed.day < 1 || parsed.day > days) continue;
    cents[parsed.day - 1] += item.amountCents;
  }

  const heaviest = Math.max(...cents, 0);

  return cents.map((value, index) => ({
    day: index + 1,
    cents: value,
    intensity: heaviest === 0 ? 0 : value / heaviest,
  }));
}

/** Dia 0 do mês seguinte é o último dia deste. Cobre bissexto sem tabela. */
function daysInMonth(year: number, month: number) {
  return new Date(year, month, 0).getDate();
}

/**
 * `YYYY-MM-DD` na mão, de propósito.
 *
 * `new Date("2026-08-10")` é interpretado como meia-noite UTC, que no fuso do
 * Brasil é dia 9. Num heatmap por dia isso desloca a coluna inteira.
 */
function parseIsoDate(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  if (!Number.isFinite(year) || !Number.isFinite(month) || !Number.isFinite(day)) {
    return null;
  }
  return { year, month, day };
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/due-dates.test.ts`
Expected: PASS, 7 testes.

- [ ] **Step 5: Criar o componente**

Este não usa recharts — são 31 divs. Recharts para uma faixa de quadrados seria trazer um motor de eixos para desenhar retângulos.

Criar `apps/m-finance/components/charts/due-date-heatmap.tsx`:

```tsx
"use client";

import { InlineEmpty } from "@/components/ui/inline-empty";
import { toDueDateBuckets } from "@/lib/calculations/charts/due-dates";
import { formatCurrency } from "@/lib/formatters/currency";

/**
 * A pressão do mês, dia a dia.
 *
 * Sem recharts: são retângulos numa grade, e trazer um motor de eixos para
 * desenhar retângulos custa mais do que entrega. A intensidade percorre a
 * opacidade do sódio — uma rampa, como manda `lib/ui/colors.ts`, e não uma
 * escala de matizes.
 */
export function DueDateHeatmap({
  items,
  month,
  year,
}: {
  items: { dueDate: string; amountCents: number }[];
  month: number;
  year: number;
}) {
  const buckets = toDueDateBuckets(items, year, month);
  const total = buckets.reduce((sum, bucket) => sum + bucket.cents, 0);

  if (total === 0) {
    return <InlineEmpty>Nenhum vencimento neste mês.</InlineEmpty>;
  }

  const heaviest = buckets.reduce((worst, bucket) =>
    bucket.cents > worst.cents ? bucket : worst,
  );

  return (
    <div>
      <ol className="flex flex-wrap gap-1">
        {buckets.map((bucket) => (
          <li key={bucket.day}>
            <div
              className="flex h-8 w-8 items-center justify-center rounded-sm border border-border-subtle text-[10px] text-text-muted"
              // Opacidade e não cor: a rampa é de clareza, e o piso de 0.08
              // mantém a célula vazia visível como célula.
              style={{
                backgroundColor: `color-mix(in srgb, var(--signal-fill) ${
                  bucket.cents === 0 ? 0 : 8 + bucket.intensity * 72
                }%, transparent)`,
              }}
              title={`Dia ${bucket.day}: ${formatCurrency(bucket.cents)}`}
            >
              {bucket.day}
            </div>
          </li>
        ))}
      </ol>
      <p className="mt-3 text-xs text-text-muted">
        Dia mais pesado: <span className="num text-text-secondary">{heaviest.day}</span>, com{" "}
        <span className="num text-text-secondary">{formatCurrency(heaviest.cents)}</span> vencendo.
      </p>
    </div>
  );
}
```

- [ ] **Step 6: Encaixar na dashboard**

Em `apps/m-finance/app/(app)/app/dashboard/page.tsx`, adicionar o import:

```tsx
import { DueDateHeatmap } from "@/components/charts/due-date-heatmap";
```

E inserir, **imediatamente depois** do card "O mês em cascata" da Task 5:

```tsx
      {currentMonth ? (
        <DashboardCard
          description="Onde os vencimentos se concentram."
          title="Pressão do mês"
        >
          <DueDateHeatmap
            items={[...realBills, ...realInvoices].map((item) => ({
              dueDate: item.dueDate,
              amountCents: item.amountCents,
            }))}
            month={currentMonth.month}
            year={currentMonth.year}
          />
        </DashboardCard>
      ) : null}
```

- [ ] **Step 7: Conferir tipo, lint e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint && npm test`
Expected: sem erro.

- [ ] **Step 8: Commit**

```bash
git add apps/m-finance/lib/calculations/charts/due-dates.ts apps/m-finance/lib/calculations/charts/due-dates.test.ts apps/m-finance/components/charts/due-date-heatmap.tsx "apps/m-finance/app/(app)/app/dashboard/page.tsx"
git commit -m "feat(m-finance): a faixa que mostra em que semana o mes aperta"
```

---

## Task 8: `BudgetThresholdBand` no `/budgets`

**Depende da Task 4.** A barra do `budget-card` já diz quanto do teto foi usado; o que falta é "estourei porque gastei demais ou porque o mês ainda não acabou".

**Files:**
- Create: `apps/m-finance/lib/calculations/charts/budget-burndown.ts`
- Test: `apps/m-finance/lib/calculations/charts/budget-burndown.test.ts`
- Create: `apps/m-finance/components/charts/budget-threshold-band.tsx`
- Modify: `apps/m-finance/app/(app)/app/budgets/page.tsx`

**Interfaces:**
- Consumes: `toDueDateBuckets` não; parseia data por conta própria. `formatCurrencyCompact` (Task 1).
- Produces: `type BurndownPoint = { day: number; spentCents: number; limitCents: number }`, `toBudgetBurndown(items, limitCents, year, month): BurndownPoint[]` e `crossingDay(points: BurndownPoint[]): number | null`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/calculations/charts/budget-burndown.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { crossingDay, toBudgetBurndown } from "@/lib/calculations/charts/budget-burndown";

const LIMITE = 100_000;

describe("toBudgetBurndown", () => {
  it("devolve uma entrada por dia do mes", () => {
    expect(toBudgetBurndown([], LIMITE, 2026, 8)).toHaveLength(31);
    expect(toBudgetBurndown([], LIMITE, 2026, 2)).toHaveLength(28);
  });

  it("acumula e nunca decresce", () => {
    const points = toBudgetBurndown(
      [
        { dueDate: "2026-08-05", amountCents: 30_000 },
        { dueDate: "2026-08-15", amountCents: 25_000 },
      ],
      LIMITE,
      2026,
      8,
    );

    expect(points[3].spentCents).toBe(0);
    expect(points[4].spentCents).toBe(30_000);
    expect(points[13].spentCents).toBe(30_000);
    expect(points[14].spentCents).toBe(55_000);
    expect(points[30].spentCents).toBe(55_000);

    const decrescente = points.some(
      (point, index) => index > 0 && point.spentCents < points[index - 1].spentCents,
    );
    expect(decrescente).toBe(false);
  });

  it("repete o limite em todos os pontos, para virar linha reta", () => {
    const points = toBudgetBurndown([], LIMITE, 2026, 8);
    expect(points.every((point) => point.limitCents === LIMITE)).toBe(true);
  });

  it("ignora lancamento de outro mes", () => {
    const points = toBudgetBurndown(
      [{ dueDate: "2026-07-05", amountCents: 30_000 }],
      LIMITE,
      2026,
      8,
    );

    expect(points[30].spentCents).toBe(0);
  });
});

describe("crossingDay", () => {
  it("aponta o primeiro dia em que o acumulado passa do limite", () => {
    const points = toBudgetBurndown(
      [
        { dueDate: "2026-08-05", amountCents: 60_000 },
        { dueDate: "2026-08-12", amountCents: 50_000 },
      ],
      LIMITE,
      2026,
      8,
    );

    expect(crossingDay(points)).toBe(12);
  });

  it("devolve nulo quando o limite aguenta o mes", () => {
    const points = toBudgetBurndown(
      [{ dueDate: "2026-08-05", amountCents: 60_000 }],
      LIMITE,
      2026,
      8,
    );

    expect(crossingDay(points)).toBeNull();
  });

  it("nao considera cruzamento quando o gasto empata com o limite", () => {
    const points = toBudgetBurndown(
      [{ dueDate: "2026-08-05", amountCents: LIMITE }],
      LIMITE,
      2026,
      8,
    );

    expect(crossingDay(points)).toBeNull();
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/budget-burndown.test.ts`
Expected: FAIL — módulo não encontrado.

- [ ] **Step 3: Implementar a função pura**

Criar `apps/m-finance/lib/calculations/charts/budget-burndown.ts`:

```ts
export type BurndownPoint = {
  day: number;
  /** Acumulado do dia 1 até este dia. */
  spentCents: number;
  /** O mesmo em todos os pontos — é o que desenha a linha reta do teto. */
  limitCents: number;
};

/**
 * O gasto acumulado dia a dia contra o teto.
 *
 * A barra de progresso do `budget-card` responde "quanto do teto já foi",
 * e é uma resposta sem tempo dentro. Estourar no dia 8 e estourar no dia 28
 * são situações diferentes, e só o acumulado no eixo do mês separa as duas.
 */
export function toBudgetBurndown(
  items: { dueDate: string; amountCents: number }[],
  limitCents: number,
  year: number,
  month: number,
): BurndownPoint[] {
  const days = new Date(year, month, 0).getDate();
  const perDay = new Array<number>(days).fill(0);

  for (const item of items) {
    const [itemYear, itemMonth, itemDay] = item.dueDate.split("-").map(Number);
    if (itemYear !== year || itemMonth !== month) continue;
    if (!Number.isFinite(itemDay) || itemDay < 1 || itemDay > days) continue;
    perDay[itemDay - 1] += item.amountCents;
  }

  let running = 0;
  return perDay.map((value, index) => {
    running += value;
    return { day: index + 1, spentCents: running, limitCents };
  });
}

/**
 * O primeiro dia em que o acumulado ultrapassa o teto, ou `null` se o mês
 * inteiro couber. Empatar com o limite não é ultrapassar.
 */
export function crossingDay(points: BurndownPoint[]): number | null {
  const crossed = points.find((point) => point.spentCents > point.limitCents);
  return crossed ? crossed.day : null;
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/budget-burndown.test.ts`
Expected: PASS, 7 testes.

- [ ] **Step 5: Criar o componente**

Criar `apps/m-finance/components/charts/budget-threshold-band.tsx`:

```tsx
"use client";

import { CartesianGrid, Line, LineChart, ReferenceLine, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { crossingDay, toBudgetBurndown } from "@/lib/calculations/charts/budget-burndown";
import { formatCurrency, formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function BudgetThresholdBand({
  items,
  limitCents,
  month,
  year,
}: {
  items: { dueDate: string; amountCents: number }[];
  limitCents: number;
  month: number;
  year: number;
}) {
  const { ref, width } = useChartWidth();
  const points = toBudgetBurndown(items, limitCents, year, month);
  const crossed = crossingDay(points);

  if (points[points.length - 1].spentCents === 0) {
    return <InlineEmpty>Nada lançado neste orçamento ainda.</InlineEmpty>;
  }

  return (
    <div>
      <div className="w-full" ref={ref}>
        {width > 0 ? (
          <LineChart
            data={points}
            height={200}
            margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
            width={width}
          >
            <CartesianGrid stroke={CHART_GRID} vertical={false} />
            <XAxis
              axisLine={false}
              dataKey="day"
              // Um rótulo a cada cinco dias: 31 números seguidos viram borrão.
              interval={4}
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
            />
            <YAxis
              axisLine={false}
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickFormatter={formatCurrencyCompact}
              tickLine={false}
              width={64}
            />
            {/* A faixa de alerta do app é 80%, a mesma de `lib/budgets.ts`. */}
            <ReferenceLine
              stroke={COLORS.textSecondary}
              strokeDasharray="2 4"
              y={limitCents * 0.8}
            />
            <ReferenceLine stroke={COLORS.negative} strokeDasharray="4 4" y={limitCents} />
            <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
            <Line
              dataKey="spentCents"
              dot={false}
              isAnimationActive={false}
              name="Gasto acumulado"
              stroke={crossed ? COLORS.negative : COLORS.accent}
              strokeWidth={2}
              type="monotone"
            />
          </LineChart>
        ) : null}
      </div>
      <p className="mt-3 text-xs text-text-muted">
        {crossed
          ? `Passou de ${formatCurrency(limitCents)} no dia ${crossed}.`
          : `O mês coube em ${formatCurrency(limitCents)}.`}
      </p>
    </div>
  );
}
```

- [ ] **Step 6: Alimentar o componente na página**

O gráfico precisa dos lançamentos com data, que `getBudgetsByMonth` não devolve — ele devolve só o total. Adicionar em `apps/m-finance/lib/budgets.ts`, ao fim do arquivo:

```ts
/**
 * Os lançamentos do orçamento com data, para o acumulado dia a dia.
 *
 * Separado de `getBudgetsByMonth` de propósito: o card precisa de um número e
 * a página inteira o chama para cada orçamento. Trazer a lista de lançamentos
 * junto encareceria todo mundo por causa de um gráfico.
 */
export async function getBudgetEntries(
  userId: string,
  monthId: string,
  type: BudgetType,
  categoryId: string | null,
  cardId: string | null,
): Promise<{ dueDate: string; amountCents: number }[]> {
  if (!db) return [];

  if (type === "card" && cardId) {
    const rows = await db
      .select({
        dueDate: creditCardExpenses.purchaseDate,
        amountCents: creditCardExpenses.amountCents,
      })
      .from(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.userId, userId),
          eq(creditCardExpenses.cardId, cardId),
          eq(creditCardExpenses.monthId, monthId),
        ),
      );

    // `purchase_date` é nullable no schema (db/schema.ts:244). Gasto sem data
    // não tem onde cair na linha do tempo, então fica de fora do acumulado —
    // ele continua contando no total que o card já mostra.
    return rows.flatMap((row) =>
      row.dueDate ? [{ dueDate: row.dueDate, amountCents: row.amountCents }] : [],
    );
  }

  const conditions = [eq(bills.userId, userId), eq(bills.monthId, monthId)];
  if (type === "category" && categoryId) {
    conditions.push(eq(bills.categoryId, categoryId));
  }

  const billRows = await db
    .select({ dueDate: bills.dueDate, amountCents: bills.amountCents })
    .from(bills)
    .where(and(...conditions));

  if (type !== "total") return billRows;

  const invoiceRows = await db
    .select({
      dueDate: creditCardInvoices.dueDate,
      amountCents: creditCardInvoices.amountCents,
    })
    .from(creditCardInvoices)
    .where(and(eq(creditCardInvoices.userId, userId), eq(creditCardInvoices.monthId, monthId)));

  return [...billRows, ...invoiceRows];
}
```

Adicionar `creditCardExpenses` e `creditCardInvoices` ao import de `@/db/schema` no topo do arquivo se ainda não estiverem lá — `budgets.ts` já importa os dois na linha 3.

- [ ] **Step 7: Encaixar na página de orçamentos**

Em `apps/m-finance/app/(app)/app/budgets/page.tsx`, adicionar os imports:

```tsx
import { BudgetThresholdBand } from "@/components/charts/budget-threshold-band";
import { getBudgetEntries, getBudgetsByMonth } from "@/lib/budgets";
```

(substituindo o import atual de `getBudgetsByMonth`).

Depois da linha `const budgetList = await getBudgetsByMonth(month.id, appUser.id);`, adicionar:

```tsx
  // Só o orçamento total ganha gráfico. Um por card multiplicaria a consulta
  // pelo número de orçamentos e repetiria a mesma forma na tela inteira.
  const totalBudget = budgetList.find((budget) => budget.budgetType === "total") ?? null;
  const totalEntries = totalBudget
    ? await getBudgetEntries(appUser.id, month.id, "total", null, null)
    : [];
```

E inserir, logo depois do bloco `<DashboardCard title="Resumo dos orçamentos">`:

```tsx
      {totalBudget ? (
        <DashboardCard
          description="Gasto acumulado contra o teto, dia a dia."
          title="O mês contra o limite"
        >
          <BudgetThresholdBand
            items={totalEntries}
            limitCents={totalBudget.limitCents}
            month={month.month}
            year={month.year}
          />
        </DashboardCard>
      ) : null}
```

- [ ] **Step 8: Conferir tipo, lint e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint && npm test`
Expected: sem erro.

- [ ] **Step 9: Commit**

```bash
git add apps/m-finance/lib/calculations/charts/budget-burndown.ts apps/m-finance/lib/calculations/charts/budget-burndown.test.ts apps/m-finance/components/charts/budget-threshold-band.tsx apps/m-finance/lib/budgets.ts "apps/m-finance/app/(app)/app/budgets/page.tsx"
git commit -m "feat(m-finance): estourar no dia 8 e estourar no dia 28 param de parecer igual"
```

---

## Task 9: `GoalPriorityMatrix` no `/goals`

Dez cards de meta não dizem qual está em risco. O scatter diz.

**Files:**
- Create: `apps/m-finance/lib/calculations/charts/goal-matrix.ts`
- Test: `apps/m-finance/lib/calculations/charts/goal-matrix.test.ts`
- Create: `apps/m-finance/components/charts/goal-priority-matrix.tsx`
- Modify: `apps/m-finance/app/(app)/app/goals/page.tsx`

**Interfaces:**
- Consumes: `GoalWithProgress` de `@/lib/goals` — `{ id, name, targetAmountCents, currentAmountCents, deadline: string | null, priority, status, notes, progressPercent, remainingCents }`.
- Produces: `type GoalMatrixPoint = { id: string; name: string; daysLeft: number; remainingPercent: number; remainingCents: number }` e `toGoalMatrix(goals: GoalWithProgress[], today: Date): { points: GoalMatrixPoint[]; withoutDeadline: GoalWithProgress[] }`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/calculations/charts/goal-matrix.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { toGoalMatrix } from "@/lib/calculations/charts/goal-matrix";
import type { GoalWithProgress } from "@/lib/goals";

const HOJE = new Date(2026, 7, 24); // 24 de agosto de 2026

function goal(overrides: Partial<GoalWithProgress> & Pick<GoalWithProgress, "id">): GoalWithProgress {
  return {
    name: `Meta ${overrides.id}`,
    targetAmountCents: 100_000,
    currentAmountCents: 25_000,
    deadline: "2026-09-23",
    priority: "medium",
    status: "active",
    notes: null,
    progressPercent: 25,
    remainingCents: 75_000,
    ...overrides,
  };
}

describe("toGoalMatrix", () => {
  it("mede os dias que faltam ate o prazo", () => {
    const { points } = toGoalMatrix([goal({ id: "a", deadline: "2026-09-23" })], HOJE);
    expect(points[0].daysLeft).toBe(30);
  });

  it("devolve dias negativos quando o prazo ja passou", () => {
    const { points } = toGoalMatrix([goal({ id: "a", deadline: "2026-08-14" })], HOJE);
    expect(points[0].daysLeft).toBe(-10);
  });

  it("separa as metas sem prazo em vez de inventar um eixo para elas", () => {
    const { points, withoutDeadline } = toGoalMatrix(
      [goal({ id: "com" }), goal({ id: "sem", deadline: null })],
      HOJE,
    );

    expect(points.map((point) => point.id)).toEqual(["com"]);
    expect(withoutDeadline.map((item) => item.id)).toEqual(["sem"]);
  });

  it("usa o quanto falta, nao o quanto ja foi", () => {
    const { points } = toGoalMatrix(
      [goal({ id: "a", progressPercent: 25, remainingCents: 75_000 })],
      HOJE,
    );

    expect(points[0].remainingPercent).toBe(75);
    expect(points[0].remainingCents).toBe(75_000);
  });

  it("deixa de fora meta concluida e arquivada", () => {
    const { points, withoutDeadline } = toGoalMatrix(
      [
        goal({ id: "ativa" }),
        goal({ id: "pausada", status: "paused" }),
        goal({ id: "concluida", status: "completed" }),
        goal({ id: "arquivada", status: "archived" }),
      ],
      HOJE,
    );

    expect(points.map((point) => point.id)).toEqual(["ativa", "pausada"]);
    expect(withoutDeadline).toEqual([]);
  });

  it("nao quebra sem nenhuma meta", () => {
    expect(toGoalMatrix([], HOJE)).toEqual({ points: [], withoutDeadline: [] });
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/goal-matrix.test.ts`
Expected: FAIL — módulo não encontrado.

- [ ] **Step 3: Implementar a função pura**

Criar `apps/m-finance/lib/calculations/charts/goal-matrix.ts`:

```ts
import type { GoalWithProgress } from "@/lib/goals";

export type GoalMatrixPoint = {
  id: string;
  name: string;
  /** Negativo quando o prazo já passou. */
  daysLeft: number;
  /** 0 a 100. Quanto ainda falta, não quanto já foi. */
  remainingPercent: number;
  remainingCents: number;
};

/**
 * As metas em duas dimensões: tempo que resta e distância do objetivo.
 *
 * O eixo Y é o que **falta**, e não o progresso, porque o gráfico existe para
 * mostrar risco: quem falta muito fica em cima, onde o olho vai primeiro. O
 * canto superior esquerdo — falta muito, sobra pouco tempo — é a meta em apuro.
 *
 * Meta sem prazo não entra. Inventar um `x` para ela seria mentir sobre o dado,
 * e ela sai numa lista à parte para não sumir da tela.
 */
export function toGoalMatrix(
  goals: GoalWithProgress[],
  today: Date,
): { points: GoalMatrixPoint[]; withoutDeadline: GoalWithProgress[] } {
  const tracked = goals.filter(
    (goal) => goal.status === "active" || goal.status === "paused",
  );

  const points: GoalMatrixPoint[] = [];
  const withoutDeadline: GoalWithProgress[] = [];

  for (const goal of tracked) {
    if (!goal.deadline) {
      withoutDeadline.push(goal);
      continue;
    }

    points.push({
      id: goal.id,
      name: goal.name,
      daysLeft: daysBetween(today, goal.deadline),
      remainingPercent: Math.max(0, 100 - goal.progressPercent),
      remainingCents: goal.remainingCents,
    });
  }

  return { points, withoutDeadline };
}

/**
 * Dias inteiros entre hoje e uma data `YYYY-MM-DD`.
 *
 * Os dois lados viram `Date.UTC` antes da subtração: em milissegundos locais,
 * um mês que atravessa mudança de horário devolve 29,96 dias e arredonda para
 * o dia errado.
 */
function daysBetween(today: Date, isoDate: string) {
  const [year, month, day] = isoDate.split("-").map(Number);
  const target = Date.UTC(year, month - 1, day);
  const from = Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());
  return Math.round((target - from) / 86_400_000);
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/goal-matrix.test.ts`
Expected: PASS, 6 testes.

- [ ] **Step 5: Criar o componente**

Criar `apps/m-finance/components/charts/goal-priority-matrix.tsx`:

```tsx
"use client";

import {
  CartesianGrid,
  ReferenceLine,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
  ZAxis,
} from "recharts";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { toGoalMatrix, type GoalMatrixPoint } from "@/lib/calculations/charts/goal-matrix";
import { formatCurrency } from "@/lib/formatters/currency";
import type { GoalWithProgress } from "@/lib/goals";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function GoalPriorityMatrix({
  goals,
  today,
}: {
  goals: GoalWithProgress[];
  /** Vem da página como ISO, para o Server Component não passar `Date` cru. */
  today: string;
}) {
  const { ref, width } = useChartWidth();
  const [year, month, day] = today.split("-").map(Number);
  const { points, withoutDeadline } = toGoalMatrix(goals, new Date(year, month - 1, day));

  if (points.length === 0) {
    return (
      <InlineEmpty>
        Nenhuma meta com prazo. Defina um prazo para ver quais estão apertadas.
      </InlineEmpty>
    );
  }

  return (
    <div>
      <div className="w-full" ref={ref}>
        {width > 0 ? (
          <ScatterChart
            height={260}
            margin={{ left: 0, right: 16, top: 8, bottom: 16 }}
            width={width}
          >
            <CartesianGrid stroke={CHART_GRID} />
            <XAxis
              axisLine={false}
              dataKey="daysLeft"
              name="Dias até o prazo"
              // Invertido: prazo curto à esquerda, junto do "falta muito" no
              // topo. O canto superior esquerdo vira o canto do risco.
              reversed
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
              type="number"
              unit=" d"
            />
            <YAxis
              axisLine={false}
              dataKey="remainingPercent"
              domain={[0, 100]}
              name="Falta"
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
              type="number"
              unit="%"
              width={44}
            />
            <ZAxis dataKey="remainingCents" range={[60, 400]} />
            {/* O prazo de hoje: à esquerda desta linha, a meta já venceu. */}
            <ReferenceLine stroke={COLORS.negative} strokeDasharray="3 3" x={0} />
            <Tooltip content={<GoalTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
            <Scatter
              data={points}
              fill={COLORS.accent}
              fillOpacity={0.7}
              isAnimationActive={false}
              name="Metas"
            />
          </ScatterChart>
        ) : null}
      </div>

      {withoutDeadline.length > 0 ? (
        <p className="mt-3 text-xs text-text-muted">
          Sem prazo, fora do gráfico:{" "}
          <span className="text-text-secondary">
            {withoutDeadline.map((goal) => goal.name).join(", ")}
          </span>
          .
        </p>
      ) : null}
    </div>
  );
}

type GoalTooltipProps = {
  active?: boolean;
  payload?: { payload?: GoalMatrixPoint }[];
};

function GoalTooltip({ active, payload }: GoalTooltipProps) {
  const point = payload?.[0]?.payload;
  if (!active || !point) return null;

  return (
    <div className="rounded-md border border-border-default bg-background-elevated px-3 py-2 text-xs shadow-lg">
      <p className="font-semibold text-text-primary">{point.name}</p>
      <p className="num mt-0.5 text-text-secondary">
        faltam {formatCurrency(point.remainingCents)} ({point.remainingPercent}%)
      </p>
      <p className="num mt-0.5 text-text-muted">
        {point.daysLeft < 0
          ? `prazo venceu há ${Math.abs(point.daysLeft)} dia(s)`
          : `${point.daysLeft} dia(s) até o prazo`}
      </p>
    </div>
  );
}
```

- [ ] **Step 6: Encaixar na página de metas**

Em `apps/m-finance/app/(app)/app/goals/page.tsx`, adicionar o import:

```tsx
import { GoalPriorityMatrix } from "@/components/charts/goal-priority-matrix";
```

E inserir, logo depois do bloco `<DashboardCard title="Resumo das metas">`:

```tsx
      {trackedGoals.length > 1 ? (
        <DashboardCard
          description="Quanto falta contra quanto tempo resta. Canto superior esquerdo é aperto."
          title="Metas em risco"
        >
          <GoalPriorityMatrix goals={goals} today={new Date().toISOString().slice(0, 10)} />
        </DashboardCard>
      ) : null}
```

> `trackedGoals.length > 1`: um scatter de um ponto não compara nada.

- [ ] **Step 7: Conferir tipo, lint e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint && npm test`
Expected: sem erro.

- [ ] **Step 8: Commit**

```bash
git add apps/m-finance/lib/calculations/charts/goal-matrix.ts apps/m-finance/lib/calculations/charts/goal-matrix.test.ts apps/m-finance/components/charts/goal-priority-matrix.tsx "apps/m-finance/app/(app)/app/goals/page.tsx"
git commit -m "feat(m-finance): a meta em apuro para de se esconder entre dez cards"
```

---

## Task 10: `SimulationProjectionChart` no `/simulator`

`SimulationResult.months` já entrega `baselineRemainingCents` e `remainingWithCents` por mês. O `recommendation` afirma em texto o que o gráfico mostra.

**Files:**
- Create: `apps/m-finance/lib/calculations/charts/simulation-series.ts`
- Test: `apps/m-finance/lib/calculations/charts/simulation-series.test.ts`
- Create: `apps/m-finance/components/charts/simulation-projection-chart.tsx`
- Modify: `apps/m-finance/components/simulator/simulation-list.tsx`

**Interfaces:**
- Consumes: `SimulationMonth` de `@/lib/calculations/simulator` — `{ month: number; year: number; baselineRemainingCents: number; impactCents: number; remainingWithCents: number; health }`. E `monthName(month: number): string` do mesmo módulo.
- Produces: `type SimulationPoint = { label: string; semCompra: number; comCompra: number }`, `toSimulationSeries(months: SimulationMonth[]): SimulationPoint[]` e `firstNegativeMonth(points: SimulationPoint[]): string | null`

- [ ] **Step 1: Escrever o teste que falha**

Criar `apps/m-finance/lib/calculations/charts/simulation-series.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import {
  firstNegativeMonth,
  toSimulationSeries,
} from "@/lib/calculations/charts/simulation-series";
import type { SimulationMonth } from "@/lib/calculations/simulator";

function month(overrides: Partial<SimulationMonth> & Pick<SimulationMonth, "month">): SimulationMonth {
  return {
    year: 2026,
    baselineRemainingCents: 100_000,
    impactCents: 40_000,
    remainingWithCents: 60_000,
    health: "positive",
    ...overrides,
  };
}

describe("toSimulationSeries", () => {
  it("rotula com mes abreviado e ano de dois digitos", () => {
    const points = toSimulationSeries([month({ month: 8 }), month({ month: 9 })]);
    expect(points.map((point) => point.label)).toEqual(["ago/26", "set/26"]);
  });

  it("separa a sobra sem e com a compra", () => {
    const points = toSimulationSeries([
      month({ month: 8, baselineRemainingCents: 100_000, remainingWithCents: 60_000 }),
    ]);

    expect(points[0].semCompra).toBe(100_000);
    expect(points[0].comCompra).toBe(60_000);
  });

  it("aguenta uma projecao de um mes so", () => {
    expect(toSimulationSeries([month({ month: 8 })])).toHaveLength(1);
  });

  it("aguenta uma projecao vazia", () => {
    expect(toSimulationSeries([])).toEqual([]);
  });
});

describe("firstNegativeMonth", () => {
  it("aponta o primeiro mes em que a compra derruba a sobra abaixo de zero", () => {
    const points = toSimulationSeries([
      month({ month: 8, remainingWithCents: 20_000 }),
      month({ month: 9, remainingWithCents: -5_000 }),
      month({ month: 10, remainingWithCents: -30_000 }),
    ]);

    expect(firstNegativeMonth(points)).toBe("set/26");
  });

  it("devolve nulo quando a sobra aguenta a projecao inteira", () => {
    const points = toSimulationSeries([
      month({ month: 8, remainingWithCents: 20_000 }),
      month({ month: 9, remainingWithCents: 10_000 }),
    ]);

    expect(firstNegativeMonth(points)).toBeNull();
  });

  it("nao considera zero como negativo", () => {
    const points = toSimulationSeries([month({ month: 8, remainingWithCents: 0 })]);
    expect(firstNegativeMonth(points)).toBeNull();
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/simulation-series.test.ts`
Expected: FAIL — módulo não encontrado.

- [ ] **Step 3: Implementar a função pura**

Criar `apps/m-finance/lib/calculations/charts/simulation-series.ts`:

```ts
import { monthName, type SimulationMonth } from "@/lib/calculations/simulator";

export type SimulationPoint = {
  /** `ago/26`, no mesmo formato dos rótulos do histórico. */
  label: string;
  semCompra: number;
  comCompra: number;
};

/**
 * A projeção do simulador como duas linhas.
 *
 * O `recommendation` diz em texto que a compra cabe ou não cabe. Duas linhas
 * dizem **quando** ela deixa de caber, que é a informação que decide entre
 * comprar agora e comprar em três meses.
 */
export function toSimulationSeries(months: SimulationMonth[]): SimulationPoint[] {
  return months.map((month) => ({
    label: `${monthName(month.month).slice(0, 3)}/${String(month.year).slice(2)}`,
    semCompra: month.baselineRemainingCents,
    comCompra: month.remainingWithCents,
  }));
}

/**
 * O primeiro mês em que a sobra com a compra fica negativa, ou `null` se ela
 * aguentar a projeção inteira. Zerar não é ficar negativo.
 */
export function firstNegativeMonth(points: SimulationPoint[]): string | null {
  const breaking = points.find((point) => point.comCompra < 0);
  return breaking ? breaking.label : null;
}
```

- [ ] **Step 4: Rodar e ver passar**

Run: `cd apps/m-finance && npx vitest run lib/calculations/charts/simulation-series.test.ts`
Expected: PASS, 7 testes.

- [ ] **Step 5: Criar o componente**

Criar `apps/m-finance/components/charts/simulation-projection-chart.tsx`:

```tsx
"use client";

import { CartesianGrid, Legend, Line, LineChart, ReferenceLine, Tooltip, XAxis, YAxis } from "recharts";
import { CurrencyTooltip } from "@/components/charts/chart-tooltip";
import { useChartWidth } from "@/components/charts/use-chart-width";
import { InlineEmpty } from "@/components/ui/inline-empty";
import {
  firstNegativeMonth,
  toSimulationSeries,
} from "@/lib/calculations/charts/simulation-series";
import type { SimulationMonth } from "@/lib/calculations/simulator";
import { formatCurrencyCompact } from "@/lib/formatters/currency";
import { CHART_CURSOR_STROKE, CHART_GRID, COLORS } from "@/lib/ui/colors";

export function SimulationProjectionChart({ months }: { months: SimulationMonth[] }) {
  const { ref, width } = useChartWidth();
  const points = toSimulationSeries(months);
  const breaking = firstNegativeMonth(points);

  if (points.length < 2) {
    return <InlineEmpty>A projeção precisa de pelo menos dois meses.</InlineEmpty>;
  }

  return (
    <div>
      <div className="w-full" ref={ref}>
        {width > 0 ? (
          <LineChart
            data={points}
            height={200}
            margin={{ left: 0, right: 8, top: 8, bottom: 0 }}
            width={width}
          >
            <CartesianGrid stroke={CHART_GRID} vertical={false} />
            <XAxis
              axisLine={false}
              dataKey="label"
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickLine={false}
            />
            <YAxis
              axisLine={false}
              tick={{ fill: COLORS.muted, fontSize: 11 }}
              tickFormatter={formatCurrencyCompact}
              tickLine={false}
              width={64}
            />
            {/* O zero é o assunto: onde a linha de baixo o cruza, a compra
                deixou de caber. */}
            <ReferenceLine stroke={COLORS.negative} strokeDasharray="4 4" y={0} />
            <Tooltip content={<CurrencyTooltip />} cursor={{ stroke: CHART_CURSOR_STROKE }} />
            <Legend
              formatter={(value) => (
                <span style={{ color: COLORS.muted, fontSize: 12 }}>{value}</span>
              )}
              iconType="plainline"
            />
            <Line
              dataKey="semCompra"
              dot={false}
              isAnimationActive={false}
              name="Sem a compra"
              stroke={COLORS.muted}
              strokeWidth={1}
              type="monotone"
            />
            <Line
              dataKey="comCompra"
              dot={false}
              isAnimationActive={false}
              name="Com a compra"
              stroke={breaking ? COLORS.negative : COLORS.accent}
              strokeWidth={2}
              type="monotone"
            />
          </LineChart>
        ) : null}
      </div>
      <p className="mt-3 text-xs text-text-muted">
        {breaking
          ? `A sobra fica negativa em ${breaking}.`
          : "A sobra aguenta a projeção inteira."}
      </p>
    </div>
  );
}
```

- [ ] **Step 6: Encaixar na lista de simulações**

Em `apps/m-finance/components/simulator/simulation-list.tsx`, adicionar o import:

```tsx
import { SimulationProjectionChart } from "@/components/charts/simulation-projection-chart";
```

O arquivo já tem um bloco `Impacto por mês` com a grade de meses. O gráfico entra **entre o título e a grade**: ele mostra a forma, a grade continua dando o número exato e o `StatusBadge` de cada mês, que o gráfico não carrega.

Localizar:

```tsx
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
              Impacto por mês
            </p>
            <div className="grid gap-2 sm:grid-cols-2">
```

E inserir entre as duas linhas:

```tsx
            <SimulationProjectionChart months={simulation.result.months} />
```

A variável do `map` é `simulation`, e o tipo é `StoredSimulation`, cujo campo `result` é `SimulationResult` (montado em `lib/simulations.ts:36`).

- [ ] **Step 7: Conferir tipo, lint e suíte**

Run: `cd apps/m-finance && npx tsc --noEmit && npm run lint && npm test`
Expected: sem erro.

- [ ] **Step 8: Commit**

```bash
git add apps/m-finance/lib/calculations/charts/simulation-series.ts apps/m-finance/lib/calculations/charts/simulation-series.test.ts apps/m-finance/components/charts/simulation-projection-chart.tsx apps/m-finance/components/simulator/simulation-list.tsx
git commit -m "feat(m-finance): o simulador mostra em que mes a compra quebra"
```

---

## Task 11: Conferência na tela

Nenhum teste deste plano desenha um pixel. O ambiente vitest é `node`, sem DOM, e os componentes recharts não são testados por escolha registrada no spec §9. A conferência é olhar.

**Files:** nenhum. Esta task não escreve código — ela decide se o que foi escrito fica.

- [ ] **Step 1: Subir o app**

Run: `cd apps/m-finance && npm run dev`
Abrir `http://localhost:3000/app/dashboard`.

> Se o app não subir por falta de `.env` (Supabase, `DATABASE_URL`), pare e diga isso ao proprietário em vez de contornar. Sem banco, todas as páginas caem no `EmptyState` e nenhum gráfico aparece — a conferência não teria valor.

- [ ] **Step 2: Percorrer as cinco telas**

Em cada uma, conferir três coisas: o gráfico aparece, o vazio aparece quando não há dado, e nada estoura a largura no viewport estreito (375px, pelo devtools).

- `/app/dashboard` — cascata, pressão do mês, sparkline nos quatro cards, barra de categorias com valor e %
- `/app/history` — evolução com eixo Y e a sobra em destaque
- `/app/budgets` — o mês contra o limite
- `/app/goals` — metas em risco
- `/app/simulator` — projeção dentro de uma simulação salva

- [ ] **Step 3: Conferir a lei de design**

Abrir o devtools e procurar, no DOM dos gráficos, por `<linearGradient>` e `<radialGradient>`. Não deve haver nenhum. Conferir que nenhuma série anima ao carregar a página (recarregue e olhe).

- [ ] **Step 4: Rodar a suíte inteira e o build**

Run: `cd apps/m-finance && npm test && npm run lint && npm run build`
Expected: tudo passando, build sem erro.

- [ ] **Step 5: Relatar**

Escrever ao proprietário o que foi visto em cada tela, com o que não deu para conferir e por quê. Não afirmar que está pronto sem ter olhado.

---

## Notas de execução

**Ordem e dependências.** A Task 1 é pré-requisito das Tasks 3, 5, 8 e 10 (todas usam `formatCurrencyCompact`). A Task 4 é pré-requisito da Task 8. Fora isso, as tasks são independentes e podem ir em qualquer ordem.

**O que este plano assume que existe** e deve ser conferido no primeiro erro de import: `useChartWidth` em `components/charts/use-chart-width.ts`, `CurrencyTooltip` em `components/charts/chart-tooltip.tsx`, `InlineEmpty` em `components/ui/inline-empty.tsx`, `DashboardCard` em `components/dashboard/dashboard-card.tsx`, e os tokens `COLORS`, `CHART_PALETTE`, `CHART_GRID`, `CHART_CURSOR_FILL`, `CHART_CURSOR_STROKE` em `lib/ui/colors.ts`.

**Recharts 3.8.1.** As props usadas aqui (`isAnimationActive`, `LabelList content`, `ReferenceLine`, `ZAxis`, `Scatter`, `stackId`) existem nesta versão, mas a tipagem de `content` e do segundo argumento de `Legend formatter` mudou entre as majors. Onde o `tsc` reclamar, tipar localmente com um tipo próprio — é o que `chart-tooltip.tsx` já faz, e o comentário dele registra o porquê.

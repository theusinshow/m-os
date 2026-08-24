# Gráficos do M-Finance — a forma emprestada, o vocabulário não — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-24

**Baseline:** M/OS no commit `1f537fb`. O app é `apps/m-finance` (Next.js, Tailwind v4, recharts já instalado). Hoje existem exatamente dois gráficos — `components/charts/category-breakdown-chart.tsx` e `components/charts/history-trend-chart.tsx` — mais a infraestrutura compartilhada em `use-chart-width.ts` e `chart-tooltip.tsx`.

**Origem:** o proprietário trouxe a [matos-ui](https://matos-ui.com/charts) (registry público MIT, estilo shadcn, componentes de gráfico em recharts) e pediu para melhorar o desenho das dashboards e propor gráficos novos.

**Não revisa nenhuma ADR.** Consome os tokens que a ADR-032/033 estabeleceu.

---

## 1. Objetivo

Aumentar o que as telas do M-Finance **respondem**, não o quanto elas brilham. Cada gráfico deste desenho existe porque há uma pergunta financeira concreta que hoje só se responde somando números na cabeça.

---

## 2. A decisão que manda em todas as outras: a matos-ui entra como catálogo de formas, não como camada visual

O encaixe técnico da matos-ui é bom. Ela é MIT, os componentes são recharts — a mesma biblioteca que o M-Finance já usa — e o modelo é copy-paste, então não há acoplamento a runtime de terceiro.

O encaixe **visual** não existe, e não por acidente:

- o `animated-area-chart` da matos-ui usa gradiente linear de três stops, pattern diagonal em SVG e reveal animado via `framer-motion`;
- `styles/globals.css` registra, por escrito, que gradiente em superfície e glow **não existem** neste design system, e que a estética vem de proporção, tipografia e ritmo;
- `docs/Design.md` §22 pede "linhas finas, cores discretas, fundo transparente, nada de visual chamativo sem função", e §38.6 proíbe "animações sem função";
- a lista de anti-referências do §1 inclui, literalmente, "template financeiro pronto" e "dashboard genérico de SaaS".

Há ainda um conflito de paleta que é mais fundo que estilo. Os componentes da matos-ui pintam séries com `--chart-1..5` — matizes distintas. O `lib/ui/colors.ts` documenta a escolha oposta e explica por quê: a série categórica é **rampa de luminância**, porque o sistema tem um acento só, e porque separar por clareza sobrevive ao daltonismo enquanto uma sequência de matizes não sobrevive.

E há o atrito de instalação: o M-Finance **não é shadcn**. Não existe `components.json`, e os componentes da matos-ui referenciam `--foreground`, `--muted-foreground`, `--border` e `--chart-N`, nenhuma definida em `packages/design-system`. `npx shadcn add` não roda aqui sem antes montar a ponte inteira.

**Decisão do proprietário: portar as formas, manter o M/OS.** Toma-se da matos-ui a pergunta "que tipo de gráfico responde bem a esta pergunta", e reimplementa-se em recharts com os tokens que já existem. Nenhuma dependência nova, nenhuma variável CSS nova.

---

## 3. Contrato comum a todo gráfico deste desenho

Vale para os dois consertos e para os seis componentes novos:

1. **recharts**, já instalado. `framer-motion` e `tailwind-variants` ficam de fora.
2. **Cor só de `lib/ui/colors.ts`.** É a única exceção autorizada a hex literal em componente, e ela continua sendo a única. Nenhuma matiz nova entra no arquivo.
3. **Sem gradiente de área, sem glow, sem reveal animado.** Transição de hover é permitida; entrada animada não.
4. **Largura por `useChartWidth`**, nunca `ResponsiveContainer` — o comentário do hook registra que o `ResponsiveContainer` mede `-1` no primeiro paint deste app.
5. **Vazio por `InlineEmpty`**, com frase que diz o que fazer, não só que está vazio.
6. **Valor sempre em centavos** no dado e `formatCurrency` na apresentação.
7. **Tooltip por `CurrencyTooltip`**, que já existe e já usa o `tri-mark` da marca.
8. **Transformação de dado é função pura**, fora do componente, com teste.

Uma adição à infraestrutura: `lib/formatters/currency.ts` ganha `formatCurrencyCompact(cents)` — `R$ 1,2 mil`, `R$ 3,4 mi`. Ela existe porque hoje o `YAxis` do `history-trend-chart` está `hide`, e está escondido porque `R$ 12.345,67` não cabe num eixo. Sem o formatador compacto, o eixo continua escondido e o gráfico continua sem escala.

---

## 4. Escopo

**Dentro:**

- conserto de `history-trend-chart.tsx` e `category-breakdown-chart.tsx` (§5);
- `formatCurrencyCompact` em `lib/formatters/currency.ts`;
- `lib/calculations/charts/` — os transformadores puros, com vitest;
- `MonthWaterfallChart`, `MetricSparkline` e `DueDateHeatmap` em `/dashboard` (§6);
- `BudgetThresholdBand` em `/budgets`, `GoalPriorityMatrix` em `/goals`, `SimulationProjectionChart` em `/simulator` (§7);
- correção do bug de agregação em `lib/budgets.ts` (§8), com teste.

**Fora, e cada um por um motivo:**

- **gauge de risco** — `components/simulator/risk-badge.tsx` já diz `safe/controlled/tight/critical` em texto. Um ponteiro que repete o badge é decoração;
- **treemap de categorias** — a barra ordenada do §5 responde a mesma pergunta, e treemap sem matiz é difícil de ler. Trazê-lo obrigaria a quebrar a regra da rampa;
- **candlestick, bubble, waveform, signal flow, radar** — não existe pergunta do M-Finance que elas respondam. Entrariam como as "análises complexas" que o §22 do `Design.md` manda evitar;
- **heatmap em `/calendar`** — `components/calendar/financial-calendar.tsx` já é a grade do mês. Dois calendários na mesma tela é redundância;
- **substituir os quatro metric cards pelo waterfall** — o waterfall mostra proporção, o card mostra o valor exato. São perguntas diferentes e ambas são feitas;
- **`components.json` e a ponte shadcn** — só se justificaria se a matos-ui entrasse como camada visual, e o §2 decidiu que não entra.

---

## 5. Os dois consertos

### 5.1 `history-trend-chart` — três séries competindo em igualdade

Hoje as três linhas (`receita`, `comprometido`, `sobra`) têm `strokeWidth={2}` e cores de peso parecido, o `YAxis` está `hide`, e a `Legend` mostra só o nome. O resultado é um gráfico que não diz qual número importa.

A pergunta da tela `/history` é **"a sobra está melhorando?"**. Então:

- `sobra` vira a protagonista: sódio (`COLORS.accent`), `strokeWidth={2}`;
- `receita` e `comprometido` recuam para `strokeWidth={1}` na escala neutra — contexto, não assunto;
- `YAxis` volta, com `tickFormatter={formatCurrencyCompact}` e `width` fixo para as linhas não dançarem entre meses;
- a `Legend` passa a mostrar o valor do último mês ao lado do nome, para que a legenda vire leitura e não só decodificação de cor.

### 5.2 `category-breakdown-chart` — barra sem número e sem teto

Hoje a barra não mostra valor nem proporção, e a altura é `sorted.length * 44` — vinte categorias viram 880px de gráfico.

- `LabelList` no fim da barra com `formatCurrency(value)` e o % do total;
- corte em **8 categorias**, com a nona em diante somada como `Outras`. O corte vira parâmetro da função pura, não número mágico no componente.

---

## 6. Os novos em `/dashboard`

### 6.1 `MonthWaterfallChart` — o cálculo central do app, como narrativa

`getDashboardSummary` calcula `estimatedRemainingCents = totalIncome − totalBills − totalInvoices`. Hoje esse encadeamento aparece como quatro cards soltos, e a relação entre eles fica por conta de quem lê.

Waterfall: `Receita → −Contas → −Faturas → Sobra`. Recharts não tem waterfall nativo e não precisa ter — a técnica é uma série de offset com `fillOpacity={0}` empilhada sob a série visível. O transformador puro (`toWaterfallSteps`) devolve `{ label, offset, delta, kind }` e é onde o teste mora.

Cor por papel, dentro da rampa: entrada em `COLORS.positive`, saídas na escala neutra, o total final em sódio. Quando a sobra é negativa o total vai para `COLORS.negative` — dinheiro entrando e saindo continua colorido de propósito, como `globals.css` registra.

### 6.2 `MetricSparkline` — tendência dentro do metric card

Os quatro cards de `monthMetrics` dizem o valor do mês e nada sobre a direção. `monthlySnapshots` já guarda a série histórica de todos os quatro campos.

Sparkline de ~40px de altura no rodapé do card, linha de 1px, sem eixo, sem tooltip, sem ponto. Vazio quando há menos de dois snapshots, e vazio aqui é **não renderizar nada** — um sparkline de um ponto é ruído.

Isto exige `/dashboard` passar a chamar `getMonthlySnapshots`, que hoje só a `/history` chama. É uma query nova por render da dashboard.

### 6.3 `DueDateHeatmap` — onde o mês aperta

Faixa com uma célula por dia do mês, intensidade proporcional ao valor que vence naquele dia, somando `bills.dueDate` e `invoices.dueDate`. A intensidade percorre a rampa de opacidade do sódio, não uma escala de matizes.

Responde "em que semana eu preciso ter dinheiro", que hoje só se responde abrindo `/calendar` e contando. Fica **só** no `/dashboard`, pelo motivo do §4.

---

## 7. Os novos nas outras páginas

### 7.1 `/budgets` — `BudgetThresholdBand`

`components/budgets/budget-card.tsx` já tem barra de progresso, que responde "quanto do teto eu já usei". O que ela não responde é **"eu estourei porque gastei demais ou porque o mês ainda não acabou?"**.

Linha de gasto acumulado dia a dia contra a linha horizontal do limite, com a faixa de alerta (80%) marcada. Onde a linha cruza o limite é a resposta.

**Depende do §8.** Sem a correção, a linha desenha um número errado.

### 7.2 `/goals` — `GoalPriorityMatrix`

Scatter das metas ativas: `x` = dias até o prazo, `y` = % que ainda falta, área do ponto = valor faltante em reais. O quadrante superior-esquerdo — falta muito, sobra pouco tempo — é a meta em risco, e o gráfico existe para que ela salte sem que ninguém precise comparar dez cards.

Metas sem `deadline` não entram no scatter; elas ficam listadas abaixo como "sem prazo", porque inventar um `x` para elas seria mentir sobre o dado.

### 7.3 `/simulator` — `SimulationProjectionChart`

`SimulationResult.months` já entrega, por mês, `baselineRemainingCents` e `remainingWithCents`. Duas linhas mais a linha do zero em destaque: onde a linha "com a compra" cruza o zero é o mês em que a compra quebra o orçamento.

É a informação que o `recommendation` em texto afirma, mostrada em vez de afirmada.

---

## 8. O bug que apareceu na leitura, e por que ele entra neste desenho

`lib/budgets.ts:57-105` — `getSpentForBudget` **nunca agrega**. Nos três ramos (`total`, `category`, `card`) ele faz

```ts
const [row] = await db.select({ total: bills.amountCents }).from(bills).where(...)
return row?.total ?? 0;
```

Isso devolve o valor da **primeira linha**, não a soma. Não há `sum()` nem `groupBy` no arquivo — nenhum import de `sum` ou `sql` existe.

O erro contamina `spentCents` e tudo que desce dele: `percentage`, `remainingCents`, `isOverBudget`, `isWarning`, o card "Gasto" e a contagem de "Alertas" na `/budgets`. Um orçamento com cinco contas mostra o valor de uma.

**Correção:** trocar por `sum(bills.amountCents)` com `sql` cast para número. No ramo `total`, as duas consultas (contas e faturas) somam separadamente e depois se juntam.

Ele entra neste desenho porque o §7.1 desenha em cima de `spentCents`. Um gráfico sobre um número errado o afirma com mais confiança do que o card já afirmava.

---

## 9. Testes

Vitest já está configurado (`vitest.config.ts`, `npm test`), com o padrão de teste ao lado do arquivo (`lib/payables.test.ts`).

**Com teste** — as funções puras em `lib/calculations/charts/`:

| Função | O que o teste fixa |
|---|---|
| `toWaterfallSteps` | offsets somam; sobra negativa muda `kind`; mês zerado não quebra |
| `toCategorySlices` | ordenação, corte em 8, agrupamento em `Outras`, % soma 100 |
| `toDueDateBuckets` | dia sem vencimento é zero; meses de 28/30/31 dias; intensidade normalizada |
| `toBudgetBurndown` | acumulado é monotônico; cruzamento do limite no dia certo |
| `toGoalMatrix` | meta sem prazo é excluída; prazo vencido é `x` negativo |
| `toSimulationSeries` | cruzamento do zero; projeção de um mês só |
| `formatCurrencyCompact` | milhar, milhão, negativo, zero |

Mais o teste de regressão de `getSpentForBudget` do §8, com múltiplas linhas — o teste que teria pego o bug.

**Sem teste unitário** — os componentes recharts. Eles se conferem renderizando, e é assim que a conferência acontece: subir o `/dashboard` e olhar, antes de dizer que está pronto.

---

## 10. Arquivos

**Novos:**

```
lib/calculations/charts/waterfall.ts          + .test.ts
lib/calculations/charts/categories.ts         + .test.ts
lib/calculations/charts/due-dates.ts          + .test.ts
lib/calculations/charts/budget-burndown.ts    + .test.ts
lib/calculations/charts/goal-matrix.ts        + .test.ts
lib/calculations/charts/simulation-series.ts  + .test.ts
components/charts/month-waterfall-chart.tsx
components/charts/metric-sparkline.tsx
components/charts/due-date-heatmap.tsx
components/charts/budget-threshold-band.tsx
components/charts/goal-priority-matrix.tsx
components/charts/simulation-projection-chart.tsx
lib/budgets.test.ts
```

**Alterados:**

```
components/charts/history-trend-chart.tsx      §5.1
components/charts/category-breakdown-chart.tsx §5.2
lib/formatters/currency.ts                     formatCurrencyCompact
lib/budgets.ts                                 §8
app/(app)/app/dashboard/page.tsx               §6
app/(app)/app/budgets/page.tsx                 §7.1
app/(app)/app/goals/page.tsx                   §7.2
components/simulator/simulation-list.tsx       §7.3
```

---

## 11. Ordem sugerida

O §8 vem antes do §7.1 — é a única dependência dura. Fora isso, cada gráfico é independente dos outros, e o par "função pura + teste" sempre precede o componente que a consome.

Sugestão de fases, para que haja algo de pé cedo:

1. `formatCurrencyCompact` + os dois consertos do §5;
2. o bug do §8 com seu teste de regressão;
3. `/dashboard` — waterfall, sparkline, heatmap;
4. as outras três páginas.

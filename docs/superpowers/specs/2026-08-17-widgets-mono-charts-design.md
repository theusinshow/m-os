# Widgets da Home — geometria macia, moldura e formas novas — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-17

**Baseline:** M/OS `v0.2.11` + trilha UI/UX vNext até o Lote 5 (motion e consistência transversal). Família de widgets em `packages/design-system/widgets.css`, primitivo de anel em `apps/desktop/src/Ring.tsx`, widgets da Home em `App.tsx`, `Widgets.tsx` e `TimeWidgets.tsx`.

**Origem:** referência trazida pelo proprietário — `https://amicro.vercel.app/mono-charts`, uma coleção de 30 visualizadores monocromáticos construídos sobre geometria de cantos e pontas arredondadas. Da referência foram adotados três recortes e recusado um, conforme §3 e §8.

**Revisa:** ADR-034 em três pontos. A ADR-040 nasce junto deste trabalho e é pré-requisito da implementação, não consequência dela.

## 1. Objetivo

Ampliar o vocabulário visual dos widgets da Home em três frentes decididas pelo proprietário: formas de gráfico que o M/OS ainda não tem, o acabamento de card da referência aplicado a todos os widgets, e a geometria arredondada como direção.

O que **não** muda: nenhuma regra de negócio, API, banco, schema, navegação ou contrato de domínio. Nenhum widget novo, nenhum dado novo, nenhuma medida que já não esteja na tela hoje.

## 2. Escopo

**Dentro:**
- moldura de card em todos os 15 widgets da Home, com superfície aninhada para a forma e rodapé de metas;
- quatro primitivos de forma novos (`Bars`, `Stack`, `Bullet`, `Spark`) em `apps/desktop/src/Plot.tsx`;
- adoção de geometria arredondada, com compensação aritmética onde a ponta arredondada distorce o valor;
- dois raios concêntricos no escopo de widget, com o charter que os limita;
- ADR-040 registrando as três revisões da ADR-034 e a recusa da paleta monocromática.

**Fora:**
- as telas do Tempo, que têm o próprio `Card` e o próprio lote pendente na trilha;
- Calendar, Hermes e qualquer superfície fora da Home;
- o `Panel`, que continua sem moldura e é usado em Settings, no Inspector de Workspaces e no Tempo;
- widgets novos, incluindo os sete que a ADR-034 deixou de fora por falta de dado;
- a paleta monocromática da referência (recusada, ver §8);
- qualquer alteração no `--radius: 3px` global.

## 3. O que foi lido da referência

A referência é uma galeria de 30 cards, cada um com: eyebrow em caixa alta com badge em pílula, número grande com unidade pequena ao lado, o gráfico dentro de uma superfície aninhada mais escura, e um rodapé com duas metas em fonte mono. Escala de cinza pura, dark-only, e "rounded corner geometry" como tese declarada — metade dos cards se chama `Rounded Caps`, `Rounded Arc Dial`, `Soft Arc Caps`.

Das ~30 formas, a maioria não tem domínio no M/OS: candlestick, Sankey, pirâmide de hierarquia, scatter, donut de quatro fatias. Quatro têm dado real hoje e são as adotadas em §5.3.

**Decidido pelo proprietário:** adotar formas novas, o acabamento de card **em todos** os widgets, e a geometria arredondada. Recusada a paleta monocromática.

## 4. A moldura e as superfícies

### 4.1 Onde nasce

Uma regra escopada a `.home-grid .widget` em `App.css`. Fora da Home nada muda — a regra não alcança Settings, Workspaces nem Tempo. Nenhum JSX de widget é alterado para a moldura existir: ela chega aos quinze de uma vez, e os slots de conteúdo entram depois, widget a widget (§6).

### 4.2 As duas superfícies, nos dois modos

O card usa `--surface-raised`; a forma dentro dele volta para `--surface`.

| Modo | Card | Superfície aninhada | Diferença |
|---|---|---|---|
| Dark | `#171B1F` | `#101316` | um degrau para baixo, legível |
| Light | `#FFFFFF` | `#FAFBFC` | ~2%, invisível na prática |

Daí a regra: **a superfície aninhada se declara por preenchimento no escuro e por borda no claro.** Os dois modos escrevem as duas propriedades; muda qual faz o trabalho. No claro, `1px solid var(--border)` (`#D6DBDE`) desenha o retângulo que o preenchimento não consegue. O card externo leva borda nos dois modos.

A referência é dark-only e não tem esse problema. O gate de QA da trilha exige Light onde superfície ou borda mudem materialmente — que aqui é em todo lugar, por isso §9 pede Light nas quatro larguras e não só numa.

Em `forced-colors`, a superfície aninhada perde o preenchimento e sobra a borda. É o mesmo mecanismo do modo claro, então o fallback não é caso especial.

### 4.3 Raio

`--radius: 3px` é o padrão do sistema e o comentário do token reserva `--radius-lg: 8px` a "somente app icon e overlay grande". A referência é macia em tudo.

Decisão: **dois raios concêntricos no escopo de widget.**

- `--radius-widget: 12px` — moldura externa (token novo em `tokens.css`);
- `--radius-lg: 8px` — superfície aninhada (token existente, charter ampliado pela ADR-040);
- `--space-3` (12px) de respiro entre as duas bordas;
- `--radius: 3px` **inalterado** para botão, campo, linha e todo o resto do sistema.

Subir o raio global foi considerado e recusado: vazaria a maciez para o sistema inteiro sem que ninguém tenha pedido isso.

### 4.4 Rodapé

Uma linha com duas metas em `--text-meta` (12px, `--font-system`, tracking `0.05em`), esquerda e direita, separada da forma por `1px solid var(--border)`.

## 5. Os primitivos de forma

### 5.1 A regra que organiza tudo

Canto arredondado e ponta arredondada são coisas diferentes, e só uma delas mente:

- **`rect` com `rx`** arredonda para dentro da geometria. A barra mantém a altura exata do valor. **Não precisa de compensação.**
- **`stroke-linecap: round`** arredonda para fora: acrescenta meia espessura de traço em cada ponta. **Precisa.**

A maciez entra de graça em tudo que é retângulo — barras, empilhada, bullet, células de densidade. Só as formas de traço passam pela compensação.

### 5.2 A compensação

Desenha-se `L' = max(ε, L − espessura)` para `L > 0`, e `L' = 0` para `L = 0`, onde `L = valor × circunferência`. O pintado volta a ser `L`.

O piso `ε` é um traço quase-zero (`0.01`), e não zero puro, por uma razão de renderização: um dash de comprimento zero com cap redondo fica a critério do renderizador, enquanto um dash de 0,01 pinta o disco de forma determinística. É ele que garante o comportamento descrito abaixo.

Sem ela, o erro é a espessura sobre a circunferência, nos tamanhos que a família já usa:

| Tamanho | Raio | Espessura | Circunferência | Erro sem compensação |
|---|---|---|---|---|
| 88 | 41 | 6 | 257,6 | 2,3 pp |
| 44 | 19 | 4 | 119,4 | 3,4 pp |
| 14 | 5,75 | 2,5 | 36,1 | 6,9 pp |

O último é o que justifica a proibição original ter sido escrita.

**O limite, declarado:** quando o valor é menor que uma espessura de traço, `L'` chega a zero e o cap arredondado pinta um ponto. Ali o anel **para de medir e passa a afirmar presença** — "existe algo, menor que o menor traço que este anel sabe desenhar". Zero continua não desenhando nada, pela guarda `drawn <= 0` que já existe em `Ring.tsx`.

### 5.3 Os quatro primitivos

Todos sobre dado que já está na tela hoje. Nenhum lê fonte nova.

| Primitivo | Construção | Onde entra | Dado que já existe | Por quê |
|---|---|---|---|---|
| `Bars` | `rect rx` | **TASKS NA SEMANA**, no lugar dos 7 anéis de 44px | `days[].done` em `WeekRings` | comparar sete alturas é mais rápido que comparar sete ângulos, e 44px é onde o cap mais distorceria |
| `Stack` | `rect rx` empilhado | **HORAS POR PROJECT**, no lugar dos 4 anéis | `ranked` em `WeekByProject` | a pergunta do widget é "onde foi parar a semana?", que é composição; quatro anéis pedem comparação par a par |
| `Bullet` | `rect rx` + marca de meta | **META**, no lugar do anel de 88px | `target` em `BudgetRing` | resolve uma limitação escrita no próprio código: "passa de 100% e o anel PARA em cheio... o estouro é dito no texto". O bullet desenha o estouro |
| `Spark` | traço com cap compensado | **HORAS HOJE**, somando-se ao arco de 270° | `week` de `dailySeconds(entries, 7)` em `TodayHours` | o widget já calcula sete dias e usa só o de hoje e o pico; a linha mostra o que já foi computado |

**Continuam anel**, ganhando o cap arredondado compensado: CONCLUÍDO, INBOX e o arco de 270° de HORAS HOJE — são proporção de uma coisa só, que é exatamente a tese da família do anel. **MÊS** continua densidade, com `rx` nas células.

## 6. A anatomia aplicada aos quinze

Duas regras decidem quem ganha o quê, e as duas existem para não inventar dado.

**Regra 1 — o número aparece uma vez só.** Quando a forma já o carrega no centro (anel com `RingLabel`), o slot de manchete fica vazio. Quando não carrega, a manchete existe.

**Regra 2 — o rodapé diz escala e extremo.** Esquerda: contra o que se mede. Direita: o extremo (pico, alvo, total). Lista não tem escala, então lista não tem rodapé.

| Widget | Manchete | Forma | Rodapé |
|---|---|---|---|
| EM ANDAMENTO | `3` · em andamento | lista | — |
| CRONÔMETRO | — (o cronômetro é o número) | — | — |
| HORAS HOJE | — (no centro do arco) | arco 270° + `Spark` | `7 DIAS · CONTRA O PICO` / `PICO 3,2 H` |
| INBOX | — (no centro do anel) | anel | `ENVELHECENDO` / `2 DE 9` |
| RECENTES | `12` · capturas | lista | — |
| PROJECTS | `5` · ativos | lista | — |
| MÊS | `41` · registros | densidade `rx` | `30 DIAS · 4 DEGRAUS` / `PICO 7` |
| TASKS NA SEMANA | `12` · concluídas | `Bars` | `SEG–DOM · CONTRA O PICO` / `PICO 5` |
| HORAS POR PROJECT | `18,4 h` · na semana | `Stack` | `4 PROJECTS · 7 DIAS` / `MAIOR: <nome>` |
| CONCLUÍDO | — (no centro do anel) | anel | — (o `RingLabel` já diz `12 DE 30`) |
| META | `84%` · da meta | `Bullet` | `META 40 H` / `FALTAM 6,4 H` |
| RECURSOS | `5` · recursos | lista | — |
| APPS | `4` · apps | grade de ícones | — |
| AÇÕES | — | botões | — |
| SISTEMA | — | estado | — |

Onde a manchete é contagem, o valor é o `count` que o `Panel` **já recebe hoje** — a mesma medida que está na tela, promovida de 12px a número grande. Não é métrica nova.

Seis widgets ficam sem manchete (CRONÔMETRO, HORAS HOJE, INBOX, CONCLUÍDO, AÇÕES, SISTEMA) e três sem forma (CRONÔMETRO, AÇÕES, SISTEMA). Todos levam a moldura, que é o que a decisão de §3 determina para os quinze.

### 6.1 Onde os slots moram

O componente `<Widget>` em `App.tsx:113` já embrulha os quinze. Ele ganha `value`, `unit`, `footLeft` e `footRight`, todos opcionais. O `Panel` **não é tocado** — é o que impede a moldura de vazar para Settings (3 usos), Inspector de Workspaces (5 usos) e Tempo (2 usos).

### 6.2 Arquivos

| Arquivo | Mudança |
|---|---|
| `apps/desktop/src/Plot.tsx` | **novo** — `Bars`, `Stack`, `Bullet`, `Spark` |
| `apps/desktop/src/Ring.tsx` | cap arredondado com compensação; comentário de regra atualizado |
| `packages/design-system/widgets.css` | família de plot; `stroke-linecap: round` no `.mos-ring-value`; `rx` nas células de densidade |
| `packages/design-system/tokens.css` | `--radius-widget: 12px`; charter de `--radius-lg` ampliado |
| `apps/desktop/src/App.css` | moldura, superfície aninhada e rodapé, escopados a `.home-grid .widget` |
| `apps/desktop/src/App.tsx` | slots no `<Widget>`; troca de forma em três widgets |
| `apps/desktop/src/Widgets.tsx` | `WeekRings` passa a usar `Bars` |
| `apps/desktop/src/TimeWidgets.tsx` | `WeekByProject` → `Stack`; `BudgetRing` → `Bullet`; `TodayHours` ganha `Spark` |
| `docs/DECISIONS.md` | ADR-040 |
| `docs/UI-UX-REFINEMENT.md` | seção de estado de execução |

`Plot.tsx` é arquivo próprio porque `App.tsx` já tem 2.651 linhas e não deve receber SVG.

## 7. Movimento e acessibilidade

O orçamento da ADR-034 continua valendo e as formas novas obedecem:

- barras e empilhada crescem da linha de base; o `Spark` desenha por `stroke-dashoffset`, como o anel já faz; o bullet cresce da esquerda — **movimento que carrega dado**, sem shimmer, float ou parallax;
- cascata de 40ms com teto de 8; as sete barras da semana cabem sob o teto;
- **nenhum loop novo**: o único da Home é o cronômetro correndo, e continua sendo o único;
- `reduced-motion` não precisa de regra nova — a fonte única em `tokens.css` zera os tokens e as formas nascem no valor final;
- SVG segue `aria-hidden`, com o número em texto ao lado, que é o padrão da família.

## 8. O que foi recusado

**A paleta monocromática.** O sódio continua reservado para carga, e agora/hoje continuam traço branco de 2px. Metade do charme da referência vem do cinza puro, e por isso a recusa precisa estar escrita: quem reabrir o assunto encontra a decisão em vez de achar que foi esquecimento.

**Subir o raio global.** Ver §4.3.

**As formas sem domínio** — candlestick, Sankey, pirâmide, scatter, donut de quatro fatias. A ADR-034 já fixou a razão: "um anel bonito preenchido com número inventado é pior que a ausência".

**Aplicar a moldura via `Panel`.** Levaria card para dentro de Settings e do Inspector de Workspaces.

## 9. Gates de QA

Os mesmos da trilha UI/UX vNext:

- `npm run build`, `npm test -- --run`, `npx impeccable detect apps/desktop/src`, `git diff --check`;
- Dark em 840×600, 1280×800, 1440×900 e 1920×1080;
- **Light nas quatro larguras** — superfície e borda mudam materialmente em todas (§4.2);
- árvore de acessibilidade, foco e teclado sem regressão;
- `reduced-motion` confirmando tokens em `0ms`;
- `forced-colors` confirmando o fallback por borda;
- zero overflow de página, botão sem nome, campo sem label ou ID duplicado.

A verificação em tela é do proprietário: a janela do Tauri não é legível pelo agente.

## 10. ADR-040 — o que ela decide

Três revisões da ADR-034, numa ADR só porque são uma direção só:

1. **Ponta reta → ponta arredondada com compensação.** A regra antiga estava certa sobre o problema e o resolvia proibindo; a nova o resolve compensando, e registra a aritmética (§5.2) e o limite de uma espessura.
2. **A moldura entra na Home.** Reverte a posição anti-cardização **apenas nesse escopo**. O `Panel` sem moldura continua sendo a resposta fora da Home, e a nota do `Surface.tsx` segue valendo para o resto do sistema.
3. **O raio ganha charter novo.** `--radius-widget: 12px` e `--radius-lg` liberado para superfície aninhada de widget; `--radius: 3px` inalterado.

Mais o registro da recusa de §8.

A ADR-040 é **pré-requisito** da implementação: sem ela, o código passaria a contradizer uma ADR aceita, que é exatamente o que `UI-UX-REFINEMENT.md` §15 proíbe como "mudança silenciosa de IA".

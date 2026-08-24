# A Home elege um destaque, e o vazio não paga aluguel de cartão — Design

**Status:** proposta, aguardando aprovação

**Data:** 2026-08-24

**Baseline:** M/OS `v0.3.0` no commit `023e3df`, que entregou os charts do M-Finance.

**Origem:** o retrabalho da Home, pendente desde 2026-08-18. A Fase 1 do arranjo
foi entregue e o dono do produto disse que não convenceu. O `UI-UX-REFINEMENT.md`
diagnosticou a mesma coisa por outro caminho: *"a Home mostra quase tudo com peso
semelhante, em vez de orientar o momento atual."*

---

## 1. O que a foto mostrou

A Home foi fotografada inteira, com o app real rodando, antes de qualquer
desenho. A foto muda o diagnóstico — para melhor, porque o torna concreto.

O problema não é só "tudo com o mesmo peso". É que **a Home dá peso máximo ao
vazio**:

| Widget | Altura gasta | O que diz |
| --- | --- | --- |
| `EM ANDAMENTO` | ~230px | "Nada em andamento" |
| `PARADAS` | ~240px | "Nada parado." |
| `FACULDADE` | linha inteira | "Nenhum semestre." |
| `INBOX` | ~240px | anel vazio, `0` |
| faixa `Visão` inteira | ~450px | `0`, `0%`, `0,0 H`, 12 registros |

Mais de metade da altura da Home é ausência desenhada com a mesma moldura, o
mesmo padding e a mesma presença de um cartão cheio. E a única coisa acionável da
tela — o botão `Iniciar meu dia` — é o menor elemento amarelo da página, dentro
de um cartão que não se distingue dos outros dezessete.

**A causa é uma só, e vale nos dois estados do banco:** o peso é fixo e o
conteúdo é variável. Com o banco vazio isso vira um mural de zeros; com o banco
cheio vira dezoito cartões gritando junto. A ADR-034 pede leitura em meio
segundo, e nenhum dos dois entrega.

**Ressalva honesta sobre a base fotografada:** o banco de dogfood está quase
vazio — 1 Project, 5 capturas de 11 dias atrás, sem semestre. A foto exagera o
lado "vazio" do problema. Não o inventa: o lado "cheio" tem a mesma raiz.

### 1.1 A hierarquia existe no modelo e não existe na tela

`HomeWidgetRole` já tem cinco valores — `focus`, `attention`, `overview`,
`collection`, `utility` — e o `App.css:1060` declara a intenção por escrito:

> `data-role` descreve por que o widget está na tela. Isso evita confundir
> tamanho com importância.

Mas o papel governa **uma única regra de CSS** em todo o arquivo:

```css
.widget[data-role="focus"] .panel { min-height: calc(var(--height-row) * 3); }
```

Ou seja: o vocabulário de hierarquia foi criado, documentado, e nunca ganhou
expressão visual. Todo widget renderiza como o mesmo `SpotlightCard` com o mesmo
`Panel`. O único diferenciador real é a largura — e a largura foi declarada, no
mesmo comentário, como não sendo importância.

Este desenho não inventa uma hierarquia nova. Ele dá corpo à que já foi decidida.

---

## 2. As quatro decisões

Tomadas com o dono do produto, nesta ordem:

| Pergunta | Decisão |
| --- | --- |
| O que incomoda | **a leitura**, não a edição. O modo de arrumar fica como está. |
| Quem decide o destaque | **o sistema**, pelo estado atual. Sem arrumação manual. |
| O que acontece com o vazio | **colapsa em uma linha**, e volta a cartão quando tiver conteúdo. |
| Onde o eleito aparece | **num lugar de honra acima das faixas**, fora da grade. |

---

## 3. Três registros

A Home passa a ter três pesos, e só três:

```
—— lugar de honra (fora da grade) ——
┌─────────────────────────────────────────┐
│ HOJE                                    │
│ Bom dia. 1 task aberta · 1 project ativo│
│ [ Iniciar meu dia ]                     │
└─────────────────────────────────────────┘

Agora                    ← HOJE saiu daqui
┌───────────────┐ ┌───────────────┐
│ CRONÔMETRO    │ │ HORAS HOJE    │        cartões
└───────────────┘ └───────────────┘
· Nada em andamento   · Sem semestre       linhas

Retomar
┌───────────────┐
│ RECENTES    5 │
└───────────────┘
· Inbox vazia  · Nada parado  · Sem projects
```

1. **o eleito** — um só, no lugar de honra, acima de todas as faixas;
2. **o cartão** — o que existe hoje, para widget com conteúdo;
3. **a linha** — widget sem nada a dizer, colapsado.

As faixas, a grade de doze colunas e os três tamanhos **não mudam**. O que muda é
o peso, não a estrutura.

---

## 4. A eleição

### 4.1 A formulação

**O lugar de honra é do `daily_session` por padrão, e a eleição é a regra de quem
o destrona.** Formular assim — e não como "escolher um entre dezoito" — é o que
mantém a regra pequena, legível e testável.

### 4.2 Os sinais que existem

Todos já carregados pela Home. Nenhuma chamada nova ao backend:

| Sinal | Origem | A eleição usa? |
| --- | --- | --- |
| `academic.upcoming[].horizonte` | `academic.ts:44`, `types.ts:1271` | **sim** |
| `estadoDoDia` | `daily.ts:54` | não — ver §4.3 |
| `semanaPendente: Week \| null` | `App.tsx:80` | não |
| `stale.paradas` | `stale.ts:37` | não |
| `attentionCount` | `App.tsx:3174` | não |

Os quatro últimos ficam registrados porque são os candidatos naturais a virar
regra quando houver âncora de tempo — e porque dizer que existem e não são usados
é mais honesto que omiti-los.

### 4.3 A regra

Primeira que casar vence:

1. **existe compromisso com horizonte `overdue`, `today` ou `tomorrow`** →
   `academic`;
2. **qualquer outro caso** → `daily_session`.

O horizonte vem do backend já classificado (`types.ts:1271`), e a regra usa esse
vocabulário em vez de recalcular uma janela de horas no front. Uma segunda
definição de "urgente" seria uma segunda fonte de verdade sobre a mesma pergunta.

**O `estadoDoDia` não entra na eleição, e isso é deliberado.** Ele decide o que o
widget do dia *diz* — "ontem ficou aberto", "iniciar meu dia", "3 de 5 feitos" —
e essa é a responsabilidade do `DailyFocusWidget`, que já a cumpre. Quem é eleito
não muda: em todos os quatro estados, o dia continua sendo o dia. Passar
`estadoDoDia` para o eleitor seria carregar um parâmetro que nenhum ramo lê.

### 4.4 Por que só duas regras

Porque só há duas honestas hoje, e o motivo está registrado:

> sem prazo em `Task` e sem `Event`, logo sem âncora de tempo futuro
> — `ATTENTION-SYSTEM.md`, P0

A faculdade é **a única coisa no M/OS que sabe o que vai acontecer amanhã**,
porque `Compromisso` tem `plannedAt`. Enquanto `Task` não tiver prazo e `Event`
não existir — Fase 4 do `ROADMAP.md` — não existe um terceiro candidato com
lastro. Uma terceira regra hoje seria inventada.

O que a eleição entrega agora: no dia da prova, a Home abre com a prova; nos
outros dias abre com o dia, que é o certo. O que ela entrega depois: o lugar onde
reunião gravando, lembrete vencido e Task com prazo entram sem redesenhar nada.

### 4.5 As guardas

- **widget oculto nunca é eleito.** A escolha da pessoa ganha da eleição do
  sistema, sempre;
- **eleito indisponível cai para o seguinte.** Se não sobrar ninguém, o lugar de
  honra não renderiza e a Home começa em `Agora`;
- **o eleito sai da faixa** enquanto está na honra, para não existir duas vezes;
- **a eleição é derivada, nunca gravada.** Recalcula a cada render. Não há
  migration, não há coluna, não há estado a sincronizar.

---

## 5. O recuo

### 5.1 O contrato

Cada widget declara, no ponto de `App.tsx` onde o dado já está, um campo novo:

```ts
quieto?: string
```

Preenchido significa: *não tenho nada a dizer, e é isto que digo em uma linha.*

### 5.2 Quem colapsa

| Widget | Colapsa quando | Linha |
| --- | --- | --- |
| `now` | nada em Doing | `Nada em andamento` |
| `stale` | zero paradas | `Nada parado` |
| `academic` | sem semestre | `Sem semestre` |
| `inbox_pulse` | inbox vazia | `Inbox vazia` |
| `recent` | sem capturas | `Sem capturas` |
| `projects` | sem projects | `Sem projects` |
| `task_progress` | sem tasks | `Sem tasks` |
| `recent_resources` | zero recursos | `Sem recursos` |
| `month_density` | zero registros no mês | `Sem registros` |
| `week_rings` | zero tasks na semana | `Sem tasks na semana` |
| `week_by_project` | zero horas na semana | `Sem horas na semana` |

### 5.3 Quem nunca colapsa

`timer`, `today_hours`, `quick_actions`, `apps` e `system_health`.

O cronômetro é um formulário e as ações são botões: eles não têm estado vazio,
têm função. Colapsar um formulário esconderia a única maneira de usá-lo.

### 5.4 Onde as linhas moram

As linhas quietas saem da grade e se juntam **numa única fileira no pé da faixa**
— cada uma clicável, levando ao mesmo lugar que o cartão levava.

A faixa cujos widgets colapsaram **todos** vira só o título mais a fileira. A
faixa que fica sem nenhum widget continua sumindo, como já faz hoje.

### 5.5 Três regras que só aparecem construindo

- **`available: false` e `quieto` são coisas diferentes.** O `budget_ring` sem
  meta é *"não existe para você"* e some; `quieto` é *"existe e está vazio"* e
  vira linha. Continuam separados.
- **Enquanto carrega, nada colapsa.** Senão a linha quieta pisca e vira cartão
  meio segundo depois — trocar layout durante o carregamento é pior que esperar.
- **No modo "Arrumar", nada colapsa.** Mesma razão que o código já aplica ao
  widget oculto: *"o que se esconde precisa continuar alcançável de onde se
  escondeu."* Um widget colapsado que não pode ser redimensionado seria uma porta
  de mão única.

---

## 6. Onde o código mora

### 6.1 Novo: `apps/desktop/src/homeDestaque.ts`

Só a política de eleição, e nada de React:

```ts
/** A ordem é a prioridade: primeiro candidato elegível vence. */
export const CANDIDATOS = ["academic", "daily_session"] as const;

export type SinaisDaHome = {
  /** Há compromisso `overdue`, `today` ou `tomorrow`. */
  academicoUrgente: boolean;
  ocultos: Set<string>;
  disponiveis: Set<string>;
};

export function elegerDestaque(sinais: SinaisDaHome): string | null;
```

`CANDIDATOS` exportado, e não escondido dentro da função, porque é ele que
responde "quem pode ser eleito" — a pergunta que o próximo sinal vai fazer.

Módulo próprio porque **eleição é política e arranjo é forma**. Misturar as duas
no `homeLayout.ts` faria a próxima regra de eleição — reunião, lembrete, prazo —
crescer dentro do arquivo que já é a fonte de verdade de outra coisa.

### 6.2 `homeLayout.ts` ganha uma função, e nada mais

```ts
export function repartirFaixa(
  slots: ArrangedWidget[],
  quietos: Map<string, string>,
): { cartoes: ArrangedWidget[]; linhas: { id: string; label: string; texto: string }[] };
```

Ela mora aqui porque **é arranjo**, e o cabeçalho do arquivo já declara ser a
única cópia dessa regra. A duplicata anterior em `crates/mos-core/src/work.rs`
ficou para trás em silêncio, com os testes dela passando — que é o pior jeito de
ficar para trás.

`fillBand` continua depois de `repartirFaixa`: ela fecha a última linha do que
sobrou na grade, e o que colapsou não está mais lá para contar colunas.

### 6.3 `App.tsx`

`HomeBoard` recebe `eleitoId: string | null` e `quietos: Map<string, string>`.
Renderiza a honra antes de mapear as faixas, e filtra o eleito do arranjo.

### 6.4 `App.css`

Duas classes novas: `.home-honra` e `.home-quietas`. Sem token novo, sem valor
fora dos tokens.

### 6.5 Nada em Rust, nenhuma migration

Pelo mesmo argumento que o `homeLayout.ts` já faz sobre largura:

> largura de widget não é conceito de produto, e sim desenho da Home

Destaque também não é. O `CORE.md` lista o que o crate carrega — Capture, Inbox,
Project, Task, Workspace — e nenhum deles muda aqui.

---

## 7. Testes

Não há teste de DOM neste repo, por decisão registrada no `vitest.config.ts`. A
consequência prática é que **o que dá para verificar tem de ser função pura** — e
este desenho respeita isso de propósito, colocando toda a regra em duas funções
puras e deixando o `App.tsx` só com a montagem.

**`homeDestaque.test.ts`** (novo):

- compromisso `tomorrow` destrona o dia;
- compromisso `today` destrona o dia;
- compromisso `overdue` destrona o dia;
- compromisso `this_week` **não** destrona;
- compromisso `later` **não** destrona;
- sem semestre, o dia vence;
- `academic` oculto não é eleito, mesmo com prova amanhã;
- `academic` indisponível cai para o dia;
- nada disponível devolve `null`.

**`homeLayout.test.ts`** (somados):

- widget com `quieto` sai dos cartões e entra nas linhas;
- widget sem `quieto` fica nos cartões;
- faixa com todos quietos devolve zero cartões e N linhas;
- `fillBand` fecha a linha do que **sobrou**, e não do que colapsou;
- o eleito filtrado não aparece em nenhuma faixa.

---

## 8. Fora de escopo

- **o modo de arrumar** — não muda, por decisão explícita;
- **widget novo** — nenhum;
- **ordem das faixas** — continua fixa, do desenho;
- **fixar o destaque à mão** — foi considerado e não escolhido;
- **animar a troca do eleito** — o orçamento de movimento da ADR-034 é um loop
  por tela, e ele já está gasto.

---

## 9. Um defeito menor encontrado no caminho

`homeLayout.ts:80` rotula `today_hours` como `"HOJE"`, mesmo rótulo do
`daily_session`. O card renderiza `Panel label="HORAS HOJE"` e na tela não há
ambiguidade — mas o rótulo do catálogo é o que alimenta os `aria-label` da barra
de arrumar e o inspetor de Workspace, e lá existem dois "HOJE".

Correção de uma linha. O `id` não muda: ele vai para o banco, e renomear apagaria
em silêncio a escolha de quem tinha ocultado ou movido o widget.

---

## 10. ADR

Este desenho pede uma ADR nova — **ADR-059** — porque decide duas coisas que
valem além desta tela:

1. hierarquia na Home é **eleita pelo estado**, não arrumada à mão;
2. widget sem conteúdo **não paga aluguel de cartão**.

# O Settings ganha um mapa — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a página de configurações passa de cinco seções mal agrupadas numa coluna sem mapa para sete seções honestas com navegação lateral.

**Architecture:** três movimentos, nesta ordem, e cada um verificável sozinho: primeiro a extração mecânica do `App.tsx` (**zero mudança de comportamento**), depois o catálogo das seções como função pura testável, depois o reagrupamento e a navegação. Extrair primeiro não é preferência: a página é hoje uma única linha de JSX, e reagrupar dentro dela produziria um diff impossível de revisar.

**Tech Stack:** React + TypeScript, vitest para a função pura.

**Spec:** `docs/superpowers/specs/2026-08-28-sync-automatico-e-settings-design.md` §6

**Depende do plano de sync?** Só na Task 4, que cria a seção "Sincronização". As
Tasks 1–3 são independentes e podem rodar antes, depois ou em paralelo.

## Global Constraints

- **Nada muda na tela nas Tasks 1 e 2.** Elas são refactor. Qualquer diferença visual é um defeito, não uma melhoria.
- **`cargo` não entra neste plano** — é tudo TypeScript. Se precisar rodar algo em Rust, exporte `TMP`/`TEMP` para o scratchpad antes (ver o plano de sync).
- **Nunca rodar `Stop-Process` em `mos-desktop`.** Mata a sessão real do dono do produto; feche pela janela.
- **Toda cópia de interface em português**, no tom do resto do app.
- **Comentário explica POR QUÊ, não o quê.**
- Commits em português, `tipo(escopo): frase minúscula`, direto na `master`.

---

## Estrutura de arquivos

| Arquivo | Responsabilidade | Ação |
| --- | --- | --- |
| `apps/desktop/src/SettingsPage.tsx` | a página e os `*Settings` que só ela usa | Criar |
| `apps/desktop/src/settingsNav.ts` | **puro**: o catálogo das sete seções e a resolução da seção visível | Criar |
| `apps/desktop/src/settingsNav.test.ts` | os testes dele | Criar |
| `apps/desktop/src/functionLabels.ts` | os três `Record` de rótulo que a busca **e** o Settings usam | Criar |
| `apps/desktop/src/App.tsx` | perde ~600 linhas; importa a página | Modificar |
| `apps/desktop/src/App.css` | o estilo da navegação lateral | Modificar |

**Por que `functionLabels.ts` existe:** `functionRiskLabels` é usado pela busca
(`App.tsx:2573`) **e** pelo painel FUNCTIONS. Deixá-lo no `App.tsx` e importar de
lá criaria um ciclo (`App` → `SettingsPage` → `App`). Um módulo terceiro que os
dois importam é a saída, e é a mesma razão pela qual `Surface.tsx` já existe.

---

## Task 1: Extrair a página, sem mudar nada

O objetivo desta task é um diff que **não muda um pixel**. É a fundação das
outras duas: enquanto a página for uma linha só, nenhum reagrupamento é
revisável.

**Files:**
- Create: `apps/desktop/src/SettingsPage.tsx`
- Create: `apps/desktop/src/functionLabels.ts`
- Modify: `apps/desktop/src/App.tsx` (remover `:103-128` SHORTCUTS, `:130-132` os rótulos, `:2597-2660` `UnivirtusSettings`, `:2695-2710` `resumoDoSync`, `:2712-2793` `HermesSettings`, `:2794-2863` `SyncSettings`, `:2864-2929` `FinanceActionSettings`, `:2930-3033` `StartupSettings`, `:3034-3085` `DiagnosticoPanel`, `:3086-3236` `SettingsPage`)

**Interfaces:**
- Produces: `export function SettingsPage(props)` com **exatamente** a assinatura de hoje (`:3086`), e `export const functionRiskLabels` / `functionCategoryLabels` / `functionConfirmationLabels` em `functionLabels.ts`.

- [ ] **Step 1: Fotografar a página de antes**

Antes de mover uma linha. Use a skill `ver-o-app`: abra o M/OS, vá em Settings, e
tire foto da página **inteira** (ela rola). Guarde no scratchpad como
`settings-antes.png`. É a única prova possível de que a extração não mudou nada
— não há teste de DOM neste repositório.

- [ ] **Step 2: Os rótulos compartilhados**

Crie `apps/desktop/src/functionLabels.ts`:

```ts
/**
 * Como cada Function se chama na tela.
 *
 * Mora fora do `App.tsx` porque DOIS lugares precisam: a busca, que mostra o
 * risco na linha do resultado, e o painel FUNCTIONS do Settings. Com os rotulos
 * no `App.tsx`, a pagina de Settings extraida teria de importar do `App.tsx`
 * que a importa — um ciclo. Um modulo terceiro e a saida, e e a mesma razao
 * pela qual o `Surface.tsx` existe.
 */
import type { FunctionDefinition } from "./types";

export const functionCategoryLabels: Record<FunctionDefinition["category"], string> = { capture: "CAPTURE", daily: "DIA", work: "WORK", time: "TEMPO", attention: "ATENÇÃO", memory: "MEMORY", meeting: "REUNIÕES", app: "APP", data: "DATA", system: "SYSTEM" };
export const functionRiskLabels: Record<FunctionDefinition["risk"], string> = { low: "baixo", medium: "medio", high: "alto" };
export const functionConfirmationLabels: Record<FunctionDefinition["confirmation"], string> = { none: "sem confirmacao", explicit: "confirmacao explicita" };
```

Apague as três linhas `:130-132` do `App.tsx` e importe deste módulo.

- [ ] **Step 3: Mover os componentes, VERBATIM**

Crie `apps/desktop/src/SettingsPage.tsx` e mova para lá, **sem editar o corpo de
nenhum**, na ordem em que aparecem hoje:

`SHORTCUTS` (`:103`), `UnivirtusSettings` (`:2597`), `resumoDoSync` (`:2695`),
`HermesSettings` (`:2712`), `SyncSettings` (`:2794`), `FinanceActionSettings`
(`:2864`), `StartupSettings` (`:2930`), `DiagnosticoPanel` (`:3034`),
`SettingsPage` (`:3086`).

Exporte só `SettingsPage`; o resto fica privado ao módulo.

Cabeçalho do arquivo:

```tsx
/**
 * A pagina de configuracoes.
 *
 * Saiu do `App.tsx` porque ela era UMA linha de JSX de dezenas de milhares de
 * caracteres, dentro de um arquivo de 4017 linhas. Isso nao e estetica: um diff
 * que cabe numa linha nao e revisavel, e a pagina precisava ser reagrupada.
 *
 * O que mora aqui sao os paineis que SO o Settings usa. O que a busca tambem
 * usa foi para o `functionLabels.ts`; o que qualquer pagina usa continua no
 * `Surface.tsx`.
 */
```

Os imports que ela precisa — confira contra o corpo depois de mover, o
`tsc` vai apontar o que faltar:

```tsx
import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { MeetingSettings } from "./MeetingSettings";
import { PaneHeader, Panel, StateMessage } from "./Surface";
import { functionCategoryLabels, functionConfirmationLabels, functionRiskLabels } from "./functionLabels";
```

- [ ] **Step 4: Importar no `App.tsx`**

```tsx
import { SettingsPage } from "./SettingsPage";
```

O ponto de uso não muda: a `SettingsPage` já era chamada por nome.

- [ ] **Step 5: Provar que compila e que os testes seguem verdes**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx tsc --noEmit 2>&1 | head -20 && npm test 2>&1 | tail -10
```

Esperado: `tsc` sem saída, todos os testes passando. Erro de import não resolvido aqui é o `tsc` fazendo o trabalho dele — conserte e rode de novo.

- [ ] **Step 6: Provar que o `App.tsx` encolheu de verdade**

```bash
cd /c/Dev/pessoal/m-os && wc -l apps/desktop/src/App.tsx apps/desktop/src/SettingsPage.tsx
```

Esperado: `App.tsx` perto de 3400 linhas (era 4017), `SettingsPage.tsx` perto de 600.

- [ ] **Step 7: Fotografar de novo e comparar**

Abra o M/OS, vá em Settings, fotografe a página inteira como `settings-depois.png`.
**Compare com a de antes, painel por painel.** Qualquer diferença é um defeito
desta task — conserte antes de commitar. Confira também que os painéis que têm
formulário ainda salvam: mude o tema e volte.

- [ ] **Step 8: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src/SettingsPage.tsx apps/desktop/src/functionLabels.ts apps/desktop/src/App.tsx && git commit -m "refactor(settings): a pagina sai do App.tsx, e nada muda na tela

Ela era UMA linha de JSX de dezenas de milhares de caracteres dentro de um
arquivo de 4017 linhas. Nao se reagrupa o que nao se consegue ler, e nao se
revisa um diff que cabe numa linha — entao a extracao vem antes do
reagrupamento, e sozinha.

Os componentes foram movidos verbatim: nenhum corpo editado. Os tres Record de
rotulo de Function foram para um modulo proprio porque a BUSCA tambem os usa, e
importa-los do App.tsx a partir de uma pagina que o App.tsx importa seria um
ciclo.

Conferido por foto da pagina inteira, antes e depois, painel por painel.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: O catálogo das seções, como função pura

Sem teste de DOM neste repositório, o que decide vira função pura — a mesma
regra do `homeLayout.ts`, do `daily.ts` e do `syncFaixa.ts`.

**Files:**
- Create: `apps/desktop/src/settingsNav.ts`
- Create: `apps/desktop/src/settingsNav.test.ts`

**Interfaces:**
- Produces: `SETTINGS_SECTIONS: SettingsSection[]` e `secaoVisivel(posicoes, scrollTop): string`. A Task 3 monta a navegação em cima.

- [ ] **Step 1: Escrever os testes que falham**

Crie `apps/desktop/src/settingsNav.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { SETTINGS_SECTIONS, secaoVisivel } from "./settingsNav";

describe("o catálogo", () => {
  it("tem as sete seções, na ordem do desenho", () => {
    expect(SETTINGS_SECTIONS.map((s) => s.id)).toEqual([
      "sync", "conexoes", "aparencia", "inicio", "reunioes", "dados", "avancado",
    ]);
  });

  it("põe Sincronização primeiro, porque é a que se visita", () => {
    expect(SETTINGS_SECTIONS[0].id).toBe("sync");
  });

  it("todo id é único — ele vira âncora de URL e alvo de scroll", () => {
    const ids = SETTINGS_SECTIONS.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("todo título é frase, e não GRITO: o micro-label é o do painel, não o da seção", () => {
    for (const secao of SETTINGS_SECTIONS) {
      expect(secao.title).not.toBe(secao.title.toUpperCase());
    }
  });
});

describe("qual seção está visível", () => {
  const posicoes = [
    { id: "sync", top: 0 },
    { id: "conexoes", top: 400 },
    { id: "aparencia", top: 900 },
  ];

  it("é a primeira quando ainda não rolou", () => {
    expect(secaoVisivel(posicoes, 0)).toBe("sync");
  });

  it("troca ao passar do topo da próxima", () => {
    expect(secaoVisivel(posicoes, 420)).toBe("conexoes");
  });

  it("um pouco ANTES do topo já conta, senão o título fica colado sem marcar", () => {
    expect(secaoVisivel(posicoes, 390)).toBe("conexoes");
  });

  it("no fim da página é a última, mesmo que ela seja curta demais para encher a tela", () => {
    expect(secaoVisivel(posicoes, 5000)).toBe("aparencia");
  });

  it("sem seções não quebra", () => {
    expect(secaoVisivel([], 100)).toBe("");
  });
});
```

- [ ] **Step 2: Rodar e ver falhar**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx vitest run src/settingsNav.test.ts 2>&1 | tail -15
```

Esperado: FAIL — `Failed to resolve import "./settingsNav"`.

- [ ] **Step 3: Escrever o catálogo**

Crie `apps/desktop/src/settingsNav.ts`:

```ts
/**
 * As secoes do Settings: quais existem, como se chamam, e qual esta a vista.
 *
 * Vive fora da pagina para poder ser testado — nao ha teste de DOM neste
 * repositorio (`vitest.config.ts`), entao o que decide vira funcao pura. Mesma
 * forma do `HOME_SECTIONS` no `homeLayout.ts`.
 *
 * ESTA E A UNICA COPIA DA ORDEM. A pagina itera daqui; nao ha uma segunda lista
 * no JSX. A licao veio do `arrange_widgets`, que existiu em Rust e em
 * TypeScript ao mesmo tempo e ficou para tras em silencio, com os testes dele
 * passando.
 */

export type SettingsSection = {
  /** Vira ancora e alvo de scroll. Renomear quebra link salvo. */
  id: string;
  /** O que a navegacao e o `<h2>` mostram. */
  title: string;
};

/* A ordem e o desenho, e nao acaso.

   Sincronizacao primeiro porque e a que se visita: ela e a unica cuja
   configuracao alguem procura em vez de encontrar por acaso. O resto desce da
   coisa que fala com FORA (conexoes) para a coisa que so importa quando algo
   deu errado (avancado).

   O agrupamento velho era "Conexao e aparencia", e o "e" no meio do titulo era
   a confissao de que nunca houve criterio: ele juntava Hermes, Univirtus, sync,
   a ponte do M-Finance e o tema claro. */
export const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: "sync", title: "Sincronização" },
  { id: "conexoes", title: "Conexões" },
  { id: "aparencia", title: "Aparência e entrada" },
  { id: "inicio", title: "Início e atualizações" },
  { id: "reunioes", title: "Reuniões" },
  { id: "dados", title: "Dados" },
  { id: "avancado", title: "Avançado" },
];

/**
 * Quanto ANTES do topo de uma secao ela ja conta como a visivel.
 *
 * Sem esta margem, o titulo chega colado no topo e a navegacao ainda marca a
 * secao anterior — a marca fica sempre um passo atras do olho.
 */
const MARGEM = 24;

/**
 * Qual secao esta a vista, dado onde cada uma comeca e o quanto rolou.
 *
 * A ULTIMA que ja passou, e nao a mais proxima: uma secao curta no fim da
 * pagina nunca encheria a tela, e a regra da proximidade deixaria a marca presa
 * na penultima para sempre.
 */
export function secaoVisivel(
  posicoes: { id: string; top: number }[],
  scrollTop: number,
): string {
  let atual = "";
  for (const posicao of posicoes) {
    if (scrollTop >= posicao.top - MARGEM) atual = posicao.id;
  }
  // Antes da primeira, a primeira. Nao rolou nada ainda, e "nenhuma marcada"
  // faria a navegacao parecer quebrada na chegada.
  return atual || posicoes[0]?.id || "";
}
```

- [ ] **Step 4: Rodar e ver passar**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx vitest run src/settingsNav.test.ts 2>&1 | tail -15
```

Esperado: 9 testes verdes.

- [ ] **Step 5: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src/settingsNav.ts apps/desktop/src/settingsNav.test.ts && git commit -m "feat(settings): o catalogo das secoes, antes da navegacao

Sete secoes com nomes que dizem o que tem dentro. O agrupamento velho era
'Conexao e aparencia', e o 'e' no meio do titulo era a confissao de que nunca
houve criterio: ele juntava Hermes, Univirtus, sync, a ponte do M-Finance e o
tema claro.

Sincronizacao primeiro porque e a unica que alguem PROCURA em vez de encontrar
por acaso.

A secao visivel e a ultima que ja passou, e nao a mais proxima: uma secao curta
no fim da pagina nunca encheria a tela, e a proximidade deixaria a marca presa
na penultima para sempre.

Uma copia so da ordem — a pagina itera daqui. A licao veio do arrange_widgets,
que existiu em Rust e em TypeScript ao mesmo tempo e ficou para tras em
silencio, com os testes dele passando.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Reagrupar, e montar a navegação

**Files:**
- Modify: `apps/desktop/src/SettingsPage.tsx`
- Modify: `apps/desktop/src/App.css`

**Interfaces:**
- Consumes: `SETTINGS_SECTIONS`, `secaoVisivel` (Task 2).

- [ ] **Step 1: A página itera o catálogo**

No `return` da `SettingsPage`, troque as cinco `<section>` escritas à mão por uma
estrutura de duas colunas que **itera** `SETTINGS_SECTIONS`. O conteúdo de cada
seção vem de um mapa id → JSX, para a ordem morar num lugar só:

```tsx
  const conteudo: Record<string, ReactNode> = {
    sync: <SyncSettings />,
    conexoes: <><HermesSettings /><UnivirtusSettings /><FinanceActionSettings /></>,
    aparencia: <>
      <Panel label="APARÊNCIA">{/* o bloco do tema, como estava */}</Panel>
      <Panel label="CAPTURA RÁPIDA">{/* como estava */}</Panel>
      <Panel label="ATALHOS">{/* como estava */}</Panel>
    </>,
    inicio: <><StartupSettings /><Panel label="ATUALIZAÇÕES">{/* como estava */}</Panel></>,
    reunioes: <MeetingSettings />,
    dados: <>{/* portabilidade, archive e trash, integridade, DiagnosticoPanel, e os dois <dialog> */}</>,
    avancado: <><Panel label="FUNCTIONS">{/* como estava */}</Panel><Panel label="CRONOCAD">{/* como estava */}</Panel></>,
  };

  return <div className="page settings-page">
    <PaneHeader segments={["M", "SETTINGS"]} meta="SISTEMA" />
    <div className="settings-layout">
      {/* Navegacao de PAGINA, e nao o rail do app: o rail troca de pagina, e
          isto salta dentro de uma. Onze paineis numa coluna so, sem mapa,
          faziam achar 'Integridade' ser rolar e procurar. */}
      <nav className="settings-nav" aria-label="Seções das configurações">
        {SETTINGS_SECTIONS.map((secao) => <a
          key={secao.id}
          href={`#settings-${secao.id}`}
          aria-current={secao.id === visivel ? "true" : undefined}
          data-selected={secao.id === visivel || undefined}
          onClick={(event) => { event.preventDefault(); saltar(secao.id); }}
        >{secao.title}</a>)}
      </nav>
      <div className="settings-content" ref={coluna}>
        {SETTINGS_SECTIONS.map((secao) => <section
          key={secao.id}
          id={`settings-${secao.id}`}
          className="settings-section"
          aria-labelledby={`settings-${secao.id}-title`}
        >
          {/* `settings-section-title` nas SETE. Reunioes era a unica com
              `micro-label`, e por isso parecia uma subsecao das outras. */}
          <h2 id={`settings-${secao.id}-title`} className="settings-section-title">{secao.title}</h2>
          {conteudo[secao.id]}
        </section>)}
        {message ? <StateMessage state={messageState} label={message} /> : null}
      </div>
    </div>
  </div>;
```

Os `<dialog>` de exclusão e de restore ficam na seção `dados`, onde estão hoje —
eles pertencem aos botões que os abrem.

- [ ] **Step 2: A seção visível**

Ainda em `SettingsPage`:

```tsx
  const coluna = useRef<HTMLDivElement>(null);
  const [visivel, setVisivel] = useState(SETTINGS_SECTIONS[0].id);

  /* Mede a posicao das secoes a cada rolagem em vez de guardar no estado: a
     altura muda quando um `<details>` do Archive abre, e uma medida guardada na
     montagem apontaria para o lugar errado a partir do primeiro clique. */
  useEffect(() => {
    const alvo = coluna.current;
    if (!alvo) return;
    const aoRolar = () => {
      const posicoes = SETTINGS_SECTIONS.map((secao) => ({
        id: secao.id,
        top: (document.getElementById(`settings-${secao.id}`)?.offsetTop ?? 0),
      }));
      setVisivel(secaoVisivel(posicoes, alvo.scrollTop));
    };
    aoRolar();
    alvo.addEventListener("scroll", aoRolar, { passive: true });
    return () => alvo.removeEventListener("scroll", aoRolar);
  }, []);

  const saltar = useCallback((id: string) => {
    document.getElementById(`settings-${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);
```

- [ ] **Step 3: O estilo**

Em `App.css`, junto dos outros blocos de `settings-`. Confira os tokens reais
antes — não invente nenhum:

```bash
cd /c/Dev/pessoal/m-os && grep -n "^  --" packages/design-system/tokens.css | head -40
```

```css
/* Duas colunas: a navegacao gruda, o conteudo rola. */
.settings-layout { display: grid; grid-template-columns: 180px 1fr; gap: var(--space-6); align-items: start; }
.settings-nav { position: sticky; top: var(--space-4); display: flex; flex-direction: column; gap: var(--space-1); }
.settings-nav a { padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); color: var(--text-secondary); text-decoration: none; border-left: 2px solid transparent; }
.settings-nav a[data-selected] { color: var(--text-primary); border-left-color: var(--accent); background: var(--surface-raised); }

/* Numa janela estreita a navegacao vira uma linha acima do conteudo, em vez de
   espremer as duas colunas ate nenhuma servir. */
@media (max-width: 900px) {
  .settings-layout { grid-template-columns: 1fr; }
  .settings-nav { position: static; flex-direction: row; flex-wrap: wrap; }
}
```

- [ ] **Step 4: Compilar e testar**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx tsc --noEmit 2>&1 | head -20 && npm test 2>&1 | tail -10
```

Esperado: sem erro, todos verdes.

- [ ] **Step 5: Ver na janela de verdade**

Skill `ver-o-app`. Confira:

1. as sete seções aparecem, na ordem do catálogo, com Sincronização primeiro;
2. clicar em cada uma das sete salta para ela;
3. rolar à mão move a marca da navegação, e a marca não fica um passo atrás;
4. abrir um `<details>` do Archive e rolar de novo: a marca continua certa — é o caso que a medida por rolagem existe para cobrir;
5. **nenhum painel sumiu.** Compare com a `settings-antes.png` da Task 1, painel por painel: onze deles, mais os dois `<dialog>`;
6. estreitar a janela para menos de 900px: a navegação vira linha e nada fica espremido.

- [ ] **Step 6: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src/SettingsPage.tsx apps/desktop/src/App.css && git commit -m "feat(settings): sete secoes com nome honesto, e uma navegacao que salta

Onze paineis numa coluna so, sem mapa: achar 'Integridade' ou 'CronoCAD' era
rolar e procurar. A navegacao e de PAGINA e nao o rail do app — o rail troca de
pagina, isto salta dentro de uma.

O agrupamento mudou junto porque o mapa de um lugar mal agrupado so ajuda a
chegar rapido no lugar errado. 'Conexao e aparencia' juntava Hermes, Univirtus,
sync, a ponte do M-Finance e o tema claro; o 'e' no titulo era a confissao.

Reunioes ganhou `settings-section-title` como as outras. Ela era a unica com
`micro-label`, e por isso parecia uma subsecao das outras seis.

A secao visivel e medida a cada rolagem, e nao na montagem: a altura muda quando
um `<details>` do Archive abre, e a medida guardada apontaria errado a partir do
primeiro clique.

Conferido na janela real: [PREENCHER com os seis passos].

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: A seção Sincronização recebe o automático

**Só faça esta task depois do plano de sync** (`2026-08-28-sync-automatico.md`).
Antes dele não há automático para descrever.

**Files:**
- Modify: `apps/desktop/src/SettingsPage.tsx` (`SyncSettings`)

- [ ] **Step 1: A cópia deixa de mentir**

`SyncSettings` tem hoje, no cabeçalho, um comentário que o automático torna falso:

> O botao de sincronizar existe porque hoje a rodada e MANUAL, e dizer isso na
> tela e mais honesto que um automatico que ninguem pediu.

Troque-o pela verdade nova, sem apagar a história:

```tsx
/**
 * A sincronizacao, no Settings.
 *
 * Ate 28/08 a rodada era MANUAL, e o comentario aqui defendia isso: "dizer isso
 * na tela e mais honesto que um automatico que ninguem pediu". Estava certo, e
 * deixou de estar quando alguem pediu — o fluxo casa > trabalho > celular tinha
 * o elo do meio na mao enquanto o celular ja sincronizava sozinho.
 *
 * O botao ficou. Ele nao e mais o unico caminho: e o que ADIANTA a proxima
 * rodada, para quem esta de saida e nao quer esperar o proximo gatilho.
 */
```

- [ ] **Step 2: Mostrar o que o automático faz**

No painel, entre o formulário e a `fact-grid`, diga quando ele roda — sem isso,
"automático" é uma palavra que não se pode conferir:

```tsx
    <p className="support-copy">Sincroniza sozinho: ao abrir, ao voltar para a frente, depois de você mexer em algo, e a cada 15 minutos. O botão abaixo só adianta a próxima.</p>
```

E acrescente a última rodada à `fact-grid`, ao lado de SEGREDO e NA FILA:

```tsx
      <div><dt>ÚLTIMA</dt><dd>{status?.lastSyncAt ? relativeTime(status.lastSyncAt) : <span className="fact-empty">Nunca</span>}</dd></div>
```

`relativeTime` já existe no `App.tsx` e é usado em `:2665` — se ela não estiver
exportada, exporte-a de lá ou mova-a para um módulo compartilhado, pela mesma
razão do `functionLabels.ts`.

- [ ] **Step 3: Compilar, testar, ver**

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx tsc --noEmit && npm test 2>&1 | tail -5
```

Depois, na janela: a seção Sincronização mostra a frase dos gatilhos e o "ÚLTIMA"
com um tempo relativo que muda depois de uma rodada.

- [ ] **Step 4: Commit**

```bash
cd /c/Dev/pessoal/m-os && git add apps/desktop/src/SettingsPage.tsx && git commit -m "feat(settings): a secao de sync conta quando ele roda sozinho

'Automatico' e uma palavra que ninguem consegue conferir. Os quatro gatilhos na
tela, e a hora da ultima rodada ao lado da fila: sem isso, um sync que parou de
funcionar parece igual a um que funciona.

O comentario que defendia a rodada manual foi trocado sem apagar a historia —
ele estava certo ate alguem pedir o contrario.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

---

## Verificação final

```bash
cd /c/Dev/pessoal/m-os/apps/desktop && npx tsc --noEmit && npm test 2>&1 | tail -10
cd /c/Dev/pessoal/m-os && wc -l apps/desktop/src/App.tsx apps/desktop/src/SettingsPage.tsx
```

E a comparação que realmente importa: `settings-antes.png` contra a página de
hoje. **Os onze painéis continuam lá**, em seções diferentes e com nomes
diferentes. Um painel a menos é a única forma de esta mudança ter dado errado —
e é a que nenhum teste deste repositório pegaria.

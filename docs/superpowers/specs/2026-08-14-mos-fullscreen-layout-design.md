# Spec — layout full-screen responsivo

Data: 2026-08-14
Escopo: apenas largura e grade. Widgets da Home e Fase 4 ficam para o ciclo seguinte.

---

## 0. Problema

`.page` (`App.css:346`) declara `width: min(100%, var(--content-max))`, com
`--content-max: 1100px` (`mos-tokens.css:172`). Como `.page` é o container mais externo
de toda tela, o teto de 1100px é herdado por tudo — inclusive por grades, que não têm
relação com legibilidade de texto.

Não há `margin-inline: auto`, então a faixa de 1100px encosta à esquerda. Numa janela de
~2545px sobram ~1300px de vazio assimétrico à direita.

As media queries de `1280px` (`App.css:2242`) e `960px` (`App.css:2254`) já tratam o lado
estreito. O buraco é só para cima: acima de 1100px nada acontece.

---

## 1. Decisão

Inverter onde a medida de leitura mora. A página passa a ser fluida; o teto desce para os
blocos que contêm texto corrido.

Superfícies fluidas: Home, kanban, Library — ocupam 100% e ganham colunas conforme a
largura.

Superfícies de leitura: Settings e blocos de texto — mantêm medida legível.

---

## 2. Tokens

Um token novo, no bloco `/* extensões de implementação */` de `mos-tokens.css`, ao lado
de `--content-max`:

```css
--measure: 70ch;
```

Extensão explícita e documentada, como `DECISIONS.md` ADR-019 exige.

`--content-max: 1100px` **não é removido**. Deixa de ser aplicado por `.page` e passa a
ser opt-in de `.settings-page`, a única página que é leitura e formulário de ponta a ponta.

---

## 3. Mudanças em `App.css`

**`.page` (linha 346)** — remover o teto:

```css
.page {
  width: 100%;
  min-height: 100%;
  padding: var(--space-5) var(--content-margin) var(--space-7);
}
```

**`.settings-page` (linha 1919)** — assumir o teto que a página largou:

```css
.settings-page {
  display: grid;
  gap: var(--space-6);
  max-width: var(--content-max);
}
```

Alinhado à esquerda, **sem** `margin-inline: auto`. Todo o resto do app alinha à
esquerda a partir do rail; centralizar só o Settings criaria uma exceção visual sem
motivo.

**`.home-sections` (linha 531)** — `repeat(2, minmax(0, 1fr))` passa a:

```css
grid-template-columns: repeat(auto-fit, minmax(var(--column-min), 1fr));
```

**`.context-switcher` (linha 538)** — `repeat(4, minmax(0, 1fr))` passa a:

```css
grid-template-columns: repeat(auto-fill, minmax(var(--column-min), 1fr));
```

A diferença entre `auto-fit` e `auto-fill` é intencional e é o ponto sutil desta spec.
`auto-fit` colapsa trilhas vazias, então os painéis da Home dividem a largura inteira
entre si. `auto-fill` preserva as trilhas, então um único workspace continua do tamanho
de um chip em vez de esticar pela tela toda.

Nenhum breakpoint novo. `--column-min: 260px` já existe e resolve sozinho.

**Blocos de texto sem cap** — receber `max-width: var(--measure)`:

- `.support-copy` (linha 1940) — parágrafos do Settings
- `.empty-state` (linha 1121)

---

## 4. O que não muda

- Media queries de `1280px` e `960px`.
- Kanban de seis colunas (`App.css:1407`).
- Mínimo de janela 840×600 (`tauri.conf.json`).
- Caps de medida que já existem e já estão corretos: `.startup-state h1/p` (linha 470),
  `.detail-header p` (1181), `.resource-note p` (1358), `.hermes-empty` (770),
  `.hermes-quiet` (2358).

---

## 5. Verificação

O front não tem infraestrutura de teste — `package.json` define apenas
`"build": "tsc && vite build"`, sem vitest ou playwright. Esta spec não promete teste
automatizado.

Verificação é manual e tem três larguras obrigatórias:

1. **840px** (mínimo da janela) — media queries antigas continuam valendo, nada quebra.
2. **1180px** (padrão da janela) — Home não fica pior do que hoje.
3. **~2545px** — conteúdo ocupa a largura, vazio assimétrico some, texto do Settings
   permanece legível.

Mais `npm run build` passando.

---

## 6. Limite conhecido

A 2545px, quatro painéis dividindo a largura dão ~600px cada. Listas de texto curto ficam
ralas nessa medida. Esta spec corrige o enquadramento, não a densidade.

A densidade é problema do ciclo seguinte: os oito widgets do
`M-OS Home v0.6.dc.html` foram desenhados para essa grade. Registrar isso aqui evita que
o resultado desta spec seja lido como falha.

---

## 7. Fora de escopo

- Widgets da Home (`NOW`, `INBOX`, `PARADO`, `TODAY`, `PROJECTS`, `SALVO ESTA SEMANA`,
  `SHORTCUTS`, `SEM HORA`).
- Fase 4 do roadmap: data de agendamento em Task, sessão de trabalho para o `NOW`.
- Qualquer mudança em core, storage ou API. Esta spec é CSS e tokens.

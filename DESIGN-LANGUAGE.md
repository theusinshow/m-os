# Design Language — Sistema Portátil

> Destilação do sistema de design da **Coded by M** em uma referência **portátil e reutilizável**.
> Copie este arquivo para qualquer projeto novo. Ele carrega a *gramática visual* (estrutura, temperatura,
> raridade, motion) independente da marca — e mostra onde trocar o que for específico de marca.
>
> A fonte original é específica da Home cinematográfica da CbM (`docs/10-design-system.md`,
> `docs/08.5-visual-direction.md`). **Este documento é a versão que viaja.** Valores aqui refletem o
> **código em produção** (`tailwind.config.ts`, componentes), não apenas a especificação.

---

## 0. O núcleo em uma frase

**Fundo profundo + estrutura quente + um único sinal que corta.**

Três elementos, nada mais. Toda a identidade é construída sobre essa gramática de três cores.
Se você entender só isso, já pode replicar 80% do caráter em outro projeto:

| Papel | O que é | Regra |
|---|---|---|
| **Base** | O espaço escuro onde tudo existe | Nunca preto puro — sempre com *temperatura* |
| **Estrutura** | Texto, linhas, geometria, UI primária | Nunca branco puro — sempre quente |
| **Sinal** | A cor de ação/destaque que corta | Raro. Máx. 2–3 por viewport |

Quatro princípios que governam cada decisão:

1. **Construção, não decoração.** Cada linha, ponto, borda existe como parte de uma estrutura. Ornamento sem função é ruído.
2. **Temperatura, não neutro.** As cores têm viés (verde no escuro, bege no claro). Trocar por `#000`/`#fff` mata a identidade.
3. **Raridade cria impacto.** O sinal só funciona porque aparece pouco. Quanto mais raro, mais forte.
4. **Angular, não arredondado.** `border-radius: 0` em botões, cards, overlays. Curvas só em elementos circulares com motivo.

---

## 1. Cores

### 1.1 O trio canônico (a identidade inteira)

```
Base      #000F08   "Deep Black"  — fundo de página, canvas 3D, fog. NÃO é #000000 (tem verde imperceptível).
Estrutura #F5F2ED   "Off White"   — texto, wireframe 3D, logo. NÃO é #ffffff (tem bege quente).
Sinal     #FB3640   "Signal Red"  — ação, hotspot ativo, conector, foco. Raro por regra.
```

Estes três nunca mudam dentro da CbM. **Para outra marca, o `#FB3640` é o único que você deve trocar**
(veja §9 "Adaptar a uma nova marca"). Base e estrutura mantêm a *temperatura*, mesmo que o hue mude.

### 1.2 Paleta completa (valores em produção)

```
base            #000F08   fundo base
forest          #070B08   fundos secundários, cards, overlays (bem mais escuro que a base)
white           #F5F2ED   texto/estrutura primária
red             #FB3640   sinal / ação
red-dark        #C42030   hover do sinal
border          #111511   borda padrão (temperatura verde, quase invisível)
border-active   #1A2418   borda em hover/estado ativo
cursor-gray     #97938b   cor do cursor customizado

gray-100        #E8E4DE   texto sobre fundos claros
gray-200        #C8C4BE   texto de corpo importante
gray-400        #8A8780   texto secundário, labels, datas
gray-600        #4A4844   texto terciário, captions, numerações
gray-800        #1E1E1A   separadores leves
```

> **Nota de fork:** a spec antiga (`docs/10`) usava verdes mais claros (`forest #0E1810`,
> `border #1a2a1e`, `border-active #2a4a32`). O **código evoluiu para verdes mais escuros/subliminares**
> (acima). Ao portar, use os valores de código — são os que renderizam hoje. Se quiser cards mais
> presentes, os valores antigos ainda são uma alternativa válida.

### 1.3 Tokens prontos — Tailwind

```ts
// tailwind.config.ts
theme: {
  extend: {
    colors: {
      cbm: {
        black:           "#000F08",
        forest:          "#070B08",
        white:           "#F5F2ED",
        red:             "#FB3640",
        "red-dark":      "#C42030",
        border:          "#111511",
        "border-active": "#1A2418",
        gray: {
          100: "#E8E4DE",
          200: "#C8C4BE",
          400: "#8A8780",
          600: "#4A4844",
          800: "#1E1E1A",
        },
      },
    },
    fontFamily: {
      display: ["Panchang", "sans-serif"],
      body:    ["Satoshi", "sans-serif"],
    },
  },
}
```

### 1.4 Tokens prontos — CSS Custom Properties

```css
:root {
  /* Cor */
  --base:          #000F08;
  --forest:        #070B08;
  --white:         #F5F2ED;
  --signal:        #FB3640;
  --signal-dark:   #C42030;
  --border:        #111511;
  --border-active: #1A2418;
  --cursor:        #97938b;

  --gray-100: #E8E4DE;
  --gray-200: #C8C4BE;
  --gray-400: #8A8780;
  --gray-600: #4A4844;
  --gray-800: #1E1E1A;

  /* Glow do sinal — teto rígido 0.08 */
  --glow:        rgba(251, 54, 64, 0.06);
  --glow-strong: rgba(251, 54, 64, 0.08);
}
```

### 1.5 Mapeamento semântico

```
Fundo de página          base
Fundo de card/overlay    forest
Título                   white
Corpo                    gray-200
Texto secundário         gray-400
Texto terciário/caption  gray-600
Separador                gray-800 | border

Ação primária (bg)       signal        Ação primária (texto)   base
Ação secundária (texto)  white         Ação secundária (borda) border-active

Borda padrão             border        Borda hover/ativa       border-active
Label/tag                signal @ 0.7 opacity
Destaque inline / foco   signal
```

### 1.6 Contraste (verificado)

```
white sobre base       15.7:1  AAA
gray-400 sobre base     4.7:1  AA
signal (foco) sobre base — suficiente p/ WCAG AA sem cor alternativa
```

---

## 2. Tipografia

### 2.1 Dois typefaces, exclusivamente

- **Panchang** — display e hierarquia. Pesos 200–800. Serifa angular; **nunca** para corpo longo. Sem italic.
- **Satoshi** — corpo e interface. Pesos 300, 400, 500, 700.

Carregadas via **Fontshare CDN** (sem `next/font`, sem `@font-face`):

```html
<link rel="preconnect" href="https://api.fontshare.com" />
<link
  rel="stylesheet"
  href="https://api.fontshare.com/v2/css?f[]=panchang@200,300,400,500,600,700,800&f[]=satoshi@300,400,500,700&display=swap"
/>
```

```css
body { font-family: "Satoshi", system-ui, -apple-system, sans-serif; }
```

> Ao portar para outra marca, **este é o segundo lugar a customizar** (o primeiro é o sinal).
> O par "display serifada-angular + grotesk neutra de corpo" é a *fórmula* — troque as fontes,
> mantenha a divisão de papéis.

### 2.2 Escala de tipos

| Token | Família | Peso | Tamanho | Tracking | LH | Uso |
|---|---|---|---|---|---|---|
| `display` | Panchang | 800 | clamp(56px, 8vw, 80px) | -0.03em | 0.92 | Hero H1, CTA principal |
| `h1` | Panchang | 700 | clamp(40px, 5vw, 56px) | -0.02em | 1.05 | Títulos de seção |
| `h2` | Panchang | 600 | clamp(28px, 3.5vw, 36px) | -0.01em | 1.15 | Subtítulos |
| `h3` | Panchang | 500 | clamp(18px, 2vw, 22px) | 0 | 1.25 | Títulos de card, etapas |
| `body-lg` | Satoshi | 300 | clamp(14px, 1.5vw, 17px) | 0 | 1.75 | Subtítulo hero, intro |
| `body` | Satoshi | 400 | 15px | 0 | 1.7 | Corpo padrão |
| `body-sm` | Satoshi | 300 | 13px | 0 | 1.7 | Descrição de card |
| `label` | Satoshi | 500 | 9px | 0.35em | 1.4 | Tags, categorias |
| `caption` | Satoshi | 300 | 10px | 0.3em | 1.5 | Legendas, metadados |
| `ui` | Panchang | 600 | 11px | 0.15em | 1 | Botões, nav |
| `micro` | Satoshi | 400 | 8–9px | 0.25em | 1.4 | Datas, apoio |

`label`, `caption`, `ui`, `micro` são **sempre `uppercase`**.

### 2.3 Duas regras que criam a "textura" da marca

- **Títulos: tracking apertado** (`-0.02em` a `-0.03em`). Intencional, tenso.
- **Micro-labels: tracking largo** (`0.15em` a `0.5em`) + uppercase + Satoshi pequeno.
  Essa combinação de uppercase minúsculo ultra-espaçado é a assinatura tipográfica — use em tags,
  cues, seals, numerações de seção.

### 2.4 Não fazer

```
✗ Panchang em corpo > 3 linhas
✗ Satoshi 700 como "quase Panchang" (se precisa de peso, use Panchang)
✗ tracking positivo alto em display (> 0.05em)
✗ system-ui / Inter visível ao usuário final
✗ italic em Panchang
```

---

## 3. Espaçamento, layout, forma

### 3.1 Escala base — múltiplos de 4

```
4 · 8 · 12 · 16 · 24 · 32 · 40 · 48 · 64 · 80 · 96 · 100 · 128 (px)
```

### 3.2 Layout

```
Container principal   max-width 1440px · padding-x clamp(24px, 5vw, 80px) · centralizado
Coluna de conteúdo    max-width 900px
Coluna estreita       max-width 680px
Grid 2-col            1fr 1fr desktop / 1fr mobile · gap clamp(24px, 4vw, 48px)

Padding vertical de seção   60px mobile · 100px desktop · 120px hero/CTA
Padding de card             12–16 compacto · 24–28 médio · 36–48 grande
```

### 3.3 Breakpoints

```
mobile  < 640    tablet 640–1024    desktop > 1024    wide > 1440
```

### 3.4 Border-radius — essencialmente ZERO

`border-radius: 0` em botões, cards, overlays, focus ring. A linguagem é angular/estrutural.
Curvas só em círculos com propósito (hotspot, ponto âncora, avatar).
*Exceções toleradas no código:* painel de menu glass (`rounded-lg`), pill de "pular".

### 3.5 Sombras / glass / glow

```
Card                0 2px 12px rgba(0,0,0,0.28)
Glow do sinal       0 0 10px <signal>  (usar com moderação; teto rgba do sinal @ 0.08)
Glass (menu/nav)    backdrop-filter: blur(12px)
                    background: linear-gradient(270deg, rgba(0,15,8,0.95), rgba(0,15,8,0.72))
                    border: 1px solid rgba(245,242,237,0.1)
                    box-shadow: 0 26px 60px -22px rgba(0,0,0,0.7)
Scrollbar           thumb rgba(245,242,237,0.14) → hover 0.26 · track transparente · 8px
```

### 3.6 Z-index

```
Canvas 3D           0
HTML de seção       1–9
Overlays sobre 3D   10–19
Card de projeto     20
Transição de cena   40
Wipe de navegação   50
Navbar/rail/cue     60
Loading             100
Cursor customizado  9999
```

---

## 4. Motion

### 4.1 Durações

```
instant   150ms   micro-interações, hover
quick     300ms   card hover, focus, toggle
normal    600ms   entrada de seção, dissolve
slow      900ms   transição de cena, câmera
xslow     1200ms  cenas cinematográficas
narrative 2600ms  build animations
rotation  28s     loops de rotação (não é transição)
```

### 4.2 Easings

```
build / enter   power2.out     cubic-bezier(0.33, 1, 0.68, 1)   elementos surgindo
draw            power1.inOut    cubic-bezier(0.45, 0, 0.55, 1)   linhas se desenhando
exit            power1.in                                        elementos saindo
camera          smoothstep      t²(3 − 2t)                       câmera entre keyframes

/* usados nos componentes de navegação */
spring          cubic-bezier(0.34, 1.42, 0.5, 1)   overshoot — SÓ p/ abrir menu, nunca em hover
expo            cubic-bezier(0.22, 1, 0.36, 1)      saída suave — expandir/colapsar
```

### 4.3 Stagger

```
texto     100ms   linhas de copy em sequência
card      150ms   cards entrando
fragmento 220ms   fragmentos 3D surgindo
camada    550ms   camadas de terreno (trás → frente)
```

### 4.4 Princípios de motion

- **Construção antes da forma.** Nada surge pronto: ponto → linha → estrutura → forma.
- **"Porsche, não videogame."** Suave, controlado, cinematográfico. Sem bounce/neon em micro-interações.
- **Estruturas vivas.** Mesmo em repouso, vida de baixa intensidade (deriva de partículas, respiração de mesh).
- **Hover revela, não só destaca.** Mostra algo que não estava visível — não é só troca de cor.
  Entrada `power2.out` 150–200ms; saída ~80% da entrada. Sem overshoot em hover.
- **Scroll suave** via Lenis (`lerp: 0.08`).
- **`prefers-reduced-motion` é first-class:** builds pulam para o estado final, loops pausam, câmera
  salta sem interpolação. Todo hook de animação checa isso.

```css
/* Só animar propriedades de compositor */
transition: transform, opacity;   /* ✓ */
/* ✗ nunca width, height, padding, margin */
```

---

## 5. Elementos-assinatura (portáveis)

Estes são os componentes que dão o *caráter* — não só tokens. Reimplemente o conceito, adapte os valores.

### 5.1 Foco angular vermelho — o mais fácil e mais impactante

```css
:focus-visible {
  outline: 2px solid var(--signal);
  outline-offset: 3px;
  border-radius: 0;
}
```
Foco = identidade **e** acessibilidade juntas. Nunca `outline: none` sem substituto.

### 5.2 Botão primário (mesh procedural)

O CTA-assinatura é um botão retangular com **malha triangulada procedural** animada por frame
(via `setAttribute` direto no SVG, zero re-render). No hover: halo radial seguindo o cursor,
wash diagonal do sinal, seta vermelha que gira. Versão simplificada, portável:

```
Base:    px-12 py-7 · uppercase · Satoshi 500 · tracking 0.3em
         bg base/60 · backdrop-blur-sm · border 1px white/55 · radius 0
Hover:   border white/100 · halo radial rgba(245,242,237,0.22) no ponto do cursor
         + wash linear-gradient(120deg, transparent, rgba(251,54,64,0.08), transparent)
Ícone:   seta/triângulo do sinal que gira no hover
```

Variante sólida clássica (fallback simples):
```
bg signal · text base · px-7 py-3.5 · Panchang 600 11px tracking 0.15em uppercase · radius 0
hover: bg signal-dark · transition background 150ms
```

### 5.3 Cursor customizado zone-aware

Um triângulo wireframe girando (stroke `cursor-gray #97938b`, 1.5px) + ponto de mira central.
Aparece **só sobre `<canvas>` ou zonas `[data-cursor="triangle"]`** (detectado via `elementFromPoint`),
some sobre HTML normal pra não atrapalhar leitura. Follow inercial (LERP 0.25 em rAF), spin contínuo
de 6s. Nunca em `(pointer: coarse)`; desligado sob reduced-motion. Esconde o cursor nativo (`cursor: none`)
só nas zonas ativas.

### 5.4 Navegação de progresso (rail + dots)

- **Desktop:** trilho na borda direita (`fixed right-6 top-1/2 z-60`), colapsado ~22px mostrando só
  marcadores triangulares + um fio de progresso do sinal cuja "cabeça" pulsante desce com o scroll.
  Hover expande (~296px, `width 0.44s expo`) num menu glass com linhas numeradas, número gigante de
  fundo (Panchang 800, opacity 0.05), barra ativa que desliza com `spring`, entrada em cascata
  (`delay: i*34ms`). Drag-to-scrub que só engata após >6px (pra não comer cliques).
- **Mobile:** coluna de losangos 45° (7px vermelho ativo com glow, passados preenchidos, futuros
  em contorno). Touch targets de 28px.

### 5.5 Wipe de transição direcional

Navegação capítulo-a-capítulo não é scroll: um painel full-screen `base #000F08` varre na direção do
movimento (GSAP): cobre `yPercent 100→0, 0.42s power3.in`, troca o conteúdo escondido, revela
`0→-100, 0.5s power3.out`. Além dele, um **overlay acoplado ao scroll** que escurece para `0.92` de
opacidade quando a emenda entre seções cruza o centro do viewport — dissolve entre cenas pelo verde
da marca em vez de corte seco.

### 5.6 Logo como construção

Símbolo de 3 paths `strokeWidth 9` round-cap: duas estruturas `white` + uma diagonal `signal` que
"corta". A diagonal **sempre anima por último e mais rápido** — uma incisão. O loader replica essa
sequência de construção. Conceito replicável: *o logo é desenhado, não colocado.*

---

## 6. Receitas de componente

### Card

```
bg forest · border 1px border · border-left 2px signal (acento de identidade) · radius 0
shadow 0 2px 12px rgba(0,0,0,0.28)
  categoria:  Satoshi 500 · 9px · tracking 0.32em · uppercase · signal @ 0.7
  título:     Panchang 700 · 14–16px · white
  descrição:  Satoshi 300 · 12–13px · gray-400 · LH 1.7
  CTA inline: Panchang 600 · 11px · tracking 0.15em · uppercase · white
              border-bottom 1px signal · hover: white → signal
hover: border → border-active · transition border-color 200ms
```

### Navbar glass

```
fixed top-0 · z-60 · h 60px desktop / 56px mobile
bg: transparente → base/90 + backdrop-blur ao scrollar ~100px
border-bottom: none → 1px border ao scrollar
wordmark: Panchang 700 16px white · "by"/acento em signal
links: Satoshi 400 · 11px · tracking 0.15em · uppercase · gray-400 → white (hover 200ms)
Regra: navbar sempre acessível. Cinematográfico ≠ difícil de navegar.
```

### Label de seção (pré-título)

```
Satoshi 500 · 9px · tracking 0.35em · uppercase · signal @ 0.7
::before { content:''; width:24px; height:1px; background:signal; opacity:0.5 }  /* traço curto */
```

---

## 7. 3D (se aplicável)

Só se o projeto usar WebGL. O look é **estúdio escuro com iluminação técnica** — não neon, não cyberpunk.

```
Material          MeshBasicMaterial (sem luz — flat premium + performance)
Post-processing   nenhum (sem bloom/DoF)
Assets            geometria procedural — sem .glb/.gltf, sem texturas
DPR               [1, 1.5] em produção (evitar 2.0 em mobile)
frameloop         "demand" quando parado · "always" quando há movimento
Fundo/fog         mesma cor da base (#000F08) — sem horizonte visível, profundidade infinita
Câmera            fov 42 · near 0.1 · far 100
Estrutura         nós/arestas em white; ápice ativo em signal; camadas distantes recuam por opacidade
Canvas            aria-hidden="true" · conteúdo textual SEMPRE em HTML, nunca no canvas
```

Performance de contexto: monte cada cena WebGL por proximidade de viewport (lazy) pra **nunca ter
múltiplos contextos vivos** ao mesmo tempo. Passe um booleano `live`/`active` para cada cena
congelar quando fora de foco.

---

## 8. O que evitar (anti-padrões que matam a identidade)

**Cor**
```
✗ #000000 como fundo (preto puro sem temperatura → dashboard genérico)
✗ #ffffff como texto (branco frio → look bootstrap)
✗ azul, roxo, cyan, verde-lima em qualquer elemento
✗ gradiente linear como fundo de seção (#000→#111 é o sinal nº1 de dark theme sem identidade)
✗ sombras coloridas exceto o sinal @ opacity ≤ 0.08
✗ mais de 2–3 elementos do sinal visíveis ao mesmo tempo
```

**Tipografia**
```
✗ system-ui / Inter visível · ✗ display serifada em corpo longo · ✗ grotesk como título de peso
✗ tracking positivo alto em display · ✗ border-radius em botões/cards
```

**Layout / motion / 3D**
```
✗ border-radius > 4px em cards · ✗ separadores em cor quente visível
✗ bounce/overshoot em hover · ✗ hover > 300ms · ✗ transition em width/height/padding/margin
✗ animação que ignora prefers-reduced-motion
✗ emissive/bloom/glow forte · ✗ textos dentro do canvas · ✗ DPR > 1.5 em mobile
✗ cores frias em geometria 3D
```

---

## 9. Adaptar a uma nova marca

Este sistema é uma *estrutura*, não uma prisão. Para levar o caráter a outro projeto sem clonar a CbM:

1. **Troque o sinal.** `#FB3640` é a única cor de destaque. Substitua por outro hue **saturado e raro**
   (âmbar, cyan elétrico, magenta…). Mantenha a *regra de raridade* (2–3 por viewport) — é ela, não o
   hue, que cria o impacto.
2. **Mantenha a temperatura da base e da estrutura.** Se a marca é fria, use uma base azul-quase-preta
   (`#080B0F`) e um branco levemente azulado — mas **nunca** `#000`/`#fff` puros. A temperatura é o que
   separa "premium" de "template".
3. **Troque o par tipográfico, mantenha os papéis.** Uma display expressiva + uma grotesk neutra de
   corpo. Nunca misture os papéis.
4. **Guarde os invariantes estruturais:** `border-radius: 0`, foco visível na cor do sinal, tracking
   apertado em títulos / largo em micro-labels, motion sem bounce em hover, `prefers-reduced-motion`.
5. **Escolha quanto do 3D/cinemático carregar.** O sistema funciona só com cor+tipo+motion+foco.
   As cenas WebGL e o wipe são camadas opcionais de ambição.

Checklist mínimo pra "sente-se como a mesma família":
- [ ] Base escura com temperatura (não `#000`)
- [ ] Off-white quente para texto (não `#fff`)
- [ ] Um único sinal raro para ação/foco/destaque
- [ ] `border-radius: 0` na linguagem angular
- [ ] Foco `2px solid <sinal>`, offset 3px
- [ ] Títulos tracking negativo · micro-labels uppercase tracking largo
- [ ] Motion 150–300ms sem overshoot em hover · reduced-motion respeitado

---

*Derivado de `docs/10-design-system.md` e `docs/08.5-visual-direction.md` (CbM, spec 2026-05-30),
com valores atualizados a partir do código em produção (`tailwind.config.ts`, `components/**`).
Onde a spec e o código divergiam, este documento segue o código.*

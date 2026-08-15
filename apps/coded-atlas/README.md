# Coded Atlas

> Ferramenta interna da **Coded by M**. Recebe a URL de um projeto web e gera um
> **catálogo visual premium** — screenshots desktop/mobile, full page, thumbnails,
> mockups, composições para redes e dados estruturados — pronto para páginas de case,
> apresentações e portfólio.

Não é um SaaS. É uma peça do Laboratório: prova de capacidade técnica e ferramenta
real de produtividade para transformar sites finalizados em cases.

Para o **porquê** e o escopo, veja [`docs/product.md`](docs/product.md).
Para a **linguagem visual**, [`docs/design.md`](docs/design.md).
Para a **especificação técnica** (fonte da verdade), [`docs/architecture.md`](docs/architecture.md).

---

## O que ele gera

A partir de uma URL, em uma execução:

- **Screenshots** desktop (1440×900) e mobile (390×844), viewport e **full page**;
- **Thumbnails** para portfólio (640×400 e 320×640, WebP);
- **Capa** do catálogo (1.91:1) a partir da `og:image` ou de um smart crop;
- **Mockups** com moldura (navegador / celular) e **mockups 3D** em perspectiva, fundo transparente;
- **Composições para redes** prontas para postar (1:1, 9:16, 16:9);
- **Seções** capturadas individualmente, com nomes inteligentes derivados do DOM;
- **Vídeo** de scroll (desktop e mobile, WebM);
- **Páginas extras** e **estados de interação** (ex.: menu aberto) sob demanda;
- **Inspeção** do site: paleta de cores, tipografia e tech stack;
- **`catalog.json`** — dados estruturados (fonte única em [`lib/types.ts`](lib/types.ts));
- **Rascunho de case** (`case-draft.mdx`) e **manifesto** para o portfólio.

Tudo é determinístico e roda 100% local — sem banco, sem login, sem cloud, sem IA.

---

## Stack

Next.js (App Router) · TypeScript · Tailwind CSS v4 · Playwright (Chromium) · Sharp · pixelmatch.

---

## Pré-requisitos

- **Node 18+**
- **Chromium do Playwright** — instalado automaticamente pelo `postinstall`
  (`playwright install chromium`).

---

## Instalação

```bash
# 1. dependências (o postinstall já baixa o Chromium)
npm install

# 2. variáveis de ambiente (opcional — há defaults sensatos)
cp .env.example .env
```

Se o Chromium não baixar sozinho:

```bash
npx playwright install chromium
```

---

## Como rodar

### Windows (atalho)

```bat
start.bat
```

Limpa o cache `.next`, sobe o servidor em `npm run dev -- -p 5000` e abre
`http://localhost:5000` no navegador.

### Manual (qualquer SO)

```bash
npm run dev          # http://localhost:3000
# ou em outra porta:
npm run dev -- -p 5000
```

Build de produção:

```bash
npm run build
npm run start
```

---

## Como gerar um catálogo

1. Acesse `/` (landing) e clique em **Gerar Catálogo**, ou vá direto a `/generate`.
2. Cole a **URL**. O nome e o slug são sugeridos automaticamente (editáveis).
3. Escolha o **tipo de projeto** e, se quiser, ligue/desligue **Seções** e **Vídeo**
   (desligar o vídeo deixa a geração bem mais rápida).
4. Opcionalmente, adicione cliente, descrição, páginas extras e estados de interação.
5. Clique em **Gerar catálogo**. O progresso aparece em etapas reais (SSE); o status
   persiste num toast mesmo se você trocar de aba.
6. Ao concluir, abra o catálogo em `/projects/[slug]`.

### Reprocessar

Na página do projeto, **Reprocessar capturas** regenera tudo com os mesmos dados
(sem redigitar o formulário) e **preserva o rascunho de case**.

### Biblioteca

`/projects` lista todos os catálogos gerados — com busca, filtro por categoria,
ordenação, seleção em lote e ZIP. `Ctrl/Cmd + K` abre a paleta de comando.

---

## Onde ficam os arquivos

Cada projeto gera uma pasta em `public/generated/[slug]`:

```txt
public/generated/[slug]/
├─ catalog.json              # dados estruturados (contrato em lib/types.ts)
├─ case-draft.mdx            # rascunho de case (quando gerado)
├─ screenshots/
│  ├─ desktop-1440x900.png
│  ├─ desktop-fullpage.png
│  ├─ mobile-390x844.png
│  └─ mobile-fullpage.png
└─ thumbnails/
   ├─ thumb-main.webp
   ├─ thumb-mobile.webp
   └─ cover.webp             # capa, mockups e composições, quando gerados
```

O conteúdo de `public/generated/` **não é versionado** (só a pasta, via `.gitkeep`).
Caminhos no `catalog.json` e na UI são sempre públicos (`/generated/...`), nunca absolutos.

---

## Configuração

Todas as constantes de captura (viewports, delays, timeouts, formatos de composição,
mockups, etc.) vivem em [`lib/config.ts`](lib/config.ts) — nada hardcoded no meio do código.
Variáveis de ambiente sobrescrevem os defaults:

| Variável | Default | Descrição |
|---|---|---|
| `ATLAS_OUTPUT_DIR` | `public/generated` | Pasta de saída dos catálogos. |
| `ATLAS_HEADLESS` | `true` | Roda o Chromium sem janela. `false` para ver o navegador. |
| `ATLAS_NAV_TIMEOUT_MS` | `30000` | Timeout de navegação (`page.goto`). |
| `ATLAS_CAPTURE_DELAY_MS` | `3000` | Espera para animações/fontes antes do print. |
| `ATLAS_CAPTURE_VIDEO` | `true` | Default global de gravação de vídeo. |
| `ATLAS_CAPTURE_SECTIONS` | `true` | Default global de captura de seções. |

(Há mais ajustes finos em `lib/config.ts` — seções, capa, composições, mockups, etc.)

---

## Estrutura do projeto

```txt
app/
├─ page.tsx                  # landing interna
├─ generate/page.tsx         # formulário + progresso (Client)
├─ projects/page.tsx         # biblioteca de catálogos
├─ projects/[slug]/page.tsx  # catálogo (lê catalog.json — Server)
├─ lab/coded-atlas/page.tsx  # vitrine do experimento no Laboratório
└─ api/                      # generate (SSE), zip, case, export, diff, projects
components/                  # UI (url-input, generation-status, galerias, mockups…)
lib/
├─ types.ts                  # contratos — FONTE ÚNICA de verdade
├─ config.ts                 # constantes centrais
├─ errors.ts                 # AtlasError
├─ capture/                  # engine: pipeline, devices, seções, mockups, case…
├─ storage/                  # paths, pastas, escrita de JSON
├─ validation/               # URL, input, slug
├─ diff/                     # diff visual de recaptura
└─ mockup/                   # mockups 3D
docs/                        # product · design · architecture · BUILD-PLAN · ROADMAP
scripts/                     # testes por fase (npx tsx scripts/test-*.ts)
```

---

## Testes

Os testes são scripts diretos por fase/feature (sem framework). Rode com `tsx`:

```bash
npx tsx scripts/test-phase1.ts        # validação
npx tsx scripts/test-reprocess.ts     # reprocessamento preserva o case
npx tsx scripts/test-capture-options.ts
# ... demais scripts/test-*.ts
```

---

## Guard rails

- A rota de captura usa `runtime = "nodejs"` — **nunca** Edge.
- Playwright e `fs` só no servidor — nunca em Client Component.
- O navegador é sempre fechado em `finally`, inclusive em erro.
- Nenhum caminho absoluto no `catalog.json` nem na UI.
- `lib/types.ts` é a fonte única de tipos — nada diverge dela.

---

Coded Atlas · **Coded by M**

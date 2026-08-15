# Coded Atlas — Especificação Técnica (architecture.md)

# Coded by M — Coded Atlas (Especificação Técnica)

## Propósito deste documento

Este documento é a **especificação técnica** do Coded Atlas. Ele complementa:

- `product.md` → produto, escopo, MVP e roadmap (o "o quê" e o "porquê").
- `design.md` → linguagem visual, fluxos e estados de interface (a experiência).

Enquanto `product.md` e `design.md` definem **o que** construir e **como deve parecer/funcionar**, este documento define **como** construir: contratos de tipos, formato da API, pipeline de captura, configuração do Playwright, tratamento de erro, ordem de implementação e critérios de aceite.

A regra é simples:

> Dúvida sobre **intenção/produto** → `product.md`.
> Dúvida sobre **experiência/visual** → `design.md`.
> Dúvida sobre **implementação/código** → este documento (`architecture.md`).

O objetivo é remover ambiguidade técnica antes da execução no Claude Code. Quanto menos o agente precisar "adivinhar", mais previsível e limpo será o resultado.

---

## Princípios de Arquitetura

Estas decisões guiam todo o código. O Claude Code deve respeitá-las em qualquer arquivo gerado.

1. **Pipeline antes de interface.** A engine de captura é o coração. Ela deve funcionar isolada da UI (testável por script), e a UI apenas a consome.
2. **Tudo tipado.** Nenhum `any` solto. Os contratos da seção "Contratos de Tipos" são a fonte única de verdade.
3. **Configuração centralizada.** Viewports, delays, timeouts e caminhos vivem em `lib/config.ts`, nunca espalhados pelo código.
4. **Passos nomeados.** A captura é uma sequência de passos com nome e status. Isso alimenta o progresso na tela e o tratamento de erro.
5. **Erros explícitos, nunca silenciosos.** Toda falha vira um `AtlasError` com código conhecido, mapeado para mensagem e status HTTP.
6. **Sempre fechar o navegador.** Todo recurso do Playwright é liberado em `finally`, mesmo em erro.
7. **Local primeiro.** MVP roda 100% local (Node runtime + filesystem). Nada de Edge runtime, nada de banco, nada de cloud no 0.1.
8. **Modular e evolutivo.** Cada versão futura (vídeo, seções, case) deve encaixar sem reescrever o núcleo.

---

## Visão Geral da Arquitetura

```txt
┌───────────────────────────────────────────────────────────┐
│  FRONTEND (Next.js App Router)                            │
│                                                           │
│  /              → landing interna (Server Component)      │
│  /generate      → formulário + progresso (Client)         │
│  /projects/[slug] → leitura do catalog.json (Server)      │
└───────────────────────────────┬───────────────────────────┘
                                │  POST (stream de progresso)
                                ▼
┌───────────────────────────────────────────────────────────┐
│  API ROUTE  /api/generate  (runtime = 'nodejs')           │
│  - valida entrada                                         │
│  - chama a engine de captura                              │
│  - transmite progresso (SSE) e devolve catalog final      │
└───────────────────────────────┬───────────────────────────┘
                                ▼
┌───────────────────────────────────────────────────────────┐
│  ENGINE DE CAPTURA  (lib/capture)                         │
│  runCapturePipeline(input, onProgress)                    │
│   1. abrir navegador      5. fullpage mobile              │
│   2. desktop viewport     6. thumbnails (Sharp)           │
│   3. mobile viewport      7. montar catalog.json          │
│   4. fullpage desktop     8. fechar navegador             │
└───────────────────────────────┬───────────────────────────┘
                                ▼
┌───────────────────────────────────────────────────────────┐
│  STORAGE  (lib/storage)  → filesystem local               │
│  public/generated/[slug]/{screenshots,thumbnails}/        │
│  public/generated/[slug]/catalog.json                     │
└───────────────────────────────────────────────────────────┘
```

Camadas e responsabilidades:

| Camada | Pasta | Responsabilidade | Conhece o quê |
|---|---|---|---|
| UI | `app/`, `components/` | renderizar, coletar input, exibir progresso/resultado | tipos + endpoints |
| API | `app/api/generate/route.ts` | validar, orquestrar, transmitir progresso | engine + tipos |
| Engine | `lib/capture/` | abrir browser, capturar, gerar catálogo | storage + config + tipos |
| Storage | `lib/storage/` | criar pastas, escrever arquivos/JSON | filesystem |
| Config | `lib/config.ts` | constantes de captura e caminhos | nada |
| Validação | `lib/validation/` | validar URL e entrada | tipos |

Regra de dependência: **as setas só apontam para baixo**. A UI nunca importa Playwright; a engine nunca importa React.

---

## Decisões Técnicas-Chave

Cada decisão abaixo tem justificativa porque o Claude Code precisa entender o *porquê* para não "corrigir" no caminho errado.

### D1 — Runtime Node, nunca Edge

A rota `/api/generate` **deve** declarar:

```ts
export const runtime = "nodejs";
export const maxDuration = 120; // segundos; captura pode demorar
```

Motivo: Playwright e `fs` não existem no Edge runtime. Sem isso, o build quebra de forma confusa.

### D2 — Progresso via streaming (SSE), não polling

A captura leva ~10–30s e passa por etapas que a UX premium quer mostrar ("capturando desktop", "capturando mobile"...). Há três caminhos:

- **Resposta síncrona simples** → fácil, mas a UI só mostra "carregando" genérico (estados falsos).
- **Job + polling** → exige guardar estado de job (Map em memória) e endpoint extra. Overkill para uso local de 1 usuário.
- **Streaming SSE na própria resposta do POST** ✅ → estados **reais**, sem infra extra, sem risco de timeout.

**Decisão:** `POST /api/generate` responde com um stream de eventos de progresso e termina com o evento `result`. O cliente lê via `fetch` + `ReadableStream` (não `EventSource`, pois precisamos de POST com corpo).

> Fallback aceitável se o streaming travar a implementação inicial: resposta JSON síncrona com a UI animando os passos de forma otimista. Mas o stream é o alvo, porque entrega os estados que o produto pede.

### D3 — Viewport manual, não device emulation

Capturar com viewport fixo (`{ width, height }`) dá resultado **determinístico** e idêntico entre execuções. `playwright.devices['iPhone 13']` muda conforme a versão do Playwright. Usamos viewport manual + `deviceScaleFactor` controlado.

### D4 — Full page exige scroll prévio

Sites com lazy-loading só carregam imagens quando entram na viewport. Antes de `screenshot({ fullPage: true })`, a engine **rola até o fim**, espera, e volta ao topo. Sem isso, a captura full page sai com imagens em branco.

### D5 — Slug é a chave; colisão é tratada, não ignorada

A pasta de saída é `public/generated/[slug]`. Se já existir, o comportamento é definido por config (`OVERWRITE` por padrão no MVP, com aviso). Nunca falhar de forma silenciosa nem misturar assets de execuções diferentes.

### D6 — Sharp gera thumbnails, não captura

Thumbnails saem do *redimensionamento* do screenshot (não de uma nova captura). Mais rápido, consistente, e desacopla "capturar" de "preparar para portfólio".

---

## Contratos de Tipos (fonte única de verdade)

Estes tipos vivem em `lib/types.ts` e são importados por toda a aplicação. **Nenhuma estrutura de dados pode divergir disto.**

```ts
// lib/types.ts

/** Entrada do formulário / corpo da requisição. */
export interface ProjectInput {
  url: string;
  name: string;
  slug: string;
  category: string;
  client?: string;
  description?: string;
}

/** Configuração de um viewport de captura. */
export interface ViewportConfig {
  label: string;       // "desktop" | "mobile"
  width: number;
  height: number;
  deviceScaleFactor: number;
}

/** Resultado das capturas de um único device. */
export interface DeviceCapture {
  viewport: string;          // "1440x900"
  screenshot: string;        // caminho público, ex: /generated/slug/screenshots/desktop-1440x900.png
  fullpage: string;          // caminho público
}

/** Estrutura final persistida em catalog.json. */
export interface Catalog {
  version: string;           // versão do Atlas que gerou (ex: "0.1.0")
  project: ProjectInput;
  captures: {
    desktop: DeviceCapture;
    mobile: DeviceCapture;
  };
  thumbnails: {
    main: string;            // caminho público
    mobile: string;          // caminho público
  };
  videos?: {                 // reservado para v0.2; ausente no MVP
    desktop?: string;
    mobile?: string;
  };
  sections: CatalogSection[]; // vazio no MVP; reservado para v0.3
  meta: CatalogMeta;
  createdAt: string;         // new Date().toISOString()
}

/** Reservado para v0.3 — detecção de seções. Vazio no MVP. */
export interface CatalogSection {
  name: string;
  y: number;
  height: number;
  heading?: string;
}

/** Metadados da execução — útil para debug e reprocessamento. */
export interface CatalogMeta {
  captureDelayMs: number;
  navTimeoutMs: number;
  durationMs: number;        // tempo total da geração
  userAgent: string;
}

/** Passos do pipeline — alimentam o progresso na UI. */
export type CaptureStep =
  | "validating"
  | "launching"
  | "capturing-desktop"
  | "capturing-mobile"
  | "capturing-fullpage-desktop"
  | "capturing-fullpage-mobile"
  | "generating-thumbnails"
  | "writing-catalog"
  | "done"
  | "error";

/** Evento de progresso emitido pela engine e transmitido pela API. */
export interface ProgressEvent {
  step: CaptureStep;
  message: string;           // texto pronto para exibir (PT-BR)
  progress: number;          // 0–100
}

/** Evento final de sucesso. */
export interface ResultEvent {
  step: "done";
  catalog: Catalog;
  projectUrl: string;        // /projects/[slug]
}

/** Erro estruturado da aplicação. */
export type AtlasErrorCode =
  | "INVALID_URL"
  | "UNREACHABLE"
  | "NAV_TIMEOUT"
  | "BLOCKED"
  | "RENDER_TIMEOUT"
  | "CAPTURE_FAILED"
  | "STORAGE_FAILED"
  | "SLUG_CONFLICT"
  | "UNKNOWN";

export interface AtlasErrorPayload {
  step: "error";
  code: AtlasErrorCode;
  message: string;           // mensagem amigável em PT-BR
  detail?: string;           // detalhe técnico para log (não exibir cru ao usuário)
}
```

```ts
// lib/errors.ts — classe de erro usada em toda a engine
import type { AtlasErrorCode } from "./types";

export class AtlasError extends Error {
  constructor(
    public code: AtlasErrorCode,
    public userMessage: string,
    public detail?: string
  ) {
    super(detail ?? userMessage);
    this.name = "AtlasError";
  }
}
```

---

## Contrato da API

### `POST /api/generate`

**Request body** (`application/json`): um `ProjectInput`.

```json
{
  "url": "https://machadoplataformas.com.br",
  "name": "Machado Plataformas",
  "slug": "machado-plataformas",
  "category": "Site Institucional",
  "client": "Machado Plataformas",
  "description": "Site institucional premium."
}
```

**Response (modo stream — recomendado):**
`Content-Type: text/event-stream`. Cada linha de evento segue o formato SSE:

```txt
data: {"step":"launching","message":"Abrindo navegador...","progress":10}

data: {"step":"capturing-desktop","message":"Capturando desktop...","progress":30}

data: {"step":"capturing-mobile","message":"Capturando mobile...","progress":50}

data: {"step":"done","catalog":{...},"projectUrl":"/projects/machado-plataformas"}
```

Em caso de falha, o último evento é um `AtlasErrorPayload`:

```txt
data: {"step":"error","code":"NAV_TIMEOUT","message":"O site demorou demais para responder."}
```

**Response (modo síncrono — fallback):** `application/json` com `ResultEvent` em sucesso, ou `AtlasErrorPayload` com o status HTTP da tabela de erros.

**Headers obrigatórios do stream:**

```txt
Content-Type: text/event-stream
Cache-Control: no-cache, no-transform
Connection: keep-alive
```

**Consumo no cliente (referência):**

```ts
const res = await fetch("/api/generate", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(input),
});

const reader = res.body!.getReader();
const decoder = new TextDecoder();
let buffer = "";

while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  buffer += decoder.decode(value, { stream: true });
  const lines = buffer.split("\n\n");
  buffer = lines.pop() ?? "";
  for (const line of lines) {
    if (!line.startsWith("data: ")) continue;
    const event = JSON.parse(line.slice(6));
    // atualizar estado da UI conforme event.step
  }
}
```

---

## Pipeline de Captura (a engine)

Função pública única, em `lib/capture/capture-project.ts`:

```ts
export async function runCapturePipeline(
  input: ProjectInput,
  onProgress: (event: ProgressEvent) => void
): Promise<Catalog>
```

Responsabilidades, na ordem:

```txt
1. validating            → validar input + URL (lib/validation)
2. launching             → ensureProjectFolder + browser.launch
3. capturing-desktop     → viewport 1440x900 (screenshot)
4. capturing-mobile      → viewport 390x844 (screenshot)
5. capturing-fullpage-*  → scroll-to-bottom, espera, screenshot fullPage (desktop e mobile)
6. generating-thumbnails → Sharp redimensiona screenshots
7. writing-catalog       → monta Catalog + grava catalog.json
8. done                  → retorna Catalog
```

Regras da engine:

- Cada passo chama `onProgress(...)` **antes** de começar o trabalho pesado.
- Um único `browser` é aberto; cada device usa um `context` próprio (isolamento de cookies/cache).
- Qualquer falha vira `AtlasError` com o código apropriado.
- `browser.close()` **sempre** em `finally`.
- A engine **não** sabe o que é HTTP nem React — ela só recebe input e emite progresso por callback. Quem traduz isso em SSE é a API route.

Esqueleto de orquestração:

```ts
export async function runCapturePipeline(input, onProgress) {
  const startedAt = Date.now();
  let browser: Browser | undefined;
  try {
    onProgress({ step: "validating", message: "Validando URL...", progress: 5 });
    validateProjectInput(input); // lança AtlasError("INVALID_URL", ...)

    onProgress({ step: "launching", message: "Abrindo navegador...", progress: 10 });
    await ensureProjectFolder(input.slug);
    browser = await chromium.launch({ headless: config.headless });

    const desktop = await captureDevice(browser, input, config.viewports.desktop, onProgress);
    const mobile  = await captureDevice(browser, input, config.viewports.mobile,  onProgress);

    onProgress({ step: "generating-thumbnails", message: "Gerando thumbnails...", progress: 80 });
    const thumbnails = await generateThumbnails(input.slug, desktop, mobile);

    onProgress({ step: "writing-catalog", message: "Montando catálogo...", progress: 92 });
    const catalog = buildCatalog(input, { desktop, mobile }, thumbnails, startedAt);
    await writeJson(catalogPath(input.slug), catalog);

    onProgress({ step: "done", message: "Catálogo gerado.", progress: 100 });
    return catalog;
  } finally {
    await browser?.close();
  }
}
```

---

## Especificações do Playwright

Detalhes que costumam quebrar implementações e que o Claude Code precisa ter explícitos.

### Instalação

```bash
npm i -D playwright
npx playwright install chromium
```

Adicionar ao `package.json` para reinstalar o browser quando necessário:

```json
"scripts": {
  "postinstall": "playwright install chromium"
}
```

### Launch

```ts
const browser = await chromium.launch({
  headless: config.headless,        // true por padrão
  args: ["--no-sandbox", "--disable-dev-shm-usage"], // estabilidade em Docker/CI
});
```

### Contexto e navegação

```ts
const context = await browser.newContext({
  viewport: { width: vp.width, height: vp.height },
  deviceScaleFactor: vp.deviceScaleFactor,
  userAgent: config.userAgent,      // UA realista evita bloqueio simples
});
const page = await context.newPage();

await page.goto(input.url, {
  waitUntil: "networkidle",
  timeout: config.navTimeoutMs,     // ex: 30000
});
await page.waitForTimeout(config.captureDelayMs); // 2000–4000ms para animações/fontes
```

### Screenshot de viewport

```ts
await page.screenshot({ path: viewportShotPath, type: "png" });
```

### Screenshot full page (com scroll prévio — D4)

```ts
// força lazy-load antes do fullPage
await page.evaluate(async () => {
  await new Promise<void>((resolve) => {
    let y = 0;
    const step = 600;
    const timer = setInterval(() => {
      window.scrollBy(0, step);
      y += step;
      if (y >= document.body.scrollHeight) {
        clearInterval(timer);
        window.scrollTo(0, 0);
        resolve();
      }
    }, 120);
  });
});
await page.waitForTimeout(500);
await page.screenshot({ path: fullPagePath, fullPage: true, type: "png" });
```

### Limpeza

```ts
await context.close(); // sempre; libera memória e finaliza vídeo (quando v0.2)
```

### Mapeamento de falhas do Playwright → `AtlasError`

| Sintoma | Código |
|---|---|
| URL inválida na validação | `INVALID_URL` |
| `net::ERR_NAME_NOT_RESOLVED` / connection refused | `UNREACHABLE` |
| `TimeoutError` no `goto` | `NAV_TIMEOUT` |
| status 403 / desafio anti-bot / página vazia | `BLOCKED` |
| `waitForTimeout`/render nunca estabiliza | `RENDER_TIMEOUT` |
| erro em `screenshot()` | `CAPTURE_FAILED` |
| erro de `fs` ao salvar | `STORAGE_FAILED` |

### Reservado para v0.2 (vídeo) — não implementar no MVP

Vídeo é configurado **no contexto**, e o arquivo só é finalizado ao fechar o contexto:

```ts
const context = await browser.newContext({
  viewport, recordVideo: { dir, size: { width, height } },
});
// ... navegar e rolar ...
await context.close();
const videoPath = await page.video()?.path(); // disponível após close
```

A arquitetura do MVP já deve deixar `videos?` opcional no `Catalog` e um passo "gravando vídeo" previsto no enum, mesmo sem implementar.

---

## Processamento de Imagem (Sharp)

```bash
npm i sharp
```

Thumbnails saem do screenshot de viewport (não do full page), em WebP para peso menor:

```ts
import sharp from "sharp";

export async function generateThumbnails(slug, desktop, desktopShotAbsPath, mobileShotAbsPath) {
  await sharp(desktopShotAbsPath)
    .resize(640, 400, { fit: "cover", position: "top" })
    .webp({ quality: 82 })
    .toFile(thumbMainAbsPath);

  await sharp(mobileShotAbsPath)
    .resize(320, 640, { fit: "cover", position: "top" })
    .webp({ quality: 82 })
    .toFile(thumbMobileAbsPath);

  return { main: thumbMainPublic, mobile: thumbMobilePublic };
}
```

---

## Storage e Convenção de Arquivos

Toda escrita passa por `lib/storage`. Nada de `fs` espalhado pela engine.

```ts
// lib/storage/paths.ts — centraliza caminhos absolutos x públicos
export const projectDir   = (slug) => path.join(config.outputDir, slug);
export const screenshotDir = (slug) => path.join(projectDir(slug), "screenshots");
export const thumbnailDir  = (slug) => path.join(projectDir(slug), "thumbnails");
export const catalogPath   = (slug) => path.join(projectDir(slug), "catalog.json");

// caminho público (servido pelo Next a partir de /public)
export const publicPath = (slug, ...parts) =>
  "/" + path.posix.join("generated", slug, ...parts);
```

Estrutura de saída (MVP — sem vídeo, sem case ainda):

```txt
public/generated/[slug]/
├─ catalog.json
├─ screenshots/
│  ├─ desktop-1440x900.png
│  ├─ desktop-fullpage.png
│  ├─ mobile-390x844.png
│  └─ mobile-fullpage.png
└─ thumbnails/
   ├─ thumb-main.webp
   └─ thumb-mobile.webp
```

Regras:

- `ensureProjectFolder(slug)` cria as subpastas com `fs.mkdir(..., { recursive: true })`.
- Colisão de slug: por padrão `OVERWRITE` (config), emitindo aviso no log. Nunca mesclar execuções.
- Caminhos **públicos** (os que vão no `catalog.json` e na UI) sempre começam com `/generated/...`. Caminhos **absolutos** (os que o `fs`/Sharp usam) nunca aparecem no JSON nem na UI.

---

## Configuração Central e Variáveis de Ambiente

```ts
// lib/config.ts
import path from "node:path";

export const config = {
  outputDir: process.env.ATLAS_OUTPUT_DIR
    ? path.resolve(process.env.ATLAS_OUTPUT_DIR)
    : path.join(process.cwd(), "public", "generated"),

  headless: process.env.ATLAS_HEADLESS !== "false",
  navTimeoutMs: Number(process.env.ATLAS_NAV_TIMEOUT_MS ?? 30000),
  captureDelayMs: Number(process.env.ATLAS_CAPTURE_DELAY_MS ?? 3000),

  userAgent:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 " +
    "(KHTML, like Gecko) Chrome/124.0 Safari/537.36",

  viewports: {
    desktop: { label: "desktop", width: 1440, height: 900, deviceScaleFactor: 2 },
    mobile:  { label: "mobile",  width: 390,  height: 844, deviceScaleFactor: 3 },
  },

  onSlugConflict: "overwrite" as "overwrite" | "fail",
  atlasVersion: "0.1.0",
} as const;
```

`.env.example` (commitar este; nunca o `.env` real):

```txt
ATLAS_OUTPUT_DIR=
ATLAS_HEADLESS=true
ATLAS_NAV_TIMEOUT_MS=30000
ATLAS_CAPTURE_DELAY_MS=3000
```

---

## Fronteiras Server / Client (App Router)

| Arquivo | Tipo | Por quê |
|---|---|---|
| `app/page.tsx` | Server | conteúdo estático, sem interação |
| `app/generate/page.tsx` | **Client** (`"use client"`) | tem formulário, estado e consumo de stream |
| `app/projects/[slug]/page.tsx` | Server | lê `catalog.json` do filesystem com `fs` |
| `app/api/generate/route.ts` | Route Handler (`runtime="nodejs"`) | usa Playwright + `fs` |
| `components/url-input.tsx` | Client | controla campos do form |
| `components/generation-status.tsx` | Client | reage a `ProgressEvent` |
| `components/generated-gallery.tsx` | Server ou Client | só exibe; preferir Server |
| `components/device-frame.tsx` | Server | apresentação pura |

Regra: **componente que tem `useState`/`onClick` é Client. Componente que lê filesystem é Server.** A página de projeto lê o JSON assim:

```ts
// app/projects/[slug]/page.tsx (Server Component)
import { promises as fs } from "node:fs";
const raw = await fs.readFile(catalogPath(params.slug), "utf-8");
const catalog: Catalog = JSON.parse(raw);
```

---

## Taxonomia de Erros

Tabela única que liga código técnico → status HTTP → mensagem ao usuário (PT-BR). A UI mostra `message`; o `detail` vai só para o log.

| Código | HTTP | Mensagem ao usuário |
|---|---|---|
| `INVALID_URL` | 400 | A URL informada não é válida. Verifique e tente de novo. |
| `UNREACHABLE` | 502 | Não foi possível acessar este site. Confira se ele está no ar. |
| `NAV_TIMEOUT` | 504 | O site demorou demais para responder. Tente aumentar o tempo de espera. |
| `BLOCKED` | 403 | Este site bloqueou a captura automática. Tente em modo local. |
| `RENDER_TIMEOUT` | 504 | A página não terminou de carregar a tempo. |
| `CAPTURE_FAILED` | 500 | Falha ao capturar a página. Tente novamente. |
| `STORAGE_FAILED` | 500 | Não foi possível salvar os arquivos gerados. |
| `SLUG_CONFLICT` | 409 | Já existe um projeto com este slug. |
| `UNKNOWN` | 500 | Algo deu errado ao gerar o catálogo. |

---

## Estrutura de Pastas (refinada)

Evolução da estrutura proposta no `product.md`, com os arquivos técnicos que faltavam:

```txt
coded-atlas/
├─ app/
│  ├─ page.tsx
│  ├─ generate/page.tsx
│  ├─ projects/[slug]/page.tsx
│  └─ api/generate/route.ts
│
├─ components/
│  ├─ url-input.tsx
│  ├─ generation-status.tsx
│  ├─ generated-gallery.tsx
│  ├─ device-frame.tsx
│  ├─ project-catalog-card.tsx
│  └─ asset-downloads.tsx
│
├─ lib/
│  ├─ types.ts                      ← contratos (fonte única)
│  ├─ config.ts                     ← constantes centrais
│  ├─ errors.ts                     ← classe AtlasError
│  ├─ capture/
│  │  ├─ capture-project.ts         ← runCapturePipeline (orquestra)
│  │  ├─ capture-device.ts          ← screenshot viewport + fullpage de um device
│  │  ├─ scroll-to-bottom.ts        ← helper lazy-load
│  │  ├─ generate-thumbnails.ts     ← Sharp
│  │  ├─ build-catalog.ts           ← monta o objeto Catalog
│  │  └─ record-scroll.ts           ← reservado v0.2 (vídeo)
│  ├─ storage/
│  │  ├─ paths.ts
│  │  ├─ ensure-project-folder.ts
│  │  └─ write-json.ts
│  └─ validation/
│     ├─ validate-url.ts
│     └─ validate-project-input.ts
│
├─ public/generated/               ← saída (gitignore no conteúdo)
├─ docs/
│  ├─ product.md
│  ├─ design.md
│  ├─ architecture.md
│  └─ BUILD-PLAN.md
├─ .env.example
├─ .gitignore                      ← inclui /public/generated/*
└─ package.json
```

---

## Estratégia de Logging

- Engine usa um logger simples (`console` no MVP) com prefixo `[atlas]` e o slug.
- Logar **início e fim de cada passo** com duração. Isso vira a base do `meta.durationMs`.
- Em erro: logar `code` + `detail` técnico; ao usuário, só a `message`.
- Nunca logar a página inteira nem dados sensíveis.

---

## Setup e Comandos (essencial do README)

```bash
# 1. criar projeto
npx create-next-app@latest coded-atlas --ts --tailwind --app --eslint

cd coded-atlas

# 2. dependências de captura
npm i playwright sharp
npx playwright install chromium

# 3. variáveis
cp .env.example .env

# 4. rodar
npm run dev
```

O README deve conter: o que é o Atlas (1 parágrafo, linkando o `product.md`), pré-requisitos (Node 18+), os comandos acima, a estrutura de pastas, como gerar um catálogo, e onde ficam os arquivos de saída.

---

## Definition of Done — MVP (v0.1)

O MVP está pronto quando **todos** os itens abaixo passam:

- [ ] `npm run dev` sobe sem erro de runtime/build.
- [ ] `/generate` valida a URL antes de enviar (URL malformada nunca chega na engine).
- [ ] Uma URL real gera os 4 screenshots (desktop, mobile, fullpage desktop, fullpage mobile).
- [ ] Os 2 thumbnails WebP são gerados.
- [ ] `catalog.json` é escrito e bate **exatamente** com a interface `Catalog`.
- [ ] Todos os caminhos no JSON são públicos (`/generated/...`), nenhum absoluto.
- [ ] A UI exibe progresso por etapas reais durante a captura.
- [ ] `/projects/[slug]` lê o JSON e exibe dados + galerias separadas (desktop / mobile).
- [ ] Uma URL inacessível mostra mensagem amigável (não stack trace), com o código correto.
- [ ] O navegador é fechado mesmo quando há erro (sem processo Chromium pendurado).
- [ ] Visual escuro, técnico, alinhado à Coded by M — sem cara de SaaS genérico.
- [ ] README permite que outra pessoa rode do zero.

---

## Ordem de Implementação (build order para o Claude Code)

Construir e **verificar** nesta ordem. Cada fase é testável isoladamente — isso é o que mais aumenta a taxa de acerto do agente.

```txt
Fase 0  Scaffold + lib/types.ts + lib/config.ts + lib/errors.ts
Fase 1  Validação (validate-url, validate-project-input) + slugify
Fase 2  Storage (paths, ensure-project-folder, write-json)
Fase 3  Engine: capturar SÓ desktop viewport, testado por um script CLI
Fase 4  Engine: adicionar mobile + fullpage (desktop e mobile)
Fase 5  Sharp: thumbnails
Fase 6  build-catalog + escrever catalog.json (validar contra o tipo)
Fase 7  API route síncrona (sem stream) devolvendo o Catalog
Fase 8  Converter a API para stream SSE de progresso
Fase 9  UI: /generate (form + consumo do stream + estados)
Fase 10 UI: /projects/[slug] (leitura do JSON + galerias)
Fase 11 Landing / + polish visual premium
```

Regra de ouro: **não avançar de fase enquanto a anterior não rodar.** A Fase 3 deve produzir um PNG real no disco antes de existir qualquer tela.

---

## Prompt Atualizado para o Claude Code

Use quando os três documentos (`product.md`, `design.md`, `architecture.md`) estiverem em `docs/`.

```txt
Leia docs/product.md, docs/design.md e docs/architecture.md por completo ANTES de
escrever qualquer código. Onde houver conflito técnico, o architecture.md prevalece.

Implemente o MVP (v0.1) do Coded Atlas seguindo EXATAMENTE:
- os contratos de tipo da seção "Contratos de Tipos" (lib/types.ts é a fonte única);
- o contrato de API da seção "Contrato da API";
- as especificações de Playwright e Sharp;
- a taxonomia de erros;
- a estrutura de pastas refinada.

Construa na ordem da seção "Ordem de Implementação", do Fase 0 ao Fase 11.
NÃO pule fases. Ao terminar cada fase, descreva o que foi feito e como verificar,
e só então siga para a próxima.

Stack obrigatória: Next.js (App Router) + TypeScript + Tailwind + Playwright + Sharp.

Restrições (NÃO faça):
- nada de banco de dados, login, multiusuário ou pagamento;
- nada de Edge runtime (a rota usa runtime = "nodejs");
- nada de vídeo agora — apenas deixe os campos opcionais e o passo previsto;
- nenhum caminho absoluto dentro do catalog.json ou da UI;
- nenhum `any` solto;
- nada com aparência de SaaS genérico — visual escuro/técnico da Coded by M.

Critérios de aceite: todos os itens da seção "Definition of Done — MVP".

Antes de codar, explique em poucas linhas sua leitura dos dois documentos e o plano por fase.
```

---

## Guard Rails (o que o agente NÃO deve fazer)

Lista curta de violações comuns. Repetida no prompt de propósito.

1. Não usar Edge runtime na rota de captura.
2. Não colocar caminho absoluto no `catalog.json` nem na UI.
3. Não chamar Playwright dentro de Client Component.
4. Não ler filesystem dentro de Client Component.
5. Não deixar o navegador aberto em caminho de erro.
6. Não inventar campos no `Catalog` fora do tipo definido.
7. Não adicionar banco, auth ou cloud "para já deixar pronto".
8. Não transformar mensagens de erro técnicas em texto exibido cru ao usuário.
9. Não hardcodar viewport/delay no meio do código — usar `lib/config.ts`.
10. Não pular fases da ordem de implementação.

---

## Resumo

Este documento dá ao Coded Atlas o que faltava para uma execução limpa no Claude Code:

- **contratos de tipo** como fonte única;
- **contrato de API** com progresso real via stream;
- **especificação do Playwright** com as armadilhas resolvidas (fullpage, timeout, cleanup);
- **taxonomia de erros** mapeada;
- **ordem de construção em fases** verificáveis;
- **critérios de aceite** objetivos.

Com `product.md` (produto), `design.md` (experiência) e `architecture.md` (engenharia) juntos em `docs/`, o agente tem intenção, experiência e especificação — e improvisa muito menos.

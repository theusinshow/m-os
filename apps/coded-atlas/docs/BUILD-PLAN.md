# BUILD-PLAN — Coded Atlas (MVP v0.1)

Checklist **vivo** da construção. O Claude Code deve marcar cada fase como concluída
(`[x]`) e escrever uma nota de 1 linha ao terminá-la. Detalhes de cada fase estão na
seção "Ordem de Implementação" do `docs/architecture.md`.

**Status atual:** **Projeto fechado.** Todo o roadmap entregue (exceto v1.6, descartada). Software 100% determinístico, sem IA.
**Última fase concluída:** Mockups 3D (v1.4 item 12) + Diff visual (v1.7). `next build` ✓. Mockups 3D verificados por screenshot; diff verificado e2e (6.86% mudança, regiões destacadas).
**Sem frentes em aberto.** Detalhes em `docs/ROADMAP.md` (fechado).

---

## Fases

- [x] **Fase 0 — Fundação**
  Scaffold + `lib/types.ts` + `lib/config.ts` + `lib/errors.ts`.
  _Verifica:_ os tipos compilam e refletem exatamente os contratos do `architecture.md`.
  _Status:_ `npx tsc --noEmit` → zero erros. Chromium instalado via postinstall.

- [x] **Fase 1 — Validação**
  `validate-url`, `validate-project-input`, `slugify`.
  _Status:_ 19/19 casos passando via `npx tsx scripts/test-phase1.ts`. Zero erros de tipo.

- [x] **Fase 2 — Storage**
  `paths`, `ensure-project-folder`, `write-json`.
  _Status:_ 11/11 casos passando via `npx tsx scripts/test-phase2.ts`. Overwrite limpa a pasta (sem merge de execuções). `.gitignore` já configurado na Fase 0.

- [x] **Fase 3 — Engine: primeira captura** ⚠️ momento de verdade
  Capturar SÓ o screenshot desktop (viewport), testado por um script CLI.
  _Status:_ `desktop-1440x900.png` real gerado (36 KB). Browser fechado no `finally`. Guard rails de caminho público verificados.

- [x] **Fase 4 — Engine: capturas completas**
  Adicionar mobile + full page (desktop e mobile), com scroll-to-bottom prévio.
  _Status:_ 4 PNGs reais (35–50 KB cada). scroll-to-bottom com lazy-load. Eventos de progresso reais por passo. Browser fechado em finally.

- [x] **Fase 5 — Thumbnails (Sharp)**
  Gerar `thumb-main.webp` e `thumb-mobile.webp` a partir dos screenshots.
  _Status:_ 640×400 e 320×640 WebP quality-82, caminhos públicos verificados. Erros Sharp → AtlasError(STORAGE_FAILED).

- [x] **Fase 6 — Catálogo**
  `build-catalog` + escrever `catalog.json`.
  _Status:_ 17/17 verificações. Interface Catalog respeitada. absPath nunca no JSON. videos ausente. sections=[]. round-trip sem perda.

- [x] **Fase 7 — API síncrona**
  Rota `/api/generate` (runtime nodejs) que orquestra a engine e devolve o `Catalog`.
  _Status:_ POST real 12.6s → ResultEvent. URL inválida → 400 INVALID_URL amigável. `next build` OK. Browser fechado em finally.

- [x] **Fase 8 — API com stream (SSE)**
  Converter a rota para transmitir eventos de progresso por etapa.
  _Verifica:_ os eventos de progresso chegam na ordem certa até o `result`.
  _Status:_ `ReadableStream` SSE com 9 eventos reais. Progress monotônico. Erro retorna `{step:"error"}` no stream. OK do Matheus.

- [x] **Fase 9 — UI: /generate**
  Formulário + consumo do stream + estados de progresso reais.
  _Verifica:_ ao gerar, a tela mostra os passos de verdade, não animação falsa.
  _Status:_ `UrlInput` + `GenerationStatus` + page com máquina de estados. SSE consumer conforme architecture.md. OK do Matheus.

- [x] **Fase 10 — UI: /projects/[slug]**
  Server Component que lê o `catalog.json` e exibe galerias separadas (desktop/mobile).
  _Verifica:_ a página renderiza dados + screenshots a partir do JSON.
  _Status:_ Server Component com `fs.readFile().catch(() => notFound())`. Galerias desktop/mobile, DeviceFrame, AssetDownloads. OK do Matheus.

- [x] **Fase 11 — Landing + polish**
  Página `/` e acabamento visual premium (escuro/técnico Coded by M).
  _Verifica:_ a checklist completa "Definition of Done — MVP" do `architecture.md` passa.
  _Status:_ Landing com grid técnico, título mono, "Como funciona", estrutura de saída. `globals.css` com scrollbar e selection escuros. `next build` ✓.

---

- [x] **v0.4 — Gerador de case**
  `lib/capture/generate-case.ts` + `POST /api/case` + `CaseDraftSection`.
  _Verifica:_ clicar "Gerar rascunho" em `/projects/[slug]` → baixar `case-draft.mdx` com frontmatter, capturas e inspeção.
  _Status:_ `next build` ✓. Arquivo salvo em `public/generated/[slug]/case-draft.mdx`. Server detecta se já existe e pré-popula o estado.

---

## v1.0 — Ferramenta de portfólio completa

> Já entregue antes deste bloco (em `0b3b943` / `9b9a609` / `0e35809`): histórico em `/projects`, export ZIP (`/api/zip/[slug]`), toast de geração persistente.

- [x] **Reprocessar captures**
  Botão "Reprocessar capturas" em `/projects/[slug]` → `/generate?reprocess=<slug>`. A página relê o `catalog.json` público, extrai o `project` salvo e reinicia a geração com os mesmos dados (sem redigitar o form), reusando a máquina de estados/SSE/toast.
  `ensure-project-folder` passa a **preservar o `case-draft.mdx`** ao recriar a pasta (overwrite) — antes ele era apagado junto com as capturas.
  _Verifica:_ em um projeto existente, clicar "Reprocessar" regenera screenshots/vídeos/seções e mantém o rascunho de case.
  _Status:_ `next build` ✓ 8 rotas. `scripts/test-reprocess.ts` 7/7. Verificado end-to-end contra example.com.

- [x] **Integração com portfólio (manifesto)**
  `lib/capture/build-portfolio-manifest.ts` (função pura) + `GET /api/export/[slug]` + seção "Exportar para o portfólio" em `/projects/[slug]` (copiar JSON / baixar `portfolio.json`).
  Manifesto = fragmento que `/cases/[slug]` e a Paisagem Digital consomem: nome, slug, categoria, descrição, url, thumbnails, cover, acento (de `inspection.colors[0]`), paleta, techStack, `hasVideo`, data, versão. Sem dados novos — só projeta o Catalog. Caminhos públicos.
  _Status:_ `test-portfolio-manifest.ts` 16/16. E2E ✓.
  _Pendente (precisa de credenciais/repo):_ push automático via GitHub — hoje a publicação é manual assistida (baixar/copiar).

- [x] **Página pública no Laboratório**
  `/lab/coded-atlas` (Server Component): hero do experimento, problema, como funciona, capturas geradas (projeto real mais recente), vídeo de navegação, estrutura técnica, próximos passos, CTA. Link a partir da landing. Visual escuro/técnico Coded by M.
  _Status:_ `next build` ✓ (estática). E2E: HTTP 200 e renderiza o projeto featured.

- [ ] **Push automático para o GitHub do portfólio** — requer token/repo configurados; fora do escopo sem esse setup.

### Ajustes

- [x] **Aviso de lockfile** — `outputFileTracingRoot` fixado no `next.config.ts` (havia um `C:\Dev\package-lock.json` solto que o Next confundia como raiz). Build sem aviso.

---

## v1.0 — Revisão de UI/UX

Sistema visual comprometido (ver `design.md` › "Sistema visual (v1.0)"): tokens OKLCH em
`globals.css`, acento **cobre**, contraste de texto elevado, navegação global.

- [x] **Token layer + navegação global** — `globals.css` (acento cobre, superfícies, status, foco, `.bg-grid`, `.tri`) + `AppNav` persistente no `layout.tsx`.
- [x] **Home com histórico** — hero + "Projetos recentes" (6 mais novos) com empty-state que ensina o fluxo.
- [x] **Biblioteca de projetos** — `ProjectsLibrary`: busca (nome/cliente/URL/categoria), filtro por categoria, sort Recentes/A→Z, contagem; cards com badge, host e ação de reprocessar no hover.
- [x] **Formulário com menos digitação** — categoria como **dropdown** (`lib/categories.ts`, "Outro" → texto livre), nome auto-sugerido pelo domínio, slug automático, cliente/descrição recolhidos.
- [x] **Progresso explicado** — `GenerationStatus`: fases (Preparando/Desktop/Mobile/Finalizando), passo atual destacado com descrição do que está fazendo, barra cobre, mensagem ao vivo.
- [x] **Erros claros** — orientação prática por `AtlasErrorCode` + "Tentar novamente" / "Editar dados".
- [x] **Contraste e acento** aplicados em toast, galeria, downloads, device-frame, página de projeto, case-draft, portfolio-export e Laboratório.
  _Status:_ `next build` ✓ 10 rotas, zero warnings. Verificado por screenshots reais de todas as telas (home, biblioteca, formulário, progresso, projeto, lab).

- [x] **Gerenciar projetos** — excluir e cancelar.
  `lib/storage/delete-project.ts` (guarda contra path traversal, confinado a `outputDir`) + `DELETE /api/projects/[slug]` + `DeleteProject` (confirmação inline, "Zona de risco" na página do projeto). Botão **Cancelar** na tela de progresso (aborta o stream e limpa o toast).
  _Status:_ `next build` ✓ 11 rotas. `test-delete-project.ts` 10/10 (rejeita `..`, `a/b`, espaços, vazio; sentinela fora da pasta intacta). E2E: 404 em inexistente/ inválido, `{ok:true}` + pasta removida no real.

---

## v1.1–v1.3 — Roadmap pós-v1.0 (ver `docs/ROADMAP.md`)

- [x] **v1.1 — Lightbox das capturas** — `components/zoom-image.tsx` (tela cheia, zoom 1×/real, Esc/clique, scroll-lock) na galeria, capa e seções.
- [x] **v1.1 — Paleta de comando (Cmd/Ctrl+K)** — `command-palette.tsx` + `GET /api/projects`; busca projetos e ações, navegação por teclado, gatilho também por botão na nav.
- [x] **v1.2 — Seleção em lote na biblioteca** — modo de seleção em `ProjectsLibrary` + `ProjectCatalogCard` selecionável; barra de ações (excluir em lote com confirmação, baixar ZIPs, selecionar todos/limpar).
- [x] **v1.3 — Opções de captura por geração** — `CaptureOptions` em `lib/types.ts`; `captureDevice` resolve `input.options ?? config` (config vira default); toggles Vídeo/Seções no `UrlInput`; opções salvas no `catalog.json` e herdadas no reprocess.
  _Status:_ `next build` ✓ 12 rotas, zero warnings. `test-capture-options.ts` 7/7. E2E: gerar sem vídeo/seções não cria `videos/` nem `sections-*` nem a chave `videos`; reprocess herda; geração padrão segue com vídeo+seções. Verificado por screenshots (paleta, lightbox, seleção em lote, toggles).

---

## v1.5 — Captura mais rica (em andamento)

- [x] **Item 7 — Múltiplas páginas** — `ProjectInput.pages` + `lib/capture/capture-page.ts` (viewport + full page desktop/mobile por página); rota itera com dedupe e cap (`config.maxExtraPages`); passo `capturing-pages`; `catalog.pages`; seção "Outras páginas" + ZIP; reprocess herda. `buildCatalog` refatorado para objeto `extras`. `test-multipage` 8/8.
- [x] **Item 8 — Estados de interação** — `ProjectInput.states` (`{name, selector}`) + `lib/capture/capture-states.ts` (clica seletor, espera, fotografa desktop); passo `capturing-states`; `catalog.states`; campo no form (`Nome | seletor`); seção "Estados" + ZIP. `test-states` 6/6. E2E: clicou o botão de busca do próprio app e capturou a paleta aberta.
- [x] **Item 9 — Nomes de seção inteligentes** — `lib/capture/section-name.ts` (puro: `deriveSectionName` por palavra-chave classe/id/aria → "Hero/Sobre/Serviços...", senão heading > tag semântica > id significativo; `disambiguate` para repetidos; rejeita ids genéricos/hash). `detect-sections` refatorado: `page.evaluate` só extrai pistas, nomes aplicados no Node. `test-section-name` 21/21. E2E em fixture: 8 seções nomeadas corretamente.

---

## v1.4 item 12 + v1.7 — fechamento

- [x] **Mockups 3D em perspectiva** — `lib/mockup/render-3d.ts`: HTML+CSS 3D (device inclinado) fotografado pelo Playwright com fundo transparente; ângulos em `config.mockups3d`. Saem como `desktop-3d`/`mobile-3d` na mesma seção "Mockups" + ZIP. Verificado por screenshot.
- [x] **Diff visual de recaptura (v1.7)** — `lib/diff/visual-diff.ts` (Sharp decodifica RGBA, `pixelmatch` compara, Sharp grava o PNG de diff) + `POST /api/diff/[slug]` (recaptura o viewport desktop e compara com o do catálogo) + seção "Monitoramento" (`VisualDiff`) na página do projeto (antes/agora/diferença + % mudado). E2E: 6.86% num cenário com cards novos, regiões destacadas em coral.

---

## Regras de atualização

- Só marcar `[x]` depois que o critério "_Verifica_" passou de fato.
- Ao concluir uma fase, atualizar **Status atual** e **Última fase concluída** no topo.
- Não avançar de fase sem o OK do Matheus.

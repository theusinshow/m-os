# ROADMAP — Coded Atlas (pós-v1.0)

Plano vivo das próximas evoluções. Cada item é fechado e testável de forma isolada,
no mesmo espírito do `BUILD-PLAN.md`. Marcar `[x]` só quando o critério de aceite passar.

**Status:** **Roadmap concluído.** v1.1–v1.5, mockups 3D (item 12) e v1.7 (diff visual) feitos.
**v1.6 (case com IA) descartada** — decisão do Matheus: o software fica **sem IA**, 100% determinístico.
Não há frentes em aberto.

**Como usar:** atacar uma versão por vez, na ordem sugerida. Ao concluir um item,
mover o detalhe correspondente para o `BUILD-PLAN.md` (registro do que foi feito) e
marcar aqui. A ordem é uma recomendação — pode ser repriorizada.

**Legenda de esforço:** `P` ≈ uma sessão · `M` ≈ algumas sessões · `G` = grande / depende de coisa externa.

**Guard rails mantidos em tudo:** `lib/types.ts` é a fonte única; rota de captura em
`runtime = "nodejs"`; nenhum caminho absoluto no JSON/UI; navegador fechado em `finally`;
viewports/delays/timeouts só em `lib/config.ts`; nada de `any` solto; visual escuro/cobre
da Coded by M (ver `design.md` › "Sistema visual (v1.0)").

---

## v1.1 — Visualização e navegação rápida

> Puro frontend, sem dependências externas, alto valor no uso diário. Começar por aqui.

- [x] **1. Lightbox das capturas** `P`
  _Objetivo:_ ver screenshots/seções/capa em tela cheia, com zoom e navegação por teclado.
  _Escopo:_ componente `components/lightbox.tsx` (client) acionado por clique em qualquer
  imagem da galeria/seções/capa; overlay escuro, `Esc` fecha, `←/→` navega, foco preso no overlay.
  _Arquivos:_ `lightbox.tsx`, `generated-gallery.tsx`, `app/projects/[slug]/page.tsx`.
  _Aceite:_ clicar numa captura abre em tela cheia; teclado funciona; acessível (role=dialog,
  foco gerenciado); fecha sem vazar scroll-lock.

- [x] **2. Paleta de comando (Cmd/Ctrl+K)** `P`–`M`
  _Objetivo:_ buscar e abrir qualquer projeto e disparar ações sem mouse.
  _Escopo:_ `components/command-palette.tsx` montado no layout; índice de projetos via um
  endpoint leve `GET /api/projects` (lista o `ProjectSummary[]`); ações fixas (Gerar, Biblioteca,
  Laboratório) + resultados de projeto (abrir, reprocessar). Navegação por setas/Enter.
  _Arquivos:_ `command-palette.tsx`, `app/api/projects/route.ts`, `app/layout.tsx`.
  _Depende de:_ nada. (Reaproveita `listProjects`.)
  _Aceite:_ `Cmd/Ctrl+K` abre; digitar filtra; Enter abre o projeto; `Esc` fecha; funciona em
  qualquer página.

---

## v1.2 — Operações em lote

> Ganho de produtividade quando a biblioteca cresce. Reaproveita exclusão e ZIP existentes.

- [x] **3. Seleção múltipla na biblioteca** `M`
  _Objetivo:_ selecionar vários projetos e excluir / baixar ZIP / exportar manifests de uma vez.
  _Escopo:_ modo de seleção em `ProjectsLibrary` (checkbox por card, barra de ações fixa com
  contagem); excluir em lote chama `DELETE /api/projects/[slug]` em sequência com confirmação
  única; baixar em lote dispara os ZIPs (ou um endpoint `POST /api/zip` multi-slug — avaliar).
  _Arquivos:_ `projects-library.tsx`, `project-catalog-card.tsx` (variante selecionável),
  opcional `app/api/zip/route.ts`.
  _Depende de:_ exclusão (v1.0, pronto) e ZIP (pronto).
  _Aceite:_ selecionar N projetos, excluir todos com uma confirmação; contagem correta; estado
  limpa após a ação; nada é excluído sem confirmar.

---

## v1.3 — Controle de captura por geração

> Mais poder no formulário: decidir o que capturar em cada projeto, em vez de só via env.

- [x] **4. Opções de captura por geração** `M`
  _Objetivo:_ ligar/desligar **vídeo** e **seções** (e, se valer, escolher profundidade) por
  projeto, direto no formulário, sem mexer em variáveis de ambiente.
  _Escopo:_ estender `ProjectInput` com um `options?: { video?: boolean; sections?: boolean }`
  (opcional, em `lib/types.ts` — fonte única); a rota `/api/generate` passa essas opções para
  `captureDevice`, que hoje lê `config.captureVideo`/`config.captureSections` (passam a ser o
  **default**, sobrescrito pela opção do request); UI: toggles na seção "detalhes" do `UrlInput`.
  Persistir as opções no `catalog.json` (campo já coberto pelos opcionais) para o reprocess herdar.
  _Arquivos:_ `lib/types.ts`, `app/api/generate/route.ts`, `lib/capture/capture-device.ts`,
  `components/url-input.tsx`, `lib/capture/build-catalog.ts`.
  _Depende de:_ nada (a engine já tem os dois caminhos atrás de flags).
  _Risco:_ toca a fonte única de tipos e a engine — fazer com teste do contrato e do reprocess.
  _Aceite:_ gerar com vídeo desligado não cria `videos/` nem o campo `videos`; reprocessar herda
  as opções salvas; `catalog.json` continua batendo com `Catalog`.

---

## v1.4 — Saída pronta para apresentação

> Maior alavanca de portfólio: transforma screenshot crua em peça apresentável. Reaproveita o
> Sharp, que já está no pipeline (`generate-cover` / `generate-thumbnails`). Esforço baixo, valor alto.

- [ ] **5. Composições para redes sociais** `P`
  _Objetivo:_ gerar a captura (ou capa) centralizada sobre um fundo da marca, nos formatos prontos
  para postar: 1:1 (Instagram), 9:16 (stories) e 16:9 (Behance/LinkedIn).
  _Escopo:_ `lib/capture/generate-compositions.ts` (Sharp: cria o canvas do formato, aplica o fundo
  da marca, encaixa a captura com padding, cantos arredondados e sombra); formatos/cores em
  `lib/config.ts`; saídas em `compositions/` dentro do projeto; campos opcionais no `Catalog`; UI
  pra ver/baixar na página do projeto e incluir na ZIP.
  _Arquivos:_ `generate-compositions.ts`, `config.ts`, `types.ts`, `build-catalog.ts`,
  `app/api/generate/route.ts`, `app/projects/[slug]/page.tsx`.
  _Depende de:_ Sharp (já no pipeline). _Esforço:_ `P`.
  _Aceite:_ cada projeto gera os 3 formatos com caminho público; baixáveis; aparecem na ZIP.

- [ ] **6. Mockups com moldura (browser / device)** `P`–`M`
  _Objetivo:_ a captura dentro de uma moldura realista (janela de browser e/ou device) com sombra,
  parecendo produto. Inclui uma variação inclinada usando uma moldura PNG pronta.
  _Escopo:_ assets de moldura versionados; `lib/capture/generate-mockups.ts` (Sharp compõe a
  screenshot dentro da área útil da moldura); molduras disponíveis em `config`; UI pra escolher/baixar.
  _Arquivos:_ `generate-mockups.ts`, assets de moldura, `config.ts`, `types.ts`, página do projeto.
  _Esforço:_ `P`–`M` (frame reto = P; inclinado via asset = P/M).
  _Nota:_ a versão inclinada aqui é fingida com asset PNG; o 3D renderizado de verdade é o item 12.
  _Aceite:_ gerar mockup de desktop e mobile; baixável; visual premium consistente entre projetos.

- [x] **12. Mockups 3D em perspectiva (render real)** `G`
  _Objetivo:_ o site num device/browser inclinado em perspectiva **real** (não fingido), com sombra,
  profundidade e reflexo — o visual de "revelação" de Behance/Dribbble. Entra no hero do case
  (`/cases/[slug]`), na capa/thumbnail e em posts de rede.
  _Abordagem (reusa a infra atual):_ montar um template HTML com a screenshot aplicada a um elemento
  com `transform: perspective() rotateX/rotateY`, sombra e moldura via CSS, e **fotografar com o
  próprio Playwright** (fundo transparente). É a mesma renderização headless que já roda a captura —
  sem engine 3D pesada. Escalável depois para Three.js (modelos `.glb` de device) se quiser fidelidade
  de foto.
  _Escopo:_ `lib/mockup/templates/` (HTML+CSS dos cenários: laptop, phone, laptop+phone),
  `lib/mockup/render-3d.ts` (injeta a screenshot, abre no Playwright, screenshot com alpha),
  presets de ângulo/luz/fundo em `lib/config.ts`, UI pra escolher cenário e baixar.
  _Arquivos:_ `render-3d.ts`, templates, `config.ts`, `types.ts`, rota/engine, página do projeto.
  _Esforço:_ `G` (o render em si é médio; o trabalho de verdade é desenhar os cenários e tunar
  ângulo/luz/sombra até ficar premium — é iteração de design, não de código).
  _Risco:_ qualidade visual depende do template, não da tecnologia — vai pedir algumas rodadas de ajuste.
  _Aceite:_ gerar mockup 3D de desktop e mobile com fundo transparente + variação sobre fundo da
  marca; baixável; com cara de peça de portfólio premium.

---

## v1.5 — Captura mais rica

> Matéria-prima melhor por projeto. Mesma natureza das `options` da v1.3 (toca tipos + engine),
> sem tecnologia nova — o Playwright já faz tudo isso.

- [x] **7. Múltiplas páginas do site** `M`
  _Objetivo:_ capturar várias rotas do mesmo site, não só a home.
  _Escopo:_ `ProjectInput` ganha `pages?: string[]` (URLs ou paths relativos); a engine itera as
  páginas reusando o pipeline; o catálogo agrupa as capturas por página; UI pra adicionar páginas.
  _Arquivos:_ `types.ts`, `url-input.tsx`, `app/api/generate/route.ts`, orquestração da engine,
  `build-catalog.ts`, página do projeto.
  _Risco:_ toca tipos + catálogo — testar contrato. _Esforço:_ `M`.
  _Aceite:_ gerar 3 páginas produz 3 conjuntos de capturas; catálogo válido; reprocess herda.

- [x] **8. Estados de interação** `M`
  _Objetivo:_ capturar além do estático: menu aberto, modal, dark mode, hover, clicando um seletor.
  _Escopo:_ por captura, uma lista curta de "ações antes do print" (`click <seletor>`, `toggle dark`);
  o Playwright executa antes de fotografar; UI simples pra definir 1–2 passos.
  _Arquivos:_ `types.ts`, engine, `url-input.tsx`, página do projeto.
  _Esforço:_ `M` (o Playwright faz fácil; o trabalho é a UX de definir os passos).
  _Aceite:_ definir "clicar `.menu-toggle`" gera um print com o menu aberto.

- [x] **9. Nomes de seção inteligentes** `P`–`M`
  _Objetivo:_ nomear seções ("Hero", "Sobre", "Serviços") em vez de `section-001`.
  _Escopo:_ melhorar a heurística em `detect-sections` usando heading/landmark/aria/id já capturados;
  fallback pro número. (IA opcional, só se a heurística não bastar.)
  _Arquivos:_ `detect-sections.ts`, `capture-device.ts`. _Esforço:_ `P`–`M`.
  _Depende de:_ já existem `heading`/`suggestedName`.
  _Aceite:_ a maioria das seções recebe nome legível; nunca quebra (cai no número).

---

## v1.6 — Aceleração do case (IA) — DESCARTADA

> **Fora de escopo (decisão do Matheus).** O Coded Atlas fica **sem IA**, 100% determinístico
> (Playwright + Sharp). Nada foi implementado. O `case-draft.mdx` segue gerado pelo caminho
> atual, sem narrativa automática.

- [~] **10. Rascunho de case escrito com IA** — descartado; não será feito.

---

## v1.7 — Monitoramento dos sites entregues

> Dá um motivo recorrente pra abrir a ferramenta: vigiar os sites que você entregou.

- [x] **11. Diff visual de recaptura** `M`–`G`
  _Objetivo:_ recapturar um site já catalogado e comparar com a última versão, destacando o que mudou.
  _Escopo:_ guardar a captura-base; recaptura; `pixelmatch` gera a imagem de diff + % de mudança;
  UI de comparação (antes / depois / diff). Limiar em `config`.
  _Arquivos:_ `lib/capture/visual-diff.ts` (+ dep `pixelmatch`), `types.ts`, rota, página de comparação.
  _Esforço:_ `M`–`G` (o diff é fácil; o storage da base + a UI dão o trabalho).
  _Aceite:_ recapturar mostra % de mudança e destaca regiões; sem base, cria a primeira base.

---

## Ordem sugerida e dependências

```txt
v1.1  Lightbox  ──►  Cmd+K            (independentes, rápidos)            ✓
v1.2  Seleção múltipla                (usa exclusão + ZIP já prontos)     ✓
v1.3  Opções de captura por geração   (toca tipos + engine)              ✓
v1.4  Composições  ──►  Mockups       (Sharp; maior valor de portfólio)
        └─► Mockups 3D em perspectiva  (HTML+CSS 3D fotografado pelo Playwright)
v1.5  Múltiplas páginas · Estados · Nomes de seção  (captura mais rica)
v1.6  Case com IA                     (DESCARTADA — software fica sem IA)   ✗
v1.7  Diff visual de recaptura        (pixelmatch)                          ✓
```

**Roadmap fechado.** Tudo entregue exceto a v1.6, descartada por decisão de manter o software sem IA.

> Publicação automática no GitHub do portfólio ficou **fora do roadmap** (decisão do Matheus).
> A publicação segue manual assistida: baixar/copiar `portfolio.json` + `case-draft.mdx`.

Critério para fechar cada versão: `next build` limpo, type-check sem erro, teste(s) do que
for testável isoladamente e verificação visual (screenshot) das telas afetadas — o mesmo
padrão usado até aqui.

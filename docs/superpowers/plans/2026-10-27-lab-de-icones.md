# Lab de ícones e logos — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** a static HTML bench (`spikes/lab-icones.html`) that shows every icon
and logo of the M/OS family and marks each one against its rule.

**Architecture:** a Node generator in `scripts/lab-icones/` that reads the real
files and components, measures them with pure functions and writes one
self-contained page. It has no new dependencies: it uses the esbuild and React
already in `apps/desktop`.

**Spec:** `docs/superpowers/specs/2026-10-27-lab-de-icones-design.md`

## Global Constraints

- Node ESM, no new dependencies. Runs as `npm run lab` from `apps/desktop`.
- The output `spikes/lab-icones.html` goes in `.gitignore`.
- The expected values come from the code: `Symbol.tsx`, `gerar-icones.py`,
  `generate-family-icons.py` and `tokens.css`. The brief is the second source,
  and every disagreement goes to "Regras em conflito".
- Tolerances:
  - **Angle:** ±2.5° from 48px up, ±5° below 48px.
  - **Radius:** ±1.5 pp, or 0.6px, whichever is larger.
  - **Color:** RGB distance 12 to the palette or to a segment between two of its
    colors, with at most 0.5% of the pixels outside.
- Tests: only `node --test scripts/lab-icones/`.

---

### Task 1: Image readers (`imagens.mjs`)

**Produces:**
- `lerPng(buf) -> {largura, altura, rgba: Uint8Array}`
- `lerIco(buf) -> [{tamanho, imagem}]`
- `lerIcns(buf) -> [{tipo, tamanho, imagem|null}]`
- `lerRgba(buf, tamanho)`
- `escreverPng(imagem) -> Buffer`
- `reduzir(imagem, lado)`: area average.

- [ ] Test: a PNG written and read again keeps its pixels; an ICO built in the
  test with two PNG frames returns both; `reduzir` of a uniform 4×4 returns the
  same color.
- [ ] Implement PNG reading:
  - color types 0, 2, 3, 4 and 6, at 8 bits;
  - the five filters;
  - ICO with a PNG or BMP 32bpp frame;
  - ICNS with PNG chunks; legacy chunks come back `null`.
- [ ] `node --test scripts/lab-icones/imagens.test.mjs` passes.

### Task 2: The measures (`medidas.mjs`)

**Produces:**
- `faixaDe(tamanho)`
- `inclinacaoDoPoligono(pontos)`: degrees from the vertical.
- `inclinacaoNoBitmap(imagem, tinta)`
- `raioNoBitmap(imagem)`: fraction of the side.
- `coresForaDaPaleta(imagem, paleta)`
- `pareceReduzido(menor, maior)`
- `conferirIcone(svg, {lado, traco})`
- `selo(tipo, medido, esperado, nota)`

- [ ] Tests:
  - the three polygons of the `Symbol.tsx` give their lean;
  - a polygon rasterized in the test at 64 and at 16 gives a lean within the
    tolerance;
  - a rounded rectangle rasterized with radius 21% measures 21% ±1;
  - a mixed pixel between the sodium and the ink passes, and a pure red fails;
  - an SVG with viewBox 48 in a 20 icon, and a coordinate at .3, are marked.
- [ ] Implement.
- [ ] `node --test` passes.

### Task 3: Components and inventory (`componentes.mjs`, `gerar.mjs`)

- [ ] `componentes.mjs`:
  - esbuild bundles an entry that imports `Icon`, `SmallIcon`, `MosSymbol`,
    `AppIcon`/`brandOf` and `MarcaDeIA`;
  - it renders with `renderToStaticMarkup`;
  - the names come from the `IconName` union and `OUTLINE_16`, read from the
    source;
  - it returns `[{grupo, nome, variante, svg}]`.
- [ ] `gerar.mjs`:
  - `git ls-files`;
  - group by app from the path;
  - decode each file;
  - measure with the rules for its family;
  - collect the uses (references by name in text files) and mark orphans;
  - collect the uses of `filled`;
  - read the expected values from the sources with a regex: the polygons of
    `Symbol.tsx`, `RAIO`, `RECIPE`, `CAMPOS`, and the palette of `tokens.css`.
- [ ] Add `"lab": "node ../../scripts/lab-icones/gerar.mjs"` to
  `apps/desktop/package.json` and `spikes/lab-icones.html` to `.gitignore`.

### Task 4: The page (`pagina.mjs`)

- [ ] Sections, in the order of spec §4:
  1. score and filter;
  2. conflicts;
  3. family 16→256;
  4. contexts: taskbar light and dark, tray, sidebar in both themes, iPhone;
  5. groups, with real size + 8× pixelated and grid, seals and path.
- [ ] Theme toggle via `data-theme`. `tokens.css` is embedded.
- [ ] Generate, open it in headless Playwright in both themes and look at the
  screenshot.
- [ ] Commit.

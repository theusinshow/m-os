# M/OS Agent Instructions

Before making product, UX, architecture, or implementation decisions, read all documents inside `/docs`.

The documents in `/docs` are the product source of truth.

## O M/OS é multi-device por definição

Desde 2026-08-21 (ADR-052), o M/OS não é um aplicativo de Windows. Ele é um
sistema pessoal com **duas interfaces sobre o mesmo cérebro**: desktop e iPhone.

Isso muda o que "implementar uma feature" significa. Não existe mais:

> "feature do desktop que talvez seja portada depois"

Existe:

> "feature do M/OS, com manifestações apropriadas em cada plataforma"

Ao receber qualquer pedido de feature, analise os dois contextos e declare os
oito eixos do checklist em `docs/FEATURE-DEVELOPMENT.md`. **"Não se aplica ao
iOS, porque…" é uma resposta boa. Deixar o eixo em branco não é.**

Três regras duras:

1. Nada de plataforma em `mos-core` ou `mos-sync` — um `#[cfg(windows)]` ali
   significa que o desenho quebrou.
2. Pergunte o que a plataforma **pode fazer** (`apps/desktop/src/platform.ts`),
   nunca **qual ela é**.
3. Lógica de negócio não se duplica. Componentes de interface podem divergir;
   serviços de domínio, não.

Leia `docs/PLATFORMS.md` antes de decidir onde uma feature mora, e
`docs/SYNC.md` antes de mexer em qualquer coisa que atravesse dispositivos.

## Priority

1. `VISION.md`
2. `PRODUCT.md`
3. `CORE.md`
4. `UX-PRINCIPLES.md`
5. `PLATFORMS.md`
6. `FEATURE-DEVELOPMENT.md`
7. `ROADMAP.md`
8. `IDEAS.md`

Important:

- `IDEAS.md` contains possibilities, not requirements.
- `ROADMAP.md` defines intended sequencing, not technical architecture.
- Preserve the product philosophy defined in `VISION.md`.
- UX decisions must respect `UX-PRINCIPLES.md`.
- Do not silently convert future ideas into current scope.
- Do not start implementation before architecture and product foundations are reviewed.
- Prefer explicit architectural reasoning and documented decisions over premature coding.

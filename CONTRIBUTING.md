# Contributing to M/OS

M/OS is currently a personal product under active development. Contributions
must preserve the product intent before optimizing implementation details.

## Before changing behavior

Read `AGENTS.md` and every document in `docs/`. Treat `VISION.md`,
`PRODUCT.md`, `CORE.md`, and `UX-PRINCIPLES.md` as constraints. `IDEAS.md` is
not approved scope.

Before changing interface code, also read:

- `Design System/handoff/AGENTS.md`
- `Design System/handoff/mos-design-system.md`
- `Design System/handoff/mos-tokens.css`

## Local checks

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps\desktop
npm ci
npm run build
```

Interface changes also require keyboard and Windows UI Automation checks in
dark and light themes.

## Pull requests

Keep each pull request focused. State the product problem, the chosen scope,
trade-offs, validation performed, and anything deliberately left out.

Do not include local databases, backups, logs, build output, credentials, or
personal captured content.

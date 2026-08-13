## What changed

<!-- Describe the behavior and scope. -->

## Why

<!-- Link the change to product constraints or a validated problem. -->

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `npm run build` in `apps/desktop`
- [ ] Relevant Windows UI flows were checked
- [ ] Dark, light, keyboard, and reduced-motion states were considered

## Scope control

- [ ] Product documents were reviewed
- [ ] No future idea was promoted into scope silently
- [ ] User data and local runtime artifacts are not included

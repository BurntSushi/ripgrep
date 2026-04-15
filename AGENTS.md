<!-- crag:auto-start -->
# AGENTS.md

> Generated from governance.md by crag. Regenerate: `crag compile --target agents-md`

## Project: ripgrep


## Quality Gates

All changes must pass these checks before commit:

### Lint
1. `cargo clippy -- -D warnings`
2. `cargo fmt --check`

### Test
1. `cargo test`

### Ci (inferred from workflow)
1. `cargo build --verbose`
2. `cargo fmt --all --check`
3. `cargo check`

## Coding Standards

- Stack: rust
- Follow project commit conventions

## Architecture

- Type: monorepo (cargo)

## Key Directories

- `.github/` — CI/CD
- `ci/` — tooling
- `crates/` — workspace crates
- `pkg/` — source
- `scripts/` — tooling
- `tests/` — tests

## Testing

- Framework: cargo test
- Layout: flat

## Code Style

- Formatter: rustfmt

## Anti-Patterns

Do not:
- Do not use `unwrap()` in library code — return `Result` instead
- Do not `clone()` without justification — prefer borrowing
- Do not use `unsafe` without a safety comment explaining invariants

## Security

- No hardcoded secrets — grep for sk_live, AKIA, password= before commit

## Workflow

1. Read `governance.md` at the start of every session — it is the single source of truth.
2. Run all mandatory quality gates before committing.
3. If a gate fails, fix the issue and re-run only the failed gate.
4. Use the project commit conventions for all changes.

<!-- crag:auto-end -->

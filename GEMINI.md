<!-- crag:auto-start -->
# GEMINI.md

> Generated from governance.md by crag. Regenerate: `crag compile --target gemini`

## Project Context

- **Name:** ripgrep
- **Stack:** rust
- **Runtimes:** rust

## Rules

### Quality Gates

Run these checks in order before committing any changes:

1. [lint] `cargo clippy -- -D warnings`
2. [lint] `cargo fmt --check`
3. [test] `cargo test`
4. [ci (inferred from workflow)] `cargo build --verbose`
5. [ci (inferred from workflow)] `cargo fmt --all --check`
6. [ci (inferred from workflow)] `cargo check`

### Security

- No hardcoded secrets — grep for sk_live, AKIA, password= before commit

### Workflow

- Follow project commit conventions
- Run quality gates before committing
- Review security implications of all changes

<!-- crag:auto-end -->

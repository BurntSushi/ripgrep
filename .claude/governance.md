# Governance — ripgrep
# Inferred by crag analyze — review and adjust as needed

## Identity
- Project: ripgrep
- Stack: rust
- Workspace: cargo

## Gates (run in order, stop on failure)
### Lint
- cargo clippy -- -D warnings
- cargo fmt --check

### Test
- cargo test

### CI (inferred from workflow)
- cargo build --verbose
- cargo fmt --all --check
- cargo check

## Advisories (informational, not enforced)
- actionlint  # [ADVISORY]

## Branch Strategy
- Trunk-based development
- Free-form commits
- Commit trailer: Co-Authored-By: Claude <noreply@anthropic.com>

## Security
- No hardcoded secrets — grep for sk_live, AKIA, password= before commit

## Autonomy
- Auto-commit after gates pass

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

## Dependencies
- Package manager: cargo (Cargo.lock)
- Rust: >=1.85
- Rust-edition: 2024

## Anti-Patterns

Do not:
- Do not use `unwrap()` in library code — return `Result` instead
- Do not `clone()` without justification — prefer borrowing
- Do not use `unsafe` without a safety comment explaining invariants


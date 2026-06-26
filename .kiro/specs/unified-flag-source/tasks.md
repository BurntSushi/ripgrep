# Implementation Plan: unified-flag-source

## Overview

This plan consolidates ripgrep's flag-generation pipeline around the existing
`FLAGS` registry as a documented single source of truth. The work proceeds
bottom-up: first a validated read-only `RegistryView` (the test seam and
ordering authority), then a fallible `Markup_Renderer`, then the generators
(man, help, zsh, bash, fish, powershell) routed through both, then the
`--generate` command wiring, and finally the `Consistency_Checker` integration
test. Pure logic (validation, markup, generation, checking) is exercised with
`proptest` property tests built over synthetic registries.

All code is Rust, using `anyhow::Result` for fallible generation and `proptest`
for property-based testing, consistent with the design.

## Tasks

- [x] 1. Establish the validated registry view (single source of truth foundation)
  - [x] 1.1 Add `RegistryView` and registry validation
    - In `crates/core/flags/mod.rs` (and `defs.rs` as needed), add a read-only `RegistryView` over the `FLAGS` slice with `load()`, `iter()`, `by_category()`, and `lookup_long()`
    - `load()` performs registry-wide validation: detect duplicate long/short/negated names (`RegistryError::DuplicateLong/Short/Negated`) and runtime-checkable missing mandatory fields (`RegistryError::MissingField`), returning an error that names the conflicting field/value or the affected flag + field; produce no view on failure
    - `by_category()` emits categories in fixed `Category` declaration order and flags within each category in `FLAGS` declaration order; this is the shared ordering authority so a single registry edit propagates to every generator
    - _Requirements: 1.1, 1.8, 1.9, 1.10, 7.2, 7.3_

  - [x] 1.2 Build the proptest synthetic-registry strategy (test infrastructure)
    - Add a `proptest` strategy that produces synthetic registries: vectors of generated `Flag_Definition`s with varied long names, optional short names, optional negations, alias lists, categories, switch-vs-value nature, completion types, choice lists, and short/long docs (including empty `doc_short` and hyphen-rich names)
    - Add strategy variants that inject duplicate names, invalidate a mandatory field, and perturb a generated artifact, to drive the negative properties
    - _Requirements: supports Properties 1-25_

  - [x] 1.3 Write property test for duplicate-name validation
    - **Property 2: Duplicate names fail validation**
    - **Validates: Requirements 1.9**

  - [x] 1.4 Write property test for missing-field validation
    - **Property 3: Missing mandatory field halts generation**
    - **Validates: Requirements 1.10**

- [x] 2. Centralize documentation markup rendering
  - [x] 2.1 Create the fallible `Markup_Renderer`
    - Add `crates/core/flags/doc/markup.rs` with `render_markup(doc, registry, flavor) -> Result<String, MarkupError>`, the `MarkupError` enum (`UnknownFlag`, `NoNegation`, `Malformed`), and `MarkupFlavor { Roff, Plain }`
    - Resolve `\flag{name}` to the long name and `\flag-negate{name}` to the negated name via `RegistryView`; return errors (and no artifact) for unknown names, negation of non-negatable flags, and unrecognized/malformed tags
    - Centralize roff hyphen escaping so each hyphen in an emitted flag name becomes `\-` exactly once; replace the panicking `render_custom_markup` path in `doc/mod.rs`
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

  - [x] 2.2 Write property test for cross-reference resolution
    - **Property 11: Markup resolves flag and negation cross-references**
    - **Validates: Requirements 5.1, 5.2**

  - [x] 2.3 Write property test for unknown-name markup
    - **Property 12: Markup referencing an unknown name errors**
    - **Validates: Requirements 5.3**

  - [x] 2.4 Write property test for negation markup on non-negatable flag
    - **Property 13: Negation markup on a non-negatable flag errors**
    - **Validates: Requirements 5.4**

  - [x] 2.5 Write property test for malformed markup
    - **Property 14: Malformed markup errors**
    - **Validates: Requirements 5.5**

  - [x] 2.6 Write property test for roff hyphen escaping
    - **Property 15: Hyphens in flag names are escaped in roff**
    - **Validates: Requirements 5.6**

- [x] 3. Route man and help generators through the registry view and renderer
  - [x] 3.1 Update `Man_Generator`
    - In `doc/man.rs`, consume `RegistryView` and call `render_markup(.., MarkupFlavor::Roff)`; remove the generator-local hyphen replacement
    - Place each flag under exactly one matching category heading; display the value variable name immediately after the flag name for non-switch flags that have one and never for switches; document that a switch with a negated name can be disabled, showing the negated name verbatim
    - _Requirements: 1.6, 4.3, 5.6, 9.1, 9.5, 9.6_

  - [x] 3.2 Update `Help_Generator`
    - In `doc/help.rs`, consume `RegistryView` and call `render_markup(.., MarkupFlavor::Plain)` for both short (`-h`) and long (`--help`) output
    - Place each flag under exactly one matching category heading; render markup-resolved short docs in short help and long docs in long help; document that a switch with a negated name can be disabled in the long help, showing the negated name verbatim
    - _Requirements: 1.7, 4.4, 9.2, 9.3, 9.4_

  - [x] 3.3 Write property test for documented negation
    - **Property 10: Negation is documented for switches in man and long help**
    - **Validates: Requirements 4.3, 4.4**

  - [x] 3.4 Write property test for category headings
    - **Property 23: Each flag appears under exactly one matching category heading**
    - **Validates: Requirements 9.1, 9.2**

  - [x] 3.5 Write property test for help documentation rendering
    - **Property 24: Help renders every flag's documentation with markup resolved**
    - **Validates: Requirements 9.3, 9.4**

  - [x] 3.6 Write property test for man value-variable display
    - **Property 25: Man shows value variables only for value flags**
    - **Validates: Requirements 9.5, 9.6**

  - [x] 3.7 Write unit tests for man/help edge cases
    - Empty `doc_short` rendering; roff hyphen escaping on a representative real flag
    - _Requirements: 2.5, 5.6_

- [x] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Generate the Zsh completion's per-flag content from the registry
  - [x] 5.1 Generate Zsh per-flag specs from `RegistryView`
    - In `complete/zsh.rs` and the `rg.zsh` template, add a `!FLAGS!` splice point (mirroring `!ENCODINGS!`/`!HYPERLINK_ALIASES!`) and emit one `_arguments`-style spec per flag from the registry; remove the hand-maintained per-flag block while keeping the prelude, helpers, and flag-compatibility grouping
    - Each entry includes the long name; includes the short name and the negated name as separately completable options where present; uses description text character-for-character identical to `doc_short` (including the empty case); derives value completion from the completion-type mapping with names/descriptions/choices left byte-identical to the registry even under contextual grouping
    - _Requirements: 1.3, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2, 7.1, 7.4_

  - [x] 5.2 Write property test for Zsh per-flag faithfulness
    - **Property 4: Zsh produces one faithful entry per flag**
    - **Validates: Requirements 2.1, 2.2, 2.4, 2.5, 2.6**

- [x] 6. Implement consistent value completion across the remaining shells
  - [x] 6.1 Apply the completion-type mapping in `Bash_Generator`
    - In `complete/bash.rs`, consume `RegistryView`; introduce explicit `None` completion semantics and map Filename/Executable/Filetype/Encoding/Choices to Bash's native constructs; offer declared choices exactly and in declared order; offer negated names and every alias as completable flags
    - _Requirements: 1.2, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2_

  - [x] 6.2 Apply the completion-type mapping in `Fish_Generator`
    - In `complete/fish.rs`, consume `RegistryView`; map completion types to Fish constructs; ensure a switch requests no value; offer declared choices exactly and in order; offer negated names and every alias as completable flags
    - _Requirements: 1.4, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2_

  - [x] 6.3 Apply the completion-type mapping in `PowerShell_Generator`
    - In `complete/powershell.rs`, consume `RegistryView`; map completion types to PowerShell constructs; offer declared choices exactly and in order; offer negated names and every alias as completable flags
    - _Requirements: 1.5, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2_

  - [x] 6.4 Write property test for completion-type mapping across shells
    - **Property 5: Completion-type maps to the right construct in every shell**
    - **Validates: Requirements 3.1, 3.2, 3.4, 3.5**

  - [x] 6.5 Write property test for declared choices
    - **Property 6: Declared choices are offered exactly and in order**
    - **Validates: Requirements 3.3**

  - [x] 6.6 Write property test for Fish switches
    - **Property 7: Fish requests no value for switches**
    - **Validates: Requirements 3.6**

  - [x] 6.7 Write property test for negated-name completion
    - **Property 8: Negated names are completable in every shell**
    - **Validates: Requirements 2.3, 4.1**

  - [x] 6.8 Write property test for alias completion
    - **Property 9: Aliases are completable in every shell**
    - **Validates: Requirements 4.2**

- [x] 7. Wire the generation command and determinism guarantees
  - [x] 7.1 Formalize the `Generate_Command` dispatch
    - In `main.rs`, dispatch each recognized `--generate <mode>` to its generator and write only that artifact to stdout; propagate any generator/registry/markup error to stderr with empty stdout and a non-zero exit; handle unrecognized mode and missing mode the same way; exit zero on success
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8_

  - [x] 7.2 Write example tests for the `--generate` command
    - Each recognized mode (stdout equals generator output, exit zero); unrecognized mode (stderr message, empty stdout, non-zero exit); missing mode (same)
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8_

  - [x] 7.3 Write property test for single-edit propagation
    - **Property 1: Single edit propagates to every artifact**
    - **Validates: Requirements 1.8**

  - [x] 7.4 Write property test for deterministic generation
    - **Property 21: Generation is deterministic**
    - **Validates: Requirements 7.1, 7.4**

  - [x] 7.5 Write property test for fixed emission order
    - **Property 22: Flags and categories are emitted in fixed order**
    - **Validates: Requirements 7.2, 7.3**

- [x] 8. Implement the automated `Consistency_Checker`
  - [x] 8.1 Build the checker and per-shell extractors
    - Add a test module under `crates/core/tests/` defining `Violation`, `ViolationKind`, `ArtifactId`, and `check_all(registry) -> Vec<Violation>`
    - Implement format-specific extractors for each completion artifact (Bash opts list, Zsh `_arguments` specs, Fish `-l`/`-s`/`-d` lines, PowerShell `[CompletionResult]::new(...)`) and compare against the registry: missing flags, unexpected long names, unexpected short aliases, and exact description mismatches; accumulate every violation in one run; empty result on full agreement
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

  - [x] 8.2 Write property test for missing-flag detection
    - **Property 16: Checker detects missing flags**
    - **Validates: Requirements 6.1**

  - [x] 8.3 Write property test for unexpected-name detection
    - **Property 17: Checker detects unexpected names**
    - **Validates: Requirements 6.2, 6.3**

  - [x] 8.4 Write property test for description-mismatch detection
    - **Property 18: Checker detects description mismatches**
    - **Validates: Requirements 6.4**

  - [x] 8.5 Write property test for all-violations reporting
    - **Property 19: Checker reports all violations in one run**
    - **Validates: Requirements 6.5**

  - [x] 8.6 Write property test for faithful-artifact success
    - **Property 20: Faithful artifacts pass the checker**
    - **Validates: Requirements 6.6**

  - [x] 8.7 Wire the checker against the real registry as the CI drift guard
    - Run `check_all` against the real `FLAGS` registry and the real generated artifacts as an integration test, replacing the loose `ci/test-complete` zsh-only check as the authoritative guard
    - _Requirements: 6.6_

- [x] 9. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional test sub-tasks and can be skipped for a
  faster MVP; core implementation tasks are never optional.
- Each task references specific requirement sub-clauses for traceability.
- Each of the 25 design properties is implemented by exactly one property-based
  test, tagged with `// Feature: unified-flag-source, Property {n}: {text}` and
  configured for at least 100 cases, using `proptest`.
- Generators and the checker accept a `RegistryView` so property tests can
  supply synthetic registries; production call sites pass the real `FLAGS`.
- The Bash, Fish, PowerShell, man, and help outputs should be snapshot-compared
  before and after the refactor to confirm byte-stability; the Zsh output is
  expected to change and is validated by the checker and the Zsh properties.

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "2.1"] },
    { "id": 2, "tasks": ["1.3", "1.4", "2.2", "2.3", "2.4", "2.5", "2.6", "3.1", "3.2", "5.1", "6.1", "6.2", "6.3"] },
    { "id": 3, "tasks": ["3.3", "3.4", "3.5", "3.6", "3.7", "5.2", "6.4", "6.5", "6.6", "6.7", "6.8", "7.1", "8.1"] },
    { "id": 4, "tasks": ["7.2", "7.3", "7.4", "7.5", "8.2", "8.3", "8.4", "8.5", "8.6", "8.7"] }
  ]
}
```

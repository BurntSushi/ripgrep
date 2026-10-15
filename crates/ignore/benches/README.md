# Workspace discovery benchmarks

`workloads.rs` measures the `ignore` operations used by Python development tools:

| Consumer | Operation | Benchmarks |
| --- | --- | --- |
| [Ruff file discovery](https://github.com/astral-sh/ruff/blob/ae8ec3753192debd310e7b3d93255a19a5a2af85/crates/ruff_workspace/src/resolver.rs#L530) and [ty project indexing](https://github.com/astral-sh/ruff/blob/ae8ec3753192debd310e7b3d93255a19a5a2af85/crates/ruff_db/src/system/os.rs#L312) | Parallel walks, hidden files included, up to 12 workers | `walk_*_parallel_*`, `walk_explicit_files` |
| [uv script discovery](https://github.com/astral-sh/uv/blob/83d556d712c7e316932b239c49e86a5de95d124a/crates/uv/src/commands/workspace/list.rs#L137) | Serial traversal with ignore rules and hidden files included | `walk_*_serial`, `uv_scripts_source_archive` |
| [uv workspace discovery](https://github.com/astral-sh/uv/blob/83d556d712c7e316932b239c49e86a5de95d124a/crates/uv-workspace/src/workspace.rs#L1931) | Stop at the first non-directory entry, or exhaust an ignored member | `uv_*_workspace_member` |
| [ty incremental indexing](https://github.com/astral-sh/ruff/blob/ae8ec3753192debd310e7b3d93255a19a5a2af85/crates/ruff_db/src/system/os/ignore.rs#L10) | Repeated relative-path checks with cached ancestor matchers | `incremental_*` |

The synthetic trees contain Python modules, package metadata, non-Python files,
ignored bytecode directories, a virtual environment, and build outputs. The
layouts vary independently: a small project, many sibling packages, deeply
nested packages, and packages with their own ignore rules and negations. The
parallel visitor counts files locally and merges once per worker, avoiding an
artificial shared counter on every entry. One- and four-worker cases supplement
the 12-worker configuration used by Ruff and ty. The unfiltered walk models
disabling ignore rules and provides a control for traversal overhead.
The source-archive case honors `.gitignore` without a `.git` directory, matching
uv's `require_git(false)` configuration.

Fixtures are created before timing and removed afterward. Every walk checks its
Python file count against the fixture's expected count. The benchmarks retain
parent ignore discovery, but disable global Git ignore configuration so that
results do not depend on a user's Git configuration. Runs use a warm filesystem
cache. They measure `ignore`, excluding parsing, configuration resolution, and
other application work; they are not end-to-end application speedups.

`incremental_cold` includes building matchers and loading directory rules on
first use. `incremental_warm` checks 256 paths after their directories have been
cached. `incremental_warm_noncanonical` checks the same paths with an internal
`./` component, exercising normalization during cached queries.
`incremental_ignored` checks a path beneath a pruned virtual environment.
`incremental_warm_deep` checks 768 paths in a 12-level package tree, and
`incremental_ignored_parent` directly hits an ignored directory's cache entry.
`incremental_ignored_nested` checks a deeper descendant of a cached ignored
virtual environment inside an otherwise allowed package.
The `walk_dense_ignore_*` cases put rules in every directory of a deep tree,
providing a control for optimizations that skip directories without rules.
`incremental_dense_ignore_cold` and `incremental_dense_ignore_warm` use the same
tree to check incremental matching when nearly every cached directory has rules.
The `walk_multiple_roots_*` cases pass 64 sibling package directories as roots,
covering parent-matcher reuse when Ruff receives multiple directory arguments.
`walk_with_overrides_and_types` exercises explicitly configured override and
file-type matchers as a control for their more common empty configuration.
The `build_*` and `match_*` microbenchmarks help distinguish compilation and
matching costs from filesystem traversal.
The single-pattern cases cover bytecode exclusions, the directory whitelist
used in Ruff's and uv's root `.ignore` files, and ty's vendored-typeshed whitelist.
`build_single_directory_whitelist` builds that single-rule matcher from an
already populated builder.
The `*gitignore_only*` controls remove `.ignore` and `.git/info/exclude`, covering
projects that rely solely on a `.gitignore` with multiple patterns.
`walk_selected_python_files` uses two positive override patterns, exercising
file-selection rules that match most visited files.

Like the `globset` benchmarks, these use Rust's nightly `test` harness:

```console
cargo +nightly bench -p ignore --bench workloads
cargo +nightly bench -p ignore --bench workloads incremental
```

For comparisons, use the same compiler, profile, fixture parameters, and CPU
affinity for both revisions. Alternate baseline and candidate runs, and retain
the per-run results instead of relying on a single measurement. Parallel walks
also include worker startup and shutdown, which can dominate small projects.

CPU affinity also affects match-index scratch-pool sizing: available parallelism
is cached when the first pool is constructed. Pinning to one CPU therefore
changes pool construction costs as well as scheduling. Report affinity with
results, especially when comparing construction or cached matching costs.

Ruff, uv, and ty enable fat LTO in release builds, while their profiling builds
omit cross-crate LTO. Check promising changes in both configurations, especially
when small matching benchmarks give inconsistent results:

```console
CARGO_PROFILE_BENCH_LTO=fat CARGO_PROFILE_BENCH_CODEGEN_UNITS=16 cargo +nightly bench -p ignore --bench workloads
```

For focused comparisons, pass one benchmark name with `--exact` so that each
case runs in a separate process:

```console
cargo +nightly bench -p ignore --bench workloads -- --exact incremental_ignored_parent
```

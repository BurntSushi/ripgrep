This document describes how indexing ought to behave and the boundary between
ripgrep core and the `grep-index` crate.

ripgrep core owns discovery, the runtime corpus root, traversal/ignore policy,
freshness reconciliation, CLI semantics, and opening candidate files.

`grep-index` owns relative-path validation, trigram extraction/querying,
segments, tombstones, redb transactions, metadata, and compaction.


## grep-index

Most of the indexing logic should live in the `grep-index` crate, and that
crate's architecture is based on the [Nakala design]. However, instead of
rolling our own embedded database, we use [redb].

Here are some primary design constraints:

* An index ought to be relocatable. That is, it should not store any absolute
  file paths. It should store file paths relative to the index's logical
  corpus root. For example, for an index stored at `foo/.ripgrep`, its paths
  should be relative to `foo`, not `foo/.ripgrep`.
* An index's representation should be stored inside a directory. An index owns
  the contents of that directory, including an `redb` database. For example,
  if we are indexing the directory `foo`, then its index ought to live in a
  `foo/.ripgrep` directory.
* The postings list is stored as a binary encoding of sorted varints inside
  redb. The key for the posting list is a `(segment_id, trigram)` tuple. The
  varint values are segment local document IDs.
* Integration tests should be written inside of `./crates/index/tests`.
* Things like document IDs and segment IDs should not be exposed to the caller.
* Querying is done based on trigram and can use the
  `grep_index::literal::GramQuery` type.
* Index does not retain a database handle between top-level operations.
  Candidate enumeration uses one read-only handle and transaction. Filesystem
  scanning and segment construction hold no handle. Publication uses one
  writable handle and transaction.
  * One read-only handle/transaction for an entire top-level query.
  * No database handle while scanning files or constructing a segment.
  * One writable handle/transaction while publishing.
  * Segment IDs, tombstones, and catalog changes are derived from the current
    catalog inside that transaction.
  * Compaction uses a catalog generation check or equivalent retry mechanism.
* A `DatabaseAlreadyOpen` causes the caller to wait.
* Writing a new segment, replacement tombstones, deletions, and the
  active-segment catalog should occur in one redb write transaction. Filesystem
  scanning and trigram construction should happen before that transaction.
* Candidates reported by `grep-index` may be false positives, but there can't
  be any false negatives (assuming the index is fully up to date).
* For UTF-16 files, `grep-index` should include trigrams for only the transcoded
  UTF-8 bytes. Not UTF-16.
* Only the content of binary files up to the first NUL byte are indexed.
  Anything beyond that is ignored. Files that begin with a UTF-16 BOM are not
  considered as binary.
* File paths are stored as their raw bytes on Unix and as UTF-8 on Windows.
  File paths that aren't valid UTF-8 on Windows cannot be indexed. We can get
  the raw bytes using [`ByteSlice::from_path`].
* Data inside `redb` can be assumed to be correct. For example, we can assume
  that `ByteSlice::to_path` always succeeds because only data from
  `ByteSlice::from_path` was put in there. Similarly for other data like the
  postings lists. We can assume the varints are encoded correctly. Namely, we
  can also assume that `grep-index` is the only writer to the underlying
  database.

At present, older formats aren't migrated automatically. `grep-index`
should report an error for older formats that it doesn't support. Users are
then expected to perform manual intervention, like deleting the index and
re-indexing it, to fix the error.

Here are more constraints on file paths:

* There is exactly one corpus root per index.
* The root is never persisted in any index artifact.
* Core passes ROOT/.ripgrep to grep-index; the redb filename stays private.
* Stored paths must be non-empty and relative.
* Reject absolute paths, platform prefixes, and every .. component.
* Normalize . and redundant separators lexically.
* Do not canonicalize, case-fold, or Unicode-normalize path identity.
* Windows paths that cannot be represented as UTF-8 produce an error, never a silent omission.
* Core is responsible for joining the relative paths returned by `grep-index`
  with the corpus root.


## ripgrep core

ripgrep core should be responsible for discovering the right index to use. It
should also drive the core indexing routines.

An index is always addressed by the directory that is indexed. For example,
if a user needs to provide a path to an index stored at `foo/bar/.ripgrep`,
then the user actually provides the path `foo/bar`. Paths inside the index are
always stored relative to `foo/bar`.

### Reading an index

When reading an index via the `-X/--index` flag (which can only be provided
once), its location should be determined by this ordering:

1. When path operands are given on the command line, each operand is
interpreted as directory with a `.ripgrep` sub-directory containing the index.
Each such index is searched. An operand that isn't itself a directory with a
`.ripgrep` sub-directory results in an error.
2. A valid index in a `.ripgrep` directory in the current working directory.
3. A valid index in a `.ripgrep` directory in the nearest parent of the current
working directory.

If no index is found or one is found but is corrupt or not a valid index, then
ripgrep returns an error.

When indexing is enabled, most flags that do filtering or require transforming
the contents of a file in some way, aren't supported. So there should not be
a reason to build out support for them at present. The unsupported flags are
checked in `LowArgs::indexing_unsupported_flag`, and this applies to both
reading and updating an index.

In some cases, ripgrep may need to search every file in an index. For example,
when the `-v/--invert-match` flag is used or if a regex could not have any
trigrams extracted from it (e.g., `\w+`).

When ripgrep uses an index, it will always limit the files it searches to
whatever is in the index. This means ripgrep will not do a directory traversal
while it is also using an index. This also means that if the index is stale,
ripgrep may miss reporting some matches.

Indexes may be overlapping. For example, a `foo/.ripgrep` and a
`foo/bar/.ripgrep` index co-existing is legal. Assuming that `foo` was
constructed in a way to also index the contents of `foo/bar`, then the indexes
are likely duplicative. This isn't a problem other than using additional disk
space. When one searches both indexes via the `-X` flag, then duplicate
results may be shown.

### Updating an index

When updating an index with the `--x-crud` flag, ripgrep accepts zero or more
file paths. When no path is given, then the current directory is indexed.
Providing both `-X` and `--x-crud` results in an error.

An existing file is re-indexed only when its modification time is not equal to
the modification time observed when it was indexed. File size should also be
used such that when either size or modification time changes, then that file
should be updated. Use `--x-force` to re-index every selected file, including
files on file systems with unreliable or deliberately preserved modification
times.

A previously indexed path that is no longer accessible (as in, it doesn't
exist) is removed from the index.

ripgrep chooses the index location in the following order:

1. A valid index in a `.ripgrep` directory in the current working directory.
2. A valid index in a `.ripgrep` directory in the nearest parent of the current
working directory.
3. A `.ripgrep` directory in the current working directory, which is created
when necessary.

If the final location already exists but does not contain a valid index, then
ripgrep reports an error. An existing but invalid `.ripgrep` directory found
at any point should result in an error.

ripgrep must always ignore `.ripgrep` directories during traversal, so that
the index doesn't index itself.

Moreover:

* Only selected files/subtrees are reconciled.
* A missing selected directory removes all indexed descendants beneath its prefix.
* An unseen file is deleted only after a complete successful traversal.
* Permission or transient traversal failures must suppress deletion for the affected subtree.
* All update operands must be contained within the chosen corpus root. Otherwise
  ripgrep will report an error.
* Updates should try to publish what they can. If errors are hit, then those
  should be written to stderr, but the update process should still continue.

[Nakala design]: https://github.com/BurntSushi/nakala/blob/master/doc/PLAN.md
[redb]: https://docs.rs/redb
[`ByteSlice::from_path`]: https://docs.rs/bstr/latest/bstr/trait.ByteSlice.html#method.from_path

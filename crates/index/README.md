grep-index
==========

`grep-index` is an experimental persistent byte-trigram index for ripgrep.
It never claims that a file matches. It returns files that cannot be ruled
out, and ripgrep still performs the real search over those candidates.

The design keeps the useful pieces of Nakala's PLAN.md while delegating the
embedded-database work to redb:

- writes publish immutable logical segments;
- each segment assigns dense, one-based `u32` DocIDs in lexicographic path
  order;
- a monotonic `fst::Map` provides both path-to-DocID and DocID-to-path
  lookups;
- redb stores contiguous delta-varint posting-list blobs, metadata, the
  active segment catalog and compact tombstone bitmaps;
- an upsert atomically tombstones older path versions and publishes a new
  segment;
- compaction rewrites live documents into one tombstone-free segment.

Only raw byte trigrams are indexed. Document contents and positions are not
stored.

## API sketch

```rust
use grep_index::{FileMetadata, Index, Query};

let index = Index::create("corpus.index.redb")?;
index.upsert("src/lib.rs", contents, FileMetadata::default())?;

// The normal ripgrep-facing path: no metadata decoding.
let paths = index.candidate_paths(&Query::literal(b"needle"))?;

// Metadata is available when a caller wants post-index filtering/sorting.
let candidates = index.search_with(&Query::literal(b"needle"), |meta| {
    !meta.hidden && meta.size < 10_000_000
})?;
```

`IndexWriter` batches many files into one segment. `Index::index_directory`
walks a tree and commits bounded-size segments. The free `build_index`
function preserves the prototype's one-shot directory API and publishes its
initial database with a no-clobber hard link.

Logical compaction is for faster searches. A separate storage-compaction
method asks redb to reclaim unused pages when the caller can afford a slower
physical rewrite.

`Index::open_read_only` uses redb 4's read-only handle for multi-process
candidate readers. Multiple read-only handles can coexist, but redb does not
allow them to overlap a writable `Database` handle.

## Storage notes

Paths are stored as raw platform path bytes instead of UTF-8 strings. On
Unix this preserves non-UTF-8 paths; on Windows it stores UTF-16 code units.
DocIDs are assigned only after paths are sorted, which is the invariant
required by `fst::raw::Fst::get_key_into` for efficient reverse lookup.

Posting values have a count prefix followed by delta-varint encoded sorted
`u32` DocIDs. Query conjunctions decode the smallest lists first and use a
galloping intersection when list sizes are highly skewed. The implementation
is entirely safe Rust; mmap and custom unsafe codecs are intentionally left
for benchmark-driven follow-up work.

Files with a UTF-16 byte-order mark contribute trigrams from both their raw
bytes and ripgrep's default UTF-8 transcoding. This preserves conservative
candidate selection for both default searches and `--encoding none`; the
extra terms can only create false positives.

File metadata currently records size, the time at which the file was indexed,
optional modified/created/accessed timestamps, hidden, read-only, executable
and symlink flags. Metadata has a versioned binary encoding independent of
posting lists.

An index can also persist an absolute corpus root for resolving its stored
paths and a minimal set of absolute traversal scopes for ordinary-search
fallback. `build_index` sets both automatically; callers constructing an
`Index` directly can use `Index::set_root` and `Index::add_scopes`.

## Current boundaries

- `index_directory` is append/upsert-oriented. It updates files it sees, but
  does not infer deletes for files that disappeared from the tree; call
  `Index::delete` for explicit removals.
- Directory walking intentionally has only small built-in policies: it can
  skip binary files, hidden entries and `.git`, but it is not ripgrep's full
  ignore walker yet.
- Search currently materializes result vectors. The path-only API avoids
  metadata work, but a streaming visitor is a likely next step for enormous
  candidate sets.
- Logical compaction currently materializes its live remap and postings.
  A streaming, bounded-memory merge like the full Nakala plan is a future
  optimization.
- This crate uses redb 4.1 and Rust 1.97. Earlier experimental index formats
  are rebuild-only.

Dual-licensed under MIT or the [UNLICENSE](http://unlicense.org).

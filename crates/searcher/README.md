grep-searcher
-------------
A high level library for executing fast line oriented searches. This handles
things like reporting contextual lines, counting lines, inverting a search,
detecting binary data, automatic UTF-16 transcoding and deciding whether or not
to use memory maps.

[![Build status](https://github.com/BurntSushi/ripgrep/workflows/ci/badge.svg)](https://github.com/BurntSushi/ripgrep/actions)
[![](https://img.shields.io/crates/v/grep-searcher.svg)](https://crates.io/crates/grep-searcher)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).

### Documentation

[https://docs.rs/grep-searcher](https://docs.rs/grep-searcher)

**NOTE:** You probably don't want to use this crate directly. Instead, you
should prefer the facade defined in the
[`grep`](https://docs.rs/grep)
crate.


### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
grep-searcher = "0.1"
```

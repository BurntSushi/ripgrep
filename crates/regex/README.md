grep-regex
----------
The `grep-regex` crate provides an implementation of the `Matcher` trait from
the `grep-matcher` crate. This implementation permits Rust's regex engine to
be used in the `grep` crate for fast line oriented searching.

[![Build status](https://github.com/BurntSushi/ripgrep/workflows/ci/badge.svg)](https://github.com/BurntSushi/ripgrep/actions)
[![](https://img.shields.io/crates/v/grep-regex.svg)](https://crates.io/crates/grep-regex)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).

### Documentation

[https://docs.rs/grep-regex](https://docs.rs/grep-regex)

**NOTE:** You probably don't want to use this crate directly. Instead, you
should prefer the facade defined in the
[`grep`](https://docs.rs/grep)
crate.

### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
grep-regex = "0.1"
```

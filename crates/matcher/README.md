grep-matcher
------------
This crate provides a low level interface for describing regular expression
matchers. The `grep` crate uses this interface in order to make the regex
engine it uses pluggable.

[![Build status](https://github.com/BurntSushi/ripgrep/workflows/ci/badge.svg)](https://github.com/BurntSushi/ripgrep/actions)
[![](https://img.shields.io/crates/v/grep-matcher.svg)](https://crates.io/crates/grep-matcher)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).

### Documentation

[https://docs.rs/grep-matcher](https://docs.rs/grep-matcher)

**NOTE:** You probably don't want to use this crate directly. Instead, you
should prefer the facade defined in the
[`grep`](https://docs.rs/grep)
crate.


### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
grep-matcher = "0.1"
```

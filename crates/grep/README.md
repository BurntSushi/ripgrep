grep
----
ripgrep, as a library.

[![Build status](https://github.com/BurntSushi/ripgrep/workflows/ci/badge.svg)](https://github.com/BurntSushi/ripgrep/actions)
[![](https://img.shields.io/crates/v/grep.svg)](https://crates.io/crates/grep)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).


### Documentation

[https://docs.rs/grep](https://docs.rs/grep)

NOTE: This crate isn't ready for wide use yet. Ambitious individuals can
probably piece together the parts, but there is no high level documentation
describing how all of the pieces fit together.


### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
grep = "0.2"
```


### Features

This crate provides a `pcre2` feature (disabled by default) which, when
enabled, re-exports the `grep-pcre2` crate as an alternative `Matcher`
implementation to the standard `grep-regex` implementation.

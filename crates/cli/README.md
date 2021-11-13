grep-cli
--------
A utility library that provides common routines desired in search oriented
command line applications. This includes, but is not limited to, parsing hex
escapes, detecting whether stdin is readable and more. To the extent possible,
this crate strives for compatibility across Windows, macOS and Linux.

[![Build status](https://github.com/BurntSushi/ripgrep/workflows/ci/badge.svg)](https://github.com/BurntSushi/ripgrep/actions)
[![](https://img.shields.io/crates/v/grep-cli.svg)](https://crates.io/crates/grep-cli)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org/).


### Documentation

[https://docs.rs/grep-cli](https://docs.rs/grep-cli)

**NOTE:** You probably don't want to use this crate directly. Instead, you
should prefer the facade defined in the
[`grep`](https://docs.rs/grep)
crate.


### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
grep-cli = "0.1"
```

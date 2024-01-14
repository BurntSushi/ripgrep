ripgrep (rg)
------------
ripgrep is a line-oriented search tool that recursively searches the current
directory for a regex pattern. By default, ripgrep will respect gitignore rules
and automatically skip hidden files/directories and binary files. (To disable
all automatic filtering by default, use `rg -uuu`.) ripgrep has first class
support on Windows, macOS and Linux, with binary downloads available for [every
release](https://github.com/BurntSushi/ripgrep/releases). ripgrep is similar to
other popular search tools like The Silver Searcher, ack and grep.

[![Build status](https://github.com/BurntSushi/ripgrep/workflows/ci/badge.svg)](https://github.com/BurntSushi/ripgrep/actions)
[![Crates.io](https://img.shields.io/crates/v/ripgrep.svg)](https://crates.io/crates/ripgrep)
[![Packaging status](https://repology.org/badge/tiny-repos/ripgrep.svg)](https://repology.org/project/ripgrep/badges)

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org).


### CHANGELOG

Please see the [CHANGELOG](CHANGELOG.md) for a release history.

### Documentation quick links

* [Installation](#installation)
* [User Guide](GUIDE.md)
* [Frequently Asked Questions](FAQ.md)
* [Regex syntax](https://docs.rs/regex/1/regex/#syntax)
* [Configuration files](GUIDE.md#configuration-file)
* [Shell completions](FAQ.md#complete)
* [Building](#building)
* [Translations](#translations)


### Screenshot of search results

[![A screenshot of a sample search with ripgrep](https://burntsushi.net/stuff/ripgrep1.png)](https://burntsushi.net/stuff/ripgrep1.png)


### Quick examples comparing tools

This example searches the entire
[Linux kernel source tree](https://github.com/BurntSushi/linux)
(after running `make defconfig && make -j8`) for `[A-Z]+_SUSPEND`, where
all matches must be words. Timings were collected on a system with an Intel
i9-12900K 5.2 GHz.

Please remember that a single benchmark is never enough! See my
[blog post on ripgrep](https://blog.burntsushi.net/ripgrep/)
for a very detailed comparison with more benchmarks and analysis.

| Tool | Command | Line count | Time |
| ---- | ------- | ---------- | ---- |
| ripgrep (Unicode) | `rg -n -w '[A-Z]+_SUSPEND'` | 536 | **0.082s** (1.00x) |
| [hypergrep](https://github.com/p-ranav/hypergrep) | `hgrep -n -w '[A-Z]+_SUSPEND'` | 536 | 0.167s (2.04x) |
| [git grep](https://www.kernel.org/pub/software/scm/git/docs/git-grep.html) | `git grep -P -n -w '[A-Z]+_SUSPEND'` | 536 | 0.273s (3.34x) |
| [The Silver Searcher](https://github.com/ggreer/the_silver_searcher) | `ag -w '[A-Z]+_SUSPEND'` | 534 | 0.443s (5.43x) |
| [ugrep](https://github.com/Genivia/ugrep) | `ugrep -r --ignore-files --no-hidden -I -w '[A-Z]+_SUSPEND'` | 536 | 0.639s (7.82x) |
| [git grep](https://www.kernel.org/pub/software/scm/git/docs/git-grep.html) | `LC_ALL=C git grep -E -n -w '[A-Z]+_SUSPEND'` | 536 | 0.727s (8.91x) |
| [git grep (Unicode)](https://www.kernel.org/pub/software/scm/git/docs/git-grep.html) | `LC_ALL=en_US.UTF-8 git grep -E -n -w '[A-Z]+_SUSPEND'` | 536 | 2.670s (32.70x) |
| [ack](https://github.com/beyondgrep/ack3) | `ack -w '[A-Z]+_SUSPEND'` | 2677 | 2.935s (35.94x) |

Here's another benchmark on the same corpus as above that disregards gitignore
files and searches with a whitelist instead. The corpus is the same as in the
previous benchmark, and the flags passed to each command ensure that they are
doing equivalent work:

| Tool | Command | Line count | Time |
| ---- | ------- | ---------- | ---- |
| ripgrep | `rg -uuu -tc -n -w '[A-Z]+_SUSPEND'` | 447 | **0.063s** (1.00x) |
| [ugrep](https://github.com/Genivia/ugrep) | `ugrep -r -n --include='*.c' --include='*.h' -w '[A-Z]+_SUSPEND'` | 447 | 0.607s (9.62x) |
| [GNU grep](https://www.gnu.org/software/grep/) | `grep -E -r -n --include='*.c' --include='*.h' -w '[A-Z]+_SUSPEND'` | 447 | 0.674s (10.69x) |

Now we'll move to searching on single large file. Here is a straight-up
comparison between ripgrep, ugrep and GNU grep on a file cached in memory
(~13GB, [`OpenSubtitles.raw.en.gz`](http://opus.nlpl.eu/download.php?f=OpenSubtitles/v2018/mono/OpenSubtitles.raw.en.gz), decompressed):

| Tool | Command | Line count | Time |
| ---- | ------- | ---------- | ---- |
| ripgrep (Unicode) | `rg -w 'Sherlock [A-Z]\w+'` | 7882 | **1.042s** (1.00x) |
| [ugrep](https://github.com/Genivia/ugrep) | `ugrep -w 'Sherlock [A-Z]\w+'` | 7882 | 1.339s (1.28x) |
| [GNU grep (Unicode)](https://www.gnu.org/software/grep/) | `LC_ALL=en_US.UTF-8 egrep -w 'Sherlock [A-Z]\w+'` | 7882 | 6.577s (6.31x) |

In the above benchmark, passing the `-n` flag (for showing line numbers)
increases the times to `1.664s` for ripgrep and `9.484s` for GNU grep. ugrep
times are unaffected by the presence or absence of `-n`.

Beware of performance cliffs though:

| Tool | Command | Line count | Time |
| ---- | ------- | ---------- | ---- |
| ripgrep (Unicode) | `rg -w '[A-Z]\w+ Sherlock [A-Z]\w+'` | 485 | **1.053s** (1.00x) |
| [GNU grep (Unicode)](https://www.gnu.org/software/grep/) | `LC_ALL=en_US.UTF-8 grep -E -w '[A-Z]\w+ Sherlock [A-Z]\w+'` | 485 | 6.234s (5.92x) |
| [ugrep](https://github.com/Genivia/ugrep) | `ugrep -w '[A-Z]\w+ Sherlock [A-Z]\w+'` | 485 | 28.973s (27.51x) |

And performance can drop precipitously across the board when searching big
files for patterns without any opportunities for literal optimizations:

| Tool | Command | Line count | Time |
| ---- | ------- | ---------- | ---- |
| ripgrep | `rg '[A-Za-z]{30}'` | 6749 | **15.569s** (1.00x) |
| [ugrep](https://github.com/Genivia/ugrep) | `ugrep -w '[A-Z]\w+ Sherlock [A-Z]\w+'` | 6749 | 21.857s (1.40x) |
| [GNU grep](https://www.gnu.org/software/grep/) | `LC_ALL=C grep -E '[A-Za-z]{30}'` | 6749 | 32.409s (2.08x) |
| [GNU grep (Unicode)](https://www.gnu.org/software/grep/) | `LC_ALL=en_US.UTF-8 grep -E '[A-Za-z]{30}'` | 6795 | 8m30s (32.74x) |

Finally, high match counts also tend to both tank performance and smooth
out the differences between tools (because performance is dominated by how
quickly one can handle a match and not the algorithm used to detect the match,
generally speaking):

| Tool | Command | Line count | Time |
| ---- | ------- | ---------- | ---- |
| ripgrep | `rg the` | 83499915 | **6.948s** (1.00x) |
| [ugrep](https://github.com/Genivia/ugrep) | `ugrep the` | 83499915 | 11.721s (1.69x) |
| [GNU grep](https://www.gnu.org/software/grep/) | `LC_ALL=C grep the` | 83499915 | 15.217s (2.19x) |

### Why should I use ripgrep?

* It can replace many use cases served by other search tools
  because it contains most of their features and is generally faster. (See
  [the FAQ](FAQ.md#posix4ever) for more details on whether ripgrep can truly
  replace grep.)
* Like other tools specialized to code search, ripgrep defaults to
  [recursive search](GUIDE.md#recursive-search) and does [automatic
  filtering](GUIDE.md#automatic-filtering). Namely, ripgrep won't search files
  ignored by your `.gitignore`/`.ignore`/`.rgignore` files, it won't search
  hidden files and it won't search binary files. Automatic filtering can be
  disabled with `rg -uuu`.
* ripgrep can [search specific types of files](GUIDE.md#manual-filtering-file-types).
  For example, `rg -tpy foo` limits your search to Python files and `rg -Tjs
  foo` excludes JavaScript files from your search. ripgrep can be taught about
  new file types with custom matching rules.
* ripgrep supports many features found in `grep`, such as showing the context
  of search results, searching multiple patterns, highlighting matches with
  color and full Unicode support. Unlike GNU grep, ripgrep stays fast while
  supporting Unicode (which is always on).
* ripgrep has optional support for switching its regex engine to use PCRE2.
  Among other things, this makes it possible to use look-around and
  backreferences in your patterns, which are not supported in ripgrep's default
  regex engine. PCRE2 support can be enabled with `-P/--pcre2` (use PCRE2
  always) or `--auto-hybrid-regex` (use PCRE2 only if needed). An alternative
  syntax is provided via the `--engine (default|pcre2|auto-hybrid)` option.
* ripgrep has [rudimentary support for replacements](GUIDE.md#replacements),
  which permit rewriting output based on what was matched.
* ripgrep supports [searching files in text encodings](GUIDE.md#file-encoding)
  other than UTF-8, such as UTF-16, latin-1, GBK, EUC-JP, Shift_JIS and more.
  (Some support for automatically detecting UTF-16 is provided. Other text
  encodings must be specifically specified with the `-E/--encoding` flag.)
* ripgrep supports searching files compressed in a common format (brotli,
  bzip2, gzip, lz4, lzma, xz, or zstandard) with the `-z/--search-zip` flag.
* ripgrep supports
  [arbitrary input preprocessing filters](GUIDE.md#preprocessor)
  which could be PDF text extraction, less supported decompression, decrypting,
  automatic encoding detection and so on.
* ripgrep can be configured via a
  [configuration file](GUIDE.md#configuration-file).

In other words, use ripgrep if you like speed, filtering by default, fewer
bugs and Unicode support.


### Why shouldn't I use ripgrep?

Despite initially not wanting to add every feature under the sun to ripgrep,
over time, ripgrep has grown support for most features found in other file
searching tools. This includes searching for results spanning across multiple
lines, and opt-in support for PCRE2, which provides look-around and
backreference support.

At this point, the primary reasons not to use ripgrep probably consist of one
or more of the following:

* You need a portable and ubiquitous tool. While ripgrep works on Windows,
  macOS and Linux, it is not ubiquitous and it does not conform to any
  standard such as POSIX. The best tool for this job is good old grep.
* There still exists some other feature (or bug) not listed in this README that
  you rely on that's in another tool that isn't in ripgrep.
* There is a performance edge case where ripgrep doesn't do well where another
  tool does do well. (Please file a bug report!)
* ripgrep isn't possible to install on your machine or isn't available for your
  platform. (Please file a bug report!)


### Is it really faster than everything else?

Generally, yes. A large number of benchmarks with detailed analysis for each is
[available on my blog](https://blog.burntsushi.net/ripgrep/).

Summarizing, ripgrep is fast because:

* It is built on top of
  [Rust's regex engine](https://github.com/rust-lang/regex).
  Rust's regex engine uses finite automata, SIMD and aggressive literal
  optimizations to make searching very fast. (PCRE2 support can be opted into
  with the `-P/--pcre2` flag.)
* Rust's regex library maintains performance with full Unicode support by
  building UTF-8 decoding directly into its deterministic finite automaton
  engine.
* It supports searching with either memory maps or by searching incrementally
  with an intermediate buffer. The former is better for single files and the
  latter is better for large directories. ripgrep chooses the best searching
  strategy for you automatically.
* Applies your ignore patterns in `.gitignore` files using a
  [`RegexSet`](https://docs.rs/regex/1/regex/struct.RegexSet.html).
  That means a single file path can be matched against multiple glob patterns
  simultaneously.
* It uses a lock-free parallel recursive directory iterator, courtesy of
  [`crossbeam`](https://docs.rs/crossbeam) and
  [`ignore`](https://docs.rs/ignore).


### Feature comparison

Andy Lester, author of [ack](https://beyondgrep.com/), has published an
excellent table comparing the features of ack, ag, git-grep, GNU grep and
ripgrep: https://beyondgrep.com/feature-comparison/

Note that ripgrep has grown a few significant new features recently that
are not yet present in Andy's table. This includes, but is not limited to,
configuration files, passthru, support for searching compressed files,
multiline search and opt-in fancy regex support via PCRE2.


### Installation

The binary name for ripgrep is `rg`.

**[Archives of precompiled binaries for ripgrep are available for Windows,
macOS and Linux.](https://github.com/BurntSushi/ripgrep/releases)** Linux and
Windows binaries are static executables. Users of platforms not explicitly
mentioned below are advised to download one of these archives.

If you're a **macOS Homebrew** or a **Linuxbrew** user, then you can install
ripgrep from homebrew-core:

```
$ brew install ripgrep
```

If you're a **MacPorts** user, then you can install ripgrep from the
[official ports](https://www.macports.org/ports.php?by=name&substr=ripgrep):

```
$ sudo port install ripgrep
```

If you're a **Windows Chocolatey** user, then you can install ripgrep from the
[official repo](https://chocolatey.org/packages/ripgrep):

```
$ choco install ripgrep
```

If you're a **Windows Scoop** user, then you can install ripgrep from the
[official bucket](https://github.com/ScoopInstaller/Main/blob/master/bucket/ripgrep.json):

```
$ scoop install ripgrep
```

If you're a **Windows Winget** user, then you can install ripgrep from the
[winget-pkgs](https://github.com/microsoft/winget-pkgs/tree/master/manifests/b/BurntSushi/ripgrep)
repository:

```
$ winget install BurntSushi.ripgrep.MSVC
```

If you're an **Arch Linux** user, then you can install ripgrep from the official repos:

```
$ sudo pacman -S ripgrep
```

If you're a **Gentoo** user, you can install ripgrep from the
[official repo](https://packages.gentoo.org/packages/sys-apps/ripgrep):

```
$ sudo emerge sys-apps/ripgrep
```

If you're a **Fedora** user, you can install ripgrep from official
repositories.

```
$ sudo dnf install ripgrep
```

If you're an **openSUSE** user, ripgrep is included in **openSUSE Tumbleweed**
and **openSUSE Leap** since 15.1.

```
$ sudo zypper install ripgrep
```

If you're a **RHEL/CentOS 7/8** user, you can install ripgrep from
[copr](https://copr.fedorainfracloud.org/coprs/carlwgeorge/ripgrep/):

```
$ sudo yum install -y yum-utils
$ sudo yum-config-manager --add-repo=https://copr.fedorainfracloud.org/coprs/carlwgeorge/ripgrep/repo/epel-7/carlwgeorge-ripgrep-epel-7.repo
$ sudo yum install ripgrep
```

If you're a **Nix** user, you can install ripgrep from
[nixpkgs](https://github.com/NixOS/nixpkgs/blob/master/pkgs/tools/text/ripgrep/default.nix):

```
$ nix-env --install ripgrep
```

If you're a **Guix** user, you can install ripgrep from the official
package collection:

```
$ guix install ripgrep
```

If you're a **Debian** user (or a user of a Debian derivative like **Ubuntu**),
then ripgrep can be installed using a binary `.deb` file provided in each
[ripgrep release](https://github.com/BurntSushi/ripgrep/releases).

```
$ curl -LO https://github.com/BurntSushi/ripgrep/releases/download/13.0.0/ripgrep_13.0.0_amd64.deb
$ sudo dpkg -i ripgrep_13.0.0_amd64.deb
```

If you run Debian stable, ripgrep is [officially maintained by
Debian](https://tracker.debian.org/pkg/rust-ripgrep), although its version may
be older than the `deb` package available in the previous step.

```
$ sudo apt-get install ripgrep
```

If you're an **Ubuntu Cosmic (18.10)** (or newer) user, ripgrep is
[available](https://launchpad.net/ubuntu/+source/rust-ripgrep) using the same
packaging as Debian:

```
$ sudo apt-get install ripgrep
```

(N.B. Various snaps for ripgrep on Ubuntu are also available, but none of them
seem to work right and generate a number of very strange bug reports that I
don't know how to fix and don't have the time to fix. Therefore, it is no
longer a recommended installation option.)

If you're an **ALT** user, you can install ripgrep from the
[official repo](https://packages.altlinux.org/en/search?name=ripgrep):

```
$ sudo apt-get install ripgrep
```

If you're a **FreeBSD** user, then you can install ripgrep from the
[official ports](https://www.freshports.org/textproc/ripgrep/):

```
$ sudo pkg install ripgrep
```

If you're an **OpenBSD** user, then you can install ripgrep from the
[official ports](https://openports.se/textproc/ripgrep):

```
$ doas pkg_add ripgrep
```

If you're a **NetBSD** user, then you can install ripgrep from
[pkgsrc](https://pkgsrc.se/textproc/ripgrep):

```
$ sudo pkgin install ripgrep
```

If you're a **Haiku x86_64** user, then you can install ripgrep from the
[official ports](https://github.com/haikuports/haikuports/tree/master/sys-apps/ripgrep):

```
$ sudo pkgman install ripgrep
```

If you're a **Haiku x86_gcc2** user, then you can install ripgrep from the
same port as Haiku x86_64 using the x86 secondary architecture build:

```
$ sudo pkgman install ripgrep_x86
```

If you're a **Void Linux** user, then you can install ripgrep from the
[official repository](https://voidlinux.org/packages/?arch=x86_64&q=ripgrep):

```
$ sudo xbps-install -Syv ripgrep
```

If you're a **Rust programmer**, ripgrep can be installed with `cargo`.

* Note that the minimum supported version of Rust for ripgrep is **1.72.0**,
  although ripgrep may work with older versions.
* Note that the binary may be bigger than expected because it contains debug
  symbols. This is intentional. To remove debug symbols and therefore reduce
  the file size, run `strip` on the binary.

```
$ cargo install ripgrep
```

Alternatively, one can use [`cargo
binstall`](https://github.com/cargo-bins/cargo-binstall) to install a ripgrep
binary directly from GitHub:

```
$ cargo binstall ripgrep
```


### Building

ripgrep is written in Rust, so you'll need to grab a
[Rust installation](https://www.rust-lang.org/) in order to compile it.
ripgrep compiles with Rust 1.72.0 (stable) or newer. In general, ripgrep tracks
the latest stable release of the Rust compiler.

To build ripgrep:

```
$ git clone https://github.com/BurntSushi/ripgrep
$ cd ripgrep
$ cargo build --release
$ ./target/release/rg --version
0.1.3
```

If you have a Rust nightly compiler and a recent Intel CPU, then you can enable
additional optional SIMD acceleration like so:

```
RUSTFLAGS="-C target-cpu=native" cargo build --release --features 'simd-accel'
```

The `simd-accel` feature enables SIMD support in certain ripgrep dependencies
(responsible for transcoding). They are not necessary to get SIMD optimizations
for search; those are enabled automatically. Hopefully, some day, the
`simd-accel` feature will similarly become unnecessary. **WARNING:** Currently,
enabling this option can increase compilation times dramatically.

Finally, optional PCRE2 support can be built with ripgrep by enabling the
`pcre2` feature:

```
$ cargo build --release --features 'pcre2'
```

(Tip: use `--features 'pcre2 simd-accel'` to also include compile time SIMD
optimizations, which will only work with a nightly compiler.)

Enabling the PCRE2 feature works with a stable Rust compiler and will
attempt to automatically find and link with your system's PCRE2 library via
`pkg-config`. If one doesn't exist, then ripgrep will build PCRE2 from source
using your system's C compiler and then statically link it into the final
executable. Static linking can be forced even when there is an available PCRE2
system library by either building ripgrep with the MUSL target or by setting
`PCRE2_SYS_STATIC=1`.

ripgrep can be built with the MUSL target on Linux by first installing the MUSL
library on your system (consult your friendly neighborhood package manager).
Then you just need to add MUSL support to your Rust toolchain and rebuild
ripgrep, which yields a fully static executable:

```
$ rustup target add x86_64-unknown-linux-musl
$ cargo build --release --target x86_64-unknown-linux-musl
```

Applying the `--features` flag from above works as expected. If you want to
build a static executable with MUSL and with PCRE2, then you will need to have
`musl-gcc` installed, which might be in a separate package from the actual
MUSL library, depending on your Linux distribution.


### Running tests

ripgrep is relatively well-tested, including both unit tests and integration
tests. To run the full test suite, use:

```
$ cargo test --all
```

from the repository root.


### Related tools

* [delta](https://github.com/dandavison/delta) is a syntax highlighting
pager that supports the `rg --json` output format. So all you need to do to
make it work is `rg --json pattern | delta`. See [delta's manual section on
grep](https://dandavison.github.io/delta/grep.html) for more details.


### Vulnerability reporting

For reporting a security vulnerability, please
[contact Andrew Gallant](https://blog.burntsushi.net/about/).
The contact page has my email address and PGP public key if you wish to send an
encrypted message.


### Translations

The following is a list of known translations of ripgrep's documentation. These
are unofficially maintained and may not be up to date.

* [Chinese](https://github.com/chinanf-boy/ripgrep-zh#%E6%9B%B4%E6%96%B0-)
* [Spanish](https://github.com/UltiRequiem/traducciones/tree/master/ripgrep)

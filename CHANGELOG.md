TBD
===
Unreleased changes. Release notes have not yet been written.

**BREAKING CHANGES**

* `rg -C1 -A2` used to be equivalent to `rg -A2`, but now it is equivalent to
  `rg -B1 -A2`. That is, `-A` and `-B` no longer completely override `-C`.
  Instead, they only partially override `-C`.

Build process changes:

* ripgrep's shell completions and man page are now created by running ripgrep
with a new `--generate` flag. For example, `rg --generate man` will write a
man page in `roff` format on stdout. The release archives have not changed.
* The optional build dependency on `asciidoc` or `asciidoctor` has been
dropped. Previously, it was used to produce ripgrep's man page. ripgrep now
owns this process itself by writing `roff` directly.

Performance improvements:

* [PERF #1760](https://github.com/BurntSushi/ripgrep/issues/1760):
  Make most searches with `\b` look-arounds (among others) much faster.
* [PERF #2591](https://github.com/BurntSushi/ripgrep/pull/2591):
  Parallel directory traversal now uses work stealing for faster searches.
* [PERF #2642](https://github.com/BurntSushi/ripgrep/pull/2642):
  Parallel directory traversal has some contention reduced.

Feature enhancements:

* Added or improved file type filtering for Ada, DITA, Elixir, Fuchsia, Gentoo,
  Gradle, GraphQL, Markdown, Prolog, Raku, TypeScript, USD, V
* [FEATURE #1790](https://github.com/BurntSushi/ripgrep/issues/1790):
  Add new `--stop-on-nonmatch` flag.
* [FEATURE #1814](https://github.com/BurntSushi/ripgrep/issues/1814):
  Flags are now categorized in `-h/--help` output and ripgrep's man page.
* [FEATURE #2195](https://github.com/BurntSushi/ripgrep/issues/2195):
  When `extra-verbose` mode is enabled in zsh, show extra file type info.
* [FEATURE #2409](https://github.com/BurntSushi/ripgrep/pull/2409):
  Added installation instructions for `winget`.
* [FEATURE #2643](https://github.com/BurntSushi/ripgrep/issues/2643):
  Make `-d` a short flag for `--max-depth`.

Bug fixes:

* [BUG #884](https://github.com/BurntSushi/ripgrep/issues/884):
  Don't error when `-v/--invert-match` is used multiple times.
* [BUG #1275](https://github.com/BurntSushi/ripgrep/issues/1275):
  Fix bug with `\b` assertion in the regex engine.
* [BUG #1648](https://github.com/BurntSushi/ripgrep/issues/1648):
  Fix bug where sometimes short flags with values, e.g., `-M 900`, would fail.
* [BUG #1701](https://github.com/BurntSushi/ripgrep/issues/1701):
  Fix bug where some flags could not be repeated.
* [BUG #1757](https://github.com/BurntSushi/ripgrep/issues/1757):
  Fix bug when searching a sub-directory didn't have ignores applied correctly.
* [BUG #1891](https://github.com/BurntSushi/ripgrep/issues/1891):
  Fix bug when using `-w` with a regex that can match the empty string.
* [BUG #1911](https://github.com/BurntSushi/ripgrep/issues/1911):
  Disable mmap searching in all non-64-bit environments.
* [BUG #1966](https://github.com/BurntSushi/ripgrep/issues/1966):
  Fix bug where ripgrep can panic when printing to stderr.
* [BUG #2108](https://github.com/BurntSushi/ripgrep/issues/2108):
  Improve docs for `-r/--replace` syntax.
* [BUG #2198](https://github.com/BurntSushi/ripgrep/issues/2198):
  Fix bug where `--no-ignore-dot` would not ignore `.rgignore`.
* [BUG #2288](https://github.com/BurntSushi/ripgrep/issues/2288):
  `-A` and `-B` now only each partially override `-C`.
* [BUG #2236](https://github.com/BurntSushi/ripgrep/issues/2236):
  Fix gitignore parsing bug where a trailing `\/` resulted in an error.
* [BUG #2243](https://github.com/BurntSushi/ripgrep/issues/2243):
  Fix `--sort` flag for values other than `path`.
* [BUG #2381](https://github.com/BurntSushi/ripgrep/issues/2381):
  Make `-p/--pretty` override flags like `--no-line-number`.
* [BUG #2392](https://github.com/BurntSushi/ripgrep/issues/2392):
  Improve global git config parsing of the `excludesFile` field.
* [BUG #2480](https://github.com/BurntSushi/ripgrep/issues/2480):
  Fix bug when using inline regex flags with `-e/--regexp`.
* [BUG #2523](https://github.com/BurntSushi/ripgrep/issues/2523):
  Make executable searching take `.com` into account on Windows.
* [BUG #2574](https://github.com/BurntSushi/ripgrep/issues/2574):
  Fix bug in `-w/--word-regexp` that would result in incorrect match offsets.
* [BUG #2623](https://github.com/BurntSushi/ripgrep/issues/2623):
  Fix a number of bugs with the `-w/--word-regexp` flag.
* [BUG #2636](https://github.com/BurntSushi/ripgrep/pull/2636):
  Strip release binaries for macOS.


13.0.0 (2021-06-12)
===================
ripgrep 13 is a new major version release of ripgrep that primarily contains
bug fixes, some performance improvements and a few minor breaking changes.
There is also a fix for a security vulnerability on Windows
([CVE-2021-3013](https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2021-3013)).

Some highlights:

A new short flag, `-.`, has been added. It is an alias for the `--hidden` flag,
which instructs ripgrep to search hidden files and directories.

ripgrep is now using a new
[vectorized implementation of `memmem`](https://github.com/BurntSushi/memchr/pull/82),
which accelerates many common searches. If you notice any performance
regressions (or major improvements), I'd love to hear about them through an
issue report!

Also, for Windows users targeting MSVC, Cargo will now build fully static
executables of ripgrep. The release binaries for ripgrep 13 have been compiled
using this configuration.

**BREAKING CHANGES**:

**Binary detection output has changed slightly.**

In this release, a small tweak has been made to the output format when a binary
file is detected. Previously, it looked like this:

```
Binary file FOO matches (found "\0" byte around offset XXX)
```

Now it looks like this:

```
FOO: binary file matches (found "\0" byte around offset XXX)
```

**vimgrep output in multi-line now only prints the first line for each match.**

See [issue 1866](https://github.com/BurntSushi/ripgrep/issues/1866) for more
discussion on this. Previously, every line in a match was duplicated, even
when it spanned multiple lines. There are no changes to vimgrep output when
multi-line mode is disabled.

**In multi-line mode, --count is now equivalent to --count-matches.**

This appears to match how `pcre2grep` implements `--count`. Previously, ripgrep
would produce outright incorrect counts. Another alternative would be to simply
count the number of lines---even if it's more than the number of matches---but
that seems highly unintuitive.

**FULL LIST OF FIXES AND IMPROVEMENTS:**

Security fixes:

* [CVE-2021-3013](https://cve.mitre.org/cgi-bin/cvename.cgi?name=CVE-2021-3013):
  Fixes a security hole on Windows where running ripgrep with either the
  `-z/--search-zip` or `--pre` flags can result in running arbitrary
  executables from the current directory.
* [VULN #1773](https://github.com/BurntSushi/ripgrep/issues/1773):
  This is the public facing issue tracking CVE-2021-3013. ripgrep's README
  now contains a section describing how to report a vulnerability.

Performance improvements:

* [PERF #1657](https://github.com/BurntSushi/ripgrep/discussions/1657):
  Check if a file should be ignored first before issuing stat calls.
* [PERF memchr#82](https://github.com/BurntSushi/memchr/pull/82):
  ripgrep now uses a new vectorized implementation of `memmem`.

Feature enhancements:

* Added or improved file type filtering for ASP, Bazel, dvc, FlatBuffers,
  Futhark, minified files, Mint, pofiles (from GNU gettext) Racket, Red, Ruby,
  VCL, Yang.
* [FEATURE #1404](https://github.com/BurntSushi/ripgrep/pull/1404):
  ripgrep now prints a warning if nothing is searched.
* [FEATURE #1613](https://github.com/BurntSushi/ripgrep/pull/1613):
  Cargo will now produce static executables on Windows when using MSVC.
* [FEATURE #1680](https://github.com/BurntSushi/ripgrep/pull/1680):
  Add `-.` as a short flag alias for `--hidden`.
* [FEATURE #1842](https://github.com/BurntSushi/ripgrep/issues/1842):
  Add `--field-{context,match}-separator` for customizing field delimiters.
* [FEATURE #1856](https://github.com/BurntSushi/ripgrep/pull/1856):
  The README now links to a
  [Spanish translation](https://github.com/UltiRequiem/traducciones/tree/master/ripgrep).

Bug fixes:

* [BUG #1277](https://github.com/BurntSushi/ripgrep/issues/1277):
  Document cygwin path translation behavior in the FAQ.
* [BUG #1739](https://github.com/BurntSushi/ripgrep/issues/1739):
  Fix bug where replacements were buggy if the regex matched a line terminator.
* [BUG #1311](https://github.com/BurntSushi/ripgrep/issues/1311):
  Fix multi-line bug where a search & replace for `\n` didn't work as expected.
* [BUG #1401](https://github.com/BurntSushi/ripgrep/issues/1401):
  Fix buggy interaction between PCRE2 look-around and `-o/--only-matching`.
* [BUG #1412](https://github.com/BurntSushi/ripgrep/issues/1412):
  Fix multi-line bug with searches using look-around past matching lines.
* [BUG #1577](https://github.com/BurntSushi/ripgrep/issues/1577):
  Fish shell completions will continue to be auto-generated.
* [BUG #1642](https://github.com/BurntSushi/ripgrep/issues/1642):
  Fixes a bug where using `-m` and `-A` printed more matches than the limit.
* [BUG #1703](https://github.com/BurntSushi/ripgrep/issues/1703):
  Clarify the function of `-u/--unrestricted`.
* [BUG #1708](https://github.com/BurntSushi/ripgrep/issues/1708):
  Clarify how `-S/--smart-case` works.
* [BUG #1730](https://github.com/BurntSushi/ripgrep/issues/1730):
  Clarify that CLI invocation must always be valid, regardless of config file.
* [BUG #1741](https://github.com/BurntSushi/ripgrep/issues/1741):
  Fix stdin detection when using PowerShell in UNIX environments.
* [BUG #1756](https://github.com/BurntSushi/ripgrep/pull/1756):
  Fix bug where `foo/**` would match `foo`, but it shouldn't.
* [BUG #1765](https://github.com/BurntSushi/ripgrep/issues/1765):
  Fix panic when `--crlf` is used in some cases.
* [BUG #1638](https://github.com/BurntSushi/ripgrep/issues/1638):
  Correctly sniff UTF-8 and do transcoding, like we do for UTF-16.
* [BUG #1816](https://github.com/BurntSushi/ripgrep/issues/1816):
  Add documentation for glob alternate syntax, e.g., `{a,b,..}`.
* [BUG #1847](https://github.com/BurntSushi/ripgrep/issues/1847):
  Clarify how the `--hidden` flag works.
* [BUG #1866](https://github.com/BurntSushi/ripgrep/issues/1866#issuecomment-841635553):
  Fix bug when computing column numbers in `--vimgrep` mode.
* [BUG #1868](https://github.com/BurntSushi/ripgrep/issues/1868):
  Fix bug where `--passthru` and `-A/-B/-C` did not override each other.
* [BUG #1869](https://github.com/BurntSushi/ripgrep/pull/1869):
  Clarify docs for `--files-with-matches` and `--files-without-match`.
* [BUG #1878](https://github.com/BurntSushi/ripgrep/issues/1878):
  Fix bug where `\A` could produce unanchored matches in multiline search.
* [BUG 94e4b8e3](https://github.com/BurntSushi/ripgrep/commit/94e4b8e3):
  Fix column numbers with `--vimgrep` is used with `-U/--multiline`.


12.1.1 (2020-05-29)
===================
ripgrep 12.1.1 is a patch release that fixes a couple small bugs. In
particular, the ripgrep 12.1.0 release did not tag new releases for all of its
in-tree dependencies. As a result, ripgrep built dependencies from crates.io
would produce a different build than compiling ripgrep from source on the
`12.1.0` tag. Namely, some crates like `grep-cli` had unreleased changes.

Bug fixes:

* [BUG #1581](https://github.com/BurntSushi/ripgrep/issues/1581):
  Corrects some egregious markup output in `--help`.
* [BUG #1591](https://github.com/BurntSushi/ripgrep/issues/1591):
  Mention the special `$0` capture group in docs for the `-r/--replace` flag.
* [BUG #1602](https://github.com/BurntSushi/ripgrep/issues/1602):
  Fix failing test resulting from out-of-sync dependencies.


12.1.0 (2020-05-09)
===================
ripgrep 12.1.0 is a small minor version release that mostly includes bug fixes
and documentation improvements. This release also contains some important
notices for downstream packagers.

**Notices for downstream ripgrep package maintainers:**

* Fish shell completions will be removed in the ripgrep 13 release.
  See [#1577](https://github.com/BurntSushi/ripgrep/issues/1577)
  for more details.
* ripgrep has switched from `a2x` to `asciidoctor` to generate the man page.
  If `asciidoctor` is not present, then ripgrep will currently fall back to
  `a2x`. Support for `a2x` will be dropped in the ripgrep 13 release.
  See [#1544](https://github.com/BurntSushi/ripgrep/issues/1544)
  for more details.

Feature enhancements:

* [FEATURE #1547](https://github.com/BurntSushi/ripgrep/pull/1547):
  Support decompressing `.Z` files via `uncompress`.

Bug fixes:

* [BUG #1252](https://github.com/BurntSushi/ripgrep/issues/1252):
  Add a section on the `--pre` flag to the GUIDE.
* [BUG #1339](https://github.com/BurntSushi/ripgrep/issues/1339):
  Improve error message when a pattern with invalid UTF-8 is provided.
* [BUG #1524](https://github.com/BurntSushi/ripgrep/issues/1524):
  Note how to escape a `$` when using `--replace`.
* [BUG #1537](https://github.com/BurntSushi/ripgrep/issues/1537):
  Fix match bug caused by inner literal optimization.
* [BUG #1544](https://github.com/BurntSushi/ripgrep/issues/1544):
  ripgrep now uses `asciidoctor` instead of `a2x` to generate its man page.
* [BUG #1550](https://github.com/BurntSushi/ripgrep/issues/1550):
  Substantially reduce peak memory usage when searching wide directories.
* [BUG #1571](https://github.com/BurntSushi/ripgrep/issues/1571):
  Add note about configuration files in `--type-{add,clear}` docs.
* [BUG #1573](https://github.com/BurntSushi/ripgrep/issues/1573):
  Fix incorrect `--count-matches` output when using look-around.


12.0.1 (2020-03-29)
===================
ripgrep 12.0.1 is a small patch release that includes a minor bug fix relating
to superfluous error messages when searching git repositories with sub-modules.
This was a regression introduced in the 12.0.0 release.

Bug fixes:

* [BUG #1520](https://github.com/BurntSushi/ripgrep/issues/1520):
  Don't emit spurious error messages in git repositories with submodules.


12.0.0 (2020-03-15)
===================
ripgrep 12 is a new major version release of ripgrep that contains many bug
fixes, several important performance improvements and a few minor new features.

In a near future release, I am hoping to add an
[indexing feature](https://github.com/BurntSushi/ripgrep/issues/1497)
to ripgrep, which will dramatically speed up searching by building an index.
Feedback would very much be appreciated, especially on the user experience
which will be difficult to get right.

This release has no known breaking changes.

Deprecations:

* The `--no-pcre2-unicode` flag is deprecated. Instead, use the `--no-unicode`
  flag, which applies to both the default regex engine and PCRE2. For now,
  `--no-pcre2-unicode` and `--pcre2-unicode` are aliases to `--no-unicode`
  and `--unicode`, respectively. The `--[no-]pcre2-unicode` flags may be
  removed in a future release.
* The `--auto-hybrid-regex` flag is deprecated. Instead, use the new `--engine`
  flag with the `auto` value.

Performance improvements:

* [PERF #1087](https://github.com/BurntSushi/ripgrep/pull/1087):
  ripgrep is smarter when detected literals are whitespace.
* [PERF #1381](https://github.com/BurntSushi/ripgrep/pull/1381):
  Directory traversal is sped up with speculative ignore-file existence checks.
* [PERF cd8ec38a](https://github.com/BurntSushi/ripgrep/commit/cd8ec38a):
  Improve inner literal detection to cover more cases more effectively.
  e.g., ` +Sherlock Holmes +` now has ` Sherlock Holmes ` extracted instead
  of ` `.
* [PERF 6a0e0147](https://github.com/BurntSushi/ripgrep/commit/6a0e0147):
  Improve literal detection when the `-w/--word-regexp` flag is used.
* [PERF ad97e9c9](https://github.com/BurntSushi/ripgrep/commit/ad97e9c9):
  Improve overall performance of the `-w/--word-regexp` flag.

Feature enhancements:

* Added or improved file type filtering for erb, diff, Gradle, HAML, Org,
  Postscript, Skim, Slim, Slime, RPM Spec files, Typoscript, xml.
* [FEATURE #1370](https://github.com/BurntSushi/ripgrep/pull/1370):
  Add `--include-zero` flag that shows files searched without matches.
* [FEATURE #1390](https://github.com/BurntSushi/ripgrep/pull/1390):
  Add `--no-context-separator` flag that always hides context separators.
* [FEATURE #1414](https://github.com/BurntSushi/ripgrep/pull/1414):
  Add `--no-require-git` flag to allow ripgrep to respect gitignores anywhere.
* [FEATURE #1420](https://github.com/BurntSushi/ripgrep/pull/1420):
  Add `--no-ignore-exclude` to disregard rules in `.git/info/exclude` files.
* [FEATURE #1466](https://github.com/BurntSushi/ripgrep/pull/1466):
  Add `--no-ignore-files` flag to disable all `--ignore-file` flags.
* [FEATURE #1488](https://github.com/BurntSushi/ripgrep/pull/1488):
  Add '--engine' flag for easier switching between regex engines.
* [FEATURE 75cbe88f](https://github.com/BurntSushi/ripgrep/commit/75cbe88f):
  Add `--no-unicode` flag. This works on all supported regex engines.

Bug fixes:

* [BUG #1291](https://github.com/BurntSushi/ripgrep/issues/1291):
  ripgrep now works in non-existent directories.
* [BUG #1319](https://github.com/BurntSushi/ripgrep/issues/1319):
  Fix match bug due to errant literal detection.
* [**BUG #1335**](https://github.com/BurntSushi/ripgrep/issues/1335):
  Fixes a performance bug when searching plain text files with very long lines.
  This was a serious performance regression in some cases.
* [BUG #1344](https://github.com/BurntSushi/ripgrep/issues/1344):
  Document usage of `--type all`.
* [BUG #1389](https://github.com/BurntSushi/ripgrep/issues/1389):
  Fixes a bug where ripgrep would panic when searching a symlinked directory.
* [BUG #1439](https://github.com/BurntSushi/ripgrep/issues/1439):
  Improve documentation for ripgrep's automatic stdin detection.
* [BUG #1441](https://github.com/BurntSushi/ripgrep/issues/1441):
  Remove CPU features from man page.
* [BUG #1442](https://github.com/BurntSushi/ripgrep/issues/1442),
  [BUG #1478](https://github.com/BurntSushi/ripgrep/issues/1478):
  Improve documentation of the `-g/--glob` flag.
* [BUG #1445](https://github.com/BurntSushi/ripgrep/issues/1445):
  ripgrep now respects ignore rules from .git/info/exclude in worktrees.
* [BUG #1485](https://github.com/BurntSushi/ripgrep/issues/1485):
  Fish shell completions from the release Debian package are now installed to
  `/usr/share/fish/vendor_completions.d/rg.fish`.


11.0.2 (2019-08-01)
===================
ripgrep 11.0.2 is a new patch release that fixes a few bugs, including a
performance regression and a matching bug when using the `-F/--fixed-strings`
flag.

Feature enhancements:

* [FEATURE #1293](https://github.com/BurntSushi/ripgrep/issues/1293):
  Added `--glob-case-insensitive` flag that makes `--glob` behave as `--iglob`.

Bug fixes:

* [BUG #1246](https://github.com/BurntSushi/ripgrep/issues/1246):
  Add translations to README, starting with an unofficial Chinese translation.
* [BUG #1259](https://github.com/BurntSushi/ripgrep/issues/1259):
  Fix bug where the last byte of a `-f file` was stripped if it wasn't a `\n`.
* [BUG #1261](https://github.com/BurntSushi/ripgrep/issues/1261):
  Document that no error is reported when searching for `\n` with `-P/--pcre2`.
* [BUG #1284](https://github.com/BurntSushi/ripgrep/issues/1284):
  Mention `.ignore` and `.rgignore` more prominently in the README.
* [BUG #1292](https://github.com/BurntSushi/ripgrep/issues/1292):
  Fix bug where `--with-filename` was sometimes enabled incorrectly.
* [BUG #1268](https://github.com/BurntSushi/ripgrep/issues/1268):
  Fix major performance regression in GitHub `x86_64-linux` binary release.
* [BUG #1302](https://github.com/BurntSushi/ripgrep/issues/1302):
  Show better error messages when a non-existent preprocessor command is given.
* [BUG #1334](https://github.com/BurntSushi/ripgrep/issues/1334):
  Fix match regression with `-F` flag when patterns contain meta characters.


11.0.1 (2019-04-16)
===================
ripgrep 11.0.1 is a new patch release that fixes a search regression introduced
in the previous 11.0.0 release. In particular, ripgrep can enter an infinite
loop for some search patterns when searching invalid UTF-8.

Bug fixes:

* [BUG #1247](https://github.com/BurntSushi/ripgrep/issues/1247):
  Fix search bug that can cause ripgrep to enter an infinite loop.


11.0.0 (2019-04-15)
===================
ripgrep 11 is a new major version release of ripgrep that contains many bug
fixes, some performance improvements and a few feature enhancements. Notably,
ripgrep's user experience for binary file filtering has been improved. See the
[guide's new section on binary data](GUIDE.md#binary-data) for more details.

This release also marks a change in ripgrep's versioning. Where as the previous
version was `0.10.0`, this version is `11.0.0`. Moving forward, ripgrep's
major version will be increased a few times per year. ripgrep will continue to
be conservative with respect to backwards compatibility, but may occasionally
introduce breaking changes, which will always be documented in this CHANGELOG.
See [issue 1172](https://github.com/BurntSushi/ripgrep/issues/1172) for a bit
more detail on why this versioning change was made.

This release increases the **minimum supported Rust version** from 1.28.0 to
1.34.0.

**BREAKING CHANGES**:

* ripgrep has tweaked its exit status codes to be more like GNU grep's. Namely,
  if a non-fatal error occurs during a search, then ripgrep will now always
  emit a `2` exit status code, regardless of whether a match is found or not.
  Previously, ripgrep would only emit a `2` exit status code for a catastrophic
  error (e.g., regex syntax error). One exception to this is if ripgrep is run
  with `-q/--quiet`. In that case, if an error occurs and a match is found,
  then ripgrep will exit with a `0` exit status code.
* Supplying the `-u/--unrestricted` flag three times is now equivalent to
  supplying `--no-ignore --hidden --binary`. Previously, `-uuu` was equivalent
  to `--no-ignore --hidden --text`. The difference is that `--binary` disables
  binary file filtering without potentially dumping binary data into your
  terminal. That is, `rg -uuu foo` should now be equivalent to `grep -r foo`.
* The `avx-accel` feature of ripgrep has been removed since it is no longer
  necessary. All uses of AVX in ripgrep are now enabled automatically via
  runtime CPU feature detection. The `simd-accel` feature does remain available
  (only for enabling SIMD for transcoding), however, it does increase
  compilation times substantially at the moment.

Performance improvements:

* [PERF #497](https://github.com/BurntSushi/ripgrep/issues/497),
  [PERF #838](https://github.com/BurntSushi/ripgrep/issues/838):
  Make `rg -F -f dictionary-of-literals` much faster.

Feature enhancements:

* Added or improved file type filtering for Apache Thrift, ASP, Bazel, Brotli,
  BuildStream, bzip2, C, C++, Cython, gzip, Java, Make, Postscript, QML, Tex,
  XML, xz, zig and zstd.
* [FEATURE #855](https://github.com/BurntSushi/ripgrep/issues/855):
  Add `--binary` flag for disabling binary file filtering.
* [FEATURE #1078](https://github.com/BurntSushi/ripgrep/pull/1078):
  Add `--max-columns-preview` flag for showing a preview of long lines.
* [FEATURE #1099](https://github.com/BurntSushi/ripgrep/pull/1099):
  Add support for Brotli and Zstd to the `-z/--search-zip` flag.
* [FEATURE #1138](https://github.com/BurntSushi/ripgrep/pull/1138):
  Add `--no-ignore-dot` flag for ignoring `.ignore` files.
* [FEATURE #1155](https://github.com/BurntSushi/ripgrep/pull/1155):
  Add `--auto-hybrid-regex` flag for automatically falling back to PCRE2.
* [FEATURE #1159](https://github.com/BurntSushi/ripgrep/pull/1159):
  ripgrep's exit status logic should now match GNU grep. See updated man page.
* [FEATURE #1164](https://github.com/BurntSushi/ripgrep/pull/1164):
  Add `--ignore-file-case-insensitive` for case insensitive ignore globs.
* [FEATURE #1185](https://github.com/BurntSushi/ripgrep/pull/1185):
  Add `-I` flag as a short option for the `--no-filename` flag.
* [FEATURE #1207](https://github.com/BurntSushi/ripgrep/pull/1207):
  Add `none` value to `-E/--encoding` to forcefully disable all transcoding.
* [FEATURE da9d7204](https://github.com/BurntSushi/ripgrep/commit/da9d7204):
  Add `--pcre2-version` for querying showing PCRE2 version information.

Bug fixes:

* [BUG #306](https://github.com/BurntSushi/ripgrep/issues/306),
  [BUG #855](https://github.com/BurntSushi/ripgrep/issues/855):
  Improve the user experience for ripgrep's binary file filtering.
* [BUG #373](https://github.com/BurntSushi/ripgrep/issues/373),
  [BUG #1098](https://github.com/BurntSushi/ripgrep/issues/1098):
  `**` is now accepted as valid syntax anywhere in a glob.
* [BUG #916](https://github.com/BurntSushi/ripgrep/issues/916):
  ripgrep no longer hangs when searching `/proc` with a zombie process present.
* [BUG #1052](https://github.com/BurntSushi/ripgrep/issues/1052):
  Fix bug where ripgrep could panic when transcoding UTF-16 files.
* [BUG #1055](https://github.com/BurntSushi/ripgrep/issues/1055):
  Suggest `-U/--multiline` when a pattern contains a `\n`.
* [BUG #1063](https://github.com/BurntSushi/ripgrep/issues/1063):
  Always strip a BOM if it's present, even for UTF-8.
* [BUG #1064](https://github.com/BurntSushi/ripgrep/issues/1064):
  Fix inner literal detection that could lead to incorrect matches.
* [BUG #1079](https://github.com/BurntSushi/ripgrep/issues/1079):
  Fixes a bug where the order of globs could result in missing a match.
* [BUG #1089](https://github.com/BurntSushi/ripgrep/issues/1089):
  Fix another bug where ripgrep could panic when transcoding UTF-16 files.
* [BUG #1091](https://github.com/BurntSushi/ripgrep/issues/1091):
  Add note about inverted flags to the man page.
* [BUG #1093](https://github.com/BurntSushi/ripgrep/pull/1093):
  Fix handling of literal slashes in gitignore patterns.
* [BUG #1095](https://github.com/BurntSushi/ripgrep/issues/1095):
  Fix corner cases involving the `--crlf` flag.
* [BUG #1101](https://github.com/BurntSushi/ripgrep/issues/1101):
  Fix AsciiDoc escaping for man page output.
* [BUG #1103](https://github.com/BurntSushi/ripgrep/issues/1103):
  Clarify what `--encoding auto` does.
* [BUG #1106](https://github.com/BurntSushi/ripgrep/issues/1106):
  `--files-with-matches` and `--files-without-match` work with one file.
* [BUG #1121](https://github.com/BurntSushi/ripgrep/issues/1121):
  Fix bug that was triggering Windows antimalware when using the `--files`
  flag.
* [BUG #1125](https://github.com/BurntSushi/ripgrep/issues/1125),
  [BUG #1159](https://github.com/BurntSushi/ripgrep/issues/1159):
  ripgrep shouldn't panic for `rg -h | rg` and should emit correct exit status.
* [BUG #1144](https://github.com/BurntSushi/ripgrep/issues/1144):
  Fixes a bug where line numbers could be wrong on big-endian machines.
* [BUG #1154](https://github.com/BurntSushi/ripgrep/issues/1154):
  Windows files with "hidden" attribute are now treated as hidden.
* [BUG #1173](https://github.com/BurntSushi/ripgrep/issues/1173):
  Fix handling of `**` patterns in gitignore files.
* [BUG #1174](https://github.com/BurntSushi/ripgrep/issues/1174):
  Fix handling of repeated `**` patterns in gitignore files.
* [BUG #1176](https://github.com/BurntSushi/ripgrep/issues/1176):
  Fix bug where `-F`/`-x` weren't applied to patterns given via `-f`.
* [BUG #1189](https://github.com/BurntSushi/ripgrep/issues/1189):
  Document cases where ripgrep may use a lot of memory.
* [BUG #1203](https://github.com/BurntSushi/ripgrep/issues/1203):
  Fix a matching bug related to the suffix literal optimization.
* [BUG 8f14cb18](https://github.com/BurntSushi/ripgrep/commit/8f14cb18):
  Increase the default stack size for PCRE2's JIT.


0.10.0 (2018-09-07)
===================
This is a new minor version release of ripgrep that contains some major new
features, a huge number of bug fixes, and is the first release based on
libripgrep. The entirety of ripgrep's core search and printing code has been
rewritten and generalized so that anyone can make use of it.

Major new features include PCRE2 support, multi-line search and a JSON output
format.

**BREAKING CHANGES**:

* The minimum version required to compile Rust has now changed to track the
  latest stable version of Rust. Patch releases will continue to compile with
  the same version of Rust as the previous patch release, but new minor
  versions will use the current stable version of the Rust compile as its
  minimum supported version.
* The match semantics of `-w/--word-regexp` have changed slightly. They used
  to be `\b(?:<your pattern>)\b`, but now it's
  `(?:^|\W)(?:<your pattern>)(?:$|\W)`. This matches the behavior of GNU grep
  and is believed to be closer to the intended semantics of the flag. See
  [#389](https://github.com/BurntSushi/ripgrep/issues/389) for more details.

Feature enhancements:

* [FEATURE #162](https://github.com/BurntSushi/ripgrep/issues/162):
  libripgrep is now a thing. The primary crate is
  [`grep`](https://docs.rs/grep).
* [FEATURE #176](https://github.com/BurntSushi/ripgrep/issues/176):
  Add `-U/--multiline` flag that permits matching over multiple lines.
* [FEATURE #188](https://github.com/BurntSushi/ripgrep/issues/188):
  Add `-P/--pcre2` flag that gives support for look-around and backreferences.
* [FEATURE #244](https://github.com/BurntSushi/ripgrep/issues/244):
  Add `--json` flag that prints results in a JSON Lines format.
* [FEATURE #321](https://github.com/BurntSushi/ripgrep/issues/321):
  Add `--one-file-system` flag to skip directories on different file systems.
* [FEATURE #404](https://github.com/BurntSushi/ripgrep/issues/404):
  Add `--sort` and `--sortr` flag for more sorting. Deprecate `--sort-files`.
* [FEATURE #416](https://github.com/BurntSushi/ripgrep/issues/416):
  Add `--crlf` flag to permit `$` to work with carriage returns on Windows.
* [FEATURE #917](https://github.com/BurntSushi/ripgrep/issues/917):
  The `--trim` flag strips prefix whitespace from all lines printed.
* [FEATURE #993](https://github.com/BurntSushi/ripgrep/issues/993):
  Add `--null-data` flag, which makes ripgrep use NUL as a line terminator.
* [FEATURE #997](https://github.com/BurntSushi/ripgrep/issues/997):
  The `--passthru` flag now works with the `--replace` flag.
* [FEATURE #1038-1](https://github.com/BurntSushi/ripgrep/issues/1038):
  Add `--line-buffered` and `--block-buffered` for forcing a buffer strategy.
* [FEATURE #1038-2](https://github.com/BurntSushi/ripgrep/issues/1038):
  Add `--pre-glob` for filtering files through the `--pre` flag.

Bug fixes:

* [BUG #2](https://github.com/BurntSushi/ripgrep/issues/2):
  Searching with non-zero context can now use memory maps if appropriate.
* [BUG #200](https://github.com/BurntSushi/ripgrep/issues/200):
  ripgrep will now stop correctly when its output pipe is closed.
* [BUG #389](https://github.com/BurntSushi/ripgrep/issues/389):
  The `-w/--word-regexp` flag now works more intuitively.
* [BUG #643](https://github.com/BurntSushi/ripgrep/issues/643):
  Detection of readable stdin has improved on Windows.
* [BUG #441](https://github.com/BurntSushi/ripgrep/issues/441),
  [BUG #690](https://github.com/BurntSushi/ripgrep/issues/690),
  [BUG #980](https://github.com/BurntSushi/ripgrep/issues/980):
  Matching empty lines now works correctly in several corner cases.
* [BUG #764](https://github.com/BurntSushi/ripgrep/issues/764):
  Color escape sequences now coalesce, which reduces output size.
* [BUG #842](https://github.com/BurntSushi/ripgrep/issues/842):
  Add man page to binary Debian package.
* [BUG #922](https://github.com/BurntSushi/ripgrep/issues/922):
  ripgrep is now more robust with respect to memory maps failing.
* [BUG #937](https://github.com/BurntSushi/ripgrep/issues/937):
  Color escape sequences are no longer emitted for empty matches.
* [BUG #940](https://github.com/BurntSushi/ripgrep/issues/940):
  Context from the `--passthru` flag should not impact process exit status.
* [BUG #984](https://github.com/BurntSushi/ripgrep/issues/984):
  Fixes bug in `ignore` crate where first path was always treated as a symlink.
* [BUG #990](https://github.com/BurntSushi/ripgrep/issues/990):
  Read stderr asynchronously when running a process.
* [BUG #1013](https://github.com/BurntSushi/ripgrep/issues/1013):
  Add compile time and runtime CPU features to `--version` output.
* [BUG #1028](https://github.com/BurntSushi/ripgrep/pull/1028):
  Don't complete bare pattern after `-f` in zsh.


0.9.0 (2018-08-03)
==================
This is a new minor version release of ripgrep that contains some minor new
features and a panoply of bug fixes.

Releases provided on Github for `x86_64` will now work on all target CPUs, and
will also automatically take advantage of features found on modern CPUs (such
as AVX2) for additional optimizations.

This release increases the **minimum supported Rust version** from 1.20.0 to
1.23.0.

It is anticipated that the next release of ripgrep (0.10.0) will provide
multi-line search support and a JSON output format.

**BREAKING CHANGES**:

* When `--count` and `--only-matching` are provided simultaneously, the
  behavior of ripgrep is as if the `--count-matches` flag was given. That is,
  the total number of matches is reported, where there may be multiple matches
  per line. Previously, the behavior of ripgrep was to report the total number
  of matching lines. (Note that this behavior diverges from the behavior of
  GNU grep.)
* Octal syntax is no longer supported. ripgrep previously accepted expressions
  like `\1` as syntax for matching `U+0001`, but ripgrep will now report an
  error instead.
* The `--line-number-width` flag has been removed. Its functionality was not
  carefully considered with all ripgrep output formats.
  See [#795](https://github.com/BurntSushi/ripgrep/issues/795) for more
  details.

Feature enhancements:

* Added or improved file type filtering for Android, Bazel, Fuchsia, Haskell,
  Java and Puppet.
* [FEATURE #411](https://github.com/BurntSushi/ripgrep/issues/411):
  Add a `--stats` flag, which emits aggregate statistics after search results.
* [FEATURE #646](https://github.com/BurntSushi/ripgrep/issues/646):
  Add a `--no-ignore-messages` flag, which suppresses parse errors from reading
  `.ignore` and `.gitignore` files.
* [FEATURE #702](https://github.com/BurntSushi/ripgrep/issues/702):
  Support `\u{..}` Unicode escape sequences.
* [FEATURE #812](https://github.com/BurntSushi/ripgrep/issues/812):
  Add `-b/--byte-offset` flag that shows the byte offset of each matching line.
* [FEATURE #814](https://github.com/BurntSushi/ripgrep/issues/814):
  Add `--count-matches` flag, which is like `--count`, but for each match.
* [FEATURE #880](https://github.com/BurntSushi/ripgrep/issues/880):
  Add a `--no-column` flag, which disables column numbers in the output.
* [FEATURE #898](https://github.com/BurntSushi/ripgrep/issues/898):
  Add support for `lz4` when using the `-z/--search-zip` flag.
* [FEATURE #924](https://github.com/BurntSushi/ripgrep/issues/924):
  `termcolor` has moved to its own repository:
  https://github.com/BurntSushi/termcolor
* [FEATURE #934](https://github.com/BurntSushi/ripgrep/issues/934):
  Add a new flag, `--no-ignore-global`, that permits disabling global
  gitignores.
* [FEATURE #967](https://github.com/BurntSushi/ripgrep/issues/967):
  Rename `--maxdepth` to `--max-depth` for consistency. Keep `--maxdepth` for
  backwards compatibility.
* [FEATURE #978](https://github.com/BurntSushi/ripgrep/issues/978):
  Add a `--pre` option to filter inputs with an arbitrary program.
* [FEATURE fca9709d](https://github.com/BurntSushi/ripgrep/commit/fca9709d):
  Improve zsh completion.

Bug fixes:

* [BUG #135](https://github.com/BurntSushi/ripgrep/issues/135):
  Release portable binaries that conditionally use SSSE3, AVX2, etc., at
  runtime.
* [BUG #268](https://github.com/BurntSushi/ripgrep/issues/268):
  Print descriptive error message when trying to use look-around or
  backreferences.
* [BUG #395](https://github.com/BurntSushi/ripgrep/issues/395):
  Show comprehensible error messages for regexes like `\s*{`.
* [BUG #526](https://github.com/BurntSushi/ripgrep/issues/526):
  Support backslash escapes in globs.
* [BUG #795](https://github.com/BurntSushi/ripgrep/issues/795):
  Fix problems with `--line-number-width` by removing it.
* [BUG #832](https://github.com/BurntSushi/ripgrep/issues/832):
  Clarify usage instructions for `-f/--file` flag.
* [BUG #835](https://github.com/BurntSushi/ripgrep/issues/835):
  Fix small performance regression while crawling very large directory trees.
* [BUG #851](https://github.com/BurntSushi/ripgrep/issues/851):
  Fix `-S/--smart-case` detection once and for all.
* [BUG #852](https://github.com/BurntSushi/ripgrep/issues/852):
  Be robust with respect to `ENOMEM` errors returned by `mmap`.
* [BUG #853](https://github.com/BurntSushi/ripgrep/issues/853):
  Upgrade `grep` crate to `regex-syntax 0.6.0`.
* [BUG #893](https://github.com/BurntSushi/ripgrep/issues/893):
  Improve support for git submodules.
* [BUG #900](https://github.com/BurntSushi/ripgrep/issues/900):
  When no patterns are given, ripgrep should never match anything.
* [BUG #907](https://github.com/BurntSushi/ripgrep/issues/907):
  ripgrep will now stop traversing after the first file when `--quiet --files`
  is used.
* [BUG #918](https://github.com/BurntSushi/ripgrep/issues/918):
  Don't skip tar archives when `-z/--search-zip` is used.
* [BUG #934](https://github.com/BurntSushi/ripgrep/issues/934):
  Don't respect gitignore files when searching outside git repositories.
* [BUG #948](https://github.com/BurntSushi/ripgrep/issues/948):
  Use exit code 2 to indicate error, and use exit code 1 to indicate no
  matches.
* [BUG #951](https://github.com/BurntSushi/ripgrep/issues/951):
  Add stdin example to ripgrep usage documentation.
* [BUG #955](https://github.com/BurntSushi/ripgrep/issues/955):
  Use buffered writing when not printing to a tty, which fixes a performance
  regression.
* [BUG #957](https://github.com/BurntSushi/ripgrep/issues/957):
  Improve the error message shown for `--path separator /` in some Windows
  shells.
* [BUG #964](https://github.com/BurntSushi/ripgrep/issues/964):
  Add a `--no-fixed-strings` flag to disable `-F/--fixed-strings`.
* [BUG #988](https://github.com/BurntSushi/ripgrep/issues/988):
  Fix a bug in the `ignore` crate that prevented the use of explicit ignore
  files after disabling all other ignore rules.
* [BUG #995](https://github.com/BurntSushi/ripgrep/issues/995):
  Respect `$XDG_CONFIG_DIR/git/config` for detecting `core.excludesFile`.


0.8.1 (2018-02-20)
==================
This is a patch release of ripgrep that primarily fixes regressions introduced
in 0.8.0 (#820 and #824) in directory traversal on Windows. These regressions
do not impact non-Windows users.

Feature enhancements:

* Added or improved file type filtering for csv and VHDL.
* [FEATURE #798](https://github.com/BurntSushi/ripgrep/issues/798):
  Add `underline` support to `termcolor` and ripgrep. See documentation on the
  `--colors` flag for details.

Bug fixes:

* [BUG #684](https://github.com/BurntSushi/ripgrep/issues/684):
  Improve documentation for the `--ignore-file` flag.
* [BUG #789](https://github.com/BurntSushi/ripgrep/issues/789):
  Don't show `(rev )` if the revision wasn't available during the build.
* [BUG #791](https://github.com/BurntSushi/ripgrep/issues/791):
  Add man page to ARM release.
* [BUG #797](https://github.com/BurntSushi/ripgrep/issues/797):
  Improve documentation for "intense" setting in `termcolor`.
* [BUG #800](https://github.com/BurntSushi/ripgrep/issues/800):
  Fix a bug in the `ignore` crate for custom ignore files. This had no impact
  on ripgrep.
* [BUG #807](https://github.com/BurntSushi/ripgrep/issues/807):
  Fix a bug where `rg --hidden .` behaved differently from `rg --hidden ./`.
* [BUG #815](https://github.com/BurntSushi/ripgrep/issues/815):
  Clarify a common failure mode in user guide.
* [BUG #820](https://github.com/BurntSushi/ripgrep/issues/820):
  Fixes a bug on Windows where symlinks were followed even if not requested.
* [BUG #824](https://github.com/BurntSushi/ripgrep/issues/824):
  Fix a performance regression in directory traversal on Windows.


0.8.0 (2018-02-11)
==================
This is a new minor version release of ripgrep that satisfies several popular
feature requests (config files, search compressed files, true colors), fixes
many bugs and improves the quality of life for ripgrep maintainers. This
release also includes greatly improved documentation in the form of a
[User Guide](GUIDE.md) and a [FAQ](FAQ.md).

This release increases the **minimum supported Rust version** from 1.17 to
1.20.

**BREAKING CHANGES**:

Note that these are all very minor and unlikely to impact most users.

* In order to support configuration files, flag overrides needed to be
  rethought. In some cases, this changed ripgrep's behavior. For example,
  in ripgrep 0.7.1, `rg foo -s -i` will perform a case sensitive search
  since the `-s/--case-sensitive` flag was defined to always take precedence
  over the `-i/--ignore-case` flag, regardless of position. In ripgrep 0.8.0
  however, the override rule for all flags has changed to "the most recent
  flag wins among competing flags." That is, `rg foo -s -i` now performs a
  case insensitive search.
* The `-M/--max-columns` flag was tweaked so that specifying a value of `0`
  now makes ripgrep behave as if the flag was absent. This makes it possible
  to set a default value in a configuration file and then override it. The
  previous ripgrep behavior was to suppress all matching non-empty lines.
* In all globs, `[^...]` is now equivalent to `[!...]` (indicating class
  negation). Previously, `^` had no special significance in a character class.
* For **downstream packagers**, the directory hierarchy in ripgrep's archive
  releases has changed. The root directory now only contains the executable,
  README and license. There is now a new directory called `doc` which contains
  the man page (previously in the root), a user guide (new), a FAQ (new) and
  the CHANGELOG (previously not included in release). The `complete`
  directory remains the same.

Feature enhancements:

* Added or improved file type filtering for
  Apache Avro, C++, GN, Google Closure Templates, Jupyter notebooks, man pages,
  Protocol Buffers, Smarty and Web IDL.
* [FEATURE #196](https://github.com/BurntSushi/ripgrep/issues/196):
  Support a configuration file. See
  [the new user guide](GUIDE.md#configuration-file)
  for details.
* [FEATURE #261](https://github.com/BurntSushi/ripgrep/issues/261):
  Add extended or "true" color support. Works in Windows 10!
  [See the FAQ for details.](FAQ.md#colors)
* [FEATURE #539](https://github.com/BurntSushi/ripgrep/issues/539):
  Search gzip, bzip2, lzma or xz files when given `-z/--search-zip` flag.
* [FEATURE #544](https://github.com/BurntSushi/ripgrep/issues/544):
  Add support for line number alignment via a new `--line-number-width` flag.
* [FEATURE #654](https://github.com/BurntSushi/ripgrep/pull/654):
  Support linuxbrew in ripgrep's Brew tap.
* [FEATURE #673](https://github.com/BurntSushi/ripgrep/issues/673):
  Bring back `.rgignore` files. (A higher precedent, application specific
  version of `.ignore`.)
* [FEATURE #676](https://github.com/BurntSushi/ripgrep/issues/676):
  Provide ARM binaries. **WARNING:** This will be provided on a best effort
  basis.
* [FEATURE #709](https://github.com/BurntSushi/ripgrep/issues/709):
  Suggest `-F/--fixed-strings` flag on a regex syntax error.
* [FEATURE #740](https://github.com/BurntSushi/ripgrep/issues/740):
  Add a `--passthru` flag that causes ripgrep to print every line it reads.
* [FEATURE #785](https://github.com/BurntSushi/ripgrep/pull/785):
  Overhaul documentation. Cleaned up README, added user guide and FAQ.
* [FEATURE 7f5c07](https://github.com/BurntSushi/ripgrep/commit/7f5c07434be92103b5bf7e216b9c7494aed2d8cb):
  Add hidden flags for convenient overrides (e.g., `--no-text`).

Bug fixes:

* [BUG #553](https://github.com/BurntSushi/ripgrep/issues/553):
  Permit flags to be repeated.
* [BUG #633](https://github.com/BurntSushi/ripgrep/issues/633):
  Fix a bug where ripgrep would panic on Windows while following symlinks.
* [BUG #649](https://github.com/BurntSushi/ripgrep/issues/649):
  Fix handling of `!**/` in `.gitignore`.
* [BUG #663](https://github.com/BurntSushi/ripgrep/issues/663):
  **BREAKING CHANGE:** Support `[^...]` glob syntax (as identical to `[!...]`).
* [BUG #693](https://github.com/BurntSushi/ripgrep/issues/693):
  Don't display context separators when not printing matches.
* [BUG #705](https://github.com/BurntSushi/ripgrep/issues/705):
  Fix a bug that prevented ripgrep from searching OneDrive directories.
* [BUG #717](https://github.com/BurntSushi/ripgrep/issues/717):
  Improve `--smart-case` uppercase character detection.
* [BUG #725](https://github.com/BurntSushi/ripgrep/issues/725):
  Clarify that globs do not override explicitly given paths to search.
* [BUG #742](https://github.com/BurntSushi/ripgrep/pull/742):
  Write ANSI reset code as `\x1B[0m` instead of `\x1B[m`.
* [BUG #747](https://github.com/BurntSushi/ripgrep/issues/747):
  Remove `yarn.lock` from YAML file type.
* [BUG #760](https://github.com/BurntSushi/ripgrep/issues/760):
  ripgrep can now search `/sys/devices/system/cpu/vulnerabilities/*` files.
* [BUG #761](https://github.com/BurntSushi/ripgrep/issues/761):
  Fix handling of gitignore patterns that contain a `/`.
* [BUG #776](https://github.com/BurntSushi/ripgrep/pull/776):
  **BREAKING CHANGE:** `--max-columns=0` now disables the limit.
* [BUG #779](https://github.com/BurntSushi/ripgrep/issues/779):
  Clarify documentation for `--files-without-match`.
* [BUG #780](https://github.com/BurntSushi/ripgrep/issues/780),
  [BUG #781](https://github.com/BurntSushi/ripgrep/issues/781):
  Fix bug where ripgrep missed some matching lines.

Maintenance fixes:

* [MAINT #772](https://github.com/BurntSushi/ripgrep/pull/772):
  Drop `env_logger` in favor of simpler logger to avoid many new dependencies.
* [MAINT #772](https://github.com/BurntSushi/ripgrep/pull/772):
  Add git revision hash to ripgrep's version string.
* [MAINT #772](https://github.com/BurntSushi/ripgrep/pull/772):
  (Seemingly) improve compile times.
* [MAINT #776](https://github.com/BurntSushi/ripgrep/pull/776):
  Automatically generate man page during build.
* [MAINT #786](https://github.com/BurntSushi/ripgrep/pull/786):
  Remove use of `unsafe` in `globset`. :tada:
* [MAINT e9d448](https://github.com/BurntSushi/ripgrep/commit/e9d448e93bb4e1fb3b0c1afc29adb5af6ed5283d):
  Add an issue template (has already drastically improved bug reports).
* [MAINT ae2d03](https://github.com/BurntSushi/ripgrep/commit/ae2d036dd4ba2a46acac9c2d77c32e7c667eb850):
  Remove the `compile` script.

Friends of ripgrep:

I'd like to extend my gratitude to
[@balajisivaraman](https://github.com/balajisivaraman)
for their recent hard work in a number of areas, and in particular, for
implementing the "search compressed files" feature. Their work in sketching out
a specification for that and other work has been exemplary.

Thanks
[@balajisivaraman](https://github.com/balajisivaraman)!


0.7.1 (2017-10-22)
==================
This is a patch release of ripgrep that includes a fix to very bad regression
introduced in ripgrep 0.7.0.

Bug fixes:

* [BUG #648](https://github.com/BurntSushi/ripgrep/issues/648):
  Fix a bug where it was very easy to exceed standard file descriptor limits.


0.7.0 (2017-10-20)
==================
This is a new minor version release of ripgrep that includes mostly bug fixes.

ripgrep continues to require Rust 1.17, and there are no known breaking changes
introduced in this release.

Feature enhancements:

* Added or improved file type filtering for config & license files, Elm,
  Purescript, Standard ML, sh, systemd, Terraform
* [FEATURE #593](https://github.com/BurntSushi/ripgrep/pull/593):
  Using both `-o/--only-matching` and `-r/--replace` does the right thing.

Bug fixes:

* [BUG #200](https://github.com/BurntSushi/ripgrep/issues/200):
  ripgrep will stop when its pipe is closed.
* [BUG #402](https://github.com/BurntSushi/ripgrep/issues/402):
  Fix context printing bug when the `-m/--max-count` flag is used.
* [BUG #521](https://github.com/BurntSushi/ripgrep/issues/521):
  Fix interaction between `-r/--replace` and terminal colors.
* [BUG #559](https://github.com/BurntSushi/ripgrep/issues/559):
  Ignore test that tried reading a non-UTF-8 file path on macOS.
* [BUG #599](https://github.com/BurntSushi/ripgrep/issues/599):
  Fix color escapes on empty matches.
* [BUG #600](https://github.com/BurntSushi/ripgrep/issues/600):
  Avoid expensive (on Windows) file handle check when using --files.
* [BUG #618](https://github.com/BurntSushi/ripgrep/issues/618):
  Clarify installation instructions for Ubuntu users.
* [BUG #633](https://github.com/BurntSushi/ripgrep/issues/633):
  Faster symlink loop checking on Windows.


0.6.0 (2017-08-23)
==================
This is a new minor version release of ripgrep that includes many bug fixes
and a few new features such as `--iglob` and `-x/--line-regexp`.

Note that this release increases the minimum supported Rust version from 1.12
to 1.17.

Feature enhancements:

* Added or improved file type filtering for BitBake, C++, Cabal, cshtml, Julia,
  Make, msbuild, QMake, Yocto
* [FEATURE #163](https://github.com/BurntSushi/ripgrep/issues/163):
  Add an `--iglob` flag that is like `-g/--glob`, but matches globs
  case insensitively.
* [FEATURE #520](https://github.com/BurntSushi/ripgrep/pull/518):
  Add `-x/--line-regexp` flag, which requires a match to span an entire line.
* [FEATURE #551](https://github.com/BurntSushi/ripgrep/pull/551),
  [FEATURE #554](https://github.com/BurntSushi/ripgrep/pull/554):
  `ignore`: add new `matched_path_or_any_parents` method.

Bug fixes:

* [BUG #342](https://github.com/BurntSushi/ripgrep/issues/342):
  Fix invisible text in some PowerShell environments by changing the
  default color scheme on Windows.
* [BUG #413](https://github.com/BurntSushi/ripgrep/issues/413):
  Release binaries on Unix are now `strip`'d by default. This decreases
  binary size by an order of magnitude.
* [BUG #483](https://github.com/BurntSushi/ripgrep/issues/483):
  When `--quiet` is passed, `--files` should be quiet.
* [BUG #488](https://github.com/BurntSushi/ripgrep/pull/488):
  When `--vimgrep` is passed, `--with-filename` should be enabled
  automatically.
* [BUG #493](https://github.com/BurntSushi/ripgrep/issues/493):
  Fix another bug in the implementation of the `-o/--only-matching`
  flag.
* [BUG #499](https://github.com/BurntSushi/ripgrep/pull/499):
  Permit certain flags to override others.
* [BUG #523](https://github.com/BurntSushi/ripgrep/pull/523):
  `wincolor`: Re-fetch Windows console on all calls.
* [BUG #523](https://github.com/BurntSushi/ripgrep/issues/524):
  `--version` now shows enabled compile-time features.
* [BUG #532](https://github.com/BurntSushi/ripgrep/issues/532),
  [BUG #536](https://github.com/BurntSushi/ripgrep/pull/536),
  [BUG #538](https://github.com/BurntSushi/ripgrep/pull/538),
  [BUG #540](https://github.com/BurntSushi/ripgrep/pull/540),
  [BUG #560](https://github.com/BurntSushi/ripgrep/pull/560),
  [BUG #565](https://github.com/BurntSushi/ripgrep/pull/565):
  Improve zsh completion.
* [BUG #578](https://github.com/BurntSushi/ripgrep/pull/578):
  Enable SIMD for `encoding_rs` when appropriate.
* [BUG #580](https://github.com/BurntSushi/ripgrep/issues/580):
  Fix `-w/--word-regexp` in the presence of capturing groups.
* [BUG #581](https://github.com/BurntSushi/ripgrep/issues/581):
  Document that ripgrep may terminate unexpectedly when searching via
  memory maps (which can happen using default settings).

Friends of ripgrep:

I'd like to give a big Thank You to @okdana for their recent hard work on
ripgrep. This includes new features like `--line-regexp`, heroic effort on
zsh auto-completion and thinking through some thorny argv issues with me.

I'd also like to thank @ericbn for their work on improving ripgrep's argv
parsing by allowing some flags to override others.

Thanks @okdana and @ericbn!


0.5.2 (2017-05-11)
==================
Feature enhancements:

* Added or improved file type filtering for Nix.
* [FEATURE #362](https://github.com/BurntSushi/ripgrep/issues/362):
  Add `--regex-size-limit` and `--dfa-size-limit` flags.
* [FEATURE #444](https://github.com/BurntSushi/ripgrep/issues/444):
  Improve error messages for invalid globs.

Bug fixes:

* [BUG #442](https://github.com/BurntSushi/ripgrep/issues/442):
  Fix line wrapping in `--help` output.
* [BUG #451](https://github.com/BurntSushi/ripgrep/issues/451):
  Fix bug with duplicate output when using `-o/--only-matching` flag.


0.5.1 (2017-04-09)
==================
Feature enhancements:

* Added or improved file type filtering for vim.
* [FEATURE #34](https://github.com/BurntSushi/ripgrep/issues/34):
  Add a `-o/--only-matching` flag.
* [FEATURE #377](https://github.com/BurntSushi/ripgrep/issues/377):
  Column numbers can now be customized with a color. (The default is
  no color.)
* [FEATURE #419](https://github.com/BurntSushi/ripgrep/issues/419):
  Added `-0` short flag option for `--null`.

Bug fixes:

* [BUG #381](https://github.com/BurntSushi/ripgrep/issues/381):
  Include license text in all subcrates.
* [BUG #418](https://github.com/BurntSushi/ripgrep/issues/418),
  [BUG #426](https://github.com/BurntSushi/ripgrep/issues/426),
  [BUG #439](https://github.com/BurntSushi/ripgrep/issues/439):
  Fix a few bugs with `-h/--help` output.


0.5.0 (2017-03-12)
==================
This is a new minor version release of ripgrep that includes one minor breaking
change, bug fixes and several new features including support for text encodings
other than UTF-8.

A notable accomplishment with respect to Rust is that ripgrep proper now only
contains a single `unsafe` use (for accessing the contents of a memory map).

The **breaking change** is:

* [FEATURE #380](https://github.com/BurntSushi/ripgrep/issues/380):
  Line numbers are now hidden by default when ripgrep is printing to a tty
  **and** the only thing searched is stdin.

Feature enhancements:

* Added or improved file type filtering for Ceylon, CSS, Elixir, HTML, log,
  SASS, SVG, Twig
* [FEATURE #1](https://github.com/BurntSushi/ripgrep/issues/1):
  Add support for additional text encodings, including automatic detection for
  UTF-16 via BOM sniffing. Explicit text encoding support with the
  `-E/--encoding` flag was also added for latin-1, GBK, EUC-JP
  and Shift_JIS, among others. The full list can be found here:
  https://encoding.spec.whatwg.org/#concept-encoding-get
* [FEATURE #129](https://github.com/BurntSushi/ripgrep/issues/129):
  Add a new `-M/--max-columns` flag that omits lines longer than the given
  number of bytes. (Disabled by default!)
* [FEATURE #369](https://github.com/BurntSushi/ripgrep/issues/369):
  A new flag, `--max-filesize`, was added for limiting searches to files with
  a maximum file size.

Bug fixes:

* [BUG #52](https://github.com/BurntSushi/ripgrep/issues/52),
  [BUG #311](https://github.com/BurntSushi/ripgrep/issues/311):
  Tweak how binary files are detected and handled. (We are slightly less
  conservative and will no longer use memory without bound.)
* [BUG #326](https://github.com/BurntSushi/ripgrep/issues/326):
  When --files flag is given, we should never attempt to parse positional
  arguments as regexes.
* [BUG #327](https://github.com/BurntSushi/ripgrep/issues/327):
  Permit the --heading flag to override the --no-heading flag.
* [BUG #340](https://github.com/BurntSushi/ripgrep/pull/340):
  Clarify that the `-u/--unrestricted` flags are aliases.
* [BUG #343](https://github.com/BurntSushi/ripgrep/pull/343):
  Global git ignore config should use `$HOME/.config/git/ignore` and not
  `$HOME/git/ignore`.
* [BUG #345](https://github.com/BurntSushi/ripgrep/pull/345):
  Clarify docs for `-g/--glob` flag.
* [BUG #381](https://github.com/BurntSushi/ripgrep/issues/381):
  Add license files to each sub-crate.
* [BUG #383](https://github.com/BurntSushi/ripgrep/issues/383):
  Use latest version of clap (for argv parsing).
* [BUG #392](https://github.com/BurntSushi/ripgrep/issues/391):
  Fix translation of set globs (e.g., `{foo,bar,quux}`) to regexes.
* [BUG #401](https://github.com/BurntSushi/ripgrep/pull/401):
  Add PowerShell completion file to Windows release.
* [BUG #405](https://github.com/BurntSushi/ripgrep/issues/405):
  Fix bug when excluding absolute paths with the `-g/--glob` flag.


0.4.0
=====
This is a new minor version release of ripgrep that includes a couple very
minor breaking changes, a few new features and lots of bug fixes.

This version of ripgrep upgrades its `regex` dependency from `0.1` to `0.2`,
which includes a few minor syntax changes:

* POSIX character classes now require double bracketing. Previously, the regex
  `[:upper:]` would parse as the `upper` POSIX character class. Now it parses
  as the character class containing the characters `:upper:`. The fix to this
  change is to use `[[:upper:]]` instead. Note that variants like
  `[[:upper:][:blank:]]` continue to work.
* The character `[` must always be escaped inside a character class.
* The characters `&`, `-` and `~` must be escaped if any one of them are
  repeated consecutively. For example, `[&]`, `[\&]`, `[\&\&]`, `[&-&]` are all
  equivalent while `[&&]` is illegal. (The motivation for this and the prior
  change is to provide a backwards compatible path for adding character class
  set notation.)

Feature enhancements:

* Added or improved file type filtering for Crystal, Kotlin, Perl, PowerShell,
  Ruby, Swig
* [FEATURE #83](https://github.com/BurntSushi/ripgrep/issues/83):
  Type definitions can now include other type definitions.
* [FEATURE #243](https://github.com/BurntSushi/ripgrep/issues/243):
  **BREAKING CHANGE**: The `--column` flag now implies `--line-number`.
* [FEATURE #263](https://github.com/BurntSushi/ripgrep/issues/263):
  Add a new `--sort-files` flag.
* [FEATURE #275](https://github.com/BurntSushi/ripgrep/issues/275):
  Add a new `--path-separator` flag. Useful in cygwin.

Bug fixes:

* [BUG #182](https://github.com/BurntSushi/ripgrep/issues/182):
  Redux: use more portable ANSI color escape sequences when possible.
* [BUG #258](https://github.com/BurntSushi/ripgrep/issues/258):
  Fix bug that caused ripgrep's parallel iterator to spin and burn CPU.
* [BUG #262](https://github.com/BurntSushi/ripgrep/issues/262):
  Document how to install shell completion files.
* [BUG #266](https://github.com/BurntSushi/ripgrep/issues/266),
  [BUG #293](https://github.com/BurntSushi/ripgrep/issues/293):
  Fix handling of bold styling and change the default colors.
* [BUG #268](https://github.com/BurntSushi/ripgrep/issues/268):
  Make lack of backreference support more explicit.
* [BUG #271](https://github.com/BurntSushi/ripgrep/issues/271):
  Remove `~` dependency on clap.
* [BUG #277](https://github.com/BurntSushi/ripgrep/issues/277):
  Fix cosmetic issue in `globset` crate docs.
* [BUG #279](https://github.com/BurntSushi/ripgrep/issues/279):
  ripgrep did not terminate when `-q/--quiet` was given.
* [BUG #281](https://github.com/BurntSushi/ripgrep/issues/281):
  **BREAKING CHANGE**: Completely remove `^C` handling from ripgrep.
* [BUG #284](https://github.com/BurntSushi/ripgrep/issues/284):
  Make docs for `-g/--glob` clearer.
* [BUG #286](https://github.com/BurntSushi/ripgrep/pull/286):
  When stdout is redirected to a file, don't search that file.
* [BUG #287](https://github.com/BurntSushi/ripgrep/pull/287):
  Fix ZSH completions.
* [BUG #295](https://github.com/BurntSushi/ripgrep/pull/295):
  Remove superfluous `memmap` dependency in `grep` crate.
* [BUG #308](https://github.com/BurntSushi/ripgrep/pull/308):
  Improve docs for `-r/--replace`.
* [BUG #313](https://github.com/BurntSushi/ripgrep/pull/313):
  Update bytecount dep to latest version.
* [BUG #318](https://github.com/BurntSushi/ripgrep/pull/318):
  Fix invalid UTF-8 output bug in Windows consoles.


0.3.2
=====
Feature enhancements:

* Added or improved file type filtering for Less, Sass, stylus, Zsh

Bug fixes:

* [BUG #229](https://github.com/BurntSushi/ripgrep/issues/229):
  Make smart case slightly less conservative.
* [BUG #247](https://github.com/BurntSushi/ripgrep/issues/247):
  Clarify use of --heading/--no-heading.
* [BUG #251](https://github.com/BurntSushi/ripgrep/issues/251),
  [BUG #264](https://github.com/BurntSushi/ripgrep/issues/264),
  [BUG #267](https://github.com/BurntSushi/ripgrep/issues/267):
  Fix matching bug caused by literal optimizations.
* [BUG #256](https://github.com/BurntSushi/ripgrep/issues/256):
  Fix bug that caused `rg foo` and `rg foo/` to have different behavior
  when `foo` was a symlink.
* [BUG #270](https://github.com/BurntSushi/ripgrep/issues/270):
  Fix bug where patterns starting with a `-` couldn't be used with the
  `-e/--regexp` flag. (This resolves a regression that was introduced in
  ripgrep 0.3.0.)


0.3.1
=====
Bug fixes:

* [BUG #242](https://github.com/BurntSushi/ripgrep/issues/242):
  ripgrep didn't respect `--colors foo:none` correctly. Now it does.


0.3.0
=====
This is a new minor version release of ripgrep that includes two breaking
changes with lots of bug fixes and some new features and performance
improvements. Notably, if you had a problem with colors or piping on Windows
before, then that should now be fixed in this release.

**BREAKING CHANGES**:

* ripgrep now requires Rust 1.11 to compile. Previously, it could build on
  Rust 1.9. The cause of this was the move from
  [Docopt to Clap](https://github.com/BurntSushi/ripgrep/pull/233)
  for argument parsing.
* The `-e/--regexp` flag can no longer accept a pattern starting with a `-`.
  There are two work-arounds: `rg -- -foo` and `rg [-]foo` or `rg -e [-]foo`
  will all search for the same `-foo` pattern. The cause of this was the move
  from [Docopt to Clap](https://github.com/BurntSushi/ripgrep/pull/233)
  for argument parsing.
  [This may get fixed in the
  future.](https://github.com/kbknapp/clap-rs/issues/742).

Performance improvements:

* [PERF #33](https://github.com/BurntSushi/ripgrep/issues/33):
  ripgrep now performs similar to GNU grep on small corpora.
* [PERF #136](https://github.com/BurntSushi/ripgrep/issues/136):
  ripgrep no longer slows down because of argument parsing when given a large
  argument list.

Feature enhancements:

* Added or improved file type filtering for Elixir.
* [FEATURE #7](https://github.com/BurntSushi/ripgrep/issues/7):
  Add a `-f/--file` flag that causes ripgrep to read patterns from a file.
* [FEATURE #51](https://github.com/BurntSushi/ripgrep/issues/51):
  Add a `--colors` flag that enables one to customize the colors used in
  ripgrep's output.
* [FEATURE #138](https://github.com/BurntSushi/ripgrep/issues/138):
  Add a `--files-without-match` flag that shows only file paths that contain
  zero matches.
* [FEATURE #230](https://github.com/BurntSushi/ripgrep/issues/230):
  Add completion files to the release (Bash, Fish and PowerShell).

Bug fixes:

* [BUG #37](https://github.com/BurntSushi/ripgrep/issues/37):
  Use correct ANSI escape sequences when `TERM=screen.linux`.
* [BUG #94](https://github.com/BurntSushi/ripgrep/issues/94):
  ripgrep now detects stdin on Windows automatically.
* [BUG #117](https://github.com/BurntSushi/ripgrep/issues/117):
  Colors should now work correctly and automatically inside mintty.
* [BUG #182](https://github.com/BurntSushi/ripgrep/issues/182):
  Colors should now work within Emacs. In particular, `--color=always` will
  emit colors regardless of the current environment.
* [BUG #189](https://github.com/BurntSushi/ripgrep/issues/189):
  Show less content when running `rg -h`. The full help content can be
  accessed with `rg --help`.
* [BUG #210](https://github.com/BurntSushi/ripgrep/issues/210):
  Support non-UTF-8 file names on Unix platforms.
* [BUG #231](https://github.com/BurntSushi/ripgrep/issues/231):
  Switch from block buffering to line buffering.
* [BUG #241](https://github.com/BurntSushi/ripgrep/issues/241):
  Some error messages weren't suppressed when `--no-messages` was used.


0.2.9
=====
Bug fixes:

* [BUG #226](https://github.com/BurntSushi/ripgrep/issues/226):
  File paths explicitly given on the command line weren't searched in parallel.
  (This was a regression in `0.2.7`.)
* [BUG #228](https://github.com/BurntSushi/ripgrep/issues/228):
  If a directory was given to `--ignore-file`, ripgrep's memory usage would
  grow without bound.


0.2.8
=====
Bug fixes:

* Fixed a bug with the SIMD/AVX features for using bytecount in commit
  `4ca15a`.


0.2.7
=====
Performance improvements:

* [PERF #223](https://github.com/BurntSushi/ripgrep/pull/223):
  Added a parallel recursive directory iterator. This results in major
  performance improvements on large repositories.
* [PERF #11](https://github.com/BurntSushi/ripgrep/pull/11):
  ripgrep now uses the `bytecount` library for counting new lines. In some
  cases, ripgrep runs twice as fast. Use
  `RUSTFLAGS="-C target-cpu=native" cargo build --release --features 'simd-accel avx-accel'`
  to get the fastest possible binary.

Feature enhancements:

* Added or improved file type filtering for Agda, Tex, Taskpaper, Markdown,
  asciidoc, textile, rdoc, org, creole, wiki, pod, C#, PDF, C, C++.
* [FEATURE #149](https://github.com/BurntSushi/ripgrep/issues/149):
  Add a new `--no-messages` flag that suppresses error messages.
  Note that `rg foo 2> /dev/null` also works.
* [FEATURE #159](https://github.com/BurntSushi/ripgrep/issues/159):
  Add a new `-m/--max-count` flag that limits the total number of matches
  printed for each file searched.

Bug fixes:

* [BUG #199](https://github.com/BurntSushi/ripgrep/issues/199):
  Fixed a bug where `-S/--smart-case` wasn't being applied correctly to
  literal optimizations.
* [BUG #203](https://github.com/BurntSushi/ripgrep/issues/203):
  Mention the full name, ripgrep, in more places. It now appears in
  the output of `--help` and `--version`. The repository URL is now also
  in the output of `--help` and the man page.
* [BUG #215](https://github.com/BurntSushi/ripgrep/issues/215):
  Include small note about how to search for a pattern that starts with a `-`.


0.2.6
=====
Feature enhancements:

* Added or improved file type filtering for Fish.

Bug fixes:

* [BUG #206](https://github.com/BurntSushi/ripgrep/issues/206):
  Fixed a regression with `-g/--glob` flag in `0.2.5`.


0.2.5
=====
Feature enhancements:

* Added or improved file type filtering for Groovy, Handlebars, Tcl, zsh and
  Python.
* [FEATURE #9](https://github.com/BurntSushi/ripgrep/issues/9):
  Support global gitignore config and `.git/info/exclude` files.
* [FEATURE #45](https://github.com/BurntSushi/ripgrep/issues/45):
  Add --ignore-file flag for specifying additional ignore files.
* [FEATURE #202](https://github.com/BurntSushi/ripgrep/pull/202):
  Introduce a new
  [`ignore`](https://github.com/BurntSushi/ripgrep/tree/master/ignore)
  crate that encapsulates all of ripgrep's gitignore matching logic.

Bug fixes:

* [BUG #44](https://github.com/BurntSushi/ripgrep/issues/44):
  ripgrep runs slowly when given lots of positional arguments that are
  directories.
* [BUG #119](https://github.com/BurntSushi/ripgrep/issues/119):
  ripgrep didn't reset terminal colors if it was interrupted by `^C`.
  Fixed in [PR #187](https://github.com/BurntSushi/ripgrep/pull/187).
* [BUG #184](https://github.com/BurntSushi/ripgrep/issues/184):
  Fixed a bug related to interpreting gitignore files in parent directories.


0.2.4
=====
SKIPPED.


0.2.3
=====
Bug fixes:

* [BUG #164](https://github.com/BurntSushi/ripgrep/issues/164):
  Fixes a segfault on macos builds.
* [BUG #167](https://github.com/BurntSushi/ripgrep/issues/167):
  Clarify documentation for --threads.


0.2.2
=====
Packaging updates:

* `ripgrep` is now in homebrew-core. `brew install ripgrep` will do the trick
  on a Mac.
* `ripgrep` is now in the Archlinux community repository.
  `pacman -S ripgrep` will do the trick on Archlinux.
* Support has been discontinued for i686-darwin.
* Glob matching has been moved out into its own crate:
  [`globset`](https://crates.io/crates/globset).

Feature enhancements:

* Added or improved file type filtering for CMake, config, Jinja, Markdown,
  Spark.
* [FEATURE #109](https://github.com/BurntSushi/ripgrep/issues/109):
  Add a --max-depth flag for directory traversal.
* [FEATURE #124](https://github.com/BurntSushi/ripgrep/issues/124):
  Add -s/--case-sensitive flag. Overrides --smart-case.
* [FEATURE #139](https://github.com/BurntSushi/ripgrep/pull/139):
  The `ripgrep` repo is now a Homebrew tap. This is useful for installing
  SIMD accelerated binaries, which aren't available in homebrew-core.

Bug fixes:

* [BUG #87](https://github.com/BurntSushi/ripgrep/issues/87),
  [BUG #127](https://github.com/BurntSushi/ripgrep/issues/127),
  [BUG #131](https://github.com/BurntSushi/ripgrep/issues/131):
  Various issues related to glob matching.
* [BUG #116](https://github.com/BurntSushi/ripgrep/issues/116):
  --quiet should stop search after first match.
* [BUG #121](https://github.com/BurntSushi/ripgrep/pull/121):
  --color always should show colors, even when --vimgrep is used.
* [BUG #122](https://github.com/BurntSushi/ripgrep/pull/122):
  Colorize file path at beginning of line.
* [BUG #134](https://github.com/BurntSushi/ripgrep/issues/134):
  Processing a large ignore file (thousands of globs) was very slow.
* [BUG #137](https://github.com/BurntSushi/ripgrep/issues/137):
  Always follow symlinks when given as an explicit argument.
* [BUG #147](https://github.com/BurntSushi/ripgrep/issues/147):
  Clarify documentation for --replace.


0.2.1
=====
Feature enhancements:

* Added or improved file type filtering for Clojure and SystemVerilog.
* [FEATURE #89](https://github.com/BurntSushi/ripgrep/issues/89):
  Add a --null flag that outputs a NUL byte after every file path.

Bug fixes:

* [BUG #98](https://github.com/BurntSushi/ripgrep/issues/98):
  Fix a bug in single threaded mode when if opening a file failed, ripgrep
  quit instead of continuing the search.
* [BUG #99](https://github.com/BurntSushi/ripgrep/issues/99):
  Fix another bug in single threaded mode where empty lines were being printed
  by mistake.
* [BUG #105](https://github.com/BurntSushi/ripgrep/issues/105):
  Fix an off-by-one error with --column.
* [BUG #106](https://github.com/BurntSushi/ripgrep/issues/106):
  Fix a bug where a whitespace only line in a gitignore file caused ripgrep
  to panic (i.e., crash).


0.2.0
=====
Feature enhancements:

* Added or improved file type filtering for VB, R, F#, Swift, Nim, JavaScript,
  TypeScript
* [FEATURE #20](https://github.com/BurntSushi/ripgrep/issues/20):
  Adds a --no-filename flag.
* [FEATURE #26](https://github.com/BurntSushi/ripgrep/issues/26):
  Adds --files-with-matches flag. Like --count, but only prints file paths
  and doesn't need to count every match.
* [FEATURE #40](https://github.com/BurntSushi/ripgrep/issues/40):
  Switch from using `.rgignore` to `.ignore`. Note that `.rgignore` is
  still supported, but deprecated.
* [FEATURE #68](https://github.com/BurntSushi/ripgrep/issues/68):
  Add --no-ignore-vcs flag that ignores .gitignore but not .ignore.
* [FEATURE #70](https://github.com/BurntSushi/ripgrep/issues/70):
  Add -S/--smart-case flag (but is disabled by default).
* [FEATURE #80](https://github.com/BurntSushi/ripgrep/issues/80):
  Add support for `{foo,bar}` globs.

Many many bug fixes. Thanks every for reporting these and helping make
`ripgrep` better! (Note that I haven't captured every tracking issue here,
some were closed as duplicates.)

* [BUG #8](https://github.com/BurntSushi/ripgrep/issues/8):
  Don't use an intermediate buffer when --threads=1. (Permits constant memory
  usage.)
* [BUG #15](https://github.com/BurntSushi/ripgrep/issues/15):
  Improves the documentation for --type-add.
* [BUG #16](https://github.com/BurntSushi/ripgrep/issues/16),
  [BUG #49](https://github.com/BurntSushi/ripgrep/issues/49),
  [BUG #50](https://github.com/BurntSushi/ripgrep/issues/50),
  [BUG #65](https://github.com/BurntSushi/ripgrep/issues/65):
  Some gitignore globs were being treated as anchored when they weren't.
* [BUG #18](https://github.com/BurntSushi/ripgrep/issues/18):
  --vimgrep reported incorrect column number.
* [BUG #19](https://github.com/BurntSushi/ripgrep/issues/19):
  ripgrep was hanging waiting on stdin in some Windows terminals. Note that
  this introduced a new bug:
  [#94](https://github.com/BurntSushi/ripgrep/issues/94).
* [BUG #21](https://github.com/BurntSushi/ripgrep/issues/21):
  Removes leading `./` when printing file paths.
* [BUG #22](https://github.com/BurntSushi/ripgrep/issues/22):
  Running `rg --help | echo` caused `rg` to panic.
* [BUG #24](https://github.com/BurntSushi/ripgrep/issues/22):
  Clarify the central purpose of rg in its usage message.
* [BUG #25](https://github.com/BurntSushi/ripgrep/issues/25):
  Anchored gitignore globs weren't applied in subdirectories correctly.
* [BUG #30](https://github.com/BurntSushi/ripgrep/issues/30):
  Globs like `foo/**` should match contents of `foo`, but not `foo` itself.
* [BUG #35](https://github.com/BurntSushi/ripgrep/issues/35),
  [BUG #81](https://github.com/BurntSushi/ripgrep/issues/81):
  When automatically detecting stdin, only read if it's a file or a fifo.
  i.e., ignore stdin in `rg foo < /dev/null`.
* [BUG #36](https://github.com/BurntSushi/ripgrep/issues/36):
  Don't automatically pick memory maps on MacOS. Ever.
* [BUG #38](https://github.com/BurntSushi/ripgrep/issues/38):
  Trailing whitespace in gitignore wasn't being ignored.
* [BUG #43](https://github.com/BurntSushi/ripgrep/issues/43):
  --glob didn't work with directories.
* [BUG #46](https://github.com/BurntSushi/ripgrep/issues/46):
  Use one fewer worker thread than what is provided on CLI.
* [BUG #47](https://github.com/BurntSushi/ripgrep/issues/47):
  --help/--version now work even if other options are set.
* [BUG #55](https://github.com/BurntSushi/ripgrep/issues/55):
  ripgrep was refusing to search /proc/cpuinfo. Fixed by disabling memory
  maps for files with zero size.
* [BUG #64](https://github.com/BurntSushi/ripgrep/issues/64):
  The first path given with --files set was ignored.
* [BUG #67](https://github.com/BurntSushi/ripgrep/issues/67):
  Sometimes whitelist globs like `!/dir` weren't interpreted as anchored.
* [BUG #77](https://github.com/BurntSushi/ripgrep/issues/77):
  When -q/--quiet flag was passed, ripgrep kept searching even after a match
  was found.
* [BUG #90](https://github.com/BurntSushi/ripgrep/issues/90):
  Permit whitelisting hidden files.
* [BUG #93](https://github.com/BurntSushi/ripgrep/issues/93):
  ripgrep was extracting an erroneous inner literal from a repeated pattern.

/*!
This crate provides common routines used in command line applications, with a
focus on routines useful for search oriented applications. As a utility
library, there is no central type or function. However, a key focus of this
crate is to improve failure modes and provide user friendly error messages
when things go wrong.

To the best extent possible, everything in this crate works on Windows, macOS
and Linux.


# Standard I/O

The
[`is_readable_stdin`](fn.is_readable_stdin.html),
[`is_tty_stderr`](fn.is_tty_stderr.html),
[`is_tty_stdin`](fn.is_tty_stdin.html)
and
[`is_tty_stdout`](fn.is_tty_stdout.html)
routines query aspects of standard I/O. `is_readable_stdin` determines whether
stdin can be usefully read from, while the `tty` methods determine whether a
tty is attached to stdin/stdout/stderr.

`is_readable_stdin` is useful when writing an application that changes behavior
based on whether the application was invoked with data on stdin. For example,
`rg foo` might recursively search the current working directory for
occurrences of `foo`, but `rg foo < file` might only search the contents of
`file`.

The `tty` methods are useful for similar reasons. Namely, commands like `ls`
will change their output depending on whether they are printing to a terminal
or not. For example, `ls` shows a file on each line when stdout is redirected
to a file or a pipe, but condenses the output to show possibly many files on
each line when stdout is connected to a tty.


# Coloring and buffering

The
[`stdout`](fn.stdout.html),
[`stdout_buffered_block`](fn.stdout_buffered_block.html)
and
[`stdout_buffered_line`](fn.stdout_buffered_line.html)
routines are alternative constructors for
[`StandardStream`](struct.StandardStream.html).
A `StandardStream` implements `termcolor::WriteColor`, which provides a way
to emit colors to terminals. Its key use is the encapsulation of buffering
style. Namely, `stdout` will return a line buffered `StandardStream` if and
only if stdout is connected to a tty, and will otherwise return a block
buffered `StandardStream`. Line buffering is important for use with a tty
because it typically decreases the latency at which the end user sees output.
Block buffering is used otherwise because it is faster, and redirecting stdout
to a file typically doesn't benefit from the decreased latency that line
buffering provides.

The `stdout_buffered_block` and `stdout_buffered_line` can be used to
explicitly set the buffering strategy regardless of whether stdout is connected
to a tty or not.


# Escaping

The
[`escape`](fn.escape.html),
[`escape_os`](fn.escape_os.html),
[`unescape`](fn.unescape.html)
and
[`unescape_os`](fn.unescape_os.html)
routines provide a user friendly way of dealing with UTF-8 encoded strings that
can express arbitrary bytes. For example, you might want to accept a string
containing arbitrary bytes as a command line argument, but most interactive
shells make such strings difficult to type. Instead, we can ask users to use
escape sequences.

For example, `a\xFFz` is itself a valid UTF-8 string corresponding to the
following bytes:

```ignore
[b'a', b'\\', b'x', b'F', b'F', b'z']
```

However, we can
interpret `\xFF` as an escape sequence with the `unescape`/`unescape_os`
routines, which will yield

```ignore
[b'a', b'\xFF', b'z']
```

instead. For example:

```
use grep_cli::unescape;

// Note the use of a raw string!
assert_eq!(vec![b'a', b'\xFF', b'z'], unescape(r"a\xFFz"));
```

The `escape`/`escape_os` routines provide the reverse transformation, which
makes it easy to show user friendly error messages involving arbitrary bytes.


# Building patterns

Typically, regular expression patterns must be valid UTF-8. However, command
line arguments aren't guaranteed to be valid UTF-8. Unfortunately, the
standard library's UTF-8 conversion functions from `OsStr`s do not provide
good error messages. However, the
[`pattern_from_bytes`](fn.pattern_from_bytes.html)
and
[`pattern_from_os`](fn.pattern_from_os.html)
do, including reporting exactly where the first invalid UTF-8 byte is seen.

Additionally, it can be useful to read patterns from a file while reporting
good error messages that include line numbers. The
[`patterns_from_path`](fn.patterns_from_path.html),
[`patterns_from_reader`](fn.patterns_from_reader.html)
and
[`patterns_from_stdin`](fn.patterns_from_stdin.html)
routines do just that. If any pattern is found that is invalid UTF-8, then the
error includes the file path (if available) along with the line number and the
byte offset at which the first invalid UTF-8 byte was observed.


# Read process output

Sometimes a command line application needs to execute other processes and read
its stdout in a streaming fashion. The
[`CommandReader`](struct.CommandReader.html)
provides this functionality with an explicit goal of improving failure modes.
In particular, if the process exits with an error code, then stderr is read
and converted into a normal Rust error to show to end users. This makes the
underlying failure modes explicit and gives more information to end users for
debugging the problem.

As a special case,
[`DecompressionReader`](struct.DecompressionReader.html)
provides a way to decompress arbitrary files by matching their file extensions
up with corresponding decompression programs (such as `gzip` and `xz`). This
is useful as a means of performing simplistic decompression in a portable
manner without binding to specific compression libraries. This does come with
some overhead though, so if you need to decompress lots of small files, this
may not be an appropriate convenience to use.

Each reader has a corresponding builder for additional configuration, such as
whether to read stderr asynchronously in order to avoid deadlock (which is
enabled by default).


# Miscellaneous parsing

The
[`parse_human_readable_size`](fn.parse_human_readable_size.html)
routine parses strings like `2M` and converts them to the corresponding number
of bytes (`2 * 1<<20` in this case). If an invalid size is found, then a good
error message is crafted that typically tells the user how to fix the problem.
*/

#![deny(missing_docs)]

mod decompress;
mod escape;
mod human;
mod pattern;
mod process;
mod wtr;

use std::io::IsTerminal;

pub use crate::decompress::{
    resolve_binary, DecompressionMatcher, DecompressionMatcherBuilder,
    DecompressionReader, DecompressionReaderBuilder,
};
pub use crate::escape::{escape, escape_os, unescape, unescape_os};
pub use crate::human::{parse_human_readable_size, ParseSizeError};
pub use crate::pattern::{
    pattern_from_bytes, pattern_from_os, patterns_from_path,
    patterns_from_reader, patterns_from_stdin, InvalidPatternError,
};
pub use crate::process::{CommandError, CommandReader, CommandReaderBuilder};
pub use crate::wtr::{
    stdout, stdout_buffered_block, stdout_buffered_line, StandardStream,
};

/// Returns true if and only if stdin is believed to be readable.
///
/// When stdin is readable, command line programs may choose to behave
/// differently than when stdin is not readable. For example, `command foo`
/// might search the current directory for occurrences of `foo` where as
/// `command foo < some-file` or `cat some-file | command foo` might instead
/// only search stdin for occurrences of `foo`.
pub fn is_readable_stdin() -> bool {
    #[cfg(unix)]
    fn imp() -> bool {
        use same_file::Handle;
        use std::os::unix::fs::FileTypeExt;

        let ft = match Handle::stdin().and_then(|h| h.as_file().metadata()) {
            Err(_) => return false,
            Ok(md) => md.file_type(),
        };
        ft.is_file() || ft.is_fifo() || ft.is_socket()
    }

    #[cfg(windows)]
    fn imp() -> bool {
        use winapi_util as winutil;

        winutil::file::typ(winutil::HandleRef::stdin())
            .map(|t| t.is_disk() || t.is_pipe())
            .unwrap_or(false)
    }

    !is_tty_stdin() && imp()
}

/// Returns true if and only if stdin is believed to be connected to a tty
/// or a console.
pub fn is_tty_stdin() -> bool {
    std::io::stdin().is_terminal()
}

/// Returns true if and only if stdout is believed to be connected to a tty
/// or a console.
///
/// This is useful for when you want your command line program to produce
/// different output depending on whether it's printing directly to a user's
/// terminal or whether it's being redirected somewhere else. For example,
/// implementations of `ls` will often show one item per line when stdout is
/// redirected, but will condensed output when printing to a tty.
pub fn is_tty_stdout() -> bool {
    std::io::stdout().is_terminal()
}

/// Returns true if and only if stderr is believed to be connected to a tty
/// or a console.
pub fn is_tty_stderr() -> bool {
    std::io::stderr().is_terminal()
}

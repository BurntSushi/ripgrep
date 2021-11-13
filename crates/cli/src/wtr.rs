use std::io;

use termcolor;

use crate::is_tty_stdout;

/// A writer that supports coloring with either line or block buffering.
pub struct StandardStream(StandardStreamKind);

/// Returns a possibly buffered writer to stdout for the given color choice.
///
/// The writer returned is either line buffered or block buffered. The decision
/// between these two is made automatically based on whether a tty is attached
/// to stdout or not. If a tty is attached, then line buffering is used.
/// Otherwise, block buffering is used. In general, block buffering is more
/// efficient, but may increase the time it takes for the end user to see the
/// first bits of output.
///
/// If you need more fine grained control over the buffering mode, then use one
/// of `stdout_buffered_line` or `stdout_buffered_block`.
///
/// The color choice given is passed along to the underlying writer. To
/// completely disable colors in all cases, use `ColorChoice::Never`.
pub fn stdout(color_choice: termcolor::ColorChoice) -> StandardStream {
    if is_tty_stdout() {
        stdout_buffered_line(color_choice)
    } else {
        stdout_buffered_block(color_choice)
    }
}

/// Returns a line buffered writer to stdout for the given color choice.
///
/// This writer is useful when printing results directly to a tty such that
/// users see output as soon as it's written. The downside of this approach
/// is that it can be slower, especially when there is a lot of output.
///
/// You might consider using
/// [`stdout`](fn.stdout.html)
/// instead, which chooses the buffering strategy automatically based on
/// whether stdout is connected to a tty.
pub fn stdout_buffered_line(
    color_choice: termcolor::ColorChoice,
) -> StandardStream {
    let out = termcolor::StandardStream::stdout(color_choice);
    StandardStream(StandardStreamKind::LineBuffered(out))
}

/// Returns a block buffered writer to stdout for the given color choice.
///
/// This writer is useful when printing results to a file since it amortizes
/// the cost of writing data. The downside of this approach is that it can
/// increase the latency of display output when writing to a tty.
///
/// You might consider using
/// [`stdout`](fn.stdout.html)
/// instead, which chooses the buffering strategy automatically based on
/// whether stdout is connected to a tty.
pub fn stdout_buffered_block(
    color_choice: termcolor::ColorChoice,
) -> StandardStream {
    let out = termcolor::BufferedStandardStream::stdout(color_choice);
    StandardStream(StandardStreamKind::BlockBuffered(out))
}

enum StandardStreamKind {
    LineBuffered(termcolor::StandardStream),
    BlockBuffered(termcolor::BufferedStandardStream),
}

impl io::Write for StandardStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use self::StandardStreamKind::*;

        match self.0 {
            LineBuffered(ref mut w) => w.write(buf),
            BlockBuffered(ref mut w) => w.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        use self::StandardStreamKind::*;

        match self.0 {
            LineBuffered(ref mut w) => w.flush(),
            BlockBuffered(ref mut w) => w.flush(),
        }
    }
}

impl termcolor::WriteColor for StandardStream {
    #[inline]
    fn supports_color(&self) -> bool {
        use self::StandardStreamKind::*;

        match self.0 {
            LineBuffered(ref w) => w.supports_color(),
            BlockBuffered(ref w) => w.supports_color(),
        }
    }

    #[inline]
    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        use self::StandardStreamKind::*;

        match self.0 {
            LineBuffered(ref mut w) => w.set_color(spec),
            BlockBuffered(ref mut w) => w.set_color(spec),
        }
    }

    #[inline]
    fn reset(&mut self) -> io::Result<()> {
        use self::StandardStreamKind::*;

        match self.0 {
            LineBuffered(ref mut w) => w.reset(),
            BlockBuffered(ref mut w) => w.reset(),
        }
    }

    #[inline]
    fn is_synchronous(&self) -> bool {
        use self::StandardStreamKind::*;

        match self.0 {
            LineBuffered(ref w) => w.is_synchronous(),
            BlockBuffered(ref w) => w.is_synchronous(),
        }
    }
}

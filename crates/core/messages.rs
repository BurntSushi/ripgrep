/*!
This module defines some macros and some light shared mutable state.

This state is responsible for keeping track of whether we should emit certain
kinds of messages to the user (such as errors) that are distinct from the
standard "debug" or "trace" log messages. This state is specifically set at
startup time when CLI arguments are parsed and then never changed.

The other state tracked here is whether ripgrep experienced an error
condition. Aside from errors associated with invalid CLI arguments, ripgrep
generally does not abort when an error occurs (e.g., if reading a file failed).
But when an error does occur, it will alter ripgrep's exit status. Thus, when
an error message is emitted via `err_message`, then a global flag is toggled
indicating that at least one error occurred. When ripgrep exits, this flag is
consulted to determine what the exit status ought to be.
*/

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// When false, "messages" will not be printed.
static MESSAGES: AtomicBool = AtomicBool::new(false);
/// When false, "messages" related to ignore rules will not be printed.
static IGNORE_MESSAGES: AtomicBool = AtomicBool::new(false);
/// Flipped to true when an error message is printed.
static ERRORED: AtomicBool = AtomicBool::new(false);

/// Returns true when an I/O error indicates that an output pipe was closed.
///
/// On Windows, a closed pipe may surface as `ERROR_NO_DATA` (os error 232)
/// instead of `ErrorKind::BrokenPipe`.
pub(crate) fn is_output_pipe_closed(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::BrokenPipe {
        return true;
    }
    #[cfg(windows)]
    {
        if err.raw_os_error() == Some(232) {
            return true;
        }
    }
    false
}

/// Writes a single `rg: ...` line to stderr.
///
/// When stdout is a TTY, stdout is locked first to avoid interleaving with
/// search output. When stdout is not a TTY, locking stdout is skipped so that
/// error messages can still be emitted when stdout's pipe has been closed.
pub(crate) fn write_locked_message(message: &str) {
    let write_message =
        |stderr: &mut std::io::StderrLock<'_>| -> std::io::Result<()> {
            write!(stderr, "rg: ")?;
            writeln!(stderr, "{message}")?;
            Ok(())
        };
    if std::io::stdout().is_terminal() {
        let stdout = std::io::stdout().lock();
        let mut stderr = std::io::stderr().lock();
        if let Err(err) = write_message(&mut stderr) {
            if is_output_pipe_closed(&err) {
                std::process::exit(0);
            }
            std::process::exit(2);
        }
        drop(stdout);
    } else {
        let mut stderr = std::io::stderr().lock();
        if let Err(err) = write_message(&mut stderr) {
            if is_output_pipe_closed(&err) {
                std::process::exit(0);
            }
            std::process::exit(2);
        }
    }
}

/// Like eprintln, but locks stdout to prevent interleaving lines.
///
/// This locks stdout, not stderr, even though this prints to stderr. This
/// avoids the appearance of interleaving output when stdout and stderr both
/// correspond to a tty.
#[macro_export]
macro_rules! eprintln_locked {
    ($($tt:tt)*) => {{
        $crate::messages::write_locked_message(&format!($($tt)*));
    }}
}

/// Emit a non-fatal error message, unless messages were disabled.
#[macro_export]
macro_rules! message {
    ($($tt:tt)*) => {
        if crate::messages::messages() {
            eprintln_locked!($($tt)*);
        }
    }
}

/// Like message, but sets ripgrep's "errored" flag, which controls the exit
/// status.
#[macro_export]
macro_rules! err_message {
    ($($tt:tt)*) => {
        crate::messages::set_errored();
        message!($($tt)*);
    }
}

/// Emit a non-fatal ignore-related error message (like a parse error), unless
/// ignore-messages were disabled.
#[macro_export]
macro_rules! ignore_message {
    ($($tt:tt)*) => {
        if crate::messages::messages() && crate::messages::ignore_messages() {
            eprintln_locked!($($tt)*);
        }
    }
}

/// Returns true if and only if messages should be shown.
pub(crate) fn messages() -> bool {
    MESSAGES.load(Ordering::Relaxed)
}

/// Set whether messages should be shown or not.
///
/// By default, they are not shown.
pub(crate) fn set_messages(yes: bool) {
    MESSAGES.store(yes, Ordering::Relaxed)
}

/// Returns true if and only if "ignore" related messages should be shown.
pub(crate) fn ignore_messages() -> bool {
    IGNORE_MESSAGES.load(Ordering::Relaxed)
}

/// Set whether "ignore" related messages should be shown or not.
///
/// By default, they are not shown.
///
/// Note that this is overridden if `messages` is disabled. Namely, if
/// `messages` is disabled, then "ignore" messages are never shown, regardless
/// of this setting.
pub(crate) fn set_ignore_messages(yes: bool) {
    IGNORE_MESSAGES.store(yes, Ordering::Relaxed)
}

/// Returns true if and only if ripgrep came across a non-fatal error.
pub(crate) fn errored() -> bool {
    ERRORED.load(Ordering::Relaxed)
}

/// Indicate that ripgrep has come across a non-fatal error.
///
/// Callers should not use this directly. Instead, it is called automatically
/// via the `err_message` macro.
pub(crate) fn set_errored() {
    ERRORED.store(true, Ordering::Relaxed);
}

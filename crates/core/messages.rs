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

use std::sync::atomic::{AtomicBool, Ordering};

/// When false, "messages" will not be printed.
static MESSAGES: AtomicBool = AtomicBool::new(false);
/// When false, "messages" related to ignore rules will not be printed.
static IGNORE_MESSAGES: AtomicBool = AtomicBool::new(false);
/// Flipped to true when an error message is printed.
static ERRORED: AtomicBool = AtomicBool::new(false);

/// Like eprintln, but locks stdout to prevent interleaving lines.
///
/// This locks stdout, not stderr, even though this prints to stderr. This
/// avoids the appearance of interleaving output when stdout and stderr both
/// correspond to a tty.
#[macro_export]
macro_rules! eprintln_locked {
    ($($tt:tt)*) => {{
        {
            use std::io::Write;

            // This is a bit of an abstraction violation because we explicitly
            // lock stdout before printing to stderr. This avoids interleaving
            // lines within ripgrep because `search_parallel` uses `termcolor`,
            // which accesses the same stdout lock when writing lines.
            let stdout = std::io::stdout().lock();
            let mut stderr = std::io::stderr().lock();
            // We specifically ignore any errors here. One plausible error we
            // can get in some cases is a broken pipe error. And when that
            // occurs, we should exit gracefully. Otherwise, just abort with
            // an error code because there isn't much else we can do.
            //
            // See: https://github.com/BurntSushi/ripgrep/issues/1966
            if let Err(err) = write!(stderr, "rg: ") {
                if err.kind() == std::io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                } else {
                    std::process::exit(2);
                }
            }
            if let Err(err) = writeln!(stderr, $($tt)*) {
                if err.kind() == std::io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                } else {
                    std::process::exit(2);
                }
            }
            drop(stdout);
        }
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

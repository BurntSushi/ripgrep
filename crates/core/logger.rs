/*!
Defines a super simple logger that works with the `log` crate.

We don't do anything fancy. We just need basic log levels and the ability to
print to stderr. We therefore avoid bringing in extra dependencies just for
this functionality.
*/

use log::{self, Log};

/// The simplest possible logger that logs to stderr.
///
/// This logger does no filtering. Instead, it relies on the `log` crates
/// filtering via its global max_level setting.
#[derive(Debug)]
pub(crate) struct Logger(());

/// A singleton used as the target for an implementation of the `Log` trait.
const LOGGER: &'static Logger = &Logger(());

impl Logger {
    /// Create a new logger that logs to stderr and initialize it as the
    /// global logger. If there was a problem setting the logger, then an
    /// error is returned.
    pub(crate) fn init() -> Result<(), log::SetLoggerError> {
        log::set_logger(LOGGER)
    }
}

impl Log for Logger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        // We set the log level via log::set_max_level, so we don't need to
        // implement filtering here.
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        match (record.file(), record.line()) {
            (Some(file), Some(line)) => {
                eprintln_locked!(
                    "{}|{}|{}:{}: {}",
                    record.level(),
                    record.target(),
                    file,
                    line,
                    record.args()
                );
            }
            (Some(file), None) => {
                eprintln_locked!(
                    "{}|{}|{}: {}",
                    record.level(),
                    record.target(),
                    file,
                    record.args()
                );
            }
            _ => {
                eprintln_locked!(
                    "{}|{}: {}",
                    record.level(),
                    record.target(),
                    record.args()
                );
            }
        }
    }

    fn flush(&self) {
        // We use eprintln_locked! which is flushed on every call.
    }
}

/*!
Parses command line arguments into a structured and typed representation.
*/

use std::ffi::OsString;

use anyhow::Context;

use crate::flags::{
    defs::FLAGS,
    hiargs::HiArgs,
    lowargs::{LoggingMode, LowArgs, SpecialMode},
    Flag, FlagValue,
};

/// The result of parsing CLI arguments.
///
/// This is basically a `anyhow::Result<T>`, but with one extra variant that is
/// inhabited whenever ripgrep should execute a "special" mode. That is, when a
/// user provides the `-h/--help` or `-V/--version` flags.
///
/// This special variant exists to allow CLI parsing to short circuit as
/// quickly as is reasonable. For example, it lets CLI parsing avoid reading
/// ripgrep's configuration and converting low level arguments into a higher
/// level representation.
#[derive(Debug)]
pub(crate) enum ParseResult<T> {
    Special(SpecialMode),
    Ok(T),
    Err(anyhow::Error),
}

impl<T> ParseResult<T> {
    /// If this result is `Ok`, then apply `then` to it. Otherwise, return this
    /// result unchanged.
    fn and_then<U>(
        self,
        mut then: impl FnMut(T) -> ParseResult<U>,
    ) -> ParseResult<U> {
        match self {
            ParseResult::Special(mode) => ParseResult::Special(mode),
            ParseResult::Ok(t) => then(t),
            ParseResult::Err(err) => ParseResult::Err(err),
        }
    }
}

/// Parse CLI arguments and convert then to their high level representation.
pub(crate) fn parse() -> ParseResult<HiArgs> {
    parse_low().and_then(|low| match HiArgs::from_low_args(low) {
        Ok(hi) => ParseResult::Ok(hi),
        Err(err) => ParseResult::Err(err),
    })
}

/// Parse CLI arguments only into their low level representation.
///
/// This takes configuration into account. That is, it will try to read
/// `RIPGREP_CONFIG_PATH` and prepend any arguments found there to the
/// arguments passed to this process.
///
/// This will also set one-time global state flags, such as the log level and
/// whether messages should be printed.
fn parse_low() -> ParseResult<LowArgs> {
    if let Err(err) = crate::logger::Logger::init() {
        let err = anyhow::anyhow!("failed to initialize logger: {err}");
        return ParseResult::Err(err);
    }

    let parser = Parser::new();
    let mut low = LowArgs::default();
    if let Err(err) = parser.parse(std::env::args_os().skip(1), &mut low) {
        return ParseResult::Err(err);
    }
    // Even though we haven't parsed the config file yet (assuming it exists),
    // we can still use the arguments given on the CLI to setup ripgrep's
    // logging preferences. Even if the config file changes them in some way,
    // it's really the best we can do. This way, for example, folks can pass
    // `--trace` and see any messages logged during config file parsing.
    set_log_levels(&low);
    // Before we try to take configuration into account, we can bail early
    // if a special mode was enabled. This is basically only for version and
    // help output which shouldn't be impacted by extra configuration.
    if let Some(special) = low.special.take() {
        return ParseResult::Special(special);
    }
    // If the end user says no config, then respect it.
    if low.no_config {
        log::debug!("not reading config files because --no-config is present");
        return ParseResult::Ok(low);
    }
    // Look for arguments from a config file. If we got nothing (whether the
    // file is empty or RIPGREP_CONFIG_PATH wasn't set), then we don't need
    // to re-parse.
    let config_args = crate::flags::config::args();
    if config_args.is_empty() {
        log::debug!("no extra arguments found from configuration file");
        return ParseResult::Ok(low);
    }
    // The final arguments are just the arguments from the CLI appending to
    // the end of the config arguments.
    let mut final_args = config_args;
    final_args.extend(std::env::args_os().skip(1));

    // Now do the CLI parsing dance again.
    let mut low = LowArgs::default();
    if let Err(err) = parser.parse(final_args.into_iter(), &mut low) {
        return ParseResult::Err(err);
    }
    // Reset the message and logging levels, since they could have changed.
    set_log_levels(&low);
    ParseResult::Ok(low)
}

/// Sets global state flags that control logging based on low-level arguments.
fn set_log_levels(low: &LowArgs) {
    crate::messages::set_messages(!low.no_messages);
    crate::messages::set_ignore_messages(!low.no_ignore_messages);
    match low.logging {
        Some(LoggingMode::Trace) => {
            log::set_max_level(log::LevelFilter::Trace)
        }
        Some(LoggingMode::Debug) => {
            log::set_max_level(log::LevelFilter::Debug)
        }
        None => log::set_max_level(log::LevelFilter::Warn),
    }
}

/// Parse the sequence of CLI arguments given a low level typed set of
/// arguments.
///
/// This is exposed for testing that the correct low-level arguments are parsed
/// from a CLI. It just runs the parser once over the CLI arguments. It doesn't
/// setup logging or read from a config file.
///
/// This assumes the iterator given does *not* begin with the binary name.
#[cfg(test)]
pub(crate) fn parse_low_raw(
    rawargs: impl IntoIterator<Item = impl Into<OsString>>,
) -> anyhow::Result<LowArgs> {
    let mut args = LowArgs::default();
    Parser::new().parse(rawargs, &mut args)?;
    Ok(args)
}

/// Return the metadata for the flag of the given name.
pub(super) fn lookup(name: &str) -> Option<&'static dyn Flag> {
    // N.B. Creating a new parser might look expensive, but it only builds
    // the lookup trie exactly once. That is, we get a `&'static Parser` from
    // `Parser::new()`.
    match Parser::new().find_long(name) {
        FlagLookup::Match(&FlagInfo { flag, .. }) => Some(flag),
        _ => None,
    }
}

/// A parser for turning a sequence of command line arguments into a more
/// strictly typed set of arguments.
#[derive(Debug)]
struct Parser {
    /// A single map that contains all possible flag names. This includes
    /// short and long names, aliases and negations. This maps those names to
    /// indices into `info`.
    map: FlagMap,
    /// A map from IDs returned by the `map` to the corresponding flag
    /// information.
    info: Vec<FlagInfo>,
}

impl Parser {
    /// Create a new parser.
    ///
    /// This always creates the same parser and only does it once. Callers may
    /// call this repeatedly, and the parser will only be built once.
    fn new() -> &'static Parser {
        use std::sync::OnceLock;

        // Since a parser's state is immutable and completely determined by
        // FLAGS, and since FLAGS is a constant, we can initialize it exactly
        // once.
        static P: OnceLock<Parser> = OnceLock::new();
        P.get_or_init(|| {
            let mut infos = vec![];
            for &flag in FLAGS.iter() {
                infos.push(FlagInfo {
                    flag,
                    name: Ok(flag.name_long()),
                    kind: FlagInfoKind::Standard,
                });
                for alias in flag.aliases() {
                    infos.push(FlagInfo {
                        flag,
                        name: Ok(alias),
                        kind: FlagInfoKind::Alias,
                    });
                }
                if let Some(byte) = flag.name_short() {
                    infos.push(FlagInfo {
                        flag,
                        name: Err(byte),
                        kind: FlagInfoKind::Standard,
                    });
                }
                if let Some(name) = flag.name_negated() {
                    infos.push(FlagInfo {
                        flag,
                        name: Ok(name),
                        kind: FlagInfoKind::Negated,
                    });
                }
            }
            let map = FlagMap::new(&infos);
            Parser { map, info: infos }
        })
    }

    /// Parse the given CLI arguments into a low level representation.
    ///
    /// The iterator given should *not* start with the binary name.
    fn parse<I, O>(&self, rawargs: I, args: &mut LowArgs) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = O>,
        O: Into<OsString>,
    {
        let mut p = lexopt::Parser::from_args(rawargs);
        while let Some(arg) = p.next().context("invalid CLI arguments")? {
            let lookup = match arg {
                lexopt::Arg::Value(value) => {
                    args.positional.push(value);
                    continue;
                }
                lexopt::Arg::Short(ch) if ch == 'h' => {
                    // Special case -h/--help since behavior is different
                    // based on whether short or long flag is given.
                    args.special = Some(SpecialMode::HelpShort);
                    continue;
                }
                lexopt::Arg::Short(ch) if ch == 'V' => {
                    // Special case -V/--version since behavior is different
                    // based on whether short or long flag is given.
                    args.special = Some(SpecialMode::VersionShort);
                    continue;
                }
                lexopt::Arg::Short(ch) => self.find_short(ch),
                lexopt::Arg::Long(name) if name == "help" => {
                    // Special case -h/--help since behavior is different
                    // based on whether short or long flag is given.
                    args.special = Some(SpecialMode::HelpLong);
                    continue;
                }
                lexopt::Arg::Long(name) if name == "version" => {
                    // Special case -V/--version since behavior is different
                    // based on whether short or long flag is given.
                    args.special = Some(SpecialMode::VersionLong);
                    continue;
                }
                lexopt::Arg::Long(name) => self.find_long(name),
            };
            let mat = match lookup {
                FlagLookup::Match(mat) => mat,
                FlagLookup::UnrecognizedShort(name) => {
                    anyhow::bail!("unrecognized flag -{name}")
                }
                FlagLookup::UnrecognizedLong(name) => {
                    anyhow::bail!("unrecognized flag --{name}")
                }
            };
            let value = if matches!(mat.kind, FlagInfoKind::Negated) {
                // Negated flags are always switches, even if the non-negated
                // flag is not. For example, --context-separator accepts a
                // value, but --no-context-separator does not.
                FlagValue::Switch(false)
            } else if mat.flag.is_switch() {
                FlagValue::Switch(true)
            } else {
                FlagValue::Value(p.value().with_context(|| {
                    format!("missing value for flag {mat}")
                })?)
            };
            mat.flag
                .update(value, args)
                .with_context(|| format!("error parsing flag {mat}"))?;
        }
        Ok(())
    }

    /// Look for a flag by its short name.
    fn find_short(&self, ch: char) -> FlagLookup<'_> {
        if !ch.is_ascii() {
            return FlagLookup::UnrecognizedShort(ch);
        }
        let byte = u8::try_from(ch).unwrap();
        let Some(index) = self.map.find(&[byte]) else {
            return FlagLookup::UnrecognizedShort(ch);
        };
        FlagLookup::Match(&self.info[index])
    }

    /// Look for a flag by its long name.
    ///
    /// This also works for aliases and negated names.
    fn find_long(&self, name: &str) -> FlagLookup<'_> {
        let Some(index) = self.map.find(name.as_bytes()) else {
            return FlagLookup::UnrecognizedLong(name.to_string());
        };
        FlagLookup::Match(&self.info[index])
    }
}

/// The result of looking up a flag name.
#[derive(Debug)]
enum FlagLookup<'a> {
    /// Lookup found a match and the metadata for the flag is attached.
    Match(&'a FlagInfo),
    /// The given short name is unrecognized.
    UnrecognizedShort(char),
    /// The given long name is unrecognized.
    UnrecognizedLong(String),
}

/// The info about a flag associated with a flag's ID in the the flag map.
#[derive(Debug)]
struct FlagInfo {
    /// The flag object and its associated metadata.
    flag: &'static dyn Flag,
    /// The actual name that is stored in the Aho-Corasick automaton. When this
    /// is a byte, it corresponds to a short single character ASCII flag. The
    /// actual pattern that's in the Aho-Corasick automaton is just the single
    /// byte.
    name: Result<&'static str, u8>,
    /// The type of flag that is stored for the corresponding Aho-Corasick
    /// pattern.
    kind: FlagInfoKind,
}

/// The kind of flag that is being matched.
#[derive(Debug)]
enum FlagInfoKind {
    /// A standard flag, e.g., --passthru.
    Standard,
    /// A negation of a standard flag, e.g., --no-multiline.
    Negated,
    /// An alias for a standard flag, e.g., --passthrough.
    Alias,
}

impl std::fmt::Display for FlagInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.name {
            Ok(long) => write!(f, "--{long}"),
            Err(short) => write!(f, "-{short}", short = char::from(short)),
        }
    }
}

/// A map from flag names (short, long, negated and aliases) to their ID.
///
/// Once an ID is known, it can be used to look up a flag's metadata in the
/// parser's internal state.
#[derive(Debug)]
struct FlagMap {
    map: std::collections::HashMap<Vec<u8>, usize>,
}

impl FlagMap {
    /// Create a new map of flags for the given flag information.
    ///
    /// The index of each flag info corresponds to its ID.
    fn new(infos: &[FlagInfo]) -> FlagMap {
        let mut map = std::collections::HashMap::with_capacity(infos.len());
        for (i, info) in infos.iter().enumerate() {
            match info.name {
                Ok(name) => {
                    assert_eq!(None, map.insert(name.as_bytes().to_vec(), i));
                }
                Err(byte) => {
                    assert_eq!(None, map.insert(vec![byte], i));
                }
            }
        }
        FlagMap { map }
    }

    /// Look for a match of `name` in the given Aho-Corasick automaton.
    ///
    /// This only returns a match if the one found has a length equivalent to
    /// the length of the name given.
    fn find(&self, name: &[u8]) -> Option<usize> {
        self.map.get(name).copied()
    }
}

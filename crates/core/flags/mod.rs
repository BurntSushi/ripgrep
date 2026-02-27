/*!
Defines ripgrep's command line interface.

This modules deals with everything involving ripgrep's flags and positional
arguments. This includes generating shell completions, `--help` output and even
ripgrep's man page. It's also responsible for parsing and validating every
flag (including reading ripgrep's config file), and manages the contact points
between these flags and ripgrep's cast of supporting libraries. For example,
once [`HiArgs`] has been created, it knows how to create a multi threaded
recursive directory traverser.
*/
use std::{
    ffi::OsString,
    fmt::Debug,
    panic::{RefUnwindSafe, UnwindSafe},
};

pub(crate) use crate::flags::{
    complete::{
        bash::generate as generate_complete_bash,
        fish::generate as generate_complete_fish,
        powershell::generate as generate_complete_powershell,
        zsh::generate as generate_complete_zsh,
    },
    doc::{
        help::{
            generate_long as generate_help_long,
            generate_short as generate_help_short,
        },
        man::generate as generate_man_page,
        version::{
            generate_long as generate_version_long,
            generate_pcre2 as generate_version_pcre2,
            generate_short as generate_version_short,
        },
    },
    hiargs::HiArgs,
    lowargs::{GenerateMode, Mode, SearchMode, SpecialMode},
    parse::{ParseResult, parse},
};

mod complete;
mod config;
mod defs;
mod doc;
mod hiargs;
mod lowargs;
mod parse;

/// A trait that encapsulates the definition of an optional flag for ripgrep.
///
/// This trait is meant to be used via dynamic dispatch. Namely, the `defs`
/// module provides a single global slice of `&dyn Flag` values correspondings
/// to all of the flags in ripgrep.
///
/// ripgrep's required positional arguments are handled by the parser and by
/// the conversion from low-level arguments to high level arguments. Namely,
/// all of ripgrep's positional arguments are treated as file paths, except
/// in certain circumstances where the first argument is treated as a regex
/// pattern.
///
/// Note that each implementation of this trait requires a long flag name,
/// but can also optionally have a short version and even a negation flag.
/// For example, the `-E/--encoding` flag accepts a value, but it also has a
/// `--no-encoding` negation flag for reverting back to "automatic" encoding
/// detection. All three of `-E`, `--encoding` and `--no-encoding` are provided
/// by a single implementation of this trait.
///
/// ripgrep only supports flags that are switches or flags that accept a single
/// value. Flags that accept multiple values are an unsupported abberation.
trait Flag: Debug + Send + Sync + UnwindSafe + RefUnwindSafe + 'static {
    /// Returns true if this flag is a switch. When a flag is a switch, the
    /// CLI parser will not look for a value after the flag is seen.
    fn is_switch(&self) -> bool;

    /// A short single byte name for this flag. This returns `None` by default,
    /// which signifies that the flag has no short name.
    ///
    /// The byte returned must be an ASCII codepoint that is a `.` or is
    /// alpha-numeric.
    fn name_short(&self) -> Option<u8> {
        None
    }

    /// Returns the long name of this flag. All flags must have a "long" name.
    ///
    /// The long name must be at least 2 bytes, and all of its bytes must be
    /// ASCII codepoints that are either `-` or alpha-numeric.
    fn name_long(&self) -> &'static str;

    /// Returns a list of aliases for this flag.
    ///
    /// The aliases must follow the same rules as `Flag::name_long`.
    ///
    /// By default, an empty slice is returned.
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// Returns a negated name for this flag. The negation of a flag is
    /// intended to have the opposite meaning of a flag or to otherwise turn
    /// something "off" or revert it to its default behavior.
    ///
    /// Negated flags are not listed in their own section in the `-h/--help`
    /// output or man page. Instead, they are automatically mentioned at the
    /// end of the documentation section of the flag they negated.
    ///
    /// The aliases must follow the same rules as `Flag::name_long`.
    ///
    /// By default, a flag has no negation and this returns `None`.
    fn name_negated(&self) -> Option<&'static str> {
        None
    }

    /// Returns the variable name describing the type of value this flag
    /// accepts. This should always be set for non-switch flags and never set
    /// for switch flags.
    ///
    /// For example, the `--max-count` flag has its variable name set to `NUM`.
    ///
    /// The convention is to capitalize variable names.
    ///
    /// By default this returns `None`.
    fn doc_variable(&self) -> Option<&'static str> {
        None
    }

    /// Returns the category of this flag.
    ///
    /// Every flag must have a single category. Categories are used to organize
    /// flags in the generated documentation.
    fn doc_category(&self) -> Category;

    /// A (very) short documentation string describing what this flag does.
    ///
    /// This may sacrifice "proper English" in order to be as terse as
    /// possible. Generally, we try to ensure that `rg -h` doesn't have any
    /// lines that exceed 79 columns.
    fn doc_short(&self) -> &'static str;

    /// A (possibly very) longer documentation string describing in full
    /// detail what this flag does. This should be in mandoc/mdoc format.
    fn doc_long(&self) -> &'static str;

    /// If this is a non-switch flag that accepts a small set of specific
    /// values, then this should list them.
    ///
    /// This returns an empty slice by default.
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn completion_type(&self) -> CompletionType {
        CompletionType::Other
    }

    /// Given the parsed value (which might just be a switch), this should
    /// update the state in `args` based on the value given for this flag.
    ///
    /// This may update state for other flags as appropriate.
    ///
    /// The `-V/--version` and `-h/--help` flags are treated specially in the
    /// parser and should do nothing here.
    ///
    /// By convention, implementations should generally not try to "do"
    /// anything other than validate the value given. For example, the
    /// implementation for `--hostname-bin` should not try to resolve the
    /// hostname to use by running the binary provided. That should be saved
    /// for a later step. This convention is used to ensure that getting the
    /// low-level arguments is as reliable and quick as possible. It also
    /// ensures that "doing something" occurs a minimal number of times. For
    /// example, by avoiding trying to find the hostname here, we can do it
    /// once later no matter how many times `--hostname-bin` is provided.
    ///
    /// Implementations should not include the flag name in the error message
    /// returned. The flag name is included automatically by the parser.
    fn update(
        &self,
        value: FlagValue,
        args: &mut crate::flags::lowargs::LowArgs,
    ) -> anyhow::Result<()>;
}

/// The category that a flag belongs to.
///
/// Categories are used to organize flags into "logical" groups in the
/// generated documentation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
enum Category {
    /// Flags related to how ripgrep reads its input. Its "input" generally
    /// consists of the patterns it is trying to match and the haystacks it is
    /// trying to search.
    Input,
    /// Flags related to the operation of the search itself. For example,
    /// whether case insensitive matching is enabled.
    Search,
    /// Flags related to how ripgrep filters haystacks. For example, whether
    /// to respect gitignore files or not.
    Filter,
    /// Flags related to how ripgrep shows its search results. For example,
    /// whether to show line numbers or not.
    Output,
    /// Flags related to changing ripgrep's output at a more fundamental level.
    /// For example, flags like `--count` suppress printing of individual
    /// lines, and instead just print the total count of matches for each file
    /// searched.
    OutputModes,
    /// Flags related to logging behavior such as emitting non-fatal error
    /// messages or printing search statistics.
    Logging,
    /// Other behaviors not related to ripgrep's core functionality. For
    /// example, printing the file type globbing rules, or printing the list
    /// of files ripgrep would search without actually searching them.
    OtherBehaviors,
}

impl Category {
    /// Returns a string representation of this category.
    ///
    /// This string is the name of the variable used in various templates for
    /// generated documentation. This name can be used for interpolation.
    fn as_str(&self) -> &'static str {
        match *self {
            Category::Input => "input",
            Category::Search => "search",
            Category::Filter => "filter",
            Category::Output => "output",
            Category::OutputModes => "output-modes",
            Category::Logging => "logging",
            Category::OtherBehaviors => "other-behaviors",
        }
    }
}

/// The kind of argument a flag accepts, to be used for shell completions.
#[derive(Clone, Copy, Debug)]
enum CompletionType {
    /// No special category. is_switch() and doc_choices() may apply.
    Other,
    /// A path to a file.
    Filename,
    /// A command in $PATH.
    Executable,
    /// The name of a file type, as used by e.g. --type.
    Filetype,
    /// The name of an encoding_rs encoding, as used by --encoding.
    Encoding,
}

/// Represents a value parsed from the command line.
///
/// This doesn't include the corresponding flag, but values come in one of
/// two forms: a switch (on or off) or an arbitrary value.
///
/// Note that the CLI doesn't directly support negated switches. For example,
/// you can'd do anything like `-n=false` or any of that nonsense. Instead,
/// the CLI parser knows about which flag names are negations and which aren't
/// (courtesy of the `Flag` trait). If a flag given is known as a negation,
/// then a `FlagValue::Switch(false)` value is passed into `Flag::update`.
#[derive(Debug)]
enum FlagValue {
    /// A flag that is either on or off.
    Switch(bool),
    /// A flag that comes with an arbitrary user value.
    Value(OsString),
}

impl FlagValue {
    /// Return the yes or no value of this switch.
    ///
    /// If this flag value is not a switch, then this panics.
    ///
    /// This is useful when writing the implementation of `Flag::update`.
    /// namely, callers usually know whether a switch or a value is expected.
    /// If a flag is something different, then it indicates a bug, and thus a
    /// panic is acceptable.
    fn unwrap_switch(self) -> bool {
        match self {
            FlagValue::Switch(yes) => yes,
            FlagValue::Value(_) => {
                unreachable!("got flag value but expected switch")
            }
        }
    }

    /// Return the user provided value of this flag.
    ///
    /// If this flag is a switch, then this panics.
    ///
    /// This is useful when writing the implementation of `Flag::update`.
    /// namely, callers usually know whether a switch or a value is expected.
    /// If a flag is something different, then it indicates a bug, and thus a
    /// panic is acceptable.
    fn unwrap_value(self) -> OsString {
        match self {
            FlagValue::Switch(_) => {
                unreachable!("got switch but expected flag value")
            }
            FlagValue::Value(v) => v,
        }
    }
}

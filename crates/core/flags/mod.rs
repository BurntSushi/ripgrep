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
    /// Every category in the single fixed order that every generator must use.
    ///
    /// This is the enum's declaration order. It is the authoritative ordering
    /// of categories shared by every generator so that the categories are
    /// emitted identically across every artifact and every invocation
    /// (Requirement 7.3).
    const ALL: &'static [Category] = &[
        Category::Input,
        Category::Search,
        Category::Filter,
        Category::Output,
        Category::OutputModes,
        Category::Logging,
        Category::OtherBehaviors,
    ];

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

/// A structural problem detected while validating the flag registry.
///
/// `RegistryView::load` only fails for the conditions enumerated here:
/// duplicate long, short or negated names (Requirement 1.9) and
/// runtime-checkable missing mandatory fields (Requirement 1.10). It does not
/// reject a flag for any other condition such as field format, value
/// constraints, or cross-field dependencies (Requirement 1.11).
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)] // wired into generators by later tasks
enum RegistryError {
    /// Two or more flags share the same long name.
    DuplicateLong { name: String },
    /// Two or more flags share the same short name.
    DuplicateShort { name: char },
    /// Two or more flags share the same negated name.
    DuplicateNegated { name: String },
    /// A flag is missing a mandatory field that can only be checked at
    /// runtime. `flag` identifies the offending flag (by its long name, or by
    /// its debug identity when the long name itself is the problem) and
    /// `field` names the missing field.
    MissingField { flag: String, field: &'static str },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RegistryError::DuplicateLong { ref name } => write!(
                f,
                "registry validation failed: duplicate long flag name \
                 '--{name}'"
            ),
            RegistryError::DuplicateShort { name } => write!(
                f,
                "registry validation failed: duplicate short flag name \
                 '-{name}'"
            ),
            RegistryError::DuplicateNegated { ref name } => write!(
                f,
                "registry validation failed: duplicate negated flag name \
                 '--{name}'"
            ),
            RegistryError::MissingField { ref flag, field } => write!(
                f,
                "registry validation failed: flag '{flag}' is missing \
                 mandatory field '{field}'"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// A validated, read-only view over the canonical flag registry (`FLAGS`).
///
/// Construction (via [`RegistryView::load`]) performs registry-wide validation
/// (Requirement 1.9, 1.10). All generators and the consistency checker are
/// intended to consume this view rather than touching `FLAGS` directly, so
/// that validation runs exactly once per generation and so that flag and
/// category ordering is defined in a single shared place.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // wired into generators by later tasks
struct RegistryView {
    flags: &'static [&'static dyn Flag],
}

#[allow(dead_code)] // wired into generators by later tasks
impl RegistryView {
    /// Build a view over ripgrep's canonical registry, validating it first.
    ///
    /// Returns an error describing the first structural problem (a duplicate
    /// long, short or negated name, or a missing mandatory field). When
    /// validation fails, no view is produced and callers must propagate the
    /// error and emit no artifact.
    fn load() -> anyhow::Result<RegistryView> {
        RegistryView::new(crate::flags::defs::FLAGS)
    }

    /// Build a view over an arbitrary registry, validating it first.
    ///
    /// This is the shared validation entry point used by `load`. It is also
    /// the seam through which tests can validate synthetic registries.
    fn new(
        flags: &'static [&'static dyn Flag],
    ) -> anyhow::Result<RegistryView> {
        validate(flags)?;
        Ok(RegistryView { flags })
    }

    /// Iterate the flags in registry (declaration) order.
    fn iter(&self) -> impl Iterator<Item = &'static dyn Flag> + '_ {
        self.flags.iter().copied()
    }

    /// Iterate the flags grouped by category, with categories in the fixed
    /// declaration order and flags within each category in registry
    /// (declaration) order (Requirements 7.2 and 7.3).
    ///
    /// Only categories that contain at least one flag are emitted. This is the
    /// shared ordering authority: a single registry edit propagates to every
    /// generator that consumes this method.
    fn by_category(
        &self,
    ) -> impl Iterator<Item = (Category, Vec<&'static dyn Flag>)> + '_ {
        Category::ALL.iter().copied().filter_map(move |cat| {
            let flags: Vec<&'static dyn Flag> =
                self.iter().filter(|f| f.doc_category() == cat).collect();
            if flags.is_empty() { None } else { Some((cat, flags)) }
        })
    }

    /// Resolve a long name (or alias) to its flag, used by the markup
    /// renderer. Returns `None` if no flag matches.
    fn lookup_long(&self, name: &str) -> Option<&'static dyn Flag> {
        self.iter().find(|f| {
            f.name_long() == name || f.aliases().iter().any(|&a| a == name)
        })
    }
}

/// Performs registry-wide validation over `flags`.
///
/// Returns the first structural problem found, scanning flags in declaration
/// order. The only failure conditions are duplicate long/short/negated names
/// (Requirement 1.9) and runtime-checkable missing mandatory fields
/// (Requirement 1.10).
#[allow(dead_code)] // wired into generators by later tasks
fn validate(flags: &[&'static dyn Flag]) -> Result<(), RegistryError> {
    use std::collections::{BTreeMap, BTreeSet};

    let mut longs: BTreeSet<&'static str> = BTreeSet::new();
    let mut shorts: BTreeMap<u8, ()> = BTreeMap::new();
    let mut negated: BTreeSet<&'static str> = BTreeSet::new();

    for flag in flags.iter().copied() {
        let long = flag.name_long();
        // Identify the flag for missing-field diagnostics. When the long name
        // itself is missing, fall back to the flag's debug identity.
        let id = if long.is_empty() {
            format!("{flag:?}")
        } else {
            long.to_string()
        };

        // Runtime-checkable missing mandatory fields (Requirement 1.10).
        // The long name and long documentation are mandatory and non-empty;
        // the short documentation is explicitly allowed to be empty
        // (Requirement 2.5) so it is not checked here.
        if long.is_empty() {
            return Err(RegistryError::MissingField {
                flag: id,
                field: "name_long",
            });
        }
        if flag.doc_long().trim().is_empty() {
            return Err(RegistryError::MissingField {
                flag: id,
                field: "doc_long",
            });
        }
        // A non-switch flag must name the value it accepts.
        if !flag.is_switch() && flag.doc_variable().is_none() {
            return Err(RegistryError::MissingField {
                flag: id,
                field: "doc_variable",
            });
        }

        // Duplicate long name (Requirement 1.9).
        if !longs.insert(long) {
            return Err(RegistryError::DuplicateLong {
                name: long.to_string(),
            });
        }
        // Duplicate short name (Requirement 1.9).
        if let Some(short) = flag.name_short() {
            if shorts.insert(short, ()).is_some() {
                return Err(RegistryError::DuplicateShort {
                    name: char::from(short),
                });
            }
        }
        // Duplicate negated name (Requirement 1.9).
        if let Some(neg) = flag.name_negated() {
            if !negated.insert(neg) {
                return Err(RegistryError::DuplicateNegated {
                    name: neg.to_string(),
                });
            }
        }
    }
    Ok(())
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

#[cfg(test)]
mod registry_tests {
    use super::*;

    /// A configurable synthetic flag used to build synthetic registries for
    /// exercising registry validation and the read-only view.
    #[derive(Debug)]
    struct TestFlag {
        long: &'static str,
        short: Option<u8>,
        negated: Option<&'static str>,
        switch: bool,
        variable: Option<&'static str>,
        category: Category,
        short_doc: &'static str,
        long_doc: &'static str,
        aliases: &'static [&'static str],
        completion: CompletionType,
        choices: &'static [&'static str],
    }

    impl TestFlag {
        /// A minimal valid switch flag in the given category with the given
        /// long name.
        fn switch(long: &'static str, category: Category) -> TestFlag {
            TestFlag {
                long,
                short: None,
                negated: None,
                switch: true,
                variable: None,
                category,
                short_doc: "short doc",
                long_doc: "long doc",
                aliases: &[],
                completion: CompletionType::Other,
                choices: &[],
            }
        }
    }

    impl Flag for TestFlag {
        fn is_switch(&self) -> bool {
            self.switch
        }
        fn name_short(&self) -> Option<u8> {
            self.short
        }
        fn name_long(&self) -> &'static str {
            self.long
        }
        fn aliases(&self) -> &'static [&'static str] {
            self.aliases
        }
        fn name_negated(&self) -> Option<&'static str> {
            self.negated
        }
        fn doc_variable(&self) -> Option<&'static str> {
            self.variable
        }
        fn doc_category(&self) -> Category {
            self.category
        }
        fn doc_short(&self) -> &'static str {
            self.short_doc
        }
        fn doc_long(&self) -> &'static str {
            self.long_doc
        }
        fn doc_choices(&self) -> &'static [&'static str] {
            self.choices
        }
        fn completion_type(&self) -> CompletionType {
            self.completion
        }
        fn update(
            &self,
            _: FlagValue,
            _: &mut crate::flags::lowargs::LowArgs,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// Leak the given test flags into a `'static` registry slice so they can be
    /// validated and viewed exactly like the real `FLAGS` registry.
    fn registry(flags: Vec<TestFlag>) -> &'static [&'static dyn Flag] {
        let leaked: Vec<&'static dyn Flag> = flags
            .into_iter()
            .map(|f| &*Box::leak(Box::new(f)) as &'static dyn Flag)
            .collect();
        Box::leak(leaked.into_boxed_slice())
    }

    #[test]
    fn real_registry_loads_and_iterates() {
        let view = RegistryView::load().expect("real registry must validate");
        assert_eq!(view.iter().count(), crate::flags::defs::FLAGS.len());
    }

    #[test]
    fn by_category_orders_categories_then_flags() {
        // Deliberately out of category order in the registry; flags within a
        // category are in declaration order.
        let flags = registry(vec![
            TestFlag::switch("output-b", Category::Output),
            TestFlag::switch("input-a", Category::Input),
            TestFlag::switch("output-a", Category::Output),
            TestFlag::switch("input-b", Category::Input),
        ]);
        let view = RegistryView::new(flags).unwrap();

        let grouped: Vec<(Category, Vec<&'static str>)> = view
            .by_category()
            .map(|(cat, fs)| (cat, fs.iter().map(|f| f.name_long()).collect()))
            .collect();

        // Input precedes Output (declaration order), and within each category
        // flags appear in registry order.
        assert_eq!(
            grouped,
            vec![
                (Category::Input, vec!["input-a", "input-b"]),
                (Category::Output, vec!["output-b", "output-a"]),
            ]
        );
    }

    #[test]
    fn lookup_long_resolves_name_and_alias() {
        let flags = registry(vec![TestFlag {
            aliases: &["legacy-name"],
            ..TestFlag::switch("real-name", Category::Search)
        }]);
        let view = RegistryView::new(flags).unwrap();

        assert_eq!(
            view.lookup_long("real-name").unwrap().name_long(),
            "real-name"
        );
        assert_eq!(
            view.lookup_long("legacy-name").unwrap().name_long(),
            "real-name"
        );
        assert!(view.lookup_long("does-not-exist").is_none());
    }

    #[test]
    fn detects_duplicate_long() {
        let flags = registry(vec![
            TestFlag::switch("dup", Category::Search),
            TestFlag::switch("dup", Category::Output),
        ]);
        assert_eq!(
            validate(flags).unwrap_err(),
            RegistryError::DuplicateLong { name: "dup".to_string() }
        );
        assert!(RegistryView::new(flags).is_err());
    }

    #[test]
    fn detects_duplicate_short() {
        let flags = registry(vec![
            TestFlag {
                short: Some(b'x'),
                ..TestFlag::switch("alpha", Category::Search)
            },
            TestFlag {
                short: Some(b'x'),
                ..TestFlag::switch("beta", Category::Search)
            },
        ]);
        assert_eq!(
            validate(flags).unwrap_err(),
            RegistryError::DuplicateShort { name: 'x' }
        );
    }

    #[test]
    fn detects_duplicate_negated() {
        let flags = registry(vec![
            TestFlag {
                negated: Some("no-thing"),
                ..TestFlag::switch("thing", Category::Search)
            },
            TestFlag {
                negated: Some("no-thing"),
                ..TestFlag::switch("other", Category::Search)
            },
        ]);
        assert_eq!(
            validate(flags).unwrap_err(),
            RegistryError::DuplicateNegated { name: "no-thing".to_string() }
        );
    }

    #[test]
    fn detects_missing_value_variable() {
        // A non-switch flag must name its value variable.
        let flags = registry(vec![TestFlag {
            switch: false,
            variable: None,
            ..TestFlag::switch("needs-value", Category::Search)
        }]);
        assert_eq!(
            validate(flags).unwrap_err(),
            RegistryError::MissingField {
                flag: "needs-value".to_string(),
                field: "doc_variable",
            }
        );
    }

    #[test]
    fn detects_missing_long_name() {
        let flags = registry(vec![TestFlag::switch("", Category::Search)]);
        let err = validate(flags).unwrap_err();
        match err {
            RegistryError::MissingField { field, .. } => {
                assert_eq!(field, "name_long")
            }
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn detects_missing_long_doc() {
        let flags = registry(vec![TestFlag {
            long_doc: "   ",
            ..TestFlag::switch("has-empty-doc", Category::Search)
        }]);
        assert_eq!(
            validate(flags).unwrap_err(),
            RegistryError::MissingField {
                flag: "has-empty-doc".to_string(),
                field: "doc_long",
            }
        );
    }

    #[test]
    fn empty_short_doc_is_allowed() {
        // Requirement 2.5: short documentation may be empty.
        let flags = registry(vec![TestFlag {
            short_doc: "",
            ..TestFlag::switch("ok", Category::Search)
        }]);
        assert!(validate(flags).is_ok());
    }

    // ---------------------------------------------------------------------
    // Synthetic-registry proptest strategy (test infrastructure, Task 1.2).
    //
    // The strategy produces synthetic registries (vectors of generated
    // `Flag_Definition`s, leaked into a `'static` slice exactly like the real
    // `FLAGS`) so that downstream property tests can run the real
    // generation/validation/checking logic against many inputs. It also
    // provides variants that inject duplicate names, invalidate a mandatory
    // field, and perturb a generated artifact, to drive the negative
    // properties (supports Properties 1-25).
    // ---------------------------------------------------------------------

    use proptest::prelude::*;

    /// An owned, generated flag definition. This mirrors [`TestFlag`] but owns
    /// its strings so it can be produced by `proptest`. Call
    /// [`SyntheticFlag::leak`] (or [`build_registry`]) to obtain a `'static`
    /// view suitable for [`RegistryView`] and the generators.
    #[derive(Clone, Debug)]
    #[allow(dead_code)] // consumed by downstream property-test tasks
    pub(super) struct SyntheticFlag {
        pub long: String,
        pub short: Option<u8>,
        pub negated: Option<String>,
        pub switch: bool,
        pub variable: Option<String>,
        pub category: Category,
        pub short_doc: String,
        pub long_doc: String,
        pub aliases: Vec<String>,
        pub completion: CompletionType,
        pub choices: Vec<String>,
    }

    /// Leak an owned string into a `'static` string slice.
    fn leak_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// Leak an owned vector of strings into a `'static` slice of `'static`
    /// string slices.
    fn leak_strs(v: Vec<String>) -> &'static [&'static str] {
        let leaked: Vec<&'static str> = v.into_iter().map(leak_str).collect();
        Box::leak(leaked.into_boxed_slice())
    }

    impl SyntheticFlag {
        /// Convert this owned definition into a `'static` [`TestFlag`] by
        /// leaking its owned strings.
        fn into_test_flag(self) -> TestFlag {
            TestFlag {
                long: leak_str(self.long),
                short: self.short,
                negated: self.negated.map(leak_str),
                switch: self.switch,
                variable: self.variable.map(leak_str),
                category: self.category,
                short_doc: leak_str(self.short_doc),
                long_doc: leak_str(self.long_doc),
                aliases: leak_strs(self.aliases),
                completion: self.completion,
                choices: leak_strs(self.choices),
            }
        }
    }

    /// Build a `'static` registry slice from owned synthetic flags, leaking as
    /// needed. This is the synthetic counterpart of the real `FLAGS` slice and
    /// is accepted by [`RegistryView::new`] and every generator.
    #[allow(dead_code)] // consumed by downstream property-test tasks
    pub(super) fn build_registry(
        flags: Vec<SyntheticFlag>,
    ) -> &'static [&'static dyn Flag] {
        registry(
            flags.into_iter().map(SyntheticFlag::into_test_flag).collect(),
        )
    }

    /// The pool of distinct short-name bytes, used to keep generated short
    /// names unique within a registry. ASCII alphanumerics satisfy the
    /// `Flag::name_short` contract.
    fn short_pool() -> Vec<u8> {
        let mut pool = Vec::new();
        pool.extend(b'a'..=b'z');
        pool.extend(b'A'..=b'Z');
        pool.extend(b'0'..=b'9');
        pool
    }

    /// Every category, for selection by the strategy.
    fn any_category() -> impl Strategy<Value = Category> {
        prop_oneof![
            Just(Category::Input),
            Just(Category::Search),
            Just(Category::Filter),
            Just(Category::Output),
            Just(Category::OutputModes),
            Just(Category::Logging),
            Just(Category::OtherBehaviors),
        ]
    }

    /// A non-`Choices` completion kind. The `Choices` classification from the
    /// requirements is represented as `CompletionType::Other` plus a non-empty
    /// `doc_choices`, so it is modeled separately by `wants_choices` below.
    fn any_completion() -> impl Strategy<Value = CompletionType> {
        prop_oneof![
            Just(CompletionType::Other),
            Just(CompletionType::Filename),
            Just(CompletionType::Executable),
            Just(CompletionType::Filetype),
            Just(CompletionType::Encoding),
        ]
    }

    /// Short documentation, including the empty case (Requirement 2.5) and
    /// strings with interior whitespace (so description-equality properties
    /// are exercised meaningfully).
    fn short_doc_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(String::new()),
            Just("short doc".to_string()),
            Just("does a thing".to_string()),
            Just("Toggle  spaced   words".to_string()),
            Just("UPPER and lower".to_string()),
        ]
    }

    /// The random, name-independent attributes of a single synthetic flag. The
    /// names themselves are assigned during normalization so that they are
    /// unique within the registry (keeping the registry valid by default).
    #[derive(Clone, Debug)]
    struct RawFlag {
        wants_short: bool,
        wants_negated: bool,
        switch: bool,
        category: Category,
        short_doc: String,
        num_aliases: usize,
        hyphen_rich: bool,
        completion: CompletionType,
        wants_choices: bool,
        choices: Vec<String>,
    }

    fn raw_flag() -> impl Strategy<Value = RawFlag> {
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any_category(),
            short_doc_strategy(),
            0usize..3,
            any::<bool>(),
            any_completion(),
            any::<bool>(),
            prop::collection::vec("[a-z]{1,5}", 1..4),
        )
            .prop_map(
                |(
                    wants_short,
                    wants_negated,
                    switch,
                    category,
                    short_doc,
                    num_aliases,
                    hyphen_rich,
                    completion,
                    wants_choices,
                    choices,
                )| {
                    RawFlag {
                        wants_short,
                        wants_negated,
                        switch,
                        category,
                        short_doc,
                        num_aliases,
                        hyphen_rich,
                        completion,
                        wants_choices,
                        choices,
                    }
                },
            )
    }

    /// Normalize a vector of raw flags into a valid synthetic registry by
    /// assigning unique, well-formed names derived from each flag's index.
    fn normalize(raws: Vec<RawFlag>) -> Vec<SyntheticFlag> {
        let pool = short_pool();
        raws.into_iter()
            .enumerate()
            .map(|(i, raw)| {
                // Long names are unique because they embed the index. Some are
                // deliberately hyphen-rich to exercise roff escaping.
                let long = if raw.hyphen_rich {
                    format!("flag-{i}-a-b-c")
                } else {
                    format!("flag{i}long")
                };
                let short = if raw.wants_short && i < pool.len() {
                    Some(pool[i])
                } else {
                    None
                };
                let negated = if raw.wants_negated {
                    Some(format!("no-{long}"))
                } else {
                    None
                };
                // A non-switch flag must name the value it accepts; a switch
                // must not (keeps the registry valid by default).
                let variable =
                    if raw.switch { None } else { Some(format!("VAL{i}")) };
                let aliases = (0..raw.num_aliases)
                    .map(|j| format!("{long}-alias{j}"))
                    .collect();
                // Choices only make sense for value flags. When requested,
                // dedupe and prefix by index to keep them well-formed.
                let choices = if !raw.switch && raw.wants_choices {
                    let mut seen = std::collections::BTreeSet::new();
                    raw.choices
                        .iter()
                        .filter(|c| seen.insert((*c).clone()))
                        .enumerate()
                        .map(|(j, c)| format!("{c}{j}"))
                        .collect()
                } else {
                    Vec::new()
                };
                SyntheticFlag {
                    long,
                    short,
                    negated,
                    switch: raw.switch,
                    variable,
                    category: raw.category,
                    short_doc: raw.short_doc,
                    long_doc: format!("long documentation for flag {i}"),
                    aliases,
                    completion: raw.completion,
                    choices,
                }
            })
            .collect()
    }

    /// Strategy producing a valid synthetic registry: a non-empty vector of
    /// synthetic flags with unique long/short/negated names and all mandatory
    /// fields present. Loading this registry through [`RegistryView::new`]
    /// always succeeds.
    pub(super) fn synthetic_registry()
    -> impl Strategy<Value = Vec<SyntheticFlag>> {
        prop::collection::vec(raw_flag(), 1..8).prop_map(normalize)
    }

    /// The kind of duplicate-name conflict injected into a registry.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[allow(dead_code)] // consumed by downstream property-test tasks
    pub(super) enum DuplicateKind {
        Long,
        Short,
        Negated,
    }

    /// Strategy producing a synthetic registry that contains a duplicate
    /// long, short, or negated name, paired with the kind of duplicate
    /// injected. Loading this registry always fails validation
    /// (drives Property 2).
    #[allow(dead_code)] // consumed by downstream property-test tasks
    pub(super) fn synthetic_registry_with_duplicate()
    -> impl Strategy<Value = (Vec<SyntheticFlag>, DuplicateKind)> {
        let kind = prop_oneof![
            Just(DuplicateKind::Long),
            Just(DuplicateKind::Short),
            Just(DuplicateKind::Negated),
        ];
        (synthetic_registry(), kind).prop_map(|(mut flags, kind)| {
            // Use the first flag as the source of the duplicated value, and
            // append a fresh (otherwise-unique) flag that collides on exactly
            // one field.
            let mut dup = flags[0].clone();
            dup.long = format!("dup-{}", flags.len());
            dup.negated = None;
            dup.short = None;
            match kind {
                DuplicateKind::Long => {
                    dup.long = flags[0].long.clone();
                }
                DuplicateKind::Short => {
                    // Ensure the source flag has a short name to collide with.
                    flags[0].short = Some(b'z');
                    dup.short = Some(b'z');
                }
                DuplicateKind::Negated => {
                    // Ensure the source flag has a negated name to collide
                    // with.
                    flags[0].negated = Some("no-collision".to_string());
                    dup.negated = Some("no-collision".to_string());
                }
            }
            flags.push(dup);
            (flags, kind)
        })
    }

    /// Strategy producing a synthetic registry in which one flag is missing a
    /// runtime-checkable mandatory field, paired with the name of the offending
    /// field as reported by validation (drives Property 3).
    #[allow(dead_code)] // consumed by downstream property-test tasks
    pub(super) fn synthetic_registry_missing_field()
    -> impl Strategy<Value = (Vec<SyntheticFlag>, &'static str)> {
        let which = prop_oneof![
            Just("name_long"),
            Just("doc_long"),
            Just("doc_variable"),
        ];
        (synthetic_registry(), which).prop_map(|(mut flags, field)| {
            let last = flags.len() - 1;
            match field {
                "name_long" => {
                    flags[last].long = String::new();
                }
                "doc_long" => {
                    flags[last].long_doc = "   ".to_string();
                }
                "doc_variable" => {
                    // A non-switch flag with no value variable.
                    flags[last].switch = false;
                    flags[last].variable = None;
                }
                _ => unreachable!(),
            }
            (flags, field)
        })
    }

    /// A perturbation applied to a generated artifact's text, used to inject
    /// the divergences the `Consistency_Checker` must detect (drives
    /// Properties 16-20). Each variant carries enough information to be applied
    /// to an artifact string via [`apply_perturbation`].
    #[derive(Clone, Debug)]
    #[allow(dead_code)] // consumed by downstream checker property-test tasks
    pub(super) enum ArtifactPerturbation {
        /// Drop every line mentioning `flag`, simulating a missing flag.
        DropFlag { flag: String },
        /// Rewrite a flag name to an unexpected one not in the registry.
        RenameFlag { from: String, to: String },
        /// Replace a flag's description text with mismatched text.
        AlterDescription { from: String, to: String },
        /// Append an extra line referencing a flag absent from the registry.
        AddUnexpected { line: String },
    }

    /// Apply a perturbation to an artifact's text, returning the perturbed
    /// text. This is intentionally format-agnostic string surgery so it can be
    /// reused across every shell artifact by the checker property tests.
    #[allow(dead_code)] // consumed by downstream checker property-test tasks
    pub(super) fn apply_perturbation(
        artifact: &str,
        perturbation: &ArtifactPerturbation,
    ) -> String {
        match perturbation {
            ArtifactPerturbation::DropFlag { flag } => artifact
                .lines()
                .filter(|line| !line.contains(flag.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            ArtifactPerturbation::RenameFlag { from, to } => {
                artifact.replace(from.as_str(), to.as_str())
            }
            ArtifactPerturbation::AlterDescription { from, to } => {
                artifact.replace(from.as_str(), to.as_str())
            }
            ArtifactPerturbation::AddUnexpected { line } => {
                format!("{artifact}\n{line}")
            }
        }
    }

    /// Strategy producing artifact perturbations parameterized by a known flag
    /// name. The chosen flag drives which divergence is injected.
    #[allow(dead_code)] // consumed by downstream checker property-test tasks
    pub(super) fn artifact_perturbation(
        flag: String,
    ) -> impl Strategy<Value = ArtifactPerturbation> {
        prop_oneof![
            Just(ArtifactPerturbation::DropFlag { flag: flag.clone() }),
            Just(ArtifactPerturbation::RenameFlag {
                from: flag.clone(),
                to: "totally-unexpected-flag".to_string(),
            }),
            Just(ArtifactPerturbation::AlterDescription {
                from: flag.clone(),
                to: format!("{flag}-PERTURBED"),
            }),
            Just(ArtifactPerturbation::AddUnexpected {
                line: "--totally-unexpected-flag".to_string(),
            }),
        ]
    }

    proptest! {
        /// Infrastructure check: the synthetic-registry strategy always
        /// produces a registry that passes validation.
        #[test]
        fn synthetic_registry_is_valid(flags in synthetic_registry()) {
            let reg = build_registry(flags);
            prop_assert!(RegistryView::new(reg).is_ok());
        }

        /// Infrastructure check: the duplicate-name variant always produces a
        /// registry that fails validation.
        #[test]
        fn duplicate_variant_fails_validation(
            (flags, _kind) in synthetic_registry_with_duplicate(),
        ) {
            let reg = build_registry(flags);
            prop_assert!(validate(reg).is_err());
        }

        /// Infrastructure check: the missing-field variant always produces a
        /// registry that fails validation.
        #[test]
        fn missing_field_variant_fails_validation(
            (flags, _field) in synthetic_registry_missing_field(),
        ) {
            let reg = build_registry(flags);
            prop_assert!(validate(reg).is_err());
        }

        /// Infrastructure check: perturbing an artifact changes its text in the
        /// expected direction for each perturbation kind.
        #[test]
        fn perturbation_changes_artifact(
            perturbation in artifact_perturbation("flag0long".to_string()),
        ) {
            let artifact = "--flag0long\tshort doc\n--other\tother doc";
            let perturbed = apply_perturbation(artifact, &perturbation);
            match perturbation {
                ArtifactPerturbation::DropFlag { .. } => {
                    prop_assert!(!perturbed.contains("flag0long"));
                }
                ArtifactPerturbation::RenameFlag { .. } => {
                    prop_assert!(perturbed.contains("totally-unexpected-flag"));
                }
                ArtifactPerturbation::AlterDescription { .. } => {
                    prop_assert!(perturbed.contains("PERTURBED"));
                }
                ArtifactPerturbation::AddUnexpected { .. } => {
                    prop_assert!(perturbed.contains("--totally-unexpected-flag"));
                }
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 2: Duplicate names fail validation
        //
        // For any registry containing two or more flags that share a long
        // name, or share a short name, or share a negated name, registry
        // validation fails with an error identifying the conflicting field and
        // value, and no view (artifact) is produced.
        #[test]
        fn duplicate_names_fail_validation(
            (flags, kind) in synthetic_registry_with_duplicate(),
        ) {
            // Capture the conflicting long value before the flags are leaked
            // into the registry, so we can assert the error names it. The
            // duplicate variant collides the appended flag with the first
            // flag's value on exactly the injected field.
            let expected_long = flags[0].long.clone();
            let reg = build_registry(flags);

            // Validation must fail, and so must `RegistryView` construction,
            // ensuring no artifact is ever produced from an invalid registry.
            let err =
                validate(reg).expect_err("duplicate registry must fail");
            prop_assert!(RegistryView::new(reg).is_err());

            // The error variant must match the injected duplicate kind and
            // name the conflicting value.
            match kind {
                DuplicateKind::Long => prop_assert_eq!(
                    err,
                    RegistryError::DuplicateLong { name: expected_long },
                ),
                DuplicateKind::Short => prop_assert_eq!(
                    err,
                    RegistryError::DuplicateShort { name: 'z' },
                ),
                DuplicateKind::Negated => prop_assert_eq!(
                    err,
                    RegistryError::DuplicateNegated {
                        name: "no-collision".to_string(),
                    },
                ),
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 3: Missing mandatory field halts generation
        //
        // For any registry in which a flag is missing a runtime-checkable
        // mandatory field, each generator halts without producing its artifact
        // and returns an error identifying the affected flag and the missing
        // field. Generators consume `RegistryView::new`/`load`, which runs
        // validation and yields no view on failure; asserting that
        // `RegistryView::new` errors therefore demonstrates that generation
        // halts with no artifact. The injected field name is threaded through
        // the strategy so we can assert the error names exactly that field.
        #[test]
        fn missing_field_halts_generation(
            (flags, injected_field) in synthetic_registry_missing_field(),
        ) {
            let reg = build_registry(flags);

            // Validation must fail, and `RegistryView` construction (the entry
            // point every generator calls before emitting anything) must fail
            // too, so no artifact is produced.
            let err =
                validate(reg).expect_err("missing-field registry must fail");
            prop_assert!(RegistryView::new(reg).is_err());

            // The error must identify the missing mandatory field, and the
            // reported field must match the one the strategy injected.
            match err {
                RegistryError::MissingField { field, .. } => {
                    prop_assert_eq!(field, injected_field);
                }
                other => prop_assert!(
                    false,
                    "expected MissingField, got {:?}",
                    other
                ),
            }
        }
    }

    // ---------------------------------------------------------------------
    // Single-edit propagation (Task 7.3, Property 1).
    //
    // A single edit to the registry (adding or removing exactly one flag) is
    // observed in every generated artifact: the edited flag's reference flips
    // in each artifact, while every other flag's reference is unchanged. The
    // generators themselves are never touched, so this demonstrates that the
    // registry is the sole source the artifacts derive from.
    // ---------------------------------------------------------------------

    /// Whether `long` is referenced as a flag in `artifact`.
    ///
    /// Most generators emit a flag's long name verbatim, but the man generator
    /// escapes each hyphen in a flag name for roff (`-` becomes `\-`). To stay
    /// artifact-agnostic, this checks for either the literal name or its
    /// roff-escaped form. Synthetic long names embed their flag's index, so no
    /// flag's name is a substring of another's, making `contains` a sound
    /// presence test here (see `normalize`).
    fn references(artifact: &str, long: &str) -> bool {
        artifact.contains(long) || artifact.contains(&long.replace('-', r"\-"))
    }

    /// Generate every flag-facing artifact (the four shell completions, the man
    /// page, and the long help) from a single registry view, in a fixed order.
    fn all_artifacts(view: &RegistryView) -> Vec<String> {
        vec![
            crate::flags::complete::bash::generate_with(view),
            crate::flags::complete::zsh::generate_with(view),
            crate::flags::complete::fish::generate_with(view),
            crate::flags::complete::powershell::generate_with(view),
            crate::flags::doc::man::generate_with(view),
            crate::flags::doc::help::generate_long_with(view),
        ]
    }

    /// Strategy producing a base registry and an edited registry that differs
    /// from it by exactly one flag, along with that flag's long name and
    /// whether the edit was an addition (`true`) or a removal (`false`).
    ///
    /// The base is always valid; the edit keeps it valid. A removal is only
    /// chosen when the base has at least two flags (so the result keeps >= 1
    /// flag); otherwise an addition is performed. An addition appends a fresh
    /// flag whose names cannot collide with, or be a substring of, any
    /// synthetic name produced by `normalize`.
    fn registry_with_single_edit()
    -> impl Strategy<Value = (Vec<SyntheticFlag>, Vec<SyntheticFlag>, String, bool)>
    {
        (synthetic_registry(), any::<bool>(), 0usize..8).prop_map(
            |(base, prefer_add, raw_idx)| {
                // Force an addition when removal could not leave >= 1 flag.
                let do_add = prefer_add || base.len() < 2;
                if do_add {
                    let fresh = SyntheticFlag {
                        long: "zzfreshaddedflag".to_string(),
                        short: None,
                        negated: None,
                        switch: true,
                        variable: None,
                        category: Category::Search,
                        short_doc: "freshly added flag".to_string(),
                        long_doc: "long documentation for the fresh flag"
                            .to_string(),
                        aliases: Vec::new(),
                        completion: CompletionType::Other,
                        choices: Vec::new(),
                    };
                    let long = fresh.long.clone();
                    let mut edited = base.clone();
                    edited.push(fresh);
                    (base, edited, long, true)
                } else {
                    let i = raw_idx % base.len();
                    let removed_long = base[i].long.clone();
                    let mut edited = base.clone();
                    edited.remove(i);
                    (base, edited, removed_long, false)
                }
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 1: Single edit propagates to every artifact
        //
        // For any registry and any single edit to it (adding or removing one
        // flag), regenerating each artifact yields a referenced-flag set that
        // reflects exactly that edit, with no change to any generator's code.
        #[test]
        fn single_edit_propagates_to_every_artifact(
            (base, edited, edited_long, is_add) in registry_with_single_edit(),
        ) {
            // The long names of the flags shared by both registries (i.e.
            // every flag except the one that was added or removed).
            let other_longs: Vec<String> = base
                .iter()
                .map(|f| f.long.clone())
                .filter(|l| *l != edited_long)
                .collect();

            let base_reg = build_registry(base);
            let edited_reg = build_registry(edited);
            let base_view = RegistryView::new(base_reg)
                .expect("base registry must validate");
            let edited_view = RegistryView::new(edited_reg)
                .expect("edited registry must validate");

            let base_artifacts = all_artifacts(&base_view);
            let edited_artifacts = all_artifacts(&edited_view);

            for (b, e) in
                base_artifacts.iter().zip(edited_artifacts.iter())
            {
                let before = references(b, &edited_long);
                let after = references(e, &edited_long);

                // The edited flag's reference flips in exactly the direction
                // of the edit, in every artifact.
                if is_add {
                    prop_assert!(
                        !before,
                        "added flag {:?} must be absent before the edit",
                        edited_long
                    );
                    prop_assert!(
                        after,
                        "added flag {:?} must appear after the edit",
                        edited_long
                    );
                } else {
                    prop_assert!(
                        before,
                        "removed flag {:?} must be present before the edit",
                        edited_long
                    );
                    prop_assert!(
                        !after,
                        "removed flag {:?} must be gone after the edit",
                        edited_long
                    );
                }

                // Every other flag's reference is identical before and after
                // the edit: the single edit touches nothing else.
                for ol in &other_longs {
                    prop_assert_eq!(
                        references(b, ol),
                        references(e, ol),
                        "flag {:?} reference changed unexpectedly",
                        ol
                    );
                }
            }
        }
    }
}

/*!
Provides routines for generating ripgrep's "short" and "long" help
documentation.

The short version is used when the `-h` flag is given, while the long version
is used when the `--help` flag is given.
*/

use std::fmt::Write;

use crate::flags::{
    Flag, RegistryView,
    doc::{
        markup::{MarkupFlavor, render_markup},
        version,
    },
};

const TEMPLATE_SHORT: &'static str = include_str!("template.short.help");
const TEMPLATE_LONG: &'static str = include_str!("template.long.help");

/// Wraps `std::write!` and asserts there is no failure.
///
/// We only write to `String` in this module.
macro_rules! write {
    ($($tt:tt)*) => { std::write!($($tt)*).unwrap(); }
}

/// Generate short documentation, i.e., for `-h`.
pub(crate) fn generate_short() -> String {
    // The registry is validated once per generation and is the single source
    // of truth for both flag ordering (via `by_category`) and markup
    // resolution. Short docs are resolved through the shared renderer so any
    // markup tags they contain are expanded identically to the long help
    // (Requirement 9.3); the real registry's short docs contain no markup, so
    // this leaves their text byte-stable.
    generate_short_with(
        &RegistryView::load()
            .expect("ripgrep's flag registry should validate"),
    )
}

/// Generate short documentation (`-h`) from the given `registry`.
///
/// This is the registry-accepting seam used by `generate_short` (which passes
/// the real `FLAGS` registry) and by property tests (which pass synthetic
/// registries).
pub(crate) fn generate_short_with(registry: &RegistryView) -> String {
    let mut cats: Vec<(&'static str, (Vec<String>, Vec<String>))> = vec![];
    let (mut maxcol1, mut maxcol2) = (0, 0);
    for (cat, flags) in registry.by_category() {
        let (mut col1s, mut col2s) = (vec![], vec![]);
        for flag in flags {
            let (col1, col2) = generate_short_flag(flag, registry);
            maxcol1 = maxcol1.max(col1.len());
            maxcol2 = maxcol2.max(col2.len());
            col1s.push(col1);
            col2s.push(col2);
        }
        cats.push((cat.as_str(), (col1s, col2s)));
    }
    let mut out =
        TEMPLATE_SHORT.replace("!!VERSION!!", &version::generate_digits());
    for (name, (col1, col2)) in cats.iter() {
        let var = format!("!!{name}!!");
        let val = format_short_columns(col1, col2, maxcol1, maxcol2);
        out = out.replace(&var, &val);
    }
    out
}

/// Generate short for a single flag.
///
/// The first element corresponds to the flag name while the second element
/// corresponds to the documentation string.
fn generate_short_flag(
    flag: &dyn Flag,
    registry: &RegistryView,
) -> (String, String) {
    let (mut col1, mut col2) = (String::new(), String::new());

    // Some of the variable names are fine for longer form
    // docs, but they make the succinct short help very noisy.
    // So just shorten some of them.
    let var = flag.doc_variable().map(|s| {
        let mut s = s.to_string();
        s = s.replace("SEPARATOR", "SEP");
        s = s.replace("REPLACEMENT", "TEXT");
        s = s.replace("NUM+SUFFIX?", "NUM");
        s
    });

    // Generate the first column, the flag name.
    if let Some(byte) = flag.name_short() {
        let name = char::from(byte);
        write!(col1, r"-{name}");
        write!(col1, r", ");
    }
    write!(col1, r"--{name}", name = flag.name_long());
    if let Some(var) = var.as_ref() {
        write!(col1, r"={var}");
    }

    // And now the second column, with the description. Resolve any
    // `\flag{..}`/`\flag-negate{..}` markup against the registry so short help
    // renders markup-resolved short docs (Requirement 9.3). The real
    // registry's docs are known-valid, so resolution must succeed here.
    let short = render_markup(flag.doc_short(), registry, MarkupFlavor::Plain)
        .expect("flag documentation markup should resolve");
    write!(col2, "{}", short);

    (col1, col2)
}

/// Write two columns of documentation.
///
/// `maxcol1` should be the maximum length (in bytes) of the first column,
/// while `maxcol2` should be the maximum length (in bytes) of the second
/// column.
fn format_short_columns(
    col1: &[String],
    col2: &[String],
    maxcol1: usize,
    _maxcol2: usize,
) -> String {
    assert_eq!(col1.len(), col2.len(), "columns must have equal length");
    const PAD: usize = 2;
    let mut out = String::new();
    for (i, (c1, c2)) in col1.iter().zip(col2.iter()).enumerate() {
        if i > 0 {
            write!(out, "\n");
        }

        let pad = maxcol1 - c1.len() + PAD;
        write!(out, "  ");
        write!(out, "{c1}");
        write!(out, "{}", " ".repeat(pad));
        write!(out, "{c2}");
    }
    out
}

/// Generate long documentation, i.e., for `--help`.
pub(crate) fn generate_long() -> String {
    // The registry is validated once per generation; all markup is resolved
    // against this single source of truth. `by_category` is the shared
    // ordering authority: categories in fixed declaration order and flags in
    // registry order, so each flag appears under exactly one matching category
    // heading (Requirements 7.2, 7.3, 9.2).
    generate_long_with(
        &RegistryView::load()
            .expect("ripgrep's flag registry should validate"),
    )
}

/// Generate long documentation (`--help`) from the given `registry`.
///
/// This is the registry-accepting seam used by `generate_long` (which passes
/// the real `FLAGS` registry) and by property tests (which pass synthetic
/// registries).
pub(crate) fn generate_long_with(registry: &RegistryView) -> String {
    let mut out =
        TEMPLATE_LONG.replace("!!VERSION!!", &version::generate_digits());
    for (cat, flags) in registry.by_category() {
        let mut value = String::new();
        for flag in flags {
            if !value.is_empty() {
                write!(value, "\n\n");
            }
            generate_long_flag(flag, registry, &mut value);
        }
        let var = format!("!!{name}!!", name = cat.as_str());
        out = out.replace(&var, &value);
    }
    out
}

/// Write generated documentation for `flag` to `out`.
fn generate_long_flag(
    flag: &dyn Flag,
    registry: &RegistryView,
    out: &mut String,
) {
    if let Some(byte) = flag.name_short() {
        let name = char::from(byte);
        write!(out, r"    -{name}");
        if let Some(var) = flag.doc_variable() {
            write!(out, r" {var}");
        }
        write!(out, r", ");
    } else {
        write!(out, r"    ");
    }

    let name = flag.name_long();
    write!(out, r"--{name}");
    if let Some(var) = flag.doc_variable() {
        write!(out, r"={var}");
    }
    write!(out, "\n");

    let doc = flag.doc_long().trim();
    // Resolve `\flag{..}` and `\flag-negate{..}` markup against the registry.
    // The real registry's documentation is known-valid, so resolution must
    // succeed here.
    let doc = render_markup(doc, registry, MarkupFlavor::Plain)
        .expect("flag documentation markup should resolve");

    let mut cleaned = remove_roff(&doc);
    if let Some(negated) = flag.name_negated() {
        // Flags that can be negated that aren't switches, like
        // --context-separator, are somewhat weird. Because of that, the docs
        // for those flags should discuss the semantics of negation explicitly.
        // But for switches, the behavior is always the same.
        if flag.is_switch() {
            write!(cleaned, "\n\nThis flag can be disabled with --{negated}.");
        }
    }
    let indent = " ".repeat(8);
    let wrapopts = textwrap::Options::new(71)
        // Normally I'd be fine with breaking at hyphens, but ripgrep's docs
        // includes a lot of flag names, and they in turn contain hyphens.
        // Breaking flag names across lines is not great.
        .word_splitter(textwrap::WordSplitter::NoHyphenation);
    for (i, paragraph) in cleaned.split("\n\n").enumerate() {
        if i > 0 {
            write!(out, "\n\n");
        }
        let mut new = paragraph.to_string();
        if paragraph.lines().all(|line| line.starts_with("    ")) {
            // Re-indent but don't refill so as to preserve line breaks
            // in code/shell example snippets.
            new = textwrap::indent(&new, &indent);
        } else {
            new = new.replace("\n", " ");
            new = textwrap::refill(&new, &wrapopts);
            new = textwrap::indent(&new, &indent);
        }
        write!(out, "{}", new.trim_end());
    }
}

/// Removes roff syntax from `v` such that the result is approximately plain
/// text readable.
///
/// This is basically a mish mash of heuristics based on the specific roff used
/// in the docs for the flags in this tool. If new kinds of roff are used in
/// the docs, then this may need to be updated to handle them.
fn remove_roff(v: &str) -> String {
    let mut lines = vec![];
    for line in v.trim().lines() {
        assert!(!line.is_empty(), "roff should have no empty lines");
        if line.starts_with(".") {
            if line.starts_with(".IP ") {
                let item_label = line
                    .split(" ")
                    .nth(1)
                    .expect("first argument to .IP")
                    .replace(r"\(bu", r"•")
                    .replace(r"\fB", "")
                    .replace(r"\fP", ":");
                lines.push(format!("{item_label}"));
            } else if line.starts_with(".IB ") || line.starts_with(".BI ") {
                let pieces = line
                    .split_whitespace()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .concat();
                lines.push(format!("{pieces}"));
            } else if line.starts_with(".sp")
                || line.starts_with(".PP")
                || line.starts_with(".TP")
            {
                lines.push("".to_string());
            }
        } else if line.starts_with(r"\fB") && line.ends_with(r"\fP") {
            let line = line.replace(r"\fB", "").replace(r"\fP", "");
            lines.push(format!("{line}:"));
        } else {
            lines.push(line.to_string());
        }
    }
    // Squash multiple adjacent paragraph breaks into one.
    lines.dedup_by(|l1, l2| l1.is_empty() && l2.is_empty());
    lines
        .join("\n")
        .replace(r"\fB", "")
        .replace(r"\fI", "")
        .replace(r"\fP", "")
        .replace(r"\-", "-")
        .replace(r"\\", r"\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::{Category, Flag, FlagValue, RegistryView};

    /// A minimal synthetic flag used to build synthetic registries for
    /// exercising the help generators in isolation. Unlike the fixed flags in
    /// `man.rs`'s tests, this carries owned-then-leaked short and long
    /// documentation so a property test can vary the docs (including embedding
    /// `\flag{..}` markup) and then assert on the rendered output.
    ///
    /// Every synthetic flag is a switch with no value variable, so synthetic
    /// registries always validate (a non-switch flag would require a value
    /// variable).
    #[derive(Debug)]
    struct TestFlag {
        long: &'static str,
        short: Option<u8>,
        doc_short: &'static str,
        doc_long: &'static str,
    }

    impl Flag for TestFlag {
        fn is_switch(&self) -> bool {
            true
        }
        fn name_short(&self) -> Option<u8> {
            self.short
        }
        fn name_long(&self) -> &'static str {
            self.long
        }
        fn doc_category(&self) -> Category {
            Category::Search
        }
        fn doc_short(&self) -> &'static str {
            self.doc_short
        }
        fn doc_long(&self) -> &'static str {
            self.doc_long
        }
        fn update(
            &self,
            _: FlagValue,
            _: &mut crate::flags::lowargs::LowArgs,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    use proptest::prelude::*;

    /// An owned, generated flag definition produced by `proptest`. Converted
    /// to a `'static` [`TestFlag`] by [`synth_view`].
    #[derive(Clone, Debug)]
    struct SynthFlag {
        long: String,
        short: Option<u8>,
        doc_short: String,
        doc_long: String,
    }

    /// Leak an owned string into a `'static` string slice.
    fn leak_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// Build a validated [`RegistryView`] from owned synthetic flags.
    fn synth_view(flags: Vec<SynthFlag>) -> RegistryView {
        let leaked: Vec<&'static dyn Flag> = flags
            .into_iter()
            .map(|f| {
                let tf = TestFlag {
                    long: leak_str(f.long),
                    short: f.short,
                    doc_short: leak_str(f.doc_short),
                    doc_long: leak_str(f.doc_long),
                };
                &*Box::leak(Box::new(tf)) as &'static dyn Flag
            })
            .collect();
        let slice: &'static [&'static dyn Flag] =
            Box::leak(leaked.into_boxed_slice());
        RegistryView::new(slice).expect("synthetic registry should validate")
    }

    /// The pool of distinct short-name bytes, used to keep generated short
    /// names unique within a registry.
    fn short_pool() -> Vec<u8> {
        let mut pool = Vec::new();
        pool.extend(b'a'..=b'z');
        pool.extend(b'A'..=b'Z');
        pool.extend(b'0'..=b'9');
        pool
    }

    /// Strategy producing a valid synthetic registry of switch flags with
    /// unique long and short names.
    ///
    /// Each flag's short and long documentation is a distinctive plain-text
    /// token (`shortdocN` / `longdocN`, alphanumeric so wrapping and roff
    /// cleanup never split or alter it) followed by a `\flag{flag0long}`
    /// markup tag. Flag 0's long name is always `flag0long`, so the tag always
    /// resolves; when rendered as plain text it yields `--flag0long` (possibly
    /// prefixed by a short-name reference), which the test asserts appears in
    /// both the short and long help. This exercises both the per-flag doc text
    /// and markup resolution (Requirements 9.3, 9.4).
    fn synth_registry() -> impl Strategy<Value = Vec<SynthFlag>> {
        prop::collection::vec(any::<bool>(), 1..8).prop_map(|wants_shorts| {
            let pool = short_pool();
            wants_shorts
                .into_iter()
                .enumerate()
                .map(|(i, wants_short)| {
                    let long = format!("flag{i}long");
                    let short = if wants_short && i < pool.len() {
                        Some(pool[i])
                    } else {
                        None
                    };
                    // Reference flag 0's long name, which always exists.
                    let doc_short = format!(r"shortdoc{i} \flag{{flag0long}}");
                    let doc_long = format!(r"longdoc{i} \flag{{flag0long}}");
                    SynthFlag { long, short, doc_short, doc_long }
                })
                .collect()
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 24: Help renders every flag's documentation with markup resolved
        //
        // **Validates: Requirements 9.3, 9.4**
        //
        // For any registry, the short help contains each flag's
        // markup-resolved short documentation and the long help contains each
        // flag's markup-resolved long documentation. Each flag's docs carry a
        // distinctive plain token plus a `\flag{flag0long}` cross-reference;
        // the test asserts both the token and the resolved reference
        // (`--flag0long`) appear in the corresponding help output.
        #[test]
        fn prop_help_renders_docs_with_markup_resolved(
            flags in synth_registry(),
        ) {
            let view = synth_view(flags.clone());
            let short = generate_short_with(&view);
            let long = generate_long_with(&view);

            for (i, _flag) in flags.iter().enumerate() {
                // Short help renders the markup-resolved short doc
                // (Requirement 9.3): the per-flag token is present...
                let short_token = format!("shortdoc{i}");
                prop_assert!(
                    short.contains(short_token.as_str()),
                    "short help missing short doc token {short_token:?}",
                );
                // ...and the long help renders the markup-resolved long doc
                // (Requirement 9.4): the per-flag token is present.
                let long_token = format!("longdoc{i}");
                prop_assert!(
                    long.contains(long_token.as_str()),
                    "long help missing long doc token {long_token:?}",
                );
            }

            // Every flag's docs embed `\flag{flag0long}`, which must resolve
            // to flag 0's long reference `--flag0long` in both artifacts.
            prop_assert!(
                short.contains("--flag0long"),
                "short help did not resolve \\flag{{flag0long}} markup",
            );
            prop_assert!(
                long.contains("--flag0long"),
                "long help did not resolve \\flag{{flag0long}} markup",
            );
        }
    }

    // Unit test for Requirement 2.5: a flag whose short documentation is
    // empty still renders in the `-h` short help with its name column present
    // and an empty description (no crash, empty second column).
    #[test]
    fn short_help_renders_empty_doc_short() {
        let view = synth_view(vec![SynthFlag {
            long: "emptyflag".to_string(),
            short: None,
            doc_short: String::new(),
            doc_long: "long documentation".to_string(),
        }]);
        let short = generate_short_with(&view);

        // The flag's name column is present in the short help.
        let line = short
            .lines()
            .find(|l| l.contains("--emptyflag"))
            .expect("short help should contain the flag name");
        // The description (second column) is empty: once the name and the
        // surrounding padding are stripped, nothing of substance remains.
        assert_eq!(
            line.trim(),
            "--emptyflag",
            "expected an empty description for a flag with empty doc_short, \
             got line {line:?}",
        );
    }
}

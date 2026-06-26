/*!
Provides routines for generating ripgrep's man page in `roff` format.
*/

use std::{collections::BTreeMap, fmt::Write};

use crate::flags::{
    Flag, RegistryView,
    doc::{
        markup::{MarkupFlavor, render_markup},
        version,
    },
};

const TEMPLATE: &'static str = include_str!("template.rg.1");

/// Wraps `std::write!` and asserts there is no failure.
///
/// We only write to `String` in this module.
macro_rules! write {
    ($($tt:tt)*) => { std::write!($($tt)*).unwrap(); }
}

/// Wraps `std::writeln!` and asserts there is no failure.
///
/// We only write to `String` in this module.
macro_rules! writeln {
    ($($tt:tt)*) => { std::writeln!($($tt)*).unwrap(); }
}

/// Returns a `roff` formatted string corresponding to ripgrep's entire man
/// page.
pub(crate) fn generate() -> String {
    // The registry is validated once per generation; all flag content and
    // markup is resolved against this single source of truth. Iterating via
    // `by_category` places every flag under exactly one matching category
    // heading and fixes both the category order and the within-category flag
    // order (Requirements 7.2, 7.3, 9.1).
    generate_with(
        &RegistryView::load()
            .expect("ripgrep's flag registry should validate"),
    )
}

/// Returns a `roff` formatted man page generated from the given `registry`.
///
/// This is the registry-accepting seam used by `generate` (which passes the
/// real `FLAGS` registry) and by property tests (which pass synthetic
/// registries). All flag content and markup is resolved against the single
/// `registry` argument.
pub(crate) fn generate_with(registry: &RegistryView) -> String {
    let mut cats = BTreeMap::new();
    for (category, flags) in registry.by_category() {
        let cat = cats.entry(category).or_insert_with(String::new);
        for flag in flags {
            if !cat.is_empty() {
                writeln!(cat, ".sp");
            }
            generate_flag(flag, registry, cat);
        }
    }

    let mut out = TEMPLATE.replace("!!VERSION!!", &version::generate_digits());
    for (cat, value) in cats.iter() {
        let var = format!("!!{name}!!", name = cat.as_str());
        out = out.replace(&var, value);
    }
    out
}

/// Writes `roff` formatted documentation for `flag` to `out`.
fn generate_flag(
    flag: &'static dyn Flag,
    registry: &RegistryView,
    out: &mut String,
) {
    if let Some(byte) = flag.name_short() {
        let name = char::from(byte);
        write!(out, r"\fB\-{name}\fP");
        // A switch never displays a value variable; only non-switch flags
        // that declare one do (Requirements 9.5, 9.6).
        if !flag.is_switch() {
            if let Some(var) = flag.doc_variable() {
                write!(out, r" \fI{var}\fP");
            }
        }
        write!(out, r", ");
    }

    // The flag name is escaped for roff centrally via `escape_roff` so each
    // hyphen renders as a literal hyphen (Requirement 5.6). The generator no
    // longer carries its own ad hoc replacement.
    let name = escape_roff(flag.name_long());
    write!(out, r"\fB\-\-{name}\fP");
    if !flag.is_switch() {
        if let Some(var) = flag.doc_variable() {
            write!(out, r"=\fI{var}\fP");
        }
    }
    write!(out, "\n");

    writeln!(out, ".RS 4");
    let doc = flag.doc_long().trim();
    // Resolve `\flag{..}` and `\flag-negate{..}` markup against the registry,
    // escaping roff hyphens centrally in the renderer. The real registry's
    // documentation is known-valid, so resolution must succeed here.
    let doc = render_markup(doc, registry, MarkupFlavor::Roff)
        .expect("flag documentation markup should resolve");
    writeln!(out, "{doc}");
    if flag.name_negated().is_some() {
        // Flags that can be negated that aren't switches, like
        // --context-separator, are somewhat weird. Because of that, the docs
        // for those flags should discuss the semantics of negation explicitly.
        // But for switches, the behavior is always the same: document that the
        // flag can be disabled, showing the negated name verbatim
        // (Requirement 4.3). The negated name is rendered through the shared
        // markup renderer so its hyphens are escaped centrally for roff.
        if flag.is_switch() {
            let long = flag.name_long();
            let negation = render_markup(
                &format!(r"\flag-negate{{{long}}}"),
                registry,
                MarkupFlavor::Roff,
            )
            .expect("negated flag reference should resolve");
            writeln!(out, ".sp");
            writeln!(out, r"This flag can be disabled with {negation}.");
        }
    }
    writeln!(out, ".RE");
}

/// Escape every hyphen in an emitted flag name for roff output.
///
/// This mirrors the centralized escaping performed by the markup renderer for
/// flag-name cross-references, ensuring each hyphen in a flag name displayed in
/// the man page renders as a literal hyphen (Requirement 5.6). Inputs come
/// straight from the registry and contain only literal hyphens, so no double
/// escaping can occur.
fn escape_roff(name: &str) -> String {
    name.replace('-', r"\-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::{Category, Flag, FlagValue, RegistryView};

    /// A minimal synthetic flag used to build synthetic registries for
    /// exercising the man and help generators in isolation.
    #[derive(Debug)]
    struct TestFlag {
        long: &'static str,
        short: Option<u8>,
        negated: Option<&'static str>,
        switch: bool,
        variable: Option<&'static str>,
        /// The category this flag is assigned to. Most tests only care about a
        /// single category and set this to [`Category::Search`]; the category
        /// placement property generates flags spanning every category.
        category: Category,
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
            "short doc"
        }
        fn doc_long(&self) -> &'static str {
            "long documentation"
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
        negated: Option<String>,
        switch: bool,
    }

    /// Leak an owned string into a `'static` string slice.
    fn leak_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// Build a validated [`RegistryView`] from owned synthetic flags. A
    /// non-switch flag is given a value variable (and a switch is not) so the
    /// registry always validates.
    fn synth_view(flags: Vec<SynthFlag>) -> RegistryView {
        let leaked: Vec<&'static dyn Flag> = flags
            .into_iter()
            .enumerate()
            .map(|(i, f)| {
                let variable = if f.switch {
                    None
                } else {
                    Some(leak_str(format!("VAL{i}")))
                };
                let tf = TestFlag {
                    long: leak_str(f.long),
                    short: f.short,
                    negated: f.negated.map(leak_str),
                    switch: f.switch,
                    variable,
                    category: Category::Search,
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

    /// Strategy producing a valid synthetic registry of switch and value
    /// flags with unique long/short/negated names. The first flag is forced
    /// to be a switch with a negated name, so every generated registry
    /// contains at least one switch flag whose negation must be documented.
    /// Some long names are deliberately hyphen-rich to exercise roff hyphen
    /// escaping of the negated name.
    fn synth_registry() -> impl Strategy<Value = Vec<SynthFlag>> {
        // Per flag: (wants_short, wants_negated, switch, hyphen_rich).
        let raw = (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>());
        prop::collection::vec(raw, 1..8).prop_map(|raws| {
            let pool = short_pool();
            let mut flags: Vec<SynthFlag> = raws
                .into_iter()
                .enumerate()
                .map(
                    |(
                        i,
                        (wants_short, wants_negated, switch, hyphen_rich),
                    )| {
                        let long = if hyphen_rich {
                            format!("flag-{i}-a-b-c")
                        } else {
                            format!("flag{i}long")
                        };
                        let short = if wants_short && i < pool.len() {
                            Some(pool[i])
                        } else {
                            None
                        };
                        let negated = if wants_negated {
                            Some(format!("no-{long}"))
                        } else {
                            None
                        };
                        SynthFlag { long, short, negated, switch }
                    },
                )
                .collect();
            // Guarantee at least one switch flag with a negated name.
            flags[0].switch = true;
            flags[0].negated = Some(format!("no-{}", flags[0].long));
            flags
        })
    }

    /// Map a small index to one of the seven categories. Used by the category
    /// placement strategy to assign each synthetic flag a category.
    fn category_from_index(n: u8) -> Category {
        match n % 7 {
            0 => Category::Input,
            1 => Category::Search,
            2 => Category::Filter,
            3 => Category::Output,
            4 => Category::OutputModes,
            5 => Category::Logging,
            _ => Category::OtherBehaviors,
        }
    }

    /// Build a validated [`RegistryView`] from synthetic switch flags, each
    /// carrying an explicit category. Flags are plain switches (no short name,
    /// no negation, no value variable) so the only thing that varies between
    /// them is their long name and assigned category, which is exactly what
    /// the category placement property exercises.
    fn cat_view(specs: Vec<(String, Category)>) -> RegistryView {
        let leaked: Vec<&'static dyn Flag> = specs
            .into_iter()
            .map(|(long, category)| {
                let tf = TestFlag {
                    long: leak_str(long),
                    short: None,
                    negated: None,
                    switch: true,
                    variable: None,
                    category,
                };
                &*Box::leak(Box::new(tf)) as &'static dyn Flag
            })
            .collect();
        let slice: &'static [&'static dyn Flag] =
            Box::leak(leaked.into_boxed_slice());
        RegistryView::new(slice).expect("synthetic registry should validate")
    }

    /// Strategy producing a synthetic registry whose flags span the full set
    /// of categories. Each flag gets a unique long name (`flag-{i}`) so its
    /// rendered name is unambiguous, and a category chosen from across all
    /// seven categories so that category placement can be exercised broadly.
    fn cat_registry() -> impl Strategy<Value = Vec<(String, Category)>> {
        prop::collection::vec(0u8..7, 1..8).prop_map(|cats| {
            cats.into_iter()
                .enumerate()
                .map(|(i, c)| (format!("flag-{i}"), category_from_index(c)))
                .collect()
        })
    }

    /// Return the byte range `(start, end)` within `artifact` in which the
    /// flags of `target` must appear: bounded below by `target`'s heading and
    /// above by the next heading in the fixed category order. For the last
    /// category, the upper bound is `end_marker` if non-empty, otherwise the
    /// end of the artifact. Every heading is always present in the templates,
    /// so the lookups cannot fail for a well-formed artifact.
    fn section_bounds(
        artifact: &str,
        headings: &[(Category, &str)],
        end_marker: &str,
        target: Category,
    ) -> (usize, usize) {
        let pos = |needle: &str| {
            artifact
                .find(needle)
                .unwrap_or_else(|| panic!("missing heading {needle:?}"))
        };
        let k = headings.iter().position(|(c, _)| *c == target).unwrap();
        let start = pos(headings[k].1);
        let end = if k + 1 < headings.len() {
            pos(headings[k + 1].1)
        } else if !end_marker.is_empty() {
            pos(end_marker)
        } else {
            artifact.len()
        };
        (start, end)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 10: Negation is documented for switches in man and long help
        //
        // **Validates: Requirements 4.3, 4.4**
        //
        // For any registry, every switch flag that has a negated name has
        // man-page documentation and long-help documentation that states the
        // flag can be disabled, showing the negated name verbatim. The man
        // page emits roff with each hyphen of the negated name escaped as
        // `\-`; the long help emits the negated name as plain text.
        #[test]
        fn prop_negation_documented_for_switches(
            flags in synth_registry(),
        ) {
            // The switch flags that have a negated name: these are the flags
            // whose negation must be documented in both artifacts.
            let switch_negated: Vec<String> = flags
                .iter()
                .filter(|f| f.switch && f.negated.is_some())
                .map(|f| f.negated.clone().unwrap())
                .collect();
            // The strategy guarantees at least one such flag exists.
            prop_assert!(!switch_negated.is_empty());

            let view = synth_view(flags);
            let man = generate_with(&view);
            let help_long =
                crate::flags::doc::help::generate_long_with(&view);

            for negated in &switch_negated {
                // Man page (Requirement 4.3): the negated name is shown
                // verbatim in roff, with each hyphen escaped as `\-`.
                let escaped = negated.replace('-', r"\-");
                let man_expected = format!(
                    r"This flag can be disabled with \fB\-\-{escaped}\fP."
                );
                prop_assert!(
                    man.contains(man_expected.as_str()),
                    "man output missing negation documentation \
                     {man_expected:?}",
                );

                // Long help (Requirement 4.4): the negated name is shown
                // verbatim as plain text.
                let help_expected =
                    format!("This flag can be disabled with --{negated}.");
                prop_assert!(
                    help_long.contains(help_expected.as_str()),
                    "long help missing negation documentation \
                     {help_expected:?}",
                );
            }
        }

        // Feature: unified-flag-source, Property 25: Man shows value variables only for value flags
        //
        // **Validates: Requirements 9.5, 9.6**
        //
        // For any registry, the man page displays the value variable name
        // immediately following the flag name for every non-switch flag that
        // has one, and displays no value variable name for any switch flag.
        // `synth_view` assigns each non-switch flag at index `i` the value
        // variable `VAL{i}` and gives switch flags no variable, so we can
        // assert the exact roff emitted for each flag's long name.
        #[test]
        fn prop_value_variables_only_for_value_flags(
            flags in synth_registry(),
        ) {
            // Capture each flag's switch-ness in declaration order before the
            // owned flags are consumed by `synth_view`; the index lines up
            // with the `VAL{i}` value variable assigned by `synth_view`.
            let specs: Vec<(String, bool)> = flags
                .iter()
                .map(|f| (f.long.clone(), f.switch))
                .collect();

            let view = synth_view(flags);
            let man = generate_with(&view);

            for (i, (long, switch)) in specs.iter().enumerate() {
                // The long name as rendered in roff, with each hyphen escaped.
                let escaped = escape_roff(long);
                let name = format!(r"\fB\-\-{escaped}\fP");

                if *switch {
                    // Requirement 9.6: a switch flag never attaches a value
                    // variable to its name. Since `name` ends with the `\fP`
                    // terminator, the only way `{name}=\fI` could appear is if
                    // this flag emitted a value variable after its long name.
                    let attached = format!(r"{name}=\fI");
                    prop_assert!(
                        !man.contains(attached.as_str()),
                        "man output attaches a value variable to switch flag \
                         {long:?}: found {attached:?}",
                    );
                } else {
                    // Requirement 9.5: a non-switch flag displays its value
                    // variable immediately following the flag name. The
                    // variable is `VAL{i}` per `synth_view`.
                    let expected = format!(r"{name}=\fIVAL{i}\fP");
                    prop_assert!(
                        man.contains(expected.as_str()),
                        "man output missing value variable for non-switch \
                         flag {long:?}: expected {expected:?}",
                    );
                }
            }
        }

        // Feature: unified-flag-source, Property 23: Each flag appears under exactly one matching category heading
        //
        // **Validates: Requirements 9.1, 9.2**
        //
        // For any registry, both the man page and the long help place each
        // flag definition under exactly one category heading, and that
        // heading is the one matching the category assigned to the flag.
        // Synthetic flags are generated spanning all seven categories. Each
        // flag's rendered name must appear exactly once in each artifact
        // (exactly one heading), and that single occurrence must fall within
        // the section bounded by the flag's own category heading and the next
        // heading in the fixed category order (the matching heading). The
        // category headings are always present in the templates, so the
        // bounds are well defined regardless of which categories contain
        // flags.
        #[test]
        fn prop_each_flag_under_exactly_one_matching_category(
            specs in cat_registry(),
        ) {
            let view = cat_view(specs.clone());
            let man = generate_with(&view);
            let help_long =
                crate::flags::doc::help::generate_long_with(&view);

            // The category headings in the single fixed declaration order.
            // Each category's flags are emitted between its heading and the
            // following heading; a trailing marker bounds the last category.
            let man_headings = [
                (Category::Input, ".SS INPUT OPTIONS"),
                (Category::Search, ".SS SEARCH OPTIONS"),
                (Category::Filter, ".SS FILTER OPTIONS"),
                (Category::Output, ".SS OUTPUT OPTIONS"),
                (Category::OutputModes, ".SS OUTPUT MODES"),
                (Category::Logging, ".SS LOGGING OPTIONS"),
                (Category::OtherBehaviors, ".SS OTHER BEHAVIORS"),
            ];
            // The man options region is closed by the next `.SH` section.
            let man_end = ".SH EXIT STATUS";
            let help_headings = [
                (Category::Input, "INPUT OPTIONS:"),
                (Category::Search, "SEARCH OPTIONS:"),
                (Category::Filter, "FILTER OPTIONS:"),
                (Category::Output, "OUTPUT OPTIONS:"),
                (Category::OutputModes, "OUTPUT MODES:"),
                (Category::Logging, "LOGGING OPTIONS:"),
                (Category::OtherBehaviors, "OTHER BEHAVIORS:"),
            ];

            for (long, category) in &specs {
                // The fully delimited rendered names. The man name is closed
                // by the roff `\fP`; the long-help switch name is closed by
                // its trailing newline. Both are unique across the registry.
                let man_token = format!(r"\fB\-\-{}\fP", escape_roff(long));
                let help_token = format!("--{long}\n");

                // Exactly one occurrence in each artifact: the flag's entry is
                // placed under exactly one category heading (Req 9.1, 9.2).
                prop_assert_eq!(
                    man.matches(man_token.as_str()).count(),
                    1,
                    "man output should contain flag {:?} exactly once",
                    long,
                );
                prop_assert_eq!(
                    help_long.matches(help_token.as_str()).count(),
                    1,
                    "long help should contain flag {:?} exactly once",
                    long,
                );

                // That single occurrence must fall within the section bounded
                // by the flag's own category heading and the next heading: the
                // entry is grouped under the matching category heading.
                let man_idx = man.find(man_token.as_str()).unwrap();
                let (man_start, man_next) = section_bounds(
                    &man, &man_headings, man_end, *category,
                );
                prop_assert!(
                    man_start < man_idx && man_idx < man_next,
                    "man entry for {long:?} is not under the {category:?} \
                     heading",
                );

                let help_idx = help_long.find(help_token.as_str()).unwrap();
                let (help_start, help_next) = section_bounds(
                    &help_long, &help_headings, "", *category,
                );
                prop_assert!(
                    help_start < help_idx && help_idx < help_next,
                    "long help entry for {long:?} is not under the \
                     {category:?} heading",
                );
            }
        }
    }

    // Unit test for Requirement 5.6: when the man generator emits a flag name
    // into roff output, each hyphen in that name is escaped as `\-` so the
    // rendered man page displays a literal hyphen for each. This exercises the
    // REAL registry rather than a synthetic one, using `--context-separator`
    // as a representative hyphenated flag.
    #[test]
    fn man_escapes_hyphens_in_real_flag_name() {
        let man = generate_with(
            &RegistryView::load()
                .expect("ripgrep's flag registry should validate"),
        );

        // The flag name's heading is emitted with every hyphen escaped as
        // `\-`, wrapped in roff bold (`\fB ... \fP`).
        let escaped = r"\fB\-\-context\-separator\fP";
        assert!(
            man.contains(escaped),
            "man output should contain the roff-escaped flag name {escaped:?}",
        );

        // The unescaped literal flag name must never appear in the roff
        // output: every hyphen is escaped, so `--context-separator` (with raw
        // hyphens) is absent.
        assert!(
            !man.contains("--context-separator"),
            "man output should not contain an unescaped --context-separator",
        );
    }
}

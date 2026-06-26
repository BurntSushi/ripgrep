/*!
Provides completions for ripgrep's CLI for the fish shell.
*/

use crate::flags::{CompletionType, Flag, RegistryView};

const TEMPLATE: &'static str = "complete -c rg !SHORT! -l !LONG! -d '!DOC!'";
const TEMPLATE_NEGATED: &'static str = "complete -c rg -l !NEGATED! -n '__rg_contains_opt !LONG! !SHORT!' -d '!DOC!'\n";
const TEMPLATE_ALIAS: &'static str = "complete -c rg -l !ALIAS! -d '!DOC!'";

/// Generate completions for Fish.
///
/// Reference: <https://fishshell.com/docs/current/completions.html>
pub(crate) fn generate() -> String {
    generate_with(
        &RegistryView::load()
            .expect("ripgrep's flag registry should validate"),
    )
}

/// Generate completions for Fish from the given (already validated) registry
/// view.
///
/// This is the registry-accepting seam behind [`generate`]: `generate` loads
/// ripgrep's canonical registry and delegates here, while tests pass synthetic
/// registries to exercise the generator across many inputs.
pub(crate) fn generate_with(registry: &RegistryView) -> String {
    let mut out = String::new();
    out.push_str(include_str!("prelude.fish"));
    out.push('\n');
    for flag in registry.iter() {
        let short = match flag.name_short() {
            None => "".to_string(),
            Some(byte) => format!("-s {}", char::from(byte)),
        };
        let long = flag.name_long();
        let doc = flag.doc_short().replace("'", "\\'");
        let mut completion = TEMPLATE
            .replace("!SHORT!", &short)
            .replace("!LONG!", &long)
            .replace("!DOC!", &doc);
        push_value_completion(&mut completion, flag);
        completion.push('\n');
        out.push_str(&completion);

        // A negated name is offered as a separately completable flag
        // (Requirements 4.1).
        if let Some(negated) = flag.name_negated() {
            let short = match flag.name_short() {
                None => "".to_string(),
                Some(byte) => char::from(byte).to_string(),
            };
            out.push_str(
                &TEMPLATE_NEGATED
                    .replace("!NEGATED!", &negated)
                    .replace("!SHORT!", &short)
                    .replace("!LONG!", &long)
                    .replace("!DOC!", &doc),
            );
        }

        // Every alias is offered as a separately completable flag, with the
        // same value-completion behavior as the canonical long name
        // (Requirements 4.2).
        for alias in flag.aliases() {
            let mut completion = TEMPLATE_ALIAS
                .replace("!ALIAS!", alias)
                .replace("!DOC!", &doc);
            push_value_completion(&mut completion, flag);
            completion.push('\n');
            out.push_str(&completion);
        }
    }
    out
}

/// Appends the Fish value-completion suffix for `flag` to `completion`,
/// following the Completion_Type mapping (Requirement 3).
///
/// A switch requests no value: no `-r` (require argument) suffix is emitted, so
/// Fish completes the flag without expecting a value (Requirement 3.6).
/// Declared choices are offered exactly and in declared order (Requirement
/// 3.3).
fn push_value_completion(completion: &mut String, flag: &dyn Flag) {
    match flag.completion_type() {
        CompletionType::Filename => {
            completion.push_str(" -r -F");
        }
        CompletionType::Executable => {
            completion.push_str(" -r -f -a '(__fish_complete_command)'");
        }
        CompletionType::Filetype => {
            completion.push_str(
                " -r -f -a '(rg --type-list | string replace : \\t)'",
            );
        }
        CompletionType::Encoding => {
            completion.push_str(" -r -f -a '");
            completion.push_str(super::ENCODINGS);
            completion.push_str("'");
        }
        CompletionType::Other if !flag.doc_choices().is_empty() => {
            completion.push_str(" -r -f -a '");
            completion.push_str(&flag.doc_choices().join(" "));
            completion.push_str("'");
        }
        CompletionType::Other if !flag.is_switch() => {
            completion.push_str(" -r -f");
        }
        CompletionType::Other => (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::registry_tests::{SyntheticFlag, build_registry};
    use crate::flags::{Category, RegistryView};
    use proptest::prelude::*;

    // Feature: unified-flag-source, Property 7: Fish requests no value for
    // switches
    //
    // For any registry, every switch Flag_Definition has a Fish completion
    // entry that requests no value. In Fish, a value-requesting completion is
    // marked with the `-r` (require argument) token, so a switch's entry must
    // contain no `-r` token (Requirement 3.6).

    /// The shape of a single synthetic flag: either a switch (no value) or a
    /// value flag of some completion type. The property asserts on switches;
    /// the value kinds are filler that produce a realistic mixed registry.
    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Switch,
        ValueFilename,
        ValueChoices,
        ValueEncoding,
    }

    fn any_kind() -> impl Strategy<Value = Kind> {
        prop_oneof![
            Just(Kind::Switch),
            Just(Kind::ValueFilename),
            Just(Kind::ValueChoices),
            Just(Kind::ValueEncoding),
        ]
    }

    /// The name-independent attributes of a single synthetic flag. Names are
    /// assigned during normalization so they are unique within the registry.
    #[derive(Clone, Debug)]
    struct RawSpec {
        kind: Kind,
        wants_short: bool,
        wants_negated: bool,
        num_aliases: usize,
        hyphen_rich: bool,
    }

    fn raw_spec() -> impl Strategy<Value = RawSpec> {
        (any_kind(), any::<bool>(), any::<bool>(), 0usize..3, any::<bool>())
            .prop_map(
                |(
                    kind,
                    wants_short,
                    wants_negated,
                    num_aliases,
                    hyphen_rich,
                )| {
                    RawSpec {
                        kind,
                        wants_short,
                        wants_negated,
                        num_aliases,
                        hyphen_rich,
                    }
                },
            )
    }

    /// Distinct short-name bytes, assigned by index to keep short names unique.
    fn short_pool() -> Vec<u8> {
        let mut pool = Vec::new();
        pool.extend(b'a'..=b'z');
        pool.extend(b'A'..=b'Z');
        pool.extend(b'0'..=b'9');
        pool
    }

    /// Assign unique, well-formed names derived from each flag's index, turning
    /// raw specs into a valid synthetic registry. Switches carry no value
    /// variable; value flags are non-switches with a value variable.
    fn normalize(raws: Vec<RawSpec>) -> Vec<SyntheticFlag> {
        let pool = short_pool();
        raws.into_iter()
            .enumerate()
            .map(|(i, raw)| {
                // Some long names are deliberately hyphen-rich to exercise the
                // handling of hyphenated names.
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
                let (switch, completion, variable, choices) = match raw.kind {
                    Kind::Switch => {
                        (true, CompletionType::Other, None, Vec::new())
                    }
                    Kind::ValueFilename => (
                        false,
                        CompletionType::Filename,
                        Some(format!("VAL{i}")),
                        Vec::new(),
                    ),
                    Kind::ValueChoices => (
                        false,
                        CompletionType::Other,
                        Some(format!("VAL{i}")),
                        vec![format!("c{i}a"), format!("c{i}b")],
                    ),
                    Kind::ValueEncoding => (
                        false,
                        CompletionType::Encoding,
                        Some(format!("VAL{i}")),
                        Vec::new(),
                    ),
                };
                let aliases = (0..raw.num_aliases)
                    .map(|j| format!("{long}-alias{j}"))
                    .collect();
                SyntheticFlag {
                    long,
                    short,
                    negated,
                    switch,
                    variable,
                    category: Category::Search,
                    short_doc: "does a thing".to_string(),
                    long_doc: format!("long documentation for flag {i}"),
                    aliases,
                    completion,
                    choices,
                }
            })
            .collect()
    }

    /// Strategy producing a valid, mixed synthetic registry.
    fn registry_strategy() -> impl Strategy<Value = Vec<SyntheticFlag>> {
        prop::collection::vec(raw_spec(), 1..8).prop_map(normalize)
    }

    /// Returns the slice of `haystack` from the first occurrence of `start`
    /// up to and including the next occurrence of `end` (or to the end of the
    /// string if `end` is absent). Used to isolate a single flag's completion
    /// entry from a generated artifact.
    fn block<'a>(
        haystack: &'a str,
        start: &str,
        end: &str,
    ) -> Option<&'a str> {
        let i = haystack.find(start)?;
        let rest = &haystack[i..];
        let j = rest.find(end).map(|j| j + end.len()).unwrap_or(rest.len());
        Some(&rest[..j])
    }

    /// Isolate the Fish completion entry for the long flag `long`. The entry
    /// begins at the ` -l <long> ` token (the trailing space disambiguates it
    /// from aliases such as `<long>-aliasN` and from the negated name) and runs
    /// up to the start of the next `complete` entry.
    fn fish_block<'a>(out: &'a str, long: &str) -> Option<&'a str> {
        block(out, &format!(" -l {long} "), "\ncomplete ")
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn switches_request_no_value(flags in registry_strategy()) {
            // Remember which flags are switches before the registry is
            // consumed by `build_registry`.
            let switches: Vec<String> = flags
                .iter()
                .filter(|f| f.switch)
                .map(|f| f.long.clone())
                .collect();

            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            let fish = generate_with(&view);

            for long in switches {
                let entry = fish_block(&fish, &long)
                    .expect("fish must contain a completion for the switch");
                prop_assert!(
                    !entry.contains(" -r"),
                    "fish entry for switch --{long} must request no value \
                     but contains a `-r` token: {entry}"
                );
            }
        }
    }

    // Feature: unified-flag-source, Property 22: Flags and categories are
    // emitted in fixed order
    //
    // For any registry and for each generator, the categories are emitted in
    // the fixed declaration order, and within each category the flags are
    // emitted in the exact order their Flag_Definitions appear in the registry
    // (Requirements 7.2, 7.3).
    //
    // `RegistryView::by_category()` is the shared ordering authority: it yields
    // categories in the fixed `Category::ALL` order and, within each, flags in
    // registry order. The flat generators (bash, fish, powershell) emit flags
    // in plain registry order while the category-sectioned generators (man,
    // help-long, zsh) emit them grouped by category. To check all six against a
    // single expected order, we build registries whose declaration order is
    // *already* grouped by category in the fixed order (the flags are stably
    // sorted by their category's position in `Category::ALL`). Then registry
    // order equals `by_category()` flattened, so every generator must emit flag
    // long names in that one order. For each artifact we extract the order in
    // which each flag's long name first appears and assert it equals the
    // expected order; for the sectioned artifacts we additionally assert the
    // category sections themselves are in the fixed order.

    /// The kind of a synthetic flag used by the ordering property. A realistic
    /// mix of switches and value flags is generated; the property only inspects
    /// the order of flag long names, so the specific kinds are filler.
    #[derive(Clone, Copy, Debug)]
    enum OrderKind {
        Switch,
        ValueFilename,
        ValueChoices,
    }

    fn any_order_kind() -> impl Strategy<Value = OrderKind> {
        prop_oneof![
            Just(OrderKind::Switch),
            Just(OrderKind::ValueFilename),
            Just(OrderKind::ValueChoices),
        ]
    }

    /// The name-independent attributes of a single synthetic flag, plus the
    /// index of its category in the fixed `Category::ALL` order. Names are
    /// assigned during normalization so they are unique within the registry.
    #[derive(Clone, Debug)]
    struct OrderSpec {
        category: usize,
        kind: OrderKind,
        wants_short: bool,
        wants_negated: bool,
        num_aliases: usize,
    }

    fn order_spec() -> impl Strategy<Value = OrderSpec> {
        (
            0..Category::ALL.len(),
            any_order_kind(),
            any::<bool>(),
            any::<bool>(),
            0usize..3,
        )
            .prop_map(
                |(category, kind, wants_short, wants_negated, num_aliases)| {
                    OrderSpec {
                        category,
                        kind,
                        wants_short,
                        wants_negated,
                        num_aliases,
                    }
                },
            )
    }

    /// Strategy producing a valid synthetic registry whose declaration order is
    /// already grouped by category in the fixed `Category::ALL` order. The
    /// generated flags are stably sorted by their category index, so within a
    /// category they keep their generated (registry) order. Unique, well-formed
    /// names are then assigned from each flag's final index.
    ///
    /// Long names are kept hyphen-free (`flag{i}long`) so that each flag's long
    /// name is rendered as a single verbatim token in every artifact, including
    /// the man page (which escapes hyphens in flag names as `\-`). This lets a
    /// single bare-name search locate every generator's emission uniformly.
    fn ordered_registry() -> impl Strategy<Value = Vec<SyntheticFlag>> {
        prop::collection::vec(order_spec(), 1..8).prop_map(|mut specs| {
            specs.sort_by_key(|s| s.category);
            let pool = short_pool();
            specs
                .into_iter()
                .enumerate()
                .map(|(i, spec)| {
                    let long = format!("flag{i}long");
                    let short = if spec.wants_short && i < pool.len() {
                        Some(pool[i])
                    } else {
                        None
                    };
                    let negated = if spec.wants_negated {
                        Some(format!("no-{long}"))
                    } else {
                        None
                    };
                    let (switch, completion, variable, choices) =
                        match spec.kind {
                            OrderKind::Switch => {
                                (true, CompletionType::Other, None, Vec::new())
                            }
                            OrderKind::ValueFilename => (
                                false,
                                CompletionType::Filename,
                                Some(format!("VAL{i}")),
                                Vec::new(),
                            ),
                            OrderKind::ValueChoices => (
                                false,
                                CompletionType::Other,
                                Some(format!("VAL{i}")),
                                vec![format!("c{i}a"), format!("c{i}b")],
                            ),
                        };
                    let aliases = (0..spec.num_aliases)
                        .map(|j| format!("{long}-alias{j}"))
                        .collect();
                    SyntheticFlag {
                        long,
                        short,
                        negated,
                        switch,
                        variable,
                        category: Category::ALL[spec.category],
                        short_doc: "does a thing".to_string(),
                        long_doc: format!("long documentation for flag {i}"),
                        aliases,
                        completion,
                        choices,
                    }
                })
                .collect()
        })
    }

    /// Returns the region of `artifact` that carries the canonical flag-name
    /// list, within which flag long names appear in emission order.
    ///
    /// Most generators emit each flag's long name first in their primary flag
    /// list, so the whole artifact can be searched directly. PowerShell is the
    /// exception: it emits the per-value-flag completion branches (value flags
    /// only) *before* the flag-name list, which would make a value flag's name
    /// appear before a switch flag's name regardless of registry order. The
    /// flag-name list itself begins at the `'rg' {` marker and lists every flag
    /// in registry order, so the order check is scoped to that region.
    fn flag_list_region<'a>(name: &str, artifact: &'a str) -> &'a str {
        if name == "powershell" {
            let marker = "'rg' {";
            let i = artifact
                .find(marker)
                .expect("powershell artifact must contain the flag list");
            &artifact[i..]
        } else {
            artifact
        }
    }

    /// Returns the long names in `names` ordered by the byte offset at which
    /// each first appears in `artifact`. A name that is absent is sorted last
    /// (at `usize::MAX`) so a missing flag makes the result differ from the
    /// expected order and the property fails loudly.
    fn appearance_order<'a>(
        artifact: &str,
        names: &[&'a str],
    ) -> Vec<&'a str> {
        let mut found: Vec<(usize, &'a str)> = names
            .iter()
            .map(|&n| (artifact.find(n).unwrap_or(usize::MAX), n))
            .collect();
        found.sort_by_key(|&(pos, _)| pos);
        found.into_iter().map(|(_, n)| n).collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn flags_and_categories_emitted_in_fixed_order(
            flags in ordered_registry(),
        ) {
            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            // The expected order: categories in the fixed `Category::ALL`
            // order, and within each the flags in registry order. Because the
            // registry's declaration order is already grouped by category, this
            // equals plain registry order too, so it is the single order every
            // generator must produce.
            let expected: Vec<&'static str> = view
                .by_category()
                .flat_map(|(_, fs)| fs)
                .map(|f| f.name_long())
                .collect();

            // Each generator, produced through its registry-accepting seam.
            let bash = crate::flags::complete::bash::generate_with(&view);
            let zsh = crate::flags::complete::zsh::generate_with(&view);
            let fish = generate_with(&view);
            let powershell =
                crate::flags::complete::powershell::generate_with(&view);
            let man = crate::flags::doc::man::generate_with(&view);
            let help_long =
                crate::flags::doc::help::generate_long_with(&view);

            let artifacts: [(&str, &str); 6] = [
                ("bash", &bash),
                ("zsh", &zsh),
                ("fish", &fish),
                ("powershell", &powershell),
                ("man", &man),
                ("help-long", &help_long),
            ];

            // FLAG order (Requirement 7.2): for every artifact, the order in
            // which the flag long names first appear must equal the expected
            // order.
            for (name, artifact) in artifacts {
                for &long in &expected {
                    prop_assert!(
                        artifact.contains(long),
                        "{name} artifact is missing flag long name {long:?}",
                    );
                }
                let region = flag_list_region(name, artifact);
                let order = appearance_order(region, &expected);
                prop_assert_eq!(
                    &order,
                    &expected,
                    "{} emits flag long names out of fixed order",
                    name,
                );
            }

            // CATEGORY order (Requirement 7.3): for the category-sectioned
            // artifacts, the first flag of an earlier category must appear
            // before the first flag of a later category, so the category
            // sections themselves are in the fixed declaration order.
            let category_firsts: Vec<&'static str> = view
                .by_category()
                .map(|(_, fs)| fs[0].name_long())
                .collect();
            for (name, artifact) in
                [("man", &man), ("help-long", &help_long), ("zsh", &zsh)]
            {
                let positions: Vec<usize> = category_firsts
                    .iter()
                    .map(|&n| {
                        artifact.find(n).expect(
                            "category's first flag must appear in artifact",
                        )
                    })
                    .collect();
                for w in positions.windows(2) {
                    prop_assert!(
                        w[0] < w[1],
                        "{name} emits category sections out of fixed order",
                    );
                }
            }
        }
    }
}

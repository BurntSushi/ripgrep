/*!
Centralized rendering of ripgrep's custom documentation markup.

ripgrep's flag documentation embeds two custom markup tags:

* `\flag{name}` — a cross-reference to another flag, replaced with that flag's
  long name (and short name when present).
* `\flag-negate{name}` — a cross-reference to the negated form of another
  flag, replaced with that flag's negated name.

Historically these tags were resolved by `render_custom_markup` in the parent
module, which `panic!`'d on any unresolved or malformed tag and carried its own
copy of the roff hyphen-escaping logic in each generator. This module replaces
that path with a single fallible renderer ([`render_markup`]) that:

* resolves both tags against the validated [`RegistryView`] (the single source
  of truth);
* returns a [`MarkupError`] — rather than panicking — when a tag references an
  unknown flag, negates a flag that has no negation, or is otherwise
  malformed, so the caller produces no artifact (Requirement 5.3, 5.4, 5.5);
* centralizes roff hyphen escaping in exactly one place so every hyphen in an
  emitted flag name becomes `\-` exactly once (Requirement 5.6). This is the
  single location the historical man-page hyphen bug is fixed.

Only tags in the `\flag` family are recognized. Any other backslash sequence
(for example the roff escapes `\fB`, `\fI`, `\fP`, `\-`, `\(bu`, or even a
literal `\fB{...}` group used by the `--colors` documentation) is passed
through untouched.
*/

use std::fmt::Write;

use crate::flags::{Flag, RegistryView};

/// The output format that a documentation string is being rendered into.
///
/// The man page renders flag names in roff (with hyphens escaped), while the
/// `-h`/`--help` output renders them as plain text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MarkupFlavor {
    /// roff output, used by the man page. Each hyphen in an emitted flag name
    /// is escaped as `\-` (Requirement 5.6).
    Roff,
    /// Plain text output, used by `-h` and `--help`.
    Plain,
}

/// An error produced while resolving documentation markup.
///
/// When any of these errors is returned, the caller must propagate it and
/// produce no artifact (Requirement 5.3, 5.4, 5.5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MarkupError {
    /// A `\flag{name}` or `\flag-negate{name}` tag referenced a name that is
    /// absent from the registry (Requirement 5.3).
    UnknownFlag {
        /// The offending tag, reproduced verbatim (e.g. `\flag{nope}`).
        tag: String,
        /// The unresolved name (e.g. `nope`).
        name: String,
    },
    /// A `\flag-negate{name}` tag referenced a flag that has no negated name
    /// (Requirement 5.4).
    NoNegation {
        /// The offending tag, reproduced verbatim.
        tag: String,
        /// The name of the flag that has no negation.
        name: String,
    },
    /// A markup tag was unrecognized or malformed, such as an unclosed brace
    /// or an unknown tag name (Requirement 5.5).
    Malformed {
        /// The offending tag fragment, reproduced as closely as possible.
        tag: String,
    },
}

impl std::fmt::Display for MarkupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MarkupError::UnknownFlag { ref tag, ref name } => write!(
                f,
                "documentation markup error: tag '{tag}' references unknown \
                 flag name '{name}'"
            ),
            MarkupError::NoNegation { ref tag, ref name } => write!(
                f,
                "documentation markup error: tag '{tag}' negates flag \
                 '{name}', but '{name}' has no negated name"
            ),
            MarkupError::Malformed { ref tag } => write!(
                f,
                "documentation markup error: unrecognized or malformed tag \
                 '{tag}'"
            ),
        }
    }
}

impl std::error::Error for MarkupError {}

/// The opener for a flag cross-reference tag.
const FLAG_OPEN: &str = r"\flag{";
/// The opener for a negated-flag cross-reference tag.
const NEGATE_OPEN: &str = r"\flag-negate{";
/// The common prefix of every recognized markup tag.
///
/// Scanning for this literal prefix is safe: no roff escape used in ripgrep's
/// documentation begins with `\flag`, so we never misinterpret roff (such as
/// `\fB`, `\fI` or even `\fB{...}`) as markup.
const TAG_PREFIX: &str = r"\flag";

/// Resolve every supported markup tag in `doc` against `registry`, targeting
/// the given output `flavor`.
///
/// On success, returns the fully rendered string with all `\flag{...}` and
/// `\flag-negate{...}` tags replaced. Any text that is not a recognized tag —
/// including arbitrary roff — is preserved exactly. On failure, returns a
/// [`MarkupError`] and the caller must emit no artifact.
pub(crate) fn render_markup(
    doc: &str,
    registry: &RegistryView,
    flavor: MarkupFlavor,
) -> Result<String, MarkupError> {
    let mut out = String::with_capacity(doc.len());
    let mut rest = doc;
    while let Some(offset) = rest.find(TAG_PREFIX) {
        out.push_str(&rest[..offset]);
        let tail = &rest[offset..];

        // Check the negation opener first: `\flag-negate{` also begins with
        // `\flag`, but it is not `\flag{` (the character after `\flag` is `-`,
        // not `{`), so the order matters.
        if let Some(body) = tail.strip_prefix(NEGATE_OPEN) {
            let Some(end) = body.find('}') else {
                return Err(MarkupError::Malformed { tag: tag_snippet(tail) });
            };
            let name = &body[..end];
            render_negate(name, registry, flavor, &mut out)?;
            rest = &body[end + 1..];
        } else if let Some(body) = tail.strip_prefix(FLAG_OPEN) {
            let Some(end) = body.find('}') else {
                return Err(MarkupError::Malformed { tag: tag_snippet(tail) });
            };
            let name = &body[..end];
            render_flag(name, registry, flavor, &mut out)?;
            rest = &body[end + 1..];
        } else {
            // We found `\flag` but it is not the opener of a recognized tag
            // (e.g. `\flag-foo{...}` or `\flagstuff`). Treat it as malformed.
            return Err(MarkupError::Malformed { tag: tag_snippet(tail) });
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Render a `\flag{name}` cross-reference into `out`.
fn render_flag(
    name: &str,
    registry: &RegistryView,
    flavor: MarkupFlavor,
    out: &mut String,
) -> Result<(), MarkupError> {
    let Some(flag) = registry.lookup_long(name) else {
        return Err(MarkupError::UnknownFlag {
            tag: format!(r"\flag{{{name}}}"),
            name: name.to_string(),
        });
    };
    write_flag_ref(flag, flavor, out);
    Ok(())
}

/// Render a `\flag-negate{name}` cross-reference into `out`.
fn render_negate(
    name: &str,
    registry: &RegistryView,
    flavor: MarkupFlavor,
    out: &mut String,
) -> Result<(), MarkupError> {
    let Some(flag) = registry.lookup_long(name) else {
        return Err(MarkupError::UnknownFlag {
            tag: format!(r"\flag-negate{{{name}}}"),
            name: name.to_string(),
        });
    };
    let Some(negated) = flag.name_negated() else {
        return Err(MarkupError::NoNegation {
            tag: format!(r"\flag-negate{{{name}}}"),
            name: name.to_string(),
        });
    };
    match flavor {
        MarkupFlavor::Roff => {
            out.push_str(r"\fB");
            write!(out, r"\-\-{}", escape_roff(negated)).unwrap();
            out.push_str(r"\fP");
        }
        MarkupFlavor::Plain => {
            write!(out, r"--{negated}").unwrap();
        }
    }
    Ok(())
}

/// Write a reference to `flag`'s primary (long, and short when present) names
/// into `out` using the given `flavor`.
fn write_flag_ref(
    flag: &'static dyn Flag,
    flavor: MarkupFlavor,
    out: &mut String,
) {
    match flavor {
        MarkupFlavor::Roff => {
            out.push_str(r"\fB");
            if let Some(short) = flag.name_short() {
                write!(out, r"\-{}/", char::from(short)).unwrap();
            }
            write!(out, r"\-\-{}", escape_roff(flag.name_long())).unwrap();
            out.push_str(r"\fP");
        }
        MarkupFlavor::Plain => {
            if let Some(short) = flag.name_short() {
                write!(out, r"-{}/", char::from(short)).unwrap();
            }
            write!(out, r"--{}", flag.name_long()).unwrap();
        }
    }
}

/// Escape every hyphen in an emitted flag name for roff output.
///
/// This is the single, centralized place ripgrep escapes flag-name hyphens for
/// the man page, ensuring each hyphen becomes `\-` exactly once
/// (Requirement 5.6). Inputs come straight from the registry and contain
/// literal hyphens, so no double escaping can occur.
fn escape_roff(name: &str) -> String {
    name.replace('-', r"\-")
}

/// Produce a short, readable snippet of a malformed tag for error reporting.
///
/// When the tag has a closing brace, the snippet includes everything up to and
/// including it; otherwise it extends to the next whitespace. The result is
/// capped to keep diagnostics readable and always lands on a character
/// boundary.
fn tag_snippet(tail: &str) -> String {
    let raw = match tail.find('}') {
        Some(end) => &tail[..=end],
        None => {
            let cut = tail.find(char::is_whitespace).unwrap_or(tail.len());
            &tail[..cut]
        }
    };
    raw.chars().take(60).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::{Category, Flag, FlagValue, RegistryView};

    /// A minimal synthetic flag for exercising the renderer in isolation.
    #[derive(Debug)]
    struct TestFlag {
        long: &'static str,
        short: Option<u8>,
        negated: Option<&'static str>,
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
        fn name_negated(&self) -> Option<&'static str> {
            self.negated
        }
        fn doc_category(&self) -> Category {
            Category::Search
        }
        fn doc_short(&self) -> &'static str {
            "short doc"
        }
        fn doc_long(&self) -> &'static str {
            "long doc"
        }
        fn update(
            &self,
            _: FlagValue,
            _: &mut crate::flags::lowargs::LowArgs,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// Leak the given flags into a `'static` registry slice and build a view.
    fn view(flags: Vec<TestFlag>) -> RegistryView {
        let leaked: Vec<&'static dyn Flag> = flags
            .into_iter()
            .map(|f| &*Box::leak(Box::new(f)) as &'static dyn Flag)
            .collect();
        let slice: &'static [&'static dyn Flag] =
            Box::leak(leaked.into_boxed_slice());
        RegistryView::new(slice).expect("synthetic registry should validate")
    }

    fn render(
        doc: &str,
        v: &RegistryView,
        flavor: MarkupFlavor,
    ) -> Result<String, MarkupError> {
        render_markup(doc, v, flavor)
    }

    #[test]
    fn roff_resolves_flag_with_short_name() {
        let v = view(vec![TestFlag {
            long: "context",
            short: Some(b'C'),
            negated: None,
        }]);
        let out = render(r"see \flag{context} here", &v, MarkupFlavor::Roff)
            .unwrap();
        assert_eq!(out, r"see \fB\-C/\-\-context\fP here");
    }

    #[test]
    fn roff_resolves_flag_without_short_name() {
        let v = view(vec![TestFlag {
            long: "passthru",
            short: None,
            negated: None,
        }]);
        let out = render(r"\flag{passthru}", &v, MarkupFlavor::Roff).unwrap();
        assert_eq!(out, r"\fB\-\-passthru\fP");
    }

    #[test]
    fn roff_escapes_hyphens_in_long_name() {
        let v = view(vec![TestFlag {
            long: "context-separator",
            short: None,
            negated: None,
        }]);
        let out = render(r"\flag{context-separator}", &v, MarkupFlavor::Roff)
            .unwrap();
        assert_eq!(out, r"\fB\-\-context\-separator\fP");
    }

    #[test]
    fn roff_resolves_and_escapes_negation() {
        let v = view(vec![TestFlag {
            long: "context-separator",
            short: None,
            negated: Some("no-context-separator"),
        }]);
        let out =
            render(r"\flag-negate{context-separator}", &v, MarkupFlavor::Roff)
                .unwrap();
        // Every hyphen in the negated name is escaped exactly once.
        assert_eq!(out, r"\fB\-\-no\-context\-separator\fP");
    }

    #[test]
    fn plain_resolves_flag_with_short_name() {
        let v = view(vec![TestFlag {
            long: "context",
            short: Some(b'C'),
            negated: None,
        }]);
        let out = render(r"\flag{context}", &v, MarkupFlavor::Plain).unwrap();
        assert_eq!(out, r"-C/--context");
    }

    #[test]
    fn plain_resolves_negation_without_escaping() {
        let v = view(vec![TestFlag {
            long: "encoding",
            short: None,
            negated: Some("no-encoding"),
        }]);
        let out = render(
            r"reverts via \flag-negate{encoding}.",
            &v,
            MarkupFlavor::Plain,
        )
        .unwrap();
        assert_eq!(out, "reverts via --no-encoding.");
    }

    #[test]
    fn resolves_multiple_tags_in_one_string() {
        let v = view(vec![
            TestFlag { long: "count", short: Some(b'c'), negated: None },
            TestFlag { long: "only-matching", short: None, negated: None },
        ]);
        let out = render(
            r"\flag{count} and \flag{only-matching}",
            &v,
            MarkupFlavor::Plain,
        )
        .unwrap();
        assert_eq!(out, "-c/--count and --only-matching");
    }

    #[test]
    fn resolves_via_alias() {
        // lookup_long resolves aliases too; the rendered reference uses the
        // canonical long name.
        #[derive(Debug)]
        struct Aliased;
        impl Flag for Aliased {
            fn is_switch(&self) -> bool {
                true
            }
            fn name_long(&self) -> &'static str {
                "real-name"
            }
            fn aliases(&self) -> &'static [&'static str] {
                &["legacy-name"]
            }
            fn doc_category(&self) -> Category {
                Category::Search
            }
            fn doc_short(&self) -> &'static str {
                "short doc"
            }
            fn doc_long(&self) -> &'static str {
                "long doc"
            }
            fn update(
                &self,
                _: FlagValue,
                _: &mut crate::flags::lowargs::LowArgs,
            ) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let slice: &'static [&'static dyn Flag] =
            Box::leak(vec![&Aliased as &'static dyn Flag].into_boxed_slice());
        let v = RegistryView::new(slice).unwrap();
        let out =
            render(r"\flag{legacy-name}", &v, MarkupFlavor::Plain).unwrap();
        assert_eq!(out, "--real-name");
    }

    #[test]
    fn leaves_roff_and_plain_text_untouched() {
        let v = view(vec![TestFlag {
            long: "colors",
            short: None,
            negated: None,
        }]);
        // The `--colors` doc legitimately contains `\fB{...}` roff groups,
        // which must not be mistaken for markup tags.
        let input = r"format is \fB{\fP\fItype\fP\fB}\fP and \fBnone\fP";
        let out = render(input, &v, MarkupFlavor::Roff).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn unknown_flag_errors() {
        let v =
            view(vec![TestFlag { long: "known", short: None, negated: None }]);
        let err =
            render(r"\flag{unknown}", &v, MarkupFlavor::Roff).unwrap_err();
        assert_eq!(
            err,
            MarkupError::UnknownFlag {
                tag: r"\flag{unknown}".to_string(),
                name: "unknown".to_string(),
            }
        );
    }

    #[test]
    fn unknown_negate_flag_errors() {
        let v =
            view(vec![TestFlag { long: "known", short: None, negated: None }]);
        let err = render(r"\flag-negate{unknown}", &v, MarkupFlavor::Roff)
            .unwrap_err();
        assert_eq!(
            err,
            MarkupError::UnknownFlag {
                tag: r"\flag-negate{unknown}".to_string(),
                name: "unknown".to_string(),
            }
        );
    }

    #[test]
    fn negate_without_negation_errors() {
        let v = view(vec![TestFlag {
            long: "switchy",
            short: None,
            negated: None,
        }]);
        let err = render(r"\flag-negate{switchy}", &v, MarkupFlavor::Roff)
            .unwrap_err();
        assert_eq!(
            err,
            MarkupError::NoNegation {
                tag: r"\flag-negate{switchy}".to_string(),
                name: "switchy".to_string(),
            }
        );
    }

    #[test]
    fn malformed_unclosed_brace_errors() {
        let v =
            view(vec![TestFlag { long: "known", short: None, negated: None }]);
        let err =
            render(r"oops \flag{known", &v, MarkupFlavor::Roff).unwrap_err();
        assert!(matches!(err, MarkupError::Malformed { .. }));
    }

    #[test]
    fn malformed_unknown_tag_errors() {
        let v =
            view(vec![TestFlag { long: "known", short: None, negated: None }]);
        let err =
            render(r"\flagstuff{known}", &v, MarkupFlavor::Roff).unwrap_err();
        assert!(matches!(err, MarkupError::Malformed { .. }));
    }

    #[test]
    fn no_markup_is_identity() {
        let v =
            view(vec![TestFlag { long: "known", short: None, negated: None }]);
        let input = "plain documentation with no tags at all.";
        assert_eq!(render(input, &v, MarkupFlavor::Plain).unwrap(), input);
    }

    #[test]
    fn real_registry_resolves_known_flag() {
        let v = RegistryView::load().expect("real registry validates");
        let out = render(r"\flag{context}", &v, MarkupFlavor::Plain).unwrap();
        assert!(out.contains("--context"), "got: {out}");
    }

    // ---------------------------------------------------------------------
    // Property-based tests.
    //
    // These keep `markup.rs` self-contained by carrying their own small
    // synthetic-registry strategy built on the local `TestFlag`, rather than
    // reaching into mod.rs's (private, test-only) synthetic-registry module.
    // ---------------------------------------------------------------------

    use proptest::prelude::*;

    /// An owned, generated flag definition. Mirrors [`TestFlag`] but owns its
    /// strings so it can be produced by `proptest`; convert via
    /// [`synth_view`], which leaks the strings into the `'static` slice the
    /// [`RegistryView`] requires.
    #[derive(Clone, Debug)]
    struct SynthFlag {
        long: String,
        short: Option<u8>,
        negated: Option<String>,
    }

    /// Leak an owned string into a `'static` string slice.
    fn leak_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// Build a validated [`RegistryView`] from owned synthetic flags by
    /// converting them into `'static` [`TestFlag`]s.
    fn synth_view(flags: Vec<SynthFlag>) -> RegistryView {
        let test_flags = flags
            .into_iter()
            .map(|f| TestFlag {
                long: leak_str(f.long),
                short: f.short,
                negated: f.negated.map(leak_str),
            })
            .collect();
        view(test_flags)
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

    /// Strategy producing a valid synthetic registry: a non-empty vector of
    /// synthetic flags with unique long, short, and negated names (so loading
    /// through [`RegistryView::new`] always succeeds). Long names are made
    /// unique by embedding each flag's index; some are deliberately
    /// hyphen-rich to exercise roff hyphen escaping.
    fn synth_registry() -> impl Strategy<Value = Vec<SynthFlag>> {
        // Per flag: (wants_short, wants_negated, hyphen_rich).
        let raw = (any::<bool>(), any::<bool>(), any::<bool>());
        prop::collection::vec(raw, 1..8).prop_map(|raws| {
            let pool = short_pool();
            raws.into_iter()
                .enumerate()
                .map(|(i, (wants_short, wants_negated, hyphen_rich))| {
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
                    SynthFlag { long, short, negated }
                })
                .collect()
        })
    }

    /// Strategy producing a documentation string that contains an
    /// unrecognized or malformed markup tag. Three families are generated:
    ///
    /// * an unclosed `\flag{` opener with no later closing brace,
    /// * an unclosed `\flag-negate{` opener with no later closing brace,
    /// * an unknown `\flag`-prefixed tag such as `\flagstuff{...}` or
    ///   `\flag-bogus{...}`.
    ///
    /// Generated names and bodies are drawn from `[a-z0-9-]`, so they never
    /// contain a `}` that could accidentally close an opener. The closed
    /// unknown-tag variants never reproduce a recognized opener: `\flag{`
    /// always has a letter immediately after `\flag`, and the `\flag-`
    /// variant filters out the lone valid suffix `negate`.
    fn malformed_markup() -> impl Strategy<Value = String> {
        let name = "[a-z0-9-]{0,12}";
        prop_oneof![
            // Unclosed `\flag{` opener: no closing brace anywhere.
            name.prop_map(|n| format!(r"text \flag{{{n}")),
            // Unclosed `\flag-negate{` opener: no closing brace anywhere.
            name.prop_map(|n| format!(r"text \flag-negate{{{n}")),
            // Unknown `\flag`-prefixed tag, e.g. `\flagstuff{...}`. The extra
            // letters guarantee this is not the `\flag{` opener.
            ("[a-z]{1,8}", name)
                .prop_map(|(extra, n)| format!(r"\flag{extra}{{{n}}}")),
            // Unknown `\flag-`-prefixed tag, e.g. `\flag-bogus{...}`, while
            // excluding the one valid `\flag-negate{` opener.
            ("[a-z]{1,8}", name)
                .prop_filter(
                    "avoid the valid \\flag-negate opener",
                    |(extra, _)| extra != "negate",
                )
                .prop_map(|(extra, n)| format!(r"\flag-{extra}{{{n}}}")),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 11: Markup resolves flag and negation cross-references
        //
        // **Validates: Requirements 5.1, 5.2**
        //
        // For any registry and any flag X in it, rendering `\flag{X}`
        // produces output containing X's long name; and for any X that has a
        // negated name, rendering `\flag-negate{X}` produces output containing
        // X's negated name. Both the plain and roff flavors are checked, with
        // roff comparisons accounting for centralized hyphen escaping.
        #[test]
        fn prop_markup_resolves_cross_references(
            flags in synth_registry(),
        ) {
            let v = synth_view(flags.clone());
            for flag in &flags {
                // `\flag{X}` resolves to X's long name (Requirement 5.1).
                let plain = render(
                    &format!(r"\flag{{{}}}", flag.long),
                    &v,
                    MarkupFlavor::Plain,
                )
                .unwrap();
                prop_assert!(
                    plain.contains(flag.long.as_str()),
                    "plain \\flag output {plain:?} missing long name {:?}",
                    flag.long,
                );

                let roff = render(
                    &format!(r"\flag{{{}}}", flag.long),
                    &v,
                    MarkupFlavor::Roff,
                )
                .unwrap();
                // In roff each hyphen of the name is escaped as `\-`, so we
                // compare against the escaped form.
                let escaped_long = flag.long.replace('-', r"\-");
                prop_assert!(
                    roff.contains(escaped_long.as_str()),
                    "roff \\flag output {roff:?} missing escaped long name \
                     {escaped_long:?}",
                );

                // `\flag-negate{X}` resolves to X's negated name when present
                // (Requirement 5.2).
                if let Some(negated) = flag.negated.as_ref() {
                    let plain_neg = render(
                        &format!(r"\flag-negate{{{}}}", flag.long),
                        &v,
                        MarkupFlavor::Plain,
                    )
                    .unwrap();
                    prop_assert!(
                        plain_neg.contains(negated.as_str()),
                        "plain \\flag-negate output {plain_neg:?} missing \
                         negated name {negated:?}",
                    );

                    let roff_neg = render(
                        &format!(r"\flag-negate{{{}}}", flag.long),
                        &v,
                        MarkupFlavor::Roff,
                    )
                    .unwrap();
                    let escaped_neg = negated.replace('-', r"\-");
                    prop_assert!(
                        roff_neg.contains(escaped_neg.as_str()),
                        "roff \\flag-negate output {roff_neg:?} missing \
                         escaped negated name {escaped_neg:?}",
                    );
                }
            }
        }

        // Feature: unified-flag-source, Property 12: Markup referencing an unknown name errors
        //
        // **Validates: Requirements 5.3**
        //
        // For any name absent from the registry, rendering a documentation
        // string containing `\flag{name}` or `\flag-negate{name}` returns an
        // error identifying the unresolved name and the offending tag, and no
        // artifact is produced. The absent name is built with an `absent-`
        // prefix the generated long names never use (they all begin with
        // `flag`), so it is guaranteed not to be in the registry. Both tags
        // and both flavors are exercised.
        #[test]
        fn prop_markup_unknown_name_errors(
            flags in synth_registry(),
            suffix in "[a-z0-9-]{0,12}",
        ) {
            let v = synth_view(flags);
            let absent = format!("absent-{suffix}");

            for flavor in [MarkupFlavor::Plain, MarkupFlavor::Roff] {
                // `\flag{absent}` errors, naming the unresolved name and tag,
                // and produces no artifact (no `Ok` value).
                let flag_tag = format!(r"\flag{{{absent}}}");
                let err = render(&flag_tag, &v, flavor).unwrap_err();
                prop_assert_eq!(
                    err,
                    MarkupError::UnknownFlag {
                        tag: flag_tag.clone(),
                        name: absent.clone(),
                    }
                );

                // `\flag-negate{absent}` likewise errors as UnknownFlag,
                // because the name itself is unresolved.
                let negate_tag = format!(r"\flag-negate{{{absent}}}");
                let err = render(&negate_tag, &v, flavor).unwrap_err();
                prop_assert_eq!(
                    err,
                    MarkupError::UnknownFlag {
                        tag: negate_tag.clone(),
                        name: absent.clone(),
                    }
                );
            }
        }

        // Feature: unified-flag-source, Property 13: Negation markup on a non-negatable flag errors
        //
        // **Validates: Requirements 5.4**
        //
        // For any flag in the registry that has no negated name, rendering a
        // documentation string containing `\flag-negate{that flag}` returns an
        // error identifying the flag (its long name) and the offending tag, and
        // no artifact is produced. We force the first generated flag's negation
        // to `None` so that at least one non-negatable flag always exists, then
        // assert the error for every non-negatable flag in both flavors.
        #[test]
        fn prop_markup_negate_non_negatable_errors(
            flags in synth_registry(),
        ) {
            // Guarantee at least one non-negatable flag is present.
            let mut flags = flags;
            flags[0].negated = None;

            let v = synth_view(flags.clone());
            for flag in flags.iter().filter(|f| f.negated.is_none()) {
                let tag = format!(r"\flag-negate{{{}}}", flag.long);
                for flavor in [MarkupFlavor::Plain, MarkupFlavor::Roff] {
                    // Negating a flag with no negated name errors as
                    // NoNegation, identifying the flag and the offending tag,
                    // and produces no artifact (no `Ok` value).
                    let err = render(&tag, &v, flavor).unwrap_err();
                    prop_assert_eq!(
                        err,
                        MarkupError::NoNegation {
                            tag: tag.clone(),
                            name: flag.long.clone(),
                        }
                    );
                }
            }
        }

        // Feature: unified-flag-source, Property 14: Malformed markup errors
        //
        // **Validates: Requirements 5.5**
        //
        // For any documentation string containing an unrecognized or malformed
        // markup tag — an unclosed `\flag{` or `\flag-negate{` opener, or an
        // unknown `\flag`-prefixed tag such as `\flagstuff{...}` or
        // `\flag-bogus{...}` — rendering returns `Err(MarkupError::Malformed)`
        // identifying the offending tag and never `Ok`, so no artifact is
        // produced. A synthetic registry is supplied so name lookups are
        // available, but the malformation is detected independent of name
        // resolution; both flavors are exercised.
        #[test]
        fn prop_markup_malformed_errors(
            flags in synth_registry(),
            doc in malformed_markup(),
        ) {
            let v = synth_view(flags);
            for flavor in [MarkupFlavor::Plain, MarkupFlavor::Roff] {
                let result = render(&doc, &v, flavor);
                prop_assert!(
                    matches!(result, Err(MarkupError::Malformed { .. })),
                    "expected Malformed error for {doc:?}, got {result:?}",
                );
            }
        }

        // Feature: unified-flag-source, Property 15: Hyphens in flag names are escaped in roff
        //
        // **Validates: Requirements 5.6**
        //
        // For any flag whose long (or negated) name contains hyphens,
        // rendering `\flag{name}` (resp. `\flag-negate{name}`) in
        // MarkupFlavor::Roff escapes every hyphen of that name as `\-` exactly
        // once, so the rendered man page displays a literal hyphen for each.
        // We assert this two ways: (1) the output contains the fully-escaped
        // form `name.replace('-', "\\-")`, and (2) the output contains no bare
        // hyphen — every `-` byte is immediately preceded by a backslash —
        // which rules out any unescaped or doubly-escaped hyphen. The
        // synthetic registry deliberately produces hyphen-rich long names
        // (e.g. `flag-0-a-b-c`) and negated names (e.g. `no-flag-0-a-b-c`).
        #[test]
        fn prop_roff_escapes_hyphens_in_flag_names(
            flags in synth_registry(),
        ) {
            let v = synth_view(flags.clone());

            // Assert that `roff` escapes every hyphen of `name` exactly once.
            fn assert_hyphens_escaped(
                roff: &str,
                name: &str,
            ) -> Result<(), TestCaseError> {
                // (1) The fully-escaped form of the name is present verbatim.
                let escaped = name.replace('-', r"\-");
                prop_assert!(
                    roff.contains(escaped.as_str()),
                    "roff output {roff:?} missing escaped name {escaped:?}",
                );
                // (2) No bare hyphen: every `-` is preceded by a backslash.
                let bytes = roff.as_bytes();
                for (i, &b) in bytes.iter().enumerate() {
                    if b == b'-' {
                        prop_assert!(
                            i > 0 && bytes[i - 1] == b'\\',
                            "roff output {roff:?} has a bare hyphen at byte \
                             {i} not preceded by a backslash",
                        );
                    }
                }
                Ok(())
            }

            for flag in &flags {
                if flag.long.contains('-') {
                    let roff = render(
                        &format!(r"\flag{{{}}}", flag.long),
                        &v,
                        MarkupFlavor::Roff,
                    )
                    .unwrap();
                    assert_hyphens_escaped(&roff, &flag.long)?;
                }

                // Negated names are hyphen-rich (e.g. `no-flag-0-a-b-c`);
                // exercise the negation path too.
                if let Some(negated) = flag.negated.as_ref() {
                    if negated.contains('-') {
                        let roff = render(
                            &format!(r"\flag-negate{{{}}}", flag.long),
                            &v,
                            MarkupFlavor::Roff,
                        )
                        .unwrap();
                        assert_hyphens_escaped(&roff, negated)?;
                    }
                }
            }
        }
    }
}

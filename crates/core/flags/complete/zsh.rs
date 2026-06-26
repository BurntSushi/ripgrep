/*!
Provides completions for ripgrep's CLI for the zsh shell.

Unlike the completion scripts for other shells, zsh's completion function for
ripgrep is *mostly* hand maintained. This is because:

1. It is lovingly written by an expert in such things.
2. It is much higher in quality than the ones that are auto-generated for the
other shells. Namely, the zsh completions take application level context about
flag compatibility into account.
3. There is a wealth of documentation in the zsh script explaining how it works
and how it can be extended.

What is *not* hand maintained any longer is the per-flag option list itself.
That list is now derived from the single canonical flag registry (see
[`crate::flags::RegistryView`]) and spliced into the hand-written `rg.zsh`
scaffold at the `!FLAGS!` marker, mirroring how the `!ENCODINGS!` and
`!HYPERLINK_ALIASES!` markers are filled in. The scaffold keeps its prelude,
helper functions, the positional-operand specs, and the `$no`-prefix
contextual behavior that hides rarely-used negation options unless the user
asks for them. Only the per-flag `_arguments` specs are generated, so the Zsh
completion can no longer drift from the actual set of ripgrep's flags.
*/

use crate::flags::{CompletionType, Flag, RegistryView};

/// Generate completions for zsh.
pub(crate) fn generate() -> String {
    generate_with(
        &RegistryView::load().expect("ripgrep's flag registry must validate"),
    )
}

/// Generate completions for zsh from the given (already validated) registry
/// view.
///
/// This is the registry-accepting seam behind [`generate`]: `generate` loads
/// ripgrep's canonical registry and delegates here, while tests pass synthetic
/// registries to exercise the generator across many inputs.
pub(crate) fn generate_with(view: &RegistryView) -> String {
    let flags = generate_flag_specs(view);

    let hyperlink_alias_descriptions = grep::printer::hyperlink_aliases()
        .iter()
        .map(|alias| {
            format!(r#"    {}:"{}""#, alias.name(), alias.description())
        })
        .collect::<Vec<String>>()
        .join("\n");
    include_str!("rg.zsh")
        .replace("!ENCODINGS!", super::ENCODINGS.trim_end())
        .replace("!HYPERLINK_ALIASES!", &hyperlink_alias_descriptions)
        .replace("!FLAGS!", flags.trim_end())
}

/// Build the block of per-flag `_arguments` specs spliced into the template.
///
/// Flags are emitted grouped by category, with categories in the fixed
/// declaration order and flags within each category in registry order, so the
/// output is deterministic (Requirements 7.1, 7.2, 7.3, 7.4).
fn generate_flag_specs(view: &RegistryView) -> String {
    let mut block = String::new();
    for (category, flags) in view.by_category() {
        if !block.is_empty() {
            block.push('\n');
        }
        block.push_str("    + ");
        block.push_str(category.as_str());
        block.push('\n');
        for flag in flags {
            for spec in flag_specs(flag) {
                block.push_str("    ");
                block.push_str(&spec);
                block.push('\n');
            }
        }
    }
    block
}

/// Build the one-or-more `_arguments` spec elements for a single flag.
///
/// The first element carries the long name (and the short name as a separately
/// completable option where present); the negated name and any aliases are
/// emitted as their own separately completable elements. Every element uses
/// the flag's `doc_short` verbatim as its description (Requirements 2.1, 2.2,
/// 2.3, 2.4, 2.5, 4.2), and value completion is derived from the flag's
/// completion type and choices (Requirement 3).
fn flag_specs(flag: &dyn Flag) -> Vec<String> {
    let mut out = Vec::new();
    let is_switch = flag.is_switch();
    let desc = escape_description(flag.doc_short());
    let value = value_completion(flag);
    let long = flag.name_long();
    // A value-taking long flag is suffixed with `=` so zsh accepts both
    // `--flag value` and `--flag=value`; a switch takes no suffix.
    let long_suffix = if is_switch { "" } else { "=" };

    // Primary entry: combine the short name (if any) and the long name using
    // brace expansion so they share a single description, exactly as a
    // hand-written zsh spec would.
    match flag.name_short() {
        Some(short) => {
            let short = char::from(short);
            // The short form of a value flag is suffixed with `+` so zsh
            // accepts both `-x value` and `-xvalue`.
            let short_suffix = if is_switch { "" } else { "+" };
            let names =
                format!("{{-{short}{short_suffix},--{long}{long_suffix}}}");
            let body = format!("[{desc}]{value}");
            out.push(format!("{names}{}", shell_quote(&body)));
        }
        None => {
            let body = format!("--{long}{long_suffix}[{desc}]{value}");
            out.push(shell_quote(&body));
        }
    }

    // Negated entry. A negation never takes a value, so it is emitted as a
    // switch. It is prefixed with `$no` so it stays hidden from the completion
    // menu unless the user explicitly asks for negation options (this is the
    // contextual behavior the scaffold's prelude sets up).
    if let Some(negated) = flag.name_negated() {
        let body = format!("--{negated}[{desc}]");
        out.push(format!("$no{}", shell_quote(&body)));
    }

    // Alias entries. Aliases mirror the flag's value-ness and never have a
    // short form, so they are emitted as long-only specs.
    for alias in flag.aliases() {
        let body = format!("--{alias}{long_suffix}[{desc}]{value}");
        out.push(shell_quote(&body));
    }

    out
}

/// Returns the trailing value-completion portion of a flag's spec.
///
/// This maps the flag's completion type (and declared choices) onto the
/// corresponding zsh completion construct (Requirement 3). A switch takes no
/// value and so contributes nothing.
fn value_completion(flag: &dyn Flag) -> String {
    if flag.is_switch() {
        return String::new();
    }
    let var = flag.doc_variable().unwrap_or("");
    match flag.completion_type() {
        CompletionType::Filename => ": :_files".to_string(),
        CompletionType::Executable => ": :_command_names -e".to_string(),
        CompletionType::Filetype => ": :_rg_types".to_string(),
        CompletionType::Encoding => ": :_rg_encodings".to_string(),
        CompletionType::Other => {
            let choices = flag.doc_choices();
            if choices.is_empty() {
                format!(":{var}")
            } else {
                // Offer exactly the declared choices, in declared order
                // (Requirement 3.3).
                format!(":{var}:({})", choices.join(" "))
            }
        }
    }
}

/// Escape a flag description for inclusion inside a zsh `[...]` spec.
///
/// Inside the brackets, `]` terminates the description and `\` is an escape
/// character, so both are escaped. Every other byte (including the trailing
/// period and interior punctuation) is preserved so the rendered description
/// stays character-for-character identical to `doc_short` (Requirement 2.4).
fn escape_description(desc: &str) -> String {
    let mut out = String::with_capacity(desc.len());
    for ch in desc.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ']' => out.push_str("\\]"),
            _ => out.push(ch),
        }
    }
    out
}

/// Wrap a spec body in shell quotes suitable for the zsh `args` array.
///
/// Single quotes are used when possible. When the body contains a single
/// quote (e.g. a description such as "Don't ..."), double quotes are used
/// instead and the characters that remain special inside double quotes are
/// escaped. This mirrors the quoting convention used throughout the
/// hand-written scaffold.
fn shell_quote(body: &str) -> String {
    if !body.contains('\'') {
        return format!("'{body}'");
    }
    let mut out = String::with_capacity(body.len() + 2);
    out.push('"');
    for ch in body.chars() {
        match ch {
            '\\' | '$' | '`' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::registry_tests::{build_registry, synthetic_registry};
    use proptest::prelude::*;

    /// Independent re-implementation of the zsh description escaping the
    /// generator documents and applies: inside an `_arguments` `[...]`
    /// description, `]` terminates the description and `\` is the escape
    /// character, so both are escaped and every other byte is preserved. The
    /// test re-derives this rather than calling the generator's own
    /// `escape_description`, so a regression in either one is caught.
    fn expected_escape(desc: &str) -> String {
        let mut out = String::with_capacity(desc.len());
        for ch in desc.chars() {
            match ch {
                '\\' => out.push_str("\\\\"),
                ']' => out.push_str("\\]"),
                _ => out.push(ch),
            }
        }
        out
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        // Feature: unified-flag-source, Property 4: Zsh produces one faithful
        // entry per flag
        //
        // For any registry, the Zsh artifact contains exactly one primary
        // completion entry per Flag_Definition that includes the long name;
        // includes the short name as a separately completable option for every
        // flag that has one; and uses a description that is
        // character-for-character identical to that flag's short documentation
        // (including the empty-description case).
        //
        // Validates: Requirements 2.1, 2.2, 2.4, 2.5, 2.6
        #[test]
        fn zsh_one_faithful_entry_per_flag(flags in synthetic_registry()) {
            // Capture each flag's salient attributes before the owned flags are
            // consumed by `build_registry`.
            let infos: Vec<(String, Option<u8>, bool, String)> = flags
                .iter()
                .map(|f| {
                    (f.long.clone(), f.short, f.switch, f.short_doc.clone())
                })
                .collect();

            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");
            let zsh = generate_with(&view);
            let lines: Vec<&str> = zsh.lines().collect();

            for (long, short, is_switch, short_doc) in infos {
                // A value-taking long flag carries a `=` suffix; a switch none.
                let long_suffix = if is_switch { "" } else { "=" };
                let escaped = expected_escape(&short_doc);
                // The description renders inside `[...]`; an empty doc_short
                // renders as the empty token `[]` (Requirement 2.5).
                let desc_token = format!("[{escaped}]");

                // Identify the flag's single primary completion entry. With a
                // short name it uses zsh brace expansion
                // `{-x<sfx>,--<long><sfx>}` so the short name is a separately
                // completable option (Requirement 2.2); without one it is a
                // long-only spec anchored on `'--<long><sfx>[`. Either form
                // carries the long name (Requirement 2.1). The negated entry
                // (`$no'--no-...`) and alias entries (`'--<long>-alias...`) use
                // distinct tokens and so are not matched here.
                let primary: Vec<&str> = match short {
                    Some(short) => {
                        let short = char::from(short);
                        let short_suffix = if is_switch { "" } else { "+" };
                        let brace = format!(
                            "{{-{short}{short_suffix},--{long}{long_suffix}}}"
                        );
                        lines
                            .iter()
                            .copied()
                            .filter(|l| l.contains(&brace))
                            .collect()
                    }
                    None => {
                        // The `[` immediately after the (suffixed) long name
                        // is what distinguishes the primary entry from the
                        // negated and alias entries, whose names differ before
                        // the bracket. Single quoting is used unless the
                        // description contains an apostrophe, in which case the
                        // generator switches to double quotes; both are
                        // accepted here.
                        let sq = format!("'--{long}{long_suffix}[");
                        let dq = format!("\"--{long}{long_suffix}[");
                        lines
                            .iter()
                            .copied()
                            .filter(|l| l.contains(&sq) || l.contains(&dq))
                            .collect()
                    }
                };

                // Requirement 2.1: exactly one primary entry includes the long
                // name.
                prop_assert_eq!(
                    primary.len(),
                    1,
                    "expected exactly one primary zsh entry for --{} \
                     (short={:?}, switch={}), found {}: {:#?}",
                    long,
                    short,
                    is_switch,
                    primary.len(),
                    primary
                );
                let entry = primary[0];

                // Requirement 2.2: a flag with a short name offers it as a
                // separately completable option alongside the long name (the
                // brace form pairs the two under one description).
                if let Some(short) = short {
                    let short = char::from(short);
                    prop_assert!(
                        entry.contains(&format!("-{short}"))
                            && entry.contains(&format!("--{long}")),
                        "primary entry for --{} must offer short -{} \
                         alongside the long name: {}",
                        long,
                        short,
                        entry
                    );
                }

                // Requirements 2.4/2.5/2.6: the description inside `[...]` is
                // character-for-character `doc_short` under the generator's
                // escaping, including the empty-description case.
                prop_assert!(
                    entry.contains(&desc_token),
                    "primary entry for --{} must carry description {:?} \
                     (from doc_short {:?}): {}",
                    long,
                    desc_token,
                    short_doc,
                    entry
                );
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 21: Generation is
        // deterministic
        //
        // For any registry and any generator, invoking that generator two or
        // more times against the unchanged registry produces byte-identical
        // artifacts, containing no content that varies between invocations
        // (e.g. timestamps, random ordering, or addresses). Every generator
        // behind the registry-accepting seams is exercised: all four shell
        // completion generators plus the man page and the short/long help.
        //
        // Validates: Requirements 7.1, 7.4
        #[test]
        fn generation_is_deterministic(flags in synthetic_registry()) {
            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            // Each generator is invoked twice against the same unchanged
            // registry view; its two artifacts must be byte-identical.
            prop_assert_eq!(
                crate::flags::complete::bash::generate_with(&view),
                crate::flags::complete::bash::generate_with(&view),
                "bash generation is not deterministic"
            );
            prop_assert_eq!(
                crate::flags::complete::zsh::generate_with(&view),
                crate::flags::complete::zsh::generate_with(&view),
                "zsh generation is not deterministic"
            );
            prop_assert_eq!(
                crate::flags::complete::fish::generate_with(&view),
                crate::flags::complete::fish::generate_with(&view),
                "fish generation is not deterministic"
            );
            prop_assert_eq!(
                crate::flags::complete::powershell::generate_with(&view),
                crate::flags::complete::powershell::generate_with(&view),
                "powershell generation is not deterministic"
            );
            prop_assert_eq!(
                crate::flags::doc::man::generate_with(&view),
                crate::flags::doc::man::generate_with(&view),
                "man generation is not deterministic"
            );
            prop_assert_eq!(
                crate::flags::doc::help::generate_short_with(&view),
                crate::flags::doc::help::generate_short_with(&view),
                "short help generation is not deterministic"
            );
            prop_assert_eq!(
                crate::flags::doc::help::generate_long_with(&view),
                crate::flags::doc::help::generate_long_with(&view),
                "long help generation is not deterministic"
            );
        }
    }
}

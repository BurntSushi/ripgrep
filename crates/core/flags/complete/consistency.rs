/*!
The automated `Consistency_Checker` (Requirement 6).

This module builds each shell completion artifact from the canonical flag
registry and verifies that the artifact faithfully reflects the registry: every
flag is present, no artifact references a name absent from the registry, and
the description text embedded in completion artifacts is character-for-character
identical to each flag's short documentation.

The checker is intentionally a `#[cfg(test)]` module living *inside* the crate
rather than a `tests/` integration test: it must call the generators'
`generate_with(&RegistryView)` seams and consume [`RegistryView`], both of which
are private to the `flags` module and so are not visible from an external test
crate. Living under `flags::complete` keeps it a descendant of `flags`, which is
exactly the visibility it needs.

# How checking works

For each [`ArtifactId`], the matching generator produces the artifact and a
small, format-specific *extractor* recovers the set of names the artifact
references (long flag names and short aliases) together with the per-flag
description text, where the format carries one. [`check_artifact`] then compares
that extracted view against the registry and emits one [`Violation`] for every
discrepancy:

* [`ViolationKind::MissingFlag`] — a registry flag's long name is absent from
  the artifact (Requirement 6.1).
* [`ViolationKind::UnexpectedLongFlag`] — the artifact references a long name
  that no registry flag declares as its long name, negated name, or alias
  (Requirement 6.2).
* [`ViolationKind::UnexpectedShortAlias`] — the artifact references a short
  alias that no registry flag declares (Requirement 6.3).
* [`ViolationKind::DescriptionMismatch`] — the description text the artifact
  carries for a flag differs (including case and interior whitespace) from that
  flag's `doc_short` (Requirement 6.4).

[`check_all`] runs every artifact and accumulates *all* violations from a single
run rather than stopping at the first (Requirement 6.5); an empty result means
every artifact agrees with the registry (Requirement 6.6).

Description text is only extracted for the artifacts that embed it — Zsh, Fish,
and PowerShell. The Bash completion's `opts` list carries flag names but no
descriptions, so description checking is scoped out for it.

The public items here (`check_all`, `check_artifact`, `Violation`,
`ViolationKind`, `ArtifactId`) are `pub(crate)` so the consistency property
tests (Properties 16-20) can drive the same checker against perturbed
artifacts.
*/

use std::collections::{BTreeMap, BTreeSet};

use crate::flags::RegistryView;

use super::{bash, fish, powershell, zsh};

/// Identifies the completion artifact a [`Violation`] was found in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)] // consumed by downstream checker property-test tasks
pub(crate) enum ArtifactId {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl ArtifactId {
    /// All artifact kinds the checker inspects, in a fixed order.
    #[allow(dead_code)] // consumed by downstream checker property-test tasks
    pub(crate) const ALL: [ArtifactId; 4] = [
        ArtifactId::Bash,
        ArtifactId::Zsh,
        ArtifactId::Fish,
        ArtifactId::PowerShell,
    ];
}

/// The specific kind of discrepancy a [`Violation`] records.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)] // fields are read by reporting/property-test tasks
pub(crate) enum ViolationKind {
    /// A flag present in the registry is absent from the artifact
    /// (Requirement 6.1). `name` is the missing flag's long name.
    MissingFlag { name: String },
    /// The artifact references a long flag name that no registry flag declares
    /// (as its long name, negated name, or alias) (Requirement 6.2).
    UnexpectedLongFlag { name: String },
    /// The artifact references a short flag alias that no registry flag
    /// declares (Requirement 6.3).
    UnexpectedShortAlias { name: String },
    /// The description text the artifact carries for `flag` is not
    /// character-for-character identical to that flag's short documentation
    /// (Requirement 6.4).
    DescriptionMismatch { flag: String, expected: String, actual: String },
}

/// One discrepancy between a generated artifact and the registry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)] // fields are read by reporting/property-test tasks
pub(crate) struct Violation {
    /// The artifact the discrepancy was found in.
    pub artifact: ArtifactId,
    /// The kind of discrepancy.
    pub kind: ViolationKind,
}

/// Check every completion artifact against `registry`, collecting all
/// violations from a single run (Requirements 6.5, 6.6).
///
/// Each artifact is produced by its generator from `registry` and then checked
/// with [`check_artifact`]. An empty result means every artifact faithfully
/// reflects the registry.
#[allow(dead_code)] // exercised by tests in this module and Property 20
pub(crate) fn check_all(registry: &RegistryView) -> Vec<Violation> {
    let mut out = Vec::new();
    out.extend(check_artifact(
        ArtifactId::Bash,
        &bash::generate_with(registry),
        registry,
    ));
    out.extend(check_artifact(
        ArtifactId::Zsh,
        &zsh::generate_with(registry),
        registry,
    ));
    out.extend(check_artifact(
        ArtifactId::Fish,
        &fish::generate_with(registry),
        registry,
    ));
    out.extend(check_artifact(
        ArtifactId::PowerShell,
        &powershell::generate_with(registry),
        registry,
    ));
    out
}

/// Check a single `artifact` string of kind `id` against `registry`.
///
/// This is the seam the consistency property tests use: they perturb a
/// generated artifact and pass the perturbed string here to confirm the
/// corresponding violation is reported. Names and (where the format carries
/// them) descriptions are recovered by the format-specific extractor for `id`,
/// then compared against the registry.
#[allow(dead_code)] // exercised by tests in this module and Properties 16-19
pub(crate) fn check_artifact(
    id: ArtifactId,
    artifact: &str,
    registry: &RegistryView,
) -> Vec<Violation> {
    let (extracted, check_descriptions) = match id {
        // Bash's `opts` list carries no descriptions, so description checking
        // is scoped out for it.
        ArtifactId::Bash => (extract_bash(artifact), false),
        ArtifactId::Zsh => (extract_zsh(artifact), true),
        ArtifactId::Fish => (extract_fish(artifact), true),
        ArtifactId::PowerShell => (extract_powershell(artifact), true),
    };
    compare(registry, id, &extracted, check_descriptions)
}

/// The names and descriptions an extractor recovered from one artifact.
struct Extracted {
    /// Long flag names the artifact references (without the leading `--`).
    longs: BTreeSet<String>,
    /// Short flag aliases the artifact references.
    shorts: BTreeSet<char>,
    /// The description text the artifact carries for each canonical long flag
    /// name. Empty for formats that do not embed descriptions (Bash).
    descriptions: BTreeMap<String, String>,
}

/// Compare an extracted artifact view against `registry`, accumulating every
/// violation (Requirement 6.5).
fn compare(
    registry: &RegistryView,
    id: ArtifactId,
    extracted: &Extracted,
    check_descriptions: bool,
) -> Vec<Violation> {
    // The set of names the registry legitimately exposes as completable long
    // flags: each flag's long name, its negated name, and every alias. An
    // artifact long name outside this set is unexpected (Requirement 6.2).
    let mut valid_longs: BTreeSet<&str> = BTreeSet::new();
    let mut valid_shorts: BTreeSet<char> = BTreeSet::new();
    for flag in registry.iter() {
        valid_longs.insert(flag.name_long());
        if let Some(negated) = flag.name_negated() {
            valid_longs.insert(negated);
        }
        for alias in flag.aliases() {
            valid_longs.insert(alias);
        }
        if let Some(short) = flag.name_short() {
            valid_shorts.insert(char::from(short));
        }
    }

    let mut out = Vec::new();

    // Requirement 6.1: every registry flag's long name must appear.
    for flag in registry.iter() {
        if !extracted.longs.contains(flag.name_long()) {
            out.push(Violation {
                artifact: id,
                kind: ViolationKind::MissingFlag {
                    name: flag.name_long().to_string(),
                },
            });
        }
    }

    // Requirement 6.2: no artifact long name may be absent from the registry.
    for long in &extracted.longs {
        if !valid_longs.contains(long.as_str()) {
            out.push(Violation {
                artifact: id,
                kind: ViolationKind::UnexpectedLongFlag { name: long.clone() },
            });
        }
    }

    // Requirement 6.3: no artifact short alias may be absent from the registry.
    for short in &extracted.shorts {
        if !valid_shorts.contains(short) {
            out.push(Violation {
                artifact: id,
                kind: ViolationKind::UnexpectedShortAlias {
                    name: short.to_string(),
                },
            });
        }
    }

    // Requirement 6.4: the description for each present flag must match its
    // short documentation exactly. A flag whose long name is missing entirely
    // is already reported above, so only present flags are description-checked.
    if check_descriptions {
        for flag in registry.iter() {
            if let Some(actual) = extracted.descriptions.get(flag.name_long())
            {
                if actual != flag.doc_short() {
                    out.push(Violation {
                        artifact: id,
                        kind: ViolationKind::DescriptionMismatch {
                            flag: flag.name_long().to_string(),
                            expected: flag.doc_short().to_string(),
                            actual: actual.clone(),
                        },
                    });
                }
            }
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Shared token helpers
// ---------------------------------------------------------------------------

/// Whether `b` may appear in a flag name. Flag names use ASCII alphanumerics,
/// hyphens, and underscores, so this is safe to apply byte-wise.
fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Extract every `--<name>` long-flag token from `s`, returning the names
/// without their leading `--`.
///
/// The scan reads a run of name bytes after each `--`, so it stops at the first
/// terminator (`=`, `[`, `}`, a quote, whitespace, and so on). Callers pass
/// only the structural, name-bearing portion of a line (never description
/// text), so this never mistakes prose containing `--` for a flag.
fn long_tokens(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let mut j = i + 2;
            while j < bytes.len() && is_name_byte(bytes[j]) {
                j += 1;
            }
            if j > i + 2 {
                out.push(s[i + 2..j].to_string());
            }
            i = j.max(i + 2);
        } else {
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Bash extractor
// ---------------------------------------------------------------------------

/// Recover the names referenced by the Bash completion.
///
/// The Bash script offers every completable flag in a single space-separated
/// `opts="..."` word list (the one that also contains the `<PATTERN>` and
/// `<PATH>...` operands). Tokens beginning with `--` are long names and tokens
/// of the form `-x` are short aliases; the operand placeholders are ignored.
/// Bash carries no descriptions.
fn extract_bash(artifact: &str) -> Extracted {
    let mut longs = BTreeSet::new();
    let mut shorts = BTreeSet::new();

    if let Some(opts) = bash_opts_list(artifact) {
        for token in opts.split_whitespace() {
            if let Some(name) = token.strip_prefix("--") {
                if !name.is_empty() {
                    longs.insert(name.to_string());
                }
            } else if let Some(rest) = token.strip_prefix('-') {
                let mut chars = rest.chars();
                if let (Some(c), None) = (chars.next(), chars.next()) {
                    shorts.insert(c);
                }
            }
            // `<PATTERN>` and `<PATH>...` start with `<` and are ignored.
        }
    }

    Extracted { longs, shorts, descriptions: BTreeMap::new() }
}

/// Return the contents of the `opts="..."` assignment that lists the
/// completable flags (identified by its `<PATTERN>` operand), or `None` if no
/// such assignment is present. The template also contains an empty `opts=""`
/// initializer, which this skips.
fn bash_opts_list(artifact: &str) -> Option<String> {
    let mut rest = artifact;
    while let Some(idx) = rest.find("opts=\"") {
        let after = &rest[idx + "opts=\"".len()..];
        match after.find('"') {
            Some(end) => {
                let content = &after[..end];
                if content.contains("<PATTERN>") {
                    return Some(content.to_string());
                }
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Fish extractor
// ---------------------------------------------------------------------------

/// Recover the names and descriptions referenced by the Fish completion.
///
/// Each completable flag is a `complete -c rg ...` line carrying its long name
/// after `-l`, its short alias (if any) after `-s`, and its description after
/// `-d`. Only lines that begin with `complete ` are inspected, so the prelude's
/// helper function (which uses `set -l`) is never mistaken for a flag.
///
/// The description for a flag's canonical long name is taken from the line
/// whose `-l` argument equals that long name; negated-name and alias lines key
/// their descriptions under their own (distinct) names and are harmless.
fn extract_fish(artifact: &str) -> Extracted {
    let mut longs = BTreeSet::new();
    let mut shorts = BTreeSet::new();
    let mut descriptions = BTreeMap::new();

    for line in artifact.lines() {
        if !line.trim_start().starts_with("complete ") {
            continue;
        }
        let tokens = fish_tokens(line);
        let mut this_long: Option<String> = None;
        let mut this_doc: Option<String> = None;
        let mut i = 0;
        while i < tokens.len() {
            let (text, quoted) = &tokens[i];
            if !quoted {
                match text.as_str() {
                    "-l" => {
                        if let Some((name, _)) = tokens.get(i + 1) {
                            longs.insert(name.clone());
                            this_long = Some(name.clone());
                            i += 2;
                            continue;
                        }
                    }
                    "-s" => {
                        if let Some((name, _)) = tokens.get(i + 1) {
                            let mut chars = name.chars();
                            if let (Some(c), None) =
                                (chars.next(), chars.next())
                            {
                                shorts.insert(c);
                            }
                            i += 2;
                            continue;
                        }
                    }
                    "-d" => {
                        if let Some((doc, _)) = tokens.get(i + 1) {
                            this_doc = Some(doc.clone());
                            i += 2;
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        if let (Some(long), Some(doc)) = (this_long, this_doc) {
            descriptions.entry(long).or_insert(doc);
        }
    }

    Extracted { longs, shorts, descriptions }
}

/// Tokenize a single Fish `complete` line into `(text, was_quoted)` pairs.
///
/// Single- and double-quoted strings become one token each. Inside single
/// quotes the only escape the generator emits is `\'` for an embedded
/// apostrophe (it applies `doc_short.replace("'", "\\'")`), so this reverses
/// exactly that, leaving any other backslash literal. An unterminated quote
/// (as in an encoding flag's multi-line `-a '...'` list) simply runs to the end
/// of the line; the names and description appear before it, so this is fine.
fn fish_tokens(line: &str) -> Vec<(String, bool)> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '\'' {
            i += 1;
            let mut val = String::new();
            while i < chars.len() {
                if chars[i] == '\\'
                    && i + 1 < chars.len()
                    && chars[i + 1] == '\''
                {
                    val.push('\'');
                    i += 2;
                } else if chars[i] == '\'' {
                    i += 1;
                    break;
                } else {
                    val.push(chars[i]);
                    i += 1;
                }
            }
            out.push((val, true));
        } else if c == '"' {
            i += 1;
            let mut val = String::new();
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    val.push(chars[i + 1]);
                    i += 2;
                } else if chars[i] == '"' {
                    i += 1;
                    break;
                } else {
                    val.push(chars[i]);
                    i += 1;
                }
            }
            out.push((val, true));
        } else {
            let mut val = String::new();
            while i < chars.len() && !chars[i].is_whitespace() {
                val.push(chars[i]);
                i += 1;
            }
            out.push((val, false));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// PowerShell extractor
// ---------------------------------------------------------------------------

/// Recover the names and descriptions referenced by the PowerShell completion.
///
/// Each completable flag is a `[CompletionResult]::new('<dash-name>', '<name>',
/// [CompletionResultType]::ParameterName, '<doc>')` entry. The single-quoted
/// arguments are, in order, the dash-prefixed completable name, the display
/// name, and the description (the unquoted `[CompletionResultType]::...` is
/// skipped). Value-completion entries use `$`-variables rather than quoted
/// names and so contribute nothing here.
fn extract_powershell(artifact: &str) -> Extracted {
    let mut longs = BTreeSet::new();
    let mut shorts = BTreeSet::new();
    let mut descriptions = BTreeMap::new();

    for line in artifact.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("[CompletionResult]::new(") else {
            continue;
        };
        let args = powershell_quoted_args(rest);
        let Some(dash) = args.first() else {
            continue;
        };
        if let Some(name) = dash.strip_prefix("--") {
            longs.insert(name.to_string());
            if let Some(doc) = args.get(2) {
                descriptions.entry(name.to_string()).or_insert(doc.clone());
            }
        } else if let Some(rest) = dash.strip_prefix('-') {
            let mut chars = rest.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                shorts.insert(c);
            }
        }
    }

    Extracted { longs, shorts, descriptions }
}

/// Extract the PowerShell single-quoted string literals from `s`, in order,
/// decoding the `''` escape for an embedded apostrophe (the generator applies
/// `doc_short.replace("'", "''")`). Unquoted tokens between literals are
/// ignored.
fn powershell_quoted_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\'' {
            continue;
        }
        let mut val = String::new();
        loop {
            match chars.next() {
                None => break,
                Some('\'') => {
                    if chars.peek() == Some(&'\'') {
                        chars.next();
                        val.push('\'');
                    } else {
                        break;
                    }
                }
                Some(other) => val.push(other),
            }
        }
        out.push(val);
    }
    out
}

// ---------------------------------------------------------------------------
// Zsh extractor
// ---------------------------------------------------------------------------

/// Recover the names and descriptions referenced by the Zsh completion.
///
/// Only the generated per-flag `_arguments` specs are inspected, by slicing the
/// `args=( ... )` array between its start and the hand-written `+ operand`
/// group. Within that slice, category-header lines (which start with `+`) are
/// skipped and every remaining line is a flag spec.
///
/// Names are read from the portion of each spec before its `[description]`
/// bracket: `--<long>`/`--<negated>`/`--<alias>` tokens and the short name in a
/// `{-x...,--long...}` brace expansion. The description for a canonical long
/// name is decoded from that flag's primary spec (negated `$no'...'` entries
/// are skipped so the canonical entry, not the negation, supplies it).
fn extract_zsh(artifact: &str) -> Extracted {
    let mut longs = BTreeSet::new();
    let mut shorts = BTreeSet::new();
    let mut descriptions = BTreeMap::new();

    for line in zsh_flag_block(artifact).lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('+') {
            continue;
        }
        // Names live before the description bracket; never inside it.
        let name_part =
            trimmed.split_once('[').map(|(a, _)| a).unwrap_or(trimmed);
        for long in long_tokens(name_part) {
            longs.insert(long);
        }
        for short in zsh_short_tokens(name_part) {
            shorts.insert(short);
        }

        // The canonical description comes from a flag's primary entry, which is
        // not the `$no`-prefixed negation entry.
        if trimmed.starts_with("$no") {
            continue;
        }
        if let Some(long) = long_tokens(name_part).into_iter().next() {
            if let Some(desc) = zsh_decode_description(trimmed) {
                descriptions.entry(long).or_insert(desc);
            }
        }
    }

    Extracted { longs, shorts, descriptions }
}

/// Slice out the generated per-flag spec block from the Zsh artifact: the
/// contents of the `args=( ... )` array up to the hand-written `+ operand`
/// group that follows the generated flags.
fn zsh_flag_block(artifact: &str) -> &str {
    let marker = "args=(";
    let Some(start) = artifact.find(marker) else {
        return "";
    };
    let rest = &artifact[start + marker.len()..];
    let end = rest.find("+ operand").unwrap_or(rest.len());
    &rest[..end]
}

/// Extract the short-name bytes from any `{-x...,--long...}` brace expansions
/// in `s`. The short name is the character immediately following `{-`.
fn zsh_short_tokens(s: &str) -> Vec<char> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < chars.len() {
        if chars[i] == '{' && chars[i + 1] == '-' && chars[i + 2] != '-' {
            out.push(chars[i + 2]);
        }
        i += 1;
    }
    out
}

/// Decode the description carried by a single Zsh spec line back to the literal
/// `doc_short` it was generated from.
///
/// This inverts the generator's two encoding layers, in reverse order:
///
/// 1. The spec body is wrapped by `shell_quote` in single quotes (verbatim) or,
///    when it contains an apostrophe, in double quotes that backslash-escape
///    `\`, `$`, `` ` ``, and `"`. The opening quote is the first quote on the
///    line (the brace expansion and `$no` prefix precede it but contain no
///    quotes), and this layer is undone first.
/// 2. Inside the body, the description sits in `[...]` where `escape_description`
///    has replaced `\` with `\\` and `]` with `\]`. The first `[` opens the
///    description and the first *unescaped* `]` closes it; this layer is undone
///    to recover the original text.
///
/// Returns `None` if the line carries no quoted body or no `[...]` description.
fn zsh_decode_description(line: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();

    // 1. Find the opening quote and decode the shell-quoted body.
    let mut i = 0;
    while i < chars.len() && chars[i] != '\'' && chars[i] != '"' {
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    let quote = chars[i];
    i += 1;
    let mut body = String::new();
    if quote == '\'' {
        while i < chars.len() && chars[i] != '\'' {
            body.push(chars[i]);
            i += 1;
        }
    } else {
        while i < chars.len() && chars[i] != '"' {
            if chars[i] == '\\' && i + 1 < chars.len() {
                body.push(chars[i + 1]);
                i += 2;
            } else {
                body.push(chars[i]);
                i += 1;
            }
        }
    }

    // 2. Locate the `[...]` description within the body and undo the
    //    `escape_description` escaping.
    let body: Vec<char> = body.chars().collect();
    let mut j = 0;
    while j < body.len() && body[j] != '[' {
        j += 1;
    }
    if j >= body.len() {
        return None;
    }
    j += 1;
    let mut desc = String::new();
    while j < body.len() {
        if body[j] == '\\' && j + 1 < body.len() {
            desc.push(body[j + 1]);
            j += 2;
        } else if body[j] == ']' {
            return Some(desc);
        } else {
            desc.push(body[j]);
            j += 1;
        }
    }
    // No closing bracket found; return what we have.
    Some(desc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::registry_tests::{
        ArtifactPerturbation, SyntheticFlag, apply_perturbation,
        build_registry, synthetic_registry,
    };
    use crate::flags::{Category, CompletionType, RegistryView};
    use proptest::prelude::*;

    /// Generate the completion artifact of kind `id` from `view`, dispatching to
    /// the matching generator's `generate_with` seam. Shared by the checker
    /// property tests so they can drive every artifact uniformly.
    fn generate_artifact(id: ArtifactId, view: &RegistryView) -> String {
        match id {
            ArtifactId::Bash => bash::generate_with(view),
            ArtifactId::Zsh => zsh::generate_with(view),
            ArtifactId::Fish => fish::generate_with(view),
            ArtifactId::PowerShell => powershell::generate_with(view),
        }
    }

    /// Build a small, fixed, valid synthetic registry view: one switch flag
    /// with a short name and one value flag (a filename) with a negation. This
    /// is enough to exercise every extractor and violation kind deterministically.
    fn fixture() -> RegistryView {
        let flags = build_registry(vec![
            SyntheticFlag {
                long: "alpha".to_string(),
                short: Some(b'a'),
                negated: None,
                switch: true,
                variable: None,
                category: Category::Search,
                short_doc: "do alpha".to_string(),
                long_doc: "the long alpha documentation".to_string(),
                aliases: Vec::new(),
                completion: CompletionType::Other,
                choices: Vec::new(),
            },
            SyntheticFlag {
                long: "beta".to_string(),
                short: None,
                negated: Some("no-beta".to_string()),
                switch: false,
                variable: Some("BETA".to_string()),
                category: Category::Output,
                short_doc: "do beta".to_string(),
                long_doc: "the long beta documentation".to_string(),
                aliases: Vec::new(),
                completion: CompletionType::Filename,
                choices: Vec::new(),
            },
        ]);
        RegistryView::new(flags).expect("fixture registry must validate")
    }

    /// Requirement 6.6 baseline: the real registry and the real generators
    /// agree, so the checker reports no violations.
    ///
    /// This test is the authoritative CI drift guard. Run by the normal
    /// `cargo test` suite, it checks all four shell completions against the
    /// real registry and supersedes the loose, zsh-only `ci/test-complete`
    /// shell script. On failure it prints the full violation list so the
    /// divergence is visible at a glance.
    #[test]
    fn real_registry_is_consistent() {
        let view = RegistryView::load().expect("real registry must validate");
        let violations = check_all(&view);
        assert!(
            violations.is_empty(),
            "real artifacts diverge from the registry: {violations:#?}"
        );
    }

    /// Requirement 6.6: faithful synthetic artifacts pass the checker.
    #[test]
    fn faithful_synthetic_artifacts_pass() {
        let view = fixture();
        assert_eq!(check_all(&view), Vec::new());
    }

    /// Requirement 6.1: dropping a flag's primary line from an artifact is
    /// reported as a missing flag.
    #[test]
    fn detects_missing_flag() {
        let view = fixture();
        let fish = fish::generate_with(&view);
        // Remove the canonical `complete` line for `alpha`.
        let perturbed: String = fish
            .lines()
            .filter(|l| !l.contains("-l alpha "))
            .collect::<Vec<_>>()
            .join("\n");
        let violations = check_artifact(ArtifactId::Fish, &perturbed, &view);
        assert!(
            violations.contains(&Violation {
                artifact: ArtifactId::Fish,
                kind: ViolationKind::MissingFlag { name: "alpha".to_string() },
            }),
            "expected a MissingFlag for alpha, got: {violations:#?}"
        );
    }

    /// Requirement 6.2: an artifact long name absent from the registry is
    /// reported.
    #[test]
    fn detects_unexpected_long_flag() {
        let view = fixture();
        let mut fish = fish::generate_with(&view);
        fish.push_str("\ncomplete -c rg -l bogus -d 'sneaky'\n");
        let violations = check_artifact(ArtifactId::Fish, &fish, &view);
        assert!(
            violations.contains(&Violation {
                artifact: ArtifactId::Fish,
                kind: ViolationKind::UnexpectedLongFlag {
                    name: "bogus".to_string()
                },
            }),
            "expected an UnexpectedLongFlag for bogus, got: {violations:#?}"
        );
    }

    /// Requirement 6.3: an artifact short alias absent from the registry is
    /// reported. The injected line reuses the valid long name `alpha` so only
    /// the short alias is unexpected.
    #[test]
    fn detects_unexpected_short_alias() {
        let view = fixture();
        let mut fish = fish::generate_with(&view);
        fish.push_str("\ncomplete -c rg -s z -l alpha -d 'do alpha'\n");
        let violations = check_artifact(ArtifactId::Fish, &fish, &view);
        assert!(
            violations.contains(&Violation {
                artifact: ArtifactId::Fish,
                kind: ViolationKind::UnexpectedShortAlias {
                    name: "z".to_string()
                },
            }),
            "expected an UnexpectedShortAlias for z, got: {violations:#?}"
        );
    }

    /// Requirement 6.4: a description that diverges from `doc_short` is reported
    /// with both the expected and actual text.
    #[test]
    fn detects_description_mismatch() {
        let view = fixture();
        let fish = fish::generate_with(&view);
        let perturbed = fish.replace("-d 'do alpha'", "-d 'WRONG'");
        assert_ne!(perturbed, fish, "perturbation must change the artifact");
        let violations = check_artifact(ArtifactId::Fish, &perturbed, &view);
        assert!(
            violations.contains(&Violation {
                artifact: ArtifactId::Fish,
                kind: ViolationKind::DescriptionMismatch {
                    flag: "alpha".to_string(),
                    expected: "do alpha".to_string(),
                    actual: "WRONG".to_string(),
                },
            }),
            "expected a DescriptionMismatch for alpha, got: {violations:#?}"
        );
    }

    /// Requirement 6.5: a single run reports violations from every affected
    /// artifact, not just the first.
    #[test]
    fn reports_violations_across_artifacts() {
        let view = fixture();
        // Inject an unexpected long flag into both the Fish and PowerShell
        // artifacts and confirm both are reported in one run.
        let mut fish = fish::generate_with(&view);
        fish.push_str("\ncomplete -c rg -l fishbogus -d 'x'\n");
        let mut powershell = powershell::generate_with(&view);
        powershell.push_str(
            "\n      [CompletionResult]::new('--psbogus', 'psbogus', \
             [CompletionResultType]::ParameterName, 'x')\n",
        );

        let mut violations = check_artifact(ArtifactId::Fish, &fish, &view);
        violations.extend(check_artifact(
            ArtifactId::PowerShell,
            &powershell,
            &view,
        ));

        assert!(violations.contains(&Violation {
            artifact: ArtifactId::Fish,
            kind: ViolationKind::UnexpectedLongFlag {
                name: "fishbogus".to_string()
            },
        }));
        assert!(violations.contains(&Violation {
            artifact: ArtifactId::PowerShell,
            kind: ViolationKind::UnexpectedLongFlag {
                name: "psbogus".to_string()
            },
        }));
    }

    /// The Zsh and PowerShell extractors recover descriptions faithfully on a
    /// synthetic registry (so a faithful artifact yields no description
    /// mismatch). This guards the more involved Zsh decoding in particular.
    #[test]
    fn zsh_and_powershell_descriptions_round_trip() {
        let view = fixture();
        let zsh_violations =
            check_artifact(ArtifactId::Zsh, &zsh::generate_with(&view), &view);
        assert_eq!(zsh_violations, Vec::new());
        let ps_violations = check_artifact(
            ArtifactId::PowerShell,
            &powershell::generate_with(&view),
            &view,
        );
        assert_eq!(ps_violations, Vec::new());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Feature: unified-flag-source, Property 16: Checker detects missing flags
        //
        // For any registry (>= 2 flags) and any artifact from which one flag's
        // references are removed, the checker reports a `MissingFlag` for that
        // flag's long name against the affected artifact (Requirement 6.1).
        //
        // The synthetic registry assigns index-derived long names (e.g.
        // `flag0long`, `flag-1-a-b-c`), so a chosen flag's long name is never a
        // substring of another flag's long name. Dropping every line that
        // references it therefore removes only that flag (plus its own negation
        // and aliases), never another flag's primary reference.
        #[test]
        fn checker_detects_missing_flags(
            flags in synthetic_registry()
                .prop_filter("registry needs >= 2 flags", |f| f.len() >= 2),
            chosen in any::<prop::sample::Index>(),
        ) {
            let chosen_long = flags[chosen.index(flags.len())].long.clone();
            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            for id in ArtifactId::ALL {
                let artifact = generate_artifact(id, &view);
                // Simulate a missing flag: drop every line referencing the
                // chosen flag's long name.
                let perturbed = apply_perturbation(
                    &artifact,
                    &ArtifactPerturbation::DropFlag {
                        flag: chosen_long.clone(),
                    },
                );
                let violations = check_artifact(id, &perturbed, &view);
                prop_assert!(
                    violations.contains(&Violation {
                        artifact: id,
                        kind: ViolationKind::MissingFlag {
                            name: chosen_long.clone(),
                        },
                    }),
                    "expected MissingFlag {{{chosen_long}}} for {id:?}, \
                     got: {violations:#?}"
                );
            }
        }

        // Feature: unified-flag-source, Property 17: Checker detects unexpected names
        //
        // For any synthetic registry, an artifact that references a long name
        // or short alias absent from the registry is reported by the checker as
        // an `UnexpectedLongFlag` / `UnexpectedShortAlias` against the affected
        // artifact (Requirements 6.2, 6.3).
        //
        // The unexpected names are injected via a single Fish `complete` line
        // (the Fish extractor reads the long name after `-l` and the short
        // alias after `-s`). The chosen names are guaranteed absent from any
        // synthetic registry: synthetic long names are `flagNlong` /
        // `flag-N-a-b-c` (with `no-`-prefixed negations and `-aliasJ` aliases),
        // none of which equal `zzbogus`; and synthetic short names are drawn
        // from the front of the short pool (`a`, `b`, ... for at most 7 flags),
        // so `q` is never assigned.
        #[test]
        fn checker_detects_unexpected_names(
            flags in synthetic_registry(),
        ) {
            let view = RegistryView::new(build_registry(flags))
                .expect("synthetic registry must validate");

            // Inject one unexpected long name and one unexpected short alias
            // into the Fish artifact via a single `complete` line.
            let artifact = apply_perturbation(
                &generate_artifact(ArtifactId::Fish, &view),
                &ArtifactPerturbation::AddUnexpected {
                    line: "complete -c rg -s q -l zzbogus -d 'sneaky'"
                        .to_string(),
                },
            );
            let violations =
                check_artifact(ArtifactId::Fish, &artifact, &view);

            prop_assert!(
                violations.contains(&Violation {
                    artifact: ArtifactId::Fish,
                    kind: ViolationKind::UnexpectedLongFlag {
                        name: "zzbogus".to_string(),
                    },
                }),
                "expected UnexpectedLongFlag {{zzbogus}} for Fish, \
                 got: {violations:#?}"
            );
            prop_assert!(
                violations.contains(&Violation {
                    artifact: ArtifactId::Fish,
                    kind: ViolationKind::UnexpectedShortAlias {
                        name: "q".to_string(),
                    },
                }),
                "expected UnexpectedShortAlias {{q}} for Fish, \
                 got: {violations:#?}"
            );
        }

        // Feature: unified-flag-source, Property 18: Checker detects description mismatches
        //
        // For any synthetic registry whose flags carry distinctive short docs
        // (`desc0`, `desc1`, ...), replacing one chosen flag's embedded
        // description in a description-bearing artifact (Zsh, Fish, PowerShell)
        // with clearly different text makes the checker report a
        // `DescriptionMismatch` for that flag against the affected artifact,
        // carrying both the expected (`doc_short`) and actual (perturbed) text
        // (Requirement 6.4).
        //
        // Each flag's `doc_short` is overridden to the unique token `descN`
        // (N = its index, always single-digit for the <= 7 synthetic flags),
        // which appears nowhere else in any artifact: long names are
        // `flagNlong` / `flag-N-a-b-c`, value variables `VALN`, and long docs
        // carry no `desc`. So replacing `descN` targets exactly the chosen
        // flag's description and nothing else. Bash embeds no descriptions and
        // is excluded.
        #[test]
        fn checker_detects_description_mismatch(
            mut flags in synthetic_registry(),
            chosen in any::<prop::sample::Index>(),
        ) {
            // Give every flag a unique, distinctive short doc so a single
            // description can be targeted by string replacement.
            for (i, flag) in flags.iter_mut().enumerate() {
                flag.short_doc = format!("desc{i}");
            }
            let idx = chosen.index(flags.len());
            let chosen_long = flags[idx].long.clone();
            let expected = format!("desc{idx}");
            let actual = format!("MISMATCH{idx}");

            let view = RegistryView::new(build_registry(flags))
                .expect("synthetic registry must validate");

            // Bash carries no descriptions, so only these three embed them.
            let description_bearing =
                [ArtifactId::Zsh, ArtifactId::Fish, ArtifactId::PowerShell];
            for id in description_bearing {
                let artifact = generate_artifact(id, &view);
                // Replace the chosen flag's description with clearly different
                // text, leaving every other flag's description intact.
                let perturbed = apply_perturbation(
                    &artifact,
                    &ArtifactPerturbation::AlterDescription {
                        from: expected.clone(),
                        to: actual.clone(),
                    },
                );
                prop_assert_ne!(
                    perturbed.clone(),
                    artifact,
                    "perturbation must change the {:?} artifact",
                    id
                );
                let violations = check_artifact(id, &perturbed, &view);
                prop_assert!(
                    violations.contains(&Violation {
                        artifact: id,
                        kind: ViolationKind::DescriptionMismatch {
                            flag: chosen_long.clone(),
                            expected: expected.clone(),
                            actual: actual.clone(),
                        },
                    }),
                    "expected DescriptionMismatch for {chosen_long} in \
                     {id:?} (expected {expected:?}, actual {actual:?}), \
                     got: {violations:#?}"
                );
            }
        }

        // Feature: unified-flag-source, Property 19: Checker reports all violations in one run
        //
        // For any synthetic registry (>= 2 flags), injecting several
        // *distinct* violations across *multiple* artifacts of *multiple*
        // kinds in a single run — an unexpected long name in Fish, an
        // unexpected long name in PowerShell, and a dropped (missing) flag in
        // Zsh — makes the checker report *every* injected violation rather
        // than only the first. The per-artifact results from each
        // `check_artifact` are accumulated into one `Vec` exactly as
        // [`check_all`] does (Requirement 6.5).
        //
        // The unexpected names `zzbogus1` / `zzbogus2` are guaranteed absent
        // from any synthetic registry, whose names are `flagNlong` /
        // `flag-N-a-b-c` with `no-`-prefixed negations and `-aliasJ` aliases;
        // none equal `zzbogus1` or `zzbogus2`. The dropped flag's
        // index-derived long name is never a substring of another flag's long
        // name (indices are single-digit for the <= 7 synthetic flags), so
        // dropping every line that references it removes only that flag (plus
        // its own negation and aliases) while the rest of the registry stays
        // present — hence exactly one `MissingFlag` for it.
        #[test]
        fn checker_reports_all_violations_in_one_run(
            flags in synthetic_registry()
                .prop_filter("registry needs >= 2 flags", |f| f.len() >= 2),
            chosen in any::<prop::sample::Index>(),
        ) {
            let dropped_long = flags[chosen.index(flags.len())].long.clone();
            let view = RegistryView::new(build_registry(flags))
                .expect("synthetic registry must validate");

            // Violation 1: an unexpected long name in the Fish artifact.
            let fish = apply_perturbation(
                &generate_artifact(ArtifactId::Fish, &view),
                &ArtifactPerturbation::AddUnexpected {
                    line: "complete -c rg -l zzbogus1 -d 'sneaky'"
                        .to_string(),
                },
            );
            // Violation 2: an unexpected long name in the PowerShell artifact.
            let powershell = apply_perturbation(
                &generate_artifact(ArtifactId::PowerShell, &view),
                &ArtifactPerturbation::AddUnexpected {
                    line: "      [CompletionResult]::new('--zzbogus2', \
                           'zzbogus2', \
                           [CompletionResultType]::ParameterName, 'sneaky')"
                        .to_string(),
                },
            );
            // Violation 3: a dropped (missing) flag in the Zsh artifact.
            let zsh = apply_perturbation(
                &generate_artifact(ArtifactId::Zsh, &view),
                &ArtifactPerturbation::DropFlag {
                    flag: dropped_long.clone(),
                },
            );

            // Accumulate every artifact's violations into one combined result,
            // mirroring how `check_all` aggregates across artifacts.
            let mut violations =
                check_artifact(ArtifactId::Fish, &fish, &view);
            violations.extend(check_artifact(
                ArtifactId::PowerShell,
                &powershell,
                &view,
            ));
            violations
                .extend(check_artifact(ArtifactId::Zsh, &zsh, &view));

            // All three injected violations must appear in the single run,
            // not merely the first.
            prop_assert!(
                violations.contains(&Violation {
                    artifact: ArtifactId::Fish,
                    kind: ViolationKind::UnexpectedLongFlag {
                        name: "zzbogus1".to_string(),
                    },
                }),
                "expected UnexpectedLongFlag {{zzbogus1}} for Fish, \
                 got: {violations:#?}"
            );
            prop_assert!(
                violations.contains(&Violation {
                    artifact: ArtifactId::PowerShell,
                    kind: ViolationKind::UnexpectedLongFlag {
                        name: "zzbogus2".to_string(),
                    },
                }),
                "expected UnexpectedLongFlag {{zzbogus2}} for PowerShell, \
                 got: {violations:#?}"
            );
            prop_assert!(
                violations.contains(&Violation {
                    artifact: ArtifactId::Zsh,
                    kind: ViolationKind::MissingFlag {
                        name: dropped_long.clone(),
                    },
                }),
                "expected MissingFlag {{{dropped_long}}} for Zsh, \
                 got: {violations:#?}"
            );
        }

        // Feature: unified-flag-source, Property 20: Faithful artifacts pass the checker
        //
        // For any synthetic registry, running the consistency checker against
        // the artifacts actually produced by the generators (no perturbation)
        // yields a success result with no violations: every generated artifact
        // faithfully reflects the registry (Requirement 6.6).
        //
        // The synthetic registry exercises the full input space the checker
        // must accept without complaint: varied `short_doc` values (including
        // the empty string and interior-whitespace strings like
        // "Toggle  spaced   words"), every completion type, optional short
        // names, optional negations, and aliases. `check_all` runs every
        // artifact (Bash, Zsh, Fish, PowerShell) from the same registry the
        // checker compares against, so a faithful round-trip must report zero
        // violations across all of them.
        #[test]
        fn faithful_artifacts_pass_the_checker(
            flags in synthetic_registry(),
        ) {
            let view = RegistryView::new(build_registry(flags))
                .expect("synthetic registry must validate");
            let violations = check_all(&view);
            prop_assert_eq!(
                violations.clone(),
                Vec::new(),
                "faithful artifacts must report no violations, got: \
                 {:#?}",
                violations
            );
        }
    }
}

/*!
Provides completions for ripgrep's CLI for PowerShell.
*/

use crate::flags::{CompletionType, Flag, RegistryView};

const TEMPLATE: &'static str = "
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'rg' -ScriptBlock {
  param($wordToComplete, $commandAst, $cursorPosition)
  $commandElements = $commandAst.CommandElements

  # If the token preceding the one being completed is a flag that accepts a
  # value, complete that value according to the flag's completion type instead
  # of completing another flag name.
  $previousElement = ''
  if ($commandElements.Count -ge 1) {
    if ($commandElements[-1].Value -eq $wordToComplete) {
      if ($commandElements.Count -ge 2) {
        $previousElement = $commandElements[-2].Value
      }
    } else {
      $previousElement = $commandElements[-1].Value
    }
  }

  $valueCompletions = @(switch ($previousElement) {
!VALUES!
  })
  if ($valueCompletions.Count -gt 0) {
    return $valueCompletions
  }

  $command = @(
    'rg'
    for ($i = 1; $i -lt $commandElements.Count; $i++) {
        $element = $commandElements[$i]
        if ($element -isnot [StringConstantExpressionAst] -or
            $element.StringConstantType -ne [StringConstantType]::BareWord -or
            $element.Value.StartsWith('-')) {
            break
    }
    $element.Value
  }) -join ';'

  $completions = @(switch ($command) {
    'rg' {
!FLAGS!
    }
  })

  $completions.Where{ $_.CompletionText -like \"$wordToComplete*\" } |
    Sort-Object -Property ListItemText
}
";

const TEMPLATE_FLAG: &'static str = "[CompletionResult]::new('!DASH_NAME!', '!NAME!', [CompletionResultType]::ParameterName, '!DOC!')";

/// Generate completions for PowerShell.
///
/// Note that these completions are based on what was produced for ripgrep <=13
/// using Clap 2.x. Improvements on this are welcome.
///
/// All flag-specific content is derived from the canonical flag registry
/// (Requirement 1.5). The registry is validated once per generation; on
/// failure the real registry is known-valid, so validation must succeed here.
pub(crate) fn generate() -> String {
    generate_with(
        &RegistryView::load()
            .expect("ripgrep's flag registry should validate"),
    )
}

/// Generate completions for PowerShell from the given (already validated)
/// registry view.
///
/// This is the registry-accepting seam behind [`generate`]: `generate` loads
/// ripgrep's canonical registry and delegates here, while tests pass synthetic
/// registries to exercise the generator across many inputs.
pub(crate) fn generate_with(registry: &RegistryView) -> String {
    // The set of completable flags. Every name a flag can be invoked as is
    // offered as a separately completable parameter name: its long name, its
    // short name, its negated name (Requirement 4.1), and every alias
    // (Requirement 4.2).
    let mut flags = String::new();
    let mut first = true;
    for flag in registry.iter() {
        let doc = flag.doc_short().replace("'", "''");

        let dash_name = format!("--{}", flag.name_long());
        let name = flag.name_long();
        push_flag(&mut flags, &mut first, &dash_name, name, &doc);

        if let Some(byte) = flag.name_short() {
            let dash_name = format!("-{}", char::from(byte));
            let name = char::from(byte).to_string();
            push_flag(&mut flags, &mut first, &dash_name, &name, &doc);
        }

        if let Some(negated) = flag.name_negated() {
            let dash_name = format!("--{negated}");
            push_flag(&mut flags, &mut first, &dash_name, negated, &doc);
        }

        for alias in flag.aliases() {
            let dash_name = format!("--{alias}");
            push_flag(&mut flags, &mut first, &dash_name, alias, &doc);
        }
    }

    // The per-flag value completion. The completion construct is driven solely
    // by the flag's `CompletionType` (Requirement 3): `Filename` uses
    // PowerShell's native file completion, `Executable` completes commands,
    // `Filetype` completes ripgrep file types, `Encoding` completes supported
    // text encodings, and declared choices are offered exactly and in declared
    // order. Flags that request no value (switches, or value flags with no
    // specific completion) contribute no value-completion branch.
    let mut values = String::new();
    for flag in registry.iter() {
        let Some(body) = value_completion_body(flag) else {
            continue;
        };

        // The value-completion branch applies to every name the flag can be
        // invoked as, so completing a value works no matter which name the
        // user typed (Requirements 4.1, 4.2).
        push_value_branch(
            &mut values,
            &format!("--{}", flag.name_long()),
            &body,
        );
        if let Some(byte) = flag.name_short() {
            let name = format!("-{}", char::from(byte));
            push_value_branch(&mut values, &name, &body);
        }
        if let Some(negated) = flag.name_negated() {
            push_value_branch(&mut values, &format!("--{negated}"), &body);
        }
        for alias in flag.aliases() {
            push_value_branch(&mut values, &format!("--{alias}"), &body);
        }
    }

    TEMPLATE
        .trim_start()
        .replace("!FLAGS!", &flags)
        .replace("!VALUES!", values.trim_end_matches('\n'))
}

/// Appends a single parameter-name `[CompletionResult]` to `flags`, handling
/// the leading indentation and the inter-entry newline so the emitted block is
/// byte-stable.
fn push_flag(
    flags: &mut String,
    first: &mut bool,
    dash_name: &str,
    name: &str,
    doc: &str,
) {
    if *first {
        *first = false;
    } else {
        flags.push('\n');
    }
    flags.push_str("      ");
    flags.push_str(
        &TEMPLATE_FLAG
            .replace("!DASH_NAME!", dash_name)
            .replace("!NAME!", name)
            .replace("!DOC!", doc),
    );
}

/// Appends a `switch` branch that completes the value of `dash_name` using the
/// PowerShell statements in `body`.
fn push_value_branch(values: &mut String, dash_name: &str, body: &str) {
    values.push_str("    '");
    values.push_str(dash_name);
    values.push_str("' {\n");
    values.push_str(body);
    values.push_str("      break\n");
    values.push_str("    }\n");
}

/// Returns the PowerShell statements that complete a value for `flag`, or
/// `None` when the flag requests no value.
///
/// This is the PowerShell realization of the completion-type mapping
/// (Requirement 3). The requirements' "Choices" classification is
/// `CompletionType::Other` with a non-empty `doc_choices`, and "None" is
/// `CompletionType::Other` with no choices (or a switch). Each branch produces
/// `[CompletionResult]` objects (or defers to a native PowerShell completer),
/// filtered against the word being completed.
fn value_completion_body(flag: &'static dyn Flag) -> Option<String> {
    match flag.completion_type() {
        CompletionType::Filename => Some(
            "      [System.Management.Automation.CompletionCompleters]::\
             CompleteFilename($wordToComplete)\n"
                .to_string(),
        ),
        CompletionType::Executable => Some(
            "      [System.Management.Automation.CompletionCompleters]::\
             CompleteCommand($wordToComplete)\n"
                .to_string(),
        ),
        CompletionType::Filetype => Some(
            "      rg --type-list | ForEach-Object {\n\
             \x20       $rgtype = ($_ -split ':', 2)[0].Trim()\n\
             \x20       if ($rgtype -like \"$wordToComplete*\") {\n\
             \x20         [CompletionResult]::new($rgtype, $rgtype, \
             [CompletionResultType]::ParameterValue, $rgtype)\n\
             \x20       }\n\
             \x20     }\n"
                .to_string(),
        ),
        CompletionType::Encoding => {
            Some(parameter_value_body(&encodings_list()))
        }
        CompletionType::Other => {
            if flag.doc_choices().is_empty() {
                // "None" completion: this flag requests no value.
                None
            } else {
                // Offer exactly the declared choices, in declared order
                // (Requirement 3.3). The list is preserved verbatim and never
                // sorted, so order is exactly as declared.
                let choices: Vec<String> =
                    flag.doc_choices().iter().map(|c| c.to_string()).collect();
                Some(parameter_value_body(&choices))
            }
        }
    }
}

/// Builds a PowerShell branch body that offers exactly `values` as parameter
/// values, in order, each filtered against the word being completed.
///
/// Order is preserved (PowerShell's `Where-Object` and `ForEach-Object` are
/// order-preserving and no sort is applied), satisfying the "exactly and in
/// declared order" requirement for choices (Requirement 3.3).
fn parameter_value_body(values: &[String]) -> String {
    let mut out = String::from("      @(\n");
    for value in values {
        // Single quotes in a PowerShell single-quoted string are escaped by
        // doubling them.
        let escaped = value.replace("'", "''");
        out.push_str("        '");
        out.push_str(&escaped);
        out.push_str("'\n");
    }
    out.push_str(
        "      ) | Where-Object { $_ -like \"$wordToComplete*\" } | \
         ForEach-Object {\n",
    );
    out.push_str(
        "        [CompletionResult]::new($_, $_, \
         [CompletionResultType]::ParameterValue, $_)\n",
    );
    out.push_str("      }\n");
    out
}

/// Builds the flat list of supported encodings for PowerShell value
/// completion.
///
/// The shared `encodings.sh` list is written using shell brace-expansion
/// patterns (which fish and zsh expand natively). PowerShell has no such
/// expansion, so the patterns are expanded here into the literal encoding
/// names. Duplicates are removed while preserving first-seen order.
fn encodings_list() -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for line in super::ENCODINGS.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        for token in line.split_whitespace() {
            for encoding in expand_braces(token) {
                if seen.insert(encoding.clone()) {
                    out.push(encoding);
                }
            }
        }
    }
    out
}

/// Expands bash-style brace patterns in `input` into the literal words they
/// denote.
///
/// Supports comma-separated alternatives (`{a,b,c}`), empty alternatives
/// (`{,-}`), adjacency (the cartesian concatenation of neighboring groups),
/// and nesting (`cp{819,125{0,1}}`). Numeric range sequences (`{1..5}`) are
/// not used by ripgrep's encoding list and are not supported.
fn expand_braces(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    parse_sequence(&chars, &mut pos, true)
}

/// Parses a concatenation of atoms, expanding each and producing the cartesian
/// product. When `top` is false, parsing stops at a top-level `,` or `}` so the
/// caller (a brace group) can consume that delimiter.
fn parse_sequence(chars: &[char], pos: &mut usize, top: bool) -> Vec<String> {
    let mut atoms: Vec<Vec<String>> = Vec::new();
    let mut literal = String::new();
    while *pos < chars.len() {
        let c = chars[*pos];
        if c == '{' {
            let start = *pos;
            *pos += 1;
            match parse_alternatives(chars, pos) {
                Some(group) => {
                    if !literal.is_empty() {
                        atoms.push(vec![std::mem::take(&mut literal)]);
                    }
                    atoms.push(group);
                    *pos += 1; // consume the closing '}'
                }
                None => {
                    // Not a well-formed brace group; treat '{' literally.
                    *pos = start + 1;
                    literal.push('{');
                }
            }
        } else if !top && (c == ',' || c == '}') {
            break;
        } else {
            literal.push(c);
            *pos += 1;
        }
    }
    if !literal.is_empty() {
        atoms.push(vec![literal]);
    }
    if atoms.is_empty() {
        return vec![String::new()];
    }
    let mut acc = vec![String::new()];
    for atom in atoms {
        let mut next = Vec::with_capacity(acc.len() * atom.len());
        for prefix in &acc {
            for suffix in &atom {
                next.push(format!("{prefix}{suffix}"));
            }
        }
        acc = next;
    }
    acc
}

/// Parses the comma-separated alternatives of a brace group, assuming the
/// opening `{` has already been consumed. On success, `pos` points at the
/// closing `}`. Returns `None` for an unterminated or malformed group.
fn parse_alternatives(chars: &[char], pos: &mut usize) -> Option<Vec<String>> {
    let mut group = Vec::new();
    loop {
        let alt = parse_sequence(chars, pos, false);
        group.extend(alt);
        if *pos >= chars.len() {
            return None;
        }
        match chars[*pos] {
            ',' => {
                *pos += 1;
            }
            '}' => return Some(group),
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::registry_tests::{SyntheticFlag, build_registry};
    use crate::flags::{Category, RegistryView};
    use proptest::prelude::*;

    // Feature: unified-flag-source, Property 6: Declared choices are offered
    // exactly and in order
    //
    // For any registry and for each of the four shell artifacts, a flag that
    // declares value choices has a completion entry that offers exactly those
    // choices and no others, in the order they are declared in the registry
    // (Requirement 3.3). A "choices" flag is `CompletionType::Other` with a
    // non-empty `doc_choices` (and is not a switch). The four generators are
    // exercised against synthetic registries via their `generate_with` seams.

    /// The shape of a single synthetic flag. `Choices` carries the number of
    /// declared value choices (2..=4); the other two are filler that produce a
    /// realistic mixed registry (a switch and a filename value flag) and are
    /// intentionally not asserted on here.
    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Choices(usize),
        FillerSwitch,
        FillerFilename,
    }

    fn any_kind() -> impl Strategy<Value = Kind> {
        prop_oneof![
            (2usize..=4).prop_map(Kind::Choices),
            Just(Kind::FillerSwitch),
            Just(Kind::FillerFilename),
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
    }

    fn raw_spec() -> impl Strategy<Value = RawSpec> {
        (any_kind(), any::<bool>(), any::<bool>(), 0usize..3).prop_map(
            |(kind, wants_short, wants_negated, num_aliases)| RawSpec {
                kind,
                wants_short,
                wants_negated,
                num_aliases,
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
    /// raw specs into a valid synthetic registry. Each choices flag is a
    /// non-switch value flag whose declared choices are distinct, ordered,
    /// alphanumeric tokens (`ch{i}o{j}`) so both the declared order and exact
    /// membership are checkable in every shell's output.
    fn normalize(raws: Vec<RawSpec>) -> Vec<SyntheticFlag> {
        let pool = short_pool();
        raws.into_iter()
            .enumerate()
            .map(|(i, raw)| {
                let long = format!("flag{i}long");
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
                    Kind::Choices(n) => (
                        false,
                        CompletionType::Other,
                        Some(format!("VAL{i}")),
                        (0..n).map(|j| format!("ch{i}o{j}")).collect(),
                    ),
                    Kind::FillerSwitch => {
                        (true, CompletionType::Other, None, Vec::new())
                    }
                    Kind::FillerFilename => (
                        false,
                        CompletionType::Filename,
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

    /// Strategy producing a valid, mixed synthetic registry that always has at
    /// least the opportunity to contain choices flags.
    fn registry_strategy() -> impl Strategy<Value = Vec<SyntheticFlag>> {
        prop::collection::vec(raw_spec(), 1..6).prop_map(normalize)
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

    /// Extract the ordered value choices offered for `--<long>` in the Bash
    /// artifact, by reading the `compgen -W "..."` word list from that flag's
    /// `case` body. The `--<long>)` label (with the trailing `)`) isolates the
    /// canonical entry from aliases and the negated name.
    fn bash_choices(out: &str, long: &str) -> Vec<String> {
        let blk = block(out, &format!("--{long})"), ";;")
            .expect("bash must contain a case for the choices flag");
        let after = blk
            .split_once("compgen -W \"")
            .expect("bash choices case must use compgen -W")
            .1;
        let words = after
            .split_once('"')
            .expect("compgen -W word list must be quoted")
            .0;
        words.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Extract the ordered value choices offered for `--<long>` in the Zsh
    /// artifact, by reading the `:(...)` group on that flag's `_arguments`
    /// spec line. The `=` suffix on the long name disambiguates the canonical
    /// entry from aliases and the (value-less) negated entry.
    fn zsh_choices(out: &str, long: &str) -> Vec<String> {
        let needle = format!("--{long}=");
        let line = out
            .lines()
            .find(|l| l.contains(&needle))
            .expect("zsh must contain a spec for the choices flag");
        let after =
            line.split_once(":(").expect("zsh choices spec must use :(...)").1;
        let inner =
            after.split_once(')').expect("zsh choices group must close").0;
        inner.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Extract the ordered value choices offered for `--<long>` in the Fish
    /// artifact, by reading the `-a '...'` list on that flag's `complete`
    /// entry. The ` -l <long> ` token (with the trailing space) isolates the
    /// canonical entry from aliases and the negated name.
    fn fish_choices(out: &str, long: &str) -> Vec<String> {
        let blk = block(out, &format!(" -l {long} "), "\ncomplete ")
            .expect("fish must contain a completion for the choices flag");
        let after = blk
            .split_once(" -a '")
            .expect("fish choices entry must use -a '...'")
            .1;
        let inner =
            after.split_once('\'').expect("fish choices list must close").0;
        inner.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Extract the ordered value choices offered for `--<long>` in the
    /// PowerShell artifact, by reading the quoted tokens of the `@(...)` block
    /// in that flag's value-completion `switch` branch. Only lines that are a
    /// single quoted token are taken, so the surrounding pipeline statements
    /// are ignored.
    fn powershell_choices(out: &str, long: &str) -> Vec<String> {
        let blk = block(out, &format!("'--{long}' {{"), "break")
            .expect("powershell must contain a value branch for the flag");
        let arr = block(blk, "@(", ")")
            .expect("powershell choices branch must use an @(...) block");
        arr.lines()
            .map(str::trim)
            .filter(|l| {
                l.len() >= 2 && l.starts_with('\'') && l.ends_with('\'')
            })
            .map(|l| l.trim_matches('\'').to_string())
            .collect()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn declared_choices_offered_exactly_and_in_order(
            flags in registry_strategy()
        ) {
            // Remember each choices flag's long name and declared choices
            // before the registry is consumed by `build_registry`. A choices
            // flag is `CompletionType::Other` with a non-empty choices list.
            let choices_flags: Vec<(String, Vec<String>)> = flags
                .iter()
                .filter(|f| {
                    matches!(f.completion, CompletionType::Other)
                        && !f.choices.is_empty()
                })
                .map(|f| (f.long.clone(), f.choices.clone()))
                .collect();

            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            let bash = crate::flags::complete::bash::generate_with(&view);
            let zsh = crate::flags::complete::zsh::generate_with(&view);
            let fish = crate::flags::complete::fish::generate_with(&view);
            let powershell = generate_with(&view);

            for (long, expected) in choices_flags {
                // Each shell must offer exactly the declared choices, in the
                // declared order: no extras, none missing, no reordering.
                let b = bash_choices(&bash, &long);
                prop_assert_eq!(
                    &b, &expected,
                    "bash choices for --{} are not exactly the declared \
                     choices in order",
                    long
                );

                let z = zsh_choices(&zsh, &long);
                prop_assert_eq!(
                    &z, &expected,
                    "zsh choices for --{} are not exactly the declared \
                     choices in order",
                    long
                );

                let f = fish_choices(&fish, &long);
                prop_assert_eq!(
                    &f, &expected,
                    "fish choices for --{} are not exactly the declared \
                     choices in order",
                    long
                );

                let p = powershell_choices(&powershell, &long);
                prop_assert_eq!(
                    &p, &expected,
                    "powershell choices for --{} are not exactly the \
                     declared choices in order",
                    long
                );
            }
        }
    }

    // Feature: unified-flag-source, Property 9: Aliases are completable in
    // every shell
    //
    // For any registry and for each of the four shell artifacts, every alias
    // of every flag is offered as a completable flag (Requirement 4.2). The
    // four generators are exercised against synthetic registries via their
    // `generate_with` seams. Each shell surfaces an alias `<alias>` as a
    // completable flag in its own idiom:
    //   * Bash: `--<alias>` appears in the `opts="..."` word list.
    //   * Zsh: a `'--<alias>...[...]'` spec element (value flags are suffixed
    //     with `=`, switches go straight into the `[` description bracket).
    //   * Fish: a `complete -c rg -l <alias> ...` line.
    //   * PowerShell: a `[CompletionResult]::new('--<alias>', ...)` entry.

    /// The whitespace-separated `opts="..."` word list from the Bash artifact.
    /// Every completable flag name (long, short, negated, alias) is emitted
    /// here, so membership of `--<alias>` proves the alias is completable.
    fn bash_opts(out: &str) -> Vec<String> {
        // The template initializes `opts=""` before assigning the populated
        // list, so take the last `opts="` assignment (the populated one).
        let i = out.rfind("opts=\"").expect("bash must define an opts list");
        let after = &out[i + "opts=\"".len()..];
        let list =
            after.split_once('"').expect("bash opts list must be quoted").0;
        list.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Whether the Zsh artifact offers `--<alias>` as a completable spec
    /// element. An alias element is `--<alias>=[...]` for value flags or
    /// `--<alias>[...]` for switches; either form proves completability.
    fn zsh_offers_alias(out: &str, alias: &str) -> bool {
        out.contains(&format!("--{alias}="))
            || out.contains(&format!("--{alias}["))
    }

    /// Whether the Fish artifact offers `<alias>` as a completable flag via a
    /// `-l <alias>` token on a `complete` line.
    fn fish_offers_alias(out: &str, alias: &str) -> bool {
        out.contains(&format!(" -l {alias} "))
            || out.contains(&format!(" -l {alias}\n"))
    }

    /// Strategy producing a valid, mixed synthetic registry in which at least
    /// one flag is guaranteed to carry an alias, so the property is never
    /// vacuously satisfied.
    fn registry_with_aliases_strategy()
    -> impl Strategy<Value = Vec<SyntheticFlag>> {
        prop::collection::vec(raw_spec(), 1..6).prop_map(|mut raws| {
            if raws.iter().all(|r| r.num_aliases == 0) {
                raws[0].num_aliases = 1;
            }
            normalize(raws)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn aliases_are_completable_in_every_shell(
            flags in registry_with_aliases_strategy()
        ) {
            // Collect every alias across the registry before it is consumed by
            // `build_registry`.
            let aliases: Vec<String> = flags
                .iter()
                .flat_map(|f| f.aliases.iter().cloned())
                .collect();

            // The strategy guarantees at least one alias, so the property is
            // exercised on real data rather than passing vacuously.
            prop_assert!(
                !aliases.is_empty(),
                "registry strategy must produce at least one alias"
            );

            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            let bash = crate::flags::complete::bash::generate_with(&view);
            let zsh = crate::flags::complete::zsh::generate_with(&view);
            let fish = crate::flags::complete::fish::generate_with(&view);
            let powershell = generate_with(&view);

            let bash_opts = bash_opts(&bash);

            for alias in aliases {
                let dash = format!("--{alias}");

                // Bash: the alias must appear in the opts word list.
                prop_assert!(
                    bash_opts.iter().any(|o| o == &dash),
                    "bash opts list does not offer alias {} as a \
                     completable flag",
                    dash
                );

                // Zsh: the alias must appear as its own spec element.
                prop_assert!(
                    zsh_offers_alias(&zsh, &alias),
                    "zsh does not offer alias {} as a completable flag",
                    dash
                );

                // Fish: the alias must appear via a `-l <alias>` token.
                prop_assert!(
                    fish_offers_alias(&fish, &alias),
                    "fish does not offer alias {} as a completable flag \
                     (-l {})",
                    dash,
                    alias
                );

                // PowerShell: the alias must appear as a `[CompletionResult]`
                // parameter-name entry.
                prop_assert!(
                    powershell.contains(&format!("'{dash}'")),
                    "powershell does not offer alias {} as a completable \
                     flag",
                    dash
                );
            }
        }
    }
}

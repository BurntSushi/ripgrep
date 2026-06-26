/*!
Provides completions for ripgrep's CLI for the bash shell.
*/

use crate::flags::{CompletionType, Flag, RegistryView};

const TEMPLATE_FULL: &'static str = "
_rg() {
  local i cur prev opts cmds
  COMPREPLY=()
  cur=\"${COMP_WORDS[COMP_CWORD]}\"
  prev=\"${COMP_WORDS[COMP_CWORD-1]}\"
  cmd=\"\"
  opts=\"\"

  for i in ${COMP_WORDS[@]}; do
    case \"${i}\" in
      rg)
        cmd=\"rg\"
        ;;
      *)
        ;;
    esac
  done

  case \"${cmd}\" in
    rg)
      opts=\"!OPTS!\"
      if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
        COMPREPLY=($(compgen -W \"${opts}\" -- \"${cur}\"))
        return 0
      fi
      case \"${prev}\" in
!CASES!
      esac
      COMPREPLY=($(compgen -W \"${opts}\" -- \"${cur}\"))
      return 0
      ;;
  esac
}

complete -F _rg -o bashdefault -o default rg
";

/// Completes a flag's value with a file path (`CompletionType::Filename`).
const TEMPLATE_CASE: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -f \"${cur}\"))
          return 0
          ;;
";

/// Completes a flag's value with one of a fixed set of declared choices
/// (`CompletionType::Other` plus a non-empty `doc_choices`).
const TEMPLATE_CASE_CHOICES: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -W \"!CHOICES!\" -- \"${cur}\"))
          return 0
          ;;
";

/// Completes a flag's value with a command in `$PATH`
/// (`CompletionType::Executable`).
const TEMPLATE_CASE_EXECUTABLE: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -c -- \"${cur}\"))
          return 0
          ;;
";

/// Completes a flag's value with a ripgrep file type
/// (`CompletionType::Filetype`).
const TEMPLATE_CASE_FILETYPE: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -W \"$(rg --type-list | cut -d ':' -f 1)\" -- \"${cur}\"))
          return 0
          ;;
";

/// Completes a flag's value with a supported text encoding
/// (`CompletionType::Encoding`).
const TEMPLATE_CASE_ENCODING: &'static str = "
        !FLAG!)
          COMPREPLY=($(compgen -W \"!ENCODINGS!\" -- \"${cur}\"))
          return 0
          ;;
";

/// Generate completions for Bash.
///
/// Note that these completions are based on what was produced for ripgrep <=13
/// using Clap 2.x. Improvements on this are welcome.
///
/// All flag-specific content is derived from the canonical flag registry
/// (Requirement 1.2). The registry is validated once per generation; on
/// failure the real registry is known-valid, so validation must succeed here.
pub(crate) fn generate() -> String {
    generate_with(
        &RegistryView::load()
            .expect("ripgrep's flag registry should validate"),
    )
}

/// Generate completions for Bash from the given (already validated) registry
/// view.
///
/// This is the registry-accepting seam behind [`generate`]: `generate` loads
/// ripgrep's canonical registry and delegates here, while tests pass synthetic
/// registries to exercise the generator across many inputs.
pub(crate) fn generate_with(registry: &RegistryView) -> String {
    // The set of completable flags offered after `-` or as the first word.
    // Every name a flag can be invoked as is offered: its long name, its short
    // name, its negated name (Requirement 4.1), and every alias
    // (Requirement 4.2).
    let mut opts = String::new();
    for flag in registry.iter() {
        opts.push_str("--");
        opts.push_str(flag.name_long());
        opts.push(' ');
        if let Some(short) = flag.name_short() {
            opts.push('-');
            opts.push(char::from(short));
            opts.push(' ');
        }
        if let Some(name) = flag.name_negated() {
            opts.push_str("--");
            opts.push_str(name);
            opts.push(' ');
        }
        for alias in flag.aliases() {
            opts.push_str("--");
            opts.push_str(alias);
            opts.push(' ');
        }
    }
    opts.push_str("<PATTERN> <PATH>...");

    // The per-flag value completion. The completion construct is driven solely
    // by the flag's `CompletionType` (Requirement 3): `Filename` uses Bash's
    // native file completion, `Executable` completes commands in `$PATH`,
    // `Filetype` completes ripgrep file types, `Encoding` completes supported
    // text encodings, and declared choices are offered exactly and in declared
    // order. Flags that request no value (switches, or value flags with no
    // specific completion) emit no case at all.
    let mut cases = String::new();
    for flag in registry.iter() {
        let Some(template) = value_completion_template(flag) else {
            // Explicit `None` completion semantics: this flag requests no
            // value, so it gets no value-completion case.
            continue;
        };

        // The value-completion case applies to every name the flag can be
        // invoked as, so completing a value works no matter which name the
        // user typed (Requirements 4.1, 4.2).
        let name = format!("--{}", flag.name_long());
        cases.push_str(&template.replace("!FLAG!", &name));
        if let Some(short) = flag.name_short() {
            let name = format!("-{}", char::from(short));
            cases.push_str(&template.replace("!FLAG!", &name));
        }
        if let Some(negated) = flag.name_negated() {
            let name = format!("--{negated}");
            cases.push_str(&template.replace("!FLAG!", &name));
        }
        for alias in flag.aliases() {
            let name = format!("--{alias}");
            cases.push_str(&template.replace("!FLAG!", &name));
        }
    }

    TEMPLATE_FULL
        .replace("!OPTS!", &opts)
        .replace("!CASES!", &cases)
        .trim_start()
        .to_string()
}

/// Returns the Bash value-completion `case` body for `flag`, with `!FLAG!`
/// still to be substituted, or `None` when the flag requests no value.
///
/// This is the Bash realization of the completion-type mapping (Requirement
/// 3). The requirements' "Choices" classification is `CompletionType::Other`
/// with a non-empty `doc_choices`, and "None" is `CompletionType::Other` with
/// no choices (or a switch).
fn value_completion_template(flag: &'static dyn Flag) -> Option<String> {
    match flag.completion_type() {
        CompletionType::Filename => Some(TEMPLATE_CASE.trim_end().to_string()),
        CompletionType::Executable => {
            Some(TEMPLATE_CASE_EXECUTABLE.trim_end().to_string())
        }
        CompletionType::Filetype => {
            Some(TEMPLATE_CASE_FILETYPE.trim_end().to_string())
        }
        CompletionType::Encoding => Some(
            TEMPLATE_CASE_ENCODING
                .trim_end()
                .replace("!ENCODINGS!", &encodings_wordlist()),
        ),
        CompletionType::Other => {
            if flag.doc_choices().is_empty() {
                // "None" completion: request no value.
                None
            } else {
                // Offer exactly the declared choices, in declared order
                // (Requirement 3.3).
                let choices = flag.doc_choices().join(" ");
                Some(
                    TEMPLATE_CASE_CHOICES
                        .trim_end()
                        .replace("!CHOICES!", &choices),
                )
            }
        }
    }
}

/// Builds a single-line, space-separated word list of supported encodings
/// suitable for `compgen -W`.
///
/// The shared `encodings.sh` list contains comments and brace-expansion
/// patterns; comments are stripped here and the remaining words are joined so
/// Bash's brace expansion within `compgen -W` enumerates the encodings.
fn encodings_wordlist() -> String {
    super::ENCODINGS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<&str>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::registry_tests::{SyntheticFlag, build_registry};
    use crate::flags::{Category, RegistryView};
    use proptest::prelude::*;

    // Feature: unified-flag-source, Property 5: Completion-type maps to the
    // right construct in every shell
    //
    // For any registry, and for each of the Bash, Zsh, Fish and PowerShell
    // artifacts, every flag whose Completion_Type is Filename, Executable,
    // Filetype or Encoding has a completion entry that completes its value
    // using that shell's corresponding native construct. The four generators
    // are exercised against synthetic registries via their `generate_with`
    // seams.

    /// The completion kinds a synthetic flag can take. The first four are the
    /// ones Property 5 makes assertions about; the last two are filler that
    /// produce a realistic mixed registry (a switch, and a value flag carrying
    /// declared choices) and are intentionally not asserted on here.
    #[derive(Clone, Copy, Debug)]
    enum Kind {
        Filename,
        Executable,
        Filetype,
        Encoding,
        OtherSwitch,
        OtherChoices,
    }

    fn any_kind() -> impl Strategy<Value = Kind> {
        prop_oneof![
            Just(Kind::Filename),
            Just(Kind::Executable),
            Just(Kind::Filetype),
            Just(Kind::Encoding),
            Just(Kind::OtherSwitch),
            Just(Kind::OtherChoices),
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
    /// raw specs into a valid synthetic registry. Flags carrying a value
    /// completion type are non-switches with a value variable (the only shape
    /// for which value completion is meaningful); the filler switch carries no
    /// variable.
    fn normalize(raws: Vec<RawSpec>) -> Vec<SyntheticFlag> {
        let pool = short_pool();
        raws.into_iter()
            .enumerate()
            .map(|(i, raw)| {
                // Some long names are deliberately hyphen-rich to exercise the
                // shells' handling of hyphenated names.
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
                    Kind::Filename => (
                        false,
                        CompletionType::Filename,
                        Some(format!("VAL{i}")),
                        Vec::new(),
                    ),
                    Kind::Executable => (
                        false,
                        CompletionType::Executable,
                        Some(format!("VAL{i}")),
                        Vec::new(),
                    ),
                    Kind::Filetype => (
                        false,
                        CompletionType::Filetype,
                        Some(format!("VAL{i}")),
                        Vec::new(),
                    ),
                    Kind::Encoding => (
                        false,
                        CompletionType::Encoding,
                        Some(format!("VAL{i}")),
                        Vec::new(),
                    ),
                    Kind::OtherSwitch => {
                        (true, CompletionType::Other, None, Vec::new())
                    }
                    Kind::OtherChoices => (
                        false,
                        CompletionType::Other,
                        Some(format!("VAL{i}")),
                        vec![format!("c{i}a"), format!("c{i}b")],
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
    /// up to the start of the next `complete` entry. A substring slice is used
    /// rather than a single physical line because the encoding value list
    /// embeds newlines.
    fn fish_block<'a>(out: &'a str, long: &str) -> Option<&'a str> {
        block(out, &format!(" -l {long} "), "\ncomplete ")
    }

    /// Find the single Zsh `_arguments` spec line for the value flag `long`.
    /// Value flags carry a `=` suffix on the long name, which disambiguates the
    /// canonical entry from aliases and the (value-less) negated entry.
    fn zsh_line<'a>(out: &'a str, long: &str) -> Option<&'a str> {
        let needle = format!("--{long}=");
        out.lines().find(|l| l.contains(&needle))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn completion_type_maps_to_native_construct(
            flags in registry_strategy()
        ) {
            // Remember each flag's long name and completion type before the
            // registry is consumed by `build_registry`.
            let infos: Vec<(String, CompletionType)> = flags
                .iter()
                .map(|f| (f.long.clone(), f.completion))
                .collect();

            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            let bash = generate_with(&view);
            let zsh = crate::flags::complete::zsh::generate_with(&view);
            let fish = crate::flags::complete::fish::generate_with(&view);
            let powershell =
                crate::flags::complete::powershell::generate_with(&view);

            for (long, completion) in infos {
                // The native construct each shell must use for this completion
                // type. A `None` entry means the type is not covered by this
                // property (the filler kinds), so it is skipped.
                let markers = match completion {
                    CompletionType::Filename => Some((
                        "compgen -f",
                        "_files",
                        " -r -F",
                        "CompleteFilename",
                    )),
                    CompletionType::Executable => Some((
                        "compgen -c",
                        "_command_names",
                        "__fish_complete_command",
                        "CompleteCommand",
                    )),
                    CompletionType::Filetype => Some((
                        "rg --type-list",
                        "_rg_types",
                        "rg --type-list",
                        "rg --type-list",
                    )),
                    CompletionType::Encoding => Some((
                        // The encoding list is identical across the
                        // shell-native list constructs; `x-user-defined` is a
                        // stable literal member of it. Zsh defers to a helper.
                        "x-user-defined",
                        "_rg_encodings",
                        "x-user-defined",
                        "x-user-defined",
                    )),
                    CompletionType::Other => None,
                };
                let Some((bash_m, zsh_m, fish_m, ps_m)) = markers else {
                    continue;
                };

                // Bash: the per-flag `case` body for `--<long>)`.
                let bash_block = block(&bash, &format!("--{long})"), ";;")
                    .expect("bash must contain a case for the flag");
                prop_assert!(
                    bash_block.contains(bash_m),
                    "bash entry for --{long} ({completion:?}) missing \
                     {bash_m:?}: {bash_block}"
                );

                // Zsh: the `_arguments` spec line for `--<long>=`.
                let zsh_block = zsh_line(&zsh, &long)
                    .expect("zsh must contain a spec for the flag");
                prop_assert!(
                    zsh_block.contains(zsh_m),
                    "zsh entry for --{long} ({completion:?}) missing \
                     {zsh_m:?}: {zsh_block}"
                );

                // Fish: the `complete` entry for `-l <long>`.
                let fish_block = fish_block(&fish, &long)
                    .expect("fish must contain a completion for the flag");
                prop_assert!(
                    fish_block.contains(fish_m),
                    "fish entry for --{long} ({completion:?}) missing \
                     {fish_m:?}: {fish_block}"
                );

                // PowerShell: the value-completion `switch` branch for
                // `'--<long>'`.
                let ps_block =
                    block(&powershell, &format!("'--{long}' {{"), "break")
                        .expect("powershell must contain a value branch");
                prop_assert!(
                    ps_block.contains(ps_m),
                    "powershell entry for --{long} ({completion:?}) missing \
                     {ps_m:?}: {ps_block}"
                );
            }
        }
    }

    /// Strategy producing a valid, mixed synthetic registry in which at least
    /// one flag is guaranteed to have a negated name. Property 8 is only
    /// meaningful when a negated name is present, so the first flag's
    /// `wants_negated` is forced on; the remaining flags keep their randomly
    /// generated negation (and other) attributes for a realistic mix.
    fn registry_with_negated() -> impl Strategy<Value = Vec<SyntheticFlag>> {
        prop::collection::vec(raw_spec(), 1..8).prop_map(|mut raws| {
            raws[0].wants_negated = true;
            normalize(raws)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        // Feature: unified-flag-source, Property 8: Negated names are
        // completable in every shell
        //
        // For any registry and for each of the Bash, Zsh, Fish and PowerShell
        // artifacts, every Flag_Definition that has a negated name has a
        // completion entry offering that negated name as a completable flag.
        // The four generators are exercised against synthetic registries (with
        // at least one negated name guaranteed) via their `generate_with`
        // seams.
        //
        // Validates: Requirements 2.3, 4.1
        #[test]
        fn negated_names_completable_in_every_shell(
            flags in registry_with_negated()
        ) {
            // Remember each flag's negated name (when it has one) before the
            // owned flags are consumed by `build_registry`.
            let negated_names: Vec<String> = flags
                .iter()
                .filter_map(|f| f.negated.clone())
                .collect();
            // The strategy guarantees the property has something to assert on.
            prop_assert!(
                !negated_names.is_empty(),
                "registry strategy must produce at least one negated name"
            );

            let reg = build_registry(flags);
            let view = RegistryView::new(reg)
                .expect("synthetic registry must validate");

            let bash = generate_with(&view);
            let zsh = crate::flags::complete::zsh::generate_with(&view);
            let fish = crate::flags::complete::fish::generate_with(&view);
            let powershell =
                crate::flags::complete::powershell::generate_with(&view);

            for negated in negated_names {
                // Bash: the negated name appears in the `opts` word list as a
                // completable `--<negated>` token (trailing space separates
                // entries in the list).
                prop_assert!(
                    bash.contains(&format!("--{negated} ")),
                    "bash opts list must offer --{negated} as a completable \
                     flag"
                );

                // Zsh: the negated name is emitted as its own `$no`-prefixed
                // `_arguments` spec, `$no'--<negated>[...]'`. Single quoting is
                // used unless the description contains an apostrophe, in which
                // case double quotes are used; both are accepted here.
                let zsh_sq = format!("$no'--{negated}[");
                let zsh_dq = format!("$no\"--{negated}[");
                prop_assert!(
                    zsh.contains(&zsh_sq) || zsh.contains(&zsh_dq),
                    "zsh must offer --{negated} as a completable (negated) \
                     flag spec"
                );

                // Fish: the negated name is offered as a separate `complete`
                // entry, `complete -c rg -l <negated> ...` (trailing space
                // disambiguates it from longer names).
                prop_assert!(
                    fish.contains(&format!(" -l {negated} ")),
                    "fish must offer -l {negated} as a completable flag"
                );

                // PowerShell: the negated name is offered as a parameter-name
                // `[CompletionResult]::new('--<negated>', ...)` entry.
                prop_assert!(
                    powershell.contains(&format!("'--{negated}'")),
                    "powershell must offer --{negated} as a completable flag"
                );
            }
        }
    }
}

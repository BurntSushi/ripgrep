# Requirements Document

## Introduction

ripgrep exposes a large command line interface whose flags are surfaced in
multiple downstream artifacts: shell completion scripts for Bash, Zsh, Fish,
and PowerShell; the `rg.1` man page; and the `-h`/`--help` output. Today most
of these artifacts are generated from a shared in-code flag model (the `Flag`
trait and the `FLAGS` slice), but the Zsh completion script (`rg.zsh`) is
maintained by hand and only loosely guarded by a CI script that checks for the
presence or absence of flag names. This split, together with the absence of
automated checks that the other generated artifacts faithfully reflect the flag
model, has historically caused drift and a recurring class of bugs (for
example, hyphen escaping in the man page, and per-shell divergence in
`--hyperlink-format` completion behavior).

This feature establishes a single canonical source of truth for every piece of
flag metadata and requires that all shell completions, the man page, and the
help output be derived from that source. It also adds automated consistency
validation so that any divergence between the source of truth and a generated
artifact is detected before release, eliminating the manual synchronization
work and the bugs it produces.

The scope of this feature is the generation and validation pipeline for flag
metadata. It does not change ripgrep's search behavior, nor does it change the
set of flags ripgrep accepts.

## Glossary

- **rg**: The ripgrep command line binary.
- **Flag_Registry**: The single canonical in-code collection of all flag
  definitions for ripgrep. This is the single source of truth for flag
  metadata.
- **Flag_Definition**: One entry in the Flag_Registry describing a single
  logical flag, including its long name, optional short name, optional negated
  name, aliases, switch-or-value nature, value variable name, allowed value
  choices, completion type, category, short documentation, and long
  documentation.
- **Flag_Metadata**: Any individual attribute of a Flag_Definition (for
  example, the long name, the short documentation, or the completion type).
- **Completion_Type**: The classification of how a flag's value should be
  completed by a shell. One of: Filename, Executable, Filetype, Encoding,
  Choices, or None.
- **Generator**: Any subsystem that consumes the Flag_Registry and produces an
  artifact. The Generators are the Bash_Generator, Zsh_Generator,
  Fish_Generator, PowerShell_Generator, Man_Generator, and Help_Generator.
- **Bash_Generator**: The subsystem that produces the Bash completion script.
- **Zsh_Generator**: The subsystem that produces the Zsh completion script.
- **Fish_Generator**: The subsystem that produces the Fish completion script.
- **PowerShell_Generator**: The subsystem that produces the PowerShell
  completion script.
- **Man_Generator**: The subsystem that produces the `rg.1` man page in roff
  format.
- **Help_Generator**: The subsystem that produces the `-h` (short) and
  `--help` (long) output.
- **Markup_Renderer**: The subsystem that resolves custom documentation markup
  tags (such as `\flag{...}` and `\flag-negate{...}`) embedded in flag
  documentation strings into artifact-appropriate text.
- **Consistency_Checker**: The automated test subsystem that verifies every
  Generator's output reflects the Flag_Registry and reports any divergence.
- **Generate_Command**: The `rg --generate <mode>` command line interface that
  invokes a Generator and writes its artifact to standard output.
- **Artifact**: A generated output document (a completion script, the man page,
  or the help output).

## Requirements

### Requirement 1: Single Source of Truth for Flag Metadata

**User Story:** As a ripgrep maintainer, I want all flag metadata defined in one
canonical place, so that I can add or change a flag exactly once and have every
artifact reflect the change.

#### Acceptance Criteria

1. THE Flag_Registry SHALL define, for each Flag_Definition, the following
   mandatory fields: the long name, the switch-or-value nature, the
   Completion_Type, the category, the short documentation, and the long
   documentation; and SHALL define the following optional fields, populated
   only when applicable to that Flag_Definition: the short name, the negated
   name, the aliases, the value variable name, and the value choices.
2. THE Bash_Generator SHALL derive all flag-specific content of its Artifact
   solely from Flag_Registry entries and SHALL contain no flag-specific values
   that are hardcoded outside the Flag_Registry.
3. THE Zsh_Generator SHALL derive all flag-specific content of its Artifact
   solely from Flag_Registry entries and SHALL contain no flag-specific values
   that are hardcoded outside the Flag_Registry.
4. THE Fish_Generator SHALL derive all flag-specific content of its Artifact
   solely from Flag_Registry entries and SHALL contain no flag-specific values
   that are hardcoded outside the Flag_Registry.
5. THE PowerShell_Generator SHALL derive all flag-specific content of its
   Artifact solely from Flag_Registry entries and SHALL contain no
   flag-specific values that are hardcoded outside the Flag_Registry.
6. THE Man_Generator SHALL derive all flag-specific content of its Artifact
   solely from Flag_Registry entries and SHALL contain no flag-specific values
   that are hardcoded outside the Flag_Registry.
7. THE Help_Generator SHALL derive all flag-specific content of its Artifact
   solely from Flag_Registry entries and SHALL contain no flag-specific values
   that are hardcoded outside the Flag_Registry.
8. WHEN a Flag_Definition is added, modified, or removed in the Flag_Registry
   and generation is next run, THE flag-specific content of every Generator's
   Artifact SHALL change to reflect that single edit, with no edit to any
   Generator required.
9. IF two or more Flag_Definitions share the same long name, or share the same
   short name, or share the same negated name, THEN THE Flag_Registry SHALL
   fail validation with an error indicating the conflicting field and value,
   and no Artifact SHALL be produced.
10. IF a Flag_Definition is missing any mandatory field defined in criterion 1,
    THEN THE Flag_Registry SHALL fail validation with an error identifying the
    affected Flag_Definition and the missing field, AND each Generator SHALL
    operate on the available Flag_Metadata of that Flag_Definition without
    halting.
11. THE Flag_Registry SHALL fail validation only for the conditions defined in
    criteria 9 and 10 (duplicate long, short, or negated names, and missing
    mandatory fields), and SHALL NOT reject a Flag_Definition for any other
    condition such as field format, value constraints, or cross-field
    dependencies.

### Requirement 2: Zsh Completion Derived From the Source of Truth

**User Story:** As a ripgrep maintainer, I want the Zsh completion's per-flag
content generated from the single source of truth, so that the Zsh completion
no longer drifts from the actual set of flags and their metadata.

#### Acceptance Criteria

1. THE Zsh_Generator SHALL produce exactly one completion entry for every
   Flag_Definition in the Flag_Registry, and that entry SHALL include the long
   name of the Flag_Definition.
2. WHERE a Flag_Definition has a short name, THE Zsh_Generator SHALL include
   that short name in the Flag_Definition's completion entry as a separately
   completable option, exactly as defined in the Flag_Registry.
3. WHERE a Flag_Definition has a negated name, THE Zsh_Generator SHALL include
   that negated name in the Flag_Definition's completion entry as a separately
   completable option, exactly as defined in the Flag_Registry.
4. THE Zsh_Generator SHALL use, as the description text for a flag's completion
   entry, text that is character-for-character identical to the short
   documentation of that Flag_Definition.
5. IF a Flag_Definition has empty short documentation, THEN THE Zsh_Generator
   SHALL produce the completion entry for that Flag_Definition with an empty
   description text.
6. WHERE the Zsh completion applies shell-specific contextual behavior such as
   flag compatibility grouping, THE Zsh_Generator SHALL apply that behavior
   while keeping the flag names, description text, and value choices
   character-for-character identical to those taken from the Flag_Registry.

### Requirement 3: Consistent Value Completion Behavior Across Shells

**User Story:** As a ripgrep user, I want a flag's value completion to behave
the same way across shells, so that the suggestions I receive match the flag's
accepted values regardless of which shell I use.

#### Acceptance Criteria

1. WHERE a Flag_Definition has a Completion_Type of Filename, THE
   Bash_Generator, THE Zsh_Generator, THE Fish_Generator, and THE
   PowerShell_Generator SHALL each produce a completion entry that completes
   that flag's value using the shell's native file path completion.
2. WHERE a Flag_Definition has a Completion_Type of Executable, THE
   Bash_Generator, THE Zsh_Generator, THE Fish_Generator, and THE
   PowerShell_Generator SHALL each produce a completion entry that completes
   that flag's value as a command available to the shell.
3. WHERE a Flag_Definition declares value choices, THE Bash_Generator, THE
   Zsh_Generator, THE Fish_Generator, and THE PowerShell_Generator SHALL each
   produce a completion entry that offers exactly those value choices and no
   other values, in the order those choices are declared in the Flag_Registry.
4. WHERE a Flag_Definition has a Completion_Type of Filetype, THE
   Bash_Generator, THE Zsh_Generator, THE Fish_Generator, and THE
   PowerShell_Generator SHALL each produce a completion entry that completes
   that flag's value as a ripgrep file type.
5. WHERE a Flag_Definition has a Completion_Type of Encoding, THE
   Bash_Generator, THE Zsh_Generator, THE Fish_Generator, and THE
   PowerShell_Generator SHALL each produce a completion entry that completes
   that flag's value as a supported text encoding.
6. WHERE a Flag_Definition is a switch, THE Fish_Generator SHALL produce a
   completion entry that requests no value for that flag.

### Requirement 4: Consistent Treatment of Negated Names and Aliases

**User Story:** As a ripgrep user, I want negated flag names and aliases to be
completable and documented, so that every name I can type is discoverable.

#### Acceptance Criteria

1. WHERE a Flag_Definition has a negated name, THE Bash_Generator, THE
   Zsh_Generator, THE Fish_Generator, and THE PowerShell_Generator SHALL each
   produce a completion entry that offers the negated name as a completable
   flag.
2. WHERE a Flag_Definition has one or more aliases, THE Bash_Generator, THE
   Zsh_Generator, THE Fish_Generator, and THE PowerShell_Generator SHALL each
   produce a completion entry that offers every alias as a completable flag.
3. WHERE a Flag_Definition is a switch and has a negated name, THE
   Man_Generator SHALL state in that flag's documentation that the flag can be
   disabled, showing the negated name verbatim.
4. WHERE a Flag_Definition is a switch and has a negated name, THE
   Help_Generator SHALL state in the long help that the flag can be disabled,
   showing the negated name verbatim.

### Requirement 5: Centralized Documentation Markup Rendering

**User Story:** As a ripgrep maintainer, I want documentation markup and
escaping handled in one place, so that fixing an escaping defect once corrects
every artifact and prevents recurrence of bugs like unescaped hyphens in the
man page.

#### Acceptance Criteria

1. WHEN a flag documentation string contains one or more `\flag{name}` markup
   tags, THE Markup_Renderer SHALL replace each occurrence with a
   cross-reference that contains the long name of the flag identified by
   `name` as resolved through the Flag_Registry.
2. WHEN a flag documentation string contains one or more `\flag-negate{name}`
   markup tags, THE Markup_Renderer SHALL replace each occurrence with a
   cross-reference that contains the negated name of the flag identified by
   `name` as resolved through the Flag_Registry.
3. IF a flag documentation string contains a markup tag that references a name
   absent from the Flag_Registry, THEN THE Markup_Renderer SHALL report an
   error identifying the unresolved name and the offending tag, and no Artifact
   SHALL be produced.
4. IF a flag documentation string contains a `\flag-negate{name}` markup tag
   that references a Flag_Definition that has no negated name, THEN THE
   Markup_Renderer SHALL report an error identifying the flag and the offending
   tag, and no Artifact SHALL be produced.
5. IF a flag documentation string contains an unrecognized or malformed markup
   tag, THEN THE Markup_Renderer SHALL report an error identifying the
   offending tag, and no Artifact SHALL be produced.
6. WHEN the Man_Generator emits a flag name into roff output, THE Man_Generator
   SHALL escape each hyphen character in that name so the rendered man page
   displays a literal hyphen for each.

### Requirement 6: Automated Consistency Validation

**User Story:** As a ripgrep maintainer, I want automated checks that every
artifact reflects the source of truth, so that drift is detected before a
release rather than reported as a bug by users.

#### Acceptance Criteria

1. IF a Flag_Definition in the Flag_Registry is absent from the Bash_Generator
   Artifact, the Zsh_Generator Artifact, the Fish_Generator Artifact, or the
   PowerShell_Generator Artifact, THEN THE Consistency_Checker SHALL terminate
   with a failure result that identifies, for each such case, the missing flag
   name and the affected Artifact.
2. IF an Artifact produced by a Generator references a long flag name that is
   absent from the Flag_Registry, THEN THE Consistency_Checker SHALL terminate
   with a failure result that identifies the unexpected flag name and the
   affected Artifact.
3. IF an Artifact produced by a Generator references a short flag alias that is
   absent from the corresponding Flag_Definition in the Flag_Registry, THEN THE
   Consistency_Checker SHALL terminate with a failure result that identifies
   the unexpected short flag alias and the affected Artifact.
4. IF the description text for a flag in a completion Artifact is not
   character-for-character identical (including case and interior whitespace)
   to the short documentation of the corresponding Flag_Definition, THEN THE
   Consistency_Checker SHALL terminate with a failure result that identifies
   the mismatched flag, the affected Artifact, both the expected text and the
   actual text.
5. WHEN the Consistency_Checker detects one or more violations described in
   criteria 1 through 4 during a single run, THE Consistency_Checker SHALL
   report every detected violation rather than only the first.
6. WHEN every Flag_Definition is represented in every Generator Artifact with
   matching long flag names, matching short flag aliases, and
   character-for-character identical description text, and no Artifact
   references any flag name or short flag alias absent from the Flag_Registry,
   THE Consistency_Checker SHALL terminate with a success result and report no
   violations.

### Requirement 7: Deterministic and Repeatable Generation

**User Story:** As a ripgrep maintainer, I want generation to be deterministic,
so that regenerating an artifact from unchanged definitions produces identical
output and diffs stay meaningful.

#### Acceptance Criteria

1. WHEN a Generator is invoked two or more times against an unchanged
   Flag_Registry, whether within a single process or across separate process
   executions on any supported platform, THE Generator SHALL produce
   byte-identical Artifacts for every invocation.
2. THE Bash_Generator, THE Zsh_Generator, THE Fish_Generator, THE
   PowerShell_Generator, THE Man_Generator, and THE Help_Generator SHALL emit
   the flags within each category in the exact order the corresponding
   Flag_Definitions appear in the Flag_Registry.
3. THE Bash_Generator, THE Zsh_Generator, THE Fish_Generator, THE
   PowerShell_Generator, THE Man_Generator, and THE Help_Generator SHALL emit
   the categories themselves in a single fixed order that is identical across
   every invocation.
4. THE Bash_Generator, THE Zsh_Generator, THE Fish_Generator, THE
   PowerShell_Generator, THE Man_Generator, and THE Help_Generator SHALL
   exclude from every Artifact any content that varies between invocations,
   including timestamps, random identifiers, host-specific paths, and
   locale-dependent formatting.

### Requirement 8: Generation Command Interface

**User Story:** As a ripgrep packager, I want a stable command to produce each
artifact, so that release tooling can generate completions and the man page
without additional dependencies.

#### Acceptance Criteria

1. WHEN `rg --generate man` is invoked, THE Generate_Command SHALL write the
   man page produced by the Man_Generator, and no other content, to standard
   output.
2. WHEN `rg --generate complete-bash` is invoked, THE Generate_Command SHALL
   write the Artifact produced by the Bash_Generator, and no other content, to
   standard output.
3. WHEN `rg --generate complete-zsh` is invoked, THE Generate_Command SHALL
   write the Artifact produced by the Zsh_Generator, and no other content, to
   standard output.
4. WHEN `rg --generate complete-fish` is invoked, THE Generate_Command SHALL
   write the Artifact produced by the Fish_Generator, and no other content, to
   standard output.
5. WHEN `rg --generate complete-powershell` is invoked, THE Generate_Command
   SHALL write the Artifact produced by the PowerShell_Generator, and no other
   content, to standard output.
6. IF `rg --generate` is invoked with a mode that no Generator recognizes,
   THEN THE Generate_Command SHALL write an error message identifying the
   unrecognized mode to standard error, write no Artifact to standard output,
   and terminate with a non-zero exit status.
7. WHEN `rg --generate` is invoked with a mode that a Generator recognizes and
   the corresponding Artifact is written to standard output without error, THE
   Generate_Command SHALL terminate with a zero exit status.
8. IF `rg --generate` is invoked without a mode argument, THEN THE
   Generate_Command SHALL write an error message indicating that a mode
   argument is required to standard error, write no Artifact to standard
   output, and terminate with a non-zero exit status.

### Requirement 9: Help and Man Page Reflect Categories and Documentation

**User Story:** As a ripgrep user, I want the man page and help output to group
flags consistently and show their documentation, so that I can find and
understand flags reliably.

#### Acceptance Criteria

1. THE Man_Generator SHALL place each Flag_Definition under exactly one
   category heading that matches the category assigned to that Flag_Definition.
2. THE Help_Generator SHALL place each Flag_Definition under exactly one
   category heading that matches the category assigned to that Flag_Definition.
3. THE Help_Generator SHALL render the short documentation of every
   Flag_Definition in the `-h` short help output, with all documentation markup
   tags resolved by the Markup_Renderer.
4. THE Help_Generator SHALL render the long documentation of every
   Flag_Definition in the `--help` long help output, with all documentation
   markup tags resolved by the Markup_Renderer.
5. WHERE a Flag_Definition has a value variable name, THE Man_Generator SHALL
   display that value variable name immediately following the flag name within
   that flag's entry in the man page.
6. IF a Flag_Definition is a switch, THEN THE Man_Generator SHALL NOT display a
   value variable name for that flag.

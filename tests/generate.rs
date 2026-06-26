// Example/integration tests for the `--generate` command (the
// `Generate_Command`). These tests invoke the real `rg` binary and assert the
// observable command-line contract: each recognized mode writes its artifact
// to stdout and exits zero, while an unrecognized mode or a missing mode
// argument writes a diagnostic to stderr, writes nothing to stdout, and exits
// non-zero.
//
// Validates Requirements 8.1-8.8 of the unified-flag-source spec.

use crate::util::{Dir, TestCommand};

/// Runs `rg --generate <mode>` on the given command and asserts a successful
/// generation: a zero exit status (Requirement 8.7), non-empty stdout, and
/// nothing written to stderr. Returns the captured stdout so callers can
/// assert on stable tokens.
fn assert_generates(mut cmd: TestCommand, mode: &str) -> String {
    cmd.arg("--generate").arg(mode);
    let out = cmd.raw_output();

    assert_eq!(
        Some(0),
        out.status.code(),
        "expected `--generate {mode}` to exit zero, stderr: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        !out.stdout.is_empty(),
        "expected `--generate {mode}` to write an artifact to stdout",
    );
    assert!(
        out.stderr.is_empty(),
        "expected `--generate {mode}` to leave stderr empty, got: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// `rg --generate man` writes the man page (and only the man page) to stdout
// and exits zero. We assert on the stable roff header `.TH RG` emitted by the
// man template (Requirements 8.1, 8.7).
rgtest!(generate_man, |_d: Dir, cmd: TestCommand| {
    let stdout = assert_generates(cmd, "man");
    assert!(
        stdout.contains(".TH RG"),
        "man output missing stable `.TH RG` token:\n{stdout}",
    );
    assert!(
        stdout.contains("ripgrep"),
        "man output missing expected `ripgrep` token",
    );
});

// `rg --generate complete-bash` writes the Bash completion script to stdout
// and exits zero. The generated script defines the `_rg()` completion
// function (Requirements 8.2, 8.7).
rgtest!(generate_complete_bash, |_d: Dir, cmd: TestCommand| {
    let stdout = assert_generates(cmd, "complete-bash");
    assert!(
        stdout.contains("_rg()"),
        "bash completion missing stable `_rg()` token:\n{stdout}",
    );
});

// `rg --generate complete-zsh` writes the Zsh completion script to stdout and
// exits zero. The generated script begins with the `#compdef rg` directive
// (Requirements 8.3, 8.7).
rgtest!(generate_complete_zsh, |_d: Dir, cmd: TestCommand| {
    let stdout = assert_generates(cmd, "complete-zsh");
    assert!(
        stdout.contains("#compdef rg"),
        "zsh completion missing stable `#compdef rg` token:\n{stdout}",
    );
});

// `rg --generate complete-fish` writes the Fish completion script to stdout
// and exits zero. Each generated entry is a `complete -c rg` line
// (Requirements 8.4, 8.7).
rgtest!(generate_complete_fish, |_d: Dir, cmd: TestCommand| {
    let stdout = assert_generates(cmd, "complete-fish");
    assert!(
        stdout.contains("complete -c rg"),
        "fish completion missing stable `complete -c rg` token:\n{stdout}",
    );
});

// `rg --generate complete-powershell` writes the PowerShell completion script
// to stdout and exits zero. The generated script registers a native argument
// completer (Requirements 8.5, 8.7).
rgtest!(generate_complete_powershell, |_d: Dir, cmd: TestCommand| {
    let stdout = assert_generates(cmd, "complete-powershell");
    assert!(
        stdout.contains("Register-ArgumentCompleter"),
        "powershell completion missing stable \
         `Register-ArgumentCompleter` token:\n{stdout}",
    );
});

// An unrecognized `--generate` mode is rejected: ripgrep writes a diagnostic
// to stderr, writes no artifact to stdout, and exits with a non-zero status
// (Requirement 8.6).
rgtest!(generate_unrecognized_mode, |_d: Dir, mut cmd: TestCommand| {
    cmd.arg("--generate").arg("bogus");
    let out = cmd.raw_output();

    assert!(
        !out.status.success(),
        "expected an unrecognized mode to fail, but it succeeded",
    );
    // The current implementation rejects this during parsing with exit code 2.
    assert_eq!(Some(2), out.status.code());
    assert!(
        out.stdout.is_empty(),
        "expected no artifact on stdout for an unrecognized mode, got: {}",
        String::from_utf8_lossy(&out.stdout),
    );
    assert!(
        !out.stderr.is_empty(),
        "expected a diagnostic on stderr for an unrecognized mode",
    );
});

// `rg --generate` with no mode argument is rejected: ripgrep writes a
// diagnostic to stderr, writes no artifact to stdout, and exits with a
// non-zero status (Requirement 8.8).
rgtest!(generate_missing_mode, |_d: Dir, mut cmd: TestCommand| {
    cmd.arg("--generate");
    let out = cmd.raw_output();

    assert!(
        !out.status.success(),
        "expected a missing mode argument to fail, but it succeeded",
    );
    assert!(
        out.stdout.is_empty(),
        "expected no artifact on stdout when the mode argument is missing, \
         got: {}",
        String::from_utf8_lossy(&out.stdout),
    );
    assert!(
        !out.stderr.is_empty(),
        "expected a diagnostic on stderr when the mode argument is missing",
    );
});

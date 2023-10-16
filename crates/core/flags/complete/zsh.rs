/*!
Provides completions for ripgrep's CLI for the zsh shell.

Unlike completion short for other shells (at time of writing), zsh's
completions for ripgrep are maintained by hand. This is because:

1. They are lovingly written by an expert in such things.
2. Are much higher in quality than the ones below that are auto-generated.
Namely, the zsh completions take application level context about flag
compatibility into account.
3. There is a CI script that fails if a new flag is added to ripgrep that
isn't included in the zsh completions.
4. There is a wealth of documentation in the zsh script explaining how it
works and how it can be extended.

In principle, I'd be open to maintaining any completion script by hand so
long as it meets criteria 3 and 4 above.
*/

/// Generate completions for zsh.
pub(crate) fn generate() -> String {
    include_str!("rg.zsh").to_string()
}

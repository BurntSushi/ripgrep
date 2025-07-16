use crate::hyperlink::HyperlinkAlias;

/// Aliases to well-known hyperlink schemes.
///
/// These need to be sorted by name.
pub(crate) const HYPERLINK_PATTERN_ALIASES: &[HyperlinkAlias] = &[
    #[cfg(not(windows))]
    alias("default", "file://{host}{path}"),
    #[cfg(windows)]
    alias("default", "file://{path}"),
    alias("file", "file://{host}{path}"),
    // https://github.com/misaki-web/grepp
    alias("grep+", "grep+://{path}:{line}"),
    alias("kitty", "file://{host}{path}#{line}"),
    // https://macvim.org/docs/gui_mac.txt.html#mvim%3A%2F%2F
    alias(
        "macvim",
        "mvim://open?url=file://{path}&line={line}&column={column}",
    ),
    alias("none", ""),
    // https://macromates.com/blog/2007/the-textmate-url-scheme/
    alias(
        "textmate",
        "txmt://open?url=file://{path}&line={line}&column={column}",
    ),
    // https://code.visualstudio.com/docs/editor/command-line#_opening-vs-code-with-urls
    alias("vscode", "vscode://file{path}:{line}:{column}"),
    alias("vscode-insiders", "vscode-insiders://file{path}:{line}:{column}"),
    alias("vscodium", "vscodium://file{path}:{line}:{column}"),
];

/// Creates a [`HyperlinkAlias`].
const fn alias(name: &'static str, format: &'static str) -> HyperlinkAlias {
    HyperlinkAlias::new(name, format)
}

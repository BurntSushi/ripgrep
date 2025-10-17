use crate::hyperlink::HyperlinkAlias;

/// Aliases to well-known hyperlink schemes.
///
/// These need to be sorted by name.
pub(super) const HYPERLINK_PATTERN_ALIASES: &[HyperlinkAlias] = &[
    alias(
        "cursor",
        "Cursor scheme (cursor://)",
        "cursor://file{path}:{line}:{column}",
    ),
    prioritized_alias(
        0,
        "default",
        "RFC 8089 scheme (file://) (platform-aware)",
        {
            #[cfg(not(windows))]
            {
                "file://{host}{path}"
            }
            #[cfg(windows)]
            {
                "file://{path}"
            }
        },
    ),
    alias(
        "file",
        "RFC 8089 scheme (file://) with host",
        "file://{host}{path}",
    ),
    // https://github.com/misaki-web/grepp
    alias("grep+", "grep+ scheme (grep+://)", "grep+://{path}:{line}"),
    alias(
        "kitty",
        "kitty-style RFC 8089 scheme (file://) with line number",
        "file://{host}{path}#{line}",
    ),
    // https://macvim.org/docs/gui_mac.txt.html#mvim%3A%2F%2F
    alias(
        "macvim",
        "MacVim scheme (mvim://)",
        "mvim://open?url=file://{path}&line={line}&column={column}",
    ),
    prioritized_alias(1, "none", "disable hyperlinks", ""),
    // https://macromates.com/blog/2007/the-textmate-url-scheme/
    alias(
        "textmate",
        "TextMate scheme (txmt://)",
        "txmt://open?url=file://{path}&line={line}&column={column}",
    ),
    // https://code.visualstudio.com/docs/editor/command-line#_opening-vs-code-with-urls
    alias(
        "vscode",
        "VS Code scheme (vscode://)",
        "vscode://file{path}:{line}:{column}",
    ),
    alias(
        "vscode-insiders",
        "VS Code Insiders scheme (vscode-insiders://)",
        "vscode-insiders://file{path}:{line}:{column}",
    ),
    alias(
        "vscodium",
        "VSCodium scheme (vscodium://)",
        "vscodium://file{path}:{line}:{column}",
    ),
];

/// Creates a [`HyperlinkAlias`].
const fn alias(
    name: &'static str,
    description: &'static str,
    format: &'static str,
) -> HyperlinkAlias {
    HyperlinkAlias { name, description, format, display_priority: None }
}

/// Creates a [`HyperlinkAlias`] with a display priority.
const fn prioritized_alias(
    priority: i16,
    name: &'static str,
    description: &'static str,
    format: &'static str,
) -> HyperlinkAlias {
    HyperlinkAlias {
        name,
        description,
        format,
        display_priority: Some(priority),
    }
}

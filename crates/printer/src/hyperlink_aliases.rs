/// Aliases to well-known hyperlink schemes.
///
/// These need to be sorted by name.
pub const HYPERLINK_PATTERN_ALIASES: &[(&str, &str)] = &[
    #[cfg(unix)]
    ("file", "file://{host}/{file}"),
    #[cfg(windows)]
    ("file", "file:///{file}"),
    // https://github.com/misaki-web/grepp
    ("grep+", "grep+:///{file}:{line}"),
    ("kitty", "file://{host}/{file}#{line}"),
    // https://macvim.org/docs/gui_mac.txt.html#mvim%3A%2F%2F
    ("macvim", "mvim://open?url=file:///{file}&line={line}&column={column}"),
    ("none", ""),
    // https://github.com/inopinatus/sublime_url
    ("subl", "subl://open?url=file:///{file}&line={line}&column={column}"),
    // https://macromates.com/blog/2007/the-textmate-url-scheme/
    ("textmate", "txmt://open?url=file:///{file}&line={line}&column={column}"),
    // https://code.visualstudio.com/docs/editor/command-line#_opening-vs-code-with-urls
    ("vscode", "vscode://file/{file}:{line}:{column}"),
    ("vscode-insiders", "vscode-insiders://file/{file}:{line}:{column}"),
    ("vscodium", "vscodium://file/{file}:{line}:{column}"),
];

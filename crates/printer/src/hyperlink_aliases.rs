/// Aliases to well-known hyperlink schemes.
///
/// These need to be sorted by name.
const HYPERLINK_PATTERN_ALIASES: &[(&str, &str)] = &[
    #[cfg(not(windows))]
    ("default", "file://{host}{path}"),
    #[cfg(windows)]
    ("default", "file://{path}"),
    ("file", "file://{host}{path}"),
    // https://github.com/misaki-web/grepp
    ("grep+", "grep+://{path}:{line}"),
    ("kitty", "file://{host}{path}#{line}"),
    // https://macvim.org/docs/gui_mac.txt.html#mvim%3A%2F%2F
    ("macvim", "mvim://open?url=file://{path}&line={line}&column={column}"),
    ("none", ""),
    // https://macromates.com/blog/2007/the-textmate-url-scheme/
    ("textmate", "txmt://open?url=file://{path}&line={line}&column={column}"),
    // https://code.visualstudio.com/docs/editor/command-line#_opening-vs-code-with-urls
    ("vscode", "vscode://file{path}:{line}:{column}"),
    ("vscode-insiders", "vscode-insiders://file{path}:{line}:{column}"),
    ("vscodium", "vscodium://file{path}:{line}:{column}"),
];

/// Look for the hyperlink format defined by the given alias name.
///
/// If one does not exist, `None` is returned.
pub(crate) fn find(name: &str) -> Option<&str> {
    HYPERLINK_PATTERN_ALIASES
        .binary_search_by_key(&name, |&(name, _)| name)
        .map(|i| HYPERLINK_PATTERN_ALIASES[i].1)
        .ok()
}

/// Return an iterator over all available alias names and their definitions.
pub(crate) fn iter() -> impl Iterator<Item = (&'static str, &'static str)> {
    HYPERLINK_PATTERN_ALIASES.iter().copied()
}

#[cfg(test)]
mod tests {
    use crate::HyperlinkFormat;

    use super::*;

    #[test]
    fn is_sorted() {
        let mut prev = HYPERLINK_PATTERN_ALIASES
            .get(0)
            .expect("aliases should be non-empty")
            .0;
        for &(name, _) in HYPERLINK_PATTERN_ALIASES.iter().skip(1) {
            assert!(
                name > prev,
                "'{prev}' should come before '{name}' in \
                 HYPERLINK_PATTERN_ALIASES",
            );
            prev = name;
        }
    }

    #[test]
    fn alias_names_are_reasonable() {
        for &(name, _) in HYPERLINK_PATTERN_ALIASES.iter() {
            // There's no hard rule here, but if we want to define an alias
            // with a name that doesn't pass this assert, then we should
            // probably flag it as worthy of consideration. For example, we
            // really do not want to define an alias that contains `{` or `}`,
            // which might confuse it for a variable.
            assert!(name.chars().all(|c| c.is_alphanumeric()
                || c == '+'
                || c == '-'
                || c == '.'));
        }
    }

    #[test]
    fn aliases_are_valid_formats() {
        for (name, definition) in HYPERLINK_PATTERN_ALIASES {
            assert!(
                definition.parse::<HyperlinkFormat>().is_ok(),
                "invalid hyperlink alias '{name}': {definition}",
            );
        }
    }
}

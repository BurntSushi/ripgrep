/*!
Modules for generating documentation for ripgrep's flags.
*/

pub(crate) mod help;
pub(crate) mod man;
pub(crate) mod version;

/// Searches for `\tag{...}` occurrences in `doc` and calls `replacement` for
/// each such tag found.
///
/// The first argument given to `replacement` is the tag value, `...`. The
/// second argument is the buffer that accumulates the full replacement text.
///
/// Since this function is only intended to be used on doc strings written into
/// the program source code, callers should panic in `replacement` if there are
/// any errors or unexpected circumstances.
fn render_custom_markup(
    mut doc: &str,
    tag: &str,
    mut replacement: impl FnMut(&str, &mut String),
) -> String {
    let mut out = String::with_capacity(doc.len());
    let tag_prefix = format!(r"\{tag}{{");
    while let Some(offset) = doc.find(&tag_prefix) {
        out.push_str(&doc[..offset]);

        let start = offset + tag_prefix.len();
        let Some(end) = doc[start..].find('}').map(|i| start + i) else {
            unreachable!(r"found {tag_prefix} without closing }}");
        };
        let name = &doc[start..end];
        replacement(name, &mut out);
        doc = &doc[end + 1..];
    }
    out.push_str(doc);
    out
}

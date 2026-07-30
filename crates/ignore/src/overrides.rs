/*!
The overrides module provides a way to specify a set of override globs.

This provides functionality similar to `--include` or `--exclude` in command
line tools.
*/

use std::{
    collections::HashSet,
    path::{Component, Path},
};

use crate::{
    Error, Match,
    gitignore::{self, Gitignore, GitignoreBuilder},
};

/// Glob represents a single glob in an override matcher.
///
/// This is used to report information about the highest precedent glob
/// that matched.
///
/// Note that not all matches necessarily correspond to a specific glob. For
/// example, if there are one or more whitelist globs and a file path doesn't
/// match any glob in the set, then the file path is considered to be ignored.
///
/// The lifetime `'a` refers to the lifetime of the matcher that produced
/// this glob.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Glob<'a>(GlobInner<'a>);

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum GlobInner<'a> {
    /// No glob matched, but the file path should still be ignored.
    UnmatchedIgnore,
    /// A glob matched.
    Matched(&'a gitignore::Glob),
}

impl<'a> Glob<'a> {
    fn unmatched() -> Glob<'a> {
        Glob(GlobInner::UnmatchedIgnore)
    }
}

/// Manages a set of overrides provided explicitly by the end user.
#[derive(Clone, Debug)]
pub struct Override(Gitignore, Option<HashSet<String>>);

impl Override {
    /// Returns an empty matcher that never matches any file path.
    pub fn empty() -> Override {
        Override(Gitignore::empty(), None)
    }

    /// Returns the directory of this override set.
    ///
    /// All matches are done relative to this path.
    pub fn path(&self) -> &Path {
        self.0.path()
    }

    /// Returns true if and only if this matcher is empty.
    ///
    /// When a matcher is empty, it will never match any file path.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the total number of ignore globs.
    pub fn num_ignores(&self) -> u64 {
        self.0.num_whitelists()
    }

    /// Returns the total number of whitelisted globs.
    pub fn num_whitelists(&self) -> u64 {
        self.0.num_ignores()
    }

    /// Returns whether the given file path matched a pattern in this override
    /// matcher.
    ///
    /// `is_dir` should be true if the path refers to a directory and false
    /// otherwise.
    ///
    /// If there are no overrides, then this always returns `Match::None`.
    ///
    /// If there is at least one whitelist override and `is_dir` is false, then
    /// this never returns `Match::None`, since non-matches are interpreted as
    /// ignored.
    ///
    /// A directory may also be ignored if its path cannot contain a match for
    /// any whitelist override.
    ///
    /// The given path is matched to the globs relative to the path given
    /// when building the override matcher. Specifically, before matching
    /// `path`, its prefix (as determined by a common suffix of the directory
    /// given) is stripped. If there is no common suffix/prefix overlap, then
    /// `path` is assumed to reside in the same directory as the root path for
    /// this set of overrides.
    pub fn matched<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        is_dir: bool,
    ) -> Match<Glob<'a>> {
        if self.is_empty() {
            return Match::None;
        }
        let path = path.as_ref();
        let mat = self.0.matched(path, is_dir).invert();
        if mat.is_none() && self.num_whitelists() > 0 {
            if !is_dir
                || self.1.as_ref().is_some_and(|prefixes| {
                    let normalized = crate::pathutil::strip_prefix("./", path)
                        .unwrap_or(path);
                    let root = self.path();
                    if path == root || root.starts_with(normalized) {
                        return false;
                    }
                    let path = normalized;
                    let path = if root == Path::new(".") {
                        path
                    } else if let Some(relative) =
                        crate::pathutil::strip_prefix(root, path)
                    {
                        crate::pathutil::strip_prefix("/", relative)
                            .unwrap_or(relative)
                    } else {
                        path
                    };
                    path.components()
                        .next()
                        .and_then(|component| match component {
                            Component::Normal(component) => component.to_str(),
                            _ => None,
                        })
                        .is_some_and(|component| !prefixes.contains(component))
                })
            {
                return Match::Ignore(Glob::unmatched());
            }
        }
        mat.map(move |giglob| Glob(GlobInner::Matched(giglob)))
    }
}

/// Builds a matcher for a set of glob overrides.
#[derive(Clone, Debug)]
pub struct OverrideBuilder {
    builder: GitignoreBuilder,
    directory_prefixes: Option<HashSet<String>>,
    case_insensitive: bool,
}

impl OverrideBuilder {
    /// Create a new override builder.
    ///
    /// Matching is done relative to the directory path provided.
    pub fn new<P: AsRef<Path>>(path: P) -> OverrideBuilder {
        let mut builder = GitignoreBuilder::new(path);
        builder.allow_unclosed_class(false);
        OverrideBuilder {
            builder,
            directory_prefixes: Some(HashSet::new()),
            case_insensitive: false,
        }
    }

    /// Builds a new override matcher from the globs added so far.
    ///
    /// Once a matcher is built, no new globs can be added to it.
    pub fn build(&self) -> Result<Override, Error> {
        Ok(Override(self.builder.build()?, self.directory_prefixes.clone()))
    }

    /// Add a glob to the set of overrides.
    ///
    /// Globs provided here have precisely the same semantics as a single
    /// line in a `gitignore` file, where the meaning of `!` is inverted:
    /// namely, `!` at the beginning of a glob will ignore a file. Without `!`,
    /// all matches of the glob provided are treated as whitelist matches.
    pub fn add(&mut self, glob: &str) -> Result<&mut OverrideBuilder, Error> {
        self.builder.add_line(None, glob)?;
        if glob.starts_with('!')
            || glob.starts_with('#')
            || glob.trim_end().is_empty()
        {
            return Ok(self);
        }
        if self.case_insensitive {
            self.directory_prefixes = None;
            return Ok(self);
        }
        if let Some(prefixes) = &mut self.directory_prefixes {
            let glob =
                if glob.ends_with("\\ ") { glob } else { glob.trim_end() };
            let is_absolute = glob.starts_with('/');
            let glob = glob.strip_prefix('/').unwrap_or(glob);
            let (glob, is_only_dir) = match glob.strip_suffix('/') {
                Some(glob) => (glob.strip_suffix('\\').unwrap_or(glob), true),
                None => (glob, false),
            };
            let component = glob
                .split_once('/')
                .map(|(component, _)| component)
                .or_else(|| (is_absolute && is_only_dir).then_some(glob));
            match component {
                Some(component)
                    if !component.is_empty()
                        && component != "."
                        && component != ".."
                        && component.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric()
                                || matches!(byte, b'_' | b'-' | b'.')
                        }) =>
                {
                    prefixes.insert(component.to_owned());
                }
                _ => self.directory_prefixes = None,
            }
        }
        Ok(self)
    }

    /// Toggle whether the globs should be matched case insensitively or not.
    ///
    /// When this option is changed, only globs added after the change will be
    /// affected.
    ///
    /// This is disabled by default.
    pub fn case_insensitive(
        &mut self,
        yes: bool,
    ) -> Result<&mut OverrideBuilder, Error> {
        // TODO: This should not return a `Result`. Fix this in the next semver
        // release.
        self.builder.case_insensitive(yes)?;
        self.case_insensitive = yes;
        Ok(self)
    }

    /// Toggle whether unclosed character classes are allowed. When allowed,
    /// a `[` without a matching `]` is treated literally instead of resulting
    /// in a parse error.
    ///
    /// For example, if this is set then the glob `[abc` will be treated as the
    /// literal string `[abc` instead of returning an error.
    ///
    /// By default, this is false. Generally speaking, enabling this leads to
    /// worse failure modes since the glob parser becomes more permissive. You
    /// might want to enable this when compatibility (e.g., with POSIX glob
    /// implementations) is more important than good error messages.
    ///
    /// This default is different from the default for [`Gitignore`]. Namely,
    /// [`Gitignore`] is intended to match git's behavior as-is. But this
    /// abstraction for "override" globs does not necessarily conform to any
    /// other known specification and instead prioritizes better error
    /// messages.
    pub fn allow_unclosed_class(&mut self, yes: bool) -> &mut OverrideBuilder {
        self.builder.allow_unclosed_class(yes);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Override, OverrideBuilder};

    const ROOT: &'static str = "/home/andrew/foo";

    fn ov(globs: &[&str]) -> Override {
        ov_at(ROOT, globs)
    }

    fn ov_at(root: &str, globs: &[&str]) -> Override {
        let mut builder = OverrideBuilder::new(root);
        for glob in globs {
            builder.add(glob).unwrap();
        }
        builder.build().unwrap()
    }

    #[test]
    fn empty() {
        let ov = ov(&[]);
        assert!(ov.matched("a.foo", false).is_none());
        assert!(ov.matched("a", false).is_none());
        assert!(ov.matched("", false).is_none());
    }

    #[test]
    fn simple() {
        let ov = ov(&["*.foo", "!*.bar"]);
        assert!(ov.matched("a.foo", false).is_whitelist());
        assert!(ov.matched("a.foo", true).is_whitelist());
        assert!(ov.matched("a.rs", false).is_ignore());
        assert!(ov.matched("a.rs", true).is_none());
        assert!(ov.matched("a.bar", false).is_ignore());
        assert!(ov.matched("a.bar", true).is_ignore());
    }

    #[test]
    fn only_ignores() {
        let ov = ov(&["!*.bar"]);
        assert!(ov.matched("a.rs", false).is_none());
        assert!(ov.matched("a.rs", true).is_none());
        assert!(ov.matched("a.bar", false).is_ignore());
        assert!(ov.matched("a.bar", true).is_ignore());
    }

    #[test]
    fn precedence() {
        let ov = ov(&["*.foo", "!*.bar.foo"]);
        assert!(ov.matched("a.foo", false).is_whitelist());
        assert!(ov.matched("a.baz", false).is_ignore());
        assert!(ov.matched("a.bar.foo", false).is_ignore());
    }

    #[test]
    fn gitignore() {
        let ov = ov(&["/foo", "bar/*.rs", "baz/**"]);
        assert!(ov.matched("bar/lib.rs", false).is_whitelist());
        assert!(ov.matched("bar/wat/lib.rs", false).is_ignore());
        assert!(ov.matched("wat/bar/lib.rs", false).is_ignore());
        assert!(ov.matched("foo", false).is_whitelist());
        assert!(ov.matched("wat/foo", false).is_ignore());
        assert!(ov.matched("baz", false).is_ignore());
        assert!(ov.matched("baz/a", false).is_whitelist());
        assert!(ov.matched("baz/a/b", false).is_whitelist());
    }

    #[test]
    fn allow_directories() {
        // This tests that directories are NOT ignored when they are unmatched.
        let ov = ov(&["*.rs"]);
        assert!(ov.matched("foo.rs", false).is_whitelist());
        assert!(ov.matched("foo.c", false).is_ignore());
        assert!(ov.matched("foo", false).is_ignore());
        assert!(ov.matched("foo", true).is_none());
        assert!(ov.matched("src/foo.rs", false).is_whitelist());
        assert!(ov.matched("src/foo.c", false).is_ignore());
        assert!(ov.matched("src/foo", false).is_ignore());
        assert!(ov.matched("src/foo", true).is_none());
    }

    #[test]
    fn literal_prefix_prunes_unmatched_directories() {
        let matcher = ov(&["src/**/*.rs", "src/**/*.py", "!src/generated.rs"]);
        assert_eq!(matcher.1.as_ref().unwrap().len(), 1);
        assert!(matcher.matched("src/nested", true).is_none());
        assert!(matcher.matched("src/nested/main.rs", false).is_whitelist());
        assert!(matcher.matched("src/generated.rs", false).is_ignore());
        assert!(matcher.matched("outside", true).is_ignore());

        for glob in ["/src/**/*.rs", "/src/", "/src/   ", r"/src\/"] {
            let matcher = ov(&[glob]);
            assert!(!matcher.matched("src", true).is_ignore(), "{glob}");
            assert!(matcher.matched("outside", true).is_ignore(), "{glob}");
        }
    }

    #[test]
    fn literal_prefix_preserves_override_root_and_explicit_ignores() {
        for (root, path) in [
            (ROOT, ROOT),
            (ROOT, ""),
            (ROOT, "."),
            (".", "./"),
            ("", ""),
            ("repo", "repo"),
            ("repo", "./repo"),
            ("./repo", "repo"),
            ("./repo", "./repo"),
            ("src/", "./src"),
            ("./src/", "./src"),
            ("././src", "./src"),
            ("././src", "././src"),
            ("a/b", "a"),
            ("./a/b", "a"),
        ] {
            let matcher = ov_at(root, &["2/**/*.rs"]);
            assert!(
                matcher.matched(path, true).is_none(),
                "root: {root:?}, path: {path:?}"
            );
        }
        for root in ["repo", "./repo"] {
            let matcher = ov_at(root, &["src/**/*.rs", "!repo"]);
            assert!(matcher.matched("repo", true).is_ignore());
        }
    }

    #[test]
    fn unsupported_positive_globs_disable_directory_pruning() {
        for glob in [
            "*.py",
            "src/",
            "src/   ",
            r"src\/",
            "./src/**/*.rs",
            "../src/**/*.rs",
            "//src/**/*.rs",
        ] {
            let matcher = ov(&["src/**/*.rs", glob]);
            assert!(matcher.matched("outside", true).is_none(), "{glob}");
        }
        for glob in ["src/", "src/   ", r"src\/"] {
            assert!(ov(&[glob]).matched("nested/src", true).is_whitelist());
        }
    }

    #[test]
    fn literal_prefix_respects_case_mode() {
        let ov = OverrideBuilder::new(ROOT)
            .add("src/**/*.rs")
            .unwrap()
            .case_insensitive(true)
            .unwrap()
            .build()
            .unwrap();
        assert!(ov.matched("outside", true).is_ignore());

        let ov = OverrideBuilder::new(ROOT)
            .case_insensitive(true)
            .unwrap()
            .add("src/**/*.rs")
            .unwrap()
            .build()
            .unwrap();
        assert!(ov.matched("SRC/nested/main.RS", false).is_whitelist());
        assert!(ov.matched("outside", true).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn literal_prefix_preserves_relative_root_collisions() {
        for root in ["src", "./src"] {
            let matcher = ov_at(root, &["2/**/*.rs"]);
            assert!(matcher.matched("src2", true).is_none());
            assert!(matcher.matched("src2/other.rs", false).is_whitelist());
            assert!(matcher.matched("outside", true).is_ignore());
        }
    }

    #[test]
    fn many_literal_prefixes_do_not_exceed_regex_limits() {
        let mut builder = OverrideBuilder::new(".");
        let suffix = "a".repeat(242);
        for index in 0..1_300 {
            builder.add(&format!("p{index:06}_{suffix}/needle.rs")).unwrap();
        }
        let matcher = builder.build().unwrap();
        assert_eq!(matcher.1.as_ref().unwrap().len(), 1_300);
        assert!(matcher.matched("outside", true).is_ignore());
    }

    #[test]
    fn absolute_path() {
        let ov = ov(&["!/bar"]);
        assert!(ov.matched("./foo/bar", false).is_none());
    }

    #[test]
    fn case_insensitive() {
        let ov = OverrideBuilder::new(ROOT)
            .case_insensitive(true)
            .unwrap()
            .add("*.html")
            .unwrap()
            .build()
            .unwrap();
        assert!(ov.matched("foo.html", false).is_whitelist());
        assert!(ov.matched("foo.HTML", false).is_whitelist());
        assert!(ov.matched("foo.htm", false).is_ignore());
        assert!(ov.matched("foo.HTM", false).is_ignore());
    }

    #[test]
    fn default_case_sensitive() {
        let ov =
            OverrideBuilder::new(ROOT).add("*.html").unwrap().build().unwrap();
        assert!(ov.matched("foo.html", false).is_whitelist());
        assert!(ov.matched("foo.HTML", false).is_ignore());
        assert!(ov.matched("foo.htm", false).is_ignore());
        assert!(ov.matched("foo.HTM", false).is_ignore());
    }
}

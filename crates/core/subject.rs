use std::path::Path;

/// A configuration for describing how subjects should be built.
#[derive(Clone, Debug)]
struct Config {
    strip_dot_prefix: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { strip_dot_prefix: false }
    }
}

/// A builder for constructing things to search over.
#[derive(Clone, Debug)]
pub struct SubjectBuilder {
    config: Config,
}

impl SubjectBuilder {
    /// Return a new subject builder with a default configuration.
    pub fn new() -> SubjectBuilder {
        SubjectBuilder { config: Config::default() }
    }

    /// Create a new subject from a possibly missing directory entry.
    ///
    /// If the directory entry isn't present, then the corresponding error is
    /// logged if messages have been configured. Otherwise, if the subject is
    /// deemed searchable, then it is returned.
    pub fn build_from_result(
        &self,
        result: Result<ignore::DirEntry, ignore::Error>,
    ) -> Option<Subject> {
        match result {
            Ok(dent) => self.build(dent),
            Err(err) => {
                err_message!("{}", err);
                None
            }
        }
    }

    /// Create a new subject using this builder's configuration.
    ///
    /// If a subject could not be created or should otherwise not be searched,
    /// then this returns `None` after emitting any relevant log messages.
    pub fn build(&self, dent: ignore::DirEntry) -> Option<Subject> {
        let subj =
            Subject { dent, strip_dot_prefix: self.config.strip_dot_prefix };
        if let Some(ignore_err) = subj.dent.error() {
            ignore_message!("{}", ignore_err);
        }
        // If this entry was explicitly provided by an end user, then we always
        // want to search it.
        if subj.is_explicit() {
            return Some(subj);
        }
        // At this point, we only want to search something if it's explicitly a
        // file. This omits symlinks. (If ripgrep was configured to follow
        // symlinks, then they have already been followed by the directory
        // traversal.)
        if subj.is_file() {
            return Some(subj);
        }
        // We got nothing. Emit a debug message, but only if this isn't a
        // directory. Otherwise, emitting messages for directories is just
        // noisy.
        if !subj.is_dir() {
            log::debug!(
                "ignoring {}: failed to pass subject filter: \
                 file type: {:?}, metadata: {:?}",
                subj.dent.path().display(),
                subj.dent.file_type(),
                subj.dent.metadata()
            );
        }
        None
    }

    /// When enabled, if the subject's file path starts with `./` then it is
    /// stripped.
    ///
    /// This is useful when implicitly searching the current working directory.
    pub fn strip_dot_prefix(&mut self, yes: bool) -> &mut SubjectBuilder {
        self.config.strip_dot_prefix = yes;
        self
    }
}

/// A subject is a thing we want to search. Generally, a subject is either a
/// file or stdin.
#[derive(Clone, Debug)]
pub struct Subject {
    dent: ignore::DirEntry,
    strip_dot_prefix: bool,
}

impl Subject {
    /// Return the file path corresponding to this subject.
    ///
    /// If this subject corresponds to stdin, then a special `<stdin>` path
    /// is returned instead.
    pub fn path(&self) -> &Path {
        if self.strip_dot_prefix && self.dent.path().starts_with("./") {
            self.dent.path().strip_prefix("./").unwrap()
        } else {
            self.dent.path()
        }
    }

    /// Returns true if and only if this entry corresponds to stdin.
    pub fn is_stdin(&self) -> bool {
        self.dent.is_stdin()
    }

    /// Returns true if and only if this entry corresponds to a subject to
    /// search that was explicitly supplied by an end user.
    ///
    /// Generally, this corresponds to either stdin or an explicit file path
    /// argument. e.g., in `rg foo some-file ./some-dir/`, `some-file` is
    /// an explicit subject, but, e.g., `./some-dir/some-other-file` is not.
    ///
    /// However, note that ripgrep does not see through shell globbing. e.g.,
    /// in `rg foo ./some-dir/*`, `./some-dir/some-other-file` will be treated
    /// as an explicit subject.
    pub fn is_explicit(&self) -> bool {
        // stdin is obvious. When an entry has a depth of 0, that means it
        // was explicitly provided to our directory iterator, which means it
        // was in turn explicitly provided by the end user. The !is_dir check
        // means that we want to search files even if their symlinks, again,
        // because they were explicitly provided. (And we never want to try
        // to search a directory.)
        self.is_stdin() || (self.dent.depth() == 0 && !self.is_dir())
    }

    /// Returns true if and only if this subject points to a directory after
    /// following symbolic links.
    fn is_dir(&self) -> bool {
        let ft = match self.dent.file_type() {
            None => return false,
            Some(ft) => ft,
        };
        if ft.is_dir() {
            return true;
        }
        // If this is a symlink, then we want to follow it to determine
        // whether it's a directory or not.
        self.dent.path_is_symlink() && self.dent.path().is_dir()
    }

    /// Returns true if and only if this subject points to a file.
    fn is_file(&self) -> bool {
        self.dent.file_type().map_or(false, |ft| ft.is_file())
    }
}

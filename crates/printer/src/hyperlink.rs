use std::{cell::RefCell, io, path::Path, sync::Arc};

use {
    bstr::ByteSlice,
    termcolor::{HyperlinkSpec, WriteColor},
};

use crate::{hyperlink_aliases, util::DecimalFormatter};

/// Hyperlink configuration.
///
/// This configuration specifies both the [hyperlink format](HyperlinkFormat)
/// and an [environment](HyperlinkConfig) for interpolating a subset of
/// variables. The specific subset includes variables that are intended to
/// be invariant throughout the lifetime of a process, such as a machine's
/// hostname.
///
/// A hyperlink configuration can be provided to printer builders such as
/// [`StandardBuilder::hyperlink`](crate::StandardBuilder::hyperlink).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HyperlinkConfig(Arc<HyperlinkConfigInner>);

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct HyperlinkConfigInner {
    env: HyperlinkEnvironment,
    format: HyperlinkFormat,
}

impl HyperlinkConfig {
    /// Create a new configuration from an environment and a format.
    pub fn new(
        env: HyperlinkEnvironment,
        format: HyperlinkFormat,
    ) -> HyperlinkConfig {
        HyperlinkConfig(Arc::new(HyperlinkConfigInner { env, format }))
    }

    /// Returns the hyperlink environment in this configuration.
    pub(crate) fn environment(&self) -> &HyperlinkEnvironment {
        &self.0.env
    }

    /// Returns the hyperlink format in this configuration.
    pub(crate) fn format(&self) -> &HyperlinkFormat {
        &self.0.format
    }
}

/// A hyperlink format with variables.
///
/// This can be created by parsing a string using `HyperlinkFormat::from_str`.
///
/// The default format is empty. An empty format is valid and effectively
/// disables hyperlinks.
///
/// # Example
///
/// ```
/// use grep_printer::HyperlinkFormat;
///
/// let fmt = "vscode".parse::<HyperlinkFormat>()?;
/// assert_eq!(fmt.to_string(), "vscode://file{path}:{line}:{column}");
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HyperlinkFormat {
    parts: Vec<Part>,
    is_line_dependent: bool,
}

impl HyperlinkFormat {
    /// Creates an empty hyperlink format.
    pub fn empty() -> HyperlinkFormat {
        HyperlinkFormat::default()
    }

    /// Returns true if this format is empty.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Creates a [`HyperlinkConfig`] from this format and the environment
    /// given.
    pub fn into_config(self, env: HyperlinkEnvironment) -> HyperlinkConfig {
        HyperlinkConfig::new(env, self)
    }

    /// Returns true if the format can produce line-dependent hyperlinks.
    pub(crate) fn is_line_dependent(&self) -> bool {
        self.is_line_dependent
    }
}

impl std::str::FromStr for HyperlinkFormat {
    type Err = HyperlinkFormatError;

    fn from_str(s: &str) -> Result<HyperlinkFormat, HyperlinkFormatError> {
        use self::HyperlinkFormatErrorKind::*;

        #[derive(Debug)]
        enum State {
            Verbatim,
            VerbatimCloseVariable,
            OpenVariable,
            InVariable,
        }

        let mut builder = FormatBuilder::new();
        let input = match hyperlink_aliases::find(s) {
            Some(format) => format,
            None => s,
        };
        let mut name = String::new();
        let mut state = State::Verbatim;
        let err = |kind| HyperlinkFormatError { kind };
        for ch in input.chars() {
            state = match state {
                State::Verbatim => {
                    if ch == '{' {
                        State::OpenVariable
                    } else if ch == '}' {
                        State::VerbatimCloseVariable
                    } else {
                        builder.append_char(ch);
                        State::Verbatim
                    }
                }
                State::VerbatimCloseVariable => {
                    if ch == '}' {
                        builder.append_char('}');
                        State::Verbatim
                    } else {
                        return Err(err(InvalidCloseVariable));
                    }
                }
                State::OpenVariable => {
                    if ch == '{' {
                        builder.append_char('{');
                        State::Verbatim
                    } else {
                        name.clear();
                        if ch == '}' {
                            builder.append_var(&name)?;
                            State::Verbatim
                        } else {
                            name.push(ch);
                            State::InVariable
                        }
                    }
                }
                State::InVariable => {
                    if ch == '}' {
                        builder.append_var(&name)?;
                        State::Verbatim
                    } else {
                        name.push(ch);
                        State::InVariable
                    }
                }
            };
        }
        match state {
            State::Verbatim => builder.build(),
            State::VerbatimCloseVariable => Err(err(InvalidCloseVariable)),
            State::OpenVariable | State::InVariable => {
                Err(err(UnclosedVariable))
            }
        }
    }
}

impl std::fmt::Display for HyperlinkFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for part in self.parts.iter() {
            part.fmt(f)?;
        }
        Ok(())
    }
}

/// A static environment for hyperlink interpolation.
///
/// This environment permits setting the values of variables used in hyperlink
/// interpolation that are not expected to change for the lifetime of a program.
/// That is, these values are invariant.
///
/// Currently, this includes the hostname and a WSL distro prefix.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HyperlinkEnvironment {
    host: Option<String>,
    wsl_prefix: Option<String>,
}

impl HyperlinkEnvironment {
    /// Create a new empty hyperlink environment.
    pub fn new() -> HyperlinkEnvironment {
        HyperlinkEnvironment::default()
    }

    /// Set the `{host}` variable, which fills in any hostname components of
    /// a hyperlink.
    ///
    /// One can get the hostname in the current environment via the `hostname`
    /// function in the `grep-cli` crate.
    pub fn host(&mut self, host: Option<String>) -> &mut HyperlinkEnvironment {
        self.host = host;
        self
    }

    /// Set the `{wslprefix}` variable, which contains the WSL distro prefix.
    /// An example value is `wsl$/Ubuntu`. The distro name can typically be
    /// discovered from the `WSL_DISTRO_NAME` environment variable.
    pub fn wsl_prefix(
        &mut self,
        wsl_prefix: Option<String>,
    ) -> &mut HyperlinkEnvironment {
        self.wsl_prefix = wsl_prefix;
        self
    }
}

/// An error that can occur when parsing a hyperlink format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperlinkFormatError {
    kind: HyperlinkFormatErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum HyperlinkFormatErrorKind {
    /// This occurs when there are zero variables in the format.
    NoVariables,
    /// This occurs when the {path} variable is missing.
    NoPathVariable,
    /// This occurs when the {line} variable is missing, while the {column}
    /// variable is present.
    NoLineVariable,
    /// This occurs when an unknown variable is used.
    InvalidVariable(String),
    /// The format doesn't start with a valid scheme.
    InvalidScheme,
    /// This occurs when an unescaped `}` is found without a corresponding
    /// `{` preceding it.
    InvalidCloseVariable,
    /// This occurs when a `{` is found without a corresponding `}` following
    /// it.
    UnclosedVariable,
}

impl std::error::Error for HyperlinkFormatError {}

impl std::fmt::Display for HyperlinkFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use self::HyperlinkFormatErrorKind::*;

        match self.kind {
            NoVariables => {
                let aliases = hyperlink_aliases::iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<&str>>()
                    .join(", ");
                write!(
                    f,
                    "at least a {{path}} variable is required in a \
                     hyperlink format, or otherwise use a valid alias: {}",
                    aliases,
                )
            }
            NoPathVariable => {
                write!(
                    f,
                    "the {{path}} variable is required in a hyperlink format",
                )
            }
            NoLineVariable => {
                write!(
                    f,
                    "the hyperlink format contains a {{column}} variable, \
                     but no {{line}} variable is present",
                )
            }
            InvalidVariable(ref name) => {
                write!(
                    f,
                    "invalid hyperlink format variable: '{name}', choose \
                     from: path, line, column, host, wslprefix",
                )
            }
            InvalidScheme => {
                write!(
                    f,
                    "the hyperlink format must start with a valid URL scheme, \
                     i.e., [0-9A-Za-z+-.]+:",
                )
            }
            InvalidCloseVariable => {
                write!(
                    f,
                    "unopened variable: found '}}' without a \
                     corresponding '{{' preceding it",
                )
            }
            UnclosedVariable => {
                write!(
                    f,
                    "unclosed variable: found '{{' without a \
                     corresponding '}}' following it",
                )
            }
        }
    }
}

/// A builder for `HyperlinkFormat`.
///
/// Once a `HyperlinkFormat` is built, it is immutable.
#[derive(Debug)]
struct FormatBuilder {
    parts: Vec<Part>,
}

impl FormatBuilder {
    /// Creates a new hyperlink format builder.
    fn new() -> FormatBuilder {
        FormatBuilder { parts: vec![] }
    }

    /// Appends static text.
    fn append_slice(&mut self, text: &[u8]) -> &mut FormatBuilder {
        if let Some(Part::Text(contents)) = self.parts.last_mut() {
            contents.extend_from_slice(text);
        } else if !text.is_empty() {
            self.parts.push(Part::Text(text.to_vec()));
        }
        self
    }

    /// Appends a single character.
    fn append_char(&mut self, ch: char) -> &mut FormatBuilder {
        self.append_slice(ch.encode_utf8(&mut [0; 4]).as_bytes())
    }

    /// Appends a variable with the given name. If the name isn't recognized,
    /// then this returns an error.
    fn append_var(
        &mut self,
        name: &str,
    ) -> Result<&mut FormatBuilder, HyperlinkFormatError> {
        let part = match name {
            "host" => Part::Host,
            "wslprefix" => Part::WSLPrefix,
            "path" => Part::Path,
            "line" => Part::Line,
            "column" => Part::Column,
            unknown => {
                let err = HyperlinkFormatError {
                    kind: HyperlinkFormatErrorKind::InvalidVariable(
                        unknown.to_string(),
                    ),
                };
                return Err(err);
            }
        };
        self.parts.push(part);
        Ok(self)
    }

    /// Builds the format.
    fn build(&self) -> Result<HyperlinkFormat, HyperlinkFormatError> {
        self.validate()?;
        Ok(HyperlinkFormat {
            parts: self.parts.clone(),
            is_line_dependent: self.parts.contains(&Part::Line),
        })
    }

    /// Validate that the format is well-formed.
    fn validate(&self) -> Result<(), HyperlinkFormatError> {
        use self::HyperlinkFormatErrorKind::*;

        let err = |kind| HyperlinkFormatError { kind };
        // An empty format is fine. It just means hyperlink support is
        // disabled.
        if self.parts.is_empty() {
            return Ok(());
        }
        // If all parts are just text, then there are no variables. It's
        // likely a reference to an invalid alias.
        if self.parts.iter().all(|p| matches!(*p, Part::Text(_))) {
            return Err(err(NoVariables));
        }
        // Even if we have other variables, no path variable means the
        // hyperlink can't possibly work the way it is intended.
        if !self.parts.contains(&Part::Path) {
            return Err(err(NoPathVariable));
        }
        // If the {column} variable is used, then we also need a {line}
        // variable or else {column} can't possibly work.
        if self.parts.contains(&Part::Column)
            && !self.parts.contains(&Part::Line)
        {
            return Err(err(NoLineVariable));
        }
        self.validate_scheme()
    }

    /// Validate that the format starts with a valid scheme. Validation is done
    /// according to how a scheme is defined in RFC 1738 sections 2.1[1] and
    /// 5[2]. In short, a scheme is this:
    ///
    /// scheme = 1*[ lowalpha | digit | "+" | "-" | "." ]
    ///
    /// but is case insensitive.
    ///
    /// [1]: https://datatracker.ietf.org/doc/html/rfc1738#section-2.1
    /// [2]: https://datatracker.ietf.org/doc/html/rfc1738#section-5
    fn validate_scheme(&self) -> Result<(), HyperlinkFormatError> {
        let err_invalid_scheme = HyperlinkFormatError {
            kind: HyperlinkFormatErrorKind::InvalidScheme,
        };
        let Some(Part::Text(ref part)) = self.parts.first() else {
            return Err(err_invalid_scheme);
        };
        let Some(colon) = part.find_byte(b':') else {
            return Err(err_invalid_scheme);
        };
        let scheme = &part[..colon];
        if scheme.is_empty() {
            return Err(err_invalid_scheme);
        }
        let is_valid_scheme_char = |byte| match byte {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'+' | b'-' | b'.' => {
                true
            }
            _ => false,
        };
        if !scheme.iter().all(|&b| is_valid_scheme_char(b)) {
            return Err(err_invalid_scheme);
        }
        Ok(())
    }
}

/// A hyperlink format part.
///
/// A sequence of these corresponds to a complete format. (Not all sequences
/// are valid.)
#[derive(Clone, Debug, Eq, PartialEq)]
enum Part {
    /// Static text.
    ///
    /// We use `Vec<u8>` here (and more generally treat a format string as a
    /// sequence of bytes) because file paths may be arbitrary bytes. A rare
    /// case, but one for which there is no good reason to choke on.
    Text(Vec<u8>),
    /// Variable for the hostname.
    Host,
    /// Variable for a WSL path prefix.
    WSLPrefix,
    /// Variable for the file path.
    Path,
    /// Variable for the line number.
    Line,
    /// Variable for the column number.
    Column,
}

impl Part {
    /// Interpolate this part using the given `env` and `values`, and write
    /// the result of interpolation to the buffer provided.
    fn interpolate_to(
        &self,
        env: &HyperlinkEnvironment,
        values: &Values,
        dest: &mut Vec<u8>,
    ) {
        match self {
            Part::Text(ref text) => dest.extend_from_slice(text),
            Part::Host => dest.extend_from_slice(
                env.host.as_ref().map(|s| s.as_bytes()).unwrap_or(b""),
            ),
            Part::WSLPrefix => dest.extend_from_slice(
                env.wsl_prefix.as_ref().map(|s| s.as_bytes()).unwrap_or(b""),
            ),
            Part::Path => dest.extend_from_slice(&values.path.0),
            Part::Line => {
                let line = DecimalFormatter::new(values.line.unwrap_or(1));
                dest.extend_from_slice(line.as_bytes());
            }
            Part::Column => {
                let column = DecimalFormatter::new(values.column.unwrap_or(1));
                dest.extend_from_slice(column.as_bytes());
            }
        }
    }
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Part::Text(text) => write!(f, "{}", String::from_utf8_lossy(text)),
            Part::Host => write!(f, "{{host}}"),
            Part::WSLPrefix => write!(f, "{{wslprefix}}"),
            Part::Path => write!(f, "{{path}}"),
            Part::Line => write!(f, "{{line}}"),
            Part::Column => write!(f, "{{column}}"),
        }
    }
}

/// The values to replace the format variables with.
///
/// This only consists of values that depend on each path or match printed.
/// Values that are invariant throughout the lifetime of the process are set
/// via a [`HyperlinkEnvironment`].
#[derive(Clone, Debug)]
pub(crate) struct Values<'a> {
    path: &'a HyperlinkPath,
    line: Option<u64>,
    column: Option<u64>,
}

impl<'a> Values<'a> {
    /// Creates a new set of values, starting with the path given.
    ///
    /// Callers may also set the line and column number using the mutator
    /// methods.
    pub(crate) fn new(path: &'a HyperlinkPath) -> Values<'a> {
        Values { path, line: None, column: None }
    }

    /// Sets the line number for these values.
    ///
    /// If a line number is not set and a hyperlink format contains a `{line}`
    /// variable, then it is interpolated with the value of `1` automatically.
    pub(crate) fn line(mut self, line: Option<u64>) -> Values<'a> {
        self.line = line;
        self
    }

    /// Sets the column number for these values.
    ///
    /// If a column number is not set and a hyperlink format contains a
    /// `{column}` variable, then it is interpolated with the value of `1`
    /// automatically.
    pub(crate) fn column(mut self, column: Option<u64>) -> Values<'a> {
        self.column = column;
        self
    }
}

/// An abstraction for interpolating a hyperlink format with values for every
/// variable.
///
/// Interpolation of variables occurs through two different sources. The
/// first is via a `HyperlinkEnvironment` for values that are expected to
/// be invariant. This comes from the `HyperlinkConfig` used to build this
/// interpolator. The second source is via `Values`, which is provided to
/// `Interpolator::begin`. The `Values` contains things like the file path,
/// line number and column number.
#[derive(Clone, Debug)]
pub(crate) struct Interpolator {
    config: HyperlinkConfig,
    buf: RefCell<Vec<u8>>,
}

impl Interpolator {
    /// Create a new interpolator for the given hyperlink format configuration.
    pub(crate) fn new(config: &HyperlinkConfig) -> Interpolator {
        Interpolator { config: config.clone(), buf: RefCell::new(vec![]) }
    }

    /// Start interpolation with the given values by writing a hyperlink
    /// to `wtr`. Subsequent writes to `wtr`, until `Interpolator::end` is
    /// called, are the label for the hyperlink.
    ///
    /// This returns an interpolator status which indicates whether the
    /// hyperlink was written. It might not be written, for example, if the
    /// underlying writer doesn't support hyperlinks or if the hyperlink
    /// format is empty. The status should be provided to `Interpolator::end`
    /// as an instruction for whether to close the hyperlink or not.
    pub(crate) fn begin<W: WriteColor>(
        &self,
        values: &Values,
        mut wtr: W,
    ) -> io::Result<InterpolatorStatus> {
        if self.config.format().is_empty()
            || !wtr.supports_hyperlinks()
            || !wtr.supports_color()
        {
            return Ok(InterpolatorStatus::inactive());
        }
        let mut buf = self.buf.borrow_mut();
        buf.clear();
        for part in self.config.format().parts.iter() {
            part.interpolate_to(self.config.environment(), values, &mut buf);
        }
        let spec = HyperlinkSpec::open(&buf);
        wtr.set_hyperlink(&spec)?;
        Ok(InterpolatorStatus { active: true })
    }

    /// Writes the correct escape sequences to `wtr` to close any extant
    /// hyperlink, marking the end of a hyperlink's label.
    ///
    /// The status given should be returned from a corresponding
    /// `Interpolator::begin` call. Since `begin` may not write a hyperlink
    /// (e.g., if the underlying writer doesn't support hyperlinks), it follows
    /// that `finish` must not close a hyperlink that was never opened. The
    /// status indicates whether the hyperlink was opened or not.
    pub(crate) fn finish<W: WriteColor>(
        &self,
        status: InterpolatorStatus,
        mut wtr: W,
    ) -> io::Result<()> {
        if !status.active {
            return Ok(());
        }
        wtr.set_hyperlink(&HyperlinkSpec::close())
    }
}

/// A status indicating whether a hyperlink was written or not.
///
/// This is created by `Interpolator::begin` and used by `Interpolator::finish`
/// to determine whether a hyperlink was actually opened or not. If it wasn't
/// opened, then finishing interpolation is a no-op.
#[derive(Debug)]
pub(crate) struct InterpolatorStatus {
    active: bool,
}

impl InterpolatorStatus {
    /// Create an inactive interpolator status.
    #[inline]
    pub(crate) fn inactive() -> InterpolatorStatus {
        InterpolatorStatus { active: false }
    }
}

/// Represents the `{path}` part of a hyperlink.
///
/// This is the value to use as-is in the hyperlink, converted from an OS file
/// path.
#[derive(Clone, Debug)]
pub(crate) struct HyperlinkPath(Vec<u8>);

impl HyperlinkPath {
    /// Returns a hyperlink path from an OS path.
    #[cfg(unix)]
    pub(crate) fn from_path(original_path: &Path) -> Option<HyperlinkPath> {
        use std::os::unix::ffi::OsStrExt;

        // We canonicalize the path in order to get an absolute version of it
        // without any `.` or `..` or superfluous separators. Unfortunately,
        // this does also remove symlinks, and in theory, it would be nice to
        // retain them. Perhaps even simpler, we could just join the current
        // working directory with the path and be done with it. There was
        // some discussion about this on PR#2483, and there generally appears
        // to be some uncertainty about the extent to which hyperlinks with
        // things like `..` in them actually work. So for now, we do the safest
        // thing possible even though I think it can result in worse user
        // experience. (Because it means the path you click on and the actual
        // path that gets followed are different, even though they ostensibly
        // refer to the same file.)
        //
        // There's also the potential issue that path canonicalization is
        // expensive since it can touch the file system. That is probably
        // less of an issue since hyperlinks are only created when they're
        // supported, i.e., when writing to a tty.
        //
        // [1]: https://github.com/BurntSushi/ripgrep/pull/2483
        let path = match original_path.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                log::debug!(
                    "hyperlink creation for {:?} failed, error occurred \
                     during path canonicalization: {}",
                    original_path,
                    err,
                );
                return None;
            }
        };
        let bytes = path.as_os_str().as_bytes();
        // This should not be possible since one imagines that canonicalization
        // should always return an absolute path. But it doesn't actually
        // appear guaranteed by POSIX, so we check whether it's true or not and
        // refuse to create a hyperlink from a relative path if it isn't.
        if !bytes.starts_with(b"/") {
            log::debug!(
                "hyperlink creation for {:?} failed, canonicalization \
                 returned {:?}, which does not start with a slash",
                original_path,
                path,
            );
            return None;
        }
        Some(HyperlinkPath::encode(bytes))
    }

    /// Returns a hyperlink path from an OS path.
    #[cfg(windows)]
    pub(crate) fn from_path(original_path: &Path) -> Option<HyperlinkPath> {
        // On Windows, Path::canonicalize returns the result of
        // GetFinalPathNameByHandleW with VOLUME_NAME_DOS,
        // which produces paths such as the following:
        //
        //   \\?\C:\dir\file.txt           (local path)
        //   \\?\UNC\server\dir\file.txt   (network share)
        //
        // The \\?\ prefix comes from VOLUME_NAME_DOS and is constant.
        // It is followed either by the drive letter, or by UNC\
        // (universal naming convention), which denotes a network share.
        //
        // Given that the default URL format on Windows is file://{path}
        // we need to return the following from this function:
        //
        //   /C:/dir/file.txt        (local path)
        //   //server/dir/file.txt   (network share)
        //
        // Which produces the following links:
        //
        //   file:///C:/dir/file.txt        (local path)
        //   file:////server/dir/file.txt   (network share)
        //
        // This substitutes the {path} variable with the expected value for
        // the most common DOS paths, but on the other hand, network paths
        // start with a single slash, which may be unexpected. It seems to work
        // though?
        //
        // Note that the following URL syntax also seems to be valid?
        //
        //   file://server/dir/file.txt
        //
        // But the initial implementation of this routine went for the format
        // above.
        //
        // Also note that the file://C:/dir/file.txt syntax is not correct,
        // even though it often works in practice.
        //
        // In the end, this choice was confirmed by VSCode, whose format is
        //
        //   vscode://file{path}:{line}:{column}
        //
        // and which correctly understands the following URL format for network
        // drives:
        //
        //   vscode://file//server/dir/file.txt:1:1
        //
        // It doesn't parse any other number of slashes in "file//server" as a
        // network path.

        const WIN32_NAMESPACE_PREFIX: &str = r"\\?\";
        const UNC_PREFIX: &str = r"UNC\";

        // As for Unix, we canonicalize the path to make sure we have an
        // absolute path.
        let path = match original_path.canonicalize() {
            Ok(path) => path,
            Err(err) => {
                log::debug!(
                    "hyperlink creation for {:?} failed, error occurred \
                     during path canonicalization: {}",
                    original_path,
                    err,
                );
                return None;
            }
        };
        // We convert the path to a string for easier manipulation. If it
        // wasn't valid UTF-16 (and thus could not be non-lossily transcoded
        // to UTF-8), then we just give up. It's not clear we could make
        // a meaningful hyperlink from it anyway. And this should be an
        // exceptionally rare case.
        let mut string = match path.to_str() {
            Some(string) => string,
            None => {
                log::debug!(
                    "hyperlink creation for {:?} failed, path is not \
                     valid UTF-8",
                    original_path,
                );
                return None;
            }
        };
        // As the comment above says, we expect all canonicalized paths to
        // begin with a \\?\. If it doesn't, then something weird is happening
        // and we should just give up.
        if !string.starts_with(WIN32_NAMESPACE_PREFIX) {
            log::debug!(
                "hyperlink creation for {:?} failed, canonicalization \
                 returned {:?}, which does not start with \\\\?\\",
                original_path,
                path,
            );
            return None;
        }
        string = &string[WIN32_NAMESPACE_PREFIX.len()..];

        // And as above, drop the UNC prefix too, but keep the leading slash.
        if string.starts_with(UNC_PREFIX) {
            string = &string[(UNC_PREFIX.len() - 1)..];
        }
        // Finally, add a leading slash. In the local file case, this turns
        // C:\foo\bar into /C:\foo\bar (and then percent encoding turns it into
        // /C:/foo/bar). In the network share case, this turns \share\foo\bar
        // into /\share/foo/bar (and then percent encoding turns it into
        // //share/foo/bar).
        let with_slash = format!("/{string}");
        Some(HyperlinkPath::encode(with_slash.as_bytes()))
    }

    /// Percent-encodes a path.
    ///
    /// The alphanumeric ASCII characters and "-", ".", "_", "~" are unreserved
    /// as per section 2.3 of RFC 3986 (Uniform Resource Identifier (URI):
    /// Generic Syntax), and are not encoded. The other ASCII characters except
    /// "/" and ":" are percent-encoded, and "\" is replaced by "/" on Windows.
    ///
    /// Section 4 of RFC 8089 (The "file" URI Scheme) does not mandate precise
    /// encoding requirements for non-ASCII characters, and this implementation
    /// leaves them unencoded. On Windows, the UrlCreateFromPathW function does
    /// not encode non-ASCII characters. Doing so with UTF-8 encoded paths
    /// creates invalid file:// URLs on that platform.
    fn encode(input: &[u8]) -> HyperlinkPath {
        let mut result = Vec::with_capacity(input.len());
        for &byte in input.iter() {
            match byte {
                b'0'..=b'9'
                | b'A'..=b'Z'
                | b'a'..=b'z'
                | b'/'
                | b':'
                | b'-'
                | b'.'
                | b'_'
                | b'~'
                | 128.. => {
                    result.push(byte);
                }
                #[cfg(windows)]
                b'\\' => {
                    result.push(b'/');
                }
                _ => {
                    const HEX: &[u8] = b"0123456789ABCDEF";
                    result.push(b'%');
                    result.push(HEX[(byte >> 4) as usize]);
                    result.push(HEX[(byte & 0xF) as usize]);
                }
            }
        }
        HyperlinkPath(result)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn build_format() {
        let format = FormatBuilder::new()
            .append_slice(b"foo://")
            .append_slice(b"bar-")
            .append_slice(b"baz")
            .append_var("path")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(format.to_string(), "foo://bar-baz{path}");
        assert_eq!(format.parts[0], Part::Text(b"foo://bar-baz".to_vec()));
        assert!(!format.is_empty());
    }

    #[test]
    fn build_empty_format() {
        let format = FormatBuilder::new().build().unwrap();

        assert!(format.is_empty());
        assert_eq!(format, HyperlinkFormat::empty());
        assert_eq!(format, HyperlinkFormat::default());
    }

    #[test]
    fn handle_alias() {
        assert!(HyperlinkFormat::from_str("file").is_ok());
        assert!(HyperlinkFormat::from_str("none").is_ok());
        assert!(HyperlinkFormat::from_str("none").unwrap().is_empty());
    }

    #[test]
    fn parse_format() {
        let format = HyperlinkFormat::from_str(
            "foo://{host}/bar/{path}:{line}:{column}",
        )
        .unwrap();

        assert_eq!(
            format.to_string(),
            "foo://{host}/bar/{path}:{line}:{column}"
        );
        assert_eq!(format.parts.len(), 8);
        assert!(format.parts.contains(&Part::Path));
        assert!(format.parts.contains(&Part::Line));
        assert!(format.parts.contains(&Part::Column));
    }

    #[test]
    fn parse_valid() {
        assert!(HyperlinkFormat::from_str("").unwrap().is_empty());
        assert_eq!(
            HyperlinkFormat::from_str("foo://{path}").unwrap().to_string(),
            "foo://{path}"
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{path}/bar").unwrap().to_string(),
            "foo://{path}/bar"
        );

        HyperlinkFormat::from_str("f://{path}").unwrap();
        HyperlinkFormat::from_str("f:{path}").unwrap();
        HyperlinkFormat::from_str("f-+.:{path}").unwrap();
        HyperlinkFormat::from_str("f42:{path}").unwrap();
        HyperlinkFormat::from_str("42:{path}").unwrap();
        HyperlinkFormat::from_str("+:{path}").unwrap();
        HyperlinkFormat::from_str("F42:{path}").unwrap();
        HyperlinkFormat::from_str("F42://foo{{bar}}{path}").unwrap();
    }

    #[test]
    fn parse_invalid() {
        use super::HyperlinkFormatErrorKind::*;

        let err = |kind| HyperlinkFormatError { kind };
        assert_eq!(
            HyperlinkFormat::from_str("foo://bar").unwrap_err(),
            err(NoVariables),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{line}").unwrap_err(),
            err(NoPathVariable),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{path").unwrap_err(),
            err(UnclosedVariable),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{path}:{column}").unwrap_err(),
            err(NoLineVariable),
        );
        assert_eq!(
            HyperlinkFormat::from_str("{path}").unwrap_err(),
            err(InvalidScheme),
        );
        assert_eq!(
            HyperlinkFormat::from_str(":{path}").unwrap_err(),
            err(InvalidScheme),
        );
        assert_eq!(
            HyperlinkFormat::from_str("f*:{path}").unwrap_err(),
            err(InvalidScheme),
        );

        assert_eq!(
            HyperlinkFormat::from_str("foo://{bar}").unwrap_err(),
            err(InvalidVariable("bar".to_string())),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{}}bar}").unwrap_err(),
            err(InvalidVariable("".to_string())),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{b}}ar}").unwrap_err(),
            err(InvalidVariable("b".to_string())),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{bar}}}").unwrap_err(),
            err(InvalidVariable("bar".to_string())),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{{bar}").unwrap_err(),
            err(InvalidCloseVariable),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{{{bar}").unwrap_err(),
            err(InvalidVariable("bar".to_string())),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{b{{ar}").unwrap_err(),
            err(InvalidVariable("b{{ar".to_string())),
        );
        assert_eq!(
            HyperlinkFormat::from_str("foo://{bar{{}").unwrap_err(),
            err(InvalidVariable("bar{{".to_string())),
        );
    }
}

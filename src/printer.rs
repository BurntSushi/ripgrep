use std::error;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

use regex::bytes::Regex;
use termcolor::{Color, ColorSpec, ParseColorError, WriteColor};
use atty;

use pathutil::strip_prefix;
use ignore::types::FileTypeDef;

/// Enum of special values for tty_width
const NOT_A_TTY: usize = 0;
const NOT_YET_KNOWN: usize = 1;
const MIN_TTY_WIDTH: usize = NOT_YET_KNOWN + 1;

/// Printer encapsulates all output logic for searching.
///
/// Note that we currently ignore all write errors. It's probably worthwhile
/// to fix this, but printers are only ever used for writes to stdout or
/// writes to memory, neither of which commonly fail.
pub struct Printer<W> {
    /// The underlying writer.
    wtr: W,
    /// Terminal width.
    tty_width: usize,
    /// How many bytes are printed on this output line
    /// Should actually be characters, but this would require converting
    /// to output terminal encoding ... Keep it simple and assume ascii.
    written_width: usize,
    /// Whether anything has been printed to wtr yet.
    has_printed: bool,
    /// Whether to show column numbers for the first match or not.
    column: bool,
    /// The string to use to separate non-contiguous runs of context lines.
    context_separator: Vec<u8>,
    /// The end-of-line terminator used by the printer. In general, eols are
    /// printed via the match directly, but occasionally we need to insert them
    /// ourselves (for example, to print a context separator).
    eol: u8,
    /// A file separator to show before any matches are printed.
    file_separator: Option<Vec<u8>>,
    /// Whether to show file name as a heading or not.
    ///
    /// N.B. If with_filename is false, then this setting has no effect.
    heading: bool,
    /// Whether to show every match on its own line.
    line_per_match: bool,
    /// Whether to print NUL bytes after a file path instead of new lines
    /// or `:`.
    null: bool,
    /// A string to use as a replacement of each match in a matching line.
    replace: Option<Vec<u8>>,
    /// Whether to prefix each match with the corresponding file name.
    with_filename: bool,
    /// The color specifications.
    colors: ColorSpecs,
}

impl<W: WriteColor> Printer<W> {
    /// Create a new printer that writes to wtr with the given color settings.
    pub fn new(wtr: W) -> Printer<W> {
        Printer {
            wtr: wtr,
            tty_width: NOT_YET_KNOWN,
            written_width: 0,
            has_printed: false,
            column: false,
            context_separator: "--".to_string().into_bytes(),
            eol: b'\n',
            file_separator: None,
            heading: false,
            line_per_match: false,
            null: false,
            replace: None,
            with_filename: false,
            colors: ColorSpecs::default(),
        }
    }

    /// Set the color specifications.
    pub fn colors(mut self, colors: ColorSpecs) -> Printer<W> {
        self.colors = colors;
        self
    }

    /// When set, column numbers will be printed for the first match on each
    /// line.
    pub fn column(mut self, yes: bool) -> Printer<W> {
        self.column = yes;
        self
    }

    /// Set the context separator. The default is `--`.
    pub fn context_separator(mut self, sep: Vec<u8>) -> Printer<W> {
        self.context_separator = sep;
        self
    }

    /// Set the end-of-line terminator. The default is `\n`.
    pub fn eol(mut self, eol: u8) -> Printer<W> {
        self.eol = eol;
        self
    }

    /// If set, the separator is printed before any matches. By default, no
    /// separator is printed.
    pub fn file_separator(mut self, sep: Vec<u8>) -> Printer<W> {
        self.file_separator = Some(sep);
        self
    }

    /// Whether to show file name as a heading or not.
    ///
    /// N.B. If with_filename is false, then this setting has no effect.
    pub fn heading(mut self, yes: bool) -> Printer<W> {
        self.heading = yes;
        self
    }

    /// Whether to show every match on its own line.
    pub fn line_per_match(mut self, yes: bool) -> Printer<W> {
        self.line_per_match = yes;
        self
    }

    /// Whether to cause NUL bytes to follow file paths instead of other
    /// visual separators (like `:`, `-` and `\n`).
    pub fn null(mut self, yes: bool) -> Printer<W> {
        self.null = yes;
        self
    }

    /// Replace every match in each matching line with the replacement string
    /// given.
    ///
    /// The replacement string syntax is documented here:
    /// https://doc.rust-lang.org/regex/regex/bytes/struct.Captures.html#method.expand
    pub fn replace(mut self, replacement: Vec<u8>) -> Printer<W> {
        self.replace = Some(replacement);
        self
    }

    /// When set, each match is prefixed with the file name that it came from.
    pub fn with_filename(mut self, yes: bool) -> Printer<W> {
        self.with_filename = yes;
        self
    }

    /// Returns true if and only if something has been printed.
    pub fn has_printed(&self) -> bool {
        self.has_printed
    }

    /// Flushes the underlying writer and returns it.
    #[allow(dead_code)]
    pub fn into_inner(mut self) -> W {
        let _ = self.wtr.flush();
        self.wtr
    }

    /// Prints a type definition.
    pub fn type_def(&mut self, def: &FileTypeDef) {
        self.write(def.name().as_bytes());
        self.write(b": ");
        let mut first = true;
        for glob in def.globs() {
            if !first {
                self.write(b", ");
            }
            self.write(glob.as_bytes());
            first = false;
        }
        self.write_eol();
    }

    /// Prints the given path.
    pub fn path<P: AsRef<Path>>(&mut self, path: P) {
        let path = strip_prefix("./", path.as_ref()).unwrap_or(path.as_ref());
        self.write_path(path);
        if self.null {
            self.write(b"\x00");
        } else {
            self.write_eol();
        }
    }

    /// Prints the given path and a count of the number of matches found.
    pub fn path_count<P: AsRef<Path>>(&mut self, path: P, count: u64) {
        if self.with_filename {
            self.write_path(path);
            if self.null {
                self.write(b"\x00");
            } else {
                self.write(b":");
            }
        }
        self.write(count.to_string().as_bytes());
        self.write_eol();
    }

    /// Prints the context separator.
    pub fn context_separate(&mut self) {
        // N.B. We can't use `write` here because of borrowing restrictions.
        if self.context_separator.is_empty() {
            return;
        }
        self.has_printed = true;
        let _ = self.wtr.write_all(&self.context_separator);
        let _ = self.wtr.write_all(&[self.eol]);
    }

    pub fn matched<P: AsRef<Path>>(
        &mut self,
        re: &Regex,
        path: P,
        buf: &[u8],
        start: usize,
        end: usize,
        line_number: Option<u64>,
    ) {
        if !self.line_per_match {
            let column =
                if self.column {
                    Some(re.find(&buf[start..end])
                           .map(|(s, _)| s).unwrap_or(0) as u64)
                } else {
                    None
                };
            return self.write_match(
                re, path, buf, start, end, line_number, column);
        }
        for (s, _) in re.find_iter(&buf[start..end]) {
            let column = if self.column { Some(s as u64) } else { None };
            self.write_match(
                re, path.as_ref(), buf, start, end, line_number, column);
        }
    }

    /// Writes an input line, possibly as multiple output lines
    fn write_match<P: AsRef<Path>>(
        &mut self,
        re: &Regex,
        path: P,
        buf: &[u8],
        start: usize,
        end: usize,
        line_number: Option<u64>,
        column: Option<u64>,
    ) {
        // Determine the terminal width if running for first time
        if self.tty_width == NOT_YET_KNOWN {
            if atty::on_stdout() {
                match atty::width() {
                    Some(x) => {
                        let conflicts_with_special_values = x < MIN_TTY_WIDTH;
                        if conflicts_with_special_values {
                            self.tty_width = NOT_A_TTY;
                        } else {
                            self.tty_width = x;
                        }
                    },
                    None => {
                        self.tty_width = NOT_A_TTY
                    },
                }
            }
        }

        let line;
        let mut text_to_print = &buf[start..end];

        // Do the --replace if specified
        if self.replace.is_some() {
            line = re.replace_all(
                &buf[start..end], &**self.replace.as_ref().unwrap());
            text_to_print = line.as_slice();
        }

        // Each iteration prints a line, updating the text_to_print
        loop {
            self.written_width = 0;

            // Filename
            if self.heading && self.with_filename && !self.has_printed {
                self.write_file_sep();
                self.write_heading(path.as_ref());
            } else if !self.heading && self.with_filename {
                self.write_non_heading_path(path.as_ref());
            }

            // Line number
            if let Some(line_number) = line_number {
                self.line_number(line_number, b':');
            }

            // Column
            if let Some(c) = column {
                self.write((c + 1).to_string().as_bytes());
                self.write(b":");
            }

            // Write matches that fit on an output line
            let text_to_print1 = self.write_matched_line(re, text_to_print);

            // Ensure newline
            if text_to_print.last() != Some(&self.eol) {
                self.write_eol();
            }

            // Break if no more matches on this input line
            match text_to_print1 {
                None => break,
                Some(s) => text_to_print = s,
            }
        }
    }

    /// Writes a single output line, at least one match.
    /// Takes an input slice to output, and returns either what didn't fit, or None.
    ///
    /// If running not at tty, print the entire line.
    /// Otherwise, try to limit line length to terminal width.
    /// If all text from current position up to end of match fits, output it, and repeat.
    ///
    /// If not, make sure at least one match is output, maybe truncated at end.
    /// If this is the beginning of output line and some of preceding text also fits, output it.
    /// If there is not enough space until start of the next match (or end of input line),
    /// output as much of beginning text as fits.
    ///
    fn write_matched_line<'b>(&mut self, re: &Regex, buf: &'b [u8]) -> Option<&'b [u8]> {
        let max_width = self.tty_width - self.written_width;
        let is_a_tty = self.tty_width >= MIN_TTY_WIDTH;
        let width_is_limited = is_a_tty;
        let mut last_written = 0;
        let mut matches_written = 0;

        // For each match on this input line
        for (s, e) in re.find_iter(buf) {
            // Does not fit onto ouput line up to end?
            if width_is_limited && e > max_width {
                // Text up to end
                if matches_written > 0 {
                    // Next match does not fit on a line
                    self.write(&buf[last_written..max_width]);
                    // Pretend we wrote all the match (to avoid any re-matches)
                    return Some(&buf[e..]);
                } else {
                    // For this output line, first match does not fit
                    // Should almost never happen, yet is the largest case :)
                    let remaining_width = self.tty_width - self.written_width;
                    let l = e - s;
                    let mut e1 = e;
                    let mut b = last_written;
                    if l > remaining_width {
                        // Match itself doesn't fit; drop its end and all of preceding text
                        b = s;
                        e1 = s + remaining_width;
                    } else if l < remaining_width {
                        // Match fits; drop beginning of preceding text
                        b = e - remaining_width;
                        if b < last_written {
                            b = last_written;
                        }
                    }
                    self.write_one_match(buf, b, s, e1);
                    // Pretend we wrote all the match (to avoid any re-matches)
                    return Some(&buf[s+l..]);
                }
            }

            // This match and all of preceding text fits; output both
            self.write_one_match(buf, last_written, s, e);
            matches_written += 1;
            last_written = e;
        }

        // The rest of line does not contain any matches; drop the end
        let mut e = buf.len();
        if width_is_limited && e > max_width {
            e = max_width
        }
        self.write(&buf[last_written..e]);

        return None;
    }

    /// Prints:
    /// - text preceding a match (&buf[start..match_start])
    /// - match in color (&buf[match_start..match_end])
    /// Resets the color before returning.
    fn write_one_match(&mut self, buf: &[u8],
        start: usize,
        match_start: usize,
        match_end: usize
    ) {
        let color = self.wtr.supports_color();
        self.write(&buf[start..match_start]);
        if color {
            let _ = self.wtr.set_color(self.colors.matched());
        }
        self.write(&buf[match_start..match_end]);
        if color {
            let _ = self.wtr.reset();
        }
    }

    pub fn context<P: AsRef<Path>>(
        &mut self,
        path: P,
        buf: &[u8],
        start: usize,
        end: usize,
        line_number: Option<u64>,
    ) {
        if self.heading && self.with_filename && !self.has_printed {
            self.write_file_sep();
            self.write_heading(path.as_ref());
        } else if !self.heading && self.with_filename {
            self.write_path(path.as_ref());
            if self.null {
                self.write(b"\x00");
            } else {
                self.write(b"-");
            }
        }
        if let Some(line_number) = line_number {
            self.line_number(line_number, b'-');
        }
        self.write(&buf[start..end]);
        if buf[start..end].last() != Some(&self.eol) {
            self.write_eol();
        }
    }

    fn write_heading<P: AsRef<Path>>(&mut self, path: P) {
        let _ = self.wtr.set_color(self.colors.path());
        self.write_path(path.as_ref());
        let _ = self.wtr.reset();
        if self.null {
            self.write(b"\x00");
        } else {
            self.write_eol();
        }
    }

    fn write_non_heading_path<P: AsRef<Path>>(&mut self, path: P) {
        let _ = self.wtr.set_color(self.colors.path());
        self.write_path(path.as_ref());
        let _ = self.wtr.reset();
        if self.null {
            self.write(b"\x00");
        } else {
            self.write(b":");
        }
    }

    fn line_number(&mut self, n: u64, sep: u8) {
        let _ = self.wtr.set_color(self.colors.line());
        self.write(n.to_string().as_bytes());
        let _ = self.wtr.reset();
        self.write(&[sep]);
    }

    #[cfg(unix)]
    fn write_path<P: AsRef<Path>>(&mut self, path: P) {
        use std::os::unix::ffi::OsStrExt;

        let path = path.as_ref().as_os_str().as_bytes();
        self.write(path);
    }

    #[cfg(not(unix))]
    fn write_path<P: AsRef<Path>>(&mut self, path: P) {
        self.write(path.as_ref().to_string_lossy().as_bytes());
    }

    fn write(&mut self, buf: &[u8]) {
        self.has_printed = true;
        self.written_width += buf.len();
        let _ = self.wtr.write_all(buf);
    }

    fn write_eol(&mut self) {
        let eol = self.eol;
        self.write(&[eol]);
    }

    fn write_file_sep(&mut self) {
        if let Some(ref sep) = self.file_separator {
            self.has_printed = true;
            let _ = self.wtr.write_all(sep);
            let _ = self.wtr.write_all(b"\n");
        }
    }
}

/// An error that can occur when parsing color specifications.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// This occurs when an unrecognized output type is used.
    UnrecognizedOutType(String),
    /// This occurs when an unrecognized spec type is used.
    UnrecognizedSpecType(String),
    /// This occurs when an unrecognized color name is used.
    UnrecognizedColor(String, String),
    /// This occurs when an unrecognized style attribute is used.
    UnrecognizedStyle(String),
    /// This occurs when the format of a color specification is invalid.
    InvalidFormat(String),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::UnrecognizedOutType(_) => "unrecognized output type",
            Error::UnrecognizedSpecType(_) => "unrecognized spec type",
            Error::UnrecognizedColor(_, _) => "unrecognized color name",
            Error::UnrecognizedStyle(_) => "unrecognized style attribute",
            Error::InvalidFormat(_) => "invalid color spec",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::UnrecognizedOutType(ref name) => {
                write!(f, "Unrecognized output type '{}'. Choose from: \
                           path, line, match.", name)
            }
            Error::UnrecognizedSpecType(ref name) => {
                write!(f, "Unrecognized spec type '{}'. Choose from: \
                           fg, bg, style, none.", name)
            }
            Error::UnrecognizedColor(_, ref msg) => {
                write!(f, "{}", msg)
            }
            Error::UnrecognizedStyle(ref name) => {
                write!(f, "Unrecognized style attribute '{}'. Choose from: \
                           nobold, bold.", name)
            }
            Error::InvalidFormat(ref original) => {
                write!(f, "Invalid color speci format: '{}'. Valid format \
                           is '(path|line|match):(fg|bg|style):(value)'.",
                           original)
            }
        }
    }
}

impl From<ParseColorError> for Error {
    fn from(err: ParseColorError) -> Error {
        Error::UnrecognizedColor(err.invalid().to_string(), err.to_string())
    }
}

/// A merged set of color specifications.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ColorSpecs {
    path: ColorSpec,
    line: ColorSpec,
    matched: ColorSpec,
}

/// A single color specification provided by the user.
///
/// A `ColorSpecs` can be built by merging a sequence of `Spec`s.
///
/// ## Example
///
/// The only way to build a `Spec` is to parse it from a string. Once multiple
/// `Spec`s have been constructed, then can be merged into a single
/// `ColorSpecs` value.
///
/// ```rust
/// use termcolor::{Color, ColorSpecs, Spec};
///
/// let spec1: Spec = "path:fg:blue".parse().unwrap();
/// let spec2: Spec = "match:bg:green".parse().unwrap();
/// let specs = ColorSpecs::new(&[spec1, spec2]);
///
/// assert_eq!(specs.path().fg(), Some(Color::Blue));
/// assert_eq!(specs.matched().bg(), Some(Color::Green));
/// ```
///
/// ## Format
///
/// The format of a `Spec` is a triple: `{type}:{attribute}:{value}`. Each
/// component is defined as follows:
///
/// * `{type}` can be one of `path`, `line` or `match`.
/// * `{attribute}` can be one of `fg`, `bg` or `style`. `{attribute}` may also
///   be the special value `none`, in which case, `{value}` can be omitted.
/// * `{value}` is either a color name (for `fg`/`bg`) or a style instruction.
///
/// `{type}` controls which part of the output should be styled and is
/// application dependent.
///
/// When `{attribute}` is `none`, then this should cause any existing color
/// settings to be cleared.
///
/// `{value}` should be a color when `{attribute}` is `fg` or `bg`, or it
/// should be a style instruction when `{attribute}` is `style`. When
/// `{attribute}` is `none`, `{value}` must be omitted.
///
/// Valid colors are `black`, `blue`, `green`, `red`, `cyan`, `magenta`,
/// `yellow`, `white`.
///
/// Valid style instructions are `nobold` and `bold`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spec {
    ty: OutType,
    value: SpecValue,
}

/// The actual value given by the specification.
#[derive(Clone, Debug, Eq, PartialEq)]
enum SpecValue {
    None,
    Fg(Color),
    Bg(Color),
    Style(Style),
}

/// The set of configurable portions of ripgrep's output.
#[derive(Clone, Debug, Eq, PartialEq)]
enum OutType {
    Path,
    Line,
    Match,
}

/// The specification type.
#[derive(Clone, Debug, Eq, PartialEq)]
enum SpecType {
    Fg,
    Bg,
    Style,
    None,
}

/// The set of available styles for use in the terminal.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Style {
    Bold,
    NoBold,
}

impl ColorSpecs {
    /// Create color specifications from a list of user supplied
    /// specifications.
    pub fn new(user_specs: &[Spec]) -> ColorSpecs {
        let mut specs = ColorSpecs::default();
        for user_spec in user_specs {
            match user_spec.ty {
                OutType::Path => user_spec.merge_into(&mut specs.path),
                OutType::Line => user_spec.merge_into(&mut specs.line),
                OutType::Match => user_spec.merge_into(&mut specs.matched),
            }
        }
        specs
    }

    /// Return the color specification for coloring file paths.
    fn path(&self) -> &ColorSpec {
        &self.path
    }

    /// Return the color specification for coloring line numbers.
    fn line(&self) -> &ColorSpec {
        &self.line
    }

    /// Return the color specification for coloring matched text.
    fn matched(&self) -> &ColorSpec {
        &self.matched
    }
}

impl Spec {
    /// Merge this spec into the given color specification.
    fn merge_into(&self, cspec: &mut ColorSpec) {
        self.value.merge_into(cspec);
    }
}

impl SpecValue {
    /// Merge this spec value into the given color specification.
    fn merge_into(&self, cspec: &mut ColorSpec) {
        match *self {
            SpecValue::None => cspec.clear(),
            SpecValue::Fg(ref color) => { cspec.set_fg(Some(color.clone())); }
            SpecValue::Bg(ref color) => { cspec.set_bg(Some(color.clone())); }
            SpecValue::Style(ref style) => {
                match *style {
                    Style::Bold => { cspec.set_bold(true); }
                    Style::NoBold => { cspec.set_bold(false); }
                }
            }
        }
    }
}

impl FromStr for Spec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Spec, Error> {
        let pieces: Vec<&str> = s.split(":").collect();
        if pieces.len() <= 1 || pieces.len() > 3 {
            return Err(Error::InvalidFormat(s.to_string()));
        }
        let otype: OutType = try!(pieces[0].parse());
        match try!(pieces[1].parse()) {
            SpecType::None => Ok(Spec { ty: otype, value: SpecValue::None }),
            SpecType::Style => {
                if pieces.len() < 3 {
                    return Err(Error::InvalidFormat(s.to_string()));
                }
                let style: Style = try!(pieces[2].parse());
                Ok(Spec { ty: otype, value: SpecValue::Style(style) })
            }
            SpecType::Fg => {
                if pieces.len() < 3 {
                    return Err(Error::InvalidFormat(s.to_string()));
                }
                let color: Color = try!(pieces[2].parse());
                Ok(Spec { ty: otype, value: SpecValue::Fg(color) })
            }
            SpecType::Bg => {
                if pieces.len() < 3 {
                    return Err(Error::InvalidFormat(s.to_string()));
                }
                let color: Color = try!(pieces[2].parse());
                Ok(Spec { ty: otype, value: SpecValue::Bg(color) })
            }
        }
    }
}

impl FromStr for OutType {
    type Err = Error;

    fn from_str(s: &str) -> Result<OutType, Error> {
        match &*s.to_lowercase() {
            "path" => Ok(OutType::Path),
            "line" => Ok(OutType::Line),
            "match" => Ok(OutType::Match),
            _ => Err(Error::UnrecognizedOutType(s.to_string())),
        }
    }
}

impl FromStr for SpecType {
    type Err = Error;

    fn from_str(s: &str) -> Result<SpecType, Error> {
        match &*s.to_lowercase() {
            "fg" => Ok(SpecType::Fg),
            "bg" => Ok(SpecType::Bg),
            "style" => Ok(SpecType::Style),
            "none" => Ok(SpecType::None),
            _ => Err(Error::UnrecognizedSpecType(s.to_string())),
        }
    }
}

impl FromStr for Style {
    type Err = Error;

    fn from_str(s: &str) -> Result<Style, Error> {
        match &*s.to_lowercase() {
            "bold" => Ok(Style::Bold),
            "nobold" => Ok(Style::NoBold),
            _ => Err(Error::UnrecognizedStyle(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use termcolor::{Color, ColorSpec};
    use super::{ColorSpecs, Error, OutType, Spec, SpecValue, Style};

    #[test]
    fn merge() {
        let user_specs: &[Spec] = &[
            "match:fg:blue".parse().unwrap(),
            "match:none".parse().unwrap(),
            "match:style:bold".parse().unwrap(),
        ];
        let mut expect_matched = ColorSpec::new();
        expect_matched.set_bold(true);
        assert_eq!(ColorSpecs::new(user_specs), ColorSpecs {
            path: ColorSpec::default(),
            line: ColorSpec::default(),
            matched: expect_matched,
        });
    }

    #[test]
    fn specs() {
        let spec: Spec = "path:fg:blue".parse().unwrap();
        assert_eq!(spec, Spec {
            ty: OutType::Path,
            value: SpecValue::Fg(Color::Blue),
        });

        let spec: Spec = "path:bg:red".parse().unwrap();
        assert_eq!(spec, Spec {
            ty: OutType::Path,
            value: SpecValue::Bg(Color::Red),
        });

        let spec: Spec = "match:style:bold".parse().unwrap();
        assert_eq!(spec, Spec {
            ty: OutType::Match,
            value: SpecValue::Style(Style::Bold),
        });

        let spec: Spec = "line:none".parse().unwrap();
        assert_eq!(spec, Spec {
            ty: OutType::Line,
            value: SpecValue::None,
        });
    }

    #[test]
    fn spec_errors() {
        let err = "line:nonee".parse::<Spec>().unwrap_err();
        assert_eq!(err, Error::UnrecognizedSpecType("nonee".to_string()));

        let err = "".parse::<Spec>().unwrap_err();
        assert_eq!(err, Error::InvalidFormat("".to_string()));

        let err = "foo".parse::<Spec>().unwrap_err();
        assert_eq!(err, Error::InvalidFormat("foo".to_string()));

        let err = "line:style:italic".parse::<Spec>().unwrap_err();
        assert_eq!(err, Error::UnrecognizedStyle("italic".to_string()));

        let err = "line:fg:brown".parse::<Spec>().unwrap_err();
        match err {
            Error::UnrecognizedColor(name, _) => assert_eq!(name, "brown"),
            err => assert!(false, "unexpected error: {:?}", err),
        }

        let err = "foo:fg:brown".parse::<Spec>().unwrap_err();
        assert_eq!(err, Error::UnrecognizedOutType("foo".to_string()));
    }
}

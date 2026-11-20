use termcolor::{Color, ColorSpec, ParseColorError};

/// Returns a default set of color specifications.
///
/// This may change over time, but the color choices are meant to be fairly
/// conservative that work across terminal themes.
///
/// Additional color specifications can be added to the list returned. More
/// recently added specifications override previously added specifications.
pub fn default_color_specs() -> Vec<UserColorSpec> {
    vec![
        #[cfg(unix)]
        "path:fg:magenta".parse().unwrap(),
        #[cfg(windows)]
        "path:fg:cyan".parse().unwrap(),
        "line:fg:green".parse().unwrap(),
        "match:fg:red".parse().unwrap(),
        "match:style:bold".parse().unwrap(),
    ]
}

/// An error that can occur when parsing color specifications.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ColorError {
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

impl std::error::Error for ColorError {}

impl ColorError {
    fn from_parse_error(err: ParseColorError) -> ColorError {
        ColorError::UnrecognizedColor(
            err.invalid().to_string(),
            err.to_string(),
        )
    }
}

impl std::fmt::Display for ColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ColorError::UnrecognizedOutType(ref name) => write!(
                f,
                "unrecognized output type '{}'. Choose from: \
                 path, line, column, match, highlight.",
                name,
            ),
            ColorError::UnrecognizedSpecType(ref name) => write!(
                f,
                "unrecognized spec type '{}'. Choose from: \
                 fg, bg, style, none.",
                name,
            ),
            ColorError::UnrecognizedColor(_, ref msg) => write!(f, "{}", msg),
            ColorError::UnrecognizedStyle(ref name) => write!(
                f,
                "unrecognized style attribute '{}'. Choose from: \
                 nobold, bold, nointense, intense, nounderline, \
                 underline, noitalic, italic.",
                name,
            ),
            ColorError::InvalidFormat(ref original) => write!(
                f,
                "invalid color spec format: '{}'. Valid format is \
                 '(path|line|column|match|highlight):(fg|bg|style):(value)'.",
                original,
            ),
        }
    }
}

/// A merged set of color specifications.
///
/// This set of color specifications represents the various color types that
/// are supported by the printers in this crate. A set of color specifications
/// can be created from a sequence of
/// [`UserColorSpec`]s.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ColorSpecs {
    path: ColorSpec,
    line: ColorSpec,
    column: ColorSpec,
    matched: ColorSpec,
    highlight: ColorSpec,
    path_blink: bool,
    line_blink: bool,
    column_blink: bool,
    matched_blink: bool,
    highlight_blink: bool,
}

/// A single color specification provided by the user.
///
/// ## Format
///
/// The format of a `Spec` is a triple: `{type}:{attribute}:{value}`. Each
/// component is defined as follows:
///
/// * `{type}` can be one of `path`, `line`, `column`, `match` or `highlight`.
/// * `{attribute}` can be one of `fg`, `bg` or `style`. `{attribute}` may also
///   be the special value `none`, in which case, `{value}` can be omitted.
/// * `{value}` is either a color name (for `fg`/`bg`) or a style instruction.
///
/// `{type}` controls which part of the output should be styled.
///
/// When `{attribute}` is `none`, then this should cause any existing style
/// settings to be cleared for the specified `type`.
///
/// `{value}` should be a color when `{attribute}` is `fg` or `bg`, or it
/// should be a style instruction when `{attribute}` is `style`. When
/// `{attribute}` is `none`, `{value}` must be omitted.
///
/// Valid colors are `black`, `blue`, `green`, `red`, `cyan`, `magenta`,
/// `yellow`, `white`. Extended colors can also be specified, and are formatted
/// as `x` (for 256-bit colors) or `x,x,x` (for 24-bit true color), where
/// `x` is a number between 0 and 255 inclusive. `x` may be given as a normal
/// decimal number of a hexadecimal number, where the latter is prefixed by
/// `0x`.
///
/// Valid style instructions are `nobold`, `bold`, `intense`, `nointense`,
/// `underline`, `nounderline`, `italic`, `noitalic`.
///
/// ## Example
///
/// The standard way to build a `UserColorSpec` is to parse it from a string.
/// Once multiple `UserColorSpec`s have been constructed, they can be provided
/// to the standard printer where they will automatically be applied to the
/// output.
///
/// A `UserColorSpec` can also be converted to a `termcolor::ColorSpec`:
///
/// ```rust
/// # fn main() {
/// use termcolor::{Color, ColorSpec};
/// use grep_printer::UserColorSpec;
///
/// let user_spec1: UserColorSpec = "path:fg:blue".parse().unwrap();
/// let user_spec2: UserColorSpec = "match:bg:0xff,0x7f,0x00".parse().unwrap();
///
/// let spec1 = user_spec1.to_color_spec();
/// let spec2 = user_spec2.to_color_spec();
///
/// assert_eq!(spec1.fg(), Some(&Color::Blue));
/// assert_eq!(spec2.bg(), Some(&Color::Rgb(0xFF, 0x7F, 0x00)));
/// # }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserColorSpec {
    ty: OutType,
    value: SpecValue,
}

impl UserColorSpec {
    /// Convert this user provided color specification to a specification that
    /// can be used with `termcolor`. This drops the type of this specification
    /// (where the type indicates where the color is applied in the standard
    /// printer, e.g., to the file path or the line numbers, etc.).
    pub fn to_color_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::default();
        self.value.merge_into(&mut spec);
        spec
    }
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
    Column,
    Match,
    Highlight,
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
    Intense,
    NoIntense,
    Underline,
    NoUnderline,
    Italic,
    NoItalic,
    Blink,
    NoBlink,
}

impl ColorSpecs {
    /// Create color specifications from a list of user supplied
    /// specifications.
    pub fn new(specs: &[UserColorSpec]) -> ColorSpecs {
        let mut merged = ColorSpecs::default();
        // Ensure blink flags are initialized to false on creation
        merged.path_blink = false;
        merged.line_blink = false;
        merged.column_blink = false;
        merged.matched_blink = false;
        merged.highlight_blink = false;

        for spec in specs {
            match spec.ty {
                OutType::Path => match spec.value {
                    SpecValue::Fg(ref c) => { merged.path.set_fg(Some(c.clone())); },
                    SpecValue::Bg(ref c) => { merged.path.set_bg(Some(c.clone())); },
                    SpecValue::None => { merged.path.clear(); },
                    SpecValue::Style(ref style) => match *style {
                        Style::Blink => { merged.path_blink = true; },
                        Style::NoBold => { merged.path.set_bold(false); },
                        Style::Bold => { merged.path.set_bold(true); },
                        Style::Intense => { merged.path.set_intense(true); },
                        Style::NoIntense => { merged.path.set_intense(false); },
                        Style::Underline => { merged.path.set_underline(true); },
                        Style::NoUnderline => { merged.path.set_underline(false); },
                        Style::Italic => { merged.path.set_italic(true); },
                        Style::NoItalic => { merged.path.set_italic(false); },
                        Style::NoBlink => { merged.path_blink = false; },
                    },
                },
                OutType::Line => match spec.value {
                    SpecValue::Fg(ref c) => { merged.line.set_fg(Some(c.clone())); },
                    SpecValue::Bg(ref c) => { merged.line.set_bg(Some(c.clone())); },
                    SpecValue::None => { merged.line.clear(); },
                    SpecValue::Style(ref style) => match *style {
                        Style::Blink => { merged.line_blink = true; },
                        Style::NoBold => { merged.line.set_bold(false); },
                        Style::Bold => { merged.line.set_bold(true); },
                        Style::Intense => { merged.line.set_intense(true); },
                        Style::NoIntense => { merged.line.set_intense(false); },
                        Style::Underline => { merged.line.set_underline(true); },
                        Style::NoUnderline => { merged.line.set_underline(false); },
                        Style::Italic => { merged.line.set_italic(true); },
                        Style::NoItalic => { merged.line.set_italic(false); },
                        Style::NoBlink => { merged.line_blink = false; },
                    },
                },
                OutType::Column => match spec.value {
                    SpecValue::Fg(ref c) => { merged.column.set_fg(Some(c.clone())); },
                    SpecValue::Bg(ref c) => { merged.column.set_bg(Some(c.clone())); },
                    SpecValue::None => { merged.column.clear(); },
                    SpecValue::Style(ref style) => match *style {
                        Style::Blink => { merged.column_blink = true; },
                        Style::NoBold => { merged.column.set_bold(false); },
                        Style::Bold => { merged.column.set_bold(true); },
                        Style::Intense => { merged.column.set_intense(true); },
                        Style::NoIntense => { merged.column.set_intense(false); },
                        Style::Underline => { merged.column.set_underline(true); },
                        Style::NoUnderline => { merged.column.set_underline(false); },
                        Style::Italic => { merged.column.set_italic(true); },
                        Style::NoItalic => { merged.column.set_italic(false); },
                        Style::NoBlink => { merged.column_blink = false; },
                    },
                },
                OutType::Match => match spec.value {
                    SpecValue::Fg(ref c) => { merged.matched.set_fg(Some(c.clone())); },
                    SpecValue::Bg(ref c) => { merged.matched.set_bg(Some(c.clone())); },
                    SpecValue::None => { merged.matched.clear(); },
                    SpecValue::Style(ref style) => match *style {
                        Style::Blink => { merged.matched_blink = true; },
                        Style::NoBold => { merged.matched.set_bold(false); },
                        Style::Bold => { merged.matched.set_bold(true); },
                        Style::Intense => { merged.matched.set_intense(true); },
                        Style::NoIntense => { merged.matched.set_intense(false); },
                        Style::Underline => { merged.matched.set_underline(true); },
                        Style::NoUnderline => { merged.matched.set_underline(false); },
                        Style::Italic => { merged.matched.set_italic(true); },
                        Style::NoItalic => { merged.matched.set_italic(false); },
                        Style::NoBlink => { merged.matched_blink = false; },
                    },
                },
                OutType::Highlight => match spec.value {
                    SpecValue::Fg(ref c) => { merged.highlight.set_fg(Some(c.clone())); },
                    SpecValue::Bg(ref c) => { merged.highlight.set_bg(Some(c.clone())); },
                    SpecValue::None => { merged.highlight.clear(); },
                    SpecValue::Style(ref style) => match *style {
                        Style::Blink => { merged.highlight_blink = true; },
                        Style::NoBold => { merged.highlight.set_bold(false); },
                        Style::Bold => { merged.highlight.set_bold(true); },
                        Style::Intense => { merged.highlight.set_intense(true); },
                        Style::NoIntense => { merged.highlight.set_intense(false); },
                        Style::Underline => { merged.highlight.set_underline(true); },
                        Style::NoUnderline => { merged.highlight.set_underline(false); },
                        Style::Italic => { merged.highlight.set_italic(true); },
                        Style::NoItalic => { merged.highlight.set_italic(false); },
                        Style::NoBlink => { merged.highlight_blink = false; },
                    },
                },
            }
        }
        merged
    }

    /// Create a default set of specifications that have color.
    ///
    /// This is distinct from `ColorSpecs`'s `Default` implementation in that
    /// this provides a set of default color choices, where as the `Default`
    /// implementation provides no color choices.
    pub fn default_with_color() -> ColorSpecs {
        ColorSpecs::new(&default_color_specs())
    }

    /// Return the color specification for coloring file paths.
    pub fn path(&self) -> &ColorSpec {
        &self.path
    }

    /// Return the color specification for coloring line numbers.
    pub fn line(&self) -> &ColorSpec {
        &self.line
    }

    /// Return the color specification for coloring column numbers.
    pub fn column(&self) -> &ColorSpec {
        &self.column
    }

    /// Return the color specification for coloring matched text.
    pub fn matched(&self) -> &ColorSpec {
        &self.matched
    }

    /// Return the color specification for coloring entire line if there is a
    /// matched text.
    pub fn highlight(&self) -> &ColorSpec {
        &self.highlight
    }

    /// Return whether `path` styling should enable blink.
    pub fn path_blink(&self) -> bool { self.path_blink }
    /// Return whether `line` styling should enable blink.
    pub fn line_blink(&self) -> bool { self.line_blink }
    /// Return whether `column` styling should enable blink.
    pub fn column_blink(&self) -> bool { self.column_blink }
    /// Return whether `match` styling should enable blink.
    pub fn matched_blink(&self) -> bool { self.matched_blink }
    /// Return whether `highlight` styling should enable blink.
    pub fn highlight_blink(&self) -> bool { self.highlight_blink }
}

impl UserColorSpec {
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
            SpecValue::Fg(ref color) => {
                cspec.set_fg(Some(color.clone()));
            }
            SpecValue::Bg(ref color) => {
                cspec.set_bg(Some(color.clone()));
            }
            SpecValue::Style(ref style) => match *style {
                Style::Bold => {
                    cspec.set_bold(true);
                }
                Style::NoBold => {
                    cspec.set_bold(false);
                }
                Style::Intense => {
                    cspec.set_intense(true);
                }
                Style::NoIntense => {
                    cspec.set_intense(false);
                }
                Style::Underline => {
                    cspec.set_underline(true);
                }
                Style::NoUnderline => {
                    cspec.set_underline(false);
                }
                Style::Italic => {
                    cspec.set_italic(true);
                }
                Style::NoItalic => {
                    cspec.set_italic(false);
                }
                Style::Blink => {
                    // Blink is not representable in ColorSpec; handled separately.
                }
                Style::NoBlink => {
                    // No-op here; handled during ColorSpecs parsing.
                }
            },
        }
    }
}

impl std::str::FromStr for UserColorSpec {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<UserColorSpec, ColorError> {
        let pieces: Vec<&str> = s.split(':').collect();
        if pieces.len() <= 1 || pieces.len() > 3 {
            return Err(ColorError::InvalidFormat(s.to_string()));
        }
        let otype: OutType = pieces[0].parse()?;
        match pieces[1].parse()? {
            SpecType::None => {
                Ok(UserColorSpec { ty: otype, value: SpecValue::None })
            }
            SpecType::Style => {
                if pieces.len() < 3 {
                    return Err(ColorError::InvalidFormat(s.to_string()));
                }
                let style: Style = pieces[2].parse()?;
                Ok(UserColorSpec { ty: otype, value: SpecValue::Style(style) })
            }
            SpecType::Fg => {
                if pieces.len() < 3 {
                    return Err(ColorError::InvalidFormat(s.to_string()));
                }
                let color: Color =
                    pieces[2].parse().map_err(ColorError::from_parse_error)?;
                Ok(UserColorSpec { ty: otype, value: SpecValue::Fg(color) })
            }
            SpecType::Bg => {
                if pieces.len() < 3 {
                    return Err(ColorError::InvalidFormat(s.to_string()));
                }
                let color: Color =
                    pieces[2].parse().map_err(ColorError::from_parse_error)?;
                Ok(UserColorSpec { ty: otype, value: SpecValue::Bg(color) })
            }
        }
    }
}

impl std::str::FromStr for OutType {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<OutType, ColorError> {
        match &*s.to_lowercase() {
            "path" => Ok(OutType::Path),
            "line" => Ok(OutType::Line),
            "column" => Ok(OutType::Column),
            "match" => Ok(OutType::Match),
            "highlight" => Ok(OutType::Highlight),
            _ => Err(ColorError::UnrecognizedOutType(s.to_string())),
        }
    }
}

impl std::str::FromStr for SpecType {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<SpecType, ColorError> {
        match &*s.to_lowercase() {
            "fg" => Ok(SpecType::Fg),
            "bg" => Ok(SpecType::Bg),
            "style" => Ok(SpecType::Style),
            "none" => Ok(SpecType::None),
            _ => Err(ColorError::UnrecognizedSpecType(s.to_string())),
        }
    }
}

impl std::str::FromStr for Style {
    type Err = ColorError;

    fn from_str(s: &str) -> Result<Style, ColorError> {
        match &*s.to_lowercase() {
            "bold" => Ok(Style::Bold),
            "nobold" => Ok(Style::NoBold),
            "intense" => Ok(Style::Intense),
            "nointense" => Ok(Style::NoIntense),
            "underline" => Ok(Style::Underline),
            "nounderline" => Ok(Style::NoUnderline),
            "italic" => Ok(Style::Italic),
            "noitalic" => Ok(Style::NoItalic),
            "blink" => Ok(Style::Blink),
            "noblink" => Ok(Style::NoBlink),
            _ => Err(ColorError::UnrecognizedStyle(s.to_string())),
        }
    }
}

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
                 path, line, column, match.",
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
                 underline.",
                name,
            ),
            ColorError::InvalidFormat(ref original) => write!(
                f,
                "invalid color spec format: '{}'. Valid format \
                 is '(path|line|column|match):(fg|bg|style):(value)'.",
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
}

/// A single color specification provided by the user.
///
/// ## Format
///
/// The format of a `Spec` is a triple: `{type}:{attribute}:{value}`. Each
/// component is defined as follows:
///
/// * `{type}` can be one of `path`, `line`, `column` or `match`.
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
/// `underline`, `nounderline`.
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
}

impl ColorSpecs {
    /// Create color specifications from a list of user supplied
    /// specifications.
    pub fn new(specs: &[UserColorSpec]) -> ColorSpecs {
        let mut merged = ColorSpecs::default();
        for spec in specs {
            match spec.ty {
                OutType::Path => spec.merge_into(&mut merged.path),
                OutType::Line => spec.merge_into(&mut merged.line),
                OutType::Column => spec.merge_into(&mut merged.column),
                OutType::Match => spec.merge_into(&mut merged.matched),
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
            _ => Err(ColorError::UnrecognizedStyle(s.to_string())),
        }
    }
}
